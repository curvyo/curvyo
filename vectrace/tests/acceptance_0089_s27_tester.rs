//! Tester cases for `specs/0089-trace-quality-and-colour` ux-notes §27.4 at the tracer: a palette deleted down to its last
//! colour is that colour alone (one fixed colour is the whole palette), the other counts and the automatic palette are as
//! before. Pictures are generated here; the results come from the public `trace_colours`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::ops::ControlFlow;
use vectrace::{Mm, PixelMap};

use vectrace::{
    ColourParams, ColourTrace, DEFAULT_BACKGROUND_TOLERANCE, Raster, Rgb8, Selection, TraceParams,
    trace_colours,
};

const WHITE: [u8; 3] = [255, 255, 255];
/// Four flat vertical bands on white.
const BANDS: [[u8; 3]; 4] = [[200, 30, 30], [30, 160, 60], [30, 60, 200], [230, 200, 20]];

fn picture() -> (u32, u32, Vec<u8>) {
    let (w, h) = (160_u32, 120_u32);
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let c = if (10..110).contains(&y) && (20..140).contains(&x) {
                BANDS[((x - 20) / 30) as usize]
            } else {
                WHITE
            };
            rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    (w, h, rgba)
}

fn run(colours: Option<u8>, fixed: &[Rgb8]) -> ColourTrace {
    let (w, h, rgba) = picture();
    let tp = TraceParams {
        selection: Selection::RemoveBackground {
            background: Some(Rgb8::new(255, 255, 255)),
            tolerance: DEFAULT_BACKGROUND_TOLERANCE,
        },
        specks: Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    };
    let mut cp = ColourParams::new(tp);
    cp.colours = colours;
    cp.fixed = fixed.to_vec();
    trace_colours(
        Raster {
            width: w,
            height: h,
            rgba: &rgba,
        },
        PixelMap::scale(0.1, 0.1),
        &cp,
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap()
}

#[test]
fn s27_one_fixed_colour_with_count_one_is_the_whole_palette() {
    let red = Rgb8::new(200, 30, 30);
    let t = run(Some(1), &[red]);
    assert_eq!(t.palette.len(), 1, "{:?}", t.palette);
    assert_eq!(t.palette[0].colour, red);
    assert!(t.palette[0].fixed);
    assert!(!t.layers.is_empty(), "the one colour is traced");
    assert!(
        t.layers.iter().all(|l| l.sample == red),
        "only that colour is traced"
    );
}

#[test]
fn s27_one_fixed_colour_that_is_in_no_pixel_is_still_the_palette() {
    let odd = Rgb8::new(1, 2, 3);
    let t = run(Some(1), &[odd]);
    assert_eq!(t.palette.len(), 1, "{:?}", t.palette);
    assert_eq!(t.palette[0].colour, odd);
}

#[test]
fn s27_without_a_fixed_colour_the_count_still_has_a_floor_and_the_found_palette_is_unchanged() {
    // Colours 1 without any fixed colour: no fixed palette, so the floor of two (or the single-colour path) applies as before.
    let one = run(Some(1), &[]);
    let two = run(Some(2), &[]);
    assert_eq!(
        one.palette.len(),
        two.palette.len(),
        "count 1 and 2 without a fixed colour agree: {:?} / {:?}",
        one.palette,
        two.palette
    );
    let a = run(None, &[]);
    let b = run(None, &[]);
    assert_eq!(a.palette, b.palette, "deterministic");
    assert_eq!(a.palette.len(), 4, "the four bands: {:?}", a.palette);
}

#[test]
fn s27_two_fixed_colours_and_more_are_unchanged() {
    let red = Rgb8::new(200, 30, 30);
    let blue = Rgb8::new(30, 60, 200);
    let t = run(Some(2), &[red, blue]);
    assert_eq!(t.palette.len(), 2, "{:?}", t.palette);
    let t = run(Some(1), &[red, blue]);
    assert_eq!(
        t.palette.len(),
        2,
        "two fixed colours are kept even with count 1: {:?}",
        t.palette
    );
    let t = run(Some(3), &[red]);
    assert_eq!(t.palette.len(), 3, "one fixed + two found: {:?}", t.palette);
    assert_eq!(t.palette[0].colour, red);
}
