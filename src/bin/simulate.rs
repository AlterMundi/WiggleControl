//! Camera-free, independent intent/noise scenarios through the actual gesture engine.
use serde::Serialize;
use std::{error::Error, fs, path::Path};
use wigglecontrol::{filter::OneEuroConfig, Config, Engine, Event, Hand, Observation, Point};

#[derive(Serialize)]
struct Row {
    profile: String,
    trajectory: String,
    fps: u32,
    noise_amplitude: f64,
    seed: u64,
    stationary_rms_px: f64,
    moving_rms_px: f64,
    presses: usize,
    releases: usize,
    press_delay_ms: Option<f64>,
    release_delay_ms: Option<f64>,
}
#[derive(Serialize)]
struct Sample {
    source_us: u64,
    target: Point,
    observed: Point,
    filtered: Point,
    pressed: bool,
}
fn target(t: f64, slow: bool) -> Point {
    let speed = if slow { 0.01 } else { 0.3 };
    let x = if t < 2.0 {
        0.2
    } else if t < 4.0 {
        0.2 + (t - 2.0) * speed
    } else if t < 6.0 {
        0.2 + (6.0 - t) * speed
    } else {
        0.2
    };
    Point { x, y: 0.5 }
}
fn noise(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*seed >> 32) as u32 as f64 / u32::MAX as f64) * 2.0 - 1.0
}
fn run(
    name: &str,
    config: Config,
    fps: u32,
    amplitude: f64,
    seed: u64,
    slow: bool,
) -> (Row, Vec<Sample>) {
    let mut engine = Engine::new(config).expect("simulation config");
    engine.start();
    let mut random = seed;
    let mut stationary = Vec::new();
    let mut moving = Vec::new();
    let mut samples = Vec::new();
    let mut down = Vec::new();
    let mut up = Vec::new();
    for i in 0..8 * fps {
        // Irregular but increasing source time, independent of CPU execution speed.
        let time = 1 + i as u64 * 1_000_000 / fps as u64 + (i % 3) as u64 * 1700;
        let t = (time - 1) as f64 / 1_000_000.0;
        let target = target(t, slow);
        let observed = Point {
            x: target.x + amplitude * noise(&mut random),
            y: target.y + amplitude * noise(&mut random),
        };
        let closed = (3.0..5.0).contains(&t);
        let events = engine.observe(
            time,
            Observation {
                version: 1,
                source_us: time,
                hand: Some(Hand {
                    id: 7,
                    confidence: 0.99,
                    engaged: true,
                    pinch_eligible: true,
                    scrolling: false,
                    cursor: observed,
                    thumb_tip: Point { x: 0.4, y: 0.5 },
                    index_tip: Point {
                        x: if closed { 0.42 } else { 0.52 },
                        y: 0.5,
                    },
                    palm_width: 0.2,
                    pinch_distance_m: None,
                }),
            },
        );
        for event in events {
            if let Event::Button { down: pressed } = event {
                if pressed {
                    down.push(t);
                } else {
                    up.push(t);
                }
            }
        }
        let filtered = engine.cursor_position().expect("valid observed hand");
        let error =
            ((filtered.x - target.x) * 1920.0).powi(2) + ((filtered.y - target.y) * 1080.0).powi(2);
        if (0.5..2.0).contains(&t) || t >= 6.5 {
            stationary.push(error);
        }
        if (2.0..6.0).contains(&t) {
            moving.push(error);
        }
        samples.push(Sample {
            source_us: time,
            target,
            observed,
            filtered,
            pressed: engine.pinch_state() == "pressed",
        });
    }
    let rms = |xs: &[f64]| (xs.iter().sum::<f64>() / xs.len() as f64).sqrt();
    (
        Row {
            profile: name.into(),
            trajectory: if slow { "fine" } else { "fast" }.into(),
            fps,
            noise_amplitude: amplitude,
            seed,
            stationary_rms_px: rms(&stationary),
            moving_rms_px: rms(&moving),
            presses: down.len(),
            releases: up.len(),
            press_delay_ms: down.first().map(|t| (t - 3.0) * 1000.0),
            release_delay_ms: up.first().map(|t| (t - 5.0) * 1000.0),
        },
        samples,
    )
}
fn profiles() -> Vec<(String, Config)> {
    let mut profiles = Vec::new();
    for ms in [0, 30, 60, 100] {
        profiles.push((
            format!("exponential-{ms}ms"),
            Config {
                smoothing_us: ms as f64 * 1000.0,
                ..Config::default()
            },
        ));
    }
    for cutoff in [0.5, 1.0, 2.0] {
        for beta in [2.0, 5.0, 10.0, 20.0] {
            profiles.push((
                format!("adaptive-{cutoff}hz-beta{beta}"),
                Config {
                    adaptive_smoothing: Some(OneEuroConfig {
                        min_cutoff_hz: cutoff,
                        beta,
                        derivative_cutoff_hz: 1.0,
                        speed_floor: 0.0,
                    }),
                    ..Config::default()
                },
            ));
        }
    }
    for floor in [0.03, 0.05] {
        for cutoff in [0.3, 0.5] {
            for beta in [20.0, 30.0, 40.0] {
                profiles.push((
                    format!("adaptive-{cutoff}hz-beta{beta}-floor{floor}"),
                    Config {
                        adaptive_smoothing: Some(OneEuroConfig {
                            min_cutoff_hz: cutoff,
                            beta,
                            derivative_cutoff_hz: 1.0,
                            speed_floor: floor,
                        }),
                        ..Config::default()
                    },
                ));
            }
        }
    }
    profiles
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("wigglecontrol-simulate NEW_DIRECTORY: camera-free engine benchmark; no input injection");
        return Ok(());
    }
    if args.len() != 1 {
        return Err("expected NEW_DIRECTORY".into());
    }
    let root = Path::new(&args[0]);
    fs::create_dir(root)?;
    let mut rows = Vec::new();
    let mut traces = Vec::new();
    let profiles = profiles();
    let total = profiles.len() * 72;
    for (name, config) in profiles {
        for slow in [false, true] {
            for fps in [10, 20, 30] {
                for amplitude in [0.0, 0.003, 0.009] {
                    for seed in [11, 29, 101, 313] {
                        let (row, samples) = run(&name, config, fps, amplitude, seed, slow);
                        if !slow && fps == 20 && amplitude == 0.009 && seed == 313 {
                            traces.push(serde_json::json!({"profile":name,"samples":samples}));
                        }
                        rows.push(row);
                    }
                }
            }
        }
        println!(
            "{}",
            serde_json::json!({"status":"measured","profile":name,"completed":rows.len(),"total":total})
        );
    }
    let report = serde_json::json!({
        "schema":"wigglecontrol.controller-simulation/v1","benchmark_revision":3,
        "camera_opened":false,"system_input":false,
        "ground_truth":"analytic cursor trajectory and scheduled pinch, independently perturbed observations; not a rendered vision benchmark",
        "output":[1920,1080],"rows":rows,"traces":traces,
        "seed_split":{"development":[11,29,101],"held_out":[313]},
        "limits":["Synthetic observation noise is not measured camera noise", "30 FPS is a stress condition, not a supported live session setting", "Does not qualify pinch classification, hand anatomy, model accuracy or physical comfort"]
    });
    fs::write(
        root.join("results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}",
        serde_json::json!({"status":"complete","cases":total,"result":"results.json"})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_filter_preserves_known_pinch_schedule_across_noise_and_fps() {
        for (name, config) in profiles() {
            for fps in [10, 20, 30] {
                let (r, _) = run(&name, config, fps, 0.009, 101, false);
                assert_eq!((r.presses, r.releases), (1, 1), "{name}/{fps}");
                assert!(r.release_delay_ms.unwrap() <= 1000.0 / fps as f64 + 5.1);
            }
        }
    }
}
