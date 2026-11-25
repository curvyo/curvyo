//! The decoder of the shipped search keyword file (spec `0153` criteria 1, 4 and 9, ADR 0087
//! decisions 2 and 3): format 1 inside an optional gzip layer, every count and index checked.

use std::io::Read;

use crate::error::KeywordsError;
use crate::fold::fold;

/// The inflated payload is refused above this size (ADR 0087 decision 3).
pub const MAX_KEYWORDS_BYTES: usize = 4 << 20;
/// The related words per typed word at most (spec `0153` criterion 2, Proposal); a head word in
/// the file may list more, the matcher takes the strongest.
pub const MAX_RELATED: usize = 8;

const MAGIC: &[u8; 4] = b"CVYK";
const FORMAT: u8 = 1;
const GZIP: [u8; 2] = [0x1f, 0x8b];

/// One project whose keywords the file carries, as its header names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordSource {
    /// The Iconify set prefix (`lucide`).
    pub prefix: String,
    /// The project's name (`Lucide`).
    pub project: String,
    /// The npm package (`lucide-static`).
    pub package: String,
    /// The pinned version (`1.80.0`).
    pub version: String,
    /// The SPDX licence of the file the keywords come from (`ISC`).
    pub licence: String,
    /// The project's address.
    pub url: String,
    /// The name suffixes of variant bit *i* (Tabler `filled`: `<name>-filled`).
    pub variants: Vec<String>,
}

#[derive(Debug, Clone)]
struct Icon {
    name: Box<str>,
    variants: u64,
    keywords: Box<[u32]>,
    categories: Box<[u32]>,
}

/// The decoded keyword file, kept for the session.
#[derive(Debug, Clone)]
pub struct Keywords {
    sources: Vec<KeywordSource>,
    words: Vec<String>,
    /// Per source, sorted by name.
    icons: Vec<Vec<Icon>>,
    /// Per head word its related words, all indices into `words`.
    related: Vec<(u32, Box<[u32]>)>,
}

/// The keyword and category words of one icon.
#[derive(Debug, Clone, Copy)]
pub struct IconWords<'a> {
    words: &'a [String],
    icon: &'a Icon,
}

impl<'a> IconWords<'a> {
    /// The published keywords, folded and split into words.
    pub fn keywords(&self) -> impl Iterator<Item = &'a str> + 'a {
        let words = self.words;
        self.icon
            .keywords
            .iter()
            .map(move |i| words[*i as usize].as_str())
    }

    /// The category words.
    pub fn categories(&self) -> impl Iterator<Item = &'a str> + 'a {
        let words = self.words;
        self.icon
            .categories
            .iter()
            .map(move |i| words[*i as usize].as_str())
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, KeywordsError> {
        let byte = *self.data.get(self.at).ok_or(KeywordsError::Damaged)?;
        self.at += 1;
        Ok(byte)
    }

    fn num(&mut self) -> Result<u64, KeywordsError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            // The 10th byte holds bit 63 only: anything more overflows a u64.
            if shift == 63 && byte > 1 {
                return Err(KeywordsError::Damaged);
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte < 0x80 {
                return Ok(value);
            }
        }
        Err(KeywordsError::Damaged)
    }

    /// A count of items that take at least one byte each, so a damaged count allocates nothing.
    fn count(&mut self) -> Result<usize, KeywordsError> {
        let n = usize::try_from(self.num()?).map_err(|_| KeywordsError::Damaged)?;
        if n > self.data.len() - self.at {
            return Err(KeywordsError::Damaged);
        }
        Ok(n)
    }

    fn bytes(&mut self) -> Result<&[u8], KeywordsError> {
        let n = self.count()?;
        let part = &self.data[self.at..self.at + n];
        self.at += n;
        Ok(part)
    }

    fn text(&mut self) -> Result<String, KeywordsError> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| KeywordsError::Damaged)
    }

    fn index(&mut self, below: usize) -> Result<u32, KeywordsError> {
        let i = self.num()?;
        u32::try_from(i)
            .ok()
            .filter(|i| (*i as usize) < below)
            .ok_or(KeywordsError::Damaged)
    }

    fn indices(&mut self, below: usize) -> Result<Box<[u32]>, KeywordsError> {
        let n = self.count()?;
        (0..n).map(|_| self.index(below)).collect()
    }
}

/// The payload: inflated when `bytes` is gzip, as it is otherwise (a server may have removed the
/// gzip layer already).
fn payload(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, KeywordsError> {
    if !bytes.starts_with(&GZIP) {
        if bytes.len() > MAX_KEYWORDS_BYTES {
            return Err(KeywordsError::TooLarge);
        }
        return Ok(bytes.into());
    }
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_KEYWORDS_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| KeywordsError::Inflate)?;
    if out.len() > MAX_KEYWORDS_BYTES {
        return Err(KeywordsError::TooLarge);
    }
    Ok(out.into())
}

