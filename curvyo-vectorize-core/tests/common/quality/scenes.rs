//! The synthetic reference fixtures of graphics, logos, line drawings
//! (`specs/0089-trace-quality-and-colour` criterion 2), drawn from known vectors with their
//! annotations.

use kurbo::{BezPath, Point, Shape as _};

use super::degrade::Artefact;
use super::reference::{
    Built, Edge, Feature, Group, Reference, Source, Use, corners_of, sides_of, smooth,
};
use super::render::{
    Layer, Scene, Shape, arc_band, arc_points, circle_points, disc, polygon, ring, rotated_rect,
};

pub const BLACK: [u8; 4] = [0, 0, 0, 255];
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// One black layer of `paths` on white.
pub fn black_on_white(width: f64, height: f64, paths: Vec<BezPath>) -> Scene {
    Scene {
        width,
        height,
        background: WHITE,
        layers: vec![Layer::ink(Shape::Paths(paths), BLACK)],
    }
}

/// A star with `points` tips; a regular pentagram's tips are 36 degrees at `inner / outer` =
/// 0.381966.
pub fn star(c: Point, outer: f64, inner: f64, points: usize) -> Vec<Point> {
    (0..points * 2)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = std::f64::consts::PI * i as f64 / points as f64 - std::f64::consts::FRAC_PI_2;
            p(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// A circle's smooth annotation (all of it).
fn circle_edge(c: Point, r: f64, clearance: f64) -> Edge {
    smooth(circle_points(c, r, 360), clearance)
}

pub fn geo_corners() -> Built {
    let mut paths = Vec::new();
    let mut corners = Vec::new();
    let mut edges = Vec::new();
    for (i, degrees) in [0.0, 15.0, 30.0, 45.0].into_iter().enumerate() {
        let square = rotated_rect(p(90.0 + 140.0 * i as f64, 100.0), 90.0, 90.0, degrees);
        corners.extend(corners_of(&square));
        edges.extend(sides_of(&square, 20.0));
        paths.push(polygon(&square));
    }
    // A right triangle with a 30 degree corner.
    let triangle = [p(60.0, 360.0), p(220.0, 360.0), p(60.0, 360.0 - 92.376)];
    corners.extend(corners_of(&triangle));
    edges.extend(sides_of(&triangle, 20.0));
    paths.push(polygon(&triangle));
    let star = star(p(430.0, 300.0), 100.0, 38.1966, 5);
    corners.extend(corners_of(&star));
    edges.extend(sides_of(&star, 20.0));
    paths.push(polygon(&star));
    Built {
        source: Source::Scene(black_on_white(600.0, 420.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn geo_round() -> Built {
    let rounded = kurbo::RoundedRect::new(40.0, 40.0, 260.0, 170.0, 10.0).to_path(1e-4);
    let mut edges = vec![
        Edge {
            points: vec![p(50.0, 40.0), p(250.0, 40.0)],
            straight: true,
            clearance: 40.0,
        },
        Edge {
            points: vec![p(260.0, 50.0), p(260.0, 160.0)],
            straight: true,
            clearance: 40.0,
        },
        Edge {
            points: vec![p(250.0, 170.0), p(50.0, 170.0)],
            straight: true,
            clearance: 40.0,
        },
        Edge {
            points: vec![p(40.0, 160.0), p(40.0, 50.0)],
            straight: true,
            clearance: 40.0,
        },
    ];
    for (c, from) in [
        (p(250.0, 50.0), 270.0),
        (p(250.0, 160.0), 0.0),
        (p(50.0, 160.0), 90.0),
        (p(50.0, 50.0), 180.0),
    ] {
        edges.push(smooth(
            arc_points(c, 10.0, from, from + 90.0, 30, false),
            40.0,
        ));
    }
    let mut paths = vec![rounded];
    for &(c, r) in &[
        (p(420.0, 110.0), 60.0),
        (p(330.0, 250.0), 20.0),
        (p(380.0, 250.0), 10.0),
        (p(410.0, 250.0), 5.0),
        (p(500.0, 320.0), 40.0),
    ] {
        paths.push(disc(c, r));
        if r >= 20.0 {
            edges.push(circle_edge(c, r, 20.0));
        }
    }
    paths.push(ring(p(150.0, 310.0), 50.0, 36.0));
    edges.push(circle_edge(p(150.0, 310.0), 50.0, 14.0));
    edges.push(circle_edge(p(150.0, 310.0), 36.0, 14.0));
    Built {
        source: Source::Scene(black_on_white(600.0, 420.0, paths)),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges,
    }
}

/// A ring with a star inside it whose centre is a round hole.
fn badge(c: Point) -> (Vec<BezPath>, Vec<Point>, Vec<Edge>) {
    let star = star(c, 160.0, 70.0, 5);
    let mut star_path = polygon(&star);
    star_path.extend(disc(c, 30.0));
    let paths = vec![ring(c, 220.0, 190.0), star_path];
    let mut edges = sides_of(&star, 20.0);
    edges.push(circle_edge(c, 220.0, 30.0));
    edges.push(circle_edge(c, 190.0, 30.0));
    edges.push(circle_edge(c, 30.0, 30.0));
    (paths, corners_of(&star), edges)
}

pub fn clipart_badge() -> Built {
    let (paths, corners, edges) = badge(p(250.0, 250.0));
    let mut scene = black_on_white(500.0, 500.0, paths);
    scene.layers[0].seam = Some([150, 150, 150, 255]);
    Built {
        source: Source::Scene(scene),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

/// The flat logo of the 0025 set as vectors: a ring, a star and a bar.
fn logo_paths() -> (Vec<BezPath>, Vec<Point>, Vec<Edge>) {
    let star = star(p(420.0, 190.0), 140.0, 60.0, 5);
    let bar = [
        p(60.0, 345.0),
        p(540.0, 345.0),
        p(540.0, 372.0),
        p(60.0, 372.0),
    ];
    let mut corners = corners_of(&star);
    corners.extend(corners_of(&bar));
    let mut edges = sides_of(&star, 20.0);
    edges.extend(sides_of(&bar, 10.0));
    edges.push(circle_edge(p(150.0, 190.0), 110.0, 30.0));
    edges.push(circle_edge(p(150.0, 190.0), 50.0, 30.0));
    (
        vec![
            ring(p(150.0, 190.0), 110.0, 50.0),
            polygon(&star),
            polygon(&bar),
        ],
        corners,
        edges,
    )
}

pub fn logo_shapes() -> Built {
    let (paths, corners, edges) = logo_paths();
    Built {
        source: Source::Scene(black_on_white(600.0, 400.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn logo_jpeg() -> Built {
    Built {
        artefacts: vec![Artefact::Jpeg50],
        ..logo_shapes()
    }
}

pub fn logo_pale() -> Built {
    let rect = [
        p(40.0, 60.0),
        p(200.0, 60.0),
        p(200.0, 240.0),
        p(40.0, 240.0),
    ];
    let star = star(p(470.0, 160.0), 100.0, 42.0, 5);
    let mut corners = corners_of(&rect);
    corners.extend(corners_of(&star));
    let mut edges = sides_of(&rect, 20.0);
    edges.extend(sides_of(&star, 20.0));
    edges.push(circle_edge(p(290.0, 150.0), 60.0, 20.0));
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 300.0,
            background: WHITE,
            layers: vec![
                Layer::ink(Shape::Paths(vec![polygon(&rect)]), [0xFF, 0xF2, 0xA8, 255]),
                Layer::ink(
                    Shape::Paths(vec![disc(p(290.0, 150.0), 60.0)]),
                    [0xD9, 0xD9, 0xD9, 255],
                ),
                Layer::ink(Shape::Paths(vec![polygon(&star)]), [0xDC, 0xD6, 0xCF, 255]),
            ],
        }),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn logo_transparent() -> Built {
    let star = star(p(380.0, 150.0), 100.0, 42.0, 5);
    let bar = [
        p(30.0, 250.0),
        p(250.0, 250.0),
        p(250.0, 275.0),
        p(30.0, 275.0),
    ];
    let mut corners = corners_of(&star);
    corners.extend(corners_of(&bar));
    let mut edges = sides_of(&star, 20.0);
    edges.extend(sides_of(&bar, 20.0));
    edges.push(circle_edge(p(140.0, 120.0), 90.0, 30.0));
    edges.push(circle_edge(p(140.0, 120.0), 50.0, 30.0));
    Built {
        source: Source::Scene(Scene {
            width: 500.0,
            height: 300.0,
            background: [0, 0, 0, 0],
            layers: vec![
                Layer::ink(
                    Shape::Paths(vec![ring(p(140.0, 120.0), 90.0, 50.0), polygon(&bar)]),
                    BLACK,
                ),
                // Half transparent black: content by design.
                Layer::ink(Shape::Paths(vec![polygon(&star)]), [0, 0, 0, 128]),
            ],
        }),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

/// A bar of `length` and `width` from `start` at `degrees`, with its corners and long sides.
pub fn bar(start: Point, length: f64, width: f64, degrees: f64) -> (Vec<Point>, Vec<Edge>) {
    let (s, c) = degrees.to_radians().sin_cos();
    let centre = p(start.x + c * length / 2.0, start.y + s * length / 2.0);
    let corners = rotated_rect(centre, length, width, degrees);
    let edges = vec![
        Edge {
            points: vec![corners[0], corners[1]],
            straight: true,
            clearance: width,
        },
        Edge {
            points: vec![corners[2], corners[3]],
            straight: true,
            clearance: width,
        },
    ];
    (corners, edges)
}

pub fn line_angles() -> Built {
    let mut paths = Vec::new();
    let mut corners = Vec::new();
    let mut edges = Vec::new();
    let starts = [
        (p(40.0, 40.0), 0.0),
        (p(40.0, 110.0), 7.0),
        (p(40.0, 190.0), 15.0),
        (p(360.0, 40.0), 30.0),
        (p(360.0, 200.0), 45.0),
    ];
    for &(start, degrees) in &starts {
        // A wide bar and, 330 px lower, a 3 px line at the same angle.
        for (offset, width) in [(0.0, 12.0), (330.0, 3.0)] {
            let (c, e) = bar(p(start.x, start.y + offset), 240.0, width, degrees);
            corners.extend(c.iter().copied());
            edges.extend(e);
            paths.push(polygon(&c));
        }
    }
    Built {
        source: Source::Scene(black_on_white(640.0, 720.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn line_webs() -> Built {
    let mut paths = Vec::new();
    let mut corners = Vec::new();
    for (y, web) in [(40.0, 1.0), (140.0, 2.0)] {
        // Two blocks joined by a web of `web` px: one component.
        let shape = [
            p(40.0, y),
            p(100.0, y),
            p(100.0, y + 30.0 - web / 2.0),
            p(180.0, y + 30.0 - web / 2.0),
            p(180.0, y),
            p(240.0, y),
            p(240.0, y + 60.0),
            p(180.0, y + 60.0),
            p(180.0, y + 30.0 + web / 2.0),
            p(100.0, y + 30.0 + web / 2.0),
            p(100.0, y + 60.0),
            p(40.0, y + 60.0),
        ];
        corners.extend(corners_of(&shape));
        paths.push(polygon(&shape));
    }
    let mut edges = Vec::new();
    // Parallel bars 1 px and 2 px apart, on and off the pixel grid; thin lines 1 px apart.
    for &(y, width, gap) in &[
        (40.0, 6.0, 1.0),
        (120.5, 6.0, 1.0),
        (200.0, 6.0, 2.0),
        (280.0, 2.0, 1.0),
    ] {
        for k in 0..2 {
            let top = y + f64::from(k) * (width + gap);
            let (c, mut e) = bar(p(300.0, top + width / 2.0), 200.0, width, 0.0);
            for edge in &mut e {
                edge.clearance = gap;
            }
            edges.extend(e);
            corners.extend(c.iter().copied());
            paths.push(polygon(&c));
        }
    }
    Built {
        source: Source::Scene(black_on_white(560.0, 340.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn line_hatching() -> Built {
    let c = p(250.0, 250.0);
    let mut paths = vec![ring(c, 191.5, 188.5)];
    let mut edges = Vec::new();
    // 45 degree hatching inside a circle of radius 180: lines 2 px wide, 7 px apart.
    let mut offset = -175.0;
    while offset <= 175.0 {
        let half = (180.0_f64 * 180.0 - offset * offset).sqrt();
        let (s, k) = (
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        );
        // Along (k, s), across (-s, k).
        let start = p(c.x - half * k - offset * s, c.y - half * s + offset * k);
        let (corners, e) = bar(start, 2.0 * half, 2.0, 45.0);
        if 2.0 * half > 20.0 {
            edges.extend(e);
        }
        paths.push(polygon(&corners));
        offset += 7.0;
    }
    edges.push(circle_edge(c, 191.5, 3.0));
    Built {
        source: Source::Scene(black_on_white(500.0, 500.0, paths)),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges,
    }
}

pub fn line_strokes_open() -> Built {
    let mut paths = Vec::new();
    let mut edges = Vec::new();
    // Open arcs: a "C" 4 px wide and a thin arc 2 px wide.
    for &(c, r, w, from, to) in &[
        (p(100.0, 100.0), 60.0, 4.0, 40.0, 320.0),
        (p(280.0, 110.0), 40.0, 2.0, 200.0, 340.0),
    ] {
        paths.push(arc_band(c, r, w, from, to));
        let inner = (to - from) * 0.1;
        edges.push(smooth(
            arc_points(c, r + w / 2.0, from + inner, to - inner, 90, false),
            w,
        ));
        edges.push(smooth(
            arc_points(c, r - w / 2.0, from + inner, to - inner, 90, false),
            w,
        ));
    }
    // A straight open stroke.
    let (c, e) = bar(p(360.0, 60.0), 120.0, 4.0, 20.0);
    paths.push(polygon(&c));
    edges.extend(e);
    // Nearly closed rings: gaps of 1 px and 3 px stay open.
    for &(c, gap) in &[(p(100.0, 290.0), 1.0_f64), (p(280.0, 290.0), 3.0)] {
        let r = 50.0_f64;
        let half = (gap / 2.0 / (r + 2.0)).to_degrees();
        // The gap on the right; the band's ends are radial, so the gap is narrowest outside.
        let half = half.max((gap / 2.0 / (r - 2.0)).to_degrees());
        paths.push(arc_band(c, r, 4.0, half, 360.0 - half));
        edges.push(smooth(arc_points(c, r + 2.0, 20.0, 340.0, 120, false), 4.0));
    }
    Built {
        source: Source::Scene(black_on_white(500.0, 380.0, paths)),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges,
    }
}
