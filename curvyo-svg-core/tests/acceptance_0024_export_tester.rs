//! Independent tester tests for the SVG writer of `specs/0024-svg-import-export` slice 4: the
//! skeleton (criterion 52), number text (54, 57), escaping and ids (53, 157), well-formed XML with
//! the required namespaces, determinism (60, 72 export part incl. `LC_ALL`), and a structural
//! check of every exported golden in `tests/fixtures/export/` (Inkscape is not available on the
//! test machine; criteria 65 and 155 stay a hand check). Written from the specification; only the
//! public types of the writer were looked up.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_possible_truncation,
    clippy::collapsible_if,
    missing_docs
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use curvyo_svg_core::{
    Cap, DisplayUnitName, ExportImage, ExportKind, ExportLeaf, ExportNode, ExportPage, ExportState,
    ExportStroke, ExportStyle, Join, PageBackground, Paint, PathId, Shadow, SvgExport, read_svg,
    write_svg,
};
use kurbo::{Affine, BezPath, Point, Rect};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const INKSCAPE_NS: &str = "http://www.inkscape.org/namespaces/inkscape";
const SODIPODI_NS: &str = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd";
const CURVYO_NS: &str = "urn:curvyo:svg:1";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

fn stroke(width: f64) -> ExportStroke {
    ExportStroke {
        paint: Paint {
            rgb: [0, 0, 0],
            opacity: 1.0,
        },
        width_mm: width,
        dash_mm: Vec::new(),
        cap: Cap::Butt,
        join: Join::Miter,
        start: None,
        mid: None,
        end: None,
    }
}

fn state(name: Option<&str>) -> ExportState {
    ExportState {
        name: name.map(str::to_owned),
        ..ExportState::default()
    }
}

fn leaf(name: Option<&str>, kind: ExportKind) -> ExportNode {
    ExportNode::Leaf {
        state: state(name),
        leaf: Box::new(ExportLeaf {
            kind,
            style: ExportStyle {
                fill: None,
                stroke: Some(stroke(0.25)),
            },
            shadow: Shadow::default(),
        }),
    }
}

fn layer(name: &str, children: Vec<ExportNode>) -> ExportNode {
    ExportNode::Group {
        state: state(Some(name)),
        layer: true,
        children,
    }
}

fn drawing(nodes: Vec<ExportNode>) -> SvgExport {
    SvgExport {
        size: (210.0, 297.0),
        pages: vec![ExportPage {
            name: "Page 1".to_owned(),
            rect: Rect::new(0.0, 0.0, 210.0, 297.0),
        }],
        display_unit: DisplayUnitName::Mm,
        background: PageBackground::Colour([255, 255, 255], 0.0),
        nodes,
        guides: Vec::new(),
    }
}

fn path_kind(points: &[(f64, f64)]) -> ExportKind {
    let mut p = BezPath::new();
    p.move_to(points[0]);
    for &q in &points[1..] {
        p.line_to(q);
    }
    ExportKind::Path {
        node_types: "c".repeat(points.len()),
        path: p,
        id: PathId::Path,
    }
}

/// Every element with this local name (any namespace).
fn elements<'a>(doc: &'a roxmltree::Document<'a>, name: &str) -> Vec<roxmltree::Node<'a, 'a>> {
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == name)
        .collect()
}

