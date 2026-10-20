//! One parsed face: its shaper, its line and cap-height metrics and its glyph outline cache.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use harfrust::font::Kind;
use skrifa::{FontRef, MetadataProvider};

use crate::error::TextError;
use crate::outline::{GlyphOutline, draw_glyph};

/// A face's metrics in em units (1.0 = the em square), y up as fonts state them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMetrics {
    /// Top of the line above the baseline.
    pub ascender: f64,
    /// Bottom of the line, below the baseline (negative).
    pub descender: f64,
    /// Extra space between one line's descender and the next line's ascender.
    pub line_gap: f64,
    /// The height of the capital H: its measured outline top, else OS/2 `sCapHeight`, else 0.7
    /// (spec "Terms", Size).
    pub cap_height: f64,
}

impl FaceMetrics {
    /// The distance from one baseline to the next (spec criterion 4).
    #[must_use]
    pub fn line_height(&self) -> f64 {
        self.ascender - self.descender + self.line_gap
    }
}

/// The share of the em a face without an H and without `sCapHeight` uses as its cap height.
const FALLBACK_CAP_HEIGHT: f64 = 0.7;

/// The cap heights (em) accepted from a face; a hostile H outside them would scale the text to
/// millions of millimetres (architect review m2, should-fix 7).
const CAP_HEIGHT_RANGE: std::ops::RangeInclusive<f64> = 0.05..=2.0;

/// One face of a font file, parsed once; used at its default instance (criterion 21).
pub struct Face {
    bytes: Arc<Vec<u8>>,
    index: u32,
    shaper: harfrust::Font,
    units_per_em: f64,
    metrics: FaceMetrics,
    outlines: Mutex<BTreeMap<u32, Arc<GlyphOutline>>>,
}

impl std::fmt::Debug for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Face")
            .field("index", &self.index)
            .field("bytes", &self.bytes.len())
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

impl Face {
    /// Parses face `index` of `bytes`. Only TrueType and OpenType faces with TrueType or CFF
    /// outlines are accepted.
    ///
    /// # Errors
    /// [`TextError::Unreadable`] when the bytes hold no such face.
    pub(crate) fn new(bytes: Arc<Vec<u8>>, index: u32) -> Result<Self, TextError> {
        let unreadable = || TextError::Unreadable(index);
        let font_ref = FontRef::from_index(&bytes, index).map_err(|_| unreadable())?;
        font_ref.outline_glyphs().format().ok_or_else(unreadable)?;
        let blob: Arc<dyn AsRef<[u8]> + Send + Sync> = bytes.clone();
        let shaper = harfrust::Font::new(blob, index).ok_or_else(unreadable)?;
        if !matches!(shaper.kind(), Kind::Sfnt(..)) {
            return Err(unreadable());
        }
        let units_per_em = f64::from(shaper.units_per_em());
        if !(16.0..=16_384.0).contains(&units_per_em) {
            return Err(unreadable());
        }
        let mut face = Self {
            bytes,
            index,
            shaper,
            units_per_em,
            metrics: FaceMetrics {
                ascender: 0.8,
                descender: -0.2,
                line_gap: 0.0,
                cap_height: FALLBACK_CAP_HEIGHT,
            },
            outlines: Mutex::new(BTreeMap::new()),
        };
        face.metrics = face.read_metrics();
        Ok(face)
    }

    fn read_metrics(&self) -> FaceMetrics {
        let upem = self.units_per_em;
        let stated = self.shaper.metrics();
        let mut metrics = self.metrics;
        if let Some(line) = stated.h_line() {
            let ascender = line.ascender.to_f64() / upem;
            let descender = line.descender.to_f64() / upem;
            let line_gap = line.line_gap.to_f64().max(0.0) / upem;
            if ascender.is_finite() && descender.is_finite() && ascender - descender > 0.0 {
                metrics.ascender = ascender;
                metrics.descender = descender;
                metrics.line_gap = line_gap;
            }
        }
        let measured = self
            .shaper
            .charmap()
            .map_unicode('H')
            .map(|glyph| self.outline(glyph.to_u32()))
            .and_then(|outline| outline.top());
        let os2 = stated.cap_height.map(|cap| cap.to_f64() / upem);
        metrics.cap_height = [measured, os2]
            .into_iter()
            .flatten()
            .find(|cap| CAP_HEIGHT_RANGE.contains(cap))
            .unwrap_or(FALLBACK_CAP_HEIGHT);
        metrics
    }

    /// The face's metrics in em units.
    #[must_use]
    pub const fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    /// The index of this face in its file.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// The face's design units per em.
    #[must_use]
    pub const fn units_per_em(&self) -> f64 {
        self.units_per_em
    }

    pub(crate) const fn shaper(&self) -> &harfrust::Font {
        &self.shaper
    }

    /// The outline of glyph `glyph` in em units, y down, origin on the baseline; cached per face.
    /// A glyph that cannot be drawn, or has more than [`crate::limits::MAX_GLYPH_POINTS`] points,
    /// is empty.
    #[must_use]
    pub fn outline(&self, glyph: u32) -> Arc<GlyphOutline> {
        let mut cache = self.outlines.lock().unwrap_or_else(PoisonError::into_inner);
        cache
            .entry(glyph)
            .or_insert_with(|| {
                let drawn = FontRef::from_index(&self.bytes, self.index)
                    .ok()
                    .and_then(|font| draw_glyph(&font, glyph, self.units_per_em));
                Arc::new(drawn.unwrap_or_default())
            })
            .clone()
    }
}
