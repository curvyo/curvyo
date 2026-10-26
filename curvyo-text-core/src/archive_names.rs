//! Makes an archive entry's name safe as a file name (`specs/0083-font-info-and-zip` criteria 9
//! and 16): the unsafe path forms, the base name, the document rule and the sanitised, unique
//! file name. Names are never used as paths; only the sanitised base name is.

/// The longest sanitised file name, in characters (criterion 9).
const MAX_NAME_CHARS: usize = 120;

/// The longest sanitised file name in UTF-8 bytes: 120 characters of a script that takes three or
/// four bytes each would pass the 255-byte limit of ext4, APFS and NTFS names.
const MAX_NAME_BYTES: usize = 240;

/// Base names (ignoring case) that start a licence or readme file (the spec's "Terms").
const DOCUMENT_PREFIXES: [&str; 11] = [
    "ofl",
    "license",
    "licence",
    "copying",
    "copyright",
    "readme",
    "read me",
    "credits",
    "authors",
    "fontlog",
    "eula",
];

/// The extensions of a document entry.
const DOCUMENT_EXTENSIONS: [&str; 3] = ["txt", "md", "pdf"];

/// Names Windows reserves for devices, with or without an extension.
const RESERVED: [&str; 4] = ["con", "prn", "aux", "nul"];

/// The parts of an entry name, split on `/` and `\`.
pub(crate) fn parts(name: &str) -> impl DoubleEndedIterator<Item = &str> {
    name.split(['/', '\\'])
}

/// Whether `name` is an absolute path, a drive-letter path, a UNC path or has a `..` part
/// (criterion 16).
pub(crate) fn is_unsafe_path(name: &str) -> bool {
    if name.starts_with(['/', '\\']) {
        return true;
    }
    let mut chars = name.chars();
    let drive = matches!((chars.next(), chars.next()), (Some(letter), Some(':')) if letter.is_ascii_alphabetic());
    drive || parts(name).any(|part| part == "..")
}

/// The last non-empty part of `name`; empty for a folder name.
pub(crate) fn base_name(name: &str) -> &str {
    parts(name).next_back().unwrap_or("")
}

/// The lower-case extension of a base name (after its last `.`, not a leading one); `None`
/// without one.
pub(crate) fn extension(base: &str) -> Option<String> {
    base.rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map(|(_, ext)| ext.to_lowercase())
}

/// Whether the base name of `name` is a document the archive keeps (the spec's "Terms"): a
/// licence or readme name with no extension or `.txt`, `.md`, `.pdf`, or any `.txt`, `.md` or
/// `.pdf`.
#[must_use]
pub fn is_document_name(name: &str) -> bool {
    let base = base_name(name);
    if let Some(ext) = extension(base) {
        return DOCUMENT_EXTENSIONS.contains(&ext.as_str());
    }
    let lower = base.to_lowercase();
    DOCUMENT_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Whether `stem` (before the first `.`) is a name Windows reserves.
fn is_reserved(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_lowercase();
    RESERVED.contains(&stem.as_str())
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

/// `name` cut to the character and byte limits, the extension kept.
fn truncated(name: &str) -> String {
    let fits = |text: &str| text.chars().count() <= MAX_NAME_CHARS && text.len() <= MAX_NAME_BYTES;
    if fits(name) {
        return name.to_owned();
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.chars().count() <= 16 => {
            (stem, format!(".{ext}"))
        }
        _ => (name, String::new()),
    };
    let mut out = String::new();
    for c in stem.chars() {
        let candidate = format!("{out}{c}{ext}");
        if !fits(&candidate) {
            break;
        }
        out.push(c);
    }
    out.push_str(&ext);
    out
}

/// A file or folder name safe on Windows, macOS and Linux (criterion 9): `<>:"/\|?*` and
/// control characters replaced by `_`, at most 120 characters, trailing dots and spaces removed,
/// a reserved name such as `CON` prefixed with `_`; `_` for a name that leaves nothing.
#[must_use]
pub fn sanitised_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut out = truncated(&replaced).trim_end_matches(['.', ' ']).to_owned();
    if out.is_empty() {
        out.push('_');
    }
    if is_reserved(&out) {
        out.insert(0, '_');
        truncated(&out)
            .trim_end_matches(['.', ' '])
            .clone_into(&mut out);
    }
    out
}

/// `name` if no name of `taken` equals it by Unicode lower case, else the first free
/// `numbered(n)` for n = 2, 3, ...
fn first_free(name: &str, taken: &[String], numbered: impl Fn(u32) -> String) -> String {
    let is_taken = |candidate: &str| {
        let lower = candidate.to_lowercase();
        taken.iter().any(|t| t.to_lowercase() == lower)
    };
    if !is_taken(name) {
        return name.to_owned();
    }
    (2..=u32::MAX)
        .map(numbered)
        .find(|candidate| !is_taken(candidate))
        .unwrap_or_else(|| name.to_owned())
}

/// The first free name for `name` among `taken`, compared by Unicode lower case: `name`, else
/// `<stem>-2.<ext>`, `-3`, ... (the rule of `curvyo-storage-io`'s Add).
#[must_use]
pub fn free_name(name: &str, taken: &[String]) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    first_free(name, taken, |n| format!("{stem}-{n}{ext}"))
}

