//! The synthetic reference fixtures of sketches, AI-like drawings, paper captures, the diagram, the
//! photograph (`specs/0089-trace-quality-and-colour` criterion 2).

use std::sync::Arc;

use kurbo::{BezPath, Point};

use super::degrade::Artefact;
use super::fixtures::{Noise, photograph};
use super::metrics::to_segment;
use super::reference::{
    Built, Feature, Group, Reference, Source, Use, corners_of, sides_of, smooth,
};
use super::render::{Layer, Scene, Shape, circle_points, disc, polygon, ring};
use super::scenes::{BLACK, WHITE, bar, black_on_white, star};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

pub fn sketch_bowed() -> Built {
    // A square whose right edge bows out by 2 px over 200 px (criterion 7).
    let mut outline = vec![p(60.0, 60.0), p(260.0, 60.0)];
    let bow: Vec<Point> = (1..100)
        .map(|i| {
            let t = f64::from(i) / 100.0;
            // A circular arc of sagitta 2 over the chord of 200: r = (100² + 2²) / 4.
            let r = (100.0_f64 * 100.0 + 4.0) / 4.0;
            let y = 60.0 + 200.0 * t;
            let x = 260.0 + (r * r - (y - 160.0).powi(2)).sqrt() - (r - 2.0);
            p(x, y)
        })
        .collect();
    outline.extend(bow.iter().copied());
    outline.extend([p(260.0, 260.0), p(60.0, 260.0)]);
    let square = [
        p(60.0, 60.0),
        p(260.0, 60.0),
        p(260.0, 260.0),
        p(60.0, 260.0),
    ];
    let mut edges = sides_of(&square, 40.0);
    edges.remove(1);
    edges.push(smooth(bow[10..90].to_vec(), 40.0));
    // A hand-drawn wobbly loop: no corner anywhere.
    let loop_points: Vec<Point> = (0..720)
        .map(|i| {
            let a = f64::from(i).to_radians() * 0.5;
            let r = 1.0 + 0.03 * (5.0 * a).sin() + 0.015 * (13.0 * a).sin();
            p(380.0 + 80.0 * r * a.cos(), 200.0 + 120.0 * r * a.sin())
        })
        .collect();
    let mut wobble = loop_points.clone();
    wobble.push(loop_points[0]);
    edges.push(smooth(wobble, 40.0));
    let mut scene = black_on_white(500.0, 360.0, vec![polygon(&outline), polygon(&loop_points)]);
    scene.layers[0].rgba = [70, 70, 70, 255];
    Built {
        source: Source::Scene(scene),
        artefacts: vec![Artefact::ScanNoise {
            amplitude: 6.0,
            seed: 31,
        }],
        corners: corners_of(&square),
        edges,
    }
}

pub fn sketch_strokes() -> Built {
    let mut noise = Noise::new(17);
    // A box drawn twice with slightly shifted strokes, and loose curves of varying width.
    let mut strokes: Vec<((f64, f64), (f64, f64), f64)> = Vec::new();
    let corners: [(f64, f64); 4] = [(80.0, 80.0), (300.0, 70.0), (310.0, 280.0), (70.0, 290.0)];
    for pass in 0..2 {
        let j = 1.5 * f64::from(pass);
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let over = 6.0 * noise.next().abs();
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dy);
            let (ux, uy) = (dx / len, dy / len);
            strokes.push((
                (a.0 - ux * over + j, a.1 - uy * over - j),
                (b.0 + ux * over + j, b.1 + uy * over - j),
                1.2 + 0.4 * f64::from(pass),
            ));
        }
    }
    for k in 0..5 {
        let y0 = 60.0 + 55.0 * f64::from(k);
        let mut last = (360.0, y0);
        for i in 1..=40 {
            let x = 360.0 + 5.0 * f64::from(i);
            let y = y0 + 18.0 * (x / 40.0 + f64::from(k)).sin();
            // Tapered: thin at both ends.
            let t = f64::from(i) / 40.0;
            strokes.push((last, (x, y), 0.4 + 1.8 * (std::f64::consts::PI * t).sin()));
            last = (x, y);
        }
    }
    let inside = Arc::new(move |x: f64, y: f64| {
        strokes.iter().any(|&(a, b, r)| {
            x >= a.0.min(b.0) - r
                && x <= a.0.max(b.0) + r
                && y >= a.1.min(b.1) - r
                && y <= a.1.max(b.1) + r
                && to_segment((x, y), a, b) <= r
        })
    });
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 360.0,
            background: WHITE,
            layers: vec![Layer::ink(Shape::Implicit(inside), [60, 60, 60, 255])],
        }),
        artefacts: vec![Artefact::ScanNoise {
            amplitude: 8.0,
            seed: 37,
        }],
        corners: Vec::new(),
        edges: Vec::new(),
    }
}

