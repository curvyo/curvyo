//! Independent tester tests for `specs/0024-svg-import-export` slice 4 (SVG export) at the editor
//! session: the Export SVG command and its result (criterion 4), the notices (148), the options
//! with the selection and the page grid (131, 145, 147), texts in their faces (132 to 136, 146),
//! text stability (152), sketches (138), safety (157) and the budgets (71 export part, 154).
//! Written from the specification before the implementation was read; only the names of the
//! public seams were looked up.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod cpu_time;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Document, FontKind, MISSING_FACE_HASH, ObjectSnapshot, Style, TextSnapshot, Timestamp, pack,
    unpack,
};
use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_sketch_core as sk;
use curvyo_ui_core::{
    FontBlock, NOTHING_SELECTED_NOTICE, SvgExportOptions, svg_export_name, svg_export_notice,
    svg_export_notice_ms,
};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_960_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn included_faces() -> Vec<(String, String, String, u32, FontKind)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = if f["kind"].as_str() == Some("single_line") {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

fn load(s: &mut Session) -> usize {
    let wanted = s.wanted_font_files();
    for file in &wanted {
        let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
        assert!(s.add_font_file(file, &bytes), "{file} loads");
    }
    wanted.len()
}

fn prepare(s: &mut Session) {
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(), Vec::new());
}

/// Open… of an SVG with the host's loop.
fn open(bytes: &[u8]) -> Session {
    let mut s = Session::open_file(32, bytes, "poster.svg").unwrap();
    prepare(&mut s);
    for _ in 0..8 {
        load(&mut s);
        if !s.continue_open_svg(false) {
            break;
        }
    }
    for _ in 0..4 {
        if load(&mut s) == 0 {
            break;
        }
    }
    s
}

fn open_project(d: &Document) -> Session {
    let bytes = pack(d, "tester").unwrap();
    let mut s = Session::open(33, &bytes).unwrap();
    prepare(&mut s);
    for _ in 0..4 {
        if load(&mut s) == 0 {
            break;
        }
    }
    s
}

struct Out {
    text: String,
    objects: usize,
    kept: usize,
}

fn export(s: &Session, options: SvgExportOptions) -> Out {
    match s.export_svg(options) {
        SvgExportResult::Written {
            text,
            objects,
            texts_kept_live,
        } => Out {
            text,
            objects,
            kept: texts_kept_live,
        },
        SvgExportResult::NothingSelected => panic!("nothing selected"),
    }
}

fn plain(s: &Session) -> String {
    export(s, SvgExportOptions::default()).text
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn texts(d: &Document) -> Vec<TextSnapshot> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Text(t)) => Some(*t),
            _ => None,
        })
        .collect()
}

fn text_with(d: &Document, string: &str) -> TextSnapshot {
    texts(d)
        .into_iter()
        .find(|t| t.string == string)
        .unwrap_or_else(|| {
            let all: Vec<String> = texts(d).into_iter().map(|t| t.string).collect();
            panic!("a text {string:?} among {all:?}")
        })
}

/// Start tags as (name, attribute text).
fn tags(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('!') {
            continue;
        }
        let end = rest.find('>').unwrap();
        let inner = rest[..end].trim_end_matches('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace())
            .unwrap_or(inner.len());
        out.push((inner[..name_end].to_owned(), inner[name_end..].to_owned()));
        rest = &rest[end..];
    }
    out
}

fn attr(raw: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let at = raw.find(&key)? + key.len();
    let end = raw[at..].find('"')?;
    Some(raw[at..at + end].to_owned())
}

fn of_kind(text: &str, kind: &str) -> Vec<String> {
    tags(text)
        .into_iter()
        .filter(|(n, _)| n == kind)
        .map(|(_, r)| r)
        .collect()
}

/// The `<text>` element (its whole source) whose content holds `needle`.
fn text_element<'a>(svg: &'a str, needle: &str) -> &'a str {
    let mut from = 0;
    while let Some(at) = svg[from..].find("<text ") {
        let start = from + at;
        let end = start + svg[start..].find("</text>").unwrap() + 7;
        if svg[start..end].contains(needle) {
            return &svg[start..end];
        }
        from = end;
    }
    panic!("a <text> holding {needle:?} in\n{svg}")
}

