//! Independent tester tests for `specs/0024-svg-import-export` (import slice) at the reader:
//! real files (Inkscape 1.4.4 saves, the W3C SVG 1.1 test suite, Wikimedia Commons public-domain
//! files and Illustrator exports from MIT/Apache test suites, `tests/fixtures/svg_real/`), each
//! checked against Inkscape's own geometry, plus the limits and boundaries of the criteria.
//! Written from the specification before the implementation was read; only the public types were
//! looked up. Expected values are the specification's arithmetic or Inkscape's measurement.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::needless_raw_string_hashes,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::many_single_char_names,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::format_push_string,
    clippy::cast_possible_truncation,
    missing_docs
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use curvyo_svg_core::{
    DisplayUnitName, LeafKind, NodeState, PageBackground, SvgDrawing, SvgError, SvgLeaf, SvgNode,
    read_svg,
};
use kurbo::{Affine, BezPath, Rect, Shape};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;
const PX: f64 = 25.4 / 96.0;
const PT: f64 = 25.4 / 72.0;

// ---- helpers ------------------------------------------------------------------------------

fn real_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg_real")
}

fn real(rel: &str) -> Vec<u8> {
    std::fs::read(real_dir().join(rel)).unwrap()
}

fn read_real(rel: &str) -> SvgDrawing {
    read_svg(&real(rel)).unwrap_or_else(|e| panic!("{rel}: {e:?}"))
}

fn read(text: &str) -> SvgDrawing {
    read_svg(text.as_bytes()).unwrap_or_else(|e| panic!("{e:?} for {text}"))
}

/// A 100 x 100 mm drawing, one user unit = 1 mm.
fn mm(body: &str) -> SvgDrawing {
    read(&format!(
        r#"<svg {NS} width="100mm" height="100mm" viewBox="0 0 100 100">{body}</svg>"#
    ))
}

fn leaves(nodes: &[SvgNode]) -> Vec<(&NodeState, &SvgLeaf)> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => out.extend(leaves(children)),
            SvgNode::Leaf { state, leaf } => out.push((state, leaf)),
        }
    }
    out
}

/// The outline of leaf `i` in millimetres, clones resolved through their chain.
fn outline(all: &[(&NodeState, &SvgLeaf)], i: usize) -> BezPath {
    match &all[i].1.kind {
        LeafKind::Clone { original, matrix } => *matrix * outline(all, *original),
        other => other.outline_mm().unwrap(),
    }
}

fn bounds(all: &[(&NodeState, &SvgLeaf)], i: usize) -> Rect {
    outline(all, i).bounding_box()
}

fn by_name<'a>(all: &'a [(&'a NodeState, &'a SvgLeaf)], name: &str) -> (usize, &'a SvgLeaf) {
    let i = all
        .iter()
        .position(|(s, _)| s.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no leaf named {name}"));
    (i, all[i].1)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn rect_close(a: Rect, b: Rect, tol: f64) -> bool {
    close(a.x0, b.x0, tol)
        && close(a.y0, b.y0, tol)
        && close(a.x1, b.x1, tol)
        && close(a.y1, b.y1, tol)
}

fn page(d: &SvgDrawing) -> (f64, f64) {
    d.page.expect("a page")
}

fn assert_page(d: &SvgDrawing, w: f64, h: f64, tol: f64) {
    let (pw, ph) = page(d);
    assert!(
        close(pw, w, tol) && close(ph, h, tol),
        "page {pw} x {ph}, expected {w} x {h}"
    );
}

fn top_groups(d: &SvgDrawing) -> Vec<(&NodeState, bool, &[SvgNode])> {
    d.nodes
        .iter()
        .filter_map(|n| match n {
            SvgNode::Group {
                state,
                layer,
                children,
            } => Some((state, *layer, children.as_slice())),
            SvgNode::Leaf { .. } => None,
        })
        .collect()
}

fn fill_rgb(leaf: &SvgLeaf) -> Option<[u8; 3]> {
    leaf.style.fill.map(|p| p.rgb)
}

/// Whether `m` is a move, rotation or flip (criterion 47 (iii)).
fn isometry(m: Affine) -> bool {
    let [a, b, c, d, _, _] = m.as_coeffs();
    close(a * a + b * b, 1.0, 1e-6)
        && close(c * c + d * d, 1.0, 1e-6)
        && close(a * c + b * d, 0.0, 1e-6)
}

// ---- the Inkscape oracle --------------------------------------------------------------------

/// Compares the bounds of every leaf of `rel` with Inkscape 1.4.4's geometric bounds of the
/// elements that draw as one leaf (`.inkscape-bbox.csv`, Inkscape px; how it was made is in
/// `SOURCES.md`); `px_mm` converts them to millimetres. Each oracle box must be matched by a
/// different imported leaf within `tol` mm. Returns the misses and the number of boxes checked.
fn check_against_inkscape(rel: &str, px_mm: f64) -> (Vec<String>, usize) {
    let drawing = read_svg(&real(rel)).unwrap();
    let all = leaves(&drawing.nodes);
    let boxes: Vec<Rect> = (0..all.len()).map(|i| bounds(&all, i)).collect();
    let mut used = vec![false; boxes.len()];
    let csv = std::fs::read_to_string(real_dir().join(rel.replace(".svg", ".inkscape-bbox.csv")))
        .unwrap();
    let mut misses = Vec::new();
    let mut checked = 0;
    for line in csv.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let v: Vec<f64> = f[1..5]
            .iter()
            .map(|x| x.parse::<f64>().unwrap() * px_mm)
            .collect();
        if v[2] == 0.0 && v[3] == 0.0 {
            continue; // a singular transform: the element is skipped (criterion 31)
        }
        let want = Rect::new(v[0], v[1], v[0] + v[2], v[1] + v[3]);
        let tol = 0.002 + 2e-5 * want.x1.abs().max(want.y1.abs());
        checked += 1;
        match (0..boxes.len()).find(|&i| !used[i] && rect_close(boxes[i], want, tol)) {
            Some(i) => used[i] = true,
            None => misses.push(format!("{rel}: #{} expected {want:?}", f[0])),
        }
    }
    (misses, checked)
}

/// Criteria 15 to 18, 21 to 24, 26, 27, 31 to 33, 35, 47: every real file's geometry agrees with
/// Inkscape's within 2 µm (+ 20 ppm), element by element.
#[test]
fn ac15_to_ac47_real_files_match_inkscape_geometry() {
    let mut misses = Vec::new();
    let mut checked = 0;
    for dir in ["inkscape", "w3c", "wikimedia", "illustrator"] {
        let mut names: Vec<_> = std::fs::read_dir(real_dir().join(dir))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".svg"))
            .collect();
        names.sort();
        for name in names {
            let rel = format!("{dir}/{name}");
            if read_svg(&real(&rel)).is_err() {
                continue; // refusals are checked on their own
            }
            // Illustrator writes points as px (criterion 19 (a)); Inkscape reads 96 per inch.
            let px_mm = if dir == "illustrator" { PT } else { PX };
            let (m, n) = check_against_inkscape(&rel, px_mm);
            assert!(
                n > 0 || name.starts_with("Biohazard"),
                "{rel}: nothing checked"
            );
            checked += n;
            misses.extend(m);
        }
    }
    assert!(checked > 300, "{checked} boxes checked");
    assert!(
        misses.is_empty(),
        "{} of {checked} boxes missed:\n{}",
        misses.len(),
        misses.join("\n")
    );
}

