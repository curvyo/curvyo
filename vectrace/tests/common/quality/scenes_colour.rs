//! The synthetic fixtures of the selection methods (`specs/0089-trace-quality-and-colour` slice
//! C, criteria 15 to 19): colours on coloured paper, grey fringes, picked colours that touch,
//! pale colours picked on white, colour on a transparent background. Layers that are not
//! content (the colours the selection leaves out) are drawn but not part of the truth.

use std::sync::Arc;

use kurbo::{BezPath, Point};

use super::reference::{Built, Edge, Source, corners_of, sides_of, smooth};
use super::render::{Layer, Scene, Shape, circle_points, disc, polygon, ring};
use super::scenes::{WHITE, logo_pale, star};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]
}

fn circle_edge(c: Point, r: f64, clearance: f64) -> Edge {
    smooth(circle_points(c, r, 360), clearance)
}

/// The blue paper of criterion 16.
pub const BLUE_PAPER: [u8; 4] = [0x33, 0x66, 0xCC, 255];
/// The pale element of criteria 16 and 17.
pub const PALE_YELLOW: [u8; 4] = [0xFF, 0xF2, 0xA8, 255];
/// The two picked colours that touch (criterion 19).
pub const PICK_YELLOW: [u8; 4] = [0xF2, 0xC4, 0x00, 255];
pub const PICK_RED: [u8; 4] = [0xD0, 0x20, 0x20, 255];
/// The colour of the grey-fringe logo and its fringe.
pub const FRINGE_RED: [u8; 4] = [0xCC, 0x22, 0x22, 255];
pub const FRINGE_GREY: [u8; 4] = [0xB4, 0xB4, 0xB4, 255];

/// Criterion 16: a white ring, a near-black bar and a pale #FFF2A8 panel on uniform #3366CC.
pub fn logo_blue() -> Built {
    let panel = rect(330.0, 50.0, 550.0, 170.0);
    let bar = rect(320.0, 200.0, 560.0, 232.0);
    let mut corners = corners_of(&panel);
    corners.extend(corners_of(&bar));
    let mut edges = sides_of(&panel, 20.0);
    edges.extend(sides_of(&bar, 20.0));
    edges.push(circle_edge(p(150.0, 150.0), 100.0, 30.0));
    edges.push(circle_edge(p(150.0, 150.0), 55.0, 30.0));
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 300.0,
            background: BLUE_PAPER,
            layers: vec![
                Layer::ink(
                    Shape::Paths(vec![ring(p(150.0, 150.0), 100.0, 55.0)]),
                    WHITE,
                ),
                Layer::ink(Shape::Paths(vec![polygon(&bar)]), [0x1A, 0x1A, 0x1A, 255]),
                Layer::ink(Shape::Paths(vec![polygon(&panel)]), PALE_YELLOW),
            ],
        }),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

/// A red star on white with the grey fringe of cheap resampling under it; only the red is
/// content (Pick colours).
pub fn logo_fringe() -> Built {
    let shape = star(p(300.0, 160.0), 130.0, 55.0, 5);
    let mut layer = Layer::ink(Shape::Paths(vec![polygon(&shape)]), FRINGE_RED);
    layer.seam = Some(FRINGE_GREY);
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 320.0,
            background: WHITE,
            layers: vec![layer],
        }),
        artefacts: Vec::new(),
        corners: corners_of(&shape),
        edges: sides_of(&shape, 20.0),
    }
}

/// An implicit rectangle (so two touching ones have their union as the 4x truth).
fn implicit_rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
    Shape::Implicit(Arc::new(move |x, y| x >= x0 && x < x1 && y >= y0 && y < y1))
}

/// Criterion 19: a yellow and a red panel that touch along x = 300 (both picked) and a blue disc
/// (not picked) on white.
pub fn logo_picked() -> Built {
    let union = rect(60.0, 60.0, 540.0, 240.0);
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 360.0,
            background: WHITE,
            layers: vec![
                Layer::ink(implicit_rect(60.0, 60.0, 300.0, 240.0), PICK_YELLOW),
                Layer::ink(implicit_rect(300.0, 60.0, 540.0, 240.0), PICK_RED),
                Layer::other(
                    Shape::Paths(vec![disc(p(300.0, 300.0), 40.0)]),
                    [0x20, 0x50, 0xC0, 255],
                ),
            ],
        }),
        artefacts: Vec::new(),
        corners: corners_of(&union),
        edges: sides_of(&union, 20.0),
    }
}

/// Criterion 17: `logo-pale` with #FFF2A8 and #D9D9D9 picked and #DCD6CF left out.
pub fn logo_pale_picked() -> Built {
    let mut built = logo_pale();
    if let Source::Scene(scene) = &mut built.source {
        scene.layers[2].content = false;
    }
    // The star (the third layer) is no longer content: drop its annotations (the rectangle's 4
    // corners and sides come first, then the star's 10, then the disc's circle).
    built.corners.truncate(4);
    let circle = built.edges.pop();
    built.edges.truncate(4);
    built.edges.extend(circle);
    built
}

/// Criterion 15 with colour: a blue ring, a pale star and a black bar on a transparent
/// background (Remove background finds no background colour; the alpha decides).
pub fn logo_transparent_colour() -> Built {
    let shape = star(p(400.0, 140.0), 100.0, 42.0, 5);
    let bar = rect(30.0, 250.0, 250.0, 275.0);
    let mut corners = corners_of(&shape);
    corners.extend(corners_of(&bar));
    let mut edges = sides_of(&shape, 20.0);
    edges.extend(sides_of(&bar, 20.0));
    edges.push(circle_edge(p(140.0, 120.0), 90.0, 30.0));
    edges.push(circle_edge(p(140.0, 120.0), 50.0, 30.0));
    let paths: Vec<BezPath> = vec![ring(p(140.0, 120.0), 90.0, 50.0)];
    Built {
        source: Source::Scene(Scene {
            width: 520.0,
            height: 300.0,
            background: [0, 0, 0, 0],
            layers: vec![
                Layer::ink(Shape::Paths(paths), BLUE_PAPER),
                Layer::ink(Shape::Paths(vec![polygon(&shape)]), PALE_YELLOW),
                Layer::ink(Shape::Paths(vec![polygon(&bar)]), [0, 0, 0, 255]),
            ],
        }),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}