#[test]
fn c52_skeleton_of_the_spec_parses_with_its_namespaces() {
    let svg = drawing(vec![layer(
        "Cut",
        vec![leaf(
            None,
            ExportKind::Rect {
                rect: Rect::new(10.0, 20.0, 40.0, 60.0),
                radius: 0.0,
                rotation_deg: 0.0,
            },
        )],
    )]);
    let text = write_svg(&svg);
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some(SVG_NS));
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(
        root.lookup_namespace_uri(Some("inkscape")),
        Some(INKSCAPE_NS)
    );
    assert_eq!(
        root.lookup_namespace_uri(Some("sodipodi")),
        Some(SODIPODI_NS)
    );
    assert_eq!(root.lookup_namespace_uri(Some("curvyo")), Some(CURVYO_NS));
    assert_eq!(root.attribute("width"), Some("210mm"));
    assert_eq!(root.attribute("height"), Some("297mm"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 210 297"));
    assert_eq!(root.attribute((CURVYO_NS, "format")), Some("1"));
    let kids: Vec<_> = root
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    assert_eq!(kids[0].tag_name().name(), "namedview", "namedview first");
    assert_eq!(kids[0].tag_name().namespace(), Some(SODIPODI_NS));
    assert_eq!(
        kids[0].attribute((INKSCAPE_NS, "document-units")),
        Some("mm")
    );
    assert_eq!(kids[0].attribute("pagecolor"), Some("#ffffff"));
    assert_eq!(kids[0].attribute((INKSCAPE_NS, "pageopacity")), Some("0"));
    assert_eq!(kids[1].attribute((INKSCAPE_NS, "groupmode")), Some("layer"));
    assert_eq!(kids[1].attribute((INKSCAPE_NS, "label")), Some("Cut"));
    assert_eq!(kids[1].attribute("id"), Some("layer1"));
    let rect = kids[1].first_element_child().unwrap();
    assert_eq!(rect.tag_name().name(), "rect");
    assert_eq!(rect.attribute("id"), Some("rect1"));
    for (k, v) in [
        ("x", "10"),
        ("y", "20"),
        ("width", "30"),
        ("height", "40"),
        ("fill", "none"),
        ("stroke", "#000000"),
        ("stroke-width", "0.25"),
    ] {
        assert_eq!(rect.attribute(k), Some(v), "{k}");
    }
    assert_eq!(root.lookup_namespace_uri(Some("xlink")), None);
    // The reader reads its own output.
    read_svg(text.as_bytes()).expect("our reader reads the writer's output");
}

#[test]
fn c54_numbers_in_path_data() {
    let cases: &[(f64, &str)] = &[
        (0.0, "0"),
        (-0.0, "0"),
        (-1e-9, "0"),
        (-0.0000004, "0"),
        (1.0 / 3.0, "0.333333"),
        (2.0 / 3.0, "0.666667"),
        (0.1 + 0.2, "0.3"),
        (1.5e-6, "0.000002"),
        (-2.5, "-2.5"),
        (1e7, "10000000"),
        (123_456_789.125, "123456789.125"),
        (-9_999_999.999_999_9, "-10000000"),
        (1e15, "1000000000000000"),
    ];
    for &(v, want) in cases {
        let text = write_svg(&drawing(vec![leaf(None, path_kind(&[(v, 1.0), (2.0, v)]))]));
        let doc = roxmltree::Document::parse(&text).unwrap();
        let d = elements(&doc, "path")[0].attribute("d").unwrap().to_owned();
        assert_eq!(d, format!("M {want} 1 L 2 {want}"), "value {v:e}");
    }
}

#[test]
fn c53_c157_names_are_escaped_text_without_control_or_bidi() {
    let nasty = "Tom & Jerry <script>alert(1)</script> \"q\" 'a' \u{202E}rtl\u{2067}\u{0001}\u{FEFF}\u{200E}x";
    let mut svg = drawing(vec![layer(
        nasty,
        vec![leaf(Some(nasty), path_kind(&[(0.0, 0.0), (1.0, 1.0)]))],
    )]);
    svg.pages = vec![
        ExportPage {
            name: nasty.to_owned(),
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        },
        ExportPage {
            name: "B".to_owned(),
            rect: Rect::new(110.0, 0.0, 210.0, 100.0),
        },
    ];
    let text = write_svg(&svg);
    assert!(!text.contains("<script>"));
    for c in ['\u{202E}', '\u{2067}', '\u{0001}', '\u{200E}'] {
        assert!(!text.contains(c), "{c:?} written");
    }
    let doc = roxmltree::Document::parse(&text).expect("well-formed");
    let want = "Tom & Jerry <script>alert(1)</script> \"q\" 'a' rtl\u{FEFF}x";
    let labels: Vec<&str> = doc
        .descendants()
        .filter_map(|n| n.attribute((INKSCAPE_NS, "label")))
        .collect();
    assert_eq!(labels.len(), 4, "page, page, layer, leaf: {labels:?}");
    // U+FEFF is not a bidi control; the spec names control and bidirectional control characters.
    for l in &labels[..1] {
        assert!(l.starts_with("Tom & Jerry <script>"), "{l}");
    }
    assert!(
        labels.contains(&want)
            || labels
                .iter()
                .any(|l| l.replace('\u{FEFF}', "") == want.replace('\u{FEFF}', ""))
    );
    assert!(
        elements(&doc, "script").is_empty(),
        "no element from a name"
    );
}

#[test]
fn c53_ids_are_numbered_per_kind_in_document_order() {
    let rect = |x: f64| ExportKind::Rect {
        rect: Rect::new(x, 0.0, x + 1.0, 1.0),
        radius: 0.0,
        rotation_deg: 0.0,
    };
    let ell = |r: f64| ExportKind::Ellipse {
        center: Point::new(5.0, 5.0),
        rx: r,
        ry: 2.0,
        rotation_deg: 0.0,
    };
    let svg = drawing(vec![
        layer(
            "A",
            vec![
                leaf(None, rect(0.0)),
                leaf(None, ell(2.0)),
                leaf(None, path_kind(&[(0.0, 0.0), (1.0, 0.0)])),
            ],
        ),
        ExportNode::Group {
            state: state(None),
            layer: false,
            children: vec![leaf(None, ell(3.0)), leaf(None, rect(5.0))],
        },
        layer(
            "B",
            vec![
                leaf(None, path_kind(&[(0.0, 0.0), (1.0, 2.0)])),
                leaf(None, ell(2.0 + 1e-12)),
            ],
        ),
    ]);
    let text = write_svg(&svg);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let ids: Vec<&str> = doc
        .descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| n.attribute("id"))
        .collect();
    assert_eq!(
        ids,
        [
            "namedview1",
            "layer1",
            "rect1",
            "circle1",
            "path1",
            "g1",
            "ellipse1",
            "rect2",
            "layer2",
            "path2",
            "circle2"
        ],
        "ids per kind in document order (equal radii within 1e-9 are a circle)"
    );
}

