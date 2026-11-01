//! Independent tester tests for `specs/0024-svg-import-export` slice 2 (criteria 78 to 110: text
//! and images) at the editor session, the lowest place where a text is laid out in its face, an
//! import is one history step and the faces load in two phases. Written from the specification
//! before the implementation was read; only the names of the public seams were looked up.
//!
//! Real files: `curvyo-svg-core/tests/fixtures/svg_real/inkscape_text/` (made with Inkscape 1.4.4
//! by `make_slice2.sh`, see `../SOURCES.md`). The Inkscape comparison: the same file after
//! Inkscape's own Object to Path (`*.paths.svg`) is opened in Curvyo (slice 1's path import) and
//! every glyph node of the imported text, laid out in its face, must have an Inkscape node within
//! 0.05 mm and the other way round (criterion 86: "within 0.05 mm per node").

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::many_single_char_names,
    clippy::format_collect,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unreadable_literal,
    clippy::manual_midpoint,
    clippy::assert_is_empty,
    clippy::format_push_string,
    missing_docs
)]

mod cpu_time;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Document, ImageSnapshot, MISSING_FACE_HASH, NodeId, NodeKind, NodeRead, ObjectSnapshot,
    PathSide, Point, TextAlign, TextGlyphOrientation, TextSnapshot, TextWritingMode, Timestamp,
    unpack,
};
use curvyo_editor_wasm::Session;
use curvyo_text_core::{FaceKey, FaceStore};
use curvyo_ui_core::{FontBlock, lay_out, object_outline_bounds};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_950_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn ink_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/svg_real/inkscape_text")
}

fn ink(name: &str) -> Vec<u8> {
    std::fs::read(ink_dir().join(name)).unwrap()
}

fn picture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

/// `(file, family, style, index)` of every included face.
fn included_faces() -> Vec<(String, String, String, u32)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect()
}

/// Hands every wanted face over, as the host does; returns how many.
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
    s.set_font_list(FontBlock::Included, included_faces(), Vec::new());
}

fn fresh() -> Session {
    let mut s = Session::new(31);
    prepare(&mut s);
    load(&mut s);
    s
}

/// Import SVG… with the host's loop: while the session waits, load what it wants and call again.
/// The outcome as its `Debug` text ("Imported(3)", "Nothing(\"…\")").
fn import(s: &mut Session, bytes: &[u8]) -> String {
    for _ in 0..8 {
        let outcome = format!("{:?}", s.import_svg(bytes, "drawing.svg").unwrap());
        if outcome == "Waiting" {
            assert!(load(s) > 0, "a waiting import names a face to load");
        } else {
            return outcome;
        }
    }
    panic!("the import never stopped waiting");
}

/// Open… of an SVG with the host's loop.
fn open(bytes: &[u8]) -> Session {
    let mut s = Session::open_file(32, bytes, "drawing.svg").unwrap();
    prepare(&mut s);
    for _ in 0..8 {
        load(&mut s);
        if !s.continue_open_svg(false) {
            return s;
        }
    }
    panic!("the open never stopped waiting");
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
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

fn images(d: &Document) -> Vec<ImageSnapshot> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Image(i)) => Some(i),
            _ => None,
        })
        .collect()
}

fn node(d: &Document, id: NodeId) -> NodeRead {
    d.read_nodes().into_iter().find(|n| n.id == id).unwrap()
}

fn named(d: &Document, name: &str) -> Option<NodeRead> {
    d.read_nodes()
        .into_iter()
        .find(|n| n.registers.name.as_deref() == Some(name))
}

/// `ancestor` and every node below it.
fn subtree(d: &Document, ancestor: NodeId) -> std::collections::HashSet<NodeId> {
    let nodes = d.read_nodes();
    let mut set = std::collections::HashSet::from([ancestor]);
    // read_nodes lists parents before children.
    for n in &nodes {
        if n.parent.is_some_and(|p| set.contains(&p)) {
            set.insert(n.id);
        }
    }
    set
}

fn texts_below(d: &Document, ancestor: NodeId) -> Vec<TextSnapshot> {
    let set = subtree(d, ancestor);
    texts(d)
        .into_iter()
        .filter(|t| set.contains(&t.id))
        .collect()
}

fn text_named(d: &Document, name: &str) -> TextSnapshot {
    let n = named(d, name).unwrap_or_else(|| panic!("a node named {name}"));
    let t = texts_below(d, n.id);
    assert_eq!(t.len(), 1, "{name} is one text");
    t.into_iter().next().unwrap()
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

/// The included faces loaded once, by (family, style).
struct Faces {
    store: FaceStore,
    keys: HashMap<(String, String), FaceKey>,
}

impl Faces {
    fn new() -> Self {
        let mut store = FaceStore::new();
        let mut keys = HashMap::new();
        for (file, family, style, _) in included_faces() {
            let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
            let key = store.add(bytes, 0).unwrap();
            keys.insert((family, style), key);
        }
        Self { store, keys }
    }

    /// The glyph nodes of `t` laid out in its face (the substitute for a missing one), in mm.
    fn nodes(&self, t: &TextSnapshot) -> Vec<Point> {
        let found = self
            .keys
            .get(&(t.font.family.clone(), t.font.style.clone()))
            .filter(|_| t.font.face_hash != MISSING_FACE_HASH);
        let (key, substitute) = match found {
            Some(k) => (k, false),
            None => (
                &self.keys[&("Source Sans 3".to_owned(), "Regular".to_owned())],
                true,
            ),
        };
        let outline = lay_out(t, self.store.get(key).unwrap(), substitute).expect("laid out");
        outline.glyphs.all_anchors().map(|a| a.point).collect()
    }

    /// Bounds of the glyph nodes of `t`.
    fn bounds(&self, t: &TextSnapshot) -> (Point, Point) {
        bounds_of(&self.nodes(t))
    }
}

fn bounds_of(points: &[Point]) -> (Point, Point) {
    points.iter().fold(
        (pt(f64::MAX, f64::MAX), pt(f64::MIN, f64::MIN)),
        |(a, b), p| {
            (
                pt(a.x.min(p.x), a.y.min(p.y)),
                pt(b.x.max(p.x), b.y.max(p.y)),
            )
        },
    )
}

/// The largest distance from a point of `a` to its nearest point of `b`.
fn one_way(a: &[Point], b: &[Point]) -> f64 {
    a.iter()
        .map(|p| {
            b.iter()
                .map(|q| (p.x - q.x).hypot(p.y - q.y))
                .fold(f64::MAX, f64::min)
        })
        .fold(0.0, f64::max)
}

/// The symmetric node distance of two node sets.
fn node_distance(a: &[Point], b: &[Point]) -> f64 {
    one_way(a, b).max(one_way(b, a))
}

/// The nodes of every path below the node named `name` in a document opened from an Inkscape
/// `*.paths.svg` file.
fn inkscape_nodes(d: &Document, name: &str) -> Vec<Point> {
    let n = named(d, name).unwrap_or_else(|| panic!("Inkscape's conversion has {name}"));
    let set = subtree(d, n.id);
    d.object_ids()
        .into_iter()
        .filter(|id| set.contains(id))
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Path(p)) => Some(p.all_anchors().map(|a| a.point).collect()),
            _ => None,
        })
        .collect::<Vec<Vec<Point>>>()
        .concat()
}

/// The glyph nodes of every text Curvyo made of the `<text>` labelled `name` (the node with
/// that name, or a group of a split text named by its first 24 characters).
fn curvyo_nodes(d: &Document, faces: &Faces, name: &str, whole: &str) -> Vec<Point> {
    let group_name: String = whole.chars().take(24).collect();
    let n = named(d, name)
        .or_else(|| named(d, &group_name))
        .unwrap_or_else(|| panic!("Curvyo's import has {name}"));
    let t = texts_below(d, n.id);
    assert!(!t.is_empty(), "{name} gave a text");
    t.iter().flat_map(|t| faces.nodes(t)).collect()
}

/// The `aria-label` Inkscape wrote for each converted text: its whole string.
fn aria_labels(paths_svg: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut label = None;
    for line in paths_svg.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("inkscape:label=\"") {
            label = Some(v.trim_end_matches(['"', '>', '/', ' ']).to_owned());
        }
        if let Some(v) = line.strip_prefix("aria-label=\"") {
            let v = v.trim_end_matches(['>', '/', ' ']).trim_end_matches('"');
            if let Some(l) = label.take() {
                out.insert(l, v.replace("&#10;", " "));
            }
        }
    }
    out
}

/// Runs the Inkscape comparison for `labels` of `file`, returning (label, distance) pairs.
fn compare_with_inkscape(file: &str, labels: &[&str]) -> Vec<(String, f64)> {
    let ours = open(&ink(&format!("{file}.svg")));
    let theirs = open(&ink(&format!("{file}.paths.svg")));
    let (ours, theirs) = (doc_of(&ours), doc_of(&theirs));
    let whole = aria_labels(&String::from_utf8(ink(&format!("{file}.paths.svg"))).unwrap());
    let faces = Faces::new();
    labels
        .iter()
        .map(|&l| {
            let a = curvyo_nodes(&ours, &faces, l, whole.get(l).map_or("", String::as_str));
            let b = inkscape_nodes(&theirs, l);
            ((*l).to_owned(), node_distance(&a, &b))
        })
        .collect()
}

/// Inkscape's comparison tolerance (criterion 86): 0.05 mm per node. Both programs take the
/// unhinted outlines of the same font files (Inkscape lays out at 1024 times the size, so Pango's
/// position rounding is below 0.001 mm here); what is left is the shapers' rounding of advances.
const INKSCAPE_TOL: f64 = 0.05;

fn assert_like_inkscape(file: &str, labels: &[&str]) {
    let mut bad = Vec::new();
    for (l, dist) in compare_with_inkscape(file, labels) {
        eprintln!("{file} {l}: {dist:.4} mm");
        if dist > INKSCAPE_TOL {
            bad.push(format!("{l}: {dist:.3} mm"));
        }
    }
    assert!(bad.is_empty(), "nodes off Inkscape's conversion: {bad:?}");
}

/// A minimal SVG in millimetres (210 × 297, one user unit = 1 mm) around `body`.
fn mm_svg(body: &str) -> Vec<u8> {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#
    )
    .into_bytes()
}

