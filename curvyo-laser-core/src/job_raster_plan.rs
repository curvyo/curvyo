//! The sweeps of one raster item from its leaf (ADR 0068, ADR 0069 §1, §2; `specs/0123`
//! criteria 1 to 10): grid, sample, tone and dither in one pass over the rows.

use curvyo_document_core::{PlacedGradient, Point};
use curvyo_image_core::Adjusted;

use crate::job_flatten::flat;
use crate::job_input::{LeafGeometry, LeafInput, PointUm};
use crate::job_raster_dither::Ditherer;
use crate::job_raster_grid::{DotGrid, GridRow, estimated_dots};
#[cfg(test)]
use crate::job_raster_rows::Sweep;
use crate::job_raster_sample::{GreyPyramid, ImageSampler, gradient_grey};
use crate::job_raster_tone::ToneTable;
#[cfg(test)]
use crate::job_raster_work::ItemWork;
use crate::material::Interval;
use crate::operation_image_options::{Dither, ImageSettings};
use crate::operation_options::OperationOptions;

/// The most dots one raster item may have (ADR 0068 §7, criterion 34).
pub const MAX_DOTS: u64 = 50_000_000;

/// Where a raster item's greys come from.
#[derive(Debug)]
pub enum RasterSource<'a> {
    /// An image through its grey pyramid.
    Image(ImageSampler<'a>),
    /// A gradient fill placed on the page.
    Gradient(&'a PlacedGradient),
}

impl RasterSource<'_> {
    fn grey_at(&self, p: Point) -> u8 {
        match self {
            Self::Image(sampler) => sampler.grey_at(p),
            Self::Gradient(gradient) => gradient_grey(gradient, p),
        }
    }
}

/// The dot settings of one raster item (ADR 0073 note of 2026-11-20: per leaf).
#[derive(Debug, Clone, Copy)]
pub struct RasterSettings {
    /// The Image settings the item burns with: the operation's, with a picture's own dither.
    pub image: ImageSettings,
    /// The Interval in µm.
    pub interval_um: i64,
    /// The Scan angle in degrees.
    pub angle_deg: f64,
    /// Whether rows alternate direction.
    pub bidirectional: bool,
}

/// The sweeps of the item bounded by `rings` with the greys of `source`, in run order; empty for
/// an item over [`MAX_DOTS`] (it blocks) or with no dot inside.
#[cfg(test)]
#[must_use]
pub fn item_sweeps(
    rings: &[Vec<PointUm>],
    source: &RasterSource<'_>,
    settings: RasterSettings,
) -> Vec<Sweep> {
    ItemWork::new(rings, settings).map_or_else(Vec::new, |mut work| {
        let mut all = u64::MAX;
        work.advance(source, &mut all);
        work.into_sweeps()
    })
}

/// The anchor of an item's screen (ADR 0074 §2): the first corner of its first ring, the visible
/// frame's first corner for an image.
pub(crate) fn screen_anchor(rings: &[Vec<PointUm>]) -> PointUm {
    rings
        .first()
        .and_then(|ring| ring.first())
        .copied()
        .unwrap_or(PointUm::new(0, 0))
}

/// Appends the levels of `row` (one per inside dot, stretches in order) to `out`, its greys
/// through `tone` and `ditherer`; the number of dots.
pub(crate) fn row_levels(
    grid: &DotGrid,
    row: &GridRow,
    source: &RasterSource<'_>,
    tone: &ToneTable,
    ditherer: &mut Ditherer,
    greys: &mut Vec<u8>,
    out: &mut Vec<u8>,
) -> u64 {
    let mut dots = 0;
    for &(j0, j1) in &row.stretches {
        greys.clear();
        greys.extend((j0..j1).map(|j| tone.apply(source.grey_at(grid.centre_mm(row.s, j)))));
        ditherer.stretch(
            row.k,
            j0,
            greys,
            |j| grid.point(row.s, grid.centre_t(j)),
            out,
        );
        dots += u64::try_from(j1 - j0).unwrap_or(0);
    }
    dots
}

