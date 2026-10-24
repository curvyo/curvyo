//! One parsed face: its source (an OpenType shaper or an SVG font), its line and cap-height
//! metrics and its glyph outlines.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use harfrust::font::Kind;
use skrifa::{FontRef, MetadataProvider};

use crate::error::TextError;
use crate::glyph_list::ListedChar;
use crate::outline::{GlyphOutline, draw_glyph};
use crate::svg_font::{SvgFont, parse_svg_font};

/// How a face's glyphs are drawn (`specs/0076-single-line-fonts`, "Terms").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceKind {
    /// Closed areas (TrueType, OpenType).
    Outline,
    /// Open centre lines (SVG fonts): drawn with the stroke, never filled.
    SingleLine,
}

/// Where a face's glyphs come from: two kinds exist, so an enum (`adrs.md` decision 3).
pub(crate) enum Source {
    /// A TrueType or OpenType face, shaped by `harfrust`, outlines by `skrifa` on demand.
    OpenType {
        bytes: Arc<Vec<u8>>,
        shaper: Box<harfrust::Font>,
        outlines: Mutex<BTreeMap<u32, Arc<GlyphOutline>>>,
    },
    /// An SVG font, every glyph parsed when the face is added.
    Svg(SvgFont),
}

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
    source: Source,
    index: u32,
    units_per_em: f64,
    metrics: FaceMetrics,
    /// The characters the face lists, built on first use (`glyph_list.rs`).
    pub(crate) listed: OnceLock<Arc<[ListedChar]>>,
}

impl std::fmt::Debug for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Face")
            .field("index", &self.index)
            .field("kind", &self.kind())
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

const DEFAULT_METRICS: FaceMetrics = FaceMetrics {
    ascender: 0.8,
    descender: -0.2,
    line_gap: 0.0,
    cap_height: FALLBACK_CAP_HEIGHT,
};

impl Face {
    /// Parses face `index` of `bytes`: TrueType and OpenType faces with TrueType or CFF outlines,
    /// or `<font>` number `index` of an SVG font (bytes starting with `<`).
    ///
    /// # Errors
    /// [`TextError::Unreadable`] when the bytes hold no such face.
    pub(crate) fn new(bytes: Arc<Vec<u8>>, index: u32) -> Result<Self, TextError> {
        if crate::svg_font::looks_like_xml(&bytes) {
            return Self::new_svg(&bytes, index);
        }
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
            source: Source::OpenType {
                bytes,
                shaper: Box::new(shaper),
                outlines: Mutex::new(BTreeMap::new()),
            },
            index,
            units_per_em,
            metrics: DEFAULT_METRICS,
            listed: OnceLock::new(),
        };
        face.metrics = face.read_metrics();
        Ok(face)
    }

    fn new_svg(bytes: &[u8], index: u32) -> Result<Self, TextError> {
        let font = parse_svg_font(bytes, index).ok_or(TextError::Unreadable(index))?;
        let upem = font.units_per_em;
        let mut metrics = DEFAULT_METRICS;
        if let (Some(ascent), Some(descent)) = (font.ascent, font.descent)
            && ascent + descent > 0.0
        {
            metrics.ascender = ascent / upem;
            metrics.descender = -descent / upem;
        }
        let measured = font
            .glyphs
            .get(usize::try_from(font.glyph_of('H')).unwrap_or(0))
            .filter(|_| font.glyph_of('H') != 0)
            .and_then(|(_, outline)| outline.top());
        let stated = font.cap_height.map(|cap| cap / upem);
        metrics.cap_height = [measured, stated]
            .into_iter()
            .flatten()
            .find(|cap| CAP_HEIGHT_RANGE.contains(cap))
            .unwrap_or(FALLBACK_CAP_HEIGHT);
        Ok(Self {
            source: Source::Svg(font),
            index,
            units_per_em: upem,
            metrics,
            listed: OnceLock::new(),
        })
    }

    /// Whether the face draws closed areas or open centre lines.
    #[must_use]
    pub const fn kind(&self) -> FaceKind {
        match self.source {
            Source::OpenType { .. } => FaceKind::Outline,
            Source::Svg(_) => FaceKind::SingleLine,
        }
    }

    pub(crate) const fn source(&self) -> &Source {
        &self.source
    }

    fn read_metrics(&self) -> FaceMetrics {
        let Source::OpenType { shaper, .. } = &self.source else {
            return self.metrics;
        };
        let upem = self.units_per_em;
        let stated = shaper.metrics();
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
        let measured = shaper
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

    /// The distinct characters of `text` the face has no glyph for (glyph 0: `.notdef`, or an SVG
    /// font's `<missing-glyph>`), in first-seen order; whitespace and control characters are left
    /// out (`specs/0077-font-preview` criterion 18, `adrs.md` decision 3).
    #[must_use]
    pub fn missing_chars(&self, text: &str) -> Vec<char> {
        let mut missing = Vec::new();
        for c in text.chars() {
            if c.is_whitespace() || c.is_control() || missing.contains(&c) {
                continue;
            }
            if self.glyph_for(c) == 0 {
                missing.push(c);
            }
        }
        missing
    }

    /// The glyph the face draws `c` with, 0 when it has none (`.notdef`, or an SVG font's
    /// `<missing-glyph>`): the one presence check of the missing-glyph marker and the glyph table
    /// (`specs/0082-glyph-table` criterion 9, `adrs.md` decision 1).
    #[must_use]
    pub fn glyph_for(&self, c: char) -> u32 {
        match &self.source {
            Source::OpenType { shaper, .. } => shaper
                .charmap()
                .map_unicode(c)
                .map_or(0, harfrust::GlyphId::to_u32),
            Source::Svg(font) => font.glyph_of(c),
        }
    }

    /// The bytes of an OpenType face; `None` for an SVG font.
    pub(crate) fn open_type_bytes(&self) -> Option<&[u8]> {
        match &self.source {
            Source::OpenType { bytes, .. } => Some(bytes),
            Source::Svg(_) => None,
        }
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

    /// The outline of glyph `glyph` in em units, y down, origin on the baseline; cached per face.
    /// A glyph that cannot be drawn, or has more than [`crate::limits::MAX_GLYPH_POINTS`] points,
    /// is empty.
    #[must_use]
    pub fn outline(&self, glyph: u32) -> Arc<GlyphOutline> {
        match &self.source {
            Source::OpenType {
                bytes, outlines, ..
            } => {
                let mut cache = outlines.lock().unwrap_or_else(PoisonError::into_inner);
                cache
                    .entry(glyph)
                    .or_insert_with(|| {
                        let drawn = FontRef::from_index(bytes, self.index)
                            .ok()
                            .and_then(|font| draw_glyph(&font, glyph, self.units_per_em));
                        Arc::new(drawn.unwrap_or_default())
                    })
                    .clone()
            }
            Source::Svg(font) => usize::try_from(glyph)
                .ok()
                .and_then(|at| font.glyphs.get(at))
                .map_or_else(Arc::default, |(_, outline)| outline.clone()),
        }
    }
}