/// Opens `body` in a millimetre document.
fn open_mm(body: &str) -> (Session, Document) {
    let s = open(&mm_svg(body));
    let d = doc_of(&s);
    (s, d)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---------------------------------------------------------------------------------------------
// N. Text: the object, the font and the size
// ---------------------------------------------------------------------------------------------

/// Criterion 78: one text object per `<text>` at its z-order place in its layer, named by
/// `inkscape:label` (else the default), hidden/locked/opacity and fill/stroke kept; an empty
/// text, a size of 0 and a text in `defs` give nothing; a `use` of a text gives a copy; an `<a>`
/// inside a text is read as a tspan.
#[test]
fn c78_one_text_object_per_text_element() {
    let (_s, d) = open_mm(
        r##"<defs><text id="indefs" x="1" y="1">In defs</text></defs>
        <g inkscape:groupmode="layer" inkscape:label="L">
          <rect id="below" x="0" y="0" width="5" height="5"/>
          <text id="a" inkscape:label="Label A" x="10" y="20" style="font-size:5px;fill:#ff0000;stroke:#0000ff;stroke-width:0.2;opacity:0.5">Alpha</text>
          <rect id="above" x="0" y="0" width="5" height="5"/>
          <text id="h" x="10" y="40" style="font-size:5px;display:none">Hidden</text>
          <text id="k" x="10" y="50" sodipodi:insensitive="true" style="font-size:5px">Locked</text>
          <text x="10" y="60" style="font-size:5px">   </text>
          <text x="10" y="70" style="font-size:0px">Zero</text>
          <text x="10" y="80" style="font-size:5px">Go <a href="https://example.org"><tspan>linked</tspan></a> on</text>
          <use xlink:href="#a" x="0" y="100"/>
        </g>"##,
    );
    let all = texts(&d);
    let strings: Vec<&str> = all.iter().map(|t| t.string.as_str()).collect();
    assert!(
        !strings.contains(&"In defs"),
        "a text in defs gives nothing"
    );
    assert!(!strings.contains(&"Zero"), "font-size 0 gives nothing");
    assert!(
        !strings.iter().any(|s| s.trim().is_empty()),
        "a text without a character gives nothing"
    );
    assert_eq!(
        strings.iter().filter(|s| **s == "Alpha").count(),
        2,
        "the text and its use copy: {strings:?}"
    );
    assert!(strings.contains(&"Go linked on"), "an <a> reads as a tspan");
    let a = named(&d, "Label A").expect("named by inkscape:label");
    assert!(matches!(a.kind, NodeKind::Leaf(ObjectSnapshot::Text(_))));
    assert!(close(a.opacity.get(), 0.5, 1e-9), "opacity follows");
    let ta = text_named(&d, "Label A");
    let fill = ta.style.fill;
    assert!(fill.enabled, "fill kept");
    assert_eq!(
        (fill.color.r, fill.color.g, fill.color.b),
        (255, 0, 0),
        "fill red"
    );
    let stroke = ta.style.stroke;
    assert!(stroke.enabled, "stroke kept");
    assert_eq!(
        (stroke.color.r, stroke.color.g, stroke.color.b),
        (0, 0, 255)
    );
    assert!(close(stroke.width.as_mm(), 0.2, 1e-9), "stroke width in mm");
    let h = node(&d, text_with(&d, "Hidden").id);
    assert!(!h.registers.visible, "display:none is hidden");
    let k = node(&d, text_with(&d, "Locked").id);
    assert!(k.registers.locked, "sodipodi:insensitive is locked");
    // z-order: Alpha between the two rectangles of its layer.
    let layer = named(&d, "L").unwrap();
    let order: Vec<&str> = d
        .read_nodes()
        .into_iter()
        .filter(|n| n.parent == Some(layer.id))
        .map(|n| match n.kind {
            NodeKind::Leaf(ObjectSnapshot::Text(t)) => {
                if t.string == "Alpha" {
                    "Alpha"
                } else {
                    "text"
                }
            }
            _ => "other",
        })
        .collect();
    assert_eq!(
        &order[..3],
        &["other", "Alpha", "other"],
        "z-order: {order:?}"
    );
}

impl Faces {
    /// The laid-out outline of `t`.
    fn outline(&self, t: &TextSnapshot) -> curvyo_document_core::TextOutline {
        let found = self
            .keys
            .get(&(t.font.family.clone(), t.font.style.clone()))
            .filter(|_| t.font.face_hash != MISSING_FACE_HASH);
        let (key, substitute) = match found {
            Some(k) => (k, false),
            None => (
                &self.keys[&("Source Sans 3".to_owned(), "Regular".to_owned())],
                true,
            ),
        };
        lay_out(t, self.store.get(key).unwrap(), substitute).expect("laid out")
    }

    /// The nodes of each drawn glyph of `t`, in drawing order.
    fn glyphs(&self, t: &TextSnapshot) -> Vec<Vec<Point>> {
        let o = self.outline(t);
        let outlines: Vec<Vec<Point>> = std::iter::once(&o.glyphs.anchors)
            .chain(o.glyphs.extra_subpaths.iter().map(|p| &p.anchors))
            .map(|a| a.iter().map(|a| a.point).collect())
            .collect();
        let mut at = 0;
        o.glyph_outlines
            .iter()
            .map(|&n| {
                let g = outlines[at..at + n].concat();
                at += n;
                g
            })
            .collect()
    }
}

/// The glyphs of every text `keep` selects (a split text gives several), ordered by their left
/// edge.
fn glyphs_where(
    d: &Document,
    faces: &Faces,
    keep: impl Fn(&TextSnapshot) -> bool,
) -> Vec<Vec<Point>> {
    let mut all: Vec<Vec<Point>> = texts(d)
        .iter()
        .filter(|t| keep(t))
        .flat_map(|t| faces.glyphs(t))
        .collect();
    all.sort_by(|a, b| bounds_of(a).0.x.total_cmp(&bounds_of(b).0.x));
    all
}

fn red(t: &TextSnapshot) -> bool {
    t.style.fill.color.r == 255
}

fn shifted(points: &[Point], dx: f64, dy: f64) -> Vec<Point> {
    points.iter().map(|p| pt(p.x + dx, p.y + dy)).collect()
}

/// The H ratio of the face of `t` as Curvyo lays it out: the height of a laid-out "H" in that
/// face over its font size.
fn h_height(faces: &Faces, family: &str, style: &str) -> f64 {
    let (_s, d) = open_mm(&format!(
        r#"<text x="10" y="100" style="font-size:100px;font-family:'{family}';font-weight:{}">H</text>"#,
        if style.contains("Bold") { 700 } else { 400 }
    ));
    let t = text_with(&d, "H");
    let (lo, hi) = faces.bounds(&t);
    (hi.y - lo.y) / 100.0
}

/// Criterion 79: SVG whitespace, `xml:space`, `white-space: pre`, control characters, character
/// references, and `tref`/`title`/`desc` adding nothing.
#[test]
fn c79_whitespace_follows_svg() {
    let (_s, d) = open_mm(
        "<text x=\"10\" y=\"10\" style=\"font-size:5px\">  a \n  b\tc  </text>\
         <text x=\"10\" y=\"20\" xml:space=\"preserve\" style=\"font-size:5px\">  d\ne\tf </text>\
         <text x=\"10\" y=\"30\" style=\"font-size:5px;white-space:pre\">g\nh  i</text>\
         <text x=\"10\" y=\"40\" style=\"font-size:5px\">j\u{7f}k&#x2014;l<title>T</title><desc>D</desc>m</text>",
    );
    let strings: Vec<String> = texts(&d).into_iter().map(|t| t.string).collect();
    for want in ["a b c", "  d e f ", "g\nh  i", "jk\u{2014}lm"] {
        assert!(
            strings.iter().any(|s| s == want),
            "{want:?} among {strings:?}"
        );
    }
}

/// Criterion 80: the family list in order, case-insensitive family names, PostScript and full
/// names, generic names and no family at all.
#[test]
fn c80_font_family_lookup() {
    let (_s, d) = open_mm(
        r#"<text x="10" y="10" style="font-size:5px;font-family:'source sans 3'">case</text>
        <text x="10" y="20" style="font-size:5px;font-family:SourceSans3-Bold">postscript</text>
        <text x="10" y="30" style="font-size:5px;font-family:'Source Sans 3 Light'">fullname</text>
        <text x="10" y="40" style="font-size:5px;font-family:Nope, 'Source Serif 4', sans-serif">list</text>
        <text x="10" y="50" style="font-size:5px;font-family:serif">serif</text>
        <text x="10" y="60" style="font-size:5px;font-family:cursive">cursive</text>
        <text x="10" y="70" style="font-size:5px;font-family:monospace">monospace</text>
        <text x="10" y="80" style="font-size:5px;font-family:fantasy">fantasy</text>
        <text x="10" y="90" style="font-size:5px;font-family:system-ui">systemui</text>
        <text x="10" y="100" style="font-size:5px">none</text>
        <style>.ps { font-family: "Great Vibes" }</style>
        <text class="ps" x="10" y="110" style="font-size:5px">class</text>"#,
    );
    for (string, family, style) in [
        ("case", "Source Sans 3", "Regular"),
        ("postscript", "Source Sans 3", "Bold"),
        ("fullname", "Source Sans 3", "Light"),
        ("list", "Source Serif 4", "Regular"),
        ("serif", "Source Serif 4", "Regular"),
        ("cursive", "Great Vibes", "Regular"),
        ("monospace", "Source Sans 3", "Regular"),
        ("fantasy", "Source Sans 3", "Regular"),
        ("systemui", "Source Sans 3", "Regular"),
        ("none", "Source Sans 3", "Regular"),
        ("class", "Great Vibes", "Regular"),
    ] {
        let t = text_with(&d, string);
        assert_eq!(
            (t.font.family.as_str(), t.font.style.as_str()),
            (family, style),
            "{string}"
        );
        assert_ne!(
            t.font.face_hash, MISSING_FACE_HASH,
            "{string} is not missing"
        );
    }
}

/// Criterion 81: italic state, then weight (CSS nearest with the tie rule), `bolder`/`lighter`
/// relative to the parent; no synthesis; a family without the italic is not missing.
#[test]
fn c81_face_choice_in_a_family() {
    let (_s, d) = open_mm(
        r#"<g style="font-family:'Source Sans 3';font-size:5px">
        <text x="10" y="10" style="font-weight:600">w600</text>
        <text x="10" y="20" style="font-weight:300;font-style:italic">w300i</text>
        <text x="10" y="30" style="font-weight:250">w250</text>
        <text x="10" y="40" style="font-weight:800">w800</text>
        <text x="10" y="50" style="font-style:oblique">oblique</text>
        <g style="font-weight:400"><text x="10" y="60" style="font-weight:bolder">bolder</text></g>
        <g style="font-weight:700"><text x="10" y="70" style="font-weight:lighter">lighter</text></g>
        <g style="font-weight:300"><text x="10" y="80" style="font-weight:bolder">bolder300</text></g>
        </g>
        <text x="10" y="90" style="font-size:5px;font-family:'Great Vibes';font-style:italic;font-weight:bold">vibes</text>"#,
    );
    for (string, family, style) in [
        ("w600", "Source Sans 3", "Bold"),
        ("w300i", "Source Sans 3", "Light Italic"),
        ("w250", "Source Sans 3", "Extra Light"),
        ("w800", "Source Sans 3", "Bold"),
        ("oblique", "Source Sans 3", "Italic"),
        ("bolder", "Source Sans 3", "Bold"),
        ("lighter", "Source Sans 3", "Regular"),
        ("bolder300", "Source Sans 3", "Regular"),
        ("vibes", "Great Vibes", "Regular"),
    ] {
        let t = text_with(&d, string);
        assert_eq!(
            (t.font.family.as_str(), t.font.style.as_str()),
            (family, style),
            "{string}"
        );
        assert_ne!(
            t.font.face_hash, MISSING_FACE_HASH,
            "{string} is not missing"
        );
    }
}

/// Criterion 82: a family that matches nothing is a missing-font text with the first named entry
/// and the requested style, drawn in the substitute; a generic after a missing name still gives
/// a missing text.
#[test]
fn c82_missing_font_keeps_the_files_name() {
    let (_s, d) = open_mm(
        r#"<text x="10" y="10" style="font-size:5px;font-family:'Nonexistent Sans';font-weight:bold;font-style:italic">missing</text>
        <text x="10" y="20" style="font-size:5px;font-family:'Lobster', cursive">lobster</text>"#,
    );
    let faces = Faces::new();
    for (string, family, style) in [
        ("missing", "Nonexistent Sans", "Bold Italic"),
        ("lobster", "Lobster", "Regular"),
    ] {
        let t = text_with(&d, string);
        assert_eq!(
            (t.font.family.as_str(), t.font.style.as_str()),
            (family, style),
            "{string}"
        );
        assert_eq!(
            t.font.face_hash, MISSING_FACE_HASH,
            "{string}: no content hash"
        );
        assert!(faces.outline(&t).substitute || !faces.nodes(&t).is_empty());
    }
}

/// Criterion 82 with ADR note 2: the missing reference survives Save and reopen unchanged, and
/// once a face of that family and style is listed the reopened project asks for it.
#[test]
fn c82_missing_face_round_trips_and_resolves_later() {
    let (s, d) = open_mm(
        r#"<text x="10" y="10" style="font-size:5px;font-family:'Nonexistent Sans';font-weight:bold;font-style:italic">missing</text>"#,
    );
    let before = text_with(&d, "missing");
    let packed = s.pack("0.1.0").unwrap();
    let reopened = unpack(5, &packed).unwrap();
    let after = text_with(&reopened, "missing");
    assert_eq!(
        after.font, before.font,
        "the reference round-trips unchanged"
    );
    assert_eq!(after.string, before.string);
    // Reopen in a session; list a file under the missing family and style: it is wanted.
    let mut s2 = Session::open_file(6, &packed, "p.curvyo").unwrap();
    s2.set_clock(clock);
    s2.resize_viewport(1200.0, 800.0);
    s2.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s2);
    let file = "source-serif-4/SourceSerif4-BoldIt.ttf".to_owned();
    let mut listed = included_faces();
    listed.push((
        file.clone(),
        "Nonexistent Sans".to_owned(),
        "Bold Italic".to_owned(),
        0,
    ));
    s2.set_font_list(FontBlock::Included, listed, Vec::new());
    assert!(
        s2.wanted_font_files().contains(&file),
        "the text asks for the face that now exists"
    );
    let again = text_with(&doc_of(&s2), "missing");
    assert_eq!(again.font.family, "Nonexistent Sans");
}

