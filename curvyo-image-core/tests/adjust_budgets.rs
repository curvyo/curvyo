//! The adjustment budgets of `specs/0135-image-optimisation` criterion 55 (ADR 0073 §10) on a
//! 4 Mpx picture (2,000 x 2,000), thread CPU time, native and in release
//! (`cargo test --release -p curvyo-image-core --test adjust_budgets -- --ignored`): a
//! regression guard, the wasm numbers are stated in the PR.

#![allow(clippy::unwrap_used, missing_docs)]

mod common;

use std::time::Duration;

use common::thread_cpu;

use curvyo_image_core::{AdjustRun, Analysis, PixelStep, SubPx, WorkingImage, run_steps};

const SIDE: u32 = 2_000;

fn picture() -> WorkingImage {
    let mut seed = 3_u32;
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (seed >> 28) as u8;
            let g = u8::try_from((x / 8 + y / 16) % 200).unwrap() + n;
            pixels.extend([g, g.wrapping_add(10), g / 2, 255]);
        }
    }
    WorkingImage {
        width: SIDE,
        height: SIDE,
        pixels,
        original_size: (SIDE, SIDE),
        truncated: false,
        animated: false,
    }
}

/// The CPU time of `f` on this thread against `budget`.
fn timed<T>(name: &str, budget: Duration, f: impl FnOnce() -> T) -> T {
    let started = thread_cpu();
    let out = f();
    let took = thread_cpu().saturating_sub(started);
    println!("{name}: {took:?} (budget {budget:?})");
    assert!(took < budget, "{name} took {took:?}, budget {budget:?}");
    out
}

/// A grey step first, so the measured step runs on the grey plane as in a stack.
fn grey_then(step: PixelStep) -> [PixelStep; 2] {
    [
        PixelStep::Levels {
            black_pct: 0,
            white_pct: 100,
            gamma_centi: 100,
        },
        step,
    ]
}

#[test]
#[ignore = "budget benchmark, run in release"]
fn each_step_and_the_photo_stack_stay_within_their_budgets() {
    let image = picture();
    let ms = Duration::from_millis;
    // The grey conversion is part of the first step's time.
    timed("levels", ms(40), || {
        run_steps(
            &image,
            &[PixelStep::Levels {
                black_pct: 2,
                white_pct: 98,
                gamma_centi: 120,
            }],
        )
    });
    timed("sharpen", ms(120 + 40), || {
        run_steps(
            &image,
            &grey_then(PixelStep::Sharpen {
                radius: SubPx(16),
                amount_pct: 150,
                threshold_pct: 0,
            }),
        )
    });
    timed("local contrast", ms(400 + 40), || {
        run_steps(
            &image,
            &grey_then(PixelStep::LocalContrast {
                radius: SubPx(16 * 100),
                amount_pct: 80,
            }),
        )
    });
    timed("denoise", ms(800 + 40), || {
        run_steps(
            &image,
            &grey_then(PixelStep::Denoise {
                strength_pct: 40,
                detail_pct: 50,
            }),
        )
    });
    timed("analysis", ms(150), || Analysis::of(&image));
    timed("photo stack", ms(1_200), || {
        run_steps(
            &image,
            &[
                PixelStep::Denoise {
                    strength_pct: 25,
                    detail_pct: 60,
                },
                PixelStep::Levels {
                    black_pct: 1,
                    white_pct: 99,
                    gamma_centi: 110,
                },
                PixelStep::LocalContrast {
                    radius: SubPx(16 * 100),
                    amount_pct: 50,
                },
                PixelStep::Sharpen {
                    radius: SubPx(16),
                    amount_pct: 120,
                    threshold_pct: 0,
                },
            ],
        )
    });
}

/// The work one tick of the host gives an adjustment (`curvyo-laser-core`'s `ADJUST_WORK`).
const TICK_WORK: u64 = 12_000_000;

/// Criterion 55, slice 2: Channel mix, Curves, Crop and Resize each 40 ms on 4 Mpx.
#[test]
#[ignore = "budget benchmark, run in release"]
fn the_slice_2_steps_stay_within_their_budgets() {
    let image = picture();
    let ms = Duration::from_millis;
    // Curves after the grey conversion of a Levels step (40 ms of its own).
    timed("channel mix", ms(40), || {
        run_steps(
            &image,
            &[PixelStep::ChannelMix {
                weights: [60, 30, 10],
            }],
        )
    });
    timed("curves", ms(40 + 40), || {
        run_steps(
            &image,
            &grey_then(PixelStep::Curves {
                points: [
                    (0, 0),
                    (25, 15),
                    (75, 85),
                    (100, 100),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                count: 4,
            }),
        )
    });
    timed("crop", ms(40), || {
        run_steps(
            &image,
            &[PixelStep::Crop {
                pct: [10, 10, 80, 80],
            }],
        )
    });
    timed("resize", ms(40), || {
        run_steps(
            &image,
            &[PixelStep::Resize {
                width_px: image.width / 2,
            }],
        )
    });
}

/// Criterion 13: the Photo stack worked a tick at a time, no tick over 50 ms.
#[test]
#[ignore = "budget benchmark, run in release"]
fn a_stepped_photo_stack_keeps_every_tick_short() {
    let image = picture();
    let steps = [
        PixelStep::Denoise {
            strength_pct: 25,
            detail_pct: 60,
        },
        PixelStep::Levels {
            black_pct: 1,
            white_pct: 99,
            gamma_centi: 110,
        },
        PixelStep::LocalContrast {
            radius: SubPx(16 * 100),
            amount_pct: 50,
        },
        PixelStep::Sharpen {
            radius: SubPx(16),
            amount_pct: 120,
            threshold_pct: 0,
        },
        PixelStep::Contours {
            sigma: SubPx(32),
            sensitivity_pct: 50,
        },
    ];
    let started = thread_cpu();
    let mut run = AdjustRun::new(&image, &steps);
    // The first tick reads the picture too.
    let mut worst = thread_cpu().saturating_sub(started);
    let mut ticks = 0;
    loop {
        let tick = thread_cpu();
        let done = run.advance(TICK_WORK);
        worst = worst.max(thread_cpu().saturating_sub(tick));
        ticks += 1;
        if done {
            break;
        }
    }
    let total = thread_cpu().saturating_sub(started);
    println!("stepped photo stack: {ticks} ticks, worst {worst:?}, total {total:?}");
    assert!(worst < Duration::from_millis(50), "worst tick {worst:?}");
}
