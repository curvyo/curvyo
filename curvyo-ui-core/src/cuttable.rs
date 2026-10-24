//! Counts the pieces a test word in one face makes (`specs/0075-font-library` criteria 17 to 19,
//! `adrs.md` decision 5), and the line format of the per-computer cache of those counts.
//!
//! The word is laid out by the engine and its glyphs are joined by the Union kernel of `0016`
//! with no gap, one operand per glyph, so a hole of one letter is never cancelled by another
//! letter; the pieces are the outlines of even depth of the result (an island counts).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{
    BooleanOp, Outline, OutlineTriple, boolean, outline_has_area, outline_nesting,
};
use curvyo_text_core::{FaceHash, FaceStore, LayoutOptions, Mm, TextError, layout};

use crate::boolean::BOOLEAN_TOLERANCE;
use crate::font_library::CheckResult;
use crate::text_layout::contours_of;

/// The test word: lowercase, no dotted letters (a dot is always a second piece).
pub const CUTTABLE_WORD: &str = "cuttable";
/// The size of the test word (cap height), mm.
pub const CUTTABLE_SIZE_MM: f64 = 20.0;
/// Bumped when the word, the size or the kernel change: the cache is then rebuilt. Version 2:
/// Union without a gap replaced the old Weld (2026-10-20).
pub const CUTTABLE_CHECK_VERSION: u32 = 2;

/// The contours of each glyph of [`CUTTABLE_WORD`] in `face` at [`CUTTABLE_SIZE_MM`], in text
/// space, one list per glyph.
fn glyph_contours(
    face: &curvyo_text_core::Face,
) -> Result<Vec<Vec<Vec<OutlineTriple>>>, TextError> {
    let laid = layout(
        face,
        CUTTABLE_WORD,
        Mm(CUTTABLE_SIZE_MM),
        &LayoutOptions::default(),
    )?;
    let map = |p: kurbo::Point| Point::new(p.x, p.y);
    let mut next_id = 0;
    Ok(laid
        .glyph_paths(face)
        .iter()
        .map(|glyph| {
            contours_of(glyph, &map, &mut next_id)
                .into_iter()
                .map(|contour| {
                    contour
                        .anchors
                        .iter()
                        .map(|a| (a.point, a.handle_in, a.handle_out))
                        .collect()
                })
                .collect()
        })
        .collect())
}

/// The pieces of the Union of the glyphs `glyphs` (each a list of closed contours): one operand
/// per glyph with area; the outlines of even depth of the result. No glyph with area, or a
/// refusal of the kernel, gives 0.
#[must_use]
pub fn union_pieces(glyphs: &[Vec<Vec<OutlineTriple>>]) -> usize {
    let views: Vec<Vec<Outline<'_>>> = glyphs
        .iter()
        .map(|contours| {
            contours
                .iter()
                .map(|c| Outline::new(c, true))
                .collect::<Vec<_>>()
        })
        .filter(|outlines: &Vec<Outline<'_>>| outlines.iter().any(outline_has_area))
        .collect();
    if views.is_empty() {
        return 0;
    }
    let operands: Vec<&[Outline<'_>]> = views.iter().map(Vec::as_slice).collect();
    let Ok(result) = boolean(BooleanOp::Union, &operands, BOOLEAN_TOLERANCE) else {
        return 0;
    };
    let triples: Vec<Vec<OutlineTriple>> = result
        .outlines()
        .iter()
        .map(|points| {
            points
                .iter()
                .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
                .collect()
        })
        .collect();
    let outlines: Vec<Outline<'_>> = triples.iter().map(|t| Outline::new(t, true)).collect();
    outline_nesting(&outlines)
        .iter()
        .filter(|nesting| nesting.depth % 2 == 0)
        .count()
}

/// The pieces [`CUTTABLE_WORD`] in face `index` of `bytes` makes at [`CUTTABLE_SIZE_MM`]: the
/// Union of its glyphs with no gap ([`union_pieces`]). Glyphs a face lacks draw as `.notdef` or
/// nothing; no area at all is 0 pieces, and a single-line face is 0 without running Union.
///
/// # Errors
/// The engine's error for bytes that hold no usable face.
pub fn cuttable_pieces(bytes: &[u8], index: u32) -> Result<usize, TextError> {
    let mut store = FaceStore::new();
    let key = store.add(bytes.to_vec(), index)?;
    let face = store.get(&key).ok_or(TextError::UnknownFace(key))?;
    // A single-line face has no glyph with area: Union would close every stroke
    // (`specs/0076-single-line-fonts` criterion 5).
    if face.kind() == curvyo_text_core::FaceKind::SingleLine {
        return Ok(0);
    }
    Ok(union_pieces(&glyph_contours(face)?))
}

/// The first line of the cache text.
fn header() -> String {
    format!("curvyo-font-checks 1 {CUTTABLE_CHECK_VERSION}")
}

/// The line that says the system fonts were asked for once.
const SYSTEM_REQUESTED: &str = "system-requested";

/// One cached result.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    hash: FaceHash,
    result: CheckResult,
    stamp: String,
}

/// The per-computer cache of check results: rebuildable, never authoritative. Keyed by the
/// host's file id and face index (so a listed face whose stamp matches needs no bytes), and
/// looked up by content hash when the stamp does not match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontChecks {
    system_requested: bool,
    entries: BTreeMap<(String, u32), Entry>,
    by_hash: BTreeMap<(FaceHash, u32), CheckResult>,
}

