//! `specs/0131` criteria 1 and 6: the simulator's `$130` and `$131` reach the Welcome's
//! controller settings as numbers.
#![allow(clippy::unwrap_used)]

mod common;

use common::grbl_bench::Bench;
use common::grbl_sim::SimConfig;
use curvyo_connector_core::Millimetres;
use curvyo_connector_io::controller::Controller;

fn travel_of(config: SimConfig) -> Option<[Millimetres; 2]> {
    let bench = Bench::connected(config);
    bench.ctrl.info().unwrap().settings.unwrap().travel
}

#[test]
fn the_reported_travel_is_in_the_welcome() {
    let travel = travel_of(SimConfig {
        travel: [400.0, 850.0],
        ..SimConfig::default()
    });
    assert_eq!(travel, Some([Millimetres(400.0), Millimetres(850.0)]));
}

#[test]
fn the_default_simulator_reports_400_by_300() {
    assert_eq!(
        travel_of(SimConfig::default()),
        Some([Millimetres(400.0), Millimetres(300.0)])
    );
}