/// The grid of the item and each row's levels (one per inside dot, stretches in order), `None`
/// over [`MAX_DOTS`] or with no dot inside.
fn item_levels(
    rings: &[Vec<PointUm>],
    source: &RasterSource<'_>,
    settings: RasterSettings,
) -> Option<(DotGrid, Vec<Vec<u8>>)> {
    let RasterSettings {
        image,
        interval_um,
        angle_deg,
        ..
    } = settings;
    let image = &image;
    if estimated_dots(rings, interval_um, angle_deg) > MAX_DOTS {
        return None;
    }
    let grid = DotGrid::new(rings, interval_um, angle_deg)?;
    let tone = ToneTable::new(image);
    let mut ditherer =
        Ditherer::new(image, 0, grid.columns).screened(image, interval_um, screen_anchor(rings));
    let mut greys = Vec::new();
    let levels = grid
        .rows
        .iter()
        .map(|row| {
            let mut levels = Vec::new();
            row_levels(
                &grid,
                row,
                source,
                &tone,
                &mut ditherer,
                &mut greys,
                &mut levels,
            );
            levels
        })
        .collect();
    Some((grid, levels))
}

/// The dot map of one item as text (the golden format of `specs/0123` criterion 40): one line
/// per grid row, `.` off and `#` on, two hex digits per dot in Grey, outside the item spaces,
/// trailing spaces trimmed.
#[must_use]
pub fn dot_map_text(
    rings: &[Vec<PointUm>],
    source: &RasterSource<'_>,
    settings: RasterSettings,
) -> String {
    let Some((grid, levels)) = item_levels(rings, source, settings) else {
        return String::new();
    };
    let grey = settings.image.dither == Dither::Grey;
    let width = if grey { 2 } else { 1 };
    let mut text = String::new();
    let mut next_k = grid.rows.first().map_or(0, |r| r.k);
    for (row, levels) in grid.rows.iter().zip(&levels) {
        for _ in next_k..row.k {
            text.push('\n');
        }
        next_k = row.k + 1;
        let mut line = vec![b' '; grid.columns * width];
        let mut at = levels.iter();
        for &(j0, j1) in &row.stretches {
            for j in j0..j1 {
                let level = at.next().copied().unwrap_or(0);
                let column = usize::try_from(j).unwrap_or(0) * width;
                if grey {
                    let hex = format!("{level:02x}");
                    line[column..column + 2].copy_from_slice(hex.as_bytes());
                } else {
                    line[column] = if level > 0 { b'#' } else { b'.' };
                }
            }
        }
        text.push_str(String::from_utf8_lossy(&line).trim_end());
        text.push('\n');
    }
    text
}

/// The Image settings a raster leaf of an operation with `operation` burns with (ADR 0072
/// amendment A4): a picture its own dither with the operation's tone fields and Lowest power, a
/// gradient-filled leaf the operation's settings.
#[must_use]
pub fn leaf_image_settings(operation: &ImageSettings, geometry: &LeafGeometry) -> ImageSettings {
    match geometry {
        LeafGeometry::Image { adjust, .. } => operation.with_dither(adjust.dither),
        LeafGeometry::Gradient { .. } | LeafGeometry::Outlines(_) => *operation,
    }
}

/// Whether `geometry` burns raster dots: a gradient leaf or a picture whose pixels can be read.
/// An outline burns none, a picture that cannot be read burns nothing.
#[must_use]
pub(crate) fn burns_dots(geometry: &LeafGeometry) -> bool {
    !matches!(
        geometry,
        LeafGeometry::Outlines(_) | LeafGeometry::Image { pixels: None, .. }
    )
}

/// Whether any of `leaves` burns Grey with the operation's Image settings `image` (a picture with
/// its own dither included, ADR 0072 amendment A4). The one rule of the Grey check, the Settings
/// view and the numbers line; a picture that cannot be read does not count.
#[must_use]
pub fn burns_grey<'a>(
    image: &ImageSettings,
    leaves: impl IntoIterator<Item = &'a LeafInput>,
) -> bool {
    leaves.into_iter().any(|leaf| {
        burns_dots(&leaf.geometry)
            && leaf_image_settings(image, &leaf.geometry).dither == Dither::Grey
    })
}

/// The dot map text of a raster leaf (the golden format of `specs/0123` criterion 40) with an operation's Image settings,
/// Interval and Scan angle; empty for a leaf that is no raster item or whose pixels cannot be
/// read.
#[must_use]
pub fn raster_dot_map(
    geometry: &LeafGeometry,
    options: &OperationOptions,
    interval: Interval,
) -> String {
    raster_dot_map_adjusted(geometry, options, interval, &Adjusted::Unchanged)
}