#[test]
fn c137_image_bytes_are_padded_base64_without_breaks() {
    for len in [0usize, 1, 2, 3, 4, 5, 300] {
        let bytes: Vec<u8> = (0..len).map(|i| (i * 37 % 256) as u8).collect();
        let svg = drawing(vec![ExportNode::Image {
            state: state(Some("pic")),
            image: Box::new(ExportImage {
                mime: "image/png",
                bytes: Arc::from(bytes.clone()),
                placement: Affine::new([20.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
            }),
        }]);
        let text = write_svg(&svg);
        let doc = roxmltree::Document::parse(&text).unwrap();
        let img = elements(&doc, "image")[0];
        let href = img
            .attribute((XLINK_NS, "href"))
            .expect("xlink:href, xlink declared");
        let b64 = href.strip_prefix("data:image/png;base64,").unwrap();
        assert!(!b64.contains(['\n', '\r', ' ']));
        assert_eq!(b64.len() % 4, 0, "padded");
        assert_eq!(
            curvyo_svg_core::decode_data_uri(href),
            Some(bytes),
            "len {len}"
        );
    }
}

#[test]
fn c60_c72_writer_is_deterministic_and_locale_free() {
    let make = || {
        let mut svg = drawing(vec![layer(
            "L",
            (0..50)
                .map(|i| {
                    leaf(
                        Some(&format!("n{i}")),
                        path_kind(&[(f64::from(i) / 7.0, 0.25), (1.0e6 / f64::from(i + 1), -0.5)]),
                    )
                })
                .collect(),
        )]);
        svg.background = PageBackground::Colour([1, 2, 3], 0.333);
        write_svg(&svg)
    };
    if let Ok(path) = std::env::var("CURVYO_TESTER_DUMP_SVG") {
        std::fs::write(path, make()).unwrap();
        return;
    }
    let a = make();
    assert_eq!(a, make());
    let dir = std::env::temp_dir().join(format!("curvyo-0024-svgcore-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("de.svg");
    let ok = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "c60_c72_writer_is_deterministic_and_locale_free"])
        .env("CURVYO_TESTER_DUMP_SVG", &out)
        .env("LC_ALL", "de_DE.UTF-8")
        .env("LANG", "de_DE.UTF-8")
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let german = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(german, a, "LC_ALL=de_DE.UTF-8 changes nothing");
}

fn exported_goldens() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.to_string_lossy().ends_with(".exported.svg") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        &mut out,
    );
    out.sort();
    out
}

