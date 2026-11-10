//! The document goldens of format 40 read through the laser's typed reader (`specs/0102` criteria
//! 32 to 34): the damaged golden counts its text-in-a-number and skips its slot without a shape;
//! the unknown kind reads as Unknown; the `0093` grid keeps its lines.

#![allow(clippy::unwrap_used)]

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
