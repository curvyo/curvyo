//! The values of a sketch instance: what a clone may set (an exposed key), the number it sets and
//! the canonical set a solve is keyed by (`specs/0099-sketch-pages/adrs.md` A2, A3).

use std::collections::BTreeMap;

use curvyo_sketch_core::{ConstraintId, ParameterId, Quantity};

use crate::units::{Angle, Length};

/// The unit kind of an exposed value (this revision: no Count, no Scalar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum InstanceKind {
    /// A length in millimetres.
    Length,
    /// An angle, stored in degrees.
    Angle,
}

/// One number a clone sets: a length or an angle (radians in memory, degrees stored).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstanceValue {
    /// A length.
    Length(Length),
    /// An angle.
    Angle(Angle),
}

impl InstanceValue {
    /// A length of `mm` millimetres.
    #[must_use]
    pub const fn from_mm(mm: f64) -> Self {
        Self::Length(Length::from_mm(mm))
    }

    /// An angle of `degrees` degrees.
    #[must_use]
    pub fn from_deg(degrees: f64) -> Self {
        Self::Angle(Angle::from_radians(degrees.to_radians()))
    }

    /// The unit kind.
    #[must_use]
    pub const fn kind(self) -> InstanceKind {
        match self {
            Self::Length(_) => InstanceKind::Length,
            Self::Angle(_) => InstanceKind::Angle,
        }
    }

    /// The number as stored: millimetres, or degrees to a billionth (so a degree typed in comes
    /// back as typed from its radians).
    #[must_use]
    pub fn number(self) -> f64 {
        match self {
            Self::Length(length) => length.as_mm(),
            Self::Angle(angle) => (angle.as_radians().to_degrees() * 1e9).round() / 1e9,
        }
    }

    /// The sketch quantity (millimetres or radians).
    ///
    /// # Errors
    /// Fails for a number that is not finite.
    pub fn quantity(self) -> Result<Quantity, curvyo_sketch_core::SketchError> {
        match self {
            Self::Length(length) => Quantity::length(length.as_mm()),
            Self::Angle(angle) => Quantity::angle(angle.as_radians()),
        }
    }

    /// The value of the same kind with the stored `number` (millimetres or degrees).
    #[must_use]
    pub fn with_number(self, number: f64) -> Self {
        match self {
            Self::Length(_) => Self::from_mm(number),
            Self::Angle(_) => Self::from_deg(number),
        }
    }

    /// The canonical key of the stored number: kind and the bits of millimetres or degrees, with
    /// zero unsigned, so two sets compare by what is stored and never by a radian round trip.
    fn canon(self) -> (InstanceKind, u64) {
        (self.kind(), (self.number() + 0.0).to_bits())
    }
}

/// The name of one exposed value inside a sketch: `p:<ParameterId>` or `d:<ConstraintId>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct ExposedKey(String);

/// What an [`ExposedKey`] names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExposedTarget {
    /// A named parameter.
    Parameter(ParameterId),
    /// A driving dimension.
    Dimension(ConstraintId),
}

impl ExposedKey {
    /// The key of a named parameter.
    #[must_use]
    pub fn parameter(id: &ParameterId) -> Self {
        Self(format!("p:{}", id.0))
    }

    /// The key of a driving dimension.
    #[must_use]
    pub fn dimension(id: &ConstraintId) -> Self {
        Self(format!("d:{}", id.0))
    }

    /// The key a stored string names, `None` for any other shape or an empty id.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text
            .strip_prefix("p:")
            .or_else(|| text.strip_prefix("d:"))?;
        (!rest.is_empty()).then(|| Self(text.to_owned()))
    }

    /// The stored string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// What the key names.
    #[must_use]
    pub fn target(&self) -> ExposedTarget {
        match self.0.split_at(2) {
            ("p:", id) => ExposedTarget::Parameter(ParameterId(id.to_owned())),
            (_, id) => ExposedTarget::Dimension(ConstraintId(id.to_owned())),
        }
    }
}

/// A canonical set of own values, keyed by exposed key. Two sets are equal when their stored
/// numbers are, so it is the key of a solve and of a shared outline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ValueSet(BTreeMap<ExposedKey, (InstanceKind, u64)>);

impl ValueSet {
    /// Sets `key` to `value`.
    pub fn insert(&mut self, key: ExposedKey, value: InstanceValue) {
        self.0.insert(key, value.canon());
    }

    /// Removes `key`.
    pub fn remove(&mut self, key: &ExposedKey) {
        self.0.remove(key);
    }

    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &ExposedKey) -> Option<InstanceValue> {
        self.0.get(key).map(|&(kind, bits)| value_of(kind, bits))
    }

    /// Whether no value is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The values in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&ExposedKey, InstanceValue)> {
        self.0
            .iter()
            .map(|(key, &(kind, bits))| (key, value_of(kind, bits)))
    }

    /// This set with every value of `other` written over it.
    #[must_use]
    pub fn overlaid(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged
            .0
            .extend(other.0.iter().map(|(key, value)| (key.clone(), *value)));
        merged
    }

    /// The values as a sketch's [`InstanceValues`](curvyo_sketch_core::InstanceValues).
    ///
    /// # Errors
    /// Fails for a number that is not finite.
    pub fn to_sketch_values(
        &self,
    ) -> Result<curvyo_sketch_core::InstanceValues, curvyo_sketch_core::SketchError> {
        let mut values = curvyo_sketch_core::InstanceValues::default();
        for (key, value) in self.iter() {
            match key.target() {
                ExposedTarget::Parameter(id) => {
                    values.parameters.insert(id, value.quantity()?);
                }
                ExposedTarget::Dimension(id) => {
                    values.dimensions.insert(id, value.quantity()?);
                }
            }
        }
        Ok(values)
    }
}

fn value_of(kind: InstanceKind, bits: u64) -> InstanceValue {
    let number = f64::from_bits(bits);
    match kind {
        InstanceKind::Length => InstanceValue::from_mm(number),
        InstanceKind::Angle => InstanceValue::from_deg(number),
    }
}

/// One exposed value of a sketch as the read pass holds it: validated against the graph, with its
/// label, kind, limits and the default the sketch itself has.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ExposedParam {
    /// The key.
    pub key: ExposedKey,
    /// The clone field's label: the parameter's name, or the dimension's exposed name.
    pub label: String,
    /// The unit kind.
    pub kind: InstanceKind,
    /// The lower limit in millimetres or degrees, `None` for unlimited.
    pub min: Option<f64>,
    /// The upper limit in millimetres or degrees, `None` for unlimited.
    pub max: Option<f64>,
    /// The sketch's own value in millimetres or degrees.
    pub default: f64,
}

impl ExposedParam {
    /// `number` (millimetres or degrees) clamped into the limits, and whether it was.
    #[must_use]
    pub fn clamp(&self, number: f64) -> (f64, bool) {
        let mut clamped = number;
        if let Some(min) = self.min {
            clamped = clamped.max(min);
        }
        if let Some(max) = self.max {
            clamped = clamped.min(max);
        }
        #[allow(clippy::float_cmp)] // exactly the unchanged number
        (clamped, clamped != number)
    }

    /// The default as a value.
    #[must_use]
    pub fn default_value(&self) -> InstanceValue {
        match self.kind {
            InstanceKind::Length => InstanceValue::from_mm(self.default),
            InstanceKind::Angle => InstanceValue::from_deg(self.default),
        }
    }
}