/// Criterion 83: Size is the font size in mm times the face's H ratio, for every unit; clamped
/// at 2,000 mm. Spec example: Inkscape's 30 pt text in a mm document, `font-size:10.5833px`.
#[test]
fn c83_size_from_font_size_and_units() {
    let faces = Faces::new();
    let h = h_height(&faces, "Source Sans 3", "Regular");
    let (_s, d) = open_mm(
        r#"<g style="font-size:10px">
        <text x="10" y="10" style="font-size:1.5em">em</text>
        <text x="10" y="20" style="font-size:50%">pct</text>
        <text x="10" y="30" style="font-size:2ex">ex</text>
        <text x="10" y="40" style="font-size:larger">larger</text>
        <text x="10" y="50" style="font-size:smaller">smaller</text>
        </g>
        <text x="10" y="60" style="font-size:x-large">xlarge</text>
        <text x="10" y="70" style="font-size:1pc">pc</text>
        <text x="10" y="80" style="font-size:0.5in">in</text>
        <text x="10" y="90">nosize</text>
        <text x="10" y="100" style="font-size:5000px">clamp</text>
        <text x="10" y="110" style="font-size:10.5833px">inkscape</text>"#,
    );
    for (string, user_units) in [
        ("em", 15.0),
        ("pct", 5.0),
        ("ex", 10.0),
        ("larger", 12.0),
        ("smaller", 10.0 / 1.2),
        ("xlarge", 24.0),
        ("pc", 16.0),
        ("in", 48.0),
        ("nosize", 16.0),
        ("inkscape", 10.5833),
    ] {
        let size = text_with(&d, string).size.as_mm();
        assert!(
            close(size, user_units * h, 0.001),
            "{string}: Size {size} vs {}",
            user_units * h
        );
    }
    assert!(
        close(text_with(&d, "clamp").size.as_mm(), 2000.0, 1e-9),
        "clamped"
    );
    // A px document (96 user units per inch): 30 pt is 10.5833 mm.
    let s = open(
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 793.7007874 1122.519685"><text x="10" y="100" style="font-size:30pt">pt</text></svg>"#,
    );
    let size = text_with(&doc_of(&s), "pt").size.as_mm();
    assert!(close(size, 10.583_333 * h, 0.001), "30 pt: {size}");
}

/// Criterion 83's example value: 10.5833 px gives 6.943 mm (the measured H of 656 units; spec
/// corrected 2026-10-27).
#[test]
fn c83_spec_example_value() {
    let (_s, d) = open_mm(r#"<text x="10" y="10" style="font-size:10.5833px">x</text>"#);
    let size = text_with(&d, "x").size.as_mm();
    assert!(close(size, 6.943, 0.001), "Size {size}");
}

/// Criterion 84: the uniform part of the matrix goes into the Size; the rest is the text's own
/// transform; the anchor is mapped.
#[test]
fn c84_transform_split_into_size_and_placement() {
    let (_s, d) = open_mm(
        r#"<text x="10" y="20" style="font-size:5px">plain</text>
        <text x="10" y="20" transform="rotate(30)" style="font-size:5px">rotated</text>
        <text x="10" y="20" transform="scale(2 1)" style="font-size:5px">stretched</text>
        <text x="10" y="20" transform="scale(1.41421356)" style="font-size:5px">uniform</text>
        <text x="10" y="20" transform="scale(-1 1)" style="font-size:5px">mirrored</text>
        <text x="10" y="20" transform="scale(0 1)" style="font-size:5px">flat</text>"#,
    );
    let plain = text_with(&d, "plain");
    let rot = text_with(&d, "rotated");
    assert!(close(rot.size.as_mm(), plain.size.as_mm(), 1e-9));
    let r = rot.placement;
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    assert!(
        close(r.a, c, 1e-9) && close(r.b, s, 1e-9) && close(r.c, -s, 1e-9) && close(r.d, c, 1e-9),
        "{r:?}"
    );
    let anchor = (10.0 * c - 20.0 * s, 10.0 * s + 20.0 * c);
    assert!(
        close(r.e, anchor.0, 1e-9) && close(r.f, anchor.1, 1e-9),
        "{r:?}"
    );
    let st = text_with(&d, "stretched");
    let un = text_with(&d, "uniform");
    assert!(close(st.size.as_mm(), un.size.as_mm(), 1e-6));
    let p = st.placement;
    assert!(
        close(p.a, 2f64.sqrt(), 1e-9) && close(p.d, 1.0 / 2f64.sqrt(), 1e-9),
        "{p:?}"
    );
    assert!(close(p.e, 20.0, 1e-9) && close(p.f, 20.0, 1e-9), "{p:?}");
    let m = text_with(&d, "mirrored").placement;
    assert!(m.a * m.d - m.b * m.c < 0.0, "a mirror is a flipped text");
    assert!(
        !texts(&d).iter().any(|t| t.string == "flat"),
        "determinant 0 skips"
    );
}

/// Criterion 85: line starts, Line spacing from `line-height` (also inherited), an uneven gap
/// kept by a kern, an empty line kept.
#[test]
fn c85_lines_and_line_spacing() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="20" style="font-size:10px;line-height:1.25"><tspan sodipodi:role="line" x="10" y="20">x</tspan><tspan sodipodi:role="line" x="10" y="32.5">x</tspan><tspan sodipodi:role="line" x="10" y="50">x</tspan></text>
        <g style="line-height:2"><text x="10" y="100" style="font-size:5px"><tspan sodipodi:role="line">a</tspan><tspan sodipodi:role="line"></tspan><tspan sodipodi:role="line">b</tspan></text></g>
        <text x="10" y="150" style="font-size:5px"><tspan x="10" y="150">c</tspan><tspan x="10" y="160">d</tspan></text>
        <text x="100" y="20" style="font-size:10px;line-height:1.25"><tspan x="100" y="20">z</tspan><tspan x="100" y="32.5">z</tspan><tspan x="100" y="50">z</tspan></text>"#,
    );
    let z = text_with(&d, "z\nz\nz");
    let zb: Vec<f64> = faces
        .glyphs(&z)
        .iter()
        .map(|g| g.iter().map(|p| p.y).fold(f64::MIN, f64::max))
        .collect();
    for (got, want) in zb.iter().zip([20.0, 32.5, 50.0]) {
        assert!(close(*got, want, 0.01), "Illustrator lines: {zb:?}");
    }
    let t = text_with(&d, "x\nx\nx");
    let ratio = t.size.as_mm() / 10.0;
    let ls = t.layout.line_spacing.expect("line spacing set");
    assert!(close(ls, 1.25 / ratio, 1e-6), "line spacing {ls}");
    let bottoms: Vec<f64> = faces
        .glyphs(&t)
        .iter()
        .map(|g| g.iter().map(|p| p.y).fold(f64::MIN, f64::max))
        .collect();
    for (got, want) in bottoms.iter().zip([20.0, 32.5, 50.0]) {
        assert!(close(*got, want, 0.01), "baselines {bottoms:?}");
    }
    let e = text_with(&d, "a\n\nb");
    assert!(
        close(
            e.layout.line_spacing.unwrap(),
            2.0 / (e.size.as_mm() / 5.0),
            1e-6
        ),
        "inherited line-height"
    );
    let _ = text_with(&d, "c\nd");
}

/// Criterion 86 (hand-computed): absolute `x` lists and `dx`/`dy` put each glyph where SVG 1.1
/// places it, within 0.01 mm.
#[test]
fn c86_positions_from_lists() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="50" style="font-size:5px">A</text>
        <text x="10 20 35" y="60" style="font-size:5px">AAA</text>
        <text x="10" y="70" style="font-size:5px">BBB</text>
        <text x="10" y="80" dx="0 1 1" dy="0 2 -0.5" style="font-size:5px">BBB</text>"#,
    );
    let one = &faces.glyphs(&text_with(&d, "A"))[0];
    let three = glyphs_where(&d, &faces, |t| {
        t.string.contains('A') && t.placement.f > 55.0
    });
    assert_eq!(three.len(), 3);
    for (g, dx) in three.iter().zip([0.0, 10.0, 25.0]) {
        let want = shifted(one, dx, 10.0);
        assert!(
            node_distance(g, &want) <= 0.01,
            "x list: {}",
            node_distance(g, &want)
        );
    }
    let plain = glyphs_where(&d, &faces, |t| {
        t.string.contains('B') && t.placement.f < 75.0
    });
    let kerned = glyphs_where(&d, &faces, |t| {
        t.string.contains('B') && t.placement.f > 75.0
    });
    assert_eq!(kerned.len(), 3);
    for (i, (p, k)) in plain.iter().zip(&kerned).enumerate() {
        let (dx, dy) = [(0.0, 0.0), (1.0, 2.0), (2.0, 1.5)][i];
        let want = shifted(p, dx, 10.0 + dy);
        assert!(
            node_distance(k, &want) <= 0.01,
            "glyph {i}: {}",
            node_distance(k, &want)
        );
    }
}

