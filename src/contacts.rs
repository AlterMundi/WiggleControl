//! Logical direct-contact data; no device or operating-system event codes.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}