pub fn sketch_stippling() -> Built {
    let c = p(250.0, 250.0);
    let mut noise = Noise::new(23);
    let mut dots: Vec<(Point, f64)> = Vec::new();
    // Denser towards the lower left, kept 1.5 px apart, inside a ring.
    for _ in 0..6000 {
        let q = p(c.x + 170.0 * noise.next(), c.y + 170.0 * noise.next());
        let r = 1.0 + 1.5 * noise.next().abs();
        let keep = 0.5 + 0.5 * ((q.y - q.x) / 340.0);
        if q.distance(c) > 165.0 || noise.next().abs() > keep {
            continue;
        }
        if dots.iter().all(|&(o, s)| o.distance(q) > r + s + 1.5) {
            dots.push((q, r));
        }
        if dots.len() >= 400 {
            break;
        }
    }
    let mut paths: Vec<BezPath> = dots
        .iter()
        .map(|&(q, r)| polygon(&circle_points(q, r, 64)))
        .collect();
    paths.push(ring(c, 191.0, 189.0));
    Built {
        source: Source::Scene(black_on_white(500.0, 500.0, paths)),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges: vec![smooth(circle_points(c, 191.0, 360), 2.0)],
    }
}

fn ellipse(q: (f64, f64), c: (f64, f64), rx: f64, ry: f64) -> bool {
    ((q.0 - c.0) / rx).powi(2) + ((q.1 - c.1) / ry).powi(2) <= 1.0
}

pub fn ai_portrait() -> Built {
    let inside = Arc::new(|x: f64, y: f64| {
        let q = (x, y);
        let hood =
            ellipse(q, (300.0, 290.0), 190.0, 230.0) || (y > 400.0 && (90.0..510.0).contains(&x));
        let face = ellipse(q, (300.0, 320.0), 88.0, 118.0);
        // White strands cut through the hood: thin sine curves.
        let strand = (0..8).any(|k| {
            let x0 = 150.0 + 40.0 * f64::from(k);
            let sx = x0 + 14.0 * (y / 45.0 + f64::from(k)).sin();
            y < 420.0 && (x - sx).abs() < 0.9 && !ellipse(q, (300.0, 320.0), 100.0, 130.0)
        });
        let eyes = ellipse(q, (265.0, 300.0), 14.0, 7.0) || ellipse(q, (335.0, 300.0), 14.0, 7.0);
        let mouth = {
            let d = ((x - 300.0).powi(2) + (y - 330.0).powi(2)).sqrt();
            (d - 45.0).abs() < 2.0 + 1.5 * ((x - 300.0) / 45.0).cos() && y > 355.0
        };
        (hood && !face && !strand) || eyes || mouth
    });
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 600.0,
            background: WHITE,
            layers: vec![Layer::ink(Shape::Implicit(inside), BLACK)],
        }),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges: Vec::new(),
    }
}

