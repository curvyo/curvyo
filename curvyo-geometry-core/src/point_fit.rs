//! Fits a similarity map to matched point pairs by least squares (ADR 0081 §6).
//!
//! The closed form is Umeyama's without reflection: with the centroids removed, `d = Σ|P|²`,
//! `a = Σ P·Q / d`, `b = Σ P×Q / d`, the linear part is `[a −b; b a]` and the translation
//! `q̄ − M p̄`. The matrix is built from `+ − × ÷` only, summed in the pairs' order, so it is
//! bit-identical on every target; `scale` and `angle` (through `libm`) are for display only.

use curvyo_document_core::{Affine, Angle, Length, Point};
use thiserror::Error;

/// One matched pair: where a point lies in the source frame and where it was measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointPair {
    /// The point in the source frame (a mark on the sheet).
    pub from: Point,
    /// The point it should land on (the measured position).
    pub to: Point,
}

/// The least-squares similarity of a set of pairs, with how well it fits.
#[derive(Debug, Clone, PartialEq)]
pub struct SimilarityFit {
    /// The map from the source frame to the measured frame: move, turn and one uniform scale.
    pub map: Affine,
    /// The uniform scale of the map (a ratio, 1 is the same size).
    pub scale: f64,
    /// The turn of the map, counter-clockwise positive in a Y-up frame (`atan2(b, a)`).
    pub angle: Angle,
    /// Per pair, in the input order: the distance from the mapped source point to its measure.
    pub residuals: Vec<Length>,
    /// The largest residual.
    pub largest: Length,
    /// The root mean square of the residuals.
    pub rms: Length,
    /// For 3 or more pairs, the RMS residual of the best fit *with* a reflection; a mirrored
    /// measurement fits that one clearly better. `None` for 2 pairs (both fit exactly).
    pub mirrored_rms: Option<Length>,
}

/// Why no fit exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FitError {
    /// Fewer than two pairs.
    #[error("a similarity fit needs at least two point pairs")]
    TooFewPairs,
    /// No two source points, or no two measured points, lie at least the minimum spread apart.
    #[error("the points are too close together to fit")]
    TooClose,
}

/// The least-squares similarity map from each pair's `from` to its `to` (ADR 0081 §6).
///
/// Two pairs fit exactly. `min_spread` is the explicit tolerance below which two points count
/// as one place: the fit is refused unless some two source points and some two measured points
/// are at least that far apart.
///
/// # Errors
/// [`FitError::TooFewPairs`] for fewer than two pairs; [`FitError::TooClose`] as above.
pub fn fit_similarity(pairs: &[PointPair], min_spread: Length) -> Result<SimilarityFit, FitError> {
    if pairs.len() < 2 {
        return Err(FitError::TooFewPairs);
    }
    let spread = min_spread.as_mm();
    if !spread_reaches(pairs.iter().map(|p| p.from), spread)
        || !spread_reaches(pairs.iter().map(|p| p.to), spread)
    {
        return Err(FitError::TooClose);
    }
    let sums = Sums::of(pairs);
    let (a, b) = (sums.dot / sums.d, sums.cross / sums.d);
    let map = with_translation(a, b, -b, a, sums.from_mean, sums.to_mean);
    let residuals: Vec<Length> = pairs.iter().map(|p| residual(map, p)).collect();
    let mirrored_rms = (pairs.len() >= 3).then(|| {
        let (ma, mb) = (sums.mirror_a / sums.d, sums.mirror_b / sums.d);
        let mirror = with_translation(ma, mb, mb, -ma, sums.from_mean, sums.to_mean);
        rms_of(
            &pairs
                .iter()
                .map(|p| residual(mirror, p))
                .collect::<Vec<_>>(),
        )
    });
    let largest = residuals.iter().map(|r| r.as_mm()).fold(0.0_f64, f64::max);
    Ok(SimilarityFit {
        map,
        scale: libm::hypot(a, b),
        angle: Angle::from_radians(libm::atan2(b, a)),
        rms: rms_of(&residuals),
        largest: Length::from_mm(largest),
        residuals,
        mirrored_rms,
    })
}

/// The centroid-free sums of the pairs, each summed in input order.
struct Sums {
    from_mean: Point,
    to_mean: Point,
    /// `Σ |P|²`.
    d: f64,
    /// `Σ P·Q`.
    dot: f64,
    /// `Σ P×Q`.
    cross: f64,
    /// `Σ (Px Qx − Py Qy)`: the reflected fit's first entry.
    mirror_a: f64,
    /// `Σ (Py Qx + Px Qy)`: the reflected fit's second entry.
    mirror_b: f64,
}

impl Sums {
    fn of(pairs: &[PointPair]) -> Self {
        let from_mean = mean(pairs.iter().map(|p| p.from));
        let to_mean = mean(pairs.iter().map(|p| p.to));
        let mut sums = Self {
            from_mean,
            to_mean,
            d: 0.0,
            dot: 0.0,
            cross: 0.0,
            mirror_a: 0.0,
            mirror_b: 0.0,
        };
        for pair in pairs {
            let (px, py) = (pair.from.x - from_mean.x, pair.from.y - from_mean.y);
            let (qx, qy) = (pair.to.x - to_mean.x, pair.to.y - to_mean.y);
            sums.d = sums.d + px * px + py * py;
            sums.dot = sums.dot + px * qx + py * qy;
            sums.cross = sums.cross + px * qy - py * qx;
            sums.mirror_a = sums.mirror_a + px * qx - py * qy;
            sums.mirror_b = sums.mirror_b + py * qx + px * qy;
        }
        sums
    }
}

/// The map with linear part `[xx xy; yx yy]` that takes `from_mean` onto `to_mean`.
fn with_translation(
    xx: f64,
    yx: f64,
    xy: f64,
    yy: f64,
    from_mean: Point,
    to_mean: Point,
) -> Affine {
    let tx = to_mean.x - (xx * from_mean.x + xy * from_mean.y);
    let ty = to_mean.y - (yx * from_mean.x + yy * from_mean.y);
    Affine::new(xx, yx, xy, yy, tx, ty)
}

/// The distance from `pair.from` under `map` to `pair.to`, with `+ − × ÷` and one square root.
fn residual(map: Affine, pair: &PointPair) -> Length {
    let x = map.a * pair.from.x + map.c * pair.from.y + map.e - pair.to.x;
    let y = map.b * pair.from.x + map.d * pair.from.y + map.f - pair.to.y;
    Length::from_mm((x * x + y * y).sqrt())
}

fn rms_of(residuals: &[Length]) -> Length {
    let sum = residuals
        .iter()
        .fold(0.0, |sum, r| sum + r.as_mm() * r.as_mm());
    #[allow(clippy::cast_precision_loss)]
    let n = residuals.len() as f64;
    Length::from_mm((sum / n).sqrt())
}

fn mean(points: impl Iterator<Item = Point>) -> Point {
    let (mut x, mut y, mut n) = (0.0, 0.0, 0.0);
    for p in points {
        x += p.x;
        y += p.y;
        n += 1.0;
    }
    Point::new(x / n, y / n)
}

/// Whether some two of `points` lie at least `spread` mm apart.
fn spread_reaches(points: impl Iterator<Item = Point> + Clone, spread: f64) -> bool {
    let all: Vec<Point> = points.collect();
    all.iter().enumerate().any(|(i, p)| {
        all[i + 1..].iter().any(|q| {
            let (dx, dy) = (p.x - q.x, p.y - q.y);
            dx * dx + dy * dy >= spread * spread
        })
    })
}

#[cfg(test)]
#[path = "point_fit_tests.rs"]
mod tests;
