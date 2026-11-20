//! Preset quality on real-looking inputs (`specs/0135-image-optimisation` criteria 34, 35; UX
//! review U7): a synthetic pen drawing (thin dark lines of several widths and greys on slightly
//! uneven, grainy paper) and a synthetic soft-shaded photo (a lit subject on paper with a darker
//! shaded band). Line art must burn the lines and not the paper at every Strength; Black and white
//! must not burn the shaded paper at Strength 100. Burned is the Threshold 50 % dither of both
//! presets: a grey below 128.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::Point;
use curvyo_image_core::{Adjusted, GreyPlane, WorkingImage};
use curvyo_laser_core::adjust::{AdjustRegisters, Preset, adjust_picture};

/// The side of both pictures in pixels; at 0.1 mm a pixel, 40 mm square.
const SIDE: u32 = 400;
const PITCH_MM: f64 = 0.1;

fn frame() -> [Point; 4] {
    let mm = f64::from(SIDE) * PITCH_MM;
    [
        Point::new(0.0, 0.0),
        Point::new(mm, 0.0),
        Point::new(mm, mm),
        Point::new(0.0, mm),
    ]
}

fn grain(seed: &mut u32, spread: i32) -> i32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    i32::try_from(*seed >> 16).unwrap() % (2 * spread + 1) - spread
}

fn image(grey: &[u8]) -> WorkingImage {
    WorkingImage {
        width: SIDE,
        height: SIDE,
        pixels: grey.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        original_size: (SIDE, SIDE),
        truncated: false,
        animated: false,
    }
}

fn byte(v: i32) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap()
}

/// What a pixel of the pen drawing is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The core of a pen line.
    Line,
    /// Within 3 px of a line: neither counted.
    Near,
    /// Paper.
    Paper,
}

/// The pen drawing: paper 205 to 235 lit from one corner, with a blotchy texture of about 1 mm
/// (6 greys) and grain of 6 greys, and lines of 2 to 8 px at greys 40 to 110: horizontals,
/// verticals, diagonals and two circles.
fn pen_drawing() -> (Vec<u8>, Vec<Part>) {
    let n = SIDE as usize;
    let mut distance = vec![f64::MAX; n * n];
    let mut ink = vec![255_i32; n * n];
    let mut stroke = |f: &dyn Fn(f64, f64) -> f64, half: f64, grey: i32| {
        for y in 0..n {
            for x in 0..n {
                #[allow(clippy::cast_precision_loss)]
                let d = f(x as f64, y as f64) - half;
                let at = y * n + x;
                if d < distance[at] {
                    distance[at] = d;
                }
                if d <= 0.0 {
                    ink[at] = ink[at].min(grey);
                }
            }
        }
    };
    for (i, (half, grey)) in [(1.0, 40), (1.5, 70), (2.0, 90), (3.0, 110), (4.0, 60)]
        .into_iter()
        .enumerate()
    {
        #[allow(clippy::cast_precision_loss)]
        let row = 40.0 + i as f64 * 70.0;
        stroke(&|_, y| (y - row).abs(), half, grey);
        stroke(&|x, _| (x - row).abs(), half, grey);
    }
    stroke(&|x, y| (x - y).abs() / 2_f64.sqrt(), 1.5, 60);
    stroke(&|x, y| (x + y - 400.0).abs() / 2_f64.sqrt(), 1.0, 80);
    stroke(
        &|x, y| ((x - 200.0).hypot(y - 200.0) - 120.0).abs(),
        1.5,
        50,
    );
    stroke(&|x, y| ((x - 120.0).hypot(y - 280.0) - 40.0).abs(), 1.0, 90);
    let mut seed = 7;
    let mut grey = Vec::with_capacity(n * n);
    let mut parts = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let at = y * n + x;
            let paper = 235 - i32::try_from((x + y) * 30 / (2 * n)).unwrap();
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let blotch = (3.0 * ((x as f64 / 7.0).sin() + (y as f64 / 9.0 + x as f64 / 23.0).cos()))
                .round() as i32;
            let g = (paper + blotch).min(ink[at]) + grain(&mut seed, 6);
            grey.push(byte(g));
            parts.push(if distance[at] <= -0.5 {
                Part::Line
            } else if distance[at] <= 3.0 {
                Part::Near
            } else {
                Part::Paper
            });
        }
    }
    (grey, parts)
}