const TEXTS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 210 297">
  <g id="l1"><title>x</title>
  <text x="20" y="30" font-family="Source Sans 3" font-weight="700" font-size="10.5838">Boho</text>
  <text x="100" y="60" font-family="Source Sans 3" font-size="6" text-anchor="middle" letter-spacing="0.5"><tspan x="100" y="60">One</tspan><tspan x="100" y="68">Two &amp; &lt;3&gt;</tspan><tspan x="100" y="76">Three</tspan></text>
  <text x="20" y="100" font-family="Lobster" font-size="8">Lobster missing</text>
  <text x="20" y="130" font-family="Hershey Sans 1-stroke" font-size="8" fill="none" stroke="#000000" stroke-width="0.2">Stick</text>
  <text x="20" y="200" font-family="Source Sans 3" font-style="italic" font-weight="300" font-size="6" xml:space="preserve" transform="rotate(20 20 200)" fill="#00ff00" stroke="#ff0000" stroke-width="0.3">Rot  two</text>
  </g>
</svg>"##;

// ---------------------------------------------------------------------------------------------
// Criterion 4, 147, 148: the command, the project unchanged, names and notices
// ---------------------------------------------------------------------------------------------

#[test]
fn c4_export_changes_nothing_in_the_project() {
    let mut s = open(TEXTS_SVG.as_bytes());
    s.mark_saved();
    let dirty = s.is_dirty();
    let history = s.history_len();
    let before = s.pack("0.1.0").unwrap();
    let a = plain(&s);
    for options in [
        SvgExportOptions {
            text_as_paths: true,
            ..SvgExportOptions::default()
        },
        SvgExportOptions {
            strokes_as_paths: true,
            ..SvgExportOptions::default()
        },
        SvgExportOptions {
            active_page_only: true,
            ..SvgExportOptions::default()
        },
    ] {
        let _ = export(&s, options);
    }
    assert_eq!(s.is_dirty(), dirty, "not dirty");
    assert_eq!(s.history_len(), history, "no history step");
    assert_eq!(s.pack("0.1.0").unwrap(), before, "the project is unchanged");
    assert_eq!(plain(&s), a, "same bytes on the second export");
}

#[test]
fn c4_default_names_and_c148_notices() {
    assert_eq!(
        svg_export_name(Some("poster.svg"), Some("/home/u/p.curvyo")),
        "poster.svg"
    );
    assert_eq!(
        svg_export_name(None, Some("My project.curvyo")),
        "My project.svg"
    );
    assert_eq!(svg_export_name(None, None), "Untitled.svg");
    assert_eq!(
        svg_export_notice("poster.svg", false, 3, 0),
        "Exported poster.svg."
    );
    assert_eq!(
        svg_export_notice("poster.svg", true, 3, 0),
        "Downloaded poster.svg."
    );
    assert_eq!(
        svg_export_notice("Untitled.svg", false, 0, 0),
        "Exported Untitled.svg. The drawing is empty."
    );
    assert_eq!(
        svg_export_notice("poster.svg", false, 5, 2),
        "Exported poster.svg. 2 texts kept as text because their fonts are missing."
    );
    assert_eq!(
        svg_export_notice("poster.svg", true, 5, 1),
        "Downloaded poster.svg. 1 text kept as text because its font is missing."
    );
    assert_eq!(svg_export_notice_ms(0), 3000);
    assert_eq!(svg_export_notice_ms(1), 6000);
    assert_eq!(NOTHING_SELECTED_NOTICE, "Nothing is selected.");
}

#[test]
fn c131_selection_only_with_nothing_selected_writes_no_file() {
    let s = open(TEXTS_SVG.as_bytes());
    assert!(matches!(
        s.export_svg(SvgExportOptions {
            selection_only: true,
            ..SvgExportOptions::default()
        }),
        SvgExportResult::NothingSelected
    ));
}

