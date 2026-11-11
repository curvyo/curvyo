//! The document goldens of formats 40 and 41 read through the laser's typed reader (`specs/0102` criteria
//! 32 to 34): the damaged golden counts its text-in-a-number and skips its slot without a shape;
//! the unknown kind reads as Unknown; the `0093` grid keeps its lines.

#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use curvyo_document_core::{FixtureSnapshot, Length, OriginCorner, unpack};
use curvyo_laser_core::{FixtureKind, grid_spec, params_damage, read_slot};

fn fixtures(name: &str) -> Vec<FixtureSnapshot> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(1, &std::fs::read(path).unwrap())
        .unwrap()
        .manufacturings()[0]
        .fixtures
        .clone()
}

#[test]
fn the_damaged_golden_counts_one_more_value_and_skips_the_slot_without_a_shape() {
    let holder = &fixtures("fixtures_damaged_v40.curvyo")[0];
    let kind = FixtureKind::from_stored(&holder.kind);
    assert_eq!(
        params_damage(kind, &holder.params),
        1,
        "the text in a number"
    );
    let readable: Vec<_> = holder
        .slots
        .iter()
        .filter_map(|s| read_slot(&s.params))
        .collect();
    assert_eq!(readable.len(), 1, "the slot without a shape is skipped");
    let skipped = holder.slots.len() - readable.len();
    assert_eq!(
        holder.damaged + params_damage(kind, &holder.params) + skipped,
        5
    );
}

#[test]
fn an_unknown_kind_reads_as_unknown_and_its_neighbour_as_a_holder() {
    let read = fixtures("fixtures_unknown_kind_v40.curvyo");
    assert_eq!(
        FixtureKind::from_stored(&read[0].kind),
        FixtureKind::Unknown
    );
    assert_eq!(FixtureKind::from_stored(&read[1].kind), FixtureKind::Holder);
}

#[test]
fn the_format_39_grid_keeps_its_spacing_over_the_0090_corner() {
    let grid = &fixtures("fixtures_grid_v39.curvyo")[0];
    let (spec, damaged) = grid_spec(&grid.params, OriginCorner::BottomRight);
    assert_eq!(damaged, 0);
    assert_eq!(spec.spacing, Length::from_mm(10.0));
    assert_eq!(spec.corner, OriginCorner::BottomRight);
    assert!(spec.tapes.bottom && spec.tapes.top && spec.tapes.left && spec.tapes.right);
}

/// The `0103` goldens of format 41 read through the run rule (criteria 13, 17, 20 to 22).
fn job_reads(
    fixtures: &[FixtureSnapshot],
) -> Vec<(curvyo_laser_core::operations::FixtureJobRead, usize)> {
    use curvyo_laser_core::operations::{FixtureJobRead, KindRead};
    fixtures
        .iter()
        .flat_map(|fixture| {
            let kind = FixtureKind::from_stored(&fixture.kind);
            fixture.jobs.iter().map(move |job| {
                let ticked = job.targets.iter().filter(|t| t.ticked).map(|t| &t.target);
                let (parts, damaged) =
                    curvyo_laser_core::job_parts(kind, fixture.slots.len(), ticked);
                let read = FixtureJobRead {
                    kind: KindRead::of(job.kind.as_deref()),
                    output: job.output,
                    fixture_active: fixture.active,
                    parts,
                };
                (read, damaged)
            })
        })
        .collect()
}

#[test]
fn the_fixture_jobs_golden_runs_one_job_with_its_board_and_two_slots() {
    use curvyo_laser_core::FixturePart;
    use curvyo_laser_core::operations::fixture_block;
    let reads = job_reads(&fixtures("fixture_jobs_v41.curvyo"));
    let jobs: Vec<_> = reads.iter().map(|(read, _)| read.clone()).collect();
    assert_eq!(
        fixture_block(&jobs, true),
        [0],
        "Output off and inactive do not run"
    );
    assert_eq!(
        jobs[0].parts,
        [
            FixturePart::Board,
            FixturePart::Slot(0),
            FixturePart::Slot(2)
        ]
    );
    assert_eq!(jobs[1].parts, [FixturePart::GridLines]);
    assert!(reads.iter().all(|(_, damaged)| *damaged == 0));
}

#[test]
fn the_unknown_and_damaged_goldens_run_nothing_and_count_the_bad_suffix() {
    use curvyo_laser_core::operations::fixture_block;
    let unknown = job_reads(&fixtures("fixture_jobs_unknown_kind_v41.curvyo"));
    let jobs: Vec<_> = unknown.iter().map(|(read, _)| read.clone()).collect();
    assert!(fixture_block(&jobs, true).is_empty());
    let read = fixtures("fixture_jobs_damaged_v41.curvyo");
    let damaged = job_reads(&read);
    assert_eq!(damaged[0].1, 1, "the tapes of a holder");
    assert_eq!(read[0].damaged + damaged[0].1, 4, "the four damaged values");
    assert!(!damaged[0].0.runs(), "a text Output reads off");
}
