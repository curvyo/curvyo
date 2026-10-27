//! The SVG reader against `specs/0024-svg-import-export`: page and units, path data, shapes,
//! transforms, styles, structure and `use` (criteria 13, 15 to 49).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::needless_raw_string_hashes
)]

use curvyo_svg_core::{
    Cap, DisplayUnitName, Join, LeafKind, PageBackground, SvgDrawing, SvgError, SvgLeaf, SvgNode,
    read_svg,
};
use kurbo::{Affine, BezPath, ParamCurve, PathEl, Point, Shape};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:xlink="http://www.w3.org/1999/xlink""#;

/// A millimetre drawing of A4 holding `body`.
fn mm(body: &str) -> SvgDrawing {
    read(&format!(
        r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#
    ))
}

fn read(text: &str) -> SvgDrawing {
    read_svg(text.as_bytes()).unwrap()
}

fn leaves(nodes: &[SvgNode]) -> Vec<(&curvyo_svg_core::NodeState, &SvgLeaf)> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => out.extend(leaves(children)),
            SvgNode::Leaf { state, leaf } => out.push((state, leaf)),
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
    out
}

fn only_path(drawing: &SvgDrawing) -> BezPath {
    match &leaves(&drawing.nodes)[0].1.kind {
        LeafKind::Path { path, .. } => path.clone(),
        other => panic!("{other:?}"),
    }
}

