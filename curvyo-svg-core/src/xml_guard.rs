//! The one gate every untrusted SVG passes before the XML reader sees it (ADR 0018 §3, §6):
//! size, compression, encoding, the prolog and DOCTYPE, the element depth and the element count
//! are checked on the bytes, then `roxmltree` parses with a node limit.
//!
//! The DOCTYPE rule is the caller's: SVG fonts refuse any internal subset
//! (`specs/0076-single-line-fonts` criterion 23), the import accepts a bounded set of plain
//! literal entities (`specs/0024-svg-import-export` criterion 68). Nothing is fetched or resolved
//! outside the document; an external DTD is never read.

use std::borrow::Cow;
use std::collections::BTreeSet;

use roxmltree::{Document, ParsingOptions};

/// The deepest element nesting any SVG may have, checked on the bytes before parsing (the XML
/// reader recurses per level).
pub const MAX_DEPTH: usize = 256;

/// The most entity declarations the literal rule accepts.
pub const MAX_ENTITIES: usize = 64;

/// The longest entity value (bytes) the literal rule accepts.
pub const MAX_ENTITY_BYTES: usize = 256;

/// The most references to declared entities the literal rule accepts in one file.
pub const MAX_ENTITY_REFERENCES: usize = 100_000;

/// What a DOCTYPE may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctypeRule {
    /// No internal subset at all (SVG fonts).
    NoInternalSubset,
    /// An internal subset of plain literal entity declarations only, within [`MAX_ENTITIES`],
    /// [`MAX_ENTITY_BYTES`] and [`MAX_ENTITY_REFERENCES`] (the import).
    LiteralEntities,
}

/// The caller's limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLimits {
    /// The largest file read, in bytes.
    pub max_bytes: usize,
    /// The most XML nodes the reader builds; more refuse the file.
    pub max_nodes: u32,
    /// The most elements kept: the file is cut before the next element start and the open
    /// elements are closed, so everything before it is read. `None` keeps all.
    pub max_elements: Option<usize>,
    /// The DOCTYPE rule.
    pub doctype: DoctypeRule,
}

/// Why the bytes were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    /// Larger than the limit.
    TooLarge,
    /// gzip bytes (an `.svgz`).
    Compressed,
    /// Not UTF-8 text (a UTF-16 byte order mark included).
    NotUtf8,
    /// A DOCTYPE the rule does not allow.
    Entities,
    /// Elements nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// Not well-formed XML, with the reader's position (1-based) when it gives one.
    NotXml {
        /// The line.
        line: u32,
        /// The column.
        column: u32,
    },
}

/// Bytes that passed the checks, ready to parse.
#[derive(Debug)]
pub struct CheckedXml<'a> {
    text: Cow<'a, str>,
    max_nodes: u32,
    /// Whether the element cap cut the file.
    pub truncated: bool,
}

impl CheckedXml<'_> {
    /// Parses the checked text.
    ///
    /// # Errors
    /// [`GuardError::NotXml`] when it is not well-formed or has more nodes than allowed.
    pub fn parse(&self) -> Result<Document<'_>, GuardError> {
        let options = ParsingOptions {
            allow_dtd: true,
            nodes_limit: self.max_nodes,
        };
        Document::parse_with_options(&self.text, options).map_err(|error| {
            let pos = error.pos();
            GuardError::NotXml {
                line: pos.row,
                column: pos.col,
            }
        })
    }
}

/// Whether the first non-blank byte after an optional UTF-8 byte order mark is `<`.
#[must_use]
pub fn looks_like_xml(bytes: &[u8]) -> bool {
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    body.iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'<')
}

/// Whether `bytes` start with the gzip magic.
#[must_use]
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1F, 0x8B])
}

/// Whether `bytes` start with a UTF-16 byte order mark.
#[must_use]
pub fn is_utf16(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF])
}

