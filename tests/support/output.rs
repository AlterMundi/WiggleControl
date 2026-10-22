//! Device-free native-event sink for shared controller/delivery integration fixtures.
use std::io;
use wigglecontrol::{
    contacts::Contact,
    output::{self, MouseBackend, MouseIntent, TouchBackend},
    MouseButton,
};
#[derive(Debug, PartialEq)]
pub enum Edge {
    Button(MouseButton, bool),
    Position(f64, f64),
}
#[derive(Default)]
pub struct Mouse {
    held: Vec<MouseButton>,
}
impl MouseBackend for Mouse {
    type Event = Edge;
    fn is_held(&self) -> bool {
        !self.held.is_empty()
    }
    fn release(&mut self) -> Vec<Edge> {
        self.held
            .drain(..)
            .map(|b| Edge::Button(b, false))
            .collect()
    }
    fn apply(&mut self, intent: MouseIntent, _gain: f64) -> io::Result<Vec<Edge>> {
        match intent {
            MouseIntent::Button { down } => self.apply(
                MouseIntent::NamedButton {
                    button: MouseButton::Left,
                    down,
                },
                1.0,
            ),
            MouseIntent::NamedButton { button, down } => {
                if self.held.contains(&button) == down {
                    return Ok(vec![]);
                }
                if down {
                    self.held.push(button);
                } else {
                    self.held.retain(|b| *b != button);
                }
                Ok(vec![Edge::Button(button, down)])
            }
            MouseIntent::Absolute { x, y } => Ok(vec![Edge::Position(x, y)]),
            MouseIntent::Release | MouseIntent::Stop => Ok(self.release()),
            _ => Ok(vec![]),
        }
    }
}
#[derive(Default)]
pub struct Contacts;
impl TouchBackend for Contacts {
    type Event = ();
    fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<()>> {
        if !contacts.is_empty() {
            return Err(io::Error::other("this test sink has no touch device"));
        }
        Ok(vec![])
    }
    fn release(&mut self) -> Vec<()> {
        vec![]
    }
}
pub type State = output::State<Mouse, Contacts>;