impl Keywords {
    /// Decodes the keyword file (gzip or plain format 1); a damaged file is an error, never a
    /// panic.
    ///
    /// # Errors
    /// [`KeywordsError`] when it does not inflate, is too large, is not format 1 or is damaged.
    pub fn decode(bytes: &[u8]) -> Result<Self, KeywordsError> {
        let data = payload(bytes)?;
        let head = data.get(..5).ok_or(KeywordsError::NotKeywords)?;
        if &head[..4] != MAGIC || head[4] != FORMAT {
            return Err(KeywordsError::NotKeywords);
        }
        let mut r = Reader { data: &data, at: 5 };
        let sources = (0..r.count()?)
            .map(|_| read_source(&mut r))
            .collect::<Result<Vec<_>, _>>()?;
        // Re-folded, so the matcher's word rule holds whatever wrote the file.
        let words = (0..r.count()?)
            .map(|_| r.text().map(|w| fold(&w)))
            .collect::<Result<Vec<_>, _>>()?;
        let icons = sources
            .iter()
            .map(|source| read_icons(&mut r, words.len(), source.variants.len()))
            .collect::<Result<Vec<_>, _>>()?;
        let related = (0..r.count()?)
            .map(|_| {
                let head = r.index(words.len())?;
                Ok((head, r.indices(words.len())?))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if r.at != data.len() {
            return Err(KeywordsError::Damaged);
        }
        Ok(Self {
            sources,
            words,
            icons,
            related,
        })
    }

    /// The projects the keywords come from, in the file's order.
    #[must_use]
    pub fn sources(&self) -> &[KeywordSource] {
        &self.sources
    }

    /// The keyword and category words of Iconify icon `prefix:name`, a variant name
    /// (`tabler:trash-filled`) included; `None` when the file has none for it (criterion 9).
    #[must_use]
    pub fn of(&self, prefix: &str, name: &str) -> Option<IconWords<'_>> {
        let s = self.sources.iter().position(|s| s.prefix == prefix)?;
        let icons = &self.icons[s];
        let find = |name: &str| {
            icons
                .binary_search_by(|icon| icon.name.as_ref().cmp(name))
                .ok()
                .map(|at| &icons[at])
        };
        let icon = find(name).or_else(|| {
            self.sources[s]
                .variants
                .iter()
                .enumerate()
                .find_map(|(bit, suffix)| {
                    let stem = name.strip_suffix(suffix.as_str())?.strip_suffix('-')?;
                    find(stem).filter(|icon| icon.variants & (1 << bit) != 0)
                })
        })?;
        Some(IconWords {
            words: &self.words,
            icon,
        })
    }

    /// Every head word with its related words, strongest first (the file's order).
    pub fn related(&self) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
        self.related.iter().map(|(head, rel)| {
            (
                self.words[*head as usize].as_str(),
                rel.iter().map(|i| self.words[*i as usize].as_str()),
            )
        })
    }
}

#[cfg(test)]
impl Keywords {
    /// A file with only a related-words table, for the matcher's tests.
    pub(crate) fn with_related(table: &[(&str, &[&str])]) -> Self {
        let mut words: Vec<String> = Vec::new();
        let mut id = |w: &str| {
            let at = words.iter().position(|x| x == w).unwrap_or_else(|| {
                words.push(w.to_owned());
                words.len() - 1
            });
            u32::try_from(at).unwrap()
        };
        let related = table
            .iter()
            .map(|(head, rel)| (id(head), rel.iter().map(|w| id(w)).collect()))
            .collect();
        Self {
            sources: Vec::new(),
            words,
            icons: Vec::new(),
            related,
        }
    }
}

fn read_source(r: &mut Reader<'_>) -> Result<KeywordSource, KeywordsError> {
    let mut field = || r.text();
    let source = KeywordSource {
        prefix: field()?,
        project: field()?,
        package: field()?,
        version: field()?,
        licence: field()?,
        url: field()?,
        variants: Vec::new(),
    };
    let n = r.count()?;
    if n > 64 {
        return Err(KeywordsError::Damaged);
    }
    let variants = (0..n).map(|_| r.text()).collect::<Result<_, _>>()?;
    Ok(KeywordSource { variants, ..source })
}

fn read_icons(
    r: &mut Reader<'_>,
    words: usize,
    variants: usize,
) -> Result<Vec<Icon>, KeywordsError> {
    let n = r.count()?;
    // Each icon takes at least 5 bytes: a damaged count reserves no more than the data can hold.
    let mut icons: Vec<Icon> = Vec::with_capacity(n.min((r.data.len() - r.at) / 5));
    let mut name = Vec::new();
    for _ in 0..n {
        let shared = usize::try_from(r.num()?).map_err(|_| KeywordsError::Damaged)?;
        if shared > name.len() {
            return Err(KeywordsError::Damaged);
        }
        name.truncate(shared);
        name.extend_from_slice(r.bytes()?);
        let text = std::str::from_utf8(&name).map_err(|_| KeywordsError::Damaged)?;
        if text.is_empty() || icons.last().is_some_and(|last| *last.name >= *text) {
            return Err(KeywordsError::Damaged);
        }
        let mask = r.num()?;
        if variants < 64 && mask >> variants != 0 {
            return Err(KeywordsError::Damaged);
        }
        icons.push(Icon {
            name: text.into(),
            variants: mask,
            keywords: r.indices(words)?,
            categories: r.indices(words)?,
        });
    }
    Ok(icons)
}

#[cfg(test)]
#[path = "keywords_tests.rs"]
mod tests;
