//! Measures how far a line's letters lean (`specs/0091-trace-text-recognition` criterion 29:
//! italic for the fallback face only when the lean is clear).
//!
//! The lean is the length-weighted median angle of the outlines' near-vertical stretches (stems
//! and the steep sides of round letters), measured over chords of a tenth of the ink height so
//! the sampling's small steps do not count; upright letters give 0, an italic its own angle.

use kurbo::Vec2;

use super::Recogniser;

/// A stretch counts as a stem side when it is within this many degrees of vertical.
const STEM_DEGREES: f64 = 30.0;
/// The chord over which a stretch's direction is taken, as a share of the ink height.
const CHORD_SHARE: f64 = 0.1;

impl Recogniser {
    /// How far the line's letters lean along `direction`, degrees: positive leans right (as
    /// italics), 0 without outlines.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the frame u, n and its coordinates
    pub fn slant(&self, direction: Vec2) -> f64 {
        let length = direction.hypot();
        if self.loops.is_empty() || length <= 0.0 {
            return 0.0;
        }
        let u = direction / length;
        let n = Vec2::new(-u.y, u.x);
        let (mut low, mut high) = (f64::MAX, f64::MIN);
        for p in self.loops.iter().flat_map(|l| l.points.iter()) {
            let b = p.to_vec2().dot(n);
            low = low.min(b);
            high = high.max(b);
        }
        let chord = CHORD_SHARE * (high - low);
        if chord <= 0.0 {
            return 0.0;
        }
        let steep = STEM_DEGREES.to_radians().tan();
        // (lean in degrees, length) of every near-vertical chord.
        let mut leans: Vec<(f64, f64)> = Vec::new();
        for ring in self.loops.iter().map(|l| &l.points) {
            let count = ring.len();
            if count < 3 {
                continue;
            }
            let mut i = 0;
            while i < count {
                // The next point a chord's length away along the ring.
                let start = ring[i];
                let mut j = i + 1;
                while j < i + count && (ring[j % count] - start).hypot() < chord {
                    j += 1;
                }
                let d = ring[j % count] - start;
                let (da, db) = (d.dot(u), d.dot(n));
                if db.abs() > 0.0 && da.abs() <= steep * db.abs() {
                    // y runs down: a stretch whose top lies right of its bottom leans right.
                    leans.push(((-da / db).atan().to_degrees(), d.hypot()));
                }
                i = j.max(i + 1);
            }
        }
        if leans.is_empty() {
            return 0.0;
        }
        leans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let half = 0.5 * leans.iter().map(|l| l.1).sum::<f64>();
        let mut seen = 0.0;
        for (lean, weight) in &leans {
            seen += weight;
            if seen >= half {
                return *lean;
            }
        }
        0.0
    }
}