// ---- real files: Inkscape 1.4.4 ---------------------------------------------------------------

/// Criteria 15, 20 (b), 20 (c), 45: Inkscape's default A4 mm page, its page colour at
/// `inkscape:pageopacity="0.0"` is no background, layers with their states.
#[test]
fn ac15_ac20_ac45_inkscape_a4_page_namedview_and_layers() {
    let d = read_real("inkscape/inkscape_mm_a4.svg");
    assert_page(&d, 210.0, 297.0, 1e-9);
    assert_eq!(d.display_unit, Some(DisplayUnitName::Mm));
    assert_eq!(
        d.background,
        Some(PageBackground::None),
        "pageopacity 0 is no background"
    );
    let layers = top_groups(&d);
    let names: Vec<_> = layers
        .iter()
        .map(|(s, _, _)| s.name.as_deref().unwrap())
        .collect();
    assert_eq!(names, ["Cut", "Engrave", "Reference", "Empty"]);
    assert!(layers.iter().all(|(_, layer, _)| *layer));
    let (cut, engrave, reference, empty) = (layers[0].0, layers[1].0, layers[2].0, layers[3]);
    assert!(!cut.hidden && !cut.locked && cut.opacity == 1.0);
    assert!(engrave.locked && close(engrave.opacity, 0.5, 1e-12) && !engrave.hidden);
    assert!(reference.hidden);
    assert!(empty.2.is_empty(), "an empty layer is kept (criterion 45)");
    assert!(!d.truncated);
}

/// Criteria 26, 27, 28, 33 on genuine Inkscape output: rectangles, rounded, rx != ry, a rotated
/// rectangle, circle, ellipse, the star and polygon tool (`d` rebuilt by Inkscape).
#[test]
fn ac26_to_ac28_ac33_inkscape_primitives() {
    let d = read_real("inkscape/inkscape_mm_a4.svg");
    let all = leaves(&d.nodes);
    let LeafKind::Rect { rx, ry, .. } = by_name(&all, "r2").1.kind else {
        panic!("r2 is a rectangle")
    };
    assert!(
        close(rx, 4.0, 1e-9) && close(ry, 4.0, 1e-9),
        "ry takes rx (criterion 26)"
    );
    let LeafKind::Rect { rx, ry, .. } = by_name(&all, "r3").1.kind else {
        panic!("r3 is read as a rectangle; the mapping makes it a path")
    };
    assert!(close(rx, 6.0, 1e-9) && close(ry, 3.0, 1e-9));
    let LeafKind::Rect { rect, matrix, .. } = by_name(&all, "r4").1.kind else {
        panic!("rotated rectangle")
    };
    // Inkscape wrote rotate(30); the rectangle is still 30 x 20.
    assert!(close(rect.width(), 30.0, 1e-9) && close(rect.height(), 20.0, 1e-9));
    let [a, b, ..] = matrix.as_coeffs();
    assert!(close(b.atan2(a).to_degrees(), 30.0, 1e-9));
    assert!(
        matches!(by_name(&all, "c1").1.kind, LeafKind::Ellipse { rx, ry, .. } if rx == 10.0 && ry == 10.0)
    );
    assert!(
        matches!(by_name(&all, "e1").1.kind, LeafKind::Ellipse { rx, ry, .. } if rx == 15.0 && ry == 8.0)
    );
    let LeafKind::Star { star, .. } = by_name(&all, "s1").1.kind else {
        panic!("Inkscape star")
    };
    assert_eq!(star.sides, 5);
    assert!(!star.flat && close(star.r1, 12.0, 1e-9) && close(star.r2, 5.0, 1e-9));
    let LeafKind::Star { star, .. } = by_name(&all, "s2").1.kind else {
        panic!("Inkscape polygon")
    };
    assert!(star.flat && star.sides == 6);
}

/// Criterion 47 on genuine Inkscape Alt+D clones: moved, rotated, mirrored and a clone inside a
/// rotated group are linked clones with an isometric M; a scaled clone and a clone of a rectangle
/// are copies.
#[test]
fn ac47_inkscape_clones() {
    let d = read_real("inkscape/inkscape_mm_a4.svg");
    let all = leaves(&d.nodes);
    let (p1, _) = by_name(&all, "p1");
    for (name, det) in [
        ("clone_moved", 1.0),
        ("clone_rot30", 1.0),
        ("clone_flip", -1.0),
    ] {
        let LeafKind::Clone { original, matrix } = by_name(&all, name).1.kind else {
            panic!("{name} is a linked clone")
        };
        assert_eq!(original, p1, "{name}");
        assert!(isometry(matrix), "{name}: {matrix:?}");
        assert!(close(matrix.determinant(), det, 1e-9), "{name}");
    }
    let LeafKind::Clone { matrix, .. } = by_name(&all, "clone_moved").1.kind else {
        unreachable!()
    };
    // Inkscape: transform="translate(0,10.583333)" (40 px).
    assert!(close(matrix.as_coeffs()[5], 10.583_333, 1e-9));
    let (_, clone_in_group) = by_name(&all, "clone_in_rot_group");
    let (p2, _) = by_name(&all, "p2");
    assert!(
        matches!(clone_in_group.kind, LeafKind::Clone { original, .. } if original == p2),
        "a clone in a rotated group: {:?}",
        clone_in_group.kind
    );
    assert!(!matches!(
        by_name(&all, "clone_scaled").1.kind,
        LeafKind::Clone { .. }
    ));
    assert!(!matches!(
        by_name(&all, "clone_of_rect").1.kind,
        LeafKind::Clone { .. }
    ));
    // The copy of the scaled clone is 2 x 20 mm and keeps the original's red fill.
    let (i, scaled) = by_name(&all, "clone_scaled");
    assert!(close(bounds(&all, i).width(), 40.0, 1e-6));
    assert_eq!(fill_rgb(scaled), Some([255, 0, 0]));
}

