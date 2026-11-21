//! Runs a list of pixel steps on a working image (ADR 0073 §3, §7, §9), at once or a band of rows
//! at a time between the host's ticks (`specs/0135` criterion 13): the adjusted image.

use crate::adjust_blur::SubPx;
use crate::adjust_clahe::ClaheRun;
use crate::adjust_denoise::DenoiseRun;
use crate::adjust_detail::DetailRun;
use crate::adjust_geometry::{crop_plane, resize_plane};
use crate::adjust_plane::{GreyPlane, luminance, over_white};
use crate::adjust_tone::{TableRun, channel_mix_grey};
use crate::working::WorkingImage;

/// One adjustment step in pixel units, as it runs (ADR 0073 §1): radii in 1/16 pixel of the
/// step's input, greys and percentages as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelStep {
    /// Keeps the pixel rectangle of left, top, width and height in percent
    /// ([`crate::crop_rect`]).
    Crop {
        /// Left, top, width and height, whole percent.
        pct: [u8; 4],
    },
    /// Reduces the picture to this width in pixels, the height following
    /// ([`crate::resized_size`]).
    Resize {
        /// The width in pixels.
        width_px: u32,
    },
    /// How colour becomes grey: red, green and blue weights in whole percent (-200 to 200).
    ChannelMix {
        /// Red, green and blue.
        weights: [i16; 3],
    },
    /// The bilateral filter: Strength and Detail kept in percent.
    Denoise {
        /// Strength, 0 to 100 %.
        strength_pct: u8,
        /// Detail kept, 0 to 100 %.
        detail_pct: u8,
    },
    /// Black point, White point in percent of the grey range, Gamma in hundredths.
    Levels {
        /// Black point, 0 to 98 %.
        black_pct: u8,
        /// White point, 2 to 100 %.
        white_pct: u8,
        /// Gamma in hundredths, 10 to 400.
        gamma_centi: u16,
    },
    /// The monotone curve through the first `count` of `points` (Input %, Output %).
    Curves {
        /// The points, Input strictly increasing; the ones past `count` unused.
        points: [(u8, u8); 8],
        /// How many points are used.
        count: u8,
    },
    /// CLAHE with tiles of 2 x `radius`.
    LocalContrast {
        /// The radius.
        radius: SubPx,
        /// Amount, 0 to 200 %.
        amount_pct: u16,
    },
    /// The dark lines of a difference of blurs at `sigma` (half the line width) and 1.6 x that.
    Contours {
        /// Half the line width.
        sigma: SubPx,
        /// Sensitivity, 0 to 100 %.
        sensitivity_pct: u8,
    },
    /// The unsharp mask.
    Sharpen {
        /// The blur's sigma.
        radius: SubPx,
        /// Amount, 0 to 500 %.
        amount_pct: u16,
        /// Threshold, 0 to 50 %.
        threshold_pct: u8,
    },
}

impl PixelStep {
    /// Whether the step runs on each colour channel while the picture is still in colour.
    const fn per_channel(self) -> bool {
        matches!(
            self,
            Self::Denoise { .. } | Self::Crop { .. } | Self::Resize { .. }
        )
    }

    /// The step's run on `plane`, nothing worked yet.
    fn start(self, plane: &GreyPlane) -> StepRun {
        match self {
            Self::Crop { pct } => StepRun::Geometry(Geometry::Crop(pct)),
            Self::Resize { width_px } => StepRun::Geometry(Geometry::Resize(width_px)),
            // The grey conversion is the step; on a grey plane it changes nothing (criterion 6).
            Self::ChannelMix { .. } => StepRun::Table(TableRun::curves(&[])),
            Self::Curves { points, count } => {
                StepRun::Table(TableRun::curves(&points[..usize::from(count.min(8))]))
            }
            Self::Denoise {
                strength_pct,
                detail_pct,
            } => StepRun::Denoise(Box::new(DenoiseRun::new(plane, strength_pct, detail_pct))),
            Self::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            } => StepRun::Table(TableRun::levels(black_pct, white_pct, gamma_centi)),
            Self::LocalContrast { radius, amount_pct } => {
                StepRun::Clahe(ClaheRun::new(plane, radius, amount_pct))
            }
            Self::Contours {
                sigma,
                sensitivity_pct,
            } => StepRun::Detail(DetailRun::contours(plane, sigma, sensitivity_pct)),
            Self::Sharpen {
                radius,
                amount_pct,
                threshold_pct,
            } => StepRun::Detail(DetailRun::sharpen(plane, radius, amount_pct, threshold_pct)),
        }
    }
}

/// One step being worked on one plane.
#[derive(Debug, Clone)]
enum StepRun {
    Geometry(Geometry),
    Denoise(Box<DenoiseRun>),
    Table(TableRun),
    Clahe(ClaheRun),
    Detail(DetailRun),
}

