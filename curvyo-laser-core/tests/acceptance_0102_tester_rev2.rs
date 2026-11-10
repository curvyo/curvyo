//! Independent acceptance tests for the revision 2 of `specs/0102-laser-fixtures` in
//! `curvyo-laser-core` (2026-11-10): the fifth kind `screw` is read from `curvyo.laser/screw`, and
//! the kinds stay case-sensitive and exact (ADR 0049 §1, criterion 32). Written from the
//! specification and ADR 0049 before the implementation was read for these points.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_laser_core::FixtureKind;

#[test]
fn ac2_37_the_screw_suffix_is_the_fifth_kind() {
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/screw"),
        FixtureKind::Screw
    );
}

#[test]
fn ac32_kinds_are_exact_and_any_other_suffix_is_unknown() {
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/Screw"),
        FixtureKind::Unknown,
        "the suffix is case-sensitive"
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/screws"),
        FixtureKind::Unknown
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/hex_screw"),
        FixtureKind::Unknown,
        "hex and countersunk are not kinds (customer, 2026-11-10)"
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/holder"),
        FixtureKind::Holder,
        "the four older kinds keep their reading"
    );
}
