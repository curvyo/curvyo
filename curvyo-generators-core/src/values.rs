//! The parameter values of one generation: read from the stored map, clamped by the schema first
//! and by the generator's dependent bounds second, with every change reported (permit over
//! refuse, ADR 0080 §2).

use curvyo_document_core::Length;
use curvyo_extension_core::{Bound, Clamp, Param, ParamKind, ParamValue, StoredParams};

/// The values of one generation and the clamps they went through.
pub(crate) struct Values<'a> {
    schema: &'a [Param],
    stored: StoredParams,
    clamps: Vec<Clamp>,
}

impl<'a> Values<'a> {
    /// Reads every parameter of `schema` from `params`. A missing or mistyped value reads as the
    /// default; a number outside the schema's bounds is clamped and reported.
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
                .and_then(|value| {
                    param
                        .accept(value.clone())
                        .map(|accepted| (value, accepted))
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
            (ParamValue::Angle(a), ParamValue::Angle(b)) => a.0 < b.0,
            _ => return,
        };
        self.set_clamp(
            param.key,
            accepted.clone(),
            if below {
                Bound::Minimum
            } else {
                Bound::Maximum
            },
        );
    }

    fn set_clamp(&mut self, key: &'static str, value: ParamValue, bound: Bound) {
        self.clamps.retain(|clamp| clamp.key != key);
        self.clamps.push(Clamp { key, value, bound });
    }

    fn param(&self, key: &str) -> Option<&Param> {
        self.schema.iter().find(|param| param.key == key)
    }

    fn value(&self, key: &str) -> ParamValue {
        self.stored
            .get(key)
            .and_then(ParamValue::from_stored)
            .or_else(|| self.param(key).map(|param| param.default.clone()))
            .unwrap_or(ParamValue::Switch(false))
    }

    pub(crate) fn mm(&self, key: &str) -> f64 {
        self.value(key).length().map_or(0.0, Length::as_mm)
    }

    pub(crate) fn count(&self, key: &str) -> u32 {
        self.value(key).count().unwrap_or(0)
    }

    pub(crate) fn degrees(&self, key: &str) -> f64 {
        self.value(key).degrees().unwrap_or(0.0)
    }

    pub(crate) fn switch(&self, key: &str) -> bool {
        self.value(key).switch().unwrap_or(false)
    }

    pub(crate) fn choice(&self, key: &str) -> String {
        self.value(key).choice().unwrap_or_default().to_owned()
    }

    /// Writes a length computed by the generator (a scale, a derived value) without a clamp
    /// report; the schema bounds still apply, so a later dependent clamp sees a valid number.
    pub(crate) fn put_mm(&mut self, key: &'static str, mm: f64) {
        let Some(param) = self.param(key) else {
            return;
        };
        let asked = ParamValue::Length(Length::from_mm(mm));
        let Some(accepted) = param.accept(asked.clone()) else {
            return;
        };
        if accepted != asked {
            let below = accepted.length().is_some_and(|l| l.as_mm() > mm);
            self.set_clamp(
                key,
                accepted.clone(),
                if below {
                    Bound::Minimum
                } else {
                    Bound::Maximum
                },
            );
        }
        self.stored.insert(key.to_owned(), accepted.to_stored());
    }

    /// Moves the length `key` into `min..=max` (a bound that depends on other parameters) and
    /// reports it as a dependent clamp when it moved. `max` wins when the bounds cross.
    pub(crate) fn limit_mm(&mut self, key: &'static str, min: f64, max: f64) {
        let current = self.mm(key);
        let limited = current.max(min).min(max);
        if (limited - current).abs() > 1e-12 {
            let value = ParamValue::Length(Length::from_mm(limited));
            self.stored.insert(key.to_owned(), value.to_stored());
            let bound = if limited > current {
                Bound::DependentMinimum
            } else {
                Bound::Dependent
            };
            self.set_clamp(key, value, bound);
        }
    }

    /// As `limit_mm` for a lower bound that a choice sets: the notice names `subject`.
    pub(crate) fn limit_min_for(&mut self, key: &'static str, min: f64, subject: &'static str) {
        let before = self.mm(key);
        self.limit_mm(key, min, f64::MAX);
        if self.mm(key) > before + 1e-12 {
            let value = ParamValue::Length(Length::from_mm(self.mm(key)));
            self.set_clamp(key, value, Bound::DependentMinimumFor(subject));
        }
    }

    /// The clamp bounds the schema gives the length `key`.
    pub(crate) fn schema_range(&self, key: &str) -> (f64, f64) {
        match self.param(key).map(|param| param.kind) {
            Some(ParamKind::Length { min, max, .. }) => (min.as_mm(), max.as_mm()),
            _ => (f64::MIN, f64::MAX),
        }
    }

    pub(crate) fn finish(self) -> (StoredParams, Vec<Clamp>) {
        (self.stored, self.clamps)
    }
}
