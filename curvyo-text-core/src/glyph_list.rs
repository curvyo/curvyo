//! Lists the characters a face draws, for the glyph table (`specs/0082-glyph-table` criteria 5,
//! 7, 9 and 23; `adrs.md` decision 1).
//!
//! The candidates are a superset (every `cmap` subtable, the shaper's own charmap with its Mac
//! Roman and symbol rules, the symbol aliases `c - 0xF000`, an SVG font's single-character
//! glyphs); a candidate is listed exactly when [`Face::glyph_for`] is not 0 and it is not a
//! control character, so the list and the missing-glyph check agree by construction.

use std::sync::Arc;

use skrifa::raw::TableProvider;
use skrifa::raw::tables::cmap::CmapIterLimits;
use skrifa::{FontRef, GlyphId, GlyphNames};

use crate::face::{Face, Source};

/// One character of a face's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedChar {
    /// The character.
    pub c: char,
    /// The face's glyph for it (never 0).
    pub glyph: u32,
    /// The glyph's name in the face (`post`, the CFF charset, an SVG `glyph-name`); `None` when
    /// the face names it not or only by a made-up name.
    pub name: Option<Box<str>>,
}

/// The number of Unicode code points.
const CODE_POINTS: usize = 0x11_0000;

/// A set of code points.
struct Candidates(Vec<u64>);

impl Candidates {
    fn new() -> Self {
        Self(vec![0; CODE_POINTS / 64])
    }

    fn add(&mut self, code: u32) {
        if let Ok(at) = usize::try_from(code)
            && at < CODE_POINTS
        {
            self.0[at / 64] |= 1 << (at % 64);
        }
    }

    fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().enumerate().flat_map(|(word, bits)| {
            (0..64_u32)
                .filter(move |bit| bits & (1 << bit) != 0)
                .map(move |bit| u32::try_from(word).unwrap_or(0) * 64 + bit)
        })
    }
}

impl Face {
    /// The characters the face draws, in code point order: built once per face, then shared.
    #[must_use]
    pub fn listed_chars(&self) -> Arc<[ListedChar]> {
        self.listed.get_or_init(|| self.build_list()).clone()
    }

    fn build_list(&self) -> Arc<[ListedChar]> {
        let font = self
            .open_type_bytes()
            .and_then(|bytes| FontRef::from_index(bytes, self.index()).ok());
        let mut candidates = Candidates::new();
        if let Some(font) = &font {
            open_type_candidates(font, &mut candidates);
        }
        match self.source() {
            // The shaper's own charmap: Mac Roman codes as Unicode, the symbol fallback.
            Source::OpenType { shaper, .. } => {
                for (code, _) in shaper.charmap().iter_unicodes() {
                    candidates.add(code);
                }
            }
            Source::Svg(svg) => {
                for c in svg.chars() {
                    candidates.add(u32::from(c));
                }
            }
        }
        let names = font.as_ref().map(GlyphNames::new);
        candidates
            .iter()
            .filter_map(char::from_u32)
            .filter(|c| !c.is_control())
            .filter_map(|c| {
                let glyph = self.glyph_for(c);
                (glyph != 0).then(|| ListedChar {
                    c,
                    glyph,
                    name: self.glyph_name(names.as_ref(), glyph),
                })
            })
            .collect()
    }

    fn glyph_name(&self, names: Option<&GlyphNames<'_>>, glyph: u32) -> Option<Box<str>> {
        match (names, self.source()) {
            (Some(names), _) => names
                .get(GlyphId::new(glyph))
                .filter(|name| !name.is_synthesized() && !name.as_str().is_empty())
                .map(|name| Box::from(name.as_str())),
            (None, Source::Svg(svg)) => svg.glyph_name(glyph).map(Box::from),
            (None, Source::OpenType { .. }) => None,
        }
    }
}

/// Every code an OpenType face's character maps can reach.
fn open_type_candidates(font: &FontRef<'_>, candidates: &mut Candidates) {
    let limits = CmapIterLimits {
        max_char: 0x10_FFFF,
        glyph_count: u32::MAX,
    };
    if let Ok(cmap) = font.cmap() {
        for record in cmap.encoding_records() {
            let Ok(subtable) = record.subtable(cmap.offset_data()) else {
                continue;
            };
            for (code, _) in subtable.iter_with_limits(limits) {
                candidates.add(code);
            }
            for code in 0..=0xFF_u32 {
                if subtable.map_codepoint(code).is_some() {
                    candidates.add(code);
                }
            }
        }
    }
    // Symbol fonts: a code in U+F000..U+F0FF is also reached by its low byte.
    let symbol: Vec<u32> = candidates
        .iter()
        .filter(|code| (0xF000..=0xF0FF).contains(code))
        .collect();
    for code in symbol {
        candidates.add(code - 0xF000);
    }
}