/// Criterion 86 (Inkscape): every straight text of the real Inkscape file lands on Inkscape's own
/// Object to Path within 0.05 mm per node (lines, alignment, faces, spacing, transforms, the
/// SVG 1.1 lines of `inline-size` and `shape-inside`).
#[test]
fn c86_inkscape_texts_match_inkscapes_conversion() {
    assert_like_inkscape(
        "text_ink",
        &[
            "t_one",
            "t_multi",
            "t_right",
            "t_bold",
            "t_italic",
            "t_light",
            "t_bolditalic",
            "t_semibold",
            "t_serif",
            "t_spacing",
            "t_rot30",
            "t_stretch",
            "t_mirror",
            "t_inline",
            "t_shape",
        ],
    );
}

/// Criterion 87 (hand-computed): "ABC" with `rotate="0 90 0"`: B is turned a quarter clockwise
/// about its origin on the baseline; A and C are unchanged.
#[test]
fn c87_rotate_turns_about_the_glyph_origin() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10 30 50" y="50" style="font-size:10px">ABC</text>
        <text x="10 30 50" y="50" rotate="0 90 0" style="font-size:10px;fill:#ff0000">ABC</text>"#,
    );
    let p = glyphs_where(&d, &faces, |t| !red(t));
    let r = glyphs_where(&d, &faces, red);
    assert_eq!((p.len(), r.len()), (3, 3));
    assert!(node_distance(&p[0], &r[0]) <= 0.01, "A unchanged");
    assert!(node_distance(&p[2], &r[2]) <= 0.01, "C unchanged");
    let want: Vec<Point> = p[1]
        .iter()
        .map(|q| pt(30.0 - (q.y - 50.0), 50.0 + (q.x - 30.0)))
        .collect();
    let dist = node_distance(&r[1], &want);
    assert!(dist <= 0.01, "B turned about (30, 50): {dist}");
}

/// Criterion 87 (Inkscape): Alt+arrow kerns and a turned character match Inkscape's conversion.
#[test]
fn c87_inkscape_kerns_and_turned_character() {
    assert_like_inkscape("text_ink", &["t_kerns"]);
}

/// Criterion 88: letter and word spacing become the text's values (times Size); a spacing outside
/// the range keeps the look; `textLength` stretches the spacing.
#[test]
fn c88_spacing_and_text_length() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="10" style="font-size:5px;letter-spacing:0.5px;word-spacing:2px">a b</text>
        <text x="10" y="20" style="font-size:5px;letter-spacing:30px">AB</text>
        <text x="10" y="20" dx="0 30" style="font-size:5px;fill:#ff0000">AB</text>"#,
    );
    let t = text_with(&d, "a b");
    let size = t.size.as_mm();
    assert!(
        close(t.layout.letter_spacing * size, 0.5, 1e-6),
        "letter spacing"
    );
    assert!(
        close(t.layout.word_spacing * size, 2.0, 1e-6),
        "word spacing"
    );
    let wide = glyphs_where(&d, &faces, |t| !red(t) && t.placement.f > 15.0).concat();
    let dx = glyphs_where(&d, &faces, red).concat();
    let dist = node_distance(&wide, &dx);
    assert!(
        dist <= 0.01,
        "30 px letter spacing looks like dx 30: {dist}"
    );
}

/// Criterion 88: `textLength` (lengthAdjust spacing) makes the line extent equal to it.
#[test]
fn c88_text_length() {
    // Lead decision 2026-10-27: a split of criterion 92 (b) stays; the extent is that of all
    // pieces of the text together.
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="20" textLength="40" style="font-size:5px">AAAA</text>
        <text x="10" y="40" textLength="60" style="font-size:5px">AAAA</text>"#,
    );
    let a = glyphs_where(&d, &faces, |t| {
        t.placement.f < 30.0 && t.string.contains('A')
    });
    let b = glyphs_where(&d, &faces, |t| {
        t.placement.f > 30.0 && t.string.contains('A')
    });
    assert_eq!((a.len(), b.len()), (4, 4), "four glyphs over all pieces");
    assert!(
        node_distance(&shifted(&a[0], 0.0, 20.0), &b[0]) <= 0.01,
        "first A in place"
    );
    for i in 1..4 {
        let want = 20.0 * f64::from(u8::try_from(i).unwrap()) / 3.0;
        let dist = node_distance(&shifted(&a[i], want, 20.0), &b[i]);
        assert!(
            dist <= 0.01,
            "A {i} moves by {want} mm of the 20 mm difference: {dist}"
        );
    }
}

/// Criterion 89: `text-anchor` gives the alignment; the Inkscape centred and right texts
/// (compared in c86) are Centre and Right.
#[test]
fn c89_text_anchor_alignment() {
    let s = open(&ink("text_ink.svg"));
    let d = doc_of(&s);
    assert_eq!(text_named(&d, "t_multi").layout.align, TextAlign::Centre);
    assert_eq!(text_named(&d, "t_right").layout.align, TextAlign::Right);
    assert_eq!(text_named(&d, "t_one").layout.align, TextAlign::Left);
    let (_s, d) = open_mm(
        r#"<g style="text-anchor:end"><text x="100" y="20" style="font-size:5px">inherited</text></g>"#,
    );
    assert_eq!(text_with(&d, "inherited").layout.align, TextAlign::Right);
}

/// Criterion 90: `dominant-baseline` middle, central, hanging; `baseline-shift` length.
#[test]
fn c90_baselines() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="50" dominant-baseline="middle" style="font-size:10px">x</text>
        <text x="30" y="50" dominant-baseline="central" style="font-size:10px">x</text>
        <text x="50" y="50" style="font-size:10px;dominant-baseline:hanging">x</text>
        <text x="70" y="50" style="font-size:10px">x<tspan style="baseline-shift:2px">x</tspan></text>"#,
    );
    let bounds_at = |x: f64| {
        let t = texts(&d)
            .into_iter()
            .find(|t| close(t.placement.e, x, 1.0))
            .unwrap();
        faces.glyphs(&t)
    };
    let mid = bounds_of(&bounds_at(10.0)[0]);
    assert!(
        close((mid.0.y + mid.1.y) / 2.0, 50.0, 0.05),
        "middle: {mid:?}"
    );
    // central: down by half of ascender plus descender (1000 and -326 of 1000 in hhea).
    let cen = bounds_of(&bounds_at(30.0)[0]);
    assert!(close(cen.1.y, 50.0 + 3.37, 0.05), "central: {cen:?}");
    // hanging: down by 0.8 of the ascender.
    let han = bounds_of(&bounds_at(50.0)[0]);
    assert!(close(han.1.y, 58.0, 0.05), "hanging: {han:?}");
    let shift = bounds_at(70.0);
    let (a, b) = (bounds_of(&shift[0]), bounds_of(&shift[1]));
    assert!(
        close(a.1.y - b.1.y, 2.0, 0.01),
        "baseline-shift 2px up: {a:?} {b:?}"
    );
}

/// Criterion 90: `super` and `sub` use the face's OS/2 offsets (Source Sans 3: 350 and 75 of
/// 1,000 units), not the 1/3 and 1/5 fallback.
#[test]
fn c90_super_and_sub_use_os2_offsets() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="50" style="font-size:7px">x<tspan style="baseline-shift:super">2</tspan></text>
        <text x="40" y="50" style="font-size:7px">x<tspan style="baseline-shift:sub">2</tspan></text>
        <text x="70" y="50" style="font-size:7px">x2</text>"#,
    );
    let pieces = |x0: f64, x1: f64| -> Vec<Point> {
        texts(&d)
            .into_iter()
            .filter(|t| t.placement.e > x0 && t.placement.e < x1)
            .flat_map(|t| {
                faces
                    .glyphs(&t)
                    .into_iter()
                    .filter(|g| g.len() > 4)
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect()
    };
    let plain = bounds_of(&pieces(65.0, 100.0));
    let sup = bounds_of(&pieces(5.0, 35.0));
    let sub = bounds_of(&pieces(35.0, 65.0));
    assert!(
        close(plain.0.y - sup.0.y, 0.35 * 7.0, 0.01),
        "super {} ",
        plain.0.y - sup.0.y
    );
    assert!(
        close(sub.1.y - plain.1.y, 0.075 * 7.0, 0.01),
        "sub {}",
        sub.1.y - plain.1.y
    );
}

/// Criterion 91: writing modes and glyph orientations.
#[test]
fn c91_writing_modes() {
    let (_s, d) = open_mm(
        r#"<text x="10" y="10" style="font-size:5px;writing-mode:vertical-rl">vrl</text>
        <text x="20" y="10" style="font-size:5px;writing-mode:tb-rl;text-orientation:upright">tbrl</text>
        <text x="30" y="10" style="font-size:5px;writing-mode:tb;text-orientation:sideways">tb</text>
        <text x="40" y="10" style="font-size:5px;writing-mode:vertical-lr;glyph-orientation-vertical:0">vlr</text>
        <text x="50" y="10" style="font-size:5px;writing-mode:vertical-rl;glyph-orientation-vertical:90">g90</text>
        <text x="60" y="10" style="font-size:5px;writing-mode:lr-tb">lrtb</text>
        <text x="70" y="10" writing-mode="rl" style="font-size:5px">rl</text>"#,
    );
    for (string, mode, orientation) in [
        (
            "vrl",
            TextWritingMode::VerticalRl,
            TextGlyphOrientation::Auto,
        ),
        (
            "tbrl",
            TextWritingMode::VerticalRl,
            TextGlyphOrientation::Upright,
        ),
        (
            "tb",
            TextWritingMode::VerticalRl,
            TextGlyphOrientation::Sideways,
        ),
        (
            "vlr",
            TextWritingMode::VerticalLr,
            TextGlyphOrientation::Upright,
        ),
        (
            "g90",
            TextWritingMode::VerticalRl,
            TextGlyphOrientation::Sideways,
        ),
        (
            "lrtb",
            TextWritingMode::Horizontal,
            TextGlyphOrientation::Auto,
        ),
        (
            "rl",
            TextWritingMode::Horizontal,
            TextGlyphOrientation::Auto,
        ),
    ] {
        let t = text_with(&d, string);
        assert_eq!(
            (t.layout.writing_mode, t.layout.orientation),
            (mode, orientation),
            "{string}"
        );
    }
}

/// Criterion 91 (Inkscape): the upright vertical text Inkscape wrote (`tb-rl`) matches its
/// conversion (the spec's own test of criterion 86 lists it).
#[test]
fn c91_inkscape_vertical_text() {
    assert_like_inkscape("text_ink", &["t_vertical"]);
}

/// Criterion 92: "Hello **world**" is a group of two texts named by the whole text; a far
/// placed tspan starts a new text; more than 10,000 characters split; a one-object text is never
/// grouped. Inkscape's mixed text (bold word, smaller red word) matches its conversion.
#[test]
fn c92_split_into_texts_in_a_group() {
    let long = "a".repeat(10_001);
    let (_s, d) = open_mm(&format!(
        r#"<g inkscape:groupmode="layer" inkscape:label="L">
        <text x="10" y="20" style="font-size:5px;font-family:'Source Sans 3'">Hello <tspan style="font-weight:bold">world</tspan></text>
        <text style="font-size:3px"><tspan x="10" y="40">Left</tspan><tspan x="190" y="40">Right</tspan></text>
        <text x="10" y="60" style="font-size:3px">{long}</text>
        <text x="10" y="80" style="font-size:3px">Single</text></g>"#
    ));
    let group = named(&d, "Hello world").expect("a group named by the whole text");
    assert_eq!(group.kind, NodeKind::Group);
    let parts = texts_below(&d, group.id);
    let mut got: Vec<(String, String)> = parts
        .iter()
        .map(|t| (t.string.clone(), t.font.style.clone()))
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            ("Hello ".to_owned(), "Regular".to_owned()),
            ("world".to_owned(), "Bold".to_owned())
        ]
    );
    assert!(
        texts(&d).iter().any(|t| t.string == "Left"),
        "far tspan splits (b)"
    );
    assert!(texts(&d).iter().any(|t| t.string == "Right"));
    let lens: Vec<usize> = texts(&d)
        .iter()
        .filter(|t| t.string.starts_with("aaa") || t.string == "a")
        .map(|t| t.string.chars().count())
        .collect();
    assert_eq!(lens.iter().sum::<usize>(), 10_001);
    assert!(lens.iter().all(|&n| n <= 10_000), "{lens:?}");
    let layer = named(&d, "L").unwrap();
    assert_eq!(
        node(&d, text_with(&d, "Single").id).parent,
        Some(layer.id),
        "never grouped"
    );
    assert_like_inkscape("text_ink", &["t_mixed"]);
}

