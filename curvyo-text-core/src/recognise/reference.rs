//! Prepares a face's reference glyphs once per session: outline points in em, box, holes and
//! descriptor of every character of the recognised set (`adrs.md` decision 5).

use std::sync::Arc;

use kurbo::{Point, Rect};

use super::blobs::{Loop, bounds, nest};
use super::descriptor::Descriptor;
use super::sample::{Ring, sample_closed};
use crate::face::{Face, FaceKind};

/// The spacing of reference points, in em (about 0.1 mm at 24 mm cap height).
const EM_SPACING: f64 = 0.004;
/// The most points one reference glyph may have.
const MAX_GLYPH_POINTS: usize = 20_000;

/// One character of a face, ready to compare.
#[derive(Debug, Clone)]
pub(crate) struct ReferenceGlyph {
    /// The character.
    pub c: char,
    /// The closed rings in em, y down, origin on the baseline.
    pub rings: Vec<Ring>,
    /// The box of the rings, em.
    pub bbox: Rect,
    /// The shortlist descriptor.
    pub descriptor: Descriptor,
}

/// The reference glyphs of one face, in character order; empty for a single-line face.
#[derive(Debug, Default)]
pub(crate) struct ReferenceGlyphs {
    pub glyphs: Vec<ReferenceGlyph>,
}

/// The characters recognised: printable ASCII and the Latin-1 letters and punctuation
/// (`adrs.md` decision 5), without the soft hyphen.
pub(crate) fn charset() -> impl Iterator<Item = char> {
    ('\u{21}'..='\u{7e}').chain(('\u{a1}'..='\u{ff}').filter(|c| *c != '\u{ad}'))
}

impl ReferenceGlyphs {
    fn build(face: &Face) -> Self {
        if face.kind() != FaceKind::Outline {
            return Self::default();
        }
        let mut glyphs = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        for c in charset() {
            let glyph = face.glyph_for(c);
            // A glyph drawn for two characters is kept for the first only: the margin between
            // two equal outlines would make both unsure.
            if glyph == 0 || seen.contains(&glyph) {
                continue;
            }
            seen.push(glyph);
            let outline = face.outline(glyph);
            let Some(rings) = sample_closed(&outline.path, EM_SPACING, MAX_GLYPH_POINTS) else {
                continue;
            };
            let points: Vec<Point> = rings.iter().flatten().copied().collect();
            let bbox = bounds(&points);
            if bbox.width() <= 0.0 && bbox.height() <= 0.0 {
                continue;
            }
            let loops: Vec<Loop> = rings.iter().map(|r| Loop::new(r.clone(), 0)).collect();
            let holes = nest(&loops).iter().map(super::blobs::Blob::holes).sum();
            // The descriptor needs no fine outline: every fourth point (at least a triangle).
            let thinned: Vec<Ring> = rings
                .iter()
                .map(|r| {
                    r.iter()
                        .step_by(4.min((r.len() / 3).max(1)))
                        .copied()
                        .collect()
                })
                .collect();
            let refs: Vec<&[Point]> = thinned.iter().map(Vec::as_slice).collect();
            let descriptor = Descriptor::of(&refs, bbox, holes);
            glyphs.push(ReferenceGlyph {
                c,
                rings,
                bbox,
                descriptor,
            });
        }
        Self { glyphs }
    }
}

impl Face {
    /// The face's reference glyphs, built on first use and kept with the face for the session.
    pub(crate) fn reference_glyphs(&self) -> Arc<ReferenceGlyphs> {
        self.recognise
            .get_or_init(|| Arc::new(ReferenceGlyphs::build(self)))
            .clone()
    }

    /// Whether the face's recognition references (outlines and descriptors of its characters)
    /// are built.
    #[must_use]
    pub fn recognition_ready(&self) -> bool {
        self.recognise.get().is_some()
    }

    /// Builds the face's recognition references now, so a caller can build a cold tier one face
    /// per step and stay responsive between steps.
    pub fn prepare_recognition(&self) {
        let _ = self.reference_glyphs();
    }
}
