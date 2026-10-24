//! Decodes the shipped Unicode character names (`specs/0082-glyph-table` criteria 16, 17 and 19;
//! `adrs.md` decision 2; format in `tools/unicode-data/README.md`).
//!
//! The file comes from the frontend's own assets, but it is read as untrusted bytes: every count,
//! code point and word index is checked, and a damaged file is an error, never a panic.

use crate::error::TextError;

const MAGIC: &[u8; 4] = b"CVYN";
const FORMAT: u8 = 1;
const MAX_CODE: u32 = 0x10_FFFF;
/// More words than the file could hold for every code point is a damaged file.
const MAX_WORDS: usize = 0x11_0000;
/// The longest stored string (a word, prefix or alias).
const MAX_TEXT: usize = 256;

/// The first and last Hangul syllable (Unicode name rule NR1).
const HANGUL: (u32, u32) = (0xAC00, 0xD7A3);
const JAMO_L: [&str; 19] = [
    "G", "GG", "N", "D", "DD", "R", "M", "B", "BB", "S", "SS", "", "J", "JJ", "C", "K", "T", "P",
    "H",
];
const JAMO_V: [&str; 21] = [
    "A", "AE", "YA", "YAE", "EO", "E", "YEO", "YE", "O", "WA", "WAE", "OE", "YO", "U", "WEO", "WE",
    "WI", "YU", "EU", "YI", "I",
];
const JAMO_T: [&str; 28] = [
    "", "G", "GG", "GS", "N", "NJ", "NH", "D", "L", "LG", "LM", "LB", "LS", "LT", "LP", "LH", "M",
    "B", "BS", "S", "SS", "NG", "J", "C", "K", "T", "P", "H",
];

/// The character names of one Unicode version.
#[derive(Debug, Clone)]
pub struct UnicodeNames {
    version: (u8, u8, u8),
    words: Vec<Box<str>>,
    /// `(first, last, prefix)`: the name is the prefix and the code point in hex.
    ranges: Vec<(u32, u32, Box<str>)>,
    /// The code points with a stored name, ascending, and where their words start in `word_ids`.
    codes: Vec<u32>,
    starts: Vec<u32>,
    word_ids: Vec<u32>,
    abbreviations: Vec<(u32, Box<str>)>,
}

/// Reads the file front to back.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, TextError> {
        let byte = *self.bytes.get(self.at).ok_or(TextError::UnreadableNames)?;
        self.at += 1;
        Ok(byte)
    }

    fn varint(&mut self) -> Result<u32, TextError> {
        let mut value: u32 = 0;
        for shift in (0..35).step_by(7) {
            let byte = self.byte()?;
            let part = u32::from(byte & 0x7F)
                .checked_shl(shift)
                .filter(|part| part >> shift == u32::from(byte & 0x7F))
                .ok_or(TextError::UnreadableNames)?;
            value |= part;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(TextError::UnreadableNames)
    }

    fn count(&mut self, max: usize) -> Result<usize, TextError> {
        usize::try_from(self.varint()?)
            .ok()
            .filter(|count| *count <= max)
            .ok_or(TextError::UnreadableNames)
    }

    fn text(&mut self) -> Result<Box<str>, TextError> {
        let len = self.count(MAX_TEXT)?;
        let end = self.at + len;
        let raw = self
            .bytes
            .get(self.at..end)
            .ok_or(TextError::UnreadableNames)?;
        self.at = end;
        std::str::from_utf8(raw)
            .ok()
            .filter(|text| text.is_ascii())
            .map(Box::from)
            .ok_or(TextError::UnreadableNames)
    }

    /// The next code point of an ascending list, `delta` past `last`.
    fn code_after(&mut self, last: u32) -> Result<u32, TextError> {
        last.checked_add(self.varint()?)
            .filter(|code| *code <= MAX_CODE)
            .ok_or(TextError::UnreadableNames)
    }
}