#[test]
fn c4_c148_empty_document_counts_zero_objects() {
    let s = open_project(&Document::new(5));
    let out = export(&s, SvgExportOptions::default());
    assert_eq!(out.objects, 0);
    assert!(Session::open_file(1, out.text.as_bytes(), "a.svg").is_ok());
    // An empty active page with content elsewhere: also "empty".
    let s = open(TEXTS_SVG.as_bytes());
    let out = export(
        &s,
        SvgExportOptions {
            active_page_only: true,
            ..SvgExportOptions::default()
        },
    );
    assert!(out.objects > 0);
}

#[test]
fn c145_grid_and_origin_corner_change_nothing() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 210 297"><rect x="10" y="20" width="30" height="40" fill="none" stroke="#000"/></svg>"##;
    let mut s = open(svg);
    let a = plain(&s);
    assert!(a.contains(" x=\"10\" y=\"20\""), "{a}");
    assert!(s.set_active_page_origin("bottom_left"));
    let _ = s.set_active_page_grid_shown(true);
    let b = plain(&s);
    let _ = s.set_active_page_grid_shown(false);
    let c = plain(&s);
    assert_eq!(a, b, "origin corner and grid are not exported");
    assert_eq!(a, c);
}

// ---------------------------------------------------------------------------------------------
// Criteria 132 to 136: texts
// ---------------------------------------------------------------------------------------------

#[test]
fn c132_live_text_with_face_size_and_shadow_data() {
    let s = open(TEXTS_SVG.as_bytes());
    let d = doc_of(&s);
    let boho = text_with(&d, "Boho");
    let svg = plain(&s);
    let el = text_element(&svg, ">Boho<");
    let (_, raw) = &tags(el)[0];
    assert!(attr(raw, "id").unwrap().starts_with("text"), "{el}");
    assert_eq!(attr(raw, "xml:space").as_deref(), Some("preserve"));
    let family = attr(raw, "font-family").unwrap();
    assert!(
        family == "'Source Sans 3'" || family == "Source Sans 3",
        "family {family} (the rule asks for quotes with a space, the example has none)"
    );
    assert_eq!(attr(raw, "font-weight").as_deref(), Some("700"));
    assert_eq!(attr(raw, "font-style").as_deref(), Some("normal"));
    assert_eq!(attr(raw, "font-stretch"), None, "normal width: omitted");
    let fs: f64 = attr(raw, "font-size").unwrap().parse().unwrap();
    assert!((fs - 10.5838).abs() < 1e-4, "font-size {fs}");
    assert_eq!(
        attr(raw, "curvyo:size"),
        Some(format!("{}", boho.size.as_mm()))
    );
    let face = attr(raw, "curvyo:face").unwrap();
    let parts: Vec<&str> = face.split('|').collect();
    assert_eq!(parts.len(), 3, "{face}");
    assert_eq!(parts[0], "Source Sans 3");
    assert_eq!(parts[1], "Bold");
    assert_eq!(parts[2].len(), 64);
    assert!(parts[2].bytes().all(|b| b.is_ascii_hexdigit()));
    let tspans = of_kind(el, "tspan");
    assert_eq!(tspans.len(), 1);
    assert_eq!(attr(&tspans[0], "sodipodi:role").as_deref(), Some("line"));
    assert!(svg.contains("xmlns:sodipodi"));
    // Read back: same string, same face reference exactly, Size within 1e-6 mm.
    let back = doc_of(&open(svg.as_bytes()));
    let b = text_with(&back, "Boho");
    assert_eq!(b.font, boho.font, "face reference exactly");
    assert!((b.size.as_mm() - boho.size.as_mm()).abs() < 1e-6);
}

/// The example of criterion 132: "Boho", Left, anchor (20, 30) is
/// `<text … x="20" y="30" …><tspan sodipodi:role="line" x="20" y="30">Boho</tspan>`.
#[test]
fn c132_example_positions_without_transform() {
    let s = open(TEXTS_SVG.as_bytes());
    let svg = plain(&s);
    let el = text_element(&svg, ">Boho<");
    let raw = &tags(el)[0].1;
    assert_eq!(
        attr(raw, "transform"),
        None,
        "unrotated text: no transform (example of criterion 132): {el}"
    );
    assert_eq!(attr(raw, "x").as_deref(), Some("20"));
    assert_eq!(attr(&of_kind(el, "tspan")[0], "y").as_deref(), Some("30"));
}

