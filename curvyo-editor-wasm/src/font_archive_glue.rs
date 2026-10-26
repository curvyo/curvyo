//! A zip archive of fonts for the browser's Add font (`specs/0083-font-info-and-zip` criteria 8,
//! 15, 16 and 18, `adrs.md` decisions 1 and 5): the core reader's entry table as JSON and one
//! entry's bytes at a time, so the page keeps each in a JS buffer outside the wasm heap.

use curvyo_text_core::{ArchiveError, EntryClass, EntrySkip, FontArchive, FontFileKind};
use serde_json::json;

/// An opened archive.
#[derive(Debug)]
pub struct FontArchiveGlue {
    archive: FontArchive,
}

impl FontArchiveGlue {
    /// Opens `bytes`; the error is `"unreadable"` or `"over_limit"` (criterion 13's reasons).
    ///
    /// # Errors
    /// The refusal's code when the archive is refused whole.
    pub fn open(bytes: Vec<u8>) -> Result<Self, &'static str> {
        FontArchive::open(bytes)
            .map(|archive| Self { archive })
            .map_err(|error| match error {
                ArchiveError::OverLimit => "over_limit",
                ArchiveError::NotZip | ArchiveError::Damaged => "unreadable",
            })
    }

    /// The entries as JSON `[{"name", "file_name", "class", "kind"}]`: `class` is `"font"`,
    /// `"document"`, `"left_out"` or `"unsafe"`; `kind` (fonts only) `"ttf"`, `"otf"`, `"ttc"`
    /// or `"svg"`, in the order a face offered twice is taken in.
    #[must_use]
    pub fn entries(&self) -> String {
        let entries: Vec<serde_json::Value> = self
            .archive
            .entries()
            .iter()
            .map(|entry| {
                let (class, kind) = match entry.class {
                    EntryClass::Font(kind) => (
                        "font",
                        Some(match kind {
                            FontFileKind::Ttf => "ttf",
                            FontFileKind::Otf => "otf",
                            FontFileKind::Ttc => "ttc",
                            FontFileKind::Svg => "svg",
                        }),
                    ),
                    EntryClass::Document => ("document", None),
                    EntryClass::LeftOut => ("left_out", None),
                    EntryClass::Unsafe(_) => ("unsafe", None),
                };
                json!({ "name": entry.name, "file_name": entry.file_name, "class": class, "kind": kind })
            })
            .collect();
        serde_json::Value::Array(entries).to_string()
    }

    /// The bytes of font or document entry `index`.
    ///
    /// # Errors
    /// `"unsafe"` for a skipped entry, `"over_limit"` once the archive passed its total.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>, &'static str> {
        self.archive.read(index).map_err(|skip| match skip {
            EntrySkip::ArchiveOverLimit => "over_limit",
            EntrySkip::Unsafe | EntrySkip::NotKept => "unsafe",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_and_reads_cross_as_json_and_bytes() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bytes =
            std::fs::read(dir.join("../curvyo-text-core/tests/fixtures/fonts/zip/plain.zip"))
                .unwrap();
        let mut glue = FontArchiveGlue::open(bytes).unwrap();
        let entries: serde_json::Value = serde_json::from_str(&glue.entries()).unwrap();
        assert_eq!(entries[0]["class"], "font");
        assert_eq!(entries[0]["kind"], "svg");
        assert_eq!(entries[3]["class"], "left_out");
        assert_eq!(glue.read(0).map(|b| b.len() > 1000), Ok(true));
        assert_eq!(glue.read(3), Err("unsafe"));
        assert_eq!(
            FontArchiveGlue::open(b"text".to_vec()).unwrap_err(),
            "unreadable"
        );
    }
}
