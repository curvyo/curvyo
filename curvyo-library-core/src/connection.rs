//! How a machine profile of any family reaches its machine (ADR 0097 §3): the connector address,
//! the controller port and the baud rate, as stored data.

use serde::{Deserialize, Serialize};

/// The most characters of the connector address and of the controller port.
pub const MAX_CONNECTION_CHARS: usize = 200;

/// The serial baud rate of the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct BaudRate(u32);

impl BaudRate {
    /// The smallest value.
    pub const MIN: u32 = 1;
    /// The largest value.
    pub const MAX: u32 = 4_000_000;
    /// The value of a new profile.
    pub const DEFAULT: Self = Self(115_200);

    /// The value `n`, `None` outside the range.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u32::try_from(n)
            .ok()
            .filter(|n| (Self::MIN..=Self::MAX).contains(n))
            .map(Self)
    }

    /// The plain number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for BaudRate {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u64> for BaudRate {
    type Error = &'static str;
    fn try_from(n: u64) -> Result<Self, Self::Error> {
        Self::new(n).ok_or("out of range")
    }
}

impl From<BaudRate> for u64 {
    fn from(value: BaudRate) -> Self {
        Self::from(value.0)
    }
}

/// How a profile reaches its machine: stored data only, read by the connector (`0064`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connection {
    /// The connector's address, up to [`MAX_CONNECTION_CHARS`] characters.
    pub connector_address: String,
    /// The controller port (`/dev/ttyUSB0`, `COM3`), up to [`MAX_CONNECTION_CHARS`] characters.
    pub controller_port: String,
    /// The baud rate.
    pub baud_rate: BaudRate,
}

impl Connection {
    /// Whether the address and the port are within [`MAX_CONNECTION_CHARS`].
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.connector_address.chars().count() <= MAX_CONNECTION_CHARS
            && self.controller_port.chars().count() <= MAX_CONNECTION_CHARS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_baud_rate_has_its_range_and_default() {
        assert_eq!(BaudRate::default().get(), 115_200);
        assert!(BaudRate::new(0).is_none());
        assert_eq!(BaudRate::new(4_000_000).unwrap().get(), 4_000_000);
        assert!(BaudRate::new(4_000_001).is_none());
        assert_eq!(serde_json::to_string(&BaudRate::DEFAULT).unwrap(), "115200");
        assert!(serde_json::from_str::<BaudRate>("0").is_err());
    }

    #[test]
    fn a_connection_is_valid_up_to_200_characters_each() {
        let mut connection = Connection::default();
        assert!(connection.is_valid());
        connection.controller_port = "p".repeat(MAX_CONNECTION_CHARS);
        assert!(connection.is_valid());
        connection.connector_address = "a".repeat(MAX_CONNECTION_CHARS + 1);
        assert!(!connection.is_valid());
    }
}
