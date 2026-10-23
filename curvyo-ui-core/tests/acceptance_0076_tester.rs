//! Tester acceptance test for `specs/0076-single-line-fonts` criterion 5: the cuttable check on
//! the shipped single-line faces. Written from `specification.md` and the public names before
//! the implementation was read.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_ui_core::{
    BuiltInTag, CheckResult, FamilyFacts, FontLibrary, MemberSource, cuttable_pieces,
};

fn single_line(check: Option<CheckResult>) -> FamilyFacts {
    FamilyFacts {
        single_line: true,
        check,
        ..FamilyFacts::default()
    }
}

const SHIPPED: [&str; 4] = [
    "hershey-sans-1/HersheySans1.svg",
    "hershey-script-1/HersheyScript1.svg",
    "ems-readability/EMSReadability.svg",
    "ems-allure/EMSAllure.svg",
];

/// Criterion 5: the check gives 0 pieces for every shipped single-line face, a single-line
/// family is never Cuttable automatically (not even with a stale one-piece result), and the
/// maker can add Cuttable by hand.
#[test]
fn ac5_cuttable_check_skips_single_line_faces() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    for file in SHIPPED {
        let bytes = std::fs::read(dir.join(file)).unwrap();
        assert_eq!(cuttable_pieces(&bytes, 0).unwrap(), 0, "{file}");
    }
    let mut lib = FontLibrary::default();
    for check in [
        None,
        Some(CheckResult::Pieces(0)),
        Some(CheckResult::Pieces(1)),
    ] {
        let facts = single_line(check);
        assert!(
            !lib.membership(BuiltInTag::Cuttable, "hershey", &facts)
                .member,
            "{check:?}"
        );
    }
    let facts = single_line(Some(CheckResult::Pieces(0)));
    lib.add_tag("hershey", "Cuttable", &facts).unwrap();
    let m = lib.membership(BuiltInTag::Cuttable, "hershey", &facts);
    assert!(m.member);
    assert_eq!(m.source, MemberSource::ByYou);
}
