//! Independent tester tests for `specs/0071-pattern-fill` at the document model (ADR 0029 §1, §2,
//! §5, §11, §12): format 32 and its golden, older and newer files, damaged pattern registers,
//! absent parameters, the kept pattern under a Colour paint, fresh and stored seeds, copies and
//! linked clones, edit validation and the history labels. Written from the specification and the
//! ADR; only public names and the stored register shape (seen through `export_json`) were looked
//! up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]
#![allow(clippy::cast_precision_loss)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CopySource, Document,
    FillPaint, GeneratorKind, Length, NewAnchor, NodeId, ObjectSnapshot, OpenError, PatternFill,
    PatternGenerator, PatternMode, PatternParam, PatternSeed, Point, RectBounds, StyleEdit,
    StyleEditError, Vec2, pack, pattern_params, unpack,
};
use loro::{LoroDoc, LoroValue};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn rect(d: &Document) -> NodeId {
    d.create_rect(RectBounds::from_corners(pt(0.0, 0.0), pt(60.0, 40.0)))
}

fn square_path(d: &Document, n: u64, x: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), pt(x, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n + 1), pt(x + 30.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n + 2), pt(x + 30.0, 30.0)),
            NewAnchor::corner(AnchorId::new(1, n + 3), pt(x, 30.0)),
        ],
        true,
    )
}

fn object(d: &Document, id: NodeId) -> ObjectSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find(|o| o.id() == id)
        .cloned()
        .expect("a live object")
}

fn pattern(d: &Document, id: NodeId) -> Option<PatternFill> {
    object(d, id).style().fill.painted_pattern()
}

fn stored(d: &Document, id: NodeId) -> Option<PatternFill> {
    object(d, id).style().fill.pattern.as_deref().copied()
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn member(zip_bytes: &[u8], name: &str) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
    let mut f = z.by_name(name).unwrap();
    let mut out = Vec::new();
    f.read_to_end(&mut out).unwrap();
    out
}

