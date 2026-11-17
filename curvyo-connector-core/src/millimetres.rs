//! A length in millimetres, the unit of machine positions in the protocol.

use serde::{Deserialize, Serialize};

/// A length in millimetres. The protocol carries it as a plain JSON number.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millimetres(pub f64);