fn points(path: &BezPath) -> Vec<(f64, f64)> {
    path.elements()
        .iter()
        .filter_map(PathEl::end_point)
        .map(|p| ((p.x * 1e6).round() / 1e6, (p.y * 1e6).round() / 1e6))
        .collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// ---- C. The page, the units and the viewBox ----------------------------------------------

#[test]
fn ac15_sizes_and_the_view_box_give_the_page_and_the_map() {
    let d = mm(r#"<rect x="10" y="20" width="30" height="40"/>"#);
    assert_eq!(d.page, Some((210.0, 297.0)));
    let LeafKind::Rect { rect, matrix, .. } = &leaves(&d.nodes)[0].1.kind else {
        panic!()
    };
    assert_eq!(*matrix * rect.origin(), Point::new(10.0, 20.0));
    let d = read(&format!(
        r#"<svg {NS} width="2in" height="1in" viewBox="0 0 200 100"><rect x="100" width="100" height="100"/></svg>"#
    ));
    assert_eq!(d.page, Some((50.8, 25.4)));
    let LeafKind::Rect { rect, matrix, .. } = &leaves(&d.nodes)[0].1.kind else {
        panic!()
    };
    let b = (*matrix * rect.to_path(0.0)).bounding_box();
    assert!(near(b.x0, 25.4) && near(b.x1, 50.8) && near(b.y1, 25.4));
    let d = read(&format!(
        r#"<svg {NS} width="100mm" height="100mm" viewBox="10 10 100 100"><path d="M10 10 L20 10"/></svg>"#
    ));
    assert_eq!(points(&only_path(&d)), [(0.0, 0.0), (10.0, 0.0)]);
}

#[test]
fn ac16_ac17_missing_view_box_or_size() {
    let d = read(&format!(
        r#"<svg {NS} width="210mm" height="297mm"><path d="M0 0 H100 V50"/></svg>"#
    ));
    let b = only_path(&d).bounding_box();
    assert!(near(b.x1, 26.458_333_333_333_33) && near(b.y1, 13.229_166_666_666_666));
    let d = read(&format!(r#"<svg {NS} viewBox="0 0 100 50"/>"#));
    let (w, h) = d.page.unwrap();
    assert!(near(w, 26.458_333_333_333_33) && near(h, 13.229_166_666_666_666));
    let d = read(&format!(
        r#"<svg {NS}><rect x="10" y="10" width="100" height="50"/></svg>"#
    ));
    assert_eq!(d.page, Some((30.0, 16.0)));
    let d = read(&format!(r#"<svg {NS}/>"#));
    assert_eq!(d.page, None);
    let d = read(&format!(
        r#"<svg {NS} width="100%" height="auto" viewBox="0 0 100 50"/>"#
    ));
    assert!(near(d.page.unwrap().0, 26.458_333_333_333_33));
}

#[test]
fn ac18_preserve_aspect_ratio() {
    let at = |par: &str| {
        let d = read(&format!(
            r#"<svg {NS} width="100mm" height="100mm" viewBox="0 0 200 100" preserveAspectRatio="{par}"><path d="M0 0 H200 V100"/></svg>"#
        ));
        let b = only_path(&d).bounding_box();
        (b.x0, b.y0, b.x1, b.y1)
    };
    assert_eq!(at("xMidYMid meet"), (0.0, 25.0, 100.0, 75.0));
    assert_eq!(at("none"), (0.0, 0.0, 100.0, 100.0));
    assert_eq!(at("xMinYMax"), (0.0, 50.0, 100.0, 100.0));
    assert_eq!(at("xMidYMid slice"), (-50.0, 0.0, 150.0, 100.0));
}

#[test]
fn ac19_known_generators_set_the_pixel() {
    let illustrator = read(&format!(
        r#"<?xml version="1.0"?><!-- Generator: Adobe Illustrator 27.0, SVG Export Plug-In . SVG Version: 6.00 Build 0) -->
        <svg {NS} width="595.28px" height="841.89px" viewBox="0 0 595.28 841.89"/>"#
    ));
    let (w, h) = illustrator.page.unwrap();
    assert!(
        (w - 210.0).abs() < 0.01 && (h - 297.0).abs() < 0.01,
        "{w} {h}"
    );
    let other = read(&format!(
        r#"<svg {NS} width="595.28px" height="841.89px" viewBox="0 0 595.28 841.89"/>"#
    ));
    let (w, h) = other.page.unwrap();
    assert!(
        (w - 157.50).abs() < 0.01 && (h - 222.75).abs() < 0.01,
        "{w} {h}"
    );
    let legacy = read(&format!(
        r#"<svg {NS} inkscape:version="0.48.4 r9939" width="90" height="90"><path d="M0 0 H90"/></svg>"#
    ));
    assert!(near(legacy.page.unwrap().0, 25.4));
    assert!(near(only_path(&legacy).bounding_box().x1, 25.4));
}

#[test]
fn ac20_namedview_and_page_limits() {
    let d = read(&format!(
        r##"<svg {NS} width="0.1mm" height="200000mm" viewBox="0 0 1 1">
        <sodipodi:namedview inkscape:document-units="in" pagecolor="#ff0000" inkscape:pageopacity="0.5"/></svg>"##
    ));
    assert_eq!(d.page, Some((1.0, 100_000.0)));
    assert_eq!(d.display_unit, Some(DisplayUnitName::In));
    assert_eq!(d.background, Some(PageBackground::Colour([255, 0, 0], 0.5)));
    let d = mm(
        r##"<sodipodi:namedview inkscape:document-units="px" pagecolor="#ffffff" inkscape:pageopacity="0.0"/>"##,
    );
    assert_eq!(d.display_unit, None);
    assert_eq!(d.background, Some(PageBackground::None));
    let nested = mm(
        r#"<svg x="10" y="10" width="20" height="20" viewBox="0 0 2 2"><path d="M0 0 H2"/></svg>"#,
    );
    assert_eq!(points(&only_path(&nested)), [(10.0, 10.0), (30.0, 10.0)]);
}

// ---- D. Shapes and paths -----------------------------------------------------------------

#[test]
fn ac21_ac23_the_path_grammar_and_its_errors() {
    let compact =
        mm(r#"<path d="M10-5l1.5.5.5.5H20v1e-3Q25 0 30 0T40 0C40 5 45 5 45 0S50-5 50 0Z"/>"#);
    let pts = points(&only_path(&compact));
    assert_eq!(
        pts[..5],
        [
            (10.0, -5.0),
            (11.5, -4.5),
            (12.0, -4.0),
            (20.0, -4.0),
            (20.0, -3.999)
        ]
    );
    let path = only_path(&compact);
    let PathEl::CurveTo(c1, _, _) = path.elements()[6] else {
        panic!()
    };
    assert_eq!(c1, Point::new(30.0 + 5.0 * 2.0 / 3.0, 0.0), "T reflects Q");
    let PathEl::CurveTo(c1, _, _) = path.elements()[8] else {
        panic!()
    };
    assert_eq!(c1, Point::new(45.0, -5.0), "S reflects C");
    let cut = mm(r#"<path d="M 10 10 L 20 20 L 30"/>"#);
    assert_eq!(points(&only_path(&cut)), [(10.0, 10.0), (20.0, 20.0)]);
    let reopened = mm(r#"<path d="M0 0 H10 V10 Z L 5 20"/>"#);
    let els = only_path(&reopened).elements().to_vec();
    assert!(matches!(els[4], PathEl::MoveTo(p) if p == Point::ZERO));
    assert_eq!(
        mm(r#"<path d="M 10 10"/><path d="M 0 0 Z"/><path d="L 5 5"/>"#).nodes,
        []
    );
}

#[test]
fn ac22_arcs_in_eighths_within_a_micron() {
    let quarter = only_path(&mm(r#"<path d="M 10 0 A 10 10 0 0 1 0 10"/>"#));
    let segs: Vec<_> = quarter.segments().collect();
    assert_eq!(segs.len(), 2, "a quarter circle is two 45° cubics");
    for seg in &segs {
        for i in 0..=50 {
            let p = seg.eval(f64::from(i) / 50.0);
            assert!((p.to_vec2().hypot() - 10.0).abs() < 0.001);
            assert!(
                p.x >= -1e-9 && p.y >= -1e-9,
                "sweep 1 goes clockwise on screen through +x +y"
            );
        }
    }
    assert_eq!(segs[1].end(), Point::new(0.0, 10.0));
    let other = only_path(&mm(r#"<path d="M 10 0 A 10 10 0 0 0 0 10"/>"#));
    let mid = other.segments().next().unwrap().eval(1.0);
    assert!(
        ((mid - Point::new(10.0, 10.0)).hypot() - 10.0).abs() < 0.001,
        "flags 0 0 take the circle about (10, 10): {mid:?}"
    );
    let rotated = only_path(&mm(r#"<path d="M 0 0 A 20 10 30 1 1 30 10"/>"#));
    assert!(rotated.segments().count() >= 4);
    let line = only_path(&mm(r#"<path d="M 0 0 A 0 10 0 0 1 10 0"/>"#));
    assert_eq!(points(&line), [(0.0, 0.0), (10.0, 0.0)]);
    let tiny = only_path(&mm(r#"<path d="M 0 0 A 1 1 0 0 1 10 0"/>"#));
    assert!(
        near(tiny.bounding_box().y0, -5.0),
        "radii scaled up to a half circle"
    );
    let packed = only_path(&mm(r#"<path d="M0 0a1 1 0 00 5 5"/>"#));
    assert_eq!(
        packed.segments().last().unwrap().end(),
        Point::new(5.0, 5.0)
    );
}

#[test]
fn ac26_ac27_basic_shapes() {
    let d = mm(r#"
        <rect width="20" height="10" rx="3"/>
        <rect width="20" height="10" rx="30" ry="2"/>
        <rect width="0" height="10"/>
        <circle cx="5" cy="5" r="2"/>
        <circle r="0"/>
        <ellipse cx="1" cy="2" rx="3" ry="4"/>
        <line x1="1" y1="2" x2="3" y2="4"/>
        <polyline points="0,0 10,0 10,10 5"/>
        <polygon points="0 0 10 0 10 10"/>"#);
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 7);
    let LeafKind::Rect { rx, ry, .. } = all[0].1.kind else {
        panic!()
    };
    assert_eq!((rx, ry), (3.0, 3.0));
    let LeafKind::Rect { rx, ry, .. } = all[1].1.kind else {
        panic!()
    };
    assert_eq!((rx, ry), (10.0, 2.0), "clamped to half the side");
    assert!(matches!(
        all[2].1.kind,
        LeafKind::Ellipse {
            rx: 2.0,
            ry: 2.0,
            ..
        }
    ));
    assert!(matches!(
        all[3].1.kind,
        LeafKind::Ellipse {
            rx: 3.0,
            ry: 4.0,
            ..
        }
    ));
    let LeafKind::Path { path, .. } = &all[5].1.kind else {
        panic!()
    };
    assert_eq!(points(path), [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    assert!(!path.elements().contains(&PathEl::ClosePath));
    let LeafKind::Path { path, .. } = &all[6].1.kind else {
        panic!()
    };
    assert!(path.elements().contains(&PathEl::ClosePath));
}

#[test]
fn ac28_inkscape_star_data_is_read() {
    let d = mm(
        r#"<path sodipodi:type="star" sodipodi:sides="5" sodipodi:cx="0" sodipodi:cy="0" sodipodi:r1="10" sodipodi:r2="4" sodipodi:arg1="0" sodipodi:arg2="0.6" inkscape:flatsided="false" inkscape:rounded="0" inkscape:randomized="0" d="M 10,0 Z"/>
        <path sodipodi:type="star" sodipodi:sides="5" sodipodi:cx="0" sodipodi:cy="0" sodipodi:r1="10" sodipodi:r2="4" sodipodi:arg1="0" inkscape:rounded="0.2" d="M 10,0 L 0 0"/>"#,
    );
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 1, "the first has no segment and is skipped");
    assert!(
        matches!(all[0].1.kind, LeafKind::Path { .. }),
        "rounded stars are paths"
    );
    let d = mm(
        r#"<path sodipodi:type="star" sodipodi:sides="6" sodipodi:cx="5" sodipodi:cy="5" sodipodi:r1="10" sodipodi:r2="8.66" sodipodi:arg1="0" inkscape:flatsided="true" d="M 15,5 L 0 0"/>"#,
    );
    let LeafKind::Star { star, .. } = &leaves(&d.nodes)[0].1.kind else {
        panic!()
    };
    assert_eq!((star.sides, star.flat, star.r1), (6, true, 10.0));
}

#[test]
fn ac30_out_of_range_numbers_skip_the_element() {
    let d = mm(
        r#"<path d="M0 0 L1e308 0"/><path d="M0 0 L NaN 0"/><rect x="1e8" width="1" height="1"/>
        <path d="M0 0 L5 0" stroke="black" stroke-width="-1" fill="red"/>"#,
    );
    let all = leaves(&d.nodes);
    assert_eq!(
        all.len(),
        1,
        "a NaN ends the path after its move, which is skipped"
    );
    let style = &all[0].1.style;
    assert!(style.stroke.is_none() && style.fill.is_some());
}

// ---- E. Transforms -----------------------------------------------------------------------

#[test]
fn ac31_transforms_compose_in_svg_order() {
    let d = mm(
        r#"<g transform="translate(100 50)"><path transform="rotate(90)" d="M10 0 L20 0"/></g>"#,
    );
    assert_eq!(points(&only_path(&d)), [(100.0, 60.0), (100.0, 70.0)]);
    let d =
        mm(r#"<path transform="scale(0)" d="M0 0 L1 1"/><path transform="bogus(" d="M0 0 L1 1"/>"#);
    assert_eq!(points(&only_path(&d)), [(0.0, 0.0), (1.0, 1.0)]);
    let d = mm(r#"<rect width="20" height="10" transform="rotate(30)"/>"#);
    let LeafKind::Rect { matrix, .. } = &leaves(&d.nodes)[0].1.kind else {
        panic!()
    };
    let c = *matrix * Point::new(10.0, 5.0);
    assert!(near(c.x, 6.160_254_037_844_386) && near(c.y, 9.330_127_018_922_193));
}

#[test]
fn ac34_the_stroke_scales_by_the_root_of_the_determinant() {
    let d = mm(
        r#"<path transform="scale(2 8)" stroke="black" stroke-width="1" stroke-dasharray="2 1" d="M0 0 H1"/>"#,
    );
    let style = &leaves(&d.nodes)[0].1.style;
    assert_eq!(style.stroke_width_mm, Some(4.0));
    assert_eq!(style.dash, [2.0, 1.0]);
    let px = read(&format!(
        r#"<svg {NS} width="96" height="96"><path stroke="red" stroke-width="2" stroke-dasharray="6 4" d="M0 0 H1"/></svg>"#
    ));
    let style = &leaves(&px.nodes)[0].1.style;
    assert_eq!(style.dash, [3.0, 2.0]);
    assert!(near(style.stroke_width_mm.unwrap(), 2.0 * 25.4 / 96.0));
}

// ---- F. Style ----------------------------------------------------------------------------

#[test]
fn ac37_to_ac41_paints_opacity_and_stroke_parts() {
    let d = mm(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs>
        <path fill="rgba(255,0,0,0.5)" fill-opacity="0.5" opacity="0.8" d="M0 0 H1"/>
        <path fill="url(#g)" stroke="url(#nothing)" d="M0 0 H1"/>
        <path fill="none" stroke="#00f" stroke-linecap="round" stroke-linejoin="arcs" stroke-width="0.5" stroke-dasharray="2 1" d="M0 0 H1"/>
        <path fill="red" stroke="none" stroke-width="3" stroke-linecap="square" d="M0 0 H1"/>"##,
    );
    let all = leaves(&d.nodes);
    let (state, leaf) = all[0];
    let fill = leaf.style.fill.unwrap();
    assert_eq!(
        (fill.rgb, fill.opacity, state.opacity),
        ([255, 0, 0], 0.25, 0.8)
    );
    assert_eq!(all[1].1.style.fill.unwrap().rgb, [0x80; 3]);
    assert!(all[1].1.style.stroke.is_none());
    let s = &all[2].1.style;
    assert!(s.fill.is_none());
    assert_eq!(
        (s.cap, s.join, s.dash.as_slice()),
        (Some(Cap::Round), Some(Join::Miter), &[4.0, 2.0][..])
    );
    let s = &all[3].1.style;
    assert!(s.stroke.is_none());
    assert_eq!((s.stroke_width_mm, s.cap), (Some(3.0), Some(Cap::Square)));
}

#[test]
fn ac42_hidden_objects_come_in_hidden() {
    let d = mm(
        r#"<g style="display:none"><path d="M0 0 H1"/></g><path visibility="hidden" d="M0 0 H1"/><g visibility="hidden"><path visibility="visible" d="M0 0 H1"/></g>"#,
    );
    let SvgNode::Group { state, .. } = &d.nodes[0] else {
        panic!()
    };
    assert!(state.hidden);
    let all = leaves(&d.nodes);
    assert!(!all[0].0.hidden && all[1].0.hidden && !all[2].0.hidden);
}

// ---- G. Structure ------------------------------------------------------------------------

#[test]
fn ac44_to_ac46_groups_layers_names_and_locks() {
    let d = mm(r#"
        <g inkscape:groupmode="layer" inkscape:label="Cut" sodipodi:insensitive="true" opacity="0.5">
            <g inkscape:groupmode="layer" inkscape:label="Sub"><path d="M0 0 H1"/></g>
        </g>
        <g inkscape:groupmode="layer" id="layer7"/>
        <g inkscape:groupmode="layer"/>
        <g id="g1"><g/></g>
        <a><path inkscape:label="Named" id="p1" sodipodi:insensitive="true" d="M0 0 H1"/></a>
        <switch><path d="M0 0 H2"/><path d="M0 0 H3"/></switch>
        <path d="M0 0 H4"/>"#);
    let names: Vec<_> = d
        .nodes
        .iter()
        .map(|n| {
            (
                n.state().name.clone(),
                matches!(n, SvgNode::Group { layer: true, .. }),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            (Some("Cut".into()), true),
            (Some("layer7".into()), true),
            (Some("Layer 3".into()), true),
            (None, false),
            (None, false),
            (None, false),
        ]
    );
    let SvgNode::Group {
        state, children, ..
    } = &d.nodes[0]
    else {
        panic!()
    };
    assert!(state.locked && state.opacity == 0.5);
    assert!(
        matches!(&children[0], SvgNode::Group { layer: false, .. }),
        "a layer in a layer is a group"
    );
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 4);
    assert_eq!(
        (all[1].0.name.as_deref(), all[1].0.locked),
        (Some("Named"), true)
    );
    assert_eq!(
        points(match &all[2].1.kind {
            LeafKind::Path { path, .. } => path,
            _ => panic!(),
        })[1],
        (2.0, 0.0)
    );
}

#[test]
fn ac47_use_becomes_a_clone_or_a_copy() {
    let d = mm(r##"
        <path id="p" d="M0 0 H10 V10 H0 Z"/>
        <use xlink:href="#p" x="30"/>
        <use href="#p" transform="rotate(90)"/>
        <use href="#p" transform="scale(-1 1)"/>
        <use href="#p" transform="scale(2)"/>
        <use href="#p" fill="red"/>
        <use href="#c2"/>
        <use id="c2" href="#p" x="5"/>
        <rect id="r" width="1" height="1"/><use href="#r" x="3"/>
        <defs><path id="hidden" d="M0 0 H1"/></defs><use href="#hidden"/>
        <use href="#missing"/><use href="other.svg#p"/>
        <g id="loop"><use href="#loop"/></g>
        <symbol id="s" viewBox="0 0 1 1"><path d="M0 0 H1"/></symbol><use href="#s" width="10" height="10"/>
        <g transform="translate(5 5)"><path id="q" d="M0 0 H10"/></g><use href="#q" x="30"/>"##);
    let all = leaves(&d.nodes);
    let clone = |i: usize| match all[i].1.kind {
        LeafKind::Clone { original, matrix } => Some((original, matrix.as_coeffs())),
        _ => None,
    };
    assert_eq!(clone(1), Some((0, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0])));
    let (_, m) = clone(2).unwrap();
    assert!(near(m[0], 0.0) && near(m[1], 1.0) && near(m[2], -1.0) && near(m[3], 0.0));
    assert_eq!(clone(3), Some((0, [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0])));
    assert_eq!(clone(4), None, "a scaled use is a copy");
    let LeafKind::Path { path, .. } = &all[4].1.kind else {
        panic!()
    };
    assert!(near(path.bounding_box().x1, 20.0));
    assert_eq!(all[4].1.style.stroke, None);
    assert_eq!(clone(5), None, "a use that sets a fill is a copy");
    assert_eq!(all[5].1.style.fill.unwrap().rgb, [255, 0, 0]);
    assert_eq!(
        clone(6),
        Some((7, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])),
        "a clone of a later clone"
    );
    assert_eq!(clone(7), Some((0, [1.0, 0.0, 0.0, 1.0, 5.0, 0.0])));
    assert!(
        matches!(all[9].1.kind, LeafKind::Rect { .. }),
        "a use of a primitive is a copy"
    );
    assert!(
        matches!(all[10].1.kind, LeafKind::Path { .. }),
        "a target in defs is a copy"
    );
    let LeafKind::Path { path, .. } = &all[11].1.kind else {
        panic!()
    };
    assert!(
        near(path.bounding_box().x1, 10.0),
        "the symbol's viewBox maps to 10 x 10"
    );
    assert_eq!(clone(13), Some((12, [1.0, 0.0, 0.0, 1.0, 25.0, -5.0])));
    assert_eq!(all.len(), 14);
}

// ---- Ignored content and refusals --------------------------------------------------------

#[test]
fn ac13_ac49_unsupported_content_leaves_no_trace() {
    let d = mm(
        r#"<title>t</title><metadata><x/></metadata><text>Boho</text><image href="data:image/png;base64,AA"/>
        <foreignObject><div/></foreignObject><script>alert(1)</script><animate/><unknown><path d="M0 0 H1"/></unknown>
        <inkstitch:x xmlns:inkstitch="http://inkstitch.org/namespace"/><clipPath id="c"><path d="M0 0 H1"/></clipPath>
        <path clip-path="url(#c)" filter="url(#f)" d="M0 0 H1"/>"#,
    );
    assert_eq!(leaves(&d.nodes).len(), 1);
    // Slice 2 (criterion 78): a text is read as a text node, no leaf.
    let only_text = mm(r#"<text>only text</text>"#).nodes;
    assert!(matches!(only_text.as_slice(), [SvgNode::Text { .. }]));
}

#[test]
fn ac6_refusals() {
    let err = |bytes: &[u8]| read_svg(bytes).unwrap_err();
    assert_eq!(err(b"\x1F\x8B\x08\x00"), SvgError::Compressed);
    assert_eq!(err(b"\xFF\xFE<\x00s\x00"), SvgError::NotUtf8);
    assert_eq!(
        err(br#"<!DOCTYPE svg [<!ENTITY a "x"><!ENTITY b "&a;&a;">]><svg/>"#),
        SvgError::Entities
    );
    assert_eq!(err(b"<html/>"), SvgError::NotSvg(None));
    assert_eq!(err(b"<svg>\n<g>\n</svg>"), SvgError::NotSvg(Some((3, 1))));
    let deep = format!("<svg>{}</svg>", "<g>".repeat(300));
    assert_eq!(err(deep.as_bytes()), SvgError::NotSvg(None));
    assert!(
        read_svg(b"<svg><path d='M0 0 H1'/></svg>").is_ok(),
        "a root without xmlns is read"
    );
}

#[test]
fn ac69_limits_keep_what_was_read() {
    let many = format!(
        r#"<svg {NS}>{}</svg>"#,
        r#"<path d="M0 0 H1"/>"#.repeat(500_010)
    );
    let d = read(&many);
    assert!(d.truncated);
    assert_eq!(
        leaves(&d.nodes).len(),
        499_999,
        "the root counts as an element"
    );
    let dashes = format!("{} 1", "1 ".repeat(2000));
    let d = mm(&format!(
        r#"<path stroke="red" stroke-dasharray="{dashes}" d="M0 0 H1"/>"#
    ));
    assert_eq!(leaves(&d.nodes)[0].1.style.dash.len(), 1000);
}

#[test]
fn clone_matrices_compose_to_the_copy() {
    let d = mm(
        r##"<g transform="rotate(30) translate(3 4)"><path id="p" transform="translate(1 2)" d="M0 0 C 1 2 3 4 5 0"/></g>
        <g transform="translate(40 0)"><use href="#p" transform="rotate(45)" x="2"/></g>"##,
    );
    let all = leaves(&d.nodes);
    let LeafKind::Path { path, .. } = &all[0].1.kind else {
        panic!()
    };
    let LeafKind::Clone { matrix, .. } = all[1].1.kind else {
        panic!()
    };
    let expected = Affine::translate((40.0, 0.0))
        * Affine::rotate(45_f64.to_radians())
        * Affine::translate((2.0, 0.0))
        * Affine::translate((1.0, 2.0));
    let local = BezPath::from_svg("M0 0 C 1 2 3 4 5 0").unwrap();
    let copy = expected * local;
    for (a, b) in (matrix * path.clone())
        .elements()
        .iter()
        .zip(copy.elements())
    {
        let (Some(a), Some(b)) = (a.end_point(), b.end_point()) else {
            continue;
        };
        assert!((a - b).hypot() < 1e-9);
    }
}