fn loro_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn file_from_loro(loro: &LoroDoc, version: u32) -> Vec<u8> {
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": version,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        w.start_file(name, o).unwrap();
        w.write_all(&bytes).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn tree_id(id: NodeId) -> loro::TreeID {
    let v = serde_json::to_value(id).unwrap();
    loro::TreeID::new(
        v["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(v["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn to_loro(v: &serde_json::Value) -> LoroValue {
    match v {
        serde_json::Value::Null => LoroValue::Null,
        serde_json::Value::Bool(b) => LoroValue::from(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| LoroValue::from(n.as_f64().unwrap()), LoroValue::from),
        serde_json::Value::String(s) => LoroValue::from(s.as_str()),
        serde_json::Value::Array(a) => LoroValue::from(a.iter().map(to_loro).collect::<Vec<_>>()),
        serde_json::Value::Object(m) => LoroValue::from(
            m.iter()
                .map(|(k, v)| (k.clone(), to_loro(v)))
                .collect::<std::collections::HashMap<_, _>>(),
        ),
    }
}

/// A patterned rectangle whose `key` register is overwritten with `value`, saved as `version`.
fn with_register(key: &str, value: LoroValue, version: u32) -> Result<Document, OpenError> {
    let d = Document::new(1);
    let id = rect(&d);
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let l = loro_of(&d);
    let meta = l.get_tree("paths").get_meta(tree_id(id)).unwrap();
    assert!(meta.get(key).is_some(), "register {key} is written");
    meta.insert(key, value).unwrap();
    unpack(2, &file_from_loro(&l, version))
}

fn pattern_value(v: &serde_json::Value) -> Result<Document, OpenError> {
    with_register("fill_pattern", to_loro(v), CURRENT_FORMAT_VERSION)
}

fn good() -> serde_json::Value {
    serde_json::json!({
        "generator": "hatch", "margin": 0.0, "mode": "lines", "seed": 5,
        "params": { "spacing": 2.0, "angle": 45.0 }
    })
}

// ---------- format 32: version, golden, round trip, older and newer files ----------

#[test]
fn format_version_is_32_and_a_saved_file_says_so() {
    // `0069` took 33 for tilings after this slice merged.
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 32);
    let d = Document::new(1);
    rect(&d);
    let m: serde_json::Value =
        serde_json::from_slice(&member(&pack(&d, "t").unwrap(), "manifest.json")).unwrap();
    assert_eq!(m["format_version"], CURRENT_FORMAT_VERSION);
}

#[test]
fn golden_v32_has_every_generator_both_modes_a_margin_and_a_holed_region_and_round_trips() {
    let d = unpack(2, &fixture("pattern_v32.curvyo")).unwrap();
    let read = d.read_objects();
    let patterns: Vec<(PatternFill, &ObjectSnapshot)> = read
        .objects
        .iter()
        .filter_map(|o| o.style().fill.painted_pattern().map(|p| (p, o)))
        .collect();
    for kind in GeneratorKind::ALL {
        assert!(
            patterns.iter().any(|(p, _)| p.generator.kind() == kind),
            "the golden has {kind:?}"
        );
    }
    assert!(patterns.iter().any(|(p, _)| p.mode == PatternMode::Lines));
    assert!(patterns.iter().any(|(p, _)| p.mode == PatternMode::Shapes));
    assert!(patterns.iter().any(|(p, _)| p.margin.as_mm() > 0.0));
    assert!(
        patterns.iter().any(|(_, o)| match o {
            ObjectSnapshot::Path(p) => p.is_compound(),
            _ => false,
        }),
        "a compound region with a hole"
    );
    // Save and reopen: every style is the same.
    let again = unpack(3, &pack(&d, "t").unwrap()).unwrap();
    let a: Vec<_> = read.objects.iter().map(|o| o.style().clone()).collect();
    let b: Vec<_> = again
        .read_objects()
        .objects
        .iter()
        .map(|o| o.style().clone())
        .collect();
    assert_eq!(a, b);
}

#[test]
fn save_and_reopen_keep_every_generator_parameter_mode_margin_and_seed() {
    let d = Document::new(1);
    let mut ids = Vec::new();
    for (i, kind) in GeneratorKind::ALL.into_iter().enumerate() {
        let id = square_path(&d, 10 * (i as u64 + 1), 40.0 * i as f64);
        d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
            .unwrap();
        d.edit_style(&[id], &StyleEdit::PatternGenerator(kind))
            .unwrap();
        // Every parameter off its default (inside its range).
        for spec in pattern_params(kind) {
            let v = if spec.max <= 1.0 {
                spec.default * 0.5 + spec.min * 0.5
            } else {
                spec.default + 1.0
            };
            let v = if spec.max <= 1000.0 && spec.max > 1.0 {
                spec.default.min(spec.max - 1.0) + 0.5
            } else {
                v
            };
            let v = if matches!(spec.param, PatternParam::Rings | PatternParam::Lines) {
                spec.default + 2.0
            } else {
                v
            };
            let r = d.edit_style(&[id], &StyleEdit::PatternParam(spec.param, v));
            assert!(r.is_ok(), "{kind:?} {} = {v}: {r:?}", spec.name);
        }
        d.edit_style(&[id], &StyleEdit::PatternMargin(Length::from_mm(0.75)))
            .unwrap();
        d.edit_style(&[id], &StyleEdit::PatternMode(PatternMode::Shapes))
            .unwrap();
        d.edit_style(&[id], &StyleEdit::PatternSeed(PatternSeed::new(u32::MAX)))
            .unwrap();
        ids.push(id);
    }
    let again = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    for id in ids {
        let before = pattern(&d, id).unwrap();
        assert_eq!(pattern(&again, id), Some(before), "round trip");
        assert_eq!(before.seed.get(), u32::MAX);
        assert_eq!(before.mode, PatternMode::Shapes);
        assert_eq!(before.margin.as_mm(), 0.75);
        assert_ne!(
            before.generator,
            PatternGenerator::defaults(before.generator.kind())
        );
    }
}

#[test]
fn older_files_open_without_patterns_and_a_newer_one_is_refused() {
    for name in [
        "dimension_v31.curvyo",
        "grid_v30.curvyo",
        "sketch_slot_v29.curvyo",
    ] {
        let d = unpack(2, &fixture(name)).unwrap();
        for o in d.read_objects().objects.iter() {
            assert_eq!(o.style().fill.paint, FillPaint::Colour, "{name}");
            assert!(o.style().fill.pattern.is_none(), "{name}");
        }
    }
    let d = Document::new(1);
    rect(&d);
    let newer = CURRENT_FORMAT_VERSION + 1;
    let bytes = file_from_loro(&loro_of(&d), newer);
    assert!(matches!(
        unpack(2, &bytes),
        Err(OpenError::FormatTooNew { found, supported })
            if found == newer && supported == CURRENT_FORMAT_VERSION
    ));
}

#[test]
fn a_file_without_the_pattern_keys_reads_as_colour() {
    let d = Document::new(1);
    let id = rect(&d);
    let again = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    let o = object(&again, id);
    let f = &o.style().fill;
    assert_eq!(f.paint, FillPaint::Colour);
    assert!(f.pattern.is_none());
    assert!(f.painted_pattern().is_none());
}

// ---------- damaged registers (ADR 0029 §2) ----------

#[test]
fn damaged_pattern_registers_are_refused() {
    let mut cases: Vec<(&str, serde_json::Value)> = Vec::new();
    let mut v = good();
    v["generator"] = "voronoi".into();
    cases.push(("unknown generator", v));
    let mut v = good();
    v["generator"] = 3.into();
    cases.push(("generator not a string", v));
    let mut v = good();
    v["params"]["spacing"] = "2".into();
    cases.push(("parameter of the wrong type", v));
    let mut v = good();
    v["params"]["spacing"] = 0.0.into();
    cases.push(("spacing below its minimum", v));
    let mut v = good();
    v["params"]["spacing"] = (-1.0).into();
    cases.push(("negative spacing", v));
    let mut v = good();
    v["seed"] = 3.5.into();
    cases.push(("seed not an integer", v));
    let mut v = good();
    v["seed"] = (-1).into();
    cases.push(("negative seed", v));
    let mut v = good();
    v["seed"] = 4_294_967_296_i64.into();
    cases.push(("seed above u32", v));
    let mut v = good();
    v["mode"] = "dots".into();
    cases.push(("unknown mode", v));
    let mut v = good();
    v["margin"] = (-1.0).into();
    cases.push(("negative margin", v));
    let mut v = good();
    v["margin"] = "1".into();
    cases.push(("margin of the wrong type", v));
    let mut v = good();
    v["whole_pieces"] = "yes".into();
    cases.push(("whole pieces not a bool", v));
    let mut v = good();
    v["whole_pieces"] = 1.into();
    cases.push(("whole pieces a number", v));
    cases.push(("pattern not a map", serde_json::json!("hatch")));
    for (what, value) in cases {
        assert!(
            matches!(pattern_value(&value), Err(OpenError::Damaged)),
            "{what} must be refused as damaged"
        );
    }
}

#[test]
fn non_finite_parameters_are_refused() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let d = Document::new(1);
        let id = rect(&d);
        d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
            .unwrap();
        let l = loro_of(&d);
        let meta = l.get_tree("paths").get_meta(tree_id(id)).unwrap();
        let mut params = std::collections::HashMap::new();
        params.insert("spacing".to_owned(), LoroValue::from(bad));
        params.insert("angle".to_owned(), LoroValue::from(45.0));
        let mut m = std::collections::HashMap::new();
        m.insert("generator".to_owned(), LoroValue::from("hatch"));
        m.insert("margin".to_owned(), LoroValue::from(0.0));
        m.insert("mode".to_owned(), LoroValue::from("lines"));
        m.insert("seed".to_owned(), LoroValue::from(5_i64));
        m.insert("params".to_owned(), LoroValue::from(params));
        meta.insert("fill_pattern", LoroValue::from(m)).unwrap();
        assert!(
            matches!(unpack(2, &file_from_loro(&l, 32)), Err(OpenError::Damaged)),
            "spacing {bad} must be refused"
        );
    }
}

#[test]
fn unknown_fill_paint_is_refused() {
    for v in [LoroValue::from("gradient"), LoroValue::from(1_i64)] {
        assert!(matches!(
            with_register("fill_paint", v, 32),
            Err(OpenError::Damaged)
        ));
    }
}

#[test]
fn an_absent_parameter_reads_as_its_frozen_default() {
    let v = serde_json::json!({
        "generator": "dots", "margin": 0.0, "mode": "shapes", "seed": 9, "params": {}
    });
    let d = pattern_value(&v).unwrap();
    let id = d.read_objects().objects[0].id();
    let p = pattern(&d, id).unwrap();
    assert_eq!(p.generator, PatternGenerator::defaults(GeneratorKind::Dots));
    assert_eq!(p.seed.get(), 9);
    let v = serde_json::json!({
        "generator": "waves", "margin": 0.0, "mode": "lines", "seed": 0,
        "params": { "amplitude": 0.0 }
    });
    let d = pattern_value(&v).unwrap();
    let id = d.read_objects().objects[0].id();
    let g = pattern(&d, id).unwrap().generator;
    assert_eq!(
        g.get(PatternParam::Amplitude),
        Some(0.0),
        "amplitude 0 is allowed"
    );
    assert_eq!(g.get(PatternParam::Wavelength), Some(10.0));
}

#[test]
fn a_cell_wall_not_smaller_than_its_cell_is_refused_on_open() {
    let v = serde_json::json!({
        "generator": "honeycomb", "margin": 0.0, "mode": "lines", "seed": 0,
        "params": { "cell_size": 2.0, "wall": 3.0 }
    });
    assert!(matches!(pattern_value(&v), Err(OpenError::Damaged)));
}

// ---------- criterion 1, 3: the paint and the kept pattern ----------

#[test]
fn c3_switching_to_colour_or_none_keeps_the_pattern_and_switching_back_restores_it() {
    let d = Document::new(1);
    let id = rect(&d);
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::PatternGenerator(GeneratorKind::Honeycomb),
    )
    .unwrap();
    d.edit_style(&[id], &StyleEdit::PatternParam(PatternParam::CellSize, 9.0))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::PatternMargin(Length::from_mm(2.0)))
        .unwrap();
    let before = pattern(&d, id).unwrap();
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Colour))
        .unwrap();
    assert!(pattern(&d, id).is_none(), "Colour paints no pattern");
    assert_eq!(stored(&d, id), Some(before), "kept while Colour");
    d.edit_style(&[id], &StyleEdit::FillEnabled(false)).unwrap();
    assert!(pattern(&d, id).is_none(), "None paints no pattern");
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    assert_eq!(pattern(&d, id), Some(before), "back with its parameters");
    // Undo restores exactly too.
    d.edit_style(&[id], &StyleEdit::FillEnabled(false)).unwrap();
    undo_last(&d);
    assert_eq!(pattern(&d, id), Some(before));
}

