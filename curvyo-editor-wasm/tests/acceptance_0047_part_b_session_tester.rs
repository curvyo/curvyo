//! Independent tester acceptance tests for the Rust side of Part B of
//! `specs/0047-transform-polish/specification.md` (criteria 17, 19, 20, 22): what the ratio
//! sliders write through `Session` (one commit for all selected stars, "Mixed", Escape drops the
//! preview, an equal value writes nothing), and the Select bar's "Object to path" flag of
//! criterion 11/13. The pointer gestures themselves live in the DOM and are covered by the
//! node tests in `frontend/tests/acceptance_0047_sliders_tester.test.ts`.

#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{
    Angle, Document, InnerRatio, Length, ObjectSnapshot, Point, PointCount, RectBounds, Shape,
    StarFrame, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::BarValue;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn objects(s: &Session) -> Vec<ObjectSnapshot> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect()
}

fn loro_of(d: &Document) -> loro::LoroDoc {
    let l = loro::LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn change_count(s: &Session) -> usize {
    loro_of(&doc_of(s)).len_changes()
}

fn bytes_of(s: &Session) -> Vec<u8> {
    doc_of(s).export_loro_snapshot().unwrap()
}

fn key(s: &mut Session, key: &str) -> KeyOutcome {
    s.key_down(KeyInput {
        key,
        ..KeyInput::default()
    })
}

fn select_all(s: &mut Session) {
    key(s, "Escape");
    let out = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::SelectedAll);
}

fn stars(ratios: &[f64]) -> Document {
    let d = Document::new(1);
    for (i, r) in ratios.iter().enumerate() {
        let _ = d.create_star(
            StarFrame {
                center: pt(50.0 + 40.0 * i as f64, 60.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(-FRAC_PI_2),
            },
            PointCount::new(5).unwrap(),
            InnerRatio::new(*r).unwrap(),
        );
    }
    d
}

fn ratios_of(s: &Session) -> Vec<f64> {
    objects(s)
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Star { inner_ratio, .. } => Some(inner_ratio.get()),
                _ => None,
            },
            ObjectSnapshot::Path(_) => None,
        })
        .collect()
}

fn ratio_i(x: f64) -> InnerRatio {
    InnerRatio::new(x).unwrap()
}

/// Criterion 20: stars with different ratios: the bar says Mixed; a click on the track
/// (`set_selected_ratio`) sets all selected stars to the clicked value in one commit.
#[test]
fn ac20_mixed_ratio_click_sets_all_in_one_commit() {
    let mut s = open(&stars(&[0.30, 0.45, 0.60]));
    select_all(&mut s);
    assert_eq!(s.select_bar_state().ratio, Some(BarValue::Mixed));
    let c = change_count(&s);
    s.set_selected_ratio(ratio_i(0.50));
    assert_eq!(change_count(&s), c + 1, "one commit");
    for r in ratios_of(&s) {
        assert!((r - 0.50).abs() < 1e-9, "ratio {r}");
    }
    assert_eq!(s.select_bar_state().ratio, Some(BarValue::Uniform(0.50)));
}

/// Criterion 17: a click that gives the value the slider already has writes nothing.
#[test]
fn ac17_an_equal_value_writes_nothing() {
    let mut s = open(&stars(&[0.45, 0.45]));
    select_all(&mut s);
    let before = bytes_of(&s);
    let c = change_count(&s);
    s.set_selected_ratio(ratio_i(0.45));
    assert_eq!(change_count(&s), c, "no commit");
    assert!(bytes_of(&s) == before);
}