#[test]
fn c133_lines_alignment_spacing_and_escaping() {
    let s = open(TEXTS_SVG.as_bytes());
    let svg = plain(&s);
    let el = text_element(&svg, ">One<");
    let (_, raw) = &tags(el)[0];
    assert_eq!(attr(raw, "text-anchor").as_deref(), Some("middle"));
    let ls: f64 = attr(raw, "letter-spacing").unwrap().parse().unwrap();
    assert!((ls - 0.5).abs() < 1e-6);
    let lines = of_kind(el, "tspan");
    assert_eq!(lines.len(), 3, "{el}");
    assert!(
        el.contains(">Two &amp; &lt;3&gt;<"),
        "escaped content: {el}"
    );
    // The line anchor in document millimetres: the text's own transform applied to (x, y).
    let tf = attr(raw, "transform").unwrap_or_else(|| "matrix(1 0 0 1 0 0)".to_owned());
    let m: Vec<f64> = tf
        .trim_start_matches("matrix(")
        .trim_end_matches(')')
        .split_whitespace()
        .map(|t| t.parse().unwrap())
        .collect();
    for l in &lines {
        assert_eq!(attr(l, "sodipodi:role").as_deref(), Some("line"));
        let (x, y): (f64, f64) = (
            attr(l, "x").unwrap().parse().unwrap(),
            attr(l, "y").unwrap().parse().unwrap(),
        );
        let doc_x = m[0] * x + m[2] * y + m[4];
        assert!(
            (doc_x - 100.0).abs() < 1e-6,
            "criterion 133 (a): anchor x of a centred line is 100, got {doc_x} ({tf}, x={x})"
        );
    }
    let ys: Vec<f64> = lines
        .iter()
        .map(|l| attr(l, "y").unwrap().parse().unwrap())
        .collect();
    assert!(
        (ys[0] - 60.0).abs() < 1e-3 && ys[1] > ys[0] && ys[2] > ys[1],
        "{ys:?}"
    );
    // Runs of spaces kept.
    assert!(svg.contains(">Rot  two<"), "spaces kept with xml:space");
    let rot = text_element(&svg, "Rot  two");
    assert!(
        attr(&tags(rot)[0].1, "transform")
            .unwrap()
            .starts_with("matrix(")
    );
    assert_eq!(
        attr(&tags(rot)[0].1, "font-style").as_deref(),
        Some("italic")
    );
    assert_eq!(attr(&tags(rot)[0].1, "font-weight").as_deref(), Some("300"));
    // Lengths at most 6 decimals.
    for (_, r) in tags(el) {
        for key in ["x", "y", "dx", "dy", "font-size", "letter-spacing"] {
            if let Some(v) = attr(&r, key) {
                for t in v.split_whitespace() {
                    let dec = t.split_once('.').map_or(0, |(_, f)| f.len());
                    assert!(dec <= 6, "{key}={t}");
                }
            }
        }
    }
}

#[test]
fn c135_missing_face_is_written_and_comes_back_missing() {
    let s = open(TEXTS_SVG.as_bytes());
    let d = doc_of(&s);
    let lob = text_with(&d, "Lobster missing");
    assert_eq!(lob.font.face_hash, MISSING_FACE_HASH);
    let svg = plain(&s);
    let el = text_element(&svg, "Lobster missing");
    let raw = &tags(el)[0].1;
    assert!(attr(raw, "font-family").unwrap().contains("Lobster"));
    let face = attr(raw, "curvyo:face").unwrap();
    assert!(face.ends_with(&"0".repeat(64)), "{face}");
    let back = doc_of(&open(svg.as_bytes()));
    assert_eq!(text_with(&back, "Lobster missing").font, lob.font);
}