#[test]
fn c4_the_picker_lists_exactly_the_ten_generators_in_order() {
    let labels: Vec<&str> = GeneratorKind::ALL.iter().map(|k| k.label()).collect();
    assert_eq!(
        labels,
        [
            "Hatch",
            "Crosshatch",
            "Grid",
            "Dots",
            "Honeycomb",
            "Brick wall",
            "Leaf forest",
            "Sea waves",
            "Waves",
            "Interpolation waves"
        ]
    );
    for k in GeneratorKind::ALL {
        assert_eq!(GeneratorKind::from_id(k.id()), Some(k));
    }
}

#[test]
fn c12_interpolation_waves_has_a_line_count_only() {
    let p: Vec<PatternParam> = pattern_params(GeneratorKind::InterpolationWaves)
        .iter()
        .map(|s| s.param)
        .collect();
    assert_eq!(p, [PatternParam::Lines]);
}

#[test]
fn c8_brick_wall_parameters_are_width_height_joint() {
    let p: Vec<PatternParam> = pattern_params(GeneratorKind::BrickWall)
        .iter()
        .map(|s| s.param)
        .collect();
    assert_eq!(
        p,
        [
            PatternParam::BrickWidth,
            PatternParam::BrickHeight,
            PatternParam::Joint
        ]
    );
    let p: Vec<PatternParam> = pattern_params(GeneratorKind::LeafForest)
        .iter()
        .map(|s| s.param)
        .collect();
    assert_eq!(
        p,
        [
            PatternParam::LeafSize,
            PatternParam::SizeJitter,
            PatternParam::RotationJitter,
            PatternParam::Density
        ]
    );
}