/// Criterion 17 / 19: a drag previews per tick and commits once on release; Escape drops the
/// preview and the release writes nothing.
#[test]
fn ac19_drag_previews_then_commits_once_escape_drops_it() {
    let mut s = open(&stars(&[0.30, 0.45]));
    select_all(&mut s);
    let before = bytes_of(&s);
    let c = change_count(&s);
    for v in [0.35, 0.4, 0.55, 0.7] {
        s.preview_selected_ratio(ratio_i(v));
        assert!(bytes_of(&s) == before, "{v}: previews write nothing");
    }
    s.commit_selected_ratio();
    assert_eq!(change_count(&s), c + 1, "one commit for the whole drag");
    for r in ratios_of(&s) {
        assert!((r - 0.7).abs() < 1e-9);
    }
    // Escape: the preview is dropped, the release writes nothing.
    let mut s = open(&stars(&[0.30, 0.45]));
    select_all(&mut s);
    let before = bytes_of(&s);
    s.preview_selected_ratio(ratio_i(0.9));
    s.cancel_selected_ratio();
    s.commit_selected_ratio();
    assert!(bytes_of(&s) == before, "Escape: nothing written");
    assert_eq!(s.select_bar_state().ratio, Some(BarValue::Mixed));
}

/// Criterion 22 / 17: the end values of the range and the rounding the slider produces are accepted
/// by the session (0.01 and 0.99), and out-of-range construction is refused by the type.
#[test]
fn ac22_range_ends() {
    assert!(InnerRatio::new(0.01).is_ok());
    assert!(InnerRatio::new(0.99).is_ok());
    assert!(InnerRatio::new(0.0).is_err());
    assert!(InnerRatio::new(1.0).is_err());
    assert!(InnerRatio::new(f64::NAN).is_err());
    let mut s = open(&stars(&[0.45]));
    select_all(&mut s);
    s.set_selected_ratio(ratio_i(0.01));
    assert!((ratios_of(&s)[0] - 0.01).abs() < 1e-9);
    s.set_selected_ratio(ratio_i(0.99));
    assert!((ratios_of(&s)[0] - 0.99).abs() < 1e-9);
}

/// Criterion 30: no stars selected: the slider is not shown (the bar has no ratio).
#[test]
fn ac30_no_stars_no_ratio() {
    let d = Document::new(1);
    let _ = d.create_rect(RectBounds {
        origin: pt(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let mut s = open(&d);
    assert_eq!(s.select_bar_state().ratio, None);
    select_all(&mut s);
    assert_eq!(s.select_bar_state().ratio, None);
}

/// Criteria 11, 13: "Object to path" is shown while the selection holds a primitive and not
/// otherwise; after a skew converted everything it is gone; after a stretch that converted only
/// the polygon but not the aligned rectangle it is still there.
#[test]
fn ac11_ac13_object_to_path_flag_follows_the_primitives_left() {
    let make = || {
        let d = Document::new(1);
        let _ = d.create_polygon(
            StarFrame {
                center: pt(100.0, 100.0),
                radius: Length::from_mm(8.0),
                angle: Angle::from_radians(-FRAC_PI_2),
            },
            PointCount::new(5).unwrap(),
        );
        let _ = d.create_rect(RectBounds {
            origin: pt(130.0, 94.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(12.0),
        });
        d
    };
    let mut s = open(&make());
    select_all(&mut s);
    assert!(s.select_bar_state().object_to_path);
    // Skew: everything converts.
    assert_eq!(key(&mut s, "k"), KeyOutcome::EntryOpened);
    s.commit_transform_entry("20", "", 0);
    assert!(objects(&s).iter().all(|o| matches!(o, ObjectSnapshot::Path(_))));
    assert!(!s.select_bar_state().object_to_path, "all paths: no button");
    // Stretch: only the polygon converts, the aligned rectangle stays.
    let mut s = open(&make());
    select_all(&mut s);
    key(&mut s, "s");
    let v = s.transform_entry().unwrap();
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    s.commit_transform_entry(&format!("{}", w * 2.0), &format!("{h}"), 0);
    let os = objects(&s);
    assert!(matches!(os[0], ObjectSnapshot::Path(_)));
    assert!(matches!(os[1], ObjectSnapshot::Primitive(_)));
    assert!(s.select_bar_state().object_to_path, "a primitive is left in the selection");
}