/// Runs every byte check in the order of the refusals: size, compression, encoding, DOCTYPE,
/// depth; then applies the element cap.
///
/// # Errors
/// The first [`GuardError`] that applies.
pub fn check(bytes: &[u8], limits: GuardLimits) -> Result<CheckedXml<'_>, GuardError> {
    if bytes.len() > limits.max_bytes {
        return Err(GuardError::TooLarge);
    }
    if is_gzip(bytes) {
        return Err(GuardError::Compressed);
    }
    if is_utf16(bytes) {
        return Err(GuardError::NotUtf8);
    }
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = std::str::from_utf8(body).map_err(|_| GuardError::NotUtf8)?;
    let entities = match limits.doctype {
        DoctypeRule::NoInternalSubset => {
            if internal_subset(body).is_some() {
                return Err(GuardError::Entities);
            }
            BTreeSet::new()
        }
        DoctypeRule::LiteralEntities => literal_entities(body)?,
    };
    if !entities.is_empty() && entity_references(body, &entities) > MAX_ENTITY_REFERENCES {
        return Err(GuardError::Entities);
    }
    let scan = scan_elements(body, limits.max_elements);
    if scan.too_deep {
        return Err(GuardError::TooDeep);
    }
    let (text, truncated) = match scan.cut {
        Some((at, open)) => {
            let mut cut = text.get(..at).unwrap_or_default().to_owned();
            for name in open.iter().rev() {
                cut.push_str("</");
                cut.push_str(name);
                cut.push('>');
            }
            (Cow::Owned(cut), true)
        }
        None => (Cow::Borrowed(text), false),
    };
    Ok(CheckedXml {
        text,
        max_nodes: limits.max_nodes,
        truncated,
    })
}

/// The byte range of the DOCTYPE's internal subset (between `[` and its `]`), or `None` when the
/// prolog has no DOCTYPE or one without a subset. The prolog is read as the XML reader reads it:
/// the XML declaration, processing instructions, comments and whitespace before the root element
/// are skipped, so a decoy `<!DOCTYPE` inside a comment or an instruction hides nothing.
fn internal_subset(bytes: &[u8]) -> Option<usize> {
    let skip_past = |from: usize, end: &[u8]| {
        bytes
            .get(from..)
            .and_then(|rest| rest.windows(end.len()).position(|w| w == end))
            .map(|at| from + at + end.len())
    };
    let mut at = 0;
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let rest = bytes.get(at..).unwrap_or_default();
        let next = if rest.starts_with(b"<?") {
            skip_past(at + 2, b"?>")
        } else if rest.starts_with(b"<!--") {
            skip_past(at + 4, b"-->")
        } else if rest.starts_with(b"<!DOCTYPE") {
            let mut quote: Option<u8> = None;
            for (i, &b) in rest.iter().enumerate().skip(9) {
                match (quote, b) {
                    (Some(q), _) if b == q => quote = None,
                    (None, b'"' | b'\'') => quote = Some(b),
                    (None, b'[') => return Some(at + i + 1),
                    (None, b'>') => return None,
                    _ => {}
                }
            }
            return None;
        } else {
            return None;
        };
        at = next?;
    }
}

/// The names of the internal subset's entity declarations when every declaration is a plain
/// literal within the limits; [`GuardError::Entities`] for anything else in the subset.
fn literal_entities(bytes: &[u8]) -> Result<BTreeSet<String>, GuardError> {
    let mut names = BTreeSet::new();
    let Some(mut at) = internal_subset(bytes) else {
        return Ok(names);
    };
    let refuse = Err(GuardError::Entities);
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let rest = bytes.get(at..).unwrap_or_default();
        if rest.starts_with(b"]") {
            return Ok(names);
        }
        if rest.starts_with(b"<!--") {
            let Some(end) = rest.windows(3).position(|w| w == b"-->") else {
                return refuse;
            };
            at += end + 3;
            continue;
        }
        if !rest.starts_with(b"<!ENTITY") || names.len() >= MAX_ENTITIES {
            return refuse;
        }
        let mut i = 8;
        let ws = |i: &mut usize| {
            let from = *i;
            while rest.get(*i).is_some_and(u8::is_ascii_whitespace) {
                *i += 1;
            }
            *i > from
        };
        if !ws(&mut i) {
            return refuse;
        }
        let name_start = i;
        while rest
            .get(i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
        {
            i += 1;
        }
        let name = std::str::from_utf8(&rest[name_start..i]).unwrap_or_default();
        if name.is_empty() || !ws(&mut i) {
            return refuse;
        }
        let Some(&quote @ (b'"' | b'\'')) = rest.get(i) else {
            return refuse;
        };
        i += 1;
        let value_start = i;
        while let Some(&b) = rest.get(i) {
            if b == quote {
                break;
            }
            if matches!(b, b'&' | b'<' | b'%') {
                return refuse;
            }
            i += 1;
        }
        if rest.get(i).is_none() || i - value_start > MAX_ENTITY_BYTES {
            return refuse;
        }
        i += 1;
        ws(&mut i);
        if rest.get(i) != Some(&b'>') {
            return refuse;
        }
        names.insert(name.to_owned());
        at += i + 1;
    }
}