impl UnicodeNames {
    /// Decodes a names file (format 1).
    ///
    /// # Errors
    /// [`TextError::UnreadableNames`] for a file that is not one, damaged or cut short.
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut r = Reader { bytes, at: 0 };
        let mut magic = [0_u8; 4];
        for slot in &mut magic {
            *slot = r.byte()?;
        }
        if &magic != MAGIC || r.byte()? != FORMAT {
            return Err(TextError::UnreadableNames);
        }
        let version = (r.byte()?, r.byte()?, r.byte()?);
        let word_count = r.count(MAX_WORDS)?;
        let words = (0..word_count)
            .map(|_| r.text())
            .collect::<Result<Vec<_>, _>>()?;
        let range_count = r.count(MAX_WORDS)?;
        let mut ranges = Vec::with_capacity(range_count.min(1024));
        for _ in 0..range_count {
            let first = r.varint()?;
            let last = first
                .checked_add(r.varint()?)
                .filter(|last| *last <= MAX_CODE)
                .ok_or(TextError::UnreadableNames)?;
            ranges.push((first, last, r.text()?));
        }
        let name_count = r.count(MAX_WORDS)?;
        let mut codes = Vec::with_capacity(name_count);
        let mut starts = Vec::with_capacity(name_count + 1);
        let mut word_ids = Vec::new();
        let mut last = 0;
        for i in 0..name_count {
            let code = r.code_after(last)?;
            if i > 0 && code == last {
                return Err(TextError::UnreadableNames);
            }
            last = code;
            codes.push(code);
            starts.push(u32::try_from(word_ids.len()).map_err(|_| TextError::UnreadableNames)?);
            for _ in 0..r.count(64)? {
                let id = r.varint()?;
                if usize::try_from(id).map_or(true, |id| id >= words.len()) {
                    return Err(TextError::UnreadableNames);
                }
                word_ids.push(id);
            }
        }
        starts.push(u32::try_from(word_ids.len()).map_err(|_| TextError::UnreadableNames)?);
        let alias_count = r.count(MAX_WORDS)?;
        let mut abbreviations = Vec::with_capacity(alias_count.min(4096));
        let mut last = 0;
        for _ in 0..alias_count {
            last = r.code_after(last)?;
            abbreviations.push((last, r.text()?));
        }
        if r.at != bytes.len() {
            return Err(TextError::UnreadableNames);
        }
        Ok(Self {
            version,
            words,
            ranges,
            codes,
            starts,
            word_ids,
            abbreviations,
        })
    }

    /// The Unicode version the names follow.
    #[must_use]
    pub const fn version(&self) -> (u8, u8, u8) {
        self.version
    }

    /// The word dictionary the stored names index into.
    #[must_use]
    pub fn words(&self) -> &[Box<str>] {
        &self.words
    }

    /// The dictionary indices of `c`'s stored name; `None` when its name is computed or it has
    /// none.
    #[must_use]
    pub fn stored_words(&self, c: char) -> Option<&[u32]> {
        let at = self.codes.binary_search(&u32::from(c)).ok()?;
        let start = usize::try_from(*self.starts.get(at)?).ok()?;
        let end = usize::try_from(*self.starts.get(at + 1)?).ok()?;
        self.word_ids.get(start..end)
    }

    /// `c`'s Unicode name ("LATIN SMALL LETTER A WITH DIAERESIS"); `None` for a character
    /// without one (controls, private use, unassigned).
    #[must_use]
    pub fn name_of(&self, c: char) -> Option<String> {
        if let Some(ids) = self.stored_words(c) {
            let parts: Vec<&str> = ids
                .iter()
                .filter_map(|id| self.words.get(usize::try_from(*id).ok()?))
                .map(AsRef::as_ref)
                .collect();
            return Some(parts.join(" "));
        }
        computed_name(&self.ranges, u32::from(c))
    }

    /// `c`'s abbreviation alias ("SP", "NBSP", "ZWJ"), the label of an empty cell.
    #[must_use]
    pub fn abbreviation_of(&self, c: char) -> Option<&str> {
        let code = u32::from(c);
        let at = self
            .abbreviations
            .binary_search_by_key(&code, |(k, _)| *k)
            .ok()?;
        self.abbreviations.get(at).map(|(_, alias)| alias.as_ref())
    }
}

/// The names rules NR1 (Hangul syllables) and NR2 (a prefix and the hex code point) give.
fn computed_name(ranges: &[(u32, u32, Box<str>)], code: u32) -> Option<String> {
    if (HANGUL.0..=HANGUL.1).contains(&code) {
        let index = usize::try_from(code - HANGUL.0).ok()?;
        let (l, v, t) = (index / (21 * 28), index % (21 * 28) / 28, index % 28);
        return Some(format!(
            "HANGUL SYLLABLE {}{}{}",
            JAMO_L.get(l)?,
            JAMO_V.get(v)?,
            JAMO_T.get(t)?
        ));
    }
    ranges
        .iter()
        .find(|(first, last, _)| (*first..=*last).contains(&code))
        .map(|(_, _, prefix)| format!("{prefix}{code:04X}"))
}