#[test]
fn c136_text_as_paths_missing_face_kept_and_single_line_open() {
    let s = open(TEXTS_SVG.as_bytes());
    let out = export(
        &s,
        SvgExportOptions {
            text_as_paths: true,
            ..SvgExportOptions::default()
        },
    );
    let svg = out.text;
    let live: Vec<String> = of_kind(&svg, "text");
    assert_eq!(
        live.len(),
        1,
        "only the missing-face text stays live:\n{svg}"
    );
    assert!(svg.contains("Lobster missing"));
    assert_eq!(out.kept, 1, "counted for the notice");
    // "Boho" Bold as outlines: the counters of B and o are holes (more subpaths than glyphs).
    let subpaths: usize = of_kind(&svg, "path")
        .iter()
        .map(|r| attr(r, "d").unwrap_or_default().matches('M').count())
        .sum();
    assert!(
        subpaths >= 4 + 2,
        "Boho outlines with the counters of B and o"
    );
    // Single-line font: open paths with no fill, also with the option off.
    for options in [
        SvgExportOptions::default(),
        SvgExportOptions {
            text_as_paths: true,
            ..SvgExportOptions::default()
        },
    ] {
        let svg = export(&s, options).text;
        assert!(
            !svg.contains(">Stick<"),
            "a single-line text is never live text"
        );
        let stick: Vec<String> = tags(&svg)
            .into_iter()
            .filter(|(n, _)| n == "path")
            .map(|(_, r)| r)
            .filter(|r| {
                attr(r, "fill").as_deref() == Some("none")
                    && attr(r, "stroke").as_deref() == Some("#000000")
            })
            .collect();
        assert!(!stick.is_empty(), "stick outlines stroked, unfilled");
        assert!(
            stick.iter().all(|r| !attr(r, "d").unwrap().contains('Z')),
            "open paths"
        );
    }
}

#[test]
fn c146_text_with_stroke_under_strokes_as_paths_is_outlines() {
    let s = open(TEXTS_SVG.as_bytes());
    let out = export(
        &s,
        SvgExportOptions {
            strokes_as_paths: true,
            ..SvgExportOptions::default()
        },
    );
    assert!(
        !out.text.contains("Rot  two"),
        "a stroked text is written as outlines"
    );
    assert!(
        out.text.contains("Boho"),
        "a text without stroke stays live (only stroked texts convert)"
    );
    assert!(
        !out.text.contains("stroke-width"),
        "no stroke attributes under Strokes as paths:\n{}",
        out.text
    );
}

#[test]
fn c152_text_export_is_byte_stable() {
    let s = open(TEXTS_SVG.as_bytes());
    let first = plain(&s);
    let again = plain(&open(first.as_bytes()));
    assert_eq!(
        again, first,
        "export(open(export(d))) == export(d) for texts"
    );
}

#[test]
fn c152_turned_character_and_vertical_upright_text_are_byte_stable() {
    // The implementer reports these as converging over trips instead of byte-stable; the spec
    // (criterion 152) demands byte stability.
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 210 297">
  <text x="50" y="50" font-family="Source Sans 3" font-size="8" text-anchor="middle"><tspan x="50" y="50" dx="0 1" rotate="0 90 0">ABC</tspan></text>
  <text x="150" y="40" font-family="Source Sans 3" font-size="6" writing-mode="vertical-rl" text-orientation="upright">Vert</text>
</svg>"#;
    let s = open(svg);
    let first = plain(&s);
    assert!(
        first.contains("rotate=\""),
        "the turned character is written as rotate\n{first}"
    );
    assert!(first.contains("writing-mode=\"vertical-rl\""));
    assert!(first.contains("text-orientation=\"upright\""));
    let again = plain(&open(first.as_bytes()));
    let diff = again
        .lines()
        .zip(first.lines())
        .find(|(a, b)| a != b)
        .map(|(a, b)| format!("second: {a}\nfirst:  {b}"));
    assert!(again == first, "criterion 152: not byte-stable\n{diff:?}");
}

#[test]
fn c134_text_on_path_href_resolves_and_position_comes_back() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="210mm" height="297mm" viewBox="0 0 210 297">
  <path id="carrier" d="M 20 100 C 60 40 120 40 160 100" fill="none" stroke="#000000" stroke-width="0.25"/>
  <text font-family="Source Sans 3" font-size="6"><textPath xlink:href="#carrier" startOffset="50%" text-anchor="middle">CURVYO</textPath></text>