/// Criteria 24, 50, 51 inputs: Path > Combine keeps one compound path with `fill-rule:evenodd`;
/// the text converted to paths keeps every glyph outline in one path.
#[test]
fn ac24_ac49_inkscape_combine_and_text_to_path() {
    let d = read_real("inkscape/inkscape_mm_a4.svg");
    let all = leaves(&d.nodes);
    for name in ["donut", "overlap"] {
        let (i, leaf) = by_name(&all, name);
        assert!(leaf.style.even_odd, "{name}");
        let subpaths = outline(&all, i)
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
            .count();
        assert_eq!(subpaths, 2, "{name}");
    }
    // "Boho" as paths: 4 glyphs with holes in B (2), o (1), o (1): at least 8 outlines; Inkscape
    // wrote no fill, so the fill is the default black.
    let t = all
        .iter()
        .find(|(_, l)| {
            matches!(&l.kind, LeafKind::Path { path, .. }
                if path.elements().iter().filter(|e| matches!(e, kurbo::PathEl::MoveTo(_))).count() >= 8)
        })
        .expect("the converted text");
    assert_eq!(fill_rgb(t.1), Some([0, 0, 0]));
}

/// The plain-SVG export of the same file: no layers (no Inkscape namespace), stars are plain
/// paths, same geometry (checked by the oracle test), no background.
#[test]
fn ac45_ac28_inkscape_plain_svg_has_groups_not_layers() {
    let d = read_real("inkscape/inkscape_mm_a4_plain.svg");
    assert_page(&d, 210.0, 297.0, 1e-9);
    assert!(top_groups(&d).iter().all(|(_, layer, _)| !layer));
    let all = leaves(&d.nodes);
    assert!(
        all.iter()
            .all(|(_, l)| !matches!(l.kind, LeafKind::Star { .. }))
    );
    assert_ne!(
        d.background,
        Some(PageBackground::Colour([255, 255, 255], 1.0))
    );
}

/// Criteria 15, 20 (b), 20 (c), 36 on an Inkscape file in inches with a `<style>` sheet:
/// presentation attribute < sheet rule (element < class < id) < `style` attribute; a rule with a
/// combinator or pseudo-class is skipped; the page colour at opacity 0.5 is the background.
#[test]
fn ac15_ac20_ac36_inkscape_inches_and_css() {
    for rel in [
        "inkscape/inkscape_in_css.svg",
        "inkscape/inkscape_in_css_plain.svg",
    ] {
        let d = read_real(rel);
        assert_page(&d, 203.2, 101.6, 1e-9);
        let all = leaves(&d.nodes);
        if rel.ends_with("css.svg") {
            assert_eq!(d.display_unit, Some(DisplayUnitName::In));
            let Some(PageBackground::Colour(rgb, alpha)) = d.background else {
                panic!("{:?}", d.background)
            };
            assert_eq!(rgb, [0x33, 0x66, 0x99]);
            assert!(close(alpha, 0.5, 1e-12));
        }
        let fills: Vec<_> = all.iter().take(4).map(|(_, l)| fill_rgb(l)).collect();
        assert_eq!(
            fills,
            [
                Some([0, 0, 255]),        // a1: `rect` rule; `g > rect` is skipped
                None,                     // a2: `.cut` fill none
                Some([0, 255, 0]),        // a3: `#special` beats `.cut` and the attribute
                Some([0xab, 0xcd, 0xef])  // a4: the style attribute wins
            ],
            "{rel}"
        );
        let a2 = all[1].1;
        assert_eq!(a2.style.stroke.map(|p| p.rgb), Some([255, 0, 0]));
        assert!(
            close(a2.style.stroke_width_mm.unwrap(), 2.0 * PX, 1e-9),
            "{rel}"
        );
        // b1 inherits fill="#808000" from G1; `path:hover` is skipped.
        assert_eq!(fill_rgb(all[4].1), Some([0x80, 0x80, 0x00]), "{rel}");
        let tri = all.iter().rev().nth(1).unwrap().1;
        assert!(tri.style.even_odd);
    }
}

