//! The canvas look's queue and transparency (`specs/0135` criterion 15; ADR 0073 §7, §8).

use std::sync::Arc;

use curvyo_document_core::{Affine, NewImage, NodeId};
use curvyo_image_core::{WorkingImage, encode_png};

use super::super::tests::drawn_pictures;
use crate::session::Session;

/// A 40 x 20 picture whose left half is transparent, its right half an opaque ramp.
fn half_clear_png() -> Vec<u8> {
    let mut pixels = Vec::new();
    for _ in 0..20 {
        for x in 0..40_u32 {
            let g = u8::try_from(40 + x * 4).unwrap();
            pixels.extend_from_slice(&[g, g, g, if x < 20 { 0 } else { 255 }]);
        }
    }
    encode_png(&WorkingImage {
        width: 40,
        height: 20,
        pixels,
        original_size: (40, 20),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

fn place(s: &Session, bytes: &[u8]) -> NodeId {
    s.document
        .add_images(&[NewImage {
            bytes: Arc::from(bytes),
            placement: Affine::new(8.0, 0.0, 0.0, 4.0, 10.0, 10.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0]
}

fn finish_ticks(s: &Session) {
    let mut ticks = 0;
    while s.ticks_pending() {
        s.advance_tick();
        ticks += 1;
        assert!(ticks < 100);
    }
}

/// A4: the look of a transparent picture keeps its alpha; its colour is the adjusted picture.
#[test]
fn a_look_keeps_the_pictures_transparency() {
    let mut s = Session::new(290_137);
    let image = place(&s, &half_clear_png());
    s.selection.set(&[image]);
    assert!(s.add_adjust_step("levels"));
    assert!(s.edit_adjust_field(0, "black_pct", 40.0).unwrap());
    drawn_pictures(&mut s);
    finish_ticks(&s);
    let look = drawn_pictures(&mut s);
    let picture = &look[0].1;
    let row = &picture.pixels[..picture.width as usize * 4];
    let (left, right) = row.split_at(row.len() / 2);
    assert!(left.chunks(4).all(|p| p[3] == 0), "clear stays clear");
    assert!(right.chunks(4).all(|p| p[3] == 255), "opaque stays opaque");
}

/// A3: only the looks the last frame drew wait, the newest first: a drag's passed values are
/// dropped from the queue.
#[test]
fn only_the_looks_drawn_last_wait_newest_first() {
    let mut s = Session::new(290_138);
    let image = place(&s, &super::super::tests::ramp_png(40, 20));
    s.selection.set(&[image]);
    assert!(s.add_adjust_step("levels"));
    for black in [10.0, 20.0, 30.0] {
        assert!(s.edit_adjust_field(0, "black_pct", black).unwrap());
        drawn_pictures(&mut s);
    }
    let pending = s.image_adjust.look.pending.borrow().len();
    assert_eq!(pending, 1, "the passed values are not made");
    // Two pictures drawn: both wait, the one queued last first.
    let other = place(&s, &super::super::tests::ramp_png(40, 20));
    s.selection.set(&[other]);
    assert!(s.add_adjust_step("levels"));
    assert!(s.edit_adjust_field(0, "black_pct", 50.0).unwrap());
    drawn_pictures(&mut s);
    let look = &s.image_adjust.look;
    let queue = look.pending.borrow();
    assert_eq!(queue.len(), 2);
    let newest = &queue[0].0;
    assert!(
        newest.steps.iter().any(|step| matches!(
            step,
            curvyo_image_core::PixelStep::Levels { black_pct: 50, .. }
        )),
        "{newest:?}"
    );
}
