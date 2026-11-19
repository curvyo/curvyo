//! The dot picture of one raster item (`specs/0123` criterion 20; ADR 0069 §5; UX notes 3): its
//! sweeps rendered once into a texture of one texel per dot (or per power-of-two block of dots
//! for a grid too large for one texture), in the Engrave look on a transparent ground, so the
//! canvas draws the dots at the screen's resolution through the image pipeline's mip levels and
//! a pan or zoom never walks the dots.

use std::sync::Arc;

use curvyo_document_core::{Affine, AssetHash, ImageSnapshot, NodeId, Opacity, Point};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::job::{Levels, Sweep};
use curvyo_render_core::OP_ENGRAVE_HATCH;

/// The longest side of a dot picture in texels; a larger grid is averaged over blocks of
/// `2^n` x `2^n` dots (WebGL2 guarantees 2,048; the image pipeline skips levels a device cannot
/// hold).
const MAX_SIDE: usize = 4_096;

/// The opacity of a Grey dot of the lowest level, out of 255: 20 % (UX P26).
const GREY_FLOOR: u32 = 51;

/// One raster item's dots as a picture placed on the page.
#[derive(Debug, Clone)]
pub(in crate::session) struct RasterPicture {
    /// The image or gradient-filled leaf.
    pub node: NodeId,
    /// Its stacking place in its block, which tells apart the leaves of one clone.
    pub stacking: u32,
    /// The picture's name in the frame's image list (never an asset of the document).
    pub asset: AssetHash,
    /// The texels: the Engrave colour, alpha the dot's coverage.
    pub image: Arc<WorkingImage>,
    /// The unit square to page mm (the picture's top-left at the first row's outer edge).
    pub placement: Affine,
    /// The sweeps it was drawn from, held so a rebuilt plan with the same sweeps reuses it.
    pub sweeps: Vec<Arc<Sweep>>,
}

impl RasterPicture {
    /// Whether the picture was drawn from exactly `sweeps`.
    pub(in crate::session) fn drawn_from(&self, sweeps: &[Arc<Sweep>]) -> bool {
        self.sweeps.len() == sweeps.len()
            && self
                .sweeps
                .iter()
                .zip(sweeps)
                .all(|(a, b)| Arc::ptr_eq(a, b))
    }

    /// The picture as an image object in document mm (the page's corner at `origin`), drawn
    /// with `opacity`.
    pub(in crate::session) fn object(&self, origin: Point, opacity: Opacity) -> ImageSnapshot {
        let placement = Affine::translation(curvyo_document_core::Vec2::new(origin.x, origin.y))
            .compose(self.placement);
        let mut image = ImageSnapshot::new(self.node, self.asset.clone(), placement);
        image.shown_opacity = opacity;
        image
    }
}

/// The alpha of a dot of `level` (0 off): 255 in the binary modes, 20 % to 100 % by level in
/// Grey (UX P26).
fn dot_alpha(levels: &Levels, level: u8) -> u32 {
    match (levels, level) {
        (_, 0) => 0,
        (Levels::Binary { .. }, _) => 255,
        (Levels::Grey(_), level) => GREY_FLOOR + (255 - GREY_FLOOR) * u32::from(level) / 255,
    }
}

/// The frame every sweep of one item shares: the row direction, its normal, the interval.
struct GridFrame {
    origin: (f64, f64),
    along: (f64, f64),
    across: (f64, f64),
    interval: f64,
}

impl GridFrame {
    /// The frame of the longest of `sweeps` (the µm rounding of its ends tilts it least),
    /// turned to run as the first sweep runs: its start, its direction, its dot pitch, in µm.
    fn of(sweeps: &[Arc<Sweep>]) -> Option<Self> {
        let first = sweeps.first()?;
        let longest = sweeps.iter().rev().max_by_key(|sweep| sweep.levels.len())?;
        #[allow(clippy::cast_precision_loss)]
        // exact: page coordinates in µm are far below 2^53
        let delta = |s: &Sweep| ((s.end.x - s.start.x) as f64, (s.end.y - s.start.y) as f64);
        let (dx, dy) = delta(longest);
        let (fx, fy) = delta(first);
        let turn = if dx * fx + dy * fy < 0.0 { -1.0 } else { 1.0 };
        let length = (dx * dx + dy * dy).sqrt();
        #[allow(clippy::cast_precision_loss)]
        let interval = length / longest.levels.len().max(1) as f64;
        let (dx, dy) = (turn * dx / length, turn * dy / length);
        (length > 0.0 && interval > 0.0).then(|| Self {
            #[allow(clippy::cast_precision_loss)]
            origin: (longest.start.x as f64, longest.start.y as f64),
            along: (dx, dy),
            across: (-dy, dx),
            interval,
        })
    }

    /// The column and row (in dots, from the frame's origin) of `(x, y)` µm.
    fn cell(&self, x: f64, y: f64) -> (f64, f64) {
        let (px, py) = (x - self.origin.0, y - self.origin.1);
        (
            (px * self.along.0 + py * self.along.1) / self.interval,
            (px * self.across.0 + py * self.across.1) / self.interval,
        )
    }
}