// ---------- edit validation ----------

#[test]
fn bad_edits_write_nothing() {
    let d = Document::new(1);
    let id = rect(&d);
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let before = pattern(&d, id);
    let steps = d.own_steps().len();
    for edit in [
        StyleEdit::PatternParam(PatternParam::Spacing, 0.0),
        StyleEdit::PatternParam(PatternParam::Spacing, f64::NAN),
        StyleEdit::PatternParam(PatternParam::Diameter, 1.0),
        StyleEdit::PatternMargin(Length::from_mm(-1.0)),
        StyleEdit::PatternMargin(Length::from_mm(f64::INFINITY)),
    ] {
        let r = d.edit_style(&[id], &edit);
        assert!(
            matches!(
                r,
                Err(StyleEditError::InvalidPattern(_) | StyleEditError::InvalidMargin)
            ),
            "{edit:?}: {r:?}"
        );
    }
    assert_eq!(pattern(&d, id), before);
    assert_eq!(d.own_steps().len(), steps, "no step for a refused edit");
}

#[test]
fn switching_generator_carries_spacing_and_angle() {
    let d = Document::new(1);
    let id = rect(&d);
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::PatternParam(PatternParam::Spacing, 3.25))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::PatternParam(PatternParam::Angle, 12.0))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::PatternGenerator(GeneratorKind::Waves))
        .unwrap();
    let g = pattern(&d, id).unwrap().generator;
    assert_eq!(g.get(PatternParam::Spacing), Some(3.25));
    assert_eq!(g.get(PatternParam::Angle), Some(12.0));
}