/// Criterion 20 (c), Inkscape's page: background only with an opacity above 0.
#[test]
fn ac20c_page_colour_needs_page_opacity() {
    let nv = |attrs: &str| {
        read(&format!(
            r##"<svg {NS} width="10mm" height="10mm" viewBox="0 0 10 10"><sodipodi:namedview pagecolor="#ff8000" {attrs}/><rect width="1" height="1"/></svg>"##
        ))
        .background
    };
    assert_eq!(
        nv(r#"inkscape:pageopacity="0""#),
        Some(PageBackground::None)
    );
    let Some(PageBackground::Colour(rgb, a)) = nv(r#"inkscape:pageopacity="1""#) else {
        panic!()
    };
    assert_eq!(rgb, [255, 128, 0]);
    assert!(close(a, 1.0, 1e-12));
    // A missing opacity is "none" (criterion 20 (c)): no colour at all.
    assert!(!matches!(nv(""), Some(PageBackground::Colour(..))));
}

// ---- real files: W3C SVG 1.1 test suite ------------------------------------------------------

/// Criterion 17: `width="100%" height="100%"` with a viewBox: the page is the viewBox in px.
#[test]
fn ac17_w3c_percent_size_page() {
    let d = read_real("w3c/shapes-rect-01-t.svg");
    assert_page(&d, 480.0 * PX, 360.0 * PX, 1e-9);
}

/// Criterion 36: `styling-css-01-b`: element and class selectors, including a multi-class
/// attribute (`class="foo bar baz"` matches `.bar`); the test passes when all six shapes are green.
#[test]
fn ac36_w3c_styling_css_01_all_six_shapes_green() {
    let d = read_real("w3c/styling-css-01-b.svg");
    let filled: Vec<_> = leaves(&d.nodes)
        .into_iter()
        .filter_map(|(_, l)| fill_rgb(l))
        .collect();
    assert_eq!(filled.len(), 6, "{filled:?}");
    assert!(filled.iter().all(|c| *c == [0, 128, 0]), "{filled:?}");
}

/// Criterion 36: `styling-css-02-b`: `#one, #two` apply; the attribute selector and the
/// descendant combinator are skipped, so those two shapes keep their group's red.
#[test]
fn ac36_w3c_styling_css_02_ids_apply_attribute_selectors_skipped() {
    let d = read_real("w3c/styling-css-02-b.svg");
    let filled: Vec<_> = leaves(&d.nodes)
        .into_iter()
        .filter_map(|(_, l)| fill_rgb(l))
        .collect();
    let green = [0, 128, 0];
    let red = [255, 0, 0];
    assert_eq!(filled, [green, green, green, red, green, red]);
}

/// Criterion 37: `currentColor` takes the inherited or own `color` (`painting-fill-02-t`).
#[test]
fn ac37_w3c_current_color() {
    let d = read_real("w3c/painting-fill-02-t.svg");
    let filled: Vec<_> = leaves(&d.nodes)
        .into_iter()
        .filter_map(|(_, l)| fill_rgb(l))
        .collect();
    assert_eq!(filled, [[0, 128, 0], [0, 0, 255]]);
}

/// Criterion 47 (b): `struct-use-01-t`: uses of targets inside `defs` are copies; the copy takes
/// the `use`'s fill where the target sets none, and the style of the target's `defs` ancestors
/// (fill red, stroke yellow) does not leak into it (SVG: the copy inherits from the `use`).
#[test]
fn ac47b_w3c_struct_use_01_copies_inherit_from_the_use() {
    let d = read_real("w3c/struct-use-01-t.svg");
    let all = leaves(&d.nodes);
    assert!(
        all.iter()
            .all(|(_, l)| !matches!(l.kind, LeafKind::Clone { .. }))
    );
    // rect, circle, ellipse, polygon, path: green fill, no stroke.
    let greens = all
        .iter()
        .filter(|(_, l)| fill_rgb(l) == Some([0, 255, 0]) && l.style.stroke.is_none())
        .count();
    assert!(greens >= 5, "{greens} green unstroked copies");
    assert!(
        all.iter().all(|(_, l)| fill_rgb(l) != Some([255, 0, 0])),
        "the defs group's red fill leaked into a copy"
    );
    assert!(
        all.iter()
            .all(|(_, l)| l.style.stroke.map(|p| p.rgb) != Some([255, 255, 0])),
        "the defs group's yellow stroke leaked into a copy"
    );
}

/// Criterion 47 (b) with `x`, `y` and a transform on the `use`: `struct-use-03-t` draws the
/// same rotated square twice, 240 user units apart.
#[test]
fn ac47b_w3c_struct_use_03_use_equals_its_reference() {
    let d = read_real("w3c/struct-use-03-t.svg");
    let all = leaves(&d.nodes);
    let squares: Vec<usize> = (0..all.len())
        .filter(|&i| fill_rgb(all[i].1) == Some([0, 255, 255]))
        .collect();
    assert_eq!(squares.len(), 2);
    let (a, b) = (bounds(&all, squares[0]), bounds(&all, squares[1]));
    let shift = 240.0 * PX;
    assert!(
        rect_close(a + kurbo::Vec2::new(shift, 0.0), b, 1e-6),
        "{a:?} vs {b:?}"
    );
    for &i in &squares {
        assert!(close(
            all[i].1.style.stroke_width_mm.unwrap(),
            20.0 * PX,
            1e-9
        ));
    }
}

/// Criteria 36 and 44: `struct-group-01-t` inheritance through groups.
#[test]
fn ac44_w3c_struct_group_01_fills() {
    let d = read_real("w3c/struct-group-01-t.svg");
    let filled: Vec<_> = leaves(&d.nodes)
        .into_iter()
        .filter_map(|(_, l)| fill_rgb(l))
        .collect();
    let (b, g, k, y) = ([0, 0, 255], [0, 128, 0], [0, 0, 0], [255, 255, 0]);
    assert_eq!(filled, [b, g, b, k, y, y]);
}

/// Criterion 68: `coords-viewattr-01-b` declares entities that hold markup (`<rect …>`): not a
/// plain literal, so the file is refused by criterion 6 (b).
#[test]
fn ac68_w3c_markup_entities_are_refused() {
    assert_eq!(
        read_svg(&real("w3c/coords-viewattr-01-b.svg")),
        Err(SvgError::Entities)
    );
}

// ---- real files: Wikimedia Commons (public domain) -------------------------------------------

/// Criterion 16: px sizes without a viewBox (`Flag_of_Japan.svg`).
#[test]
fn ac16_wikimedia_px_without_view_box() {
    let d = read_real("wikimedia/Flag_of_Japan.svg");
    assert_page(&d, 900.0 * PX, 600.0 * PX, 1e-9);
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 2);
    assert!(matches!(all[1].1.kind, LeafKind::Ellipse { rx, .. } if rx == 180.0));
    assert_eq!(fill_rgb(all[1].1), Some([0xbc, 0x00, 0x2d]));
}

/// Criterion 17: a viewBox with a negative origin and no size (`Yin_yang.svg`): the page is the
/// viewBox in px, its top-left corner on the page's.
#[test]
fn ac17_wikimedia_view_box_only_negative_origin() {
    let d = read_real("wikimedia/Yin_yang.svg");
    assert_page(&d, 80.0 * PX, 80.0 * PX, 1e-9);
    let b = d.bounds().unwrap();
    assert!(
        rect_close(b, Rect::new(PX, PX, 79.0 * PX, 79.0 * PX), 1e-6),
        "{b:?}"
    );
}

/// Criteria 15 and 18: `width="512" viewBox="0 0 32 32"` (`Flag_of_Switzerland.svg`): one user
/// unit is 16 px.
#[test]
fn ac15_wikimedia_scaled_view_box() {
    let d = read_real("wikimedia/Flag_of_Switzerland.svg");
    assert_page(&d, 512.0 * PX, 512.0 * PX, 1e-9);
    let all = leaves(&d.nodes);
    let cross = bounds(&all, 1);
    assert!(rect_close(
        cross,
        Rect::new(
            6.0 * 16.0 * PX,
            6.0 * 16.0 * PX,
            26.0 * 16.0 * PX,
            26.0 * 16.0 * PX
        ),
        1e-6
    ));
}