/// One sweep in grid cells: its row, the column of its lowest-column dot, its dots and whether
/// it runs toward lower columns.
struct Placed<'a> {
    row: i64,
    column: i64,
    sweep: &'a Sweep,
    reversed: bool,
}

#[allow(clippy::cast_possible_truncation)]
// invariant: cells of a raster item at the dot limit are far inside i64
fn round(value: f64) -> i64 {
    value.round() as i64
}

/// The sweeps in grid cells of `frame`.
fn placed<'a>(frame: &GridFrame, sweeps: &'a [Arc<Sweep>]) -> Vec<Placed<'a>> {
    sweeps
        .iter()
        .map(|sweep| {
            #[allow(clippy::cast_precision_loss)]
            let (start, end) = (
                frame.cell(sweep.start.x as f64, sweep.start.y as f64),
                frame.cell(sweep.end.x as f64, sweep.end.y as f64),
            );
            let reversed = end.0 < start.0;
            Placed {
                row: round(start.1),
                column: round(start.0.min(end.0)),
                sweep,
                reversed,
            }
        })
        .collect()
}

/// The texels of placed sweeps: one per `block` x `block` dots from cell `(min_column,
/// min_row)`, `width` x `height` of them, RGBA in the Engrave colour.
struct Texels {
    min_column: i64,
    min_row: i64,
    block: usize,
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

/// The texels of `placed`, `None` for a grid with no cell.
fn texels(placed: &[Placed<'_>]) -> Option<Texels> {
    let len = |p: &Placed<'_>| i64::try_from(p.sweep.levels.len()).unwrap_or(0);
    let (min_row, max_row) = placed.iter().fold((i64::MAX, i64::MIN), |(lo, hi), p| {
        (lo.min(p.row), hi.max(p.row))
    });
    let (min_column, max_column) = placed.iter().fold((i64::MAX, i64::MIN), |(lo, hi), p| {
        (lo.min(p.column), hi.max(p.column + len(p)))
    });
    let rows = usize::try_from(max_row - min_row + 1).ok()?;
    let columns = usize::try_from(max_column - min_column).ok()?.max(1);
    let mut block = 1_usize;
    while rows.max(columns).div_ceil(block) > MAX_SIDE {
        block *= 2;
    }
    let (width, height) = (columns.div_ceil(block), rows.div_ceil(block));
    let mut coverage = vec![0_u32; width * height];
    for p in placed {
        let row = usize::try_from(p.row - min_row).unwrap_or(0) / block;
        let first = usize::try_from(p.column - min_column).unwrap_or(0);
        let n = p.sweep.levels.len();
        for i in 0..n {
            let level = p.sweep.levels.get(if p.reversed { n - 1 - i } else { i });
            let alpha = dot_alpha(&p.sweep.levels, level);
            if alpha > 0 {
                coverage[row * width + (first + i) / block] += alpha;
            }
        }
    }
    let area = u32::try_from(block * block).unwrap_or(u32::MAX);
    let colour = OP_ENGRAVE_HATCH;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for sum in coverage {
        let alpha = u8::try_from((sum / area).min(255)).unwrap_or(u8::MAX);
        pixels.extend_from_slice(&[colour.r, colour.g, colour.b, alpha]);
    }
    Some(Texels {
        min_column,
        min_row,
        block,
        width,
        height,
        pixels,
    })
}

/// The unit square to page mm for `texels` in `frame`: the top-left at the first row's outer
/// edge.
fn placement(frame: &GridFrame, texels: &Texels) -> Affine {
    #[allow(clippy::cast_precision_loss)]
    let (corner_u, corner_v) = (texels.min_column as f64, texels.min_row as f64 - 0.5);
    let to_mm = |cells: f64| cells * frame.interval / 1000.0;
    let corner = Point::new(
        (frame.origin.0 + (corner_u * frame.along.0 + corner_v * frame.across.0) * frame.interval)
            / 1000.0,
        (frame.origin.1 + (corner_u * frame.along.1 + corner_v * frame.across.1) * frame.interval)
            / 1000.0,
    );
    #[allow(clippy::cast_precision_loss)]
    let (span_u, span_v) = (
        to_mm((texels.width * texels.block) as f64),
        to_mm((texels.height * texels.block) as f64),
    );
    Affine::new(
        frame.along.0 * span_u,
        frame.along.1 * span_u,
        frame.across.0 * span_v,
        frame.across.1 * span_v,
        corner.x,
        corner.y,
    )
}

/// The dot picture of `sweeps` (one item's, in run order; passes not repeated) named `asset`,
/// `None` without a dot.
pub(in crate::session) fn picture_of(
    (node, stacking): (NodeId, u32),
    sweeps: Vec<Arc<Sweep>>,
    asset: AssetHash,
) -> Option<RasterPicture> {
    let frame = GridFrame::of(&sweeps)?;
    let texels = texels(&placed(&frame, &sweeps))?;
    let placement = placement(&frame, &texels);
    let (w, h) = (
        u32::try_from(texels.width).ok()?,
        u32::try_from(texels.height).ok()?,
    );
    Some(RasterPicture {
        node,
        stacking,
        asset,
        image: Arc::new(WorkingImage {
            width: w,
            height: h,
            pixels: texels.pixels,
            original_size: (w, h),
            truncated: false,
            animated: false,
        }),
        placement,
        sweeps,
    })
}

#[cfg(test)]
mod tests {
    use curvyo_laser_core::job::PointUm;

    use super::*;

    fn sweep(start: (i64, i64), end: (i64, i64), levels: &[u8], grey: bool) -> Arc<Sweep> {
        Arc::new(Sweep {
            start: PointUm::new(start.0, start.1),
            end: PointUm::new(end.0, end.1),
            levels: Levels::new(levels, grey),
        })
    }

    fn node() -> NodeId {
        curvyo_document_core::Document::new(1).create_rect(
            curvyo_document_core::RectBounds::from_corners(
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0),
            ),
        )
    }

    #[test]
    fn bidirectional_rows_land_in_their_columns_and_the_picture_covers_the_dots() {
        // Two rows of four dots at 0.1 mm, the second run backwards, its last dot off.
        let sweeps = vec![
            sweep((0, 50), (400, 50), &[255, 0, 255, 255], false),
            sweep((400, 150), (0, 150), &[255, 255, 255, 0], false),
        ];
        let picture = picture_of((node(), 0), sweeps, AssetHash::of(b"t")).unwrap();
        assert_eq!((picture.image.width, picture.image.height), (4, 2));
        let alpha: Vec<u8> = picture.image.pixels.chunks(4).map(|p| p[3]).collect();
        assert_eq!(
            alpha,
            [255, 0, 255, 255, 0, 255, 255, 255],
            "row 2 read in columns"
        );
        let corners =
            [(0.0, 0.0), (1.0, 1.0)].map(|(x, y)| picture.placement.apply_point(Point::new(x, y)));
        assert!(
            (corners[0].x - 0.0).abs() < 1e-9 && (corners[0].y - 0.0).abs() < 1e-9,
            "{corners:?}"
        );
        assert!(
            (corners[1].x - 0.4).abs() < 1e-9 && (corners[1].y - 0.2).abs() < 1e-9,
            "{corners:?}"
        );
    }

    #[test]
    fn grey_dots_take_twenty_to_a_hundred_percent_and_an_unchanged_plan_is_recognised() {
        let sweeps = vec![sweep((0, 50), (300, 50), &[1, 128, 255], true)];
        let picture = picture_of((node(), 0), sweeps.clone(), AssetHash::of(b"g")).unwrap();
        let alpha: Vec<u8> = picture.image.pixels.chunks(4).map(|p| p[3]).collect();
        assert_eq!(alpha, [51, 153, 255]);
        assert!(picture.drawn_from(&sweeps));
        let other = vec![sweep((0, 50), (300, 50), &[1, 128, 255], true)];
        assert!(!picture.drawn_from(&other), "another plan's sweeps");
    }

    /// A sweep of `dots` dots of 0.1 mm on row `row` from column `column` at `angle` degrees.
    fn angled(angle: f64, row: f64, column: f64, dots: usize) -> Arc<Sweep> {
        let (sin, cos) = angle.to_radians().sin_cos();
        #[allow(clippy::cast_possible_truncation)]
        let at = |u: f64| {
            let (x, y) = (
                u * 100.0 * cos - row * 100.0 * sin,
                u * 100.0 * sin + row * 100.0 * cos,
            );
            (x.round() as i64, y.round() as i64)
        };
        #[allow(clippy::cast_precision_loss)]
        let end = at(column + dots as f64);
        sweep(at(column), end, &vec![255; dots], false)
    }

    #[test]
    fn a_one_dot_first_sweep_at_a_scan_angle_keeps_the_rows_on_the_angle() {
        let sweeps = vec![
            angled(30.0, 0.5, 3.0, 1),
            angled(30.0, 1.5, 0.0, 40),
            angled(30.0, 2.5, 0.0, 40),
        ];
        let picture = picture_of((node(), 0), sweeps, AssetHash::of(b"a")).unwrap();
        let o = picture.placement.apply_point(Point::new(0.0, 0.0));
        let u = picture.placement.apply_point(Point::new(1.0, 0.0));
        let angle = (u.y - o.y).atan2(u.x - o.x).to_degrees();
        assert!(
            (angle - 30.0).abs() < 0.05,
            "the rows run at {angle} degrees"
        );
        assert_eq!((picture.image.width, picture.image.height), (40, 3));
        let on = picture.image.pixels.chunks(4).filter(|p| p[3] > 0).count();
        assert_eq!(on, 81, "every dot in its own cell");
    }
}