// ---------- criterion 17: fresh, stored, re-rolled seeds ----------

#[test]
fn c17_a_new_pattern_gets_a_fresh_seed_that_is_stored() {
    let d = Document::new(1);
    let a = square_path(&d, 10, 0.0);
    let b = square_path(&d, 20, 40.0);
    d.edit_style(&[a, b], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let (sa, sb) = (pattern(&d, a).unwrap().seed, pattern(&d, b).unwrap().seed);
    assert_ne!(sa, sb, "two shapes get different fresh seeds");
    let again = unpack(7, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(
        pattern(&again, a).unwrap().seed,
        sa,
        "stored, not reshuffled"
    );
    // Re-roll gives a new seed; the same paint again keeps it.
    d.edit_style(&[a], &StyleEdit::PatternReroll).unwrap();
    let rolled = pattern(&d, a).unwrap().seed;
    assert_ne!(rolled, sa);
    d.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Colour))
        .unwrap();
    d.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    assert_eq!(pattern(&d, a).unwrap().seed, rolled);
}

#[test]
fn c17_the_fresh_seed_does_not_depend_on_the_peer_that_opens_the_file() {
    let d = Document::new(1);
    let a = square_path(&d, 10, 0.0);
    let bytes = pack(&d, "t").unwrap();
    let (x, y) = (unpack(5, &bytes).unwrap(), unpack(9, &bytes).unwrap());
    x.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    y.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    assert_eq!(pattern(&x, a).unwrap().seed, pattern(&y, a).unwrap().seed);
}

