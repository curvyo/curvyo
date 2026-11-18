//! SVG import of gradients (`specs/0124-gradient-fill` criteria 14 and 27): the goldens of
//! `tests/fixtures/svg_real/` written as Inkscape and Illustrator write them, each read into its
//! placed gradient, the losses counted, and every export golden `gradient_*.svg` read back.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::path::PathBuf;

use curvyo_svg_core::{
    GradientLosses, SvgDrawing, SvgGradient, SvgGradientShape, SvgLeaf, SvgNode, read_svg,
};
use kurbo::Point;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn read(rel: &str) -> SvgDrawing {
    read_svg(&std::fs::read(fixture(rel)).unwrap()).unwrap()
}

fn leaves(nodes: &[SvgNode], out: &mut Vec<SvgLeaf>) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => leaves(children, out),
            SvgNode::Leaf { leaf, .. } => out.push(leaf.clone()),
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
}

fn gradients(drawing: &SvgDrawing) -> Vec<SvgGradient> {
    let mut out = Vec::new();
    leaves(&drawing.nodes, &mut out);
    out.into_iter()
        .filter_map(|leaf| leaf.style.fill_gradient)
        .collect()
}

/// The gradient coordinate at millimetre point `p`, before clamping.
fn t_at(gradient: &SvgGradient, p: Point) -> f64 {
    let q = gradient.transform.inverse() * p;
    match gradient.shape {
        SvgGradientShape::Linear { start, end } => {
            let d = end - start;
            (q - start).dot(d) / d.hypot2()
        }
        SvgGradientShape::Radial { centre, radius } => (q - centre).hypot() / radius,
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn an_inkscape_vector_and_its_per_object_link() {
    let drawing = read("svg_real/inkscape_linear_href.svg");
    let g = &gradients(&drawing)[0];
    assert_eq!(g.stops.len(), 2);
    assert_eq!((g.stops[0].rgb, g.stops[1].rgb), ([255, 0, 0], [0, 0, 255]));
    assert!(near(t_at(g, Point::new(10.0, 50.0)), 0.0));
    assert!(near(t_at(g, Point::new(50.0, 0.0)), 0.5));
    assert!(near(t_at(g, Point::new(90.0, 13.0)), 1.0));
    assert!(drawing.gradient_losses.is_empty());
}

#[test]
fn a_transformed_radial_is_an_ellipse() {
    let drawing = read("svg_real/inkscape_radial_transform.svg");
    let g = &gradients(&drawing)[0];
    let m = g.transform.as_coeffs();
    let (u, v) = (kurbo::Vec2::new(m[0], m[1]), kurbo::Vec2::new(m[2], m[3]));
    assert!((u.hypot() - v.hypot()).abs() > 0.1, "not a circle");
    // The centre (60, 50) maps through the transform.
    let centre = g.transform * Point::new(60.0, 50.0);
    assert!(near(t_at(g, centre), 0.0));
    let edge = g.transform * Point::new(90.0, 50.0);
    assert!(near(t_at(g, edge), 1.0));
}

#[test]
fn a_chain_of_three_links_keeps_all_stops_and_counts_the_reflect() {
    let drawing = read("svg_real/inkscape_multistop_chain.svg");
    let g = &gradients(&drawing)[0];
    let offsets: Vec<f64> = g.stops.iter().map(|s| s.offset).collect();
    assert_eq!(offsets, [0.0, 0.25, 0.5, 0.5, 1.0], "the hard edge stays");
    // userSpaceOnUse from the middle link, the line from the last.
    assert!(near(t_at(g, Point::new(40.0, 100.0)), 0.0));
    assert!(near(t_at(g, Point::new(40.0, 160.0)), 1.0));
    assert_eq!(
        drawing.gradient_losses,
        GradientLosses {
            repeats: 1,
            focal_points: 0,
            meshes: 0
        }
    );
}

#[test]
fn box_units_and_user_space_under_transforms() {
    let drawing = read("svg_real/inkscape_userspace_units.svg");
    let all = gradients(&drawing);
    // objectBoundingBox on the rectangle at (10, 20), 40 by 20: from (20, 20) to (40, 40).
    assert!(near(t_at(&all[0], Point::new(20.0, 20.0)), 0.0));
    assert!(near(t_at(&all[0], Point::new(40.0, 40.0)), 1.0));
    // userSpaceOnUse in the path's user space, scaled 2 in x: x = 10 to 90 mm.
    assert!(near(t_at(&all[1], Point::new(10.0, 70.0)), 0.0));
    assert!(near(t_at(&all[1], Point::new(90.0, 65.0)), 1.0));
}

#[test]
fn stop_opacity_and_fill_opacity_multiply() {
    let drawing = read("svg_real/inkscape_stop_opacity.svg");
    let alphas: Vec<f64> = gradients(&drawing)[0]
        .stops
        .iter()
        .map(|s| s.alpha)
        .collect();
    assert_eq!(alphas, [0.5, 0.125, 0.0]);
}

#[test]
fn illustrator_gradients_with_px_units_and_a_focal_point() {
    let linear = read("svg_real/illustrator_gradient_linear.svg");
    let g = &gradients(&linear)[0];
    assert_eq!(g.stops.len(), 3);
    assert_eq!(g.stops[1].rgb, [0xF7, 0x93, 0x1E]);
    let radial = read("svg_real/illustrator_gradient_radial.svg");
    let g = &gradients(&radial)[0];
    assert_eq!(g.stops[0].offset, 0.0, "a first stop is added at 0");
    assert_eq!(g.stops.len(), 3);
    assert!(near(g.stops[2].alpha, 0.5));
    assert_eq!(radial.gradient_losses.focal_points, 1);
}

#[test]
fn a_mesh_is_no_fill_and_counted_once() {
    let text = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
        <defs><meshgradient id="m"/></defs>
        <rect width="4" height="4" fill="url(#m)"/>
        <rect x="5" width="4" height="4" fill="url(#m)"/>
      </svg>"#;
    let drawing = read_svg(text.as_bytes()).unwrap();
    let mut out = Vec::new();
    leaves(&drawing.nodes, &mut out);
    assert!(out.iter().all(|leaf| leaf.style.fill.is_none()));
    assert_eq!(drawing.gradient_losses.meshes, 1);
}

#[test]
fn every_export_golden_reads_back() {
    for name in [
        "gradient_linear.svg",
        "gradient_multistop.svg",
        "gradient_radial.svg",
        "gradient_ellipse_rotated.svg",
        "gradient_compound.svg",
        "gradient_open_path.svg",
        "gradient_text.svg",
    ] {
        let drawing = read(&format!("export/{name}"));
        let text = std::fs::read_to_string(fixture(&format!("export/{name}"))).unwrap();
        let count = text.matches("url(#gradient-").count();
        let leaves_with = gradients(&drawing).len();
        // A text keeps its first stop as its colour on import (Proposal).
        let texts = usize::from(name == "gradient_text.svg");
        assert_eq!(leaves_with + texts, count, "{name}");
        assert!(drawing.gradient_losses.is_empty(), "{name}");
    }
}
