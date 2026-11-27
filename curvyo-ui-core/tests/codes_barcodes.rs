//! The 1D codes through the host (`specs/0144` slice 2, ADR 0092): the glyph table of a real face,
//! the sentences, the switch between symbologies, the free resize with its fixed edge, the font
//! parameter through the funnel.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{Document, Length, NodeId, ObjectSnapshot, Point};
use curvyo_extension_core::{ClosedPath, ParamValue, Resize};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::ExtensionSettings;
use curvyo_ui_core::code_glyphs::{GlyphRegistry, glyph_table, set_active_glyphs};
use curvyo_ui_core::instance::{self, InstanceView};

const QR: &str = "curvyo.codes/qr";
const EAN13: &str = "curvyo.codes/ean-13";
const CODE128: &str = "curvyo.codes/code-128";
const UPCA: &str = "curvyo.codes/upc-a";

fn source_sans() -> Arc<curvyo_text_core::Face> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn with_included_sans() {
    set_active_glyphs(GlyphRegistry {
        default: Some(Arc::new(glyph_table(&source_sans()))),
        faces: HashMap::new(),
    });
}

fn placed(kind: &str) -> (Document, NodeId, ExtensionSettings) {
    with_included_sans();
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, kind, Point::new(50.0, 40.0)).unwrap();
    (document, id, settings)
}

fn text(content: &str) -> ParamValue {
    ParamValue::Text(content.to_owned())
}

fn view(document: &Document, settings: &ExtensionSettings, id: NodeId) -> InstanceView {
    instance::view(settings, &document.generated(id).unwrap())
}

fn problem(document: &Document, settings: &ExtensionSettings, id: NodeId) -> Option<String> {
    view(document, settings, id)
        .rows
        .into_iter()
        .find_map(|row| row.problem)
}

