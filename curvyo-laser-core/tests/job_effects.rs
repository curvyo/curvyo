//! The effects of an export of `specs/0029-laser-job-preview-and-output` (criteria 28 to 30;
//! ADR 0064 §11): fixture jobs off, stored numbers refreshed, Lasered grids.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use curvyo_laser_core::job::{BlockSource, PlanCache, build_job, export_effects};
use curvyo_laser_core::operations::OperationKind::{Cut, Score};
use curvyo_laser_core::settings::SetKind;
use job_fixtures::{BIRCH, Page, leaf, material, own, plain, rect, settings};
use serde_json::json;

fn linked(power: f64, modified: u64) -> curvyo_laser_core::settings::SettingsRead {
    settings(
        Cut,
        json!({"main": {"power_pct": power, "speed_mm_min": 300, "passes": 1},
               "material": {"id": BIRCH, "name": "Birch 3 mm", "modified": modified},
               "overrides": {"main": {"passes": 2}}}),
    )
}

fn set_grid(page: &mut Page, index: usize, grid: curvyo_document_core::NodeId) {
    if let BlockSource::Operation { grid: g, .. } = &mut page.input.blocks[index].source {
        *g = Some(grid);
    }
}

#[test]
fn c28_fixture_jobs_go_off_linked_numbers_refresh_and_grids_that_ran_are_lasered() {
    let mut page = Page::new();
    let fixture = page.fixture(Score, vec![leaf(vec![rect(1.0, 1.0, 5.0, 5.0)])]);
    let second_job = page.id();
    if let BlockSource::Fixture { jobs } = &mut page.input.blocks[0].source {
        jobs.push(second_job);
    }
    let cut = page.operation(
        Cut,
        "Cut 1",
        linked(80.0, 9),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    page.operation(
        Cut,
        "Grid row",
        own(Cut, 50.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(30.0, 10.0, 10.0, 10.0)])],
    );
    page.operation(
        Cut,
        "Empty row",
        own(Cut, 50.0, 300, 1),
        plain(),
        Vec::new(),
    );
    let [grid, idle_grid] = [page.id(), page.id()];
    assert_ne!(grid, idle_grid);
    set_grid(&mut page, 2, grid);
    set_grid(&mut page, 3, idle_grid);
    page.input.fixture_material = Some(BIRCH.to_owned());
    let birch = material(
        BIRCH,
        "Birch 3 mm",
        &[(SetKind::Cut, 85.0, 250, 1), (SetKind::Score, 20.0, 900, 1)],
    );
    let materials = [birch];
    let built = build_job(&page.input, &materials, &mut PlanCache::default());
    let effects = export_effects(&page.input, &built, &materials);
    assert_ne!(fixture, second_job);
    assert_eq!(effects.fixture_jobs_off, [fixture, second_job]);
    assert_eq!(effects.refreshed.len(), 1, "only the linked operation");
    let (id, value) = &effects.refreshed[0];
    assert_eq!(*id, cut);
    assert_eq!(value.main.power.percent(), 85.0, "the material's numbers");
    assert!(!value.overrides.is_empty(), "the typed passes are kept");
    assert_eq!(
        effects.lasered_grids,
        [grid],
        "criterion 29: a grid whose operation wrote no move is not Lasered"
    );
}

#[test]
fn c28_a_refresh_ignores_the_link_time_guard_and_an_equal_link_writes_nothing() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        linked(85.0, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Cut, 85.0, 300, 1)]);
    let built = build_job(
        &page.input,
        std::slice::from_ref(&birch),
        &mut PlanCache::default(),
    );
    let same = export_effects(&page.input, &built, &[birch]);
    assert!(same.refreshed.is_empty(), "nothing differs");
    // The link is newer than the record (a lagging sync): the export still refreshes.
    page.input.blocks[0].source = BlockSource::Operation {
        settings: linked(70.0, 99),
        options: Ok(plain()),
        margin: curvyo_document_core::Length::from_mm(0.0),
        grid: None,
    };
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Cut, 85.0, 300, 1)]);
    let built = build_job(
        &page.input,
        std::slice::from_ref(&birch),
        &mut PlanCache::default(),
    );
    assert_eq!(
        export_effects(&page.input, &built, &[birch])
            .refreshed
            .len(),
        1
    );
}