/// Criterion 47 on a real file: `Radiation_warning_symbol.svg` mirrors a path with
/// `use transform="scale(-1,1)"` (a linked clone, determinant −1) and rotates a group by `use`
/// (copies); its mask is ignored (criterion 48).
#[test]
fn ac47_wikimedia_radiation_symbol_clones_and_copies() {
    let d = read_real("wikimedia/Radiation_warning_symbol.svg");
    assert_page(&d, 600.0 * PX, 600.0 * PX, 1e-9);
    let all = leaves(&d.nodes);
    let clones: Vec<_> = all
        .iter()
        .filter_map(|(_, l)| match l.kind {
            LeafKind::Clone { matrix, .. } => Some(matrix),
            _ => None,
        })
        .collect();
    assert!(!clones.is_empty(), "the mirrored use is a linked clone");
    assert!(clones.iter().any(|m| close(m.determinant(), -1.0, 1e-9)));
    assert!(clones.iter().all(|m| isometry(*m)));
    // background, centre dot, three blades of two strokes each: 8 leaves.
    assert_eq!(all.len(), 8);
}

/// Criterion 15: a genuine Inkscape 1.0 file in mm (`Recycling_symbol.svg`).
#[test]
fn ac15_wikimedia_inkscape_1_0_mm_page() {
    let d = read_real("wikimedia/Recycling_symbol.svg");
    assert_page(&d, 132.292_01, 128.235_99, 1e-9);
}

// ---- real files: Illustrator exports -------------------------------------------------------

/// Criteria 19 (a) and 68: Illustrator 16 with its `ns_*` entities in the DOCTYPE: accepted, and
/// px are points (`topcoat_home.svg`, 42 px = 14.8167 mm).
#[test]
fn ac19a_ac68_illustrator_entities_and_points() {
    for name in ["topcoat_home", "topcoat_cart", "topcoat_brush"] {
        let d = read_real(&format!("illustrator/{name}.svg"));
        assert_page(&d, 42.0 * PT, 42.0 * PT, 1e-9);
        assert_eq!(d.leaf_count(), 1, "{name}");
    }
}

/// Criteria 19 (a) and 36: Illustrator 15 with a `<style>` CDATA sheet and lengths with `px`:
/// page 120 pt, the circle's radius 50 pt.
#[test]
fn ac19a_illustrator_comment_only() {
    let d = read_real("illustrator/svgo_svg2js_test.svg");
    assert_page(&d, 120.0 * PT, 120.0 * PT, 1e-9);
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 1);
    let b = bounds(&all, 0);
    assert!(
        rect_close(
            b,
            Rect::new(10.0 * PT, 10.0 * PT, 110.0 * PT, 110.0 * PT),
            1e-6
        ),
        "{b:?}"
    );
}

// ---- synthetic boundaries -------------------------------------------------------------------

/// Criterion 15: pc, pt, cm, em and ex.
#[test]
fn ac15_other_units() {
    let d = read(&format!(
        r#"<svg {NS} width="6pc" height="2em" viewBox="0 0 100 100" preserveAspectRatio="none"><rect width="1" height="1"/></svg>"#
    ));
    assert_page(&d, 25.4, 32.0 * PX, 1e-9);
    let d = read(&format!(
        r#"<svg {NS} width="72pt" height="3ex"><rect width="1" height="1"/></svg>"#
    ));
    assert_page(&d, 25.4, 24.0 * PX, 1e-9);
    let d = read(&format!(
        r#"<svg {NS} width="2.5cm" height="1cm"><rect width="96" height="1"/></svg>"#
    ));
    assert_page(&d, 25.0, 10.0, 1e-9);
    assert!(close(d.bounds().unwrap().width(), 25.4, 1e-9));
}

/// Criterion 18: `slice` and the other alignments.
#[test]
fn ac18_slice_and_alignments() {
    let rect_in = |par: &str| {
        let d = read(&format!(
            r#"<svg {NS} width="100mm" height="100mm" viewBox="0 0 200 100" preserveAspectRatio="{par}"><rect width="200" height="100"/></svg>"#
        ));
        d.bounds().unwrap()
    };
    assert!(rect_close(
        rect_in("xMinYMax meet"),
        Rect::new(0.0, 50.0, 100.0, 100.0),
        1e-9
    ));
    assert!(rect_close(
        rect_in("xMaxYMin meet"),
        Rect::new(0.0, 0.0, 100.0, 50.0),
        1e-9
    ));
    assert!(rect_close(
        rect_in("xMidYMid slice"),
        Rect::new(-50.0, 0.0, 150.0, 100.0),
        1e-9
    ));
    assert!(rect_close(
        rect_in("xMaxYMax slice"),
        Rect::new(-100.0, 0.0, 100.0, 100.0),
        1e-9
    ));
    assert!(rect_close(
        rect_in("xMinYMin slice"),
        Rect::new(0.0, 0.0, 200.0, 100.0),
        1e-9
    ));
}

