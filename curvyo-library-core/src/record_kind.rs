//! The machine family a per-user record belongs to, read from its stored `kind` (ADR 0097 §4).

use serde_json::Value;

/// A machine family: the stored `kind` of its profile and material records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// `laser` (ADR 0038).
    Laser,
    /// `cutter`, the cutting plotter (`specs/0159-cutting-plotter`).
    Cutter,
    /// `embroidery` (`specs/0160-embroidery`).
    Embroidery,
    /// `cnc` (`specs/0161-cnc-output`).
    Cnc,
}

impl Family {
    /// Every family this build knows.
    pub const ALL: [Self; 4] = [Self::Laser, Self::Cutter, Self::Embroidery, Self::Cnc];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Laser => "laser",
            Self::Cutter => "cutter",
            Self::Embroidery => "embroidery",
            Self::Cnc => "cnc",
        }
    }

    /// The family a stored spelling names, `None` for one this build does not know.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|family| family.key() == key)
    }
}

/// What a record's `kind` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindRead {
    /// A family this build knows.
    Family(Family),
    /// A text this build does not know: a newer build's family.
    Unknown,
    /// No `kind`, or one that is not a text: damage for the family's reader to find.
    Missing,
}

/// What the `kind` of the record value `value` says.
#[must_use]
pub fn record_kind(value: &Value) -> KindRead {
    match value.get("kind").and_then(Value::as_str) {
        Some(key) => Family::parse(key).map_or(KindRead::Unknown, KindRead::Family),
        None => KindRead::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_round_trips_its_key() {
        for family in Family::ALL {
            assert_eq!(Family::parse(family.key()), Some(family));
        }
        assert_eq!(Family::parse("Laser"), None);
    }

    #[test]
    fn the_kind_is_a_family_unknown_or_missing() {
        let read = |text: &str| record_kind(&serde_json::from_str(text).unwrap());
        assert_eq!(
            read(r#"{"kind": "embroidery"}"#),
            KindRead::Family(Family::Embroidery)
        );
        assert_eq!(read(r#"{"kind": "plasma"}"#), KindRead::Unknown);
        assert_eq!(read(r#"{"kind": 1}"#), KindRead::Missing);
        assert_eq!(read("{}"), KindRead::Missing);
    }
}