/// How many `&name;` references to one of `names` the bytes hold.
fn entity_references(bytes: &[u8], names: &BTreeSet<String>) -> usize {
    let mut count = 0;
    let mut at = 0;
    while let Some(offset) = bytes
        .get(at..)
        .and_then(|r| r.iter().position(|b| *b == b'&'))
    {
        at += offset + 1;
        let rest = bytes.get(at..).unwrap_or_default();
        let Some(end) = rest.iter().take(65).position(|b| *b == b';') else {
            continue;
        };
        if std::str::from_utf8(&rest[..end]).is_ok_and(|name| names.contains(name)) {
            count += 1;
        }
    }
    count
}

/// What the element scan found.
struct Scan {
    too_deep: bool,
    /// Where to cut and the names of the elements open there, outermost first.
    cut: Option<(usize, Vec<String>)>,
}

/// Scans the elements as the XML reader would recurse, stopping as soon as the nesting passes
/// [`MAX_DEPTH`] or, with `max_elements`, at the start of the element after the last one kept.
/// Comments, CDATA, processing instructions, declarations and quoted attribute values are skipped.
fn scan_elements(bytes: &[u8], max_elements: Option<usize>) -> Scan {
    let skip_to = |from: usize, end: &[u8]| {
        bytes
            .get(from..)
            .and_then(|rest| rest.windows(end.len()).position(|w| w == end))
            .map_or(bytes.len(), |at| from + at + end.len())
    };
    let mut open: Vec<String> = Vec::new();
    let mut elements = 0_usize;
    let mut at = 0;
    while let Some(offset) = bytes
        .get(at..)
        .and_then(|r| r.iter().position(|b| *b == b'<'))
    {
        at += offset;
        let rest = &bytes[at..];
        if rest.starts_with(b"<!--") {
            at = skip_to(at + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            at = skip_to(at + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            at = skip_to(at + 2, b"?>");
        } else if rest.starts_with(b"<!DOCTYPE") {
            at = skip_doctype(bytes, at);
        } else if rest.starts_with(b"<!") {
            at = skip_to(at + 2, b">");
        } else if rest.starts_with(b"</") {
            open.pop();
            at = skip_to(at + 2, b">");
        } else {
            if max_elements.is_some_and(|max| elements >= max) && !open.is_empty() {
                return Scan {
                    too_deep: false,
                    cut: Some((at, open)),
                };
            }
            elements += 1;
            let name_end = rest
                .iter()
                .skip(1)
                .position(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'>'))
                .map_or(rest.len(), |p| p + 1);
            let name = String::from_utf8_lossy(&rest[1..name_end]).into_owned();
            let mut quote: Option<u8> = None;
            let mut i = at + 1;
            let mut empty = false;
            while let Some(&b) = bytes.get(i) {
                match (quote, b) {
                    (Some(q), _) if b == q => quote = None,
                    (None, b'"' | b'\'') => quote = Some(b),
                    (None, b'>') => {
                        empty = bytes.get(i.wrapping_sub(1)) == Some(&b'/');
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }
            if !empty {
                open.push(name);
                if open.len() > MAX_DEPTH {
                    return Scan {
                        too_deep: true,
                        cut: None,
                    };
                }
            }
            at = i + 1;
        }
        if at >= bytes.len() {
            break;
        }
    }
    Scan {
        too_deep: false,
        cut: None,
    }
}

/// The index after a DOCTYPE starting at `at`, its internal subset included.
fn skip_doctype(bytes: &[u8], at: usize) -> usize {
    let mut quote: Option<u8> = None;
    let mut in_subset = false;
    let mut i = at + 9;
    while let Some(&b) = bytes.get(i) {
        match (quote, b) {
            (Some(q), _) if b == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(b),
            (None, b'[') => in_subset = true,
            (None, b']') => in_subset = false,
            (None, b'>') if !in_subset => return i + 1,
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const FONT: GuardLimits = GuardLimits {
        max_bytes: 1 << 20,
        max_nodes: 10_000,
        max_elements: None,
        doctype: DoctypeRule::NoInternalSubset,
    };
    const IMPORT: GuardLimits = GuardLimits {
        doctype: DoctypeRule::LiteralEntities,
        max_elements: Some(3),
        ..FONT
    };

    #[test]
    fn the_internal_subset_is_found_past_decoys() {
        assert!(looks_like_xml(b"\xEF\xBB\xBF \n<svg/>"));
        assert!(!looks_like_xml(b"\x00\x01\x00\x00"));
        assert!(
            internal_subset(
                br#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://x/svg11.dtd" ><svg/>"#
            )
            .is_none()
        );
        assert!(internal_subset(br#"<!DOCTYPE svg [<!ENTITY a "b">]><svg/>"#).is_some());
        assert!(internal_subset(br#"<!DOCTYPE svg PUBLIC "a>b" [ ]><svg/>"#).is_some());
        assert!(
            internal_subset(
                br#"<?xml version="1.0"?><!-- <!DOCTYPE svg> --><!DOCTYPE svg [ ]><svg/>"#
            )
            .is_some()
        );
        assert!(internal_subset(b"<svg><!-- <!DOCTYPE svg [ ]> --></svg>").is_none());
    }

    #[test]
    fn depth_is_counted_as_the_reader_recurses() {
        let deep = "<a>".repeat(MAX_DEPTH + 1);
        assert!(scan_elements(deep.as_bytes(), None).too_deep);
        let wide = "<a/><b x='<>'></b>".repeat(10_000);
        assert!(!scan_elements(format!("<svg>{wide}</svg>").as_bytes(), None).too_deep);
    }

    #[test]
    fn fonts_refuse_any_subset_the_import_takes_plain_literals() {
        let ai = br#"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" [
	<!ENTITY ns_extend "http://ns.adobe.com/Extensibility/1.0/">
	<!ENTITY ns_ai "http://ns.adobe.com/AdobeIllustrator/10.0/">
]>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:x="&ns_extend;" xmlns:i="&ns_ai;"><g/></svg>"#;
        assert_eq!(check(ai, FONT).unwrap_err(), GuardError::Entities);
        let checked = check(ai, IMPORT).unwrap();
        let doc = checked.parse().unwrap();
        let root = doc.root_element();
        assert_eq!(
            root.lookup_namespace_uri(Some("i")),
            Some("http://ns.adobe.com/AdobeIllustrator/10.0/")
        );
        for bad in [
            &br#"<!DOCTYPE svg [<!ENTITY a "&b;">]><svg/>"#[..],
            br#"<!DOCTYPE svg [<!ENTITY a SYSTEM "file:///etc/passwd">]><svg/>"#,
            br#"<!DOCTYPE svg [<!ENTITY % a "x">]><svg/>"#,
            b"<!DOCTYPE svg [<!ELEMENT svg ANY>]><svg/>",
            br#"<!DOCTYPE svg [<!ENTITY a "<g/>">]><svg/>"#,
        ] {
            assert_eq!(check(bad, IMPORT).unwrap_err(), GuardError::Entities);
        }
        let long = format!(
            r#"<!DOCTYPE svg [<!ENTITY a "{}">]><svg/>"#,
            "x".repeat(257)
        );
        assert_eq!(
            check(long.as_bytes(), IMPORT).unwrap_err(),
            GuardError::Entities
        );
        let fan = format!(
            r#"<!DOCTYPE svg [<!ENTITY a "x">]><svg>{}</svg>"#,
            "&a;".repeat(MAX_ENTITY_REFERENCES + 1)
        );
        let limits = GuardLimits {
            max_bytes: 1 << 24,
            ..IMPORT
        };
        assert_eq!(
            check(fan.as_bytes(), limits).unwrap_err(),
            GuardError::Entities
        );
    }

    #[test]
    fn the_refusals_come_in_order() {
        assert_eq!(
            check(&[0x1F, 0x8B, 0], IMPORT).unwrap_err(),
            GuardError::Compressed
        );
        assert_eq!(
            check(b"\xFF\xFE<\x00", IMPORT).unwrap_err(),
            GuardError::NotUtf8
        );
        assert_eq!(
            check(b"<svg>\xC3</svg>", IMPORT).unwrap_err(),
            GuardError::NotUtf8
        );
        let small = GuardLimits {
            max_bytes: 4,
            ..IMPORT
        };
        assert_eq!(check(b"<svg/>", small).unwrap_err(), GuardError::TooLarge);
        let broken = check(b"<svg>\n  <g></svg>", IMPORT).unwrap();
        assert!(matches!(
            broken.parse(),
            Err(GuardError::NotXml { line: 2, .. })
        ));
    }

    #[test]
    fn the_element_cap_keeps_what_came_before() {
        let checked = check(b"<svg><g><a/><b/><c/></g></svg>", IMPORT).unwrap();
        assert!(checked.truncated);
        let doc = checked.parse().unwrap();
        let names: Vec<&str> = doc
            .descendants()
            .filter(roxmltree::Node::is_element)
            .map(|n| n.tag_name().name())
            .collect();
        assert_eq!(names, ["svg", "g", "a"]);
    }
}