// ---------- criterion 20, 21: copies and clones ----------

#[test]
fn c20_a_copy_has_the_same_pattern_and_seed_and_editing_it_leaves_the_original() {
    let d = Document::new(1);
    let a = square_path(&d, 10, 0.0);
    d.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    d.edit_style(
        &[a],
        &StyleEdit::PatternGenerator(GeneratorKind::LeafForest),
    )
    .unwrap();
    let original = pattern(&d, a).unwrap();
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: (100..104).map(|n| AnchorId::new(1, n)).collect(),
            }],
            Vec2::new(50.0, 0.0),
        )
        .unwrap()[0];
    assert_eq!(pattern(&d, copy), Some(original), "same pattern and seed");
    d.edit_style(&[copy], &StyleEdit::PatternReroll).unwrap();
    d.edit_style(
        &[copy],
        &StyleEdit::PatternParam(PatternParam::Density, 0.2),
    )
    .unwrap();
    assert_eq!(pattern(&d, a), Some(original), "the original is untouched");
    assert_ne!(pattern(&d, copy), Some(original));
}

#[test]
fn c21_a_linked_clone_shows_its_sources_pattern_and_follows_edits() {
    let d = Document::new(1);
    let a = square_path(&d, 10, 0.0);
    d.edit_style(&[a], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let clone = d.linked_clone(&[a], &[]).unwrap()[0];
    assert_eq!(pattern(&d, clone), pattern(&d, a));
    d.edit_style(&[a], &StyleEdit::PatternGenerator(GeneratorKind::Dots))
        .unwrap();
    d.edit_style(&[a], &StyleEdit::PatternParam(PatternParam::Diameter, 1.75))
        .unwrap();
    assert_eq!(pattern(&d, clone), pattern(&d, a), "follows the source");
    assert_eq!(
        pattern(&d, clone).unwrap().generator.kind(),
        GeneratorKind::Dots
    );
}

// ---------- criterion 22: history labels and undo ----------

#[test]
fn c22_every_pattern_edit_is_one_step_with_its_own_label_and_undo_is_exact() {
    let d = Document::new(1);
    let id = rect(&d);
    let start = d.own_steps().len();
    let edits = [
        (StyleEdit::FillPaint(FillPaint::Pattern), "fill_pattern"),
        (
            StyleEdit::PatternGenerator(GeneratorKind::Crosshatch),
            "pattern_generator_crosshatch",
        ),
        (
            StyleEdit::PatternParam(PatternParam::SecondAngle, 100.0),
            "pattern_second_angle",
        ),
        (
            StyleEdit::PatternParam(PatternParam::Spacing, 1.5),
            "pattern_spacing",
        ),
        (
            StyleEdit::PatternMargin(Length::from_mm(1.0)),
            "pattern_margin",
        ),
        (
            StyleEdit::PatternMode(PatternMode::Shapes),
            "pattern_mode_shapes",
        ),
        (StyleEdit::PatternReroll, "pattern_seed"),
    ];
    let mut states = vec![(object(&d, id).style().clone())];
    for (edit, label) in &edits {
        d.edit_style(&[id], edit).unwrap();
        assert_eq!(labels(&d).last().unwrap(), label);
        states.push(object(&d, id).style().clone());
    }
    assert_eq!(d.own_steps().len(), start + edits.len(), "one step each");
    let steps: Vec<_> = d.own_steps()[start..].iter().map(|s| s.step).collect();
    for (step, want) in steps.iter().rev().zip(states.iter().rev().skip(1)) {
        d.undo_step(*step).unwrap();
        assert_eq!(object(&d, id).style(), want, "undo restores exactly");
    }
}
