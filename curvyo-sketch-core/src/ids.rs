//! Stable identifiers separate document identity from solver indices.
use serde::{Deserialize, Serialize};
macro_rules! identifier {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}
identifier!(
    EntityId,
    "A stable geometric entity identifier assigned by the caller."
);
identifier!(
    ConstraintId,
    "A stable user constraint identifier assigned by the caller."
);
identifier!(
    ParameterId,
    "A stable parameter identifier preserved across renaming."
);