</svg>"##;
    let s = open(svg);
    let d = doc_of(&s);
    let before = text_with(&d, "CURVYO");
    let out = plain(&s);
    assert!(out.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
    let tp = of_kind(&out, "textPath");
    assert_eq!(tp.len(), 1, "{out}");
    let href = attr(&tp[0], "xlink:href").unwrap();
    let id = href.strip_prefix('#').unwrap();
    assert!(out.contains(&format!("id=\"{id}\"")), "href resolves");
    // text-anchor="middle" on the textPath or inherited from its text (spec wording ambiguous).
    let text_raw = &tags(text_element(&out, "CURVYO"))[0].1;
    assert!(
        attr(&tp[0], "text-anchor").as_deref() == Some("middle")
            || attr(text_raw, "text-anchor").as_deref() == Some("middle")
    );
    assert!(attr(&tp[0], "startOffset").is_some());
    let back = doc_of(&open(out.as_bytes()));
    let after = text_with(&back, "CURVYO");
    let (a, b) = (
        before.on_path.expect("on path"),
        after.on_path.expect("still on a path"),
    );
    assert!(
        (a.values.position.as_mm() - b.values.position.as_mm()).abs() < 1e-6,
        "Position within 1e-6 mm"
    );
    assert_eq!(a.values.side, b.values.side);
    assert!((a.values.distance.as_mm() - b.values.distance.as_mm()).abs() < 1e-6);
    assert_eq!(plain(&open(out.as_bytes())), out, "stable");
}

// ---------------------------------------------------------------------------------------------
// Criterion 138: sketches
// ---------------------------------------------------------------------------------------------

fn sketch_doc() -> Document {
    let mut g = sk::Sketch::new("origin".into());
    let pt = |g: &mut sk::Sketch, id: &str, x: f64, y: f64| {
        g.entities.insert(
            id.into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, y),
                fixed: true,
            },
        );
    };
    pt(&mut g, "origin", 0.0, 0.0);
    for (i, (x, y)) in [(10.0, 10.0), (50.0, 10.0), (50.0, 40.0), (10.0, 40.0)]
        .into_iter()
        .enumerate()
    {
        pt(&mut g, &format!("p{i}"), x, y);
    }
    for i in 0..4 {
        g.entities.insert(
            format!("line{i}").as_str().into(),
            sk::Entity::Line {
                start: format!("p{i}").as_str().into(),
                end: format!("p{}", (i + 1) % 4).as_str().into(),
            },
        );
    }
    pt(&mut g, "c", 100.0, 100.0);
    g.entities.insert(
        "circle".into(),
        sk::Entity::Circle {
            center: "c".into(),
            radius: sk::Length(15.0),
            fixed_radius: true,
        },
    );
    pt(&mut g, "ac", 150.0, 100.0);
    pt(&mut g, "as", 160.0, 100.0);
    pt(&mut g, "ae", 150.0, 110.0);
    g.entities.insert(
        "arc".into(),
        sk::Entity::Arc {
            center: "ac".into(),
            start: "as".into(),
            end: "ae".into(),
            radius: sk::Length(10.0),
            start_angle: sk::Angle(0.0),
            end_angle: sk::Angle(std::f64::consts::FRAC_PI_2),
            fixed_shape: true,
        },
    );
    pt(&mut g, "k0", 0.0, 200.0);
    pt(&mut g, "k1", 100.0, 200.0);
    g.entities.insert(
        "construction".into(),
        sk::Entity::Line {
            start: "k0".into(),
            end: "k1".into(),
        },
    );
    g.construction.insert("construction".into());
    let d = Document::new(801);
    d.create_page_sketch(&d.pages()[0].id, &g, &Style::default())
        .unwrap();
    d
}

