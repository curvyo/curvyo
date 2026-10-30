//! Trial C1 of `specs/0089-trace-quality-and-colour` (`adrs.md` §3): which colour space and
//! which model the colour selection uses. Candidates: the distance to the picked colour in sRGB,
//! linear RGB and Oklab, and the two-colour mixing model (the distance to the segment picked
//! colour → background, the mixing fraction as the membership) in the same three spaces. Each
//! candidate's mask is traced through Brightness and measured against the truth; the separation
//! margin is the smallest distance of a colour that must stay out over the largest distance of
//! a pixel that must come in (blends from 25 % coverage up). Test code only: the product
//! implements the winner. `#[ignore]`, run in release:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test trial_c1 -- --ignored --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::many_single_char_names
)]

mod common;

use common::Picture;
use common::quality::reference::{Built, Source};
use common::quality::scenes::{WHITE, logo_shapes};
use common::quality::scenes_colour::{
    FRINGE_RED, PALE_YELLOW, PICK_RED, PICK_YELLOW, logo_fringe, logo_pale_picked, logo_picked,
};
use common::quality::shape::{deviation, samples_along, truth_segments};
use curvyo_document_core::Affine;
use curvyo_vectorize_core::TraceParams;
use kurbo::BezPath;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Space {
    Srgb,
    Linear,
    Oklab,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Model {
    /// Content within the gate of a picked colour.
    Point,
    /// Content by the mixing fraction toward the background, within the gate of the segment.
    Mix,
}

fn decode(v: u8) -> f64 {
    let c = f64::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_space(space: Space, [r, g, b]: [u8; 3]) -> [f64; 3] {
    match space {
        Space::Srgb => [r, g, b].map(|v| f64::from(v) / 255.0),
        Space::Linear => [r, g, b].map(decode),
        Space::Oklab => {
            let [r, g, b] = [r, g, b].map(decode);
            let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
            let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
            let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
            [
                0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
                1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
                0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
            ]
        }
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The fraction of `c` in `p` on the segment `bg → c`, and the distance from `p` to it.
fn mix(p: [f64; 3], c: [f64; 3], bg: [f64; 3]) -> (f64, f64) {
    let d = [c[0] - bg[0], c[1] - bg[1], c[2] - bg[2]];
    let q = [p[0] - bg[0], p[1] - bg[1], p[2] - bg[2]];
    let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let f = ((q[0] * d[0] + q[1] * d[1] + q[2] * d[2]) / len2).clamp(0.0, 1.0);
    let on = [bg[0] + f * d[0], bg[1] + f * d[1], bg[2] + f * d[2]];
    (f, dist(p, on))
}

/// The membership 0 to 1 of `p` for the picked colours, and its gate distance.
fn score(space: Space, model: Model, p: [u8; 3], picks: &[[u8; 3]], bg: [u8; 3], gate: f64) -> f64 {
    let pv = to_space(space, p);
    let bv = to_space(space, bg);
    let mut best = 0.0_f64;
    for (i, &c) in picks.iter().enumerate() {
        let cv = to_space(space, c);
        match model {
            Model::Point => {
                if dist(pv, cv) <= gate {
                    best = 1.0;
                }
            }
            Model::Mix => {
                let (f, r) = mix(pv, cv, bv);
                if r <= gate {
                    best = best.max(f);
                }
                for &other in &picks[i + 1..] {
                    let (_, r) = mix(pv, cv, to_space(space, other));
                    if r <= gate {
                        best = 1.0;
                    }
                }
            }
        }
    }
    best
}

/// The blend of `c` at `f` over `bg` as the renderer stores it (sRGB bytes).
fn blend(c: [u8; 3], bg: [u8; 3], f: f64) -> [u8; 3] {
    [0, 1, 2].map(|k| (f64::from(c[k]) * f + f64::from(bg[k]) * (1.0 - f)).round() as u8)
}

/// The gate distance of `p` for `picks` (the smallest residual or distance).
fn gate_of(space: Space, model: Model, p: [u8; 3], picks: &[[u8; 3]], bg: [u8; 3]) -> f64 {
    let pv = to_space(space, p);
    picks
        .iter()
        .map(|&c| {
            let cv = to_space(space, c);
            match model {
                Model::Point => dist(pv, cv),
                Model::Mix => mix(pv, cv, to_space(space, bg)).1,
            }
        })
        .fold(f64::INFINITY, f64::min)
}

struct Case {
    name: &'static str,
    build: fn() -> Built,
    picks: Vec<[u8; 3]>,
    /// Colours that must stay out.
    others: Vec<[u8; 3]>,
    bg: [u8; 3],
}

fn rgb(c: [u8; 4]) -> [u8; 3] {
    [c[0], c[1], c[2]]
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "pale picked (17)",
            build: logo_pale_picked,
            picks: vec![rgb(PALE_YELLOW), [0xD9, 0xD9, 0xD9]],
            others: vec![[0xDC, 0xD6, 0xCF]],
            bg: rgb(WHITE),
        },
        Case {
            name: "black AA edge (18)",
            build: logo_shapes,
            picks: vec![[0, 0, 0]],
            others: vec![],
            bg: rgb(WHITE),
        },
        Case {
            name: "grey fringe",
            build: logo_fringe,
            picks: vec![rgb(FRINGE_RED)],
            others: vec![[0xB4, 0xB4, 0xB4]],
            bg: rgb(WHITE),
        },
        Case {
            name: "touching picks (19)",
            build: logo_picked,
            picks: vec![rgb(PICK_YELLOW), rgb(PICK_RED)],
            others: vec![[0x20, 0x50, 0xC0]],
            bg: rgb(WHITE),
        },
    ]
}

const MM: f64 = 25.4 / 96.0;

/// Traces the mask `content` (`w` x `h`) through Brightness and returns the outlines in pixels.
fn trace_mask(content: &[bool], w: u32, h: u32) -> Vec<BezPath> {
    let levels: Vec<u8> = content.iter().map(|&c| if c { 0 } else { 255 }).collect();
    let picture = Picture::grey(w, h, &levels);
    let result = common::run(
        &picture,
        Affine::scale(MM, MM),
        &TraceParams::with_threshold(128),
    );
    result
        .outlines
        .iter()
        .map(|o| kurbo::Affine::scale(1.0 / MM) * o.clone())
        .collect()
}

#[test]
#[ignore = "trial C1: run in release with --ignored --nocapture"]
fn trial_c1_colour_space_and_model() {
    let candidates = [
        (Space::Srgb, Model::Point),
        (Space::Linear, Model::Point),
        (Space::Oklab, Model::Point),
        (Space::Srgb, Model::Mix),
        (Space::Linear, Model::Mix),
        (Space::Oklab, Model::Mix),
    ];
    println!(
        "| Candidate | Case | in max | out min | margin | gate | dev truth→trace max | dev trace→truth max | area err % | paths | ms/4 Mpx |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    for &(space, model) in &candidates {
        for case in cases() {
            // In: the picked colours and their blends with the background from 25 % (Point
            // needs them from 50 % to keep the edge; it is measured from 50 %).
            let from = if model == Model::Point { 0.5 } else { 0.25 };
            let mut inside = 0.0_f64;
            for &c in &case.picks {
                for k in 0..=20 {
                    let f = from + (1.0 - from) * f64::from(k) / 20.0;
                    inside = inside.max(gate_of(
                        space,
                        model,
                        blend(c, case.bg, f),
                        &case.picks,
                        case.bg,
                    ));
                }
            }
            // Out: the other colours and their blends with the background from 50 %.
            let mut outside = f64::INFINITY;
            for &o in &case.others {
                for k in 0..=10 {
                    let f = 0.5 + 0.5 * f64::from(k) / 10.0;
                    outside = outside.min(gate_of(
                        space,
                        model,
                        blend(o, case.bg, f),
                        &case.picks,
                        case.bg,
                    ));
                }
            }
            let gate = if outside.is_finite() {
                (inside.max(1e-4) * outside).sqrt()
            } else {
                inside.max(1e-4) * 3.0
            };
            let built = (case.build)();
            let Source::Scene(scene) = &built.source else {
                unreachable!()
            };
            let drawn = scene.draw(1.0);
            let (w, h) = (drawn.picture.width, drawn.picture.height);
            let t0 = common::thread_cpu_seconds();
            let content: Vec<bool> = drawn
                .picture
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[r, g, b, a]| {
                    f64::from(a) / 255.0
                        * score(space, model, [r, g, b], &case.picks, case.bg, gate)
                        >= 0.5
                })
                .collect();
            let cpu = common::thread_cpu_seconds() - t0;
            let ms_4mpx = cpu * 1e3 * 4.0e6 / f64::from(w * h);
            let outlines = trace_mask(&content, w, h);
            let segments = truth_segments(
                scene.truth_paths(1.0).as_deref(),
                &drawn.truth_4x,
                w as usize,
                h as usize,
            );
            let dev = deviation(
                &outlines,
                &segments,
                &samples_along(&segments),
                w as usize,
                h as usize,
            );
            let traced = common::quality::rasterise_4x(&outlines, w as usize, h as usize);
            let fg = drawn.truth_4x.iter().filter(|t| **t).count();
            let xor = traced
                .iter()
                .zip(&drawn.truth_4x)
                .filter(|(a, b)| a != b)
                .count();
            println!(
                "| {space:?} {model:?} | {} | {inside:.4} | {outside:.4} | {:.1} | {gate:.4} | {:.2} | {:.2} | {:.2} | {} | {ms_4mpx:.0} |",
                case.name,
                outside / inside.max(1e-4),
                dev.truth_max,
                dev.trace_max,
                xor as f64 / fg.max(1) as f64 * 100.0,
                outlines.len(),
            );
        }
    }
}