/// [`raster_dot_map`] of an image leaf whose picture is `adjusted` (ADR 0073 §7; `specs/0135`
/// criteria 19, 54); [`Adjusted::Unchanged`] is the leaf's own pixels. A test hook for the
/// goldens.
#[doc(hidden)]
#[must_use]
pub fn raster_dot_map_adjusted(
    geometry: &LeafGeometry,
    options: &OperationOptions,
    interval: Interval,
    adjusted: &Adjusted,
) -> String {
    let settings = RasterSettings {
        image: leaf_image_settings(&options.image, geometry),
        interval_um: i64::from(interval.hundredths()) * 10,
        angle_deg: options.scan_angle.degrees(),
        bidirectional: options.bidirectional,
    };
    let interval_mm = f64::from(interval.hundredths()) / 100.0;
    match geometry {
        LeafGeometry::Image {
            frame,
            pixels: Some(pixels),
            ..
        } => {
            let pyramid = GreyPyramid::of_adjusted(pixels, adjusted);
            let ring = vec![frame.map(PointUm::of_mm).to_vec()];
            ImageSampler::new(&pyramid, frame, interval_mm).map_or_else(String::new, |sampler| {
                dot_map_text(&ring, &RasterSource::Image(sampler), settings)
            })
        }
        LeafGeometry::Gradient { outlines, gradient } => {
            let rings: Vec<Vec<PointUm>> = outlines.iter().filter_map(flat).collect();
            dot_map_text(&rings, &RasterSource::Gradient(gradient), settings)
        }
        LeafGeometry::Image { pixels: None, .. } | LeafGeometry::Outlines(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Color, GradientBox, GradientFill};

    fn square(x: i64, y: i64, side: i64) -> Vec<PointUm> {
        vec![
            PointUm::new(x, y),
            PointUm::new(x + side, y),
            PointUm::new(x + side, y + side),
            PointUm::new(x, y + side),
        ]
    }

    fn settings(image: &ImageSettings, bidirectional: bool) -> RasterSettings {
        RasterSettings {
            image: *image,
            interval_um: 100,
            angle_deg: 0.0,
            bidirectional,
        }
    }

    fn ramp() -> PlacedGradient {
        let frame = GradientBox::axis_aligned(Point::new(0.0, 0.0), Point::new(10.0, 1.0));
        PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &frame)
    }

    #[test]
    fn a_gradient_burns_full_at_black_and_nothing_at_white_in_grey() {
        let image = ImageSettings {
            dither: Dither::Grey,
            ..ImageSettings::DEFAULT
        };
        let gradient = ramp();
        let sweeps = item_sweeps(
            &[square(0, 0, 10_000)],
            &RasterSource::Gradient(&gradient),
            settings(&image, false),
        );
        let first = &sweeps[0];
        assert_eq!(first.start, PointUm::new(0, 50));
        let left = first.levels.get(0);
        let mid = first.levels.get(49);
        assert!(left >= 250, "{left}");
        assert!((118..=138).contains(&mid), "{mid}");
        assert!(first.end.x <= 10_000);
        assert!(
            first.levels.get(first.levels.len() - 1) > 0,
            "the last burned dot"
        );
    }

    #[test]
    fn bidirectional_rows_alternate_and_rows_run_in_order() {
        let image = ImageSettings {
            dither: Dither::Threshold,
            ..ImageSettings::DEFAULT
        };
        let gradient = ramp();
        let source = RasterSource::Gradient(&gradient);
        let rings = [square(0, 0, 10_000)];
        let both = item_sweeps(&rings, &source, settings(&image, true));
        assert!(both[0].start.x < both[0].end.x);
        assert!(both[1].start.x > both[1].end.x, "the second row runs back");
        let one = item_sweeps(&rings, &source, settings(&image, false));
        assert!(one.iter().all(|s| s.start.x < s.end.x));
        assert!(one.windows(2).all(|w| w[0].start.y < w[1].start.y));
    }

    #[test]
    fn an_item_over_the_dot_limit_builds_nothing() {
        let image = ImageSettings::DEFAULT;
        let gradient = ramp();
        let raster = RasterSettings {
            image,
            interval_um: 10,
            angle_deg: 0.0,
            bidirectional: true,
        };
        let sweeps = item_sweeps(
            &[square(0, 0, 80_000)],
            &RasterSource::Gradient(&gradient),
            raster,
        );
        assert!(sweeps.is_empty(), "64 million dots");
    }
}