/// Criterion 87 (hand-computed, no split): in one text "ABC" with `rotate="0 90 0"` and no
/// positions, B turns about its own origin and A and C stay where the plain text has them.
#[test]
fn c87_rotate_in_one_text() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="50" style="font-size:10px">ABC</text>
        <text x="0" y="100" style="font-size:10px">B</text>
        <text x="10" y="50" rotate="0 90 0" style="font-size:10px;fill:#ff0000">ABC</text>"#,
    );
    let p = glyphs_where(&d, &faces, |t| !red(t) && t.string == "ABC");
    let r = glyphs_where(&d, &faces, red);
    let q = &glyphs_where(&d, &faces, |t| t.string == "B")[0];
    assert_eq!((p.len(), r.len()), (3, 3));
    assert!(node_distance(&p[0], &r[0]) <= 0.01, "A unchanged");
    assert!(
        node_distance(&p[2], &r[2]) <= 0.01,
        "C unchanged: {}",
        node_distance(&p[2], &r[2])
    );
    // B's origin in the plain text: the shift from the lone B (origin (0, 100)).
    let ox = bounds_of(&p[1]).0.x - bounds_of(q).0.x;
    let want: Vec<Point> = q
        .iter()
        .map(|v| pt(ox - (v.y - 100.0), 50.0 + v.x))
        .collect();
    let dist = node_distance(&r[1], &want);
    assert!(dist <= 0.01, "B turned about its origin: {dist}");
}

// ---------------------------------------------------------------------------------------------
// P. Text on path and flowed text
// ---------------------------------------------------------------------------------------------

fn on_path_texts(d: &Document) -> Vec<TextSnapshot> {
    texts(d)
        .into_iter()
        .filter(|t| t.on_path.is_some())
        .collect()
}

/// Criterion 93: texts on an open path, a circle at `startOffset` 75 % centred, a rectangle and
/// a text with its own transform land on Inkscape's conversion; the carrier is the imported
/// object.
#[test]
fn c93_text_on_path_matches_inkscape() {
    assert_like_inkscape("text_ink", &["tp_open", "tp_circle", "tp_rect", "tp_moved"]);
    let s = open(&ink("text_ink.svg"));
    let d = doc_of(&s);
    let t = text_named(&d, "tp_circle");
    let carrier = t.on_path.as_ref().unwrap().carrier.node;
    match d.object(carrier) {
        Some(ObjectSnapshot::Primitive(_)) => {}
        other => panic!("the circle is the carrier: {other:?}"),
    }
    let values = t.on_path.as_ref().unwrap().values;
    assert_eq!(values.side, PathSide::Above);
    assert!(close(values.distance.as_mm(), 0.0, 1e-9), "Distance 0");
}

/// Criterion 93: `side="right"` puts the letters where SVG 2 draws them (on the reversed path).
#[test]
fn c93_side_right_matches_inkscape() {
    let s = open(&ink("text_ink.svg"));
    let d = doc_of(&s);
    let t = text_named(&d, "tp_side");
    assert_eq!(t.on_path.as_ref().unwrap().values.side, PathSide::Below);
    assert_like_inkscape("text_ink", &["tp_side"]);
}

/// Criterion 94: a painted carrier stays drawn; an unpainted or hidden one becomes a guide; a
/// carrier in another user space or in `defs` is a new guide path just below the text.
#[test]
fn c94_carrier_roles() {
    let s = open(&ink("text_ink.svg"));
    let d = doc_of(&s);
    let carrier_of = |name: &str| {
        node(
            &d,
            text_named(&d, name).on_path.as_ref().unwrap().carrier.node,
        )
    };
    assert!(!carrier_of("tp_open").guide, "stroked path stays drawn");
    assert!(!carrier_of("tp_circle").guide, "stroked circle stays drawn");
    assert!(
        carrier_of("tp_rect").guide,
        "unpainted rectangle is a guide"
    );
    assert!(carrier_of("tp_side").guide, "unpainted path is a guide");
    let moved = carrier_of("tp_moved");
    assert!(
        moved.guide,
        "a text with its own transform gets a new guide path"
    );
    let drawn_green: Vec<NodeRead> = d
        .read_nodes()
        .into_iter()
        .filter(|n| matches!(&n.kind, NodeKind::Leaf(ObjectSnapshot::Path(p)) if p.style.stroke.enabled && p.style.stroke.color.g == 170))
        .collect();
    assert_eq!(drawn_green.len(), 1, "the drawn green path stays");
    assert!(!drawn_green[0].guide);
    assert_ne!(drawn_green[0].id, moved.id);
    // Just below the text, in its layer.
    let text = node(&d, text_named(&d, "tp_moved").id);
    assert_eq!(moved.parent, text.parent);
    let siblings: Vec<NodeId> = d
        .read_nodes()
        .into_iter()
        .filter(|n| n.parent == text.parent)
        .map(|n| n.id)
        .collect();
    let (gi, ti) = (
        siblings.iter().position(|&i| i == moved.id).unwrap(),
        siblings.iter().position(|&i| i == text.id).unwrap(),
    );
    assert_eq!(gi + 1, ti, "the guide is just below the text");
    let (_s, d) = open_mm(
        r##"<defs><path id="indefs" d="M 10,50 L 200,50"/></defs>
        <path id="hid" d="M 10,80 L 200,80" style="fill:none;stroke:#000;display:none"/>
        <text style="font-size:5px"><textPath xlink:href="#indefs">From defs</textPath></text>
        <text style="font-size:5px"><textPath xlink:href="#hid">On hidden</textPath></text>"##,
    );
    let c = node(
        &d,
        text_with(&d, "From defs")
            .on_path
            .as_ref()
            .unwrap()
            .carrier
            .node,
    );
    assert!(c.guide, "a defs carrier is a new guide path");
    let h = node(
        &d,
        text_with(&d, "On hidden")
            .on_path
            .as_ref()
            .unwrap()
            .carrier
            .node,
    );
    assert!(
        h.guide && !h.registers.visible,
        "a hidden carrier is a hidden guide"
    );
}

/// Criterion 95: mixed straight and on-path characters, several textPaths, a missing or
/// unusable target, a vertical text on a path.
#[test]
fn c95_other_text_on_path_cases() {
    let (_s, d) = open_mm(
        r##"<path id="p1" d="M 10,100 L 200,100" style="fill:none;stroke:#000"/>
        <g id="grp"><rect x="0" y="0" width="5" height="5"/></g>
        <text x="10" y="50" style="font-size:5px">Before <textPath xlink:href="#p1">On path</textPath></text>
        <text style="font-size:5px"><textPath xlink:href="#p1">One</textPath><textPath xlink:href="#p1" startOffset="100">Two</textPath></text>
        <text x="10" y="150" style="font-size:5px"><textPath xlink:href="#nothere">Missing target</textPath></text>
        <text x="10" y="170" style="font-size:5px"><textPath xlink:href="#grp">Group target</textPath></text>
        <text style="font-size:5px;writing-mode:vertical-rl"><textPath xlink:href="#p1" startOffset="150">Vert</textPath></text>
        <text style="font-size:5px;fill:#ff0000"><textPath xlink:href="#p1" startOffset="150">Vert</textPath></text>"##,
    );
    let faces = Faces::new();
    let straight: Vec<String> = texts(&d)
        .into_iter()
        .filter(|t| t.on_path.is_none())
        .map(|t| t.string.trim().to_owned())
        .collect();
    let on: Vec<String> = on_path_texts(&d).into_iter().map(|t| t.string).collect();
    for s in ["Before", "Missing target", "Group target"] {
        assert!(
            straight.iter().any(|x| x == s),
            "{s} straight: {straight:?}"
        );
    }
    for s in ["On path", "One", "Two", "Vert"] {
        assert!(on.iter().any(|x| x == s), "{s} on path: {on:?}");
    }
    let m = text_with(&d, "Missing target").placement;
    assert!(
        close(m.e, 10.0, 1e-6) && close(m.f, 150.0, 1e-6),
        "at x, y: {m:?}"
    );
    let vertical = glyphs_where(&d, &faces, |t| t.string == "Vert" && !red(t)).concat();
    let horizontal = glyphs_where(&d, &faces, |t| t.string == "Vert" && red(t)).concat();
    assert!(
        node_distance(&vertical, &horizontal) <= 0.01,
        "a vertical text on a path is laid out horizontally"
    );
}

/// Criterion 96: Inkscape's `flowRoot` (60 mm box, left and centred) gives the same line breaks
/// and line spacing as Inkscape shows. Inkscape and the spec place the first baseline
/// differently (spec: box top plus the face's ascender; Inkscape adds half-leading), so the
/// comparison aligns the first baselines and the spec's rule is checked by hand below.
#[test]
fn c96_flowroot_breaks_like_inkscape() {
    let ours = doc_of(&open(&ink("flowroot_ink.svg")));
    let theirs = doc_of(&open(&ink("flowroot_ink.paths.svg")));
    let faces = Faces::new();
    for (label, whole) in [
        (
            "flow1",
            "The quick brown fox jumps over the lazy dog and keeps on running far away. Second paragraph.",
        ),
        (
            "flow2",
            "Centred flowed text that needs to wrap more than once in its box.",
        ),
    ] {
        let a = curvyo_nodes(&ours, &faces, label, whole);
        let b = inkscape_nodes(&theirs, label);
        let dy = bounds_of(&a).0.y - bounds_of(&b).0.y;
        let dist = node_distance(&a, &shifted(&b, 0.0, dy));
        eprintln!("{label}: first-baseline offset {dy:.4} mm, nodes {dist:.4} mm");
        assert!(dist <= INKSCAPE_TOL, "{label}: {dist}");
    }
    assert_eq!(text_named(&ours, "flow2").layout.align, TextAlign::Centre);
    // Spec rule: the first baseline is the box top plus the ascender (1,000 of 1,000 units).
    let (_s, d) = open_mm(
        r#"<flowRoot style="font-size:5px;font-family:'Source Sans 3';text-align:justify"><flowRegion><rect x="20" y="30" width="60" height="80"/></flowRegion><flowPara>T</flowPara></flowRoot>
        <flowRoot style="font-size:5px;text-align:end"><flowRegion><rect x="20" y="130" width="60" height="5"/></flowRegion><flowPara>a long paragraph that does not fit the small box below at all and is kept</flowPara></flowRoot>
        <flowRoot style="font-size:5px"><flowPara>first</flowPara><flowPara>second</flowPara></flowRoot>"#,
    );
    let t = text_with(&d, "T");
    let top = faces.bounds(&t).0.y;
    assert!(close(top, 30.0 + 5.0 - t.size.as_mm(), 0.01), "T top {top}");
    assert_eq!(t.layout.align, TextAlign::Justify);
    let kept = texts(&d)
        .into_iter()
        .find(|t| t.string.starts_with("a long"))
        .unwrap();
    assert_eq!(
        kept.string.replace('\n', " "),
        "a long paragraph that does not fit the small box below at all and is kept"
    );
    assert_eq!(kept.layout.align, TextAlign::Right);
    let lines = text_with(&d, "first\nsecond");
    assert!(close(lines.placement.e, 0.0, 1e-6), "{:?}", lines.placement);
}

