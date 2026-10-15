//! Two independently engaged hands become direct contacts, never mouse events.
use crate::{touch::Contact, Config, Engine, Event, Hand, Observation, Point};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub version: u32,
    pub source_us: u64,
    /// Producer cannot establish identity, e.g. crossing/occlusion.
    pub ambiguous: bool,
    pub hands: Vec<Hand>,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Start,
    Stop,
    Tick { now_us: u64 },
    Frame { now_us: u64, observation: Frame },
}
/// The camera workspace rectangle mapped onto the configured output.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub min: Point,
    pub max: Point,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            min: Point { x: 0.0, y: 0.0 },
            max: Point { x: 1.0, y: 1.0 },
        }
    }
}
impl Workspace {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.min.valid()
            || !self.max.valid()
            || self.max.x - self.min.x < 0.05
            || self.max.y - self.min.y < 0.05
        {
            return Err(
                "touch workspace must be a finite normalized rectangle at least 0.05 wide/high",
            );
        }
        Ok(())
    }

    pub fn map(self, p: Point) -> Point {
        Point {
            x: ((p.x - self.min.x) / (self.max.x - self.min.x)).clamp(0.0, 1.0),
            y: ((p.y - self.min.y) / (self.max.y - self.min.y)).clamp(0.0, 1.0),
        }
    }
}
struct Track {
    engine: Engine,
    contact: Option<u32>,
}
pub struct Gestures {
    config: Config,
    workspace: Workspace,
    running: bool,
    watermark: Option<u64>,
    last_now: Option<u64>,
    next_contact: u32,
    tracks: BTreeMap<u64, Track>,
}
impl Gestures {
    pub fn new(config: Config, workspace: Workspace) -> Result<Self, &'static str> {
        Engine::new(config)?;
        workspace.validate()?;
        Ok(Self {
            config,
            workspace,
            running: false,
            watermark: None,
            last_now: None,
            next_contact: 0,
            tracks: BTreeMap::new(),
        })
    }
    pub fn start(&mut self) {
        self.tracks.clear();
        self.running = true;
    }
    pub fn stop(&mut self) -> Vec<Contact> {
        self.running = false;
        self.reject()
    }
    pub fn reject(&mut self) -> Vec<Contact> {
        self.tracks.clear();
        Vec::new()
    }
    /// Read-only rearm feedback from the same engines that decide contacts.
    pub fn awaiting_open_hands(&self) -> Vec<u64> {
        self.tracks
            .iter()
            .filter(|(_, track)| track.engine.pinch_state() == "open_first")
            .map(|(id, _)| *id)
            .collect()
    }
    fn events(&mut self, id: u64, events: Vec<Event>) -> Result<(), &'static str> {
        let track = self.tracks.get_mut(&id).ok_or("missing touch track")?;
        for event in events {
            match event {
                Event::Button { down: true } => {
                    if self.next_contact > i32::MAX as u32 {
                        return Err("touch identity exhausted");
                    }
                    track.contact = Some(self.next_contact);
                    self.next_contact += 1;
                }
                Event::Button { down: false } | Event::Cancel { .. } => track.contact = None,
                _ => (),
            }
        }
        Ok(())
    }
    fn contacts(&self) -> Vec<Contact> {
        self.tracks
            .values()
            .filter_map(|t| {
                let id = t.contact?;
                let p = self.workspace.map(t.engine.cursor?);
                Some(Contact { id, x: p.x, y: p.y })
            })
            .collect()
    }
    pub fn tick(&mut self, now: u64) -> Vec<Contact> {
        if self.last_now.is_some_and(|n| now < n) {
            return self.reject();
        }
        self.last_now = Some(now);
        let ids: Vec<_> = self.tracks.keys().copied().collect();
        for id in ids {
            let events = self.tracks.get_mut(&id).unwrap().engine.tick(now);
            if self.events(id, events).is_err() {
                return self.reject();
            }
        }
        self.contacts()
    }
    pub fn observe(&mut self, now: u64, frame: Frame) -> Vec<Contact> {
        let regressed = self.last_now.is_some_and(|n| now < n);
        self.tick(now);
        if !self.running {
            return Vec::new();
        }
        if regressed
            || frame.version != 1
            || frame.source_us > now
            || now - frame.source_us >= self.config.stale_us
            || self.watermark.is_some_and(|n| frame.source_us <= n)
            || frame.hands.len() > 2
            || frame.hands.iter().any(|h| !h.valid())
            || (frame.hands.len() == 2 && frame.hands[0].id == frame.hands[1].id)
        {
            return self.reject();
        }
        self.watermark = Some(frame.source_us);
        if frame.ambiguous {
            return self.reject();
        }
        self.tracks
            .retain(|id, _| frame.hands.iter().any(|h| h.id == *id));
        for mut hand in frame.hands {
            // Touch mode has no wheel semantics or parallel mouse stream.
            hand.scrolling = false;
            let id = hand.id;
            let track = self.tracks.entry(id).or_insert_with(|| {
                let mut engine = Engine::new(self.config).expect("validated config");
                engine.start();
                Track {
                    engine,
                    contact: None,
                }
            });
            let events = track.engine.observe(
                now,
                Observation {
                    version: 1,
                    source_us: frame.source_us,
                    hand: Some(hand),
                },
            );
            if self.events(id, events).is_err() {
                return self.reject();
            }
        }
        self.contacts()
    }
}
pub fn replay_stdin() -> io::Result<()> {
    let mut engine =
        Gestures::new(Config::default(), Workspace::default()).map_err(io::Error::other)?;
    let rx = crate::intent::reader::<Command>();
    let mut output = io::stdout().lock();
    for command in rx {
        let contacts = match command {
            Ok(Command::Start) => {
                engine.start();
                Vec::new()
            }
            Ok(Command::Stop) => engine.stop(),
            Ok(Command::Tick { now_us }) => engine.tick(now_us),
            Ok(Command::Frame {
                now_us,
                observation,
            }) => engine.observe(now_us, observation),
            Err(e) => {
                writeln!(output, "{{\"type\":\"frame\",\"contacts\":[]}}")?;
                output.flush()?;
                return Err(e);
            }
        };
        serde_json::to_writer(
            &mut output,
            &serde_json::json!({"type":"frame","contacts":contacts}),
        )?;
        writeln!(output)?;
    }
    writeln!(output, "{{\"type\":\"frame\",\"contacts\":[]}}")?;
    output.flush()
}
