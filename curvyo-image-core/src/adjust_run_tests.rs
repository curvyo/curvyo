use super::*;

fn image() -> WorkingImage {
    let pixels = (0..64_u8)
        .flat_map(|i| [i * 4, 255 - i * 3, i, 200])
        .collect();
    WorkingImage {
        width: 8,
        height: 8,
        pixels,
        original_size: (8, 8),
        truncated: false,
        animated: false,
    }
}

/// Criterion 56: a 16 Mpx picture runs within about 2 x its bytes in colour, 1.5 x grey.
#[test]
fn a_run_holds_the_source_and_one_buffer() {
    let px = 16_000_000_u64;
    let denoise = PixelStep::Denoise {
        strength_pct: 30,
        detail_pct: 60,
    };
    let levels = PixelStep::Levels {
        black_pct: 0,
        white_pct: 100,
        gamma_centi: 120,
    };
    assert_eq!(peak_bytes(px, &[denoise, levels]), 2 * 4 * px);
    assert_eq!(peak_bytes(px, &[levels, levels]), 6 * px, "1.5 x once grey");
    assert_eq!(peak_bytes(px, &[]), 0);
}

/// Criterion 56: a step that does not fit is not run, nor any after it; the steps before it
/// are the picture.
#[test]
fn a_step_over_the_budget_is_not_run_with_those_after_it() {
    let denoise = PixelStep::Denoise {
        strength_pct: 40,
        detail_pct: 50,
    };
    let levels = PixelStep::Levels {
        black_pct: 20,
        white_pct: 80,
        gamma_centi: 100,
    };
    let px = 64;
    // Levels first fits in 6 x, a following Denoise on the grey too; Denoise first needs 8 x.
    let run = AdjustRun::within(&image(), &[levels, denoise], 6 * px);
    assert_eq!(run.not_run(), None);
    let run = AdjustRun::within(&image(), &[denoise, levels], 7 * px);
    assert_eq!(run.not_run(), Some(0));
    assert_eq!(run.finish(), Adjusted::Unchanged, "nothing ran");
    let sharpen = PixelStep::Sharpen {
        radius: SubPx(16),
        amount_pct: 100,
        threshold_pct: 0,
    };
    let run = AdjustRun::within(&image(), &[levels, sharpen], 5 * px);
    assert_eq!(run.not_run(), Some(0));
    let all = AdjustRun::within(&image(), &[levels, sharpen], u64::MAX);
    assert_eq!(all.not_run(), None);
    assert_eq!(all.finish(), run_steps(&image(), &[levels, sharpen]));
}

#[test]
fn no_step_is_unchanged_and_neutral_levels_give_the_0123_grey() {
    assert_eq!(run_steps(&image(), &[]), Adjusted::Unchanged);
    let neutral = PixelStep::Levels {
        black_pct: 0,
        white_pct: 100,
        gamma_centi: 100,
    };
    assert_eq!(
        run_steps(&image(), &[neutral]),
        Adjusted::Grey(GreyPlane::of_rgba(&image()))
    );
}