/// Criterion 96 (a): an `inline-size` text without positioned lines is broken at that width.
#[test]
fn c96_inline_size_without_positioned_lines() {
    let faces = Faces::new();
    let (_s, d) = open_mm(
        r#"<text x="10" y="20" style="font-size:5px;inline-size:30">one two three four five six seven eight nine ten</text>"#,
    );
    let t = texts(&d)
        .into_iter()
        .find(|t| t.string.starts_with("one"))
        .unwrap();
    assert!(t.string.contains('\n'), "broken into lines: {:?}", t.string);
    let (lo, hi) = faces.bounds(&t);
    assert!(
        hi.x - lo.x <= 30.0 + 0.01,
        "within the width: {}",
        hi.x - lo.x
    );
}

/// Criterion 97: features without a counterpart are left out and the text still imports;
/// `text-transform` is applied to the string.
#[test]
fn c97_left_out_features_and_text_transform() {
    let (_s, d) = open_mm(
        r#"<style>@font-face { font-family: Web; src: url(data:font/woff2;base64,AAAA); }</style>
        <text x="10" y="10" style="font-size:5px;text-decoration:underline;font-variant:small-caps;font-feature-settings:'liga' 0;font-variation-settings:'wght' 700;font-kerning:none;paint-order:stroke;direction:rtl;unicode-bidi:bidi-override">Plain</text>
        <text x="10" y="20" style="font-size:5px;text-transform:uppercase">upper me</text>
        <text x="10" y="30" style="font-size:5px;text-transform:lowercase">LOWER ME</text>
        <text x="10" y="40" style="font-size:5px;text-transform:capitalize">hello big world</text>
        <text x="10" y="50" style="font-size:5px;font-family:Web">Webfont</text>"#,
    );
    for s in ["Plain", "UPPER ME", "lower me", "Hello Big World"] {
        let _ = text_with(&d, s);
    }
    let w = text_with(&d, "Webfont");
    assert_eq!(
        w.font.family, "Web",
        "an @font-face family is not fetched: missing"
    );
    assert_eq!(w.font.face_hash, MISSING_FACE_HASH);
}

/// Criterion 98 (session part): strings stay plain text; a font name over 128 characters is cut.
#[test]
fn c98_plain_strings_and_cut_font_names() {
    let long = "F".repeat(200);
    let (_s, d) = open_mm(&format!(
        r#"<text x="10" y="10" style="font-size:5px">&lt;b&gt;bold&lt;/b&gt; &amp; <![CDATA[<i>x</i>]]></text>
        <text x="10" y="20" style="font-size:5px;font-family:'{long}'">long name</text>"#
    ));
    let _ = text_with(&d, "<b>bold</b> & <i>x</i>");
    let t = text_with(&d, "long name");
    assert_eq!(t.font.family.chars().count(), 128, "cut at 128");
}

/// Criterion 99: `<switch>` takes the first child SVG would draw: `systemLanguage` English only,
/// `requiredFeatures` SVG 1.1 only, a bare `foreignObject` and Illustrator's are passed over.
#[test]
fn c99_switch_rules() {
    let (_s, d) = open_mm(
        r#"<switch><g systemLanguage="de"><text x="1" y="1" style="font-size:5px">Deutsch</text></g><g systemLanguage="fr, en-GB"><text x="1" y="1" style="font-size:5px">English</text></g></switch>
        <switch><g requiredFeatures="http://example.org/nofeature"><text x="1" y="1" style="font-size:5px">NoFeature</text></g><g><text x="1" y="1" style="font-size:5px">Fallback</text></g></switch>
        <switch><g requiredFeatures="http://www.w3.org/TR/SVG11/feature#Shape"><text x="1" y="1" style="font-size:5px">Feature11</text></g><g><text x="1" y="1" style="font-size:5px">NotThis</text></g></switch>
        <switch><foreignObject width="10" height="10"><p xmlns="http://www.w3.org/1999/xhtml">html</p></foreignObject><g><rect width="3" height="3"/><text x="1" y="1" style="font-size:5px">AfterBare</text></g></switch>
        <switch><foreignObject requiredExtensions="http://ns.adobe.com/AdobeIllustrator/10.0/" width="1" height="1"/><g><text x="1" y="1" style="font-size:5px">AfterAi</text></g></switch>"#,
    );
    let strings: Vec<String> = texts(&d).into_iter().map(|t| t.string).collect();
    for s in ["English", "Fallback", "Feature11", "AfterBare", "AfterAi"] {
        assert!(strings.iter().any(|x| x == s), "{s} taken: {strings:?}");
    }
    for s in ["Deutsch", "NoFeature", "NotThis", "html"] {
        assert!(
            !strings.iter().any(|x| x == s),
            "{s} passed over: {strings:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Q. Images
// ---------------------------------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(B64[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in text.bytes() {
        let Some(v) = B64.iter().position(|&b| b == c) else {
            continue;
        };
        acc = (acc << 6) | u32::try_from(v).unwrap();
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xff).unwrap());
        }
    }
    out
}

/// The bytes of the first data URI of `mime` in the Inkscape image file.
fn ink_picture(mime: &str) -> Vec<u8> {
    let svg = String::from_utf8(ink("images_ink.svg")).unwrap();
    let start = svg.find(&format!("data:{mime};base64,")).unwrap() + mime.len() + 13;
    let end = start + svg[start..].find('"').unwrap();
    unbase64(&svg[start..end])
}

fn data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64(bytes))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// An RGBA PNG of `w` × `h` with stored (uncompressed) deflate blocks, so its size is about
/// 4·w·h bytes: the large embedded scan of criterion 105, made in memory.
fn stored_png(w: u32, h: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((4 * w as usize + 1) * h as usize);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.extend_from_slice(&[(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8, 255]);
        }
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(u8::from(i + 1 == chunks.len()));
        let len = u16::try_from(c.len()).unwrap();
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    png
}

fn image_named(d: &Document, name: &str) -> ImageSnapshot {
    let n = named(d, name).unwrap_or_else(|| panic!("an image named {name}"));
    match n.kind {
        NodeKind::Leaf(ObjectSnapshot::Image(i)) => i,
        other => panic!("{name} is {other:?}"),
    }
}

fn image_names(d: &Document) -> Vec<String> {
    d.read_nodes()
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Leaf(ObjectSnapshot::Image(_))))
        .map(|n| n.registers.name.unwrap_or_default())
        .collect()
}

/// The unit-square map (a, b, c, d, e, f) of an image within `tol`.
fn assert_map(i: &ImageSnapshot, want: [f64; 6], tol: f64, what: &str) {
    let p = i.placement;
    let got = [p.a, p.b, p.c, p.d, p.e, p.f];
    assert!(
        got.iter().zip(want).all(|(g, w)| close(*g, w, tol)),
        "{what}: {got:?} vs {want:?}"
    );
}

/// Criterion 100: data URIs identified by content (MIME not trusted): PNG, JPEG, WebP come in;
/// GIF, BMP and SVG do not; percent-encoding and whitespace in base64 are read; SVG 2 `href`
/// wins over `xlink:href`.
#[test]
fn c100_data_uris_by_content() {
    let png = ink_picture("image/png");
    let jpeg = ink_picture("image/jpeg");
    let webp = picture("webp_lossy.webp");
    let gif = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;".to_vec();
    let bmp = {
        let mut b = b"BM".to_vec();
        b.extend_from_slice(&[0u8; 60]);
        b
    };
    let svg =
        b"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1'/></svg>".to_vec();
    let percent: String = png.iter().map(|b| format!("%{b:02X}")).collect();
    let spaced: String = base64(&png)
        .as_bytes()
        .chunks(76)
        .map(|c| format!("{} \n", std::str::from_utf8(c).unwrap()))
        .collect();
    let img = |label: &str, href: &str| {
        format!(
            r#"<image inkscape:label="{label}" x="0" y="0" width="10" height="10" xlink:href="{href}"/>"#
        )
    };
    let body = [
        img("png_as_jpeg", &data_uri("image/jpeg", &png)),
        img("jpeg", &data_uri("image/jpeg", &jpeg)),
        img("webp", &data_uri("image/webp", &webp)),
        img("gif", &data_uri("image/gif", &gif)),
        img("bmp", &data_uri("image/bmp", &bmp)),
        img("svg", &data_uri("image/svg+xml", &svg)),
        img("percent", &format!("data:image/png,{percent}")),
        img("spaced", &format!("data:image/png;base64,{spaced}")),
        format!(
            r#"<image inkscape:label="svg2wins" width="10" height="10" href="{}" xlink:href="{}"/>"#,
            data_uri("image/png", &png),
            data_uri("image/gif", &gif)
        ),
        format!(
            r#"<image inkscape:label="svg2gif" width="10" height="10" href="{}" xlink:href="{}"/>"#,
            data_uri("image/gif", &gif),
            data_uri("image/png", &png)
        ),
    ]
    .concat();
    let (_s, d) = open_mm(&body);
    let mut names = image_names(&d);
    names.sort();
    assert_eq!(
        names,
        [
            "jpeg",
            "percent",
            "png_as_jpeg",
            "spaced",
            "svg2wins",
            "webp"
        ],
        "only PNG, JPEG and WebP by content"
    );
}