/// The first free folder name for `name` among `taken`, compared by Unicode lower case:
/// `name`, else `<name>-2`, `-3`, ... (a folder has no extension, criterion 9).
#[must_use]
pub fn free_folder_name(name: &str, taken: &[String]) -> String {
    first_free(name, taken, |n| format!("{name}-{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 16: absolute, drive, UNC and `..` names are unsafe; `a..b` is not.
    #[test]
    fn unsafe_paths() {
        for name in [
            "/etc/x.ttf",
            "\\x.ttf",
            "C:x.ttf",
            "c:/x.ttf",
            "\\\\host\\s\\x.ttf",
            "a/../x.ttf",
            "..\\x.ttf",
        ] {
            assert!(is_unsafe_path(name), "{name}");
        }
        for name in ["a/b.ttf", "a..b.ttf", "fonts\\x.ttf", "1:x"] {
            assert!(!is_unsafe_path(name), "{name}");
        }
    }

    /// The Terms: licence and readme names, and every `.txt`, `.md`, `.pdf`.
    #[test]
    fn document_names() {
        for name in [
            "OFL.txt",
            "x/LICENSE",
            "licence.md",
            "READ ME",
            "Readme.PDF",
            "notes.txt",
            "COPYING",
            "EULA.pdf",
            "FONTLOG",
        ] {
            assert!(is_document_name(name), "{name}");
        }
        for name in ["LICENSE.html", "logo.png", "font.ttf", "notes", ".txt"] {
            assert!(!is_document_name(name), "{name}");
        }
    }

    /// Criterion 9: characters, reserved names, length and trailing dots.
    #[test]
    fn sanitising() {
        assert_eq!(sanitised_name("a<b>:c\"|?*.txt"), "a_b__c____.txt");
        assert_eq!(sanitised_name("tab\there.ttf"), "tab_here.ttf");
        assert_eq!(sanitised_name("CON"), "_CON");
        assert_eq!(sanitised_name("con.txt"), "_con.txt");
        assert_eq!(sanitised_name("LPT1.ttf"), "_LPT1.ttf");
        assert_eq!(sanitised_name("COM0.ttf"), "COM0.ttf");
        assert_eq!(sanitised_name("name. . "), "name");
        assert_eq!(sanitised_name("..."), "_");
        let long = format!("{}.ttf", "x".repeat(200));
        let cut = sanitised_name(&long);
        assert_eq!(cut.chars().count(), 120);
        assert!(
            std::path::Path::new(&cut)
                .extension()
                .is_some_and(|e| e == "ttf")
        );
        let wide = format!("{}.ttf", "漢".repeat(119));
        assert!(sanitised_name(&wide).len() <= MAX_NAME_BYTES);
    }

    #[test]
    fn free_names() {
        let taken = vec!["OFL.txt".to_owned(), "ofl-2.TXT".to_owned()];
        assert_eq!(free_name("ofl.txt", &taken), "ofl-3.txt");
        assert_eq!(free_name("Lobster", &["lobster".to_owned()]), "Lobster-2");
        assert_eq!(free_name("README", &[]), "README");
        assert_eq!(
            free_folder_name("Pack.v2", &["pack.V2".to_owned()]),
            "Pack.v2-2"
        );
    }
}