/// Criterion 19 (b): Inkscape below 0.92 without a viewBox is 90 per inch; 0.92 and a file with a
/// viewBox are 96.
#[test]
fn ac19b_legacy_inkscape_pixel() {
    let page_of = |version: &str, vb: &str| {
        page(&read(&format!(
            r#"<svg {NS} inkscape:version="{version}" width="900" height="450" {vb}><rect width="90" height="90"/></svg>"#
        )))
    };
    let (w, h) = page_of("0.91 r13725", "");
    assert!(close(w, 254.0, 1e-9) && close(h, 127.0, 1e-9), "{w} {h}");
    let (w, _) = page_of("0.48.4 r9939", "");
    assert!(close(w, 254.0, 1e-9));
    let (w, _) = page_of("0.92.4 (5da689c313, 2019-01-14)", "");
    assert!(close(w, 900.0 * PX, 1e-9));
    let (w, _) = page_of("0.91 r13725", r#"viewBox="0 0 900 450""#);
    assert!(close(w, 900.0 * PX, 1e-9));
}

/// Criterion 20 (a): page sides clamped, drawing not scaled.
#[test]
fn ac20a_page_clamped_drawing_not_scaled() {
    let d = read(&format!(
        r#"<svg {NS} width="0.2mm" height="300000mm" viewBox="0 0 0.2 300000"><rect width="0.1" height="5"/></svg>"#
    ));
    assert_page(&d, 1.0, 100_000.0, 1e-9);
    let b = d.bounds().unwrap();
    assert!(close(b.width(), 0.1, 1e-12) && close(b.height(), 5.0, 1e-9));
}

/// Criterion 20 (d): Inkscape 1.2 pages: the first sets the size when the root gives none, the
/// content of the second page is kept.
#[test]
fn ac20d_inkscape_pages() {
    let d = read(&format!(
        r#"<svg {NS}><sodipodi:namedview><inkscape:page x="0" y="0" width="100" height="50"/><inkscape:page x="110" y="0" width="100" height="50"/></sodipodi:namedview><rect x="120" y="10" width="10" height="10"/></svg>"#
    ));
    assert_page(&d, 100.0 * PX, 50.0 * PX, 1e-9);
    assert_eq!(d.leaf_count(), 1);
}

/// Criterion 20 (e): a nested `<svg>` maps its viewBox into x, y, width, height.
#[test]
fn ac20e_nested_svg_is_a_group_with_its_map() {
    let d = mm(
        r#"<svg x="10" y="20" width="20" height="40" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
    );
    // meet: scale 2, centred vertically: (10, 30) to (30, 50).
    let b = d.bounds().unwrap();
    assert!(
        rect_close(b, Rect::new(10.0, 30.0, 30.0, 50.0), 1e-9),
        "{b:?}"
    );
}

/// Criteria 6 (d) and 51: a UTF-8 byte order mark is accepted; a stray invalid byte refuses.
#[test]
fn ac6d_bom_accepted_invalid_utf8_refused() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(
        format!(r#"<svg {NS} width="10mm" height="10mm"><rect width="1" height="1"/></svg>"#)
            .as_bytes(),
    );
    assert_eq!(read_svg(&bytes).unwrap().leaf_count(), 1);
    let mut bad = format!(r#"<svg {NS}><!-- "#).into_bytes();
    bad.extend_from_slice(&[0xC3, 0x28]);
    bad.extend_from_slice(b" --><rect width='1' height='1'/></svg>");
    assert_eq!(read_svg(&bad), Err(SvgError::NotUtf8));
    // UTF-16 LE with BOM, as Windows tools write it.
    let utf16: Vec<u8> = std::iter::once(0xFEFF_u16)
        .chain(format!(r#"<svg {NS}/>"#).encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(read_svg(&utf16), Err(SvgError::NotUtf8));
}

/// Criterion 6 (a): the parse error carries a line and column.
#[test]
fn ac6a_not_xml_with_position() {
    let r =
        read_svg(format!("<svg {NS}>\n<g>\n<rect width='1' height='1'>\n</g></svg>").as_bytes());
    let Err(SvgError::NotSvg(Some((line, column)))) = r else {
        panic!("{r:?}")
    };
    assert!(line >= 3 && column >= 1, "{line}:{column}");
    assert_eq!(
        read_svg(b"<html xmlns='http://www.w3.org/1999/xhtml'/>"),
        Err(SvgError::NotSvg(None))
    );
}

/// Criterion 6 (a) as written: XML whose root is `svg` in another namespace is not an SVG
/// drawing. The implementer's deviation accepts a root without namespace; this pins the other half.
#[test]
fn ac6a_svg_root_in_a_foreign_namespace_is_refused() {
    assert!(matches!(
        read_svg(br#"<svg xmlns="http://example.com/not-svg" width="10mm" height="10mm"><rect width="1" height="1"/></svg>"#),
        Err(SvgError::NotSvg(_))
    ));
}

/// The deviation "no xmlns is accepted" (judged in the report): a hand-written file without the
/// SVG namespace still imports its shapes, so the acceptance is not an empty drawing.
#[test]
fn deviation_missing_namespace_imports_the_shapes() {
    let d = read_svg(br#"<svg width="10mm" height="10mm" viewBox="0 0 10 10"><g><rect width="5" height="5"/></g><path d="M0 0 L5 5"/></svg>"#).unwrap();
    assert_eq!(d.leaf_count(), 2);
}

/// Criteria 6 (c) and 69: 50 MiB is the limit: one byte more is refused before anything else.
#[test]
fn ac6c_fifty_mib_boundary() {
    let head = format!(r#"<svg {NS} width="10mm" height="10mm"><rect width="1" height="1"/><!--"#);
    let tail = "--></svg>";
    let limit = 50 * 1024 * 1024;
    let mut at = head.clone().into_bytes();
    at.resize(limit - tail.len(), b' ');
    at.extend_from_slice(tail.as_bytes());
    assert_eq!(at.len(), limit);
    assert_eq!(read_svg(&at).map(|d| d.leaf_count()), Ok(1));
    let mut over = at.clone();
    over.insert(head.len(), b' ');
    assert_eq!(read_svg(&over), Err(SvgError::TooLarge));
    // Larger and also not UTF-8 and compressed: the size refusal still comes first or the
    // compression one; never a parse.
    let mut gz = vec![0x1F, 0x8B];
    gz.resize(1000, 0);
    assert_eq!(read_svg(&gz), Err(SvgError::Compressed));
}

fn entities(decls: &str, body_refs: &str) -> Result<SvgDrawing, SvgError> {
    read_svg(
        format!(
            "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\" [\n{decls}\n]>\n<svg {NS} width=\"10mm\" height=\"10mm\"><rect width=\"1\" height=\"1\"/><desc>{body_refs}</desc></svg>"
        )
        .as_bytes(),
    )
}

/// Criterion 68: the literal-entity limits at their boundaries.
#[test]
fn ac68_entity_limits_at_the_boundaries() {
    let decl = |i: usize, len: usize| format!("<!ENTITY e{i} \"{}\">", "x".repeat(len));
    let n64: String = (0..64).map(|i| decl(i, 256)).collect();
    assert!(
        entities(&n64, "&e0;&e63;").is_ok(),
        "64 entities of 256 bytes are allowed"
    );
    let n65: String = (0..65).map(|i| decl(i, 1)).collect();
    assert_eq!(
        entities(&n65, "").err(),
        Some(SvgError::Entities),
        "65 entities"
    );
    assert_eq!(
        entities(&decl(0, 257), "").err(),
        Some(SvgError::Entities),
        "257 bytes"
    );
    let refs = "&e0;".repeat(100_000);
    assert!(
        entities(&decl(0, 1), &refs).is_ok(),
        "100,000 references are allowed"
    );
    let refs = "&e0;".repeat(100_001);
    assert_eq!(entities(&decl(0, 1), &refs).err(), Some(SvgError::Entities));
    for bad in [
        r#"<!ENTITY a "&#38;lt;">"#,
        r#"<!ENTITY a "x"><!ENTITY b "&a;&a;">"#,
        r#"<!ENTITY % p "x">"#,
        r#"<!ENTITY a SYSTEM "file:///etc/passwd">"#,
        r#"<!ENTITY a PUBLIC "-//x//y" "http://example.com/x">"#,
        r#"<!ENTITY a "<rect/>">"#,
    ] {
        assert_eq!(entities(bad, "").err(), Some(SvgError::Entities), "{bad}");
    }
    // An entity used in an attribute (the Illustrator `xmlns:x="&ns_extend;"` case) is read.
    let d = read_svg(
        format!("<!DOCTYPE svg [<!ENTITY w \"7mm\">]><svg {NS} width=\"&w;\" height=\"&w;\"><rect width=\"1\" height=\"1\"/></svg>").as_bytes(),
    )
    .unwrap();
    assert_page(&d, 7.0, 7.0, 1e-9);
}

/// Criterion 69: 256 nested elements read and keep their leaf (groups deeper than 32 merge).
#[test]
fn ac69_depth_256_boundary() {
    let nest = |depth: usize| {
        // the root is level 1
        let open = "<g>".repeat(depth - 2);
        let close = "</g>".repeat(depth - 2);
        format!(
            r#"<svg {NS} width="10mm" height="10mm">{open}<rect width="1" height="1"/>{close}</svg>"#
        )
    };
    let d = read_svg(nest(256).as_bytes()).expect("256 levels");
    assert_eq!(d.leaf_count(), 1);
}

/// Criterion 69: "XML nested deeper than 256 elements is refused as not an SVG drawing": a file
/// with 257 nested elements (the root counted as 1) must be refused; it is accepted, 258 is
/// refused (off by one).
#[test]
fn ac69_depth_257_is_refused() {
    let open = "<g>".repeat(255);
    let close = "</g>".repeat(255);
    let text = format!(
        r#"<svg {NS} width="10mm" height="10mm">{open}<rect width="1" height="1"/>{close}</svg>"#
    );
    assert!(matches!(
        read_svg(text.as_bytes()),
        Err(SvgError::NotSvg(_))
    ));
}

/// Criterion 69: a `stroke-dasharray` longer than 1,000 numbers is read as its first 1,000.
#[test]
fn ac69_dash_list_capped_at_1000() {
    let list: Vec<String> = (1..=1500).map(|i| i.to_string()).collect();
    let d = mm(&format!(
        r##"<path d="M0 0 H10" stroke="#000" stroke-width="1" stroke-dasharray="{}"/>"##,
        list.join(" ")
    ));
    let dash = &leaves(&d.nodes)[0].1.style.dash;
    assert_eq!(dash.len(), 1000);
    assert_eq!(dash[999], 1000.0);
}

/// Criterion 41: dashes in multiples of the width; a negative entry or a zero sum is solid; an
/// odd list is kept.
#[test]
fn ac41_dash_rules() {
    let dash = |attrs: &str| {
        let d = mm(&format!(r##"<path d="M0 0 H10" stroke="#000" {attrs}/>"##));
        leaves(&d.nodes)[0].1.style.dash.clone()
    };
    assert_eq!(
        dash(r#"stroke-width="0.25" stroke-dasharray="1, 0.5, 0.25""#),
        [4.0, 2.0, 1.0]
    );
    assert!(dash(r#"stroke-width="1" stroke-dasharray="2 -1""#).is_empty());
    assert!(dash(r#"stroke-width="1" stroke-dasharray="0 0""#).is_empty());
    assert!(dash(r#"stroke-width="1" stroke-dasharray="none""#).is_empty());
    // A transform scales the width by sqrt|det| (criterion 34) and the list stays.
    let d = mm(
        r##"<path d="M0 0 H10" stroke="#000" stroke-width="1" stroke-dasharray="2 1" transform="scale(4 1)"/>"##,
    );
    let s = &leaves(&d.nodes)[0].1.style;
    assert!(close(s.stroke_width_mm.unwrap(), 2.0, 1e-12));
    assert_eq!(s.dash, [2.0, 1.0]);
}

/// Criterion 38: gradient middle colour, pattern grey, fallbacks.
#[test]
fn ac38_paint_servers() {
    let d = mm(r##"<defs>
        <linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>
        <radialGradient id="r"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></radialGradient>
        <pattern id="p" width="1" height="1"><rect width="1" height="1"/></pattern></defs>
        <rect width="1" height="1" fill="url(#g)"/>
        <rect width="1" height="1" fill="url(#r)"/>
        <rect width="1" height="1" fill="url(#p)"/>
        <rect width="1" height="1" fill="url(#missing) #00ff00"/>
        <rect width="1" height="1" fill="url(#missing)"/>
        <rect width="1" height="1" fill="url(other.svg#g)"/>"##);
    let fills: Vec<_> = leaves(&d.nodes).iter().map(|(_, l)| fill_rgb(l)).collect();
    assert_eq!(
        fills,
        [
            Some([128, 128, 128]),
            Some([128, 0, 128]),
            Some([128, 128, 128]),
            Some([0, 255, 0]),
            None,
            None
        ]
    );
}

/// Criteria 37 and 39: colour forms and the opacity example of criterion 39.
#[test]
fn ac37_ac39_colours_and_opacity() {
    let d = mm(r##"<rect width="1" height="1" fill="hsl(120, 100%, 25%)"/>
        <rect width="1" height="1" fill="rgb(100%, 0%, 50%)"/>
        <rect width="1" height="1" fill="#0f08"/>
        <rect width="1" height="1" fill="rebeccapurple"/>
        <rect width="1" height="1" fill="transparent"/>
        <rect width="1" height="1" fill="nonsense" style="fill:nonsense"/>
        <rect width="1" height="1" fill="rgba(255,0,0,0.5)" fill-opacity="0.5" opacity="0.8"/>"##);
    let all = leaves(&d.nodes);
    assert_eq!(fill_rgb(all[0].1), Some([0, 128, 0]));
    assert_eq!(fill_rgb(all[1].1), Some([255, 0, 128]));
    let p = all[2].1.style.fill.unwrap();
    assert_eq!(p.rgb, [0, 255, 0]);
    assert!(close(p.opacity, 0x88 as f64 / 255.0, 1e-9));
    assert_eq!(fill_rgb(all[3].1), Some([0x66, 0x33, 0x99]));
    assert!(all[4].1.style.fill.is_none_or(|p| p.opacity == 0.0));
    assert_eq!(
        fill_rgb(all[5].1),
        Some([0, 0, 0]),
        "unreadable colour: the default"
    );
    let p = all[6].1.style.fill.unwrap();
    assert!(close(p.opacity, 0.25, 1e-12) && close(all[6].0.opacity, 0.8, 1e-12));
}

/// Criteria 42, 44, 46: `display:none` on an ancestor hides the leaf, `visibility:hidden` on a
/// leaf hides it, `<a>` is a group, `<switch>` takes its first child, objects lock.
#[test]
fn ac42_ac44_ac46_states_and_structure() {
    let d = mm(r#"<g style="display:none"><rect width="1" height="1"/></g>
        <rect width="1" height="1" visibility="hidden"/>
        <g visibility="hidden"><rect width="1" height="1"/></g>
        <a href="https://example.com"><rect width="1" height="1" sodipodi:insensitive="true"/></a>
        <switch><rect systemLanguage="xx" width="2" height="2"/><rect width="3" height="3"/></switch>"#);
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 5, "switch gives its first child only");
    assert!(
        close(bounds(&all, 4).width(), 2.0, 1e-12),
        "the first child, untested"
    );
    assert!(all[0].0.hidden || matches!(&d.nodes[0], SvgNode::Group { state, .. } if state.hidden));
    assert!(all[1].0.hidden);
    assert!(all[2].0.hidden, "visibility inherits to the leaf");
    assert!(all[3].0.locked);
    assert!(
        matches!(&d.nodes[3], SvgNode::Group { .. }),
        "<a> is a group"
    );
}

/// Criteria 30 and 31: non-finite and huge values skip the element; a singular transform skips
/// it; an unreadable transform is the identity.
#[test]
fn ac30_ac31_bad_numbers_and_transforms() {
    let d = mm(r#"<rect width="1e308" height="1"/>
        <rect x="9e6" width="2e6" height="1"/>
        <path d="M0 0 L1e400 0"/>
        <rect width="1" height="1" transform="scale(0)"/>
        <rect width="1" height="1" transform="matrix(1 2 2 4 0 0)"/>
        <rect x="5" width="1" height="1" transform="rotate(oops)"/>"#);
    let all = leaves(&d.nodes);
    assert_eq!(
        all.len(),
        1,
        "{:?}",
        all.iter().map(|l| &l.1.kind).collect::<Vec<_>>()
    );
    assert!(close(bounds(&all, 0).x0, 5.0, 1e-12));
}

/// Criterion 47: the example with the original inside a translated group.
#[test]
fn ac47_matrix_example_and_chain_depth() {
    let d = mm(
        r##"<g transform="translate(5 5)"><path id="path1" d="M0 0 H10 V10 H0 Z"/></g><use href="#path1" x="30"/>"##,
    );
    let all = leaves(&d.nodes);
    let LeafKind::Clone { original, matrix } = all[1].1.kind else {
        panic!("{:?}", all[1].1.kind)
    };
    assert_eq!(original, 0);
    let want = [1.0, 0.0, 0.0, 1.0, 25.0, -5.0];
    assert!(
        matrix
            .as_coeffs()
            .iter()
            .zip(want)
            .all(|(a, b)| close(*a, b, 1e-12)),
        "{matrix:?}"
    );
    assert!(rect_close(
        bounds(&all, 1),
        Rect::new(30.0, 0.0, 40.0, 10.0),
        1e-9
    ));
}

/// Criterion 47 (v): "the chain stays at most 8 clones deep; deeper uses are copies". The ninth
/// and tenth `use` of a chain must be ordinary copies; the reader re-links them as clones of the
/// eighth link's original instead (depth stays 8, but they are not copies).
#[test]
fn ac47v_a_chain_deeper_than_eight_continues_as_copies() {
    let mut body = String::from(r#"<path id="u0" d="M0 0 H1 V1 Z"/>"#);
    for i in 1..=10 {
        body.push_str(&format!(r##"<use id="u{i}" href="#u{}" x="2"/>"##, i - 1));
    }
    let d = mm(&body);
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 11);
    let clone_count = all
        .iter()
        .filter(|(_, l)| matches!(l.kind, LeafKind::Clone { .. }))
        .count();
    assert_eq!(clone_count, 8, "chains stay at most 8 clones deep");
    for i in 0..11 {
        assert!(close(bounds(&all, i).x0, 2.0 * i as f64, 1e-9), "link {i}");
    }
}

/// Criterion 47 (c): a `use` before its target, a missing target and a cycle.
#[test]
fn ac47c_forward_reference_missing_and_cycle() {
    let d = mm(
        r##"<use href="#later" x="10"/><path id="later" d="M0 0 H1 V1 Z"/>
        <use href="#nothing"/><use id="a" href="#b"/><use id="b" href="#a"/>"##,
    );
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 2);
    assert!(close(bounds(&all, 0).x0, 10.0, 1e-9));
}

/// Criterion 13: ignored content changes nothing next to it.
#[test]
fn ac13_ignored_content_leaves_the_rest_alone() {
    let plain = mm(r##"<rect width="1" height="1" fill="#f00"/><circle cx="5" cy="5" r="1"/>"##);
    let busy = mm(
        r##"<title>t</title><metadata><x/></metadata><rect width="1" height="1" fill="#f00" data-x="1" aria-label="r" paint-order="stroke" vector-effect="non-scaling-stroke" xmlns:inkstitch="http://inkstitch.org/namespace" inkstitch:running_stitch_length_mm="2"/><text>x</text><image href="data:image/png;base64,AAAA" width="5" height="5"/><foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject><script>alert(1)</script><animate attributeName="x"/><circle cx="5" cy="5" r="1"/><unknownElement/>"##,
    );
    assert_eq!(plain.nodes, busy.nodes);
}

/// Criterion 70: the real files truncated and with single bytes changed refuse or read in under
/// 2 s each and never panic.
#[test]
fn ac70_real_files_truncated_and_mutated_never_panic() {
    let mut files = Vec::new();
    for dir in ["inkscape", "w3c", "wikimedia", "illustrator"] {
        for e in std::fs::read_dir(real_dir().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "svg") {
                files.push(std::fs::read(p).unwrap());
            }
        }
    }
    let mut seed: u64 = 0x2024_0024;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for bytes in &files {
        for k in 1..=40 {
            let cut = bytes.len() * k / 41;
            let t = Instant::now();
            let _ = read_svg(&bytes[..cut]);
            assert!(t.elapsed() < Duration::from_secs(2));
        }
        for _ in 0..40 {
            let mut m = bytes.clone();
            let at = (next() as usize) % m.len();
            m[at] = (next() & 0xFF) as u8;
            let t = Instant::now();
            let _ = read_svg(&m);
            assert!(t.elapsed() < Duration::from_secs(2));
        }
    }
}