/// Criterion 101: the viewport and `preserveAspectRatio` (the Inkscape file: default meet, none,
/// xMinYMin slice, rotated, skewed), `auto` size, other alignments, a zero width.
#[test]
fn c101_image_frames() {
    let d = doc_of(&open(&ink("images_ink.svg")));
    assert_map(
        &image_named(&d, "img_png"),
        [50.0, 0.0, 0.0, 25.0, 0.0, 12.5],
        1e-6,
        "meet",
    );
    assert_map(
        &image_named(&d, "img_none"),
        [50.0, 0.0, 0.0, 50.0, 60.0, 0.0],
        1e-6,
        "none",
    );
    assert_map(
        &image_named(&d, "img_slice"),
        [100.0, 0.0, 0.0, 50.0, 120.0, 0.0],
        1e-6,
        "slice",
    );
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let (x, y) = (132.5833, 1.6025404);
    assert_map(
        &image_named(&d, "img_rot"),
        [
            60.0 * c,
            60.0 * s,
            -40.0 * s,
            40.0 * c,
            x * c - y * s,
            x * s + y * c,
        ],
        1e-4,
        "rotated",
    );
    let t = 20f64.to_radians().tan();
    assert_map(
        &image_named(&d, "img_skew"),
        [60.0, 0.0, 40.0 * t, 40.0, 10.0 + 150.0 * t, 150.0],
        1e-6,
        "skewed",
    );
    let png = data_uri("image/png", &ink_picture("image/png"));
    let (_s, d) = open_mm(&format!(
        r#"<image inkscape:label="auto" x="5" y="5" xlink:href="{png}"/>
        <image inkscape:label="maxmax" x="0" y="0" width="50" height="50" preserveAspectRatio="xMaxYMax meet" xlink:href="{png}"/>
        <image inkscape:label="midslice" x="0" y="0" width="50" height="50" preserveAspectRatio="xMidYMid slice" xlink:href="{png}"/>
        <image inkscape:label="zero" x="0" y="0" width="0" height="50" xlink:href="{png}"/>"#
    ));
    assert_map(
        &image_named(&d, "auto"),
        [200.0, 0.0, 0.0, 100.0, 5.0, 5.0],
        1e-6,
        "auto",
    );
    assert_map(
        &image_named(&d, "maxmax"),
        [50.0, 0.0, 0.0, 25.0, 0.0, 25.0],
        1e-6,
        "xMaxYMax",
    );
    assert_map(
        &image_named(&d, "midslice"),
        [100.0, 0.0, 0.0, 50.0, -25.0, 0.0],
        1e-6,
        "mid slice",
    );
    assert!(named(&d, "zero").is_none(), "width 0 skips");
}

/// Criterion 102: name, opacity, hidden, locked; a `use` of an image shares the asset; an image
/// in `defs` is not drawn; a clip on the group is ignored (the whole picture shows).
#[test]
fn c102_image_registers() {
    let d = doc_of(&open(&ink("images_ink.svg")));
    assert_map(
        &image_named(&d, "img_clip"),
        [40.0, 0.0, 0.0, 40.0, 140.0, 140.0],
        1e-6,
        "clip ignored",
    );
    let png = data_uri("image/png", &ink_picture("image/png"));
    let (_s, d) = open_mm(&format!(
        r##"<defs><image id="indefs" inkscape:label="indefs" width="5" height="5" xlink:href="{png}"/></defs>
        <image id="orig" x="0" y="0" width="10" height="10" style="opacity:0.4" xlink:href="{png}"/>
        <image inkscape:label="hidden" x="20" y="0" width="10" height="10" style="display:none" xlink:href="{png}"/>
        <image inkscape:label="locked" x="40" y="0" width="10" height="10" sodipodi:insensitive="true" xlink:href="{png}"/>
        <use xlink:href="#orig" x="0" y="50"/>"##
    ));
    let names = image_names(&d);
    assert!(
        !names.iter().any(|n| n == "indefs"),
        "defs image not drawn: {names:?}"
    );
    let all = images(&d);
    assert_eq!(all.len(), 4, "orig, hidden, locked and the use copy");
    assert!(
        all.iter().all(|i| i.asset == all[0].asset),
        "one shared asset"
    );
    let unnamed: Vec<NodeRead> = d
        .read_nodes()
        .into_iter()
        .filter(|n| n.registers.name.as_deref() == Some("Image"))
        .collect();
    assert!(!unnamed.is_empty(), "no label: the name Image");
    assert!(
        unnamed.iter().any(|n| close(n.opacity.get(), 0.4, 1e-9)),
        "opacity"
    );
    assert!(!named(&d, "hidden").unwrap().registers.visible);
    assert!(named(&d, "locked").unwrap().registers.locked);
}

/// Criterion 103: a linked image, other types, damaged base64, a truncated JPEG and a picture
/// over `0058`'s ceiling are ignored silently; the rest imports and no notice is shown.
#[test]
fn c103_unusable_images_ignored_silently() {
    let d = doc_of(&open(&ink("images_ink.svg")));
    let mut names = image_names(&d);
    names.sort();
    assert_eq!(
        names,
        [
            "img_clip",
            "img_jpeg",
            "img_none",
            "img_png",
            "img_rot",
            "img_skew",
            "img_slice"
        ],
        "the linked image is ignored"
    );
    let mut s = fresh();
    let bytes = mm_svg(&format!(
        r#"<rect width="10" height="10"/>
        <image inkscape:label="damaged" width="10" height="10" xlink:href="data:image/png;base64,!!!*not base64*"/>
        <image inkscape:label="truncated" width="10" height="10" xlink:href="{}"/>
        <image inkscape:label="bomb" width="10" height="10" xlink:href="{}"/>
        <image inkscape:label="http" width="10" height="10" xlink:href="https://example.org/a.png"/>
        <image inkscape:label="file" width="10" height="10" xlink:href="file:///etc/passwd"/>"#,
        data_uri("image/jpeg", &picture("jpeg_truncated.jpg")),
        data_uri("image/png", &picture("png_bomb_60000x60000.png")),
    ));
    let outcome = import(&mut s, &bytes);
    assert_eq!(outcome, "Imported(1)", "only the rectangle, no notice");
    assert!(images(&doc_of(&s)).is_empty());
}

/// Criterion 104: the same bytes are one asset (also across imports); the whole import with
/// texts and images is one history step "Import SVG"; Undo removes it, Redo brings it back.
#[test]
fn c104_assets_and_one_history_step() {
    let mut s = fresh();
    let before = s.history_len();
    let bytes = ink("images_ink.svg");
    let mut file = String::from_utf8(bytes).unwrap();
    file = file.replace(
        "</svg>",
        r#"<text x="10" y="280" style="font-size:5px;font-family:'Source Serif 4'">A caption</text></svg>"#,
    );
    let outcome = import(&mut s, file.as_bytes());
    assert_eq!(outcome, "Imported(8)", "7 images and 1 text");
    assert_eq!(s.history_len(), before + 1, "one step");
    assert!(s.history_rows_json(0, 1).contains("Import SVG"));
    let d = doc_of(&s);
    let pngs: Vec<_> = ["img_png", "img_none", "img_slice", "img_clip"]
        .iter()
        .map(|n| image_named(&d, n).asset)
        .collect();
    assert!(pngs.iter().all(|a| *a == pngs[0]), "same PNG, one asset");
    assert_eq!(
        image_named(&d, "img_jpeg").asset,
        image_named(&d, "img_rot").asset
    );
    assert_ne!(pngs[0], image_named(&d, "img_jpeg").asset);
    assert_eq!(texts(&d).len(), 1);
    let first = images(&d)
        .into_iter()
        .map(|i| i.asset)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(s.undo().changed);
    let d = doc_of(&s);
    assert!(
        images(&d).is_empty() && texts(&d).is_empty(),
        "undo removes all of it"
    );
    assert!(s.redo().changed);
    let d = doc_of(&s);
    assert_eq!(
        (images(&d).len(), texts(&d).len()),
        (7, 1),
        "redo brings it back"
    );
    // A second import of the same file adds no new asset.
    assert_eq!(import(&mut s, file.as_bytes()), "Imported(8)");
    let all = images(&doc_of(&s))
        .into_iter()
        .map(|i| i.asset)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(all, first, "bytes the project holds are the same asset");
}

/// Criterion 105: a `data:` href above 16 MiB (a 30 MB scan, 40 MB as base64) still comes in;
/// another attribute above 16 MiB is still skipped.
#[test]
fn c105_large_embedded_image_is_exempt() {
    let png = stored_png(2800, 2700);
    let href = data_uri("image/png", &png);
    assert!(href.len() > 40_000_000, "{}", href.len());
    let file = mm_svg(&format!(
        r#"<image inkscape:label="scan" x="0" y="0" width="100" height="100" xlink:href="{href}"/>"#
    ));
    assert!(file.len() < 50 * 1024 * 1024);
    let mut s = fresh();
    assert_eq!(import(&mut s, &file), "Imported(1)", "the scan comes in");
    let d = doc_of(&s);
    assert_eq!(images(&d).len(), 1);
    let big_d = format!("M 0,0 {}", "L 1,1 ".repeat(17 * 1024 * 1024 / 6 + 1));
    let other = mm_svg(&format!(
        r#"<rect width="5" height="5"/><path inkscape:label="huge_d" d="{big_d}"/>"#
    ));
    assert_eq!(
        import(&mut s, &other),
        "Imported(1)",
        "the 17 MiB d is skipped"
    );
    assert!(named(&doc_of(&s), "huge_d").is_none());
}

// ---------------------------------------------------------------------------------------------
// R. Routes, fixtures, budgets, documents
// ---------------------------------------------------------------------------------------------

/// The centre of the line box of `t` laid out.
fn line_box_centre(faces: &Faces, t: &TextSnapshot) -> Point {
    let b = faces.outline(t).line_box;
    pt(
        b.iter().map(|p| p.x).sum::<f64>() / 4.0,
        b.iter().map(|p| p.y).sum::<f64>() / 4.0,
    )
}