#[test]
fn c138_sketch_is_a_group_of_solved_entities_without_construction() {
    let s = open_project(&sketch_doc());
    let svg = plain(&s);
    let groups: Vec<String> = of_kind(&svg, "g")
        .into_iter()
        .filter(|g| attr(g, "inkscape:groupmode").is_none())
        .collect();
    assert_eq!(groups.len(), 1, "one group for the sketch\n{svg}");
    assert!(attr(&groups[0], "id").unwrap().starts_with('g'));
    let gid = attr(&groups[0], "id").unwrap();
    let at = svg.find(&format!("id=\"{gid}\"")).unwrap();
    let inner = &svg[at..at + svg[at..].find("</g>").unwrap()];
    let kinds: Vec<String> = tags(inner).into_iter().map(|(n, _)| n).collect();
    let drawn: Vec<&String> = kinds
        .iter()
        .filter(|k| ["path", "circle", "ellipse", "rect"].contains(&k.as_str()))
        .collect();
    assert_eq!(
        drawn.len(),
        6,
        "four lines, a circle, an arc; no construction line: {kinds:?}"
    );
    assert!(
        kinds.iter().any(|k| k == "circle"),
        "criterion 138: a circle as <circle>: {kinds:?}"
    );
    let back = doc_of(&open(svg.as_bytes()));
    assert!(
        back.object_ids()
            .into_iter()
            .all(|id| !matches!(back.object(id), Some(ObjectSnapshot::Sketch(_)))),
        "import gives plain objects"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 157: safety
// ---------------------------------------------------------------------------------------------

#[test]
fn c157_hostile_strings_are_escaped() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="210mm" height="297mm" viewBox="0 0 210 297">
  <g inkscape:groupmode="layer" inkscape:label="&lt;/g&gt;&lt;script&gt;x()&lt;/script&gt;">
    <text x="10" y="10" font-family="Evil&quot;&gt;&lt;x" font-size="5">&lt;svg onload=alert(1)&gt; &amp;amp;</text>
    <rect inkscape:label="]]&gt;&lt;!--" x="0" y="0" width="1" height="1"/>
  </g>
</svg>"##;
    let s = open(svg);
    let out = plain(&s);
    for raw in ["<script>", "<svg onload", "<x", "<!--", "]]>"] {
        assert!(!out.contains(raw), "{raw} unescaped in\n{out}");
    }
    let parsed = Session::open_file(1, out.as_bytes(), "a.svg");
    assert!(parsed.is_ok());
    let back = doc_of(&open(out.as_bytes()));
    assert!(
        texts(&back)
            .iter()
            .any(|t| t.string == "<svg onload=alert(1)> &amp;")
    );
}

/// Criterion 157 asks for "a network-denying harness in the test". On Linux the export runs in
/// a child process inside a fresh network namespace (`unshare -rn`), where any connection fails;
/// the export must still succeed and give the same bytes. Skips with a message where user
/// namespaces are not available.
#[test]
fn c157_export_succeeds_without_network() {
    if std::env::var_os("CURVYO_TESTER_NONET").is_some() {
        let s = open(TEXTS_SVG.as_bytes());
        std::fs::write(std::env::var("CURVYO_TESTER_NONET").unwrap(), plain(&s)).unwrap();
        return;
    }
    let probe = std::process::Command::new("unshare")
        .args(["-rn", "true"])
        .status();
    if !probe.is_ok_and(|s| s.success()) {
        eprintln!("unshare -rn unavailable: network-denying harness skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("curvyo-nonet-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("x.svg");
    let ok = std::process::Command::new("unshare")
        .arg("-rn")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "c157_export_succeeds_without_network"])
        .env("CURVYO_TESTER_NONET", &out)
        .status()
        .unwrap()
        .success();
    assert!(ok, "the export runs without network");
    let text = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(text, plain(&open(TEXTS_SVG.as_bytes())));
}

// ---------------------------------------------------------------------------------------------
// Budgets: criterion 71 (export part, gated) and 154 (reported)
// ---------------------------------------------------------------------------------------------

fn many_paths_svg(paths: usize, nodes_per_path: usize) -> String {
    let mut s = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">"#,
    );
    for p in 0..paths {
        let (ox, oy) = ((p % 140) as f64 * 7.0, (p / 140) as f64 * 7.0);
        s.push_str("<path d=\"M");
        for n in 0..nodes_per_path {
            let a = n as f64 / nodes_per_path as f64 * std::f64::consts::TAU;
            s.push_str(&format!(
                " {:.4} {:.4}",
                ox + 3.0 + 3.0 * a.cos(),
                oy + 3.0 + 3.0 * a.sin()
            ));
        }
        s.push_str(" Z\" fill=\"none\" stroke=\"#000\" stroke-width=\"0.1\"/>");
    }
    s.push_str("</svg>");
    s
}

