//! The factor of the shared CI runner for thread-CPU budget tests (Frage 368, CI-BUDGETS):
//! `CURVYO_BUDGET_FACTOR` (default 1.0, finite, at least 1.0) is read once, and the budget
//! clocks of this crate report CPU time divided by it, which is the same as every spec budget
//! multiplied by it (a ratio between two measurements is unchanged). Locally it stays unset and
//! the spec values apply as written.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::sync::OnceLock;
use std::time::Duration;

const VARIABLE: &str = "CURVYO_BUDGET_FACTOR";

/// The factor of `CURVYO_BUDGET_FACTOR`; panics on a value below 1.0, not finite or not a number.
pub fn budget_factor() -> f64 {
    static FACTOR: OnceLock<f64> = OnceLock::new();
    *FACTOR.get_or_init(|| {
        let Ok(text) = std::env::var(VARIABLE) else {
            return 1.0;
        };
        let factor: f64 = text
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{VARIABLE}={text:?} is not a number"));
        assert!(
            factor.is_finite() && factor >= 1.0,
            "{VARIABLE}={factor} must be finite and at least 1.0"
        );
        if factor > 1.0 {
            eprintln!(
                "{VARIABLE}={factor}: every effective budget is its spec value x {factor} \
                 (measured CPU times below count 1/{factor})"
            );
        }
        factor
    })
}

/// `cpu` as the budgets see it: the measured CPU time divided by the factor, so a spec budget
/// of `b` allows `b` x factor of real CPU time.
pub fn scaled(cpu: Duration) -> Duration {
    cpu.div_f64(budget_factor())
}