/// Criterion 106: Import SVG… waits (changing nothing) until the faces are loaded, then is one
/// step; texts and images are selected; the bounding box centred on the view includes text boxes
/// and image frames; the notice shows only when nothing came in.
#[test]
fn c106_import_route_with_texts_and_images() {
    // Phase one: nothing loaded yet, the import waits and changes nothing.
    let mut s = Session::new(41);
    prepare(&mut s);
    let before = s.history_len();
    let text_only = mm_svg(
        r#"<text x="150" y="250" style="font-size:8px;font-family:'Source Serif 4';font-weight:bold">Centre me</text>"#,
    );
    let first = format!("{:?}", s.import_svg(&text_only, "t.svg").unwrap());
    assert_eq!(first, "Waiting");
    assert_eq!(s.history_len(), before, "nothing changed while waiting");
    assert!(
        s.wanted_font_files()
            .iter()
            .any(|f| f.contains("SourceSerif4-Bold")),
        "the face to load is named"
    );
    assert_eq!(import(&mut s, &text_only), "Imported(1)");
    assert_eq!(s.history_len(), before + 1);
    assert_eq!(s.selected_object_count(), 1, "the text is selected");
    // Centring: a rectangle file's centre is the view centre; a text's line box lands there.
    let faces = Faces::new();
    let mut r = fresh();
    import(
        &mut r,
        &mm_svg(r#"<rect x="150" y="250" width="20" height="10"/>"#),
    );
    let (lo, hi) = object_outline_bounds(&doc_of(&r).read_objects().objects[0]);
    let centre = pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let mut t = fresh();
    import(&mut t, &text_only);
    let got = line_box_centre(&faces, &texts(&doc_of(&t))[0]);
    assert!(
        close(got.x, centre.x, 0.01) && close(got.y, centre.y, 0.01),
        "text box centred: {got:?} vs {centre:?}"
    );
    let png = data_uri("image/png", &ink_picture("image/png"));
    let mut i = fresh();
    import(
        &mut i,
        &mm_svg(&format!(
            r#"<image x="150" y="250" width="40" height="20" xlink:href="{png}"/>"#
        )),
    );
    let img = &images(&doc_of(&i))[0];
    let c = img.placement.apply_point(pt(0.5, 0.5));
    assert!(
        close(c.x, centre.x, 0.01) && close(c.y, centre.y, 0.01),
        "image frame centred: {c:?} vs {centre:?}"
    );
    // The notice only when nothing at all came in.
    let mut n = fresh();
    let nothing = n
        .import_svg(
            &mm_svg(
                r#"<foreignObject width="5" height="5"/><image width="5" height="5" xlink:href="pic.png"/>"#,
            ),
            "poster.svg",
        )
        .unwrap();
    assert_eq!(
        format!("{nothing:?}"),
        r#"Nothing("Nothing in poster.svg could be imported.")"#
    );
}

/// Criterion 106 and the principle "permit over refuse": an Open whose listed face never
/// arrives (the host could not read it) must not lose the text when the host gives up waiting.
#[test]
fn c106_a_face_that_never_loads_loses_nothing() {
    let mut s = Session::open_file(
        42,
        &mm_svg(r#"<text x="10" y="20" style="font-size:5px;font-family:'Source Serif 4'">Serif text</text>"#),
        "f.svg",
    )
    .unwrap();
    prepare(&mut s);
    // Hand over every wanted face except the serif one.
    for file in s.wanted_font_files() {
        if !file.contains("Serif") {
            let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
            s.add_font_file(&file, &bytes);
        }
    }
    assert!(
        s.continue_open_svg(false),
        "still waiting for the serif face"
    );
    assert!(!s.continue_open_svg(true), "forced");
    let d = doc_of(&s);
    assert!(
        texts(&d).iter().any(|t| t.string == "Serif text"),
        "the text is kept"
    );
}

/// Criterion 107: the golden fixtures of slice 2 exist, and the Inkscape-made text fixtures have
/// their `name.paths.svg` (Inkscape's own Object to Path) for criterion 86's comparison.
#[test]
fn c107_golden_fixtures_and_inkscape_conversions() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-ui-core/tests/fixtures/svg");
    for name in [
        "text_live",
        "text_inkscape",
        "text_on_path",
        "text_frame",
        "images",
        "text_flowroot",
        "text_illustrator",
        "text_generators",
        "images_webp_and_others",
    ] {
        assert!(dir.join(format!("{name}.svg")).exists(), "{name}.svg");
        assert!(
            dir.join(format!("{name}.document.json")).exists(),
            "{name}.document.json"
        );
    }
    for name in ["text_inkscape", "text_on_path", "text_frame"] {
        assert!(
            dir.join(format!("{name}.paths.svg")).exists(),
            "{name}.paths.svg"
        );
    }
}

/// A generated SVG of `n` texts of 50 characters in 3 faces, a third with `dx` lists.
fn many_texts(n: usize) -> Vec<u8> {
    let mut body = String::new();
    let families = [
        "'Source Sans 3'",
        "'Source Serif 4'",
        "'Source Sans 3';font-weight:bold",
    ];
    for i in 0..n {
        let y = 5.0 + (i % 280) as f64;
        let x = 5.0 + (i / 280) as f64 * 50.0;
        let dx = if i % 3 == 0 {
            format!(r#" dx="{}""#, vec!["0.1"; 50].join(" "))
        } else {
            String::new()
        };
        body.push_str(&format!(
            r#"<text x="{x}" y="{y}"{dx} style="font-size:1px;font-family:{}">The quick brown fox jumps over the lazy dog {i:05}</text>"#,
            families[i % 3]
        ));
    }
    mm_svg(&body)
}

/// Criterion 108 (release, `#[ignore]`): 1,000 texts in under 1.5 s; one embedded 12 Mpx JPEG
/// in under 2.3 s; a 100 KB Inkscape file with 20 texts opens in under 150 ms. Thread CPU time.
#[test]
#[ignore = "budget: run in release with --ignored"]
fn c108_budgets() {
    let factor = if cfg!(debug_assertions) { 15.0 } else { 1.0 };
    let mut s = fresh();
    let bytes = many_texts(1000);
    let (outcome, t) = cpu_time::cpu_time_of(|| import(&mut s, &bytes));
    assert_eq!(outcome, "Imported(1000)");
    eprintln!("1,000 texts: {t:?}");
    assert!(t.as_secs_f64() < 1.5 * factor);
    let jpeg = std::fs::read(ink_dir().join("../../../../../target/s2t/jpeg12mpx.jpg")).expect(
        "make it with: magick -size 4000x3000 plasma:fractal -quality 85 target/s2t/jpeg12mpx.jpg",
    );
    let file = mm_svg(&format!(
        r#"<image width="400" height="300" xlink:href="{}"/>"#,
        data_uri("image/jpeg", &jpeg)
    ));
    let mut s = fresh();
    let (outcome, t) = cpu_time::cpu_time_of(|| import(&mut s, &file));
    assert_eq!(outcome, "Imported(1)");
    eprintln!("12 Mpx JPEG ({} bytes): {t:?}", jpeg.len());
    assert!(t.as_secs_f64() < 2.3 * factor);
    // 100 KB Inkscape file with 20 texts: the real text file plus filler paths.
    let mut file = String::from_utf8(ink("text_ink.svg")).unwrap();
    let mut filler = String::new();
    while file.len() + filler.len() < 100_000 {
        filler.push_str(r#"<path style="fill:none;stroke:#000000;stroke-width:0.26" d="m 10,10 c 10,-5 20,5 30,0 c 10,-5 20,5 30,0"/>"#);
    }
    file = file.replace("</svg>", &format!("{filler}</svg>"));
    let (_, t) = cpu_time::cpu_time_of(|| open(file.as_bytes()));
    eprintln!("100 KB Inkscape file: {t:?}");
    assert!(t.as_secs_f64() < 0.150 * factor);
}

/// Criterion 109: the same bytes and faces give the same document; no `format_version` change.
#[test]
fn c109_deterministic_and_format_unchanged() {
    let a = doc_of(&open(&ink("text_ink.svg")));
    let b = doc_of(&open(&ink("text_ink.svg")));
    let strip = |d: &Document| -> Vec<String> {
        texts(d)
            .into_iter()
            .map(|t| {
                format!(
                    "{:?} {:?} {:?} {:?} {:?}",
                    t.string, t.font, t.size, t.layout, t.placement
                )
            })
            .collect()
    };
    assert_eq!(strip(&a), strip(&b));
    let ia = doc_of(&open(&ink("images_ink.svg")));
    let ib = doc_of(&open(&ink("images_ink.svg")));
    let frames = |d: &Document| -> Vec<String> {
        images(d)
            .into_iter()
            .map(|i| format!("{:?} {:?}", i.asset, i.placement))
            .collect()
    };
    assert_eq!(frames(&ia), frames(&ib));
    assert_eq!(
        curvyo_document_core::CURRENT_FORMAT_VERSION,
        29,
        "SVG import adds no format change beyond document sketches (28, 29: 0066)"
    );
}

/// Criterion 109: the fuzz target of criterion 70 gets the text and image fixtures as seeds.
#[test]
fn c109_fuzz_seeds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let corpus = root.join("curvyo-svg-core/fuzz/corpus");
    assert!(corpus.exists(), "{}", corpus.display());
}

/// Criterion 110: the supported-subset page has Text and Images sections (font order, Font
/// missing, 50 MiB rule); "Importing images" has a line on SVG; the debt entry is updated.
#[test]
fn c110_documents() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let page = std::fs::read_to_string(root.join("docs/svg-import.md")).unwrap();
    for want in ["## Text", "## Images", "Font missing", "50 MiB"] {
        assert!(page.contains(want), "docs/svg-import.md: {want}");
    }
    let images = std::fs::read_to_string(root.join("docs/importing-images.md")).unwrap();
    assert!(images.contains("SVG"), "importing-images mentions SVG");
    let debt = std::fs::read_to_string(root.join("docs/technical-debt.md")).unwrap();
    let at = debt
        .find("SVG round-trip is lossy")
        .expect("the debt entry");
    let entry = &debt[at..debt[at..].find("\n## ").map_or(debt.len(), |e| at + e)];
    assert!(
        entry.contains("text") && entry.contains("image"),
        "the entry says text and images import"
    );
}

/// Criterion 98 with criterion 70's bound (release, `#[ignore]`): the hostile text files (a
/// 2,000,000-character text, 100,000 one-character texts, 10,000 font names) import through the
/// session in under 5 s of thread CPU time each, without a panic.
#[test]
#[ignore = "budget: run in release with --ignored"]
fn c98_hostile_texts_within_bounds() {
    let factor = if cfg!(debug_assertions) { 15.0 } else { 1.0 };
    let files = [
        (
            "2,000,000 characters",
            mm_svg(&format!(r#"<text x="1" y="5" style="font-size:2px">{}</text>"#, "ab ".repeat(666_667))),
        ),
        (
            "100,000 texts",
            mm_svg(
                &(0..100_000)
                    .map(|i| format!(r#"<text x="{}" y="{}" style="font-size:1px">x</text>"#, i % 200, i / 400))
                    .collect::<String>(),
            ),
        ),
        (
            "10,000 font names",
            mm_svg(
                &(0..10_000)
                    .map(|i| format!(r#"<text x="1" y="{}" style="font-size:1px;font-family:'Family {i}'">x</text>"#, i % 280))
                    .collect::<String>(),
            ),
        ),
    ];
    let mut over = Vec::new();
    for (what, bytes) in files {
        let mut s = fresh();
        // Face bytes loaded first (the substitute and Source Sans 3), so the time is the
        // importer's own: one read, conversion and commit.
        import(
            &mut s,
            &mm_svg(
                r#"<text x="1" y="1" style="font-family:Nope">a</text><text x="1" y="9">b</text>"#,
            ),
        );
        let (outcome, t) =
            cpu_time::cpu_time_of(|| format!("{:?}", s.import_svg(&bytes, "hostile.svg").unwrap()));
        eprintln!("{what}: {outcome} in {t:?}");
        assert!(outcome.starts_with("Imported("), "{what}: {outcome}");
        if t.as_secs_f64() >= 5.0 * factor {
            over.push(format!("{what}: {t:?}"));
        }
    }
    assert!(over.is_empty(), "over criterion 70's 5 s: {over:?}");
}

/// Criterion 106 on the route the Import button takes (`import_begin`/`import_add`/
/// `import_finish`): when the host gives up waiting for a face that never loads (forced after
/// 10 s), the text still comes in, drawn in the substitute, in the one step.
#[test]
fn c106_import_button_route_loses_nothing_when_a_face_never_loads() {
    let mut s = Session::new(43);
    prepare(&mut s);
    let before = s.history_len();
    let file = mm_svg(
        r#"<rect width="5" height="5"/><text x="10" y="20" style="font-size:5px;font-family:'Source Serif 4'">Serif text</text>"#,
    );
    s.import_begin(None);
    s.import_add(&file, "serif.svg");
    let first = s.import_finish(false);
    assert!(first.waiting, "waits for the serif face");
    assert_eq!(s.history_len(), before, "nothing changed while waiting");
    for f in s.wanted_font_files() {
        if !f.contains("Serif") {
            let bytes = std::fs::read(fonts_dir().join(&f)).unwrap();
            s.add_font_file(&f, &bytes);
        }
    }
    let forced = s.import_finish(true);
    assert!(!forced.waiting);
    assert_eq!(s.history_len(), before + 1, "one step");
    assert!(s.history_rows_json(0, 1).contains("Import SVG"));
    let d = doc_of(&s);
    assert!(
        texts(&d).iter().any(|t| t.string == "Serif text"),
        "the text is kept: {:?} objects, imported {}",
        d.object_ids().len(),
        forced.imported
    );
    assert_eq!(forced.imported, 2);
}