#[test]
#[ignore = "budget: native release build, criterion 71 export part"]
fn c71_export_of_20000_paths_under_1_5_s_and_25_mb() {
    let s = open(many_paths_svg(20_000, 20).as_bytes());
    let (out, cpu) = cpu_time::cpu_time_of(|| plain(&s));
    eprintln!("criterion 71 export: {:?} CPU, {} bytes", cpu, out.len());
    let (_, cpu_strokes) = cpu_time::cpu_time_of(|| {
        export(
            &s,
            SvgExportOptions {
                strokes_as_paths: true,
                ..SvgExportOptions::default()
            },
        )
        .text
        .len()
    });
    eprintln!("criterion 154 (reported): 20,000 paths with Strokes as paths: {cpu_strokes:?} CPU");
    assert!(cpu.as_secs_f64() < 1.5, "{cpu:?}");
    assert!(out.len() < 25 * 1024 * 1024, "{} bytes", out.len());
}

#[test]
#[ignore = "budget: native release build, criterion 154 (reported, not gated)"]
fn c154_report_1000_texts_and_12_mpx_jpeg() {
    let faces = [
        ("Source Sans 3", "400", "normal"),
        ("Source Sans 3", "700", "normal"),
        ("Source Serif 4", "400", "italic"),
    ];
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">"#,
    );
    for i in 0..1000 {
        let (fam, w, st) = faces[i % 3];
        let kern = if i % 3 == 0 {
            " dx=\"0 0.2 0 0.3\""
        } else {
            ""
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{fam}\" font-weight=\"{w}\" font-style=\"{st}\" font-size=\"3\"><tspan{kern}>The quick brown fox jumps over the lazy dog 0123</tspan></text>",
            (i % 10) * 100,
            (i / 10) * 10 + 5
        ));
    }
    svg.push_str("</svg>");
    let s = open(svg.as_bytes());
    let (_, live) = cpu_time::cpu_time_of(|| plain(&s).len());
    let (_, paths) = cpu_time::cpu_time_of(|| {
        export(
            &s,
            SvgExportOptions {
                text_as_paths: true,
                ..SvgExportOptions::default()
            },
        )
        .text
        .len()
    });
    eprintln!(
        "criterion 154: 1,000 texts live {live:?} (target < 1.0 s), as paths {paths:?} (target < 3.0 s)"
    );
    // 12 Mpx JPEG: 4000 x 3000 noise.
    let mut img = image::RgbImage::new(4000, 3000);
    let mut x: u32 = 1;
    for p in img.pixels_mut() {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *p = image::Rgb([x as u8, (x >> 8) as u8, (x >> 16) as u8]);
    }
    let mut jpeg = std::io::Cursor::new(Vec::new());
    img.write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    let jpeg = jpeg.into_inner();
    let b64 = {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::with_capacity(jpeg.len() * 4 / 3 + 4);
        for c in jpeg.chunks(3) {
            let n = (u32::from(c[0]) << 16)
                | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
                | u32::from(*c.get(2).unwrap_or(&0));
            o.push(T[(n >> 18) as usize & 63] as char);
            o.push(T[(n >> 12) as usize & 63] as char);
            o.push(if c.len() > 1 {
                T[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            o.push(if c.len() > 2 {
                T[n as usize & 63] as char
            } else {
                '='
            });
        }
        o
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="400mm" height="300mm" viewBox="0 0 400 300"><image x="0" y="0" width="400" height="300" preserveAspectRatio="none" xlink:href="data:image/jpeg;base64,{b64}"/></svg>"#
    );
    let s = open(svg.as_bytes());
    let (out, cpu) = cpu_time::cpu_time_of(|| plain(&s));
    assert!(out.contains(&b64), "the JPEG bytes are written verbatim");
    eprintln!(
        "criterion 154: 12 Mpx JPEG ({} bytes) export {cpu:?} (target < 0.3 s)",
        jpeg.len()
    );
}