impl StepRun {
    fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        match self {
            Self::Geometry(step) => {
                *work = work.saturating_sub(plane.pixels());
                *plane = match *step {
                    Geometry::Crop(pct) => crop_plane(plane, pct),
                    Geometry::Resize(width_px) => resize_plane(plane, width_px),
                };
                true
            }
            Self::Denoise(run) => run.work(plane, work),
            Self::Table(run) => run.work(plane, work),
            Self::Clahe(run) => run.work(plane, work),
            Self::Detail(run) => run.work(plane, work),
        }
    }
}

/// A geometry step, worked on a plane at once (a crop is a copy, a resize one pass of each
/// axis).
#[derive(Debug, Clone, Copy)]
enum Geometry {
    Crop([u8; 4]),
    Resize(u32),
}

/// The adjusted image of a working image (ADR 0073 §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adjusted {
    /// No step: the working image itself (the `0123` input, byte for byte).
    Unchanged,
    /// Colour steps only: opaque RGBA, alpha composited over white.
    Rgba(WorkingImage),
    /// After the first grey step.
    Grey(GreyPlane),
}

/// `steps` run in order on `image`. Steps that run per channel (Denoise) before the first grey
/// step run on red, green and blue composited over white; the first other step turns the
/// picture grey with the luminance of `over_white_grey` (so a lone neutral step reproduces the
/// `0123` grey), and every step from there runs on the grey. An empty list is
/// [`Adjusted::Unchanged`].
#[must_use]
pub fn run_steps(image: &WorkingImage, steps: &[PixelStep]) -> Adjusted {
    AdjustRun::new(image, steps).finish()
}

/// The work units of one pixel turned from three channels to grey.
const CONVERT_WORK: u64 = 2;

/// The planes an adjustment is worked on.
#[derive(Debug, Clone)]
enum Planes {
    Unchanged,
    Channels(Box<[GreyPlane; 3]>),
    Grey(GreyPlane),
}

/// [`run_steps`] worked a band of rows at a time (`specs/0135` criterion 13, ADR 0073 §8): each
/// [`AdjustRun::advance`] works about the units it is given (one unit per pixel of a box pass,
/// 24 per denoised pixel) and at least one row, so no tick of the host runs a whole adjustment;
/// the result is the bytes of [`run_steps`].
#[derive(Debug, Clone)]
pub struct AdjustRun {
    steps: Vec<PixelStep>,
    /// The first step that runs on the grey.
    first_grey: Option<usize>,
    planes: Planes,
    /// The step being worked and, before the first grey step, its channel.
    next: usize,
    channel: usize,
    running: Option<StepRun>,
    /// The metadata of the input, for a colour result.
    meta: ((u32, u32), bool, bool),
    /// The first step that did not fit the memory budget, and every one after it, not run.
    not_run: Option<usize>,
}

/// The bytes a run holds while step `index` of `steps` is worked on a picture of `pixels`
/// pixels (ADR 0073 §9, `specs/0135` criterion 56): the RGBA source the caller keeps, the planes
/// (three channels before the first grey step, one grey after it; both while turning grey) and
/// the copy the step starts from.
#[must_use]
pub fn step_bytes(pixels: u64, steps: &[PixelStep], index: usize) -> u64 {
    let planes = match steps.iter().position(|s| !s.per_channel()) {
        // The three channels and the grey made from them.
        Some(grey) if grey == index && index > 0 => return 8 * pixels,
        Some(grey) if index >= grey => 1,
        _ => 3,
    };
    4 * pixels + (planes + 1) * pixels
}

/// The most bytes [`step_bytes`] reaches over `steps`: about 2 x the working image for a colour
/// chain, 1.5 x once grey.
#[must_use]
pub fn peak_bytes(pixels: u64, steps: &[PixelStep]) -> u64 {
    (0..steps.len())
        .map(|i| step_bytes(pixels, steps, i))
        .max()
        .unwrap_or(0)
}

impl AdjustRun {
    /// The run of `steps` on `image`; the picture is read (composited over white, turned grey
    /// for a first grey step) now.
    #[must_use]
    pub fn new(image: &WorkingImage, steps: &[PixelStep]) -> Self {
        Self::within(image, steps, u64::MAX)
    }

    /// [`Self::new`] holding at most `budget` bytes (criterion 56): the first step whose
    /// [`step_bytes`] exceed it and every step after it are not run, and the picture is what the
    /// steps before it made.
    #[must_use]
    pub fn within(image: &WorkingImage, steps: &[PixelStep], budget: u64) -> Self {
        let pixels = u64::from(image.width.max(1)) * u64::from(image.height.max(1));
        let fits = (0..steps.len()).find(|&i| step_bytes(pixels, steps, i) > budget);
        let steps = &steps[..fits.unwrap_or(steps.len())];
        let first_grey = steps.iter().position(|s| !s.per_channel());
        let planes = if steps.is_empty() {
            Planes::Unchanged
        } else if first_grey == Some(0) && !matches!(steps[0], PixelStep::ChannelMix { .. }) {
            Planes::Grey(GreyPlane::of_rgba(image))
        } else {
            Planes::Channels(Box::new(channels(image)))
        };
        Self {
            steps: steps.to_vec(),
            first_grey,
            planes,
            next: 0,
            channel: 0,
            running: None,
            meta: (image.original_size, image.truncated, image.animated),
            not_run: fits,
        }
    }