fn parts_bounds(document: &Document, id: NodeId) -> (f64, f64, f64, f64) {
    let snapshot = document.generated(id).unwrap();
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for numbers in snapshot.record.parts.iter().flatten() {
        let path = ClosedPath::from_flat(numbers).unwrap();
        for (x, y) in std::iter::once((path.start.x, path.start.y))
            .chain(path.segments.iter().map(|s| (s.to.x, s.to.y)))
        {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    b
}

#[test]
fn the_glyph_table_of_a_real_face_is_one_cap_height_with_the_winding_of_ink() {
    let table = glyph_table(&source_sans());
    assert_eq!(table.glyphs.len(), 95, "U+0020 to U+007E");
    let zero = &table.glyphs[&'0'];
    assert!(
        (zero.advance.as_mm() - 0.75).abs() < 0.1,
        "{}",
        zero.advance.as_mm()
    );
    // Two contours: the outer clockwise on the screen, the counter the other way.
    assert_eq!(zero.paths.len(), 2);
    let area = |path: &ClosedPath| {
        let mut points = vec![(path.start.x, path.start.y)];
        points.extend(path.segments.iter().map(|s| (s.to.x, s.to.y)));
        points
            .windows(2)
            .map(|p| p[0].0 * p[1].1 - p[1].0 * p[0].1)
            .sum::<f64>()
    };
    let (outer, counter) = if area(&zero.paths[0]).abs() > area(&zero.paths[1]).abs() {
        (&zero.paths[0], &zero.paths[1])
    } else {
        (&zero.paths[1], &zero.paths[0])
    };
    assert!(area(outer) > 0.0 && area(counter) < 0.0);
    // The top of a digit is about the cap height (1 unit) above the baseline.
    let top = zero
        .paths
        .iter()
        .flat_map(|p| p.segments.iter().map(|s| s.to.y))
        .fold(f64::MAX, f64::min);
    assert!((-top - 1.0).abs() < 0.1, "{top}");
    assert_eq!(table, glyph_table(&source_sans()), "equal twice");
}

#[test]
fn an_ean_13_draws_digits_in_the_one_part_and_its_text_size_fits_the_groups() {
    let (document, id, settings) = placed(EAN13);
    let snapshot = document.generated(id).unwrap();
    assert_eq!(snapshot.record.part_keys, ["code"]);
    // 30 bars and 13 digit outlines (the digits 1, 4 and others have one contour, 0 two, ...).
    assert!(snapshot.record.parts[0].len() > 30 + 13);
    assert_eq!(problem(&document, &settings, id), None);
    let json = serde_json::to_value(view(&document, &settings, id)).unwrap();
    assert_eq!(json["label"], "EAN-13");
    // Source Sans digits at 3 mm fit 7 modules of 0.33 mm: no clamp at the default.
    assert_eq!(snapshot.record.params["text_size"]["mm"], 3.0);
    let wide = instance::change_value(
        &document,
        &settings,
        id,
        "text_size",
        &ParamValue::Length(Length::from_mm(6.0)),
    )
    .unwrap();
    assert_eq!(wide.notices.len(), 1);
    assert!(
        wide.notices[0].starts_with("Text size set to 3.")
            && wide.notices[0].ends_with("mm, the most this module width allows."),
        "{:?}",
        wide.notices
    );
}

#[test]
fn the_sentences_of_the_refusals_follow_the_spec() {
    let (document, id, settings) = placed(EAN13);
    let before = document.generated(id).unwrap();
    for (content, sentence) in [
        ("", "EAN-13 takes 12 or 13 digits."),
        ("12345", "EAN-13 takes 12 or 13 digits."),
        ("20012345678x", "EAN-13 takes digits only."),
    ] {
        let changed =
            instance::change_value(&document, &settings, id, "content", &text(content)).unwrap();
        assert!(changed.refused && changed.notices.is_empty());
        assert_eq!(problem(&document, &settings, id).as_deref(), Some(sentence));
        assert_eq!(
            document.generated(id).unwrap().record.parts,
            before.record.parts
        );
    }
    let (document, id, settings) = placed(CODE128);
    for (content, sentence) in [
        ("", "Enter a text to encode."),
        (
            "caf\u{e9}",
            "Code 128 takes ASCII letters, digits and symbols only.",
        ),
    ] {
        instance::change_value(&document, &settings, id, "content", &text(content)).unwrap();
        assert_eq!(problem(&document, &settings, id).as_deref(), Some(sentence));
    }
    let long = "A".repeat(92);
    instance::change_value(&document, &settings, id, "content", &text(&long)).unwrap();
    assert_eq!(
        problem(&document, &settings, id).as_deref(),
        Some("Too long for Code 128: 92 characters, at most 80.")
    );
    let (document, id, settings) = placed("curvyo.codes/code-39");
    instance::change_value(&document, &settings, id, "content", &text("a_b")).unwrap();
    assert_eq!(
        problem(&document, &settings, id).as_deref(),
        Some("Code 39 takes A to Z, 0 to 9, space and - . $ / + % only.")
    );
}

#[test]
fn a_corrected_check_digit_is_a_notice_without_the_digit() {
    let (document, id, settings) = placed(EAN13);
    let changed =
        instance::change_value(&document, &settings, id, "content", &text("4006381333930"))
            .unwrap();
    assert_eq!(changed.notices, ["Check digit corrected."]);
    assert!(!changed.refused);
    assert_eq!(
        document.generated(id).unwrap().record.params["content"]["text"],
        "4006381333931"
    );
    assert_eq!(
        problem(&document, &settings, id),
        None,
        "not a lasting line"
    );
}

#[test]
fn the_names_keep_their_numbers_and_acronyms() {
    assert_eq!(instance::noun("Code 128"), "Code 128");
    assert_eq!(instance::noun("EAN-13"), "EAN-13");
    assert_eq!(instance::noun("QR code"), "QR code");
    assert_eq!(instance::noun("Rounded box"), "rounded box");
}

#[test]
fn a_switch_keeps_the_object_and_carries_what_the_maker_changed() {
    let (document, id, settings) = placed(QR);
    let frame = document.generated(id).unwrap().frame;
    let steps = document.own_step_count();
    // An untouched QR code becomes the EAN-13 placeholder.
    let switched = instance::switch_kind(&document, &settings, id, EAN13).unwrap();
    assert!(!switched.refused);
    let now = document.generated(id).unwrap();
    assert_eq!(now.record.kind, EAN13);
    assert_eq!(now.frame, frame);
    assert_eq!(now.record.params["content"]["text"], "2001234567893");
    assert_eq!(now.record.params["text"]["on"], true);
    assert_eq!(document.own_step_count(), steps + 1, "one step");
    // A changed module width and bar height cross between 1D codes; text off crosses as changed.
    instance::change_value(
        &document,
        &settings,
        id,
        "module_width",
        &ParamValue::Length(Length::from_mm(0.5)),
    )
    .unwrap();
    instance::change_value(&document, &settings, id, "text", &ParamValue::Switch(false)).unwrap();
    instance::switch_kind(&document, &settings, id, CODE128).unwrap();
    let now = document.generated(id).unwrap();
    assert_eq!(now.record.params["module_width"]["mm"], 0.5);
    assert_eq!(now.record.params["text"]["on"], false);
    assert_eq!(
        now.record.params["content"]["text"], "EXAMPLE-01",
        "the new symbology's placeholder"
    );
    // A typed content crosses and, when the target cannot encode it, keeps the old outline.
    instance::change_value(&document, &settings, id, "content", &text("Hello")).unwrap();
    let outline = document.generated(id).unwrap().record.parts;
    let refused = instance::switch_kind(&document, &settings, id, EAN13).unwrap();
    assert!(refused.refused);
    let now = document.generated(id).unwrap();
    assert_eq!(now.record.kind, EAN13);
    assert_eq!(now.record.parts, outline, "the old paths stay");
    assert_eq!(now.record.params["content"]["text"], "Hello");
    assert_eq!(
        problem(&document, &settings, id).as_deref(),
        Some("EAN-13 takes 12 or 13 digits.")
    );
    assert!(instance::is_refused(
        &settings,
        EAN13,
        &now.record.params,
        now.record.seed
    ));
    // Between QR and 1D only Content and Inverted cross.
    instance::change_value(&document, &settings, id, "content", &text("123456789012")).unwrap();
    instance::switch_kind(&document, &settings, id, QR).unwrap();
    let qr = document.generated(id).unwrap();
    assert_eq!(qr.record.params["content"]["text"], "1234567890128");
    assert_eq!(qr.record.params["module_size"]["mm"], 1.0);
    assert!(qr.record.params.get("module_width").is_none());
}

#[test]
fn a_switch_needs_the_same_family() {
    let (document, id, settings) = placed(QR);
    let other = instance::place(
        &document,
        &settings,
        "curvyo.generators/gear",
        Point::new(0.0, 0.0),
    )
    .unwrap();
    assert!(instance::switch_kind(&document, &settings, id, "curvyo.generators/gear").is_err());
    assert!(instance::switch_kind(&document, &settings, other, QR).is_err());
    assert!(instance::switch_kind(&document, &settings, id, "curvyo.codes/nope").is_err());
}

#[test]
fn the_family_and_the_resize_rule_come_from_the_manifest() {
    let settings = ExtensionSettings::default();
    let found = instance::find_kind(&settings, UPCA).unwrap();
    assert_eq!(found.kind.family.unwrap().segment, "Code");
    assert_eq!(found.kind.resize, Resize::Free);
    assert_eq!(
        instance::find_kind(&settings, QR).unwrap().kind.resize,
        Resize::Proportional
    );
}

fn stretched(document: &Document, id: NodeId, sx: f64, sy: f64, fixed: Point) -> ObjectSnapshot {
    // The outline as stored, scaled by (sx, sy) about `fixed` in document space.
    let object = document.object(id).unwrap();
    let ObjectSnapshot::Path(mut path) = object else {
        panic!("a path");
    };
    let map = |p: Point| {
        Point::new(
            fixed.x + (p.x - fixed.x) * sx,
            fixed.y + (p.y - fixed.y) * sy,
        )
    };
    let move_anchors = |anchors: &mut [curvyo_document_core::AnchorSnapshot]| {
        for a in anchors.iter_mut() {
            let p = map(a.point);
            a.handle_in = curvyo_document_core::Vec2::new(a.handle_in.x * sx, a.handle_in.y * sy);
            a.handle_out =
                curvyo_document_core::Vec2::new(a.handle_out.x * sx, a.handle_out.y * sy);
            a.point = p;
        }
    };
    move_anchors(&mut path.anchors);
    for sub in &mut path.extra_subpaths {
        move_anchors(&mut sub.anchors);
    }
    ObjectSnapshot::Path(path)
}

#[test]
fn an_edge_drag_writes_the_module_width_and_keeps_the_opposite_edge() {
    let (document, id, settings) = placed(EAN13);
    let before = document.generated(id).unwrap();
    let (x0, y0, x1, y1) = parts_bounds(&document, id);
    let frame = before.frame;
    // The right edge dragged out by 25 %: the left edge is the fixed point.
    let left = Point::new(frame.e + x0, frame.f + y0);
    let result = stretched(&document, id, 1.25, 1.0, left);
    instance::resize(&document, &settings, &result, "Scale").unwrap();
    let after = document.generated(id).unwrap();
    assert!((after.record.params["module_width"]["mm"].as_f64().unwrap() - 0.4125).abs() < 1e-9);
    assert_eq!(after.record.params["bar_height"]["mm"], 15.0, "kept");
    assert!((after.record.params["text_size"]["mm"].as_f64().unwrap() - 3.75).abs() < 1e-9);
    let (nx0, ny0, nx1, ny1) = parts_bounds(&document, id);
    let new_left = after.frame.e + nx0;
    assert!(
        (new_left - left.x).abs() < 1e-6,
        "the left edge stayed: {new_left}"
    );
    assert!(
        (nx1 - nx0 - (x1 - x0) * 1.25).abs() < 0.2,
        "about 25 % wider"
    );
    assert!(
        (after.frame.f + ny0 - (frame.f + y0)).abs() < 1e-6,
        "bars stay where they were"
    );
    let _ = (y1, ny1);
}

#[test]
fn a_bottom_edge_drag_writes_the_bar_height_and_keeps_the_top_edge() {
    let (document, id, settings) = placed(EAN13);
    let frame = document.generated(id).unwrap().frame;
    let (x0, y0, x1, y1) = parts_bounds(&document, id);
    let top = Point::new(frame.e + f64::midpoint(x0, x1), frame.f + y0);
    let result = stretched(&document, id, 1.0, 1.5, top);
    instance::resize(&document, &settings, &result, "Scale").unwrap();
    let after = document.generated(id).unwrap();
    assert_eq!(after.record.params["module_width"]["mm"], 0.33);
    let band = (y1 - y0) - 15.0;
    let expected = (y1 - y0) * 1.5 - band;
    assert!(
        (after.record.params["bar_height"]["mm"].as_f64().unwrap() - expected).abs() < 1e-6,
        "bar height {expected}"
    );
    let (_, ny0, _, ny1) = parts_bounds(&document, id);
    assert!(
        (after.frame.f + ny0 - (frame.f + y0)).abs() < 1e-6,
        "the top edge stayed"
    );
    assert!(
        ((ny1 - ny0) - (y1 - y0) * 1.5).abs() < 1e-6,
        "the box grew by half"
    );
}

#[test]
fn a_qr_code_still_resizes_proportionally_and_a_refused_code_does_not_resize() {
    let (document, id, settings) = placed(QR);
    let result = stretched(&document, id, 2.0, 2.0, Point::new(50.0, 40.0));
    instance::resize(&document, &settings, &result, "Scale").unwrap();
    assert_eq!(
        document.generated(id).unwrap().record.params["module_size"]["mm"],
        2.0
    );
    let (document, id, settings) = placed(EAN13);
    instance::change_value(&document, &settings, id, "content", &text("123")).unwrap();
    let result = stretched(&document, id, 1.5, 1.0, Point::new(50.0, 40.0));
    assert!(instance::plan_resize(&document, &settings, &result).is_err());
}

#[test]
fn the_font_parameter_reaches_the_generator_through_the_funnel() {
    let (document, id, settings) = placed(EAN13);
    let with_text = document.generated(id).unwrap().record.parts[0].len();
    // No table at all: the bars are drawn, the text is not, and it says so.
    set_active_glyphs(GlyphRegistry::default());
    instance::change_value(
        &document,
        &settings,
        id,
        "text_size",
        &ParamValue::Length(Length::from_mm(2.5)),
    )
    .unwrap();
    let bare = document.generated(id).unwrap().record.parts[0].len();
    assert_eq!(bare, 30);
    assert!(with_text > bare);
    with_included_sans();
    // A named face that is not in the registry uses the included sans.
    let face = curvyo_document_core::FontRef {
        family: "Gone Sans".into(),
        style: "Regular".into(),
        face_hash: "00".repeat(32),
        face_index: 0,
        kind: curvyo_document_core::FontKind::Outline,
    };
    instance::change_value(
        &document,
        &settings,
        id,
        "font",
        &ParamValue::Font(Some(face)),
    )
    .unwrap();
    assert!(document.generated(id).unwrap().record.parts[0].len() > 30);
    let stored = &document.generated(id).unwrap().record.params["font"]["font"];
    assert_eq!(
        stored["family"], "Gone Sans",
        "a fallback never rewrites the stored face"
    );
}
