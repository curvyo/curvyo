//! The parameter values of one generation: read from the stored map through the schema (a missing
//! or invalid one reads as its default, a number outside its bounds is clamped and reported).

use curvyo_document_core::{Color, Length};
use curvyo_extension_core::{Bound, Clamp, Param, ParamKind, ParamValue, StoredParams};

/// The values of one generation and the clamps they went through.
pub(crate) struct Values<'a> {
    schema: &'a [Param],
    stored: StoredParams,
    clamps: Vec<Clamp>,
}

impl<'a> Values<'a> {
    pub(crate) fn read(schema: &'a [Param], params: &StoredParams) -> Self {
        let mut values = Self {
            schema,
            stored: StoredParams::new(),
            clamps: Vec::new(),
        };
        for param in schema {
            let typed = params
                .get(param.key)
                .and_then(ParamValue::from_stored)
                .and_then(|asked| {
                    param
                        .accept(asked.clone())
                        .map(|accepted| (asked, accepted))
                });
            let value = match typed {
                Some((asked, accepted)) => {
                    if asked != accepted {
                        values.report(param, &asked, &accepted);
                    }
                    accepted
                }
                None => param.default.clone(),
            };
            values
                .stored
                .insert(param.key.to_owned(), value.to_stored());
        }
        values
    }

    fn report(&mut self, param: &Param, asked: &ParamValue, accepted: &ParamValue) {
        let below = match (asked, accepted) {
            (ParamValue::Length(a), ParamValue::Length(b)) => a.as_mm() < b.as_mm(),
            (ParamValue::Count(a), ParamValue::Count(b)) => a < b,
            (ParamValue::Percent(a), ParamValue::Percent(b)) => a < b,
            (ParamValue::Ratio(a), ParamValue::Ratio(b)) => a < b,
            _ => return,
        };
        let bound = if below {
            Bound::Minimum
        } else {
            Bound::Maximum
        };
        self.set_clamp(param.key, accepted.clone(), bound);
    }

    pub(crate) fn set_clamp(&mut self, key: &'static str, value: ParamValue, bound: Bound) {
        self.clamps.retain(|clamp| clamp.key != key);
        self.clamps.push(Clamp { key, value, bound });
    }

    pub(crate) fn drop_clamp(&mut self, key: &str) {
        self.clamps.retain(|clamp| clamp.key != key);
    }

    fn value(&self, key: &str) -> ParamValue {
        self.stored
            .get(key)
            .and_then(ParamValue::from_stored)
            .or_else(|| {
                self.schema
                    .iter()
                    .find(|param| param.key == key)
                    .map(|param| param.default.clone())
            })
            .unwrap_or(ParamValue::Switch(false))
    }

    /// The bounds of the length `key`.
    pub(crate) fn length_range(&self, key: &str) -> (f64, f64) {
        match self
            .schema
            .iter()
            .find(|param| param.key == key)
            .map(|p| p.kind)
        {
            Some(ParamKind::Length { min, max, .. }) => (min.as_mm(), max.as_mm()),
            _ => (f64::MIN, f64::MAX),
        }
    }

    pub(crate) fn mm(&self, key: &str) -> f64 {
        self.value(key).length().map_or(0.0, Length::as_mm)
    }

    pub(crate) fn count(&self, key: &str) -> u32 {
        self.value(key).count().unwrap_or(0)
    }

    /// The whole percentage `key` (signed, `-30` to `30` for Bar width adjust).
    pub(crate) fn percent(&self, key: &str) -> i32 {
        self.value(key).percent().unwrap_or(0)
    }

    pub(crate) fn ratio(&self, key: &str) -> f64 {
        self.value(key).ratio().unwrap_or(0.0)
    }

    pub(crate) fn switch(&self, key: &str) -> bool {
        self.value(key).switch().unwrap_or(false)
    }

    /// The colour `key`; black for a missing one.
    pub(crate) fn colour(&self, key: &str) -> Color {
        self.value(key).colour().unwrap_or(Color::BLACK)
    }

    pub(crate) fn choice(&self, key: &str) -> String {
        self.value(key).choice().unwrap_or_default().to_owned()
    }

    pub(crate) fn text(&self, key: &str) -> String {
        self.value(key).text().unwrap_or_default().to_owned()
    }

    pub(crate) fn put(&mut self, key: &str, value: &ParamValue) {
        self.stored.insert(key.to_owned(), value.to_stored());
    }

    pub(crate) fn put_mm(&mut self, key: &str, mm: f64) {
        self.put(key, &ParamValue::Length(Length::from_mm(mm)));
    }

    pub(crate) fn finish(self) -> (StoredParams, Vec<Clamp>) {
        (self.stored, self.clamps)
    }
}