/// Structural validation of every exported golden (Inkscape is not on this machine): well-formed
/// XML, the root and namespaces of criterion 52, ids unique, every `href` resolves inside the
/// file, every `url(#…)` resolves to a `<marker>`, no `style` other than `display:none`, path
/// data only `M L C Z`, no exponent and no `-0` in any standard number.
#[test]
fn c52_c61_c153_every_exported_golden_is_structurally_valid() {
    let files = exported_goldens();
    assert!(files.len() >= 20, "goldens found: {}", files.len());
    let mut problems = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let name = f
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .display()
            .to_string();
        let doc = match roxmltree::Document::parse(&text) {
            Ok(d) => d,
            Err(e) => {
                problems.push(format!("{name}: not XML: {e}"));
                continue;
            }
        };
        let root = doc.root_element();
        let checks = [
            (root.tag_name().namespace() == Some(SVG_NS), "svg namespace"),
            (
                root.lookup_namespace_uri(Some("inkscape")) == Some(INKSCAPE_NS),
                "inkscape ns",
            ),
            (
                root.lookup_namespace_uri(Some("sodipodi")) == Some(SODIPODI_NS),
                "sodipodi ns",
            ),
            (
                root.lookup_namespace_uri(Some("curvyo")) == Some(CURVYO_NS),
                "curvyo ns",
            ),
            (
                root.attribute((CURVYO_NS, "format")) == Some("1"),
                "curvyo:format",
            ),
            (
                root.attribute("width").is_some_and(|w| w.ends_with("mm")),
                "width in mm",
            ),
            (
                root.attribute("viewBox")
                    .is_some_and(|v| v.starts_with("0 0 ")),
                "viewBox",
            ),
        ];
        for (ok, what) in checks {
            if !ok {
                problems.push(format!("{name}: {what}"));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for n in doc.descendants().filter(roxmltree::Node::is_element) {
            if let Some(id) = n.attribute("id") {
                if !ids.insert(id.to_owned()) {
                    problems.push(format!("{name}: duplicate id {id}"));
                }
            }
        }
        let uses_xlink = doc
            .descendants()
            .any(|n| n.attribute((XLINK_NS, "href")).is_some());
        if uses_xlink != root.lookup_namespace_uri(Some("xlink")).is_some() {
            problems.push(format!("{name}: xlink declared iff used"));
        }
        for n in doc.descendants().filter(roxmltree::Node::is_element) {
            if let Some(h) = n.attribute((XLINK_NS, "href")) {
                if let Some(id) = h.strip_prefix('#') {
                    if !ids.contains(id) {
                        problems.push(format!("{name}: href {h} unresolved"));
                    }
                } else if !h.starts_with("data:image/") {
                    problems.push(format!("{name}: external href {h}"));
                }
            }
            for a in ["marker-start", "marker-mid", "marker-end"] {
                if let Some(u) = n.attribute(a) {
                    let id = u.trim_start_matches("url(#").trim_end_matches(')');
                    let ok = doc
                        .descendants()
                        .any(|m| m.tag_name().name() == "marker" && m.attribute("id") == Some(id));
                    if !ok {
                        problems.push(format!("{name}: {a} {u} unresolved"));
                    }
                }
            }
            if let Some(s) = n.attribute("style") {
                if s != "display:none" {
                    problems.push(format!("{name}: style {s}"));
                }
            }
            if n.tag_name().name() == "path" {
                let d = n.attribute("d").unwrap_or("");
                for t in d.split_whitespace() {
                    if t.parse::<f64>().is_err() && !["M", "L", "C", "Z"].contains(&t) {
                        problems.push(format!("{name}: command {t}"));
                        break;
                    }
                }
            }
            for a in n.attributes() {
                if a.name() == "href" || a.name() == "label" || a.namespace() == Some(CURVYO_NS) {
                    continue;
                }
                for t in a
                    .value()
                    .split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')')
                {
                    if t.parse::<f64>().is_ok() && (t == "-0" || t.contains(['e', 'E'])) {
                        problems.push(format!("{name}: number {t} in {}", a.name()));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// Criterion 153 lists the export documents; each needs its golden beside it.
#[test]
fn c153_the_listed_export_goldens_exist() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export");
    let listed = [
        "export_pages",
        "export_pages_active",
        "export_selection",
        "export_text",
        "export_text_paths",
        "export_text_on_path",
        "export_images",
        "export_sketch",
        "export_arcs",
        "export_curved",
        "export_markers",
        "export_guides",
        "export_strokes_paths",
        "export_all_options",
    ];
    let missing: Vec<&str> = listed
        .iter()
        .copied()
        .filter(|n| !dir.join(format!("{n}.exported.svg")).exists())
        .collect();
    let no_document: Vec<&str> = listed
        .iter()
        .copied()
        .filter(|n| !dir.join(format!("{n}.document.json")).exists())
        .collect();
    assert!(
        missing.is_empty() && no_document.is_empty(),
        "criterion 153: missing exported goldens {missing:?}; missing name.document.json inputs {no_document:?}"
    );
    assert!(
        dir.join("curvyo_all.svg").exists()
            || dir.parent().unwrap().join("svg/curvyo_all.svg").exists(),
        "criterion 153: curvyo_all.svg (criterion 75 item 12) pinned"
    );
}