impl FontChecks {
    /// Reads the cache text. A header of another version, or any line that does not parse,
    /// discards the whole cache.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self::try_parse(text).unwrap_or_default()
    }

    fn try_parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != header() {
            return None;
        }
        let mut checks = Self::default();
        for line in lines {
            if line == SYSTEM_REQUESTED {
                checks.system_requested = true;
                continue;
            }
            // `<hash> <index> <pieces or x> <stamp> <host id>`; the host id may hold spaces.
            let mut fields = line.splitn(5, ' ');
            let hash = FaceHash::from_hex(fields.next()?)?;
            let index: u32 = fields.next()?.parse().ok()?;
            let result = match fields.next()? {
                "x" => CheckResult::Unreadable,
                pieces => CheckResult::Pieces(pieces.parse().ok()?),
            };
            let stamp = fields.next()?.to_owned();
            let host_id = fields.next().filter(|id| !id.is_empty())?.to_owned();
            checks.record(&host_id, index, &stamp, hash, result);
        }
        Some(checks)
    }

    /// The cache text, entries sorted by host id and index.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = header();
        out.push('\n');
        if self.system_requested {
            out.push_str(SYSTEM_REQUESTED);
            out.push('\n');
        }
        for ((host_id, index), entry) in &self.entries {
            let result = match entry.result {
                CheckResult::Pieces(pieces) => pieces.to_string(),
                CheckResult::Unreadable => "x".to_owned(),
            };
            let _ = writeln!(
                out,
                "{} {index} {result} {} {host_id}",
                entry.hash, entry.stamp
            );
        }
        out
    }

    /// Whether Cuttable was chosen once on this computer, so the system fonts are checked.
    #[must_use]
    pub const fn system_requested(&self) -> bool {
        self.system_requested
    }

    /// Marks the system fonts as asked for; returns whether that is new.
    pub const fn request_system(&mut self) -> bool {
        let new = !self.system_requested;
        self.system_requested = true;
        new
    }

    /// The result of the listed face `index` of file `host_id` when its stamp still matches.
    #[must_use]
    pub fn by_stamp(&self, host_id: &str, index: u32, stamp: &str) -> Option<CheckResult> {
        self.entries
            .get(&(host_id.to_owned(), index))
            .filter(|entry| entry.stamp == stamp)
            .map(|entry| entry.result)
    }

    /// The result of a face by its file's content hash.
    #[must_use]
    pub fn by_hash(&self, hash: FaceHash, index: u32) -> Option<CheckResult> {
        self.by_hash.get(&(hash, index)).copied()
    }

    /// Keeps a result.
    pub fn record(
        &mut self,
        host_id: &str,
        index: u32,
        stamp: &str,
        hash: FaceHash,
        result: CheckResult,
    ) {
        // A stamp or an id with a line break or a space would not read back.
        if stamp.contains(char::is_whitespace) || host_id.contains(['\n', '\r']) {
            return;
        }
        self.by_hash.insert((hash, index), result);
        self.entries.insert(
            (host_id.to_owned(), index),
            Entry {
                hash,
                result,
                stamp: stamp.to_owned(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 18: results are found by stamp, then by hash; the text reads back to itself;
    /// another version or a bad line discards the whole cache.
    #[test]
    fn ac18_the_cache_round_trip_and_discard() {
        let mut checks = FontChecks::default();
        let hash = FaceHash::of(b"face");
        checks.record("idb:My Font.ttf", 0, "120", hash, CheckResult::Pieces(1));
        checks.record(
            "ab12",
            2,
            "9-100",
            FaceHash::of(b"x"),
            CheckResult::Unreadable,
        );
        checks.request_system();
        let text = checks.to_text();
        assert_eq!(FontChecks::parse(&text), checks);
        assert_eq!(FontChecks::parse(&text).to_text(), text);
        assert_eq!(
            checks.by_stamp("idb:My Font.ttf", 0, "120"),
            Some(CheckResult::Pieces(1))
        );
        assert_eq!(checks.by_stamp("idb:My Font.ttf", 0, "121"), None);
        assert_eq!(checks.by_hash(hash, 0), Some(CheckResult::Pieces(1)));
        let other = text.replace("curvyo-font-checks 1 2", "curvyo-font-checks 1 99");
        assert_eq!(FontChecks::parse(&other), FontChecks::default());
        assert_eq!(
            FontChecks::parse(&format!("{text}garbage\n")),
            FontChecks::default()
        );
    }
}