pub fn ai_hatching() -> Built {
    let inside = Arc::new(|x: f64, y: f64| {
        let d = ((x - 300.0).powi(2) + (y - 300.0).powi(2)).sqrt();
        let band = ((y + 20.0 * (x / 60.0).sin()) / 10.0).rem_euclid(1.0);
        // Engraving: the lines swell towards the lower right.
        let width = 0.15 + 0.45 * ((x + y) / 1200.0);
        (d < 250.0 && band < width) || (d - 262.0).abs() < 3.0
    });
    Built {
        source: Source::Scene(Scene {
            width: 600.0,
            height: 600.0,
            background: WHITE,
            layers: vec![Layer::ink(Shape::Implicit(inside), BLACK)],
        }),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges: Vec::new(),
    }
}

pub fn ai_symbol() -> Built {
    let c = p(300.0, 300.0);
    let mut paths = vec![
        ring(c, 281.0, 279.0),
        ring(c, 271.0, 269.0),
        ring(c, 256.0, 254.0),
    ];
    let outer = star(p(300.0, 320.0), 200.0, 100.0, 3)
        .into_iter()
        .step_by(2)
        .collect::<Vec<_>>();
    let inner_frame: Vec<Point> = outer
        .iter()
        .map(|q| p(300.0 + (q.x - 300.0) * 0.85, 320.0 + (q.y - 320.0) * 0.85))
        .collect();
    let mut frame = polygon(&outer);
    frame.extend(polygon(&inner_frame));
    paths.push(frame);
    let small: Vec<Point> = outer
        .iter()
        .map(|q| p(300.0 + (q.x - 300.0) * 0.3, 320.0 + (q.y - 320.0) * 0.3))
        .collect();
    let mut eye = polygon(&small);
    eye.extend(disc(p(300.0, 320.0), 10.0));
    paths.push(eye);
    let mut corners = corners_of(&outer);
    corners.extend(corners_of(&inner_frame));
    corners.extend(corners_of(&small));
    let mut edges = sides_of(&outer, 25.0);
    edges.extend(sides_of(&inner_frame, 25.0));
    edges.extend(sides_of(&small, 20.0));
    Built {
        source: Source::Scene(black_on_white(600.0, 600.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

/// A technical drawing: a frame, a box, rings, a slanted line and an arrow head; dark grey ink.
pub fn technical() -> Built {
    let frame_out = [
        p(20.0, 20.0),
        p(580.0, 20.0),
        p(580.0, 430.0),
        p(20.0, 430.0),
    ];
    let frame_in = [
        p(23.0, 23.0),
        p(577.0, 23.0),
        p(577.0, 427.0),
        p(23.0, 427.0),
    ];
    let box_out = [
        p(60.0, 60.0),
        p(300.0, 60.0),
        p(300.0, 220.0),
        p(60.0, 220.0),
    ];
    let box_in = [
        p(62.0, 62.0),
        p(298.0, 62.0),
        p(298.0, 218.0),
        p(62.0, 218.0),
    ];
    let mut frame = polygon(&frame_out);
    frame.extend(polygon(&frame_in));
    let mut boxed = polygon(&box_out);
    boxed.extend(polygon(&box_in));
    let (slant, slant_edges) = bar(p(70.0, 300.0), 200.0, 2.0, -20.0);
    let arrow = [p(340.0, 330.0), p(400.0, 350.0), p(340.0, 370.0)];
    let paths = vec![
        frame,
        boxed,
        ring(p(450.0, 140.0), 61.0, 59.0),
        ring(p(450.0, 140.0), 26.0, 24.0),
        polygon(&slant),
        polygon(&arrow),
    ];
    let mut corners = corners_of(&frame_out);
    corners.extend(corners_of(&frame_in));
    corners.extend(corners_of(&box_out));
    corners.extend(corners_of(&box_in));
    corners.extend(corners_of(&arrow));
    let mut edges = sides_of(&frame_out, 3.0);
    edges.extend(sides_of(&frame_in, 3.0));
    edges.extend(sides_of(&box_out, 2.0));
    edges.extend(sides_of(&box_in, 2.0));
    edges.extend(slant_edges);
    edges.push(smooth(circle_points(p(450.0, 140.0), 61.0, 360), 2.0));
    let mut scene = black_on_white(600.0, 450.0, paths);
    scene.layers[0].rgba = [40, 40, 40, 255];
    Built {
        source: Source::Scene(scene),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

fn with(artefacts: Vec<Artefact>) -> Built {
    Built {
        artefacts,
        ..technical()
    }
}

pub fn scan_noise() -> Built {
    with(vec![Artefact::ScanNoise {
        amplitude: 18.0,
        seed: 41,
    }])
}

pub fn scan_paper() -> Built {
    with(vec![
        Artefact::PaperTexture { seed: 43 },
        Artefact::ScanNoise {
            amplitude: 6.0,
            seed: 47,
        },
    ])
}

pub fn scan_jpeg() -> Built {
    with(vec![
        Artefact::ScanNoise {
            amplitude: 8.0,
            seed: 53,
        },
        Artefact::Jpeg50,
    ])
}

pub fn photo_shadow() -> Built {
    with(vec![
        Artefact::PaperTexture { seed: 59 },
        Artefact::Shadow,
        Artefact::ScanNoise {
            amplitude: 6.0,
            seed: 61,
        },
    ])
}

pub fn photo_lighting() -> Built {
    with(vec![
        Artefact::Lighting,
        Artefact::ScanNoise {
            amplitude: 6.0,
            seed: 67,
        },
    ])
}

pub fn photo_combined() -> Built {
    with(vec![
        Artefact::PaperTexture { seed: 71 },
        Artefact::Shadow,
        Artefact::Lighting,
        Artefact::ScanNoise {
            amplitude: 10.0,
            seed: 73,
        },
        Artefact::Jpeg50,
    ])
}

pub fn diagram() -> Built {
    let mut paths = Vec::new();
    let mut corners = Vec::new();
    let mut edges = Vec::new();
    for &(x, y) in &[(40.0, 40.0), (40.0, 260.0), (420.0, 150.0)] {
        let outer = [
            p(x, y),
            p(x + 160.0, y),
            p(x + 160.0, y + 90.0),
            p(x, y + 90.0),
        ];
        let inner = [
            p(x + 2.0, y + 2.0),
            p(x + 158.0, y + 2.0),
            p(x + 158.0, y + 88.0),
            p(x + 2.0, y + 88.0),
        ];
        let mut frame = polygon(&outer);
        frame.extend(polygon(&inner));
        paths.push(frame);
        corners.extend(corners_of(&outer));
        corners.extend(corners_of(&inner));
        edges.extend(sides_of(&outer, 2.0));
    }
    // Arrows: a 2 px shaft and a head as one outline, clear of the boxes.
    for &(x0, y, x1) in &[(210.0, 85.0, 410.0), (210.0, 305.0, 410.0)] {
        let arrow = [
            p(x0, y - 1.0),
            p(x1 - 14.0, y - 1.0),
            p(x1 - 14.0, y - 7.0),
            p(x1, y),
            p(x1 - 14.0, y + 7.0),
            p(x1 - 14.0, y + 1.0),
            p(x0, y + 1.0),
        ];
        paths.push(polygon(&arrow));
        corners.push(p(x1, y));
    }
    // A decision diamond frame and a round node.
    let diamond = [
        p(300.0, 140.0),
        p(350.0, 195.0),
        p(300.0, 250.0),
        p(250.0, 195.0),
    ];
    let diamond_in: Vec<Point> = diamond
        .iter()
        .map(|q| p(300.0 + (q.x - 300.0) * 0.95, 195.0 + (q.y - 195.0) * 0.95))
        .collect();
    let mut frame = polygon(&diamond);
    frame.extend(polygon(&diamond_in));
    paths.push(frame);
    corners.extend(corners_of(&diamond));
    edges.extend(sides_of(&diamond, 2.0));
    paths.push(ring(p(590.0, 340.0), 30.0, 28.0));
    Built {
        source: Source::Scene(black_on_white(640.0, 400.0, paths)),
        artefacts: Vec::new(),
        corners,
        edges,
    }
}

pub fn photo_street() -> Built {
    Built {
        source: Source::Raster(photograph()),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges: Vec::new(),
    }
}