/// The soft-shaded photo: paper 215 with a darker shaded band (165 to 190, soft edges) across its
/// lower third, and a lit subject (a soft disc 60 to 140) in the upper half; grain of 3 greys.
/// The second list marks the shaded paper (the band away from the subject).
fn shaded_photo() -> (Vec<u8>, Vec<bool>) {
    let n = SIDE as usize;
    let mut seed = 11;
    let mut grey = Vec::with_capacity(n * n);
    let mut shaded = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            #[allow(clippy::cast_precision_loss)]
            let (fx, fy) = (x as f64, y as f64);
            // The band: full shade from y 290 to 360, a 30 px soft edge each side.
            let band =
                ((fy - 260.0) / 30.0).clamp(0.0, 1.0) * ((390.0 - fy) / 30.0).clamp(0.0, 1.0);
            let shade = 165.0 + 25.0 * fx / 400.0;
            let paper = 215.0 + (shade - 215.0) * band;
            let r = (fx - 200.0).hypot(fy - 140.0);
            let disc = ((110.0 - r) / 25.0).clamp(0.0, 1.0);
            let subject = 60.0 + 80.0 * (r / 110.0).min(1.0);
            let g = paper + (subject - paper) * disc;
            #[allow(clippy::cast_possible_truncation)]
            grey.push(byte(g.round() as i32 + grain(&mut seed, 3)));
            shaded.push((290..360).contains(&y));
        }
    }
    (grey, shaded)
}

fn adjusted(grey: &[u8], preset: Preset, strength: u8) -> Vec<u8> {
    let registers = AdjustRegisters {
        preset,
        strength,
        steps: Vec::new(),
        ..AdjustRegisters::default()
    };
    let (_, out) = adjust_picture(&image(grey), &frame(), &registers, 100);
    match out {
        Adjusted::Unchanged => grey.to_vec(),
        Adjusted::Rgba(rgba) => GreyPlane::of_rgba(&rgba).grey,
        Adjusted::Grey(plane) => plane.grey,
    }
}

/// The share of the pixels `of` marks that burn at Threshold 50 %, in percent.
fn burned(grey: &[u8], of: impl Fn(usize) -> bool) -> f64 {
    let (mut all, mut hot) = (0_u32, 0_u32);
    for (i, &g) in grey.iter().enumerate() {
        if of(i) {
            all += 1;
            hot += u32::from(g < 128);
        }
    }
    f64::from(hot) * 100.0 / f64::from(all.max(1))
}

#[test]
fn line_art_burns_the_pen_lines_and_not_the_paper_at_every_strength() {
    let (grey, parts) = pen_drawing();
    let mut failed = Vec::new();
    for strength in [0_u8, 25, 50, 75, 100] {
        let out = adjusted(&grey, Preset::LineArt, strength);
        let lines = burned(&out, |i| parts[i] == Part::Line);
        let paper = burned(&out, |i| parts[i] == Part::Paper);
        println!("line art s{strength}: lines {lines:.1} %, paper {paper:.2} %");
        if lines < 90.0 || paper > 0.5 {
            failed.push(format!(
                "s{strength}: lines {lines:.1} %, paper {paper:.2} %"
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "lines >= 90 %, paper <= 0.5 %: {failed:?}"
    );
}

#[test]
fn black_and_white_keeps_shaded_paper_white_at_full_strength() {
    let (grey, shaded) = shaded_photo();
    let mut failed = Vec::new();
    for strength in [0_u8, 50, 100] {
        let out = adjusted(&grey, Preset::BlackWhite, strength);
        let paper = burned(&out, |i| shaded[i]);
        let subject = burned(&out, |i| {
            let (x, y) = (i % SIDE as usize, i / SIDE as usize);
            #[allow(clippy::cast_precision_loss)]
            let r = (x as f64 - 200.0).hypot(y as f64 - 140.0);
            r < 60.0
        });
        println!("black and white s{strength}: shaded paper {paper:.1} %, subject {subject:.1} %");
        if paper > 5.0 || subject < 90.0 {
            failed.push(format!(
                "s{strength}: paper {paper:.1} %, subject {subject:.1} %"
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "shaded paper <= 5 %, subject >= 90 %: {failed:?}"
    );
}