#[test]
fn denoise_alone_stays_in_colour_and_before_a_grey_step_runs_per_channel() {
    let d = PixelStep::Denoise {
        strength_pct: 0,
        detail_pct: 50,
    };
    let Adjusted::Rgba(out) = run_steps(&image(), &[d]) else {
        panic!("colour expected");
    };
    assert!(out.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    assert_eq!(GreyPlane::of_rgba(&out), GreyPlane::of_rgba(&image()));
    let neutral = PixelStep::Levels {
        black_pct: 0,
        white_pct: 100,
        gamma_centi: 100,
    };
    assert_eq!(
        run_steps(&image(), &[d, neutral]),
        Adjusted::Grey(GreyPlane::of_rgba(&image()))
    );
}

fn noisy(width: u32, height: u32) -> WorkingImage {
    let mut seed = 7_u32;
    let pixels = (0..width * height)
        .flat_map(|i| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = u8::try_from(seed >> 27).unwrap();
            let g = u8::try_from((i % width) * 200 / width).unwrap() + n;
            [g, g.wrapping_add(40), g / 2, 255 - n]
        })
        .collect();
    WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

/// Criteria 6, 7: Channel mix turns the colour grey with its weights; before it Denoise and
/// Crop run per channel, after the first grey step it changes nothing; a neutral picture
/// keeps its grey.
#[test]
fn channel_mix_is_the_grey_conversion_and_nothing_on_grey() {
    let red = PixelStep::ChannelMix {
        weights: [100, 0, 0],
    };
    let Adjusted::Grey(out) = run_steps(&image(), &[red]) else {
        panic!("grey expected");
    };
    let p = &image().pixels;
    assert_eq!(
        out.grey[5],
        over_white(p[20], p[23]),
        "the red channel over white"
    );
    let levels = PixelStep::Levels {
        black_pct: 0,
        white_pct: 100,
        gamma_centi: 100,
    };
    assert_eq!(
        run_steps(&image(), &[levels, red]),
        run_steps(&image(), &[levels]),
        "after grey it changes nothing"
    );
    let denoise = PixelStep::Denoise {
        strength_pct: 50,
        detail_pct: 50,
    };
    assert!(matches!(
        run_steps(&image(), &[denoise, red]),
        Adjusted::Grey(_)
    ));
    assert!(matches!(
        run_steps(&image(), &[red, denoise]),
        Adjusted::Grey(_)
    ));
    let grey_picture = WorkingImage {
        pixels: image()
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], 255])
            .collect(),
        ..image()
    };
    assert_eq!(
        run_steps(
            &grey_picture,
            &[PixelStep::ChannelMix {
                weights: [200, 0, 0]
            }]
        ),
        Adjusted::Grey(GreyPlane::of_rgba(&grey_picture)),
    );
}

/// Criteria 5, 14: Crop and Resize change the size in colour; Resize never enlarges.
#[test]
fn crop_and_resize_change_the_size() {
    let crop = PixelStep::Crop {
        pct: [25, 25, 50, 50],
    };
    let Adjusted::Rgba(out) = run_steps(&image(), &[crop]) else {
        panic!("colour expected");
    };
    assert_eq!((out.width, out.height), (4, 4));
    let resize = PixelStep::Resize { width_px: 2 };
    let Adjusted::Rgba(out) = run_steps(&image(), &[crop, resize]) else {
        panic!("colour expected");
    };
    assert_eq!((out.width, out.height), (2, 2));
    let Adjusted::Rgba(out) = run_steps(&image(), &[PixelStep::Resize { width_px: 99 }]) else {
        panic!("colour expected");
    };
    assert_eq!((out.width, out.height), (8, 8));
}

/// Criterion 13: a run worked a few units at a time gives the bytes of the one-shot run,
/// for every slice-1 step, in colour and in grey.
#[test]
fn a_stepped_run_equals_the_one_shot_run() {
    let image = noisy(41, 29);
    let denoise = PixelStep::Denoise {
        strength_pct: 70,
        detail_pct: 40,
    };
    let stacks: [&[PixelStep]; 6] = [
        &[
            PixelStep::Crop {
                pct: [10, 5, 80, 90],
            },
            PixelStep::Resize { width_px: 20 },
            PixelStep::ChannelMix {
                weights: [50, 40, 10],
            },
            PixelStep::Curves {
                points: [
                    (0, 0),
                    (30, 20),
                    (70, 90),
                    (100, 100),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                count: 4,
            },
        ],
        &[denoise, PixelStep::Resize { width_px: 17 }],
        &[denoise],
        &[
            denoise,
            PixelStep::Levels {
                black_pct: 5,
                white_pct: 90,
                gamma_centi: 130,
            },
            PixelStep::LocalContrast {
                radius: SubPx(16 * 6),
                amount_pct: 120,
            },
            PixelStep::Sharpen {
                radius: SubPx(24),
                amount_pct: 200,
                threshold_pct: 3,
            },
        ],
        &[PixelStep::Contours {
            sigma: SubPx(20),
            sensitivity_pct: 60,
        }],
        &[
            PixelStep::Sharpen {
                radius: SubPx(40),
                amount_pct: 0,
                threshold_pct: 0,
            },
            denoise,
        ],
    ];
    for steps in stacks {
        let once = run_steps(&image, steps);
        for work in [1, 50, 700] {
            let mut run = AdjustRun::new(&image, steps);
            let mut calls = 0;
            while !run.advance(work) {
                calls += 1;
                assert!(calls < 100_000);
            }
            assert!(run.done());
            if work == 1 {
                assert!(calls + 1 >= 29, "at least a call per row: {calls}");
            }
            assert_eq!(run.finish(), once, "{steps:?} at {work}");
        }
    }
}