    /// The index of the first step that did not fit the budget (it and the steps after it are
    /// not run); `None` when every step runs.
    #[must_use]
    pub const fn not_run(&self) -> Option<usize> {
        self.not_run
    }

    /// Whether every step has run.
    #[must_use]
    pub const fn done(&self) -> bool {
        self.next >= self.steps.len()
    }

    /// Works about `work` units, at least one row; whether every step has run.
    pub fn advance(&mut self, work: u64) -> bool {
        let mut work = work;
        while !self.done() {
            if Some(self.next) == self.first_grey
                && let Planes::Channels(channels) = &self.planes
            {
                work = work.saturating_sub(CONVERT_WORK * channels[0].pixels());
                self.planes = Planes::Grey(match self.steps[self.next] {
                    PixelStep::ChannelMix { weights } => mixed_grey(channels, weights),
                    _ => grey_of(channels),
                });
            }
            let step = self.steps[self.next];
            let plane = match &mut self.planes {
                Planes::Unchanged => return true,
                Planes::Channels(channels) => &mut channels[self.channel],
                Planes::Grey(grey) => grey,
            };
            let run = self.running.get_or_insert_with(|| {
                // The start copies the plane.
                work = work.saturating_sub(plane.pixels());
                step.start(plane)
            });
            if !run.work(plane, &mut work) {
                return false;
            }
            self.running = None;
            if matches!(self.planes, Planes::Channels(_)) && self.channel < 2 {
                self.channel += 1;
            } else {
                self.channel = 0;
                self.next += 1;
            }
            if work == 0 {
                break;
            }
        }
        self.done()
    }

    /// The adjusted image, the steps not run yet run now.
    #[must_use]
    pub fn finish(mut self) -> Adjusted {
        self.advance(u64::MAX);
        let (original_size, truncated, animated) = self.meta;
        match self.planes {
            Planes::Unchanged => Adjusted::Unchanged,
            Planes::Channels(channels) => {
                Adjusted::Rgba(rgba((original_size, truncated, animated), &channels))
            }
            Planes::Grey(grey) => Adjusted::Grey(grey),
        }
    }
}

/// Red, green and blue of `image` composited over white, one plane each.
fn channels(image: &WorkingImage) -> [GreyPlane; 3] {
    let (width, height) = (image.width.max(1), image.height.max(1));
    let n = width as usize * height as usize;
    if image.pixels.len() != n * 4 {
        return [
            GreyPlane::white(1, 1),
            GreyPlane::white(1, 1),
            GreyPlane::white(1, 1),
        ];
    }
    let mut planes = [vec![0; n], vec![0; n], vec![0; n]];
    for (i, p) in image.pixels.as_chunks::<4>().0.iter().enumerate() {
        for (c, plane) in planes.iter_mut().enumerate() {
            plane[i] = over_white(p[c], p[3]);
        }
    }
    planes.map(|grey| GreyPlane {
        width,
        height,
        grey,
    })
}

/// The luminance grey of three channel planes.
fn grey_of(channels: &[GreyPlane; 3]) -> GreyPlane {
    let [r, g, b] = channels;
    GreyPlane {
        width: r.width,
        height: r.height,
        grey: r
            .grey
            .iter()
            .zip(&g.grey)
            .zip(&b.grey)
            .map(|((&r, &g), &b)| luminance(r, g, b))
            .collect(),
    }
}

/// The grey of three channel planes by Channel mix `weights` (criterion 6): a picture whose
/// every pixel is neutral keeps its grey.
fn mixed_grey(channels: &[GreyPlane; 3], weights: [i16; 3]) -> GreyPlane {
    let [r, g, b] = channels;
    let neutral = r.grey == g.grey && g.grey == b.grey;
    GreyPlane {
        width: r.width,
        height: r.height,
        grey: if neutral {
            r.grey.clone()
        } else {
            r.grey
                .iter()
                .zip(&g.grey)
                .zip(&b.grey)
                .map(|((&r, &g), &b)| channel_mix_grey(r, g, b, weights))
                .collect()
        },
    }
}

/// An opaque working image of three channel planes, with the input's original size, truncated
/// and animated marks.
fn rgba(
    (original_size, truncated, animated): ((u32, u32), bool, bool),
    channels: &[GreyPlane; 3],
) -> WorkingImage {
    let [r, g, b] = channels;
    let pixels = r
        .grey
        .iter()
        .zip(&g.grey)
        .zip(&b.grey)
        .flat_map(|((&r, &g), &b)| [r, g, b, 255])
        .collect();
    WorkingImage {
        width: r.width,
        height: r.height,
        pixels,
        original_size,
        truncated,
        animated,
    }
}

#[cfg(test)]
#[path = "adjust_run_tests.rs"]
mod tests;
