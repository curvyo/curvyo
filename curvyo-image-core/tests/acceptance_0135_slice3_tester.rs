//! Tester acceptance cases for `specs/0135-image-optimisation` slice 3 in the pixel core,
//! written from the specification (criteria 47, 48) before the implementation was read: the AI
//! denoise Strength is an integer blend between the step's input (0 %) and the model output
//! (100 %), the Remove background Threshold and Edge softness map the stored mask without the
//! model, and the input hash changes with the input pixels and the model fields only.
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use curvyo_image_core::{WorkingImage, blend, input_hash, mask_map};

fn pic(w: u32, h: u32, seed: u8) -> WorkingImage {
    let pixels = (0..w * h * 4)
        .map(|i| {
            if i % 4 == 3 {
                255
            } else {
                (i as u8).wrapping_mul(7).wrapping_add(seed)
            }
        })
        .collect();
    WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    }
}

/// Criterion 47: Strength 0 keeps the input, 100 is the model output, 50 the rounded middle.
#[test]
fn c47_ai_denoise_strength_blends_input_and_output_in_integers() {
    for input in [0_u8, 1, 37, 128, 254, 255] {
        for ai in [0_u8, 3, 99, 200, 255] {
            assert_eq!(blend(input, ai, 0), input);
            assert_eq!(blend(input, ai, 100), ai);
            let b = blend(input, ai, 50);
            let (lo, hi) = (input.min(ai), input.max(ai));
            assert!(lo <= b && b <= hi, "{input} {ai} -> {b}");
            // in + (ai - in) * s / 100 rounded half away from zero, for every strength.
            for s in 0..=100_i32 {
                let num = (i32::from(ai) - i32::from(input)) * s;
                let q = num.signum() * ((num.abs() + 50) / 100);
                assert_eq!(
                    i32::from(blend(input, ai, s as u8)),
                    i32::from(input) + q,
                    "{input} {ai} {s}"
                );
            }
        }
    }
    // Monotone in the strength.
    let mut last = 10;
    for s in 0..=100 {
        let b = blend(10, 240, s);
        assert!(b >= last);
        last = b;
    }
}

/// Criterion 47: Threshold tightens or grows the subject, Edge softness widens the ramp; the
/// result is 0 to 255 and monotone in the mask value.
#[test]
fn c47_mask_threshold_and_softness_map_the_stored_mask() {
    for t in [0_u8, 25, 50, 75, 100] {
        for w in [0_u8, 5, 20] {
            let mut last = 0;
            for m in 0..=255_u8 {
                let v = mask_map(m, t, w);
                assert!(v >= last, "monotone at m {m}, T {t}, w {w}");
                last = v;
            }
        }
    }
    // Threshold 50: well below is out (0), well above is in (255).
    assert_eq!(mask_map(10, 50, 5), 0);
    assert_eq!(mask_map(245, 50, 5), 255);
    // A higher Threshold keeps less of the subject.
    let kept = |t: u8| (0..=255_u8).filter(|m| mask_map(*m, t, 5) == 255).count();
    assert!(kept(75) < kept(50) && kept(50) < kept(25));
    // More softness: more values between 0 and 255.
    let soft = |w: u8| {
        (0..=255_u8)
            .filter(|m| (1..255).contains(&mask_map(*m, 50, w)))
            .count()
    };
    assert!(soft(20) > soft(5));
}

/// Criterion 48: the input hash is the step's input pixels and its model fields; another model
/// field, another pixel, another size or another model version changes it; the same input
/// gives the same hash.
#[test]
fn c48_the_input_hash_follows_the_input_and_the_model_fields() {
    let a = pic(8, 4, 1);
    let model = ("realesr-general-x4v3", "1");
    let h = input_hash("upscale", model, &[("factor", 2)], &a);
    assert_eq!(
        h,
        input_hash("upscale", model, &[("factor", 2)], &a.clone())
    );
    assert_ne!(h, input_hash("upscale", model, &[("factor", 4)], &a));
    assert_ne!(
        h,
        input_hash(
            "upscale",
            ("realesr-general-x4v3", "2"),
            &[("factor", 2)],
            &a
        )
    );
    assert_ne!(h, input_hash("ai_denoise", model, &[("factor", 2)], &a));
    let mut b = a.clone();
    b.pixels[5] ^= 1;
    assert_ne!(h, input_hash("upscale", model, &[("factor", 2)], &b));
    // Same bytes, another shape.
    let mut c = a.clone();
    c.width = 4;
    c.height = 8;
    assert_ne!(h, input_hash("upscale", model, &[("factor", 2)], &c));
}
