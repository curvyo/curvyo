//! Reads the faces, glyphs, advances and kerning of an SVG font from bytes
//! (`specs/0076-single-line-fonts` criteria 1, 14 to 16, 23; `adrs.md` decision 2).
//!
//! The `<font>` element of SVG 1.1 as Hershey Text v3 and the EMS `svg-fonts` files write it.
//! Hostile input is refused before the XML reader sees it: the size, a DOCTYPE with an internal
//! subset (so no entity is ever declared) and the element depth are checked on the bytes; the
//! node count, glyph count, points and numbers while reading. Nothing is fetched or resolved.

use std::collections::BTreeMap;
use std::sync::Arc;

use kurbo::{Affine, BezPath};
use roxmltree::{Document, Node, ParsingOptions};

use crate::face_style::{FaceStyle, NORMAL_WIDTH, Slant};
use crate::limits::{
    MAX_GLYPH_POINTS, MAX_SVG_COORDINATE, MAX_SVG_DEPTH, MAX_SVG_FONT_BYTES, MAX_SVG_FONT_POINTS,
    MAX_SVG_GLYPHS, MAX_SVG_KERN_PAIRS, MAX_SVG_NODES,
};
use crate::outline::GlyphOutline;
use crate::path_data::parse_path_data;

/// The em fraction within which arcs become cubics (0.0002 mm at the largest size).
const ARC_TOLERANCE_EM: f64 = 1e-7;

/// The `units-per-em` of a font that states none (SVG 1.1).
const DEFAULT_UNITS_PER_EM: f64 = 1000.0;

/// One parsed `<font>`. Glyph 0 is `<missing-glyph>`; the others follow in file order.
#[derive(Debug)]
pub(crate) struct SvgFont {
    pub(crate) units_per_em: f64,
    /// `ascent` and `descent` in font units; `descent` as a distance below the baseline.
    pub(crate) ascent: Option<f64>,
    pub(crate) descent: Option<f64>,
    pub(crate) cap_height: Option<f64>,
    /// Advance (font units) and outline (em, y down) per glyph.
    pub(crate) glyphs: Vec<(f64, Arc<GlyphOutline>)>,
    by_char: BTreeMap<char, u32>,
    kerning: BTreeMap<(u32, u32), f64>,
}

impl SvgFont {
    /// The glyph of `c`, or 0 (`<missing-glyph>`).
    pub(crate) fn glyph_of(&self, c: char) -> u32 {
        self.by_char.get(&c).copied().unwrap_or(0)
    }

    /// The kerning between `first` and `second` in font units (subtracted from the advance).
    pub(crate) fn kerning(&self, first: u32, second: u32) -> f64 {
        self.kerning.get(&(first, second)).copied().unwrap_or(0.0)
    }
}

/// Whether `bytes` look like an SVG font (the first non-blank character after an optional BOM is
/// `<`): the store sends them here instead of to the OpenType reader.
pub(crate) fn looks_like_xml(bytes: &[u8]) -> bool {
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    body.iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'<')
}

/// Whether a DOCTYPE in the prolog of `bytes` has an internal subset (a `[` before the
/// DOCTYPE's own `>`, outside quotes): such a file is refused, so no entity declaration reaches
/// the parser. The prolog is read as the XML reader reads it: the XML declaration, processing
/// instructions, comments and whitespace before the root element are skipped, so a decoy
/// `<!DOCTYPE` inside a comment or an instruction hides nothing.
fn has_internal_subset(bytes: &[u8]) -> bool {
    let skip_past = |from: usize, end: &[u8]| {
        bytes
            .get(from..)
            .and_then(|rest| rest.windows(end.len()).position(|w| w == end))
            .map(|at| from + at + end.len())
    };
    let mut at = usize::from(bytes.starts_with(b"\xEF\xBB\xBF")) * 3;
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
            let mut end = None;
            for (i, &b) in rest.iter().enumerate().skip(9) {
                match (quote, b) {
                    (Some(q), _) if b == q => quote = None,
                    (None, b'"' | b'\'') => quote = Some(b),
                    (None, b'[') => return true,
                    (None, b'>') => {
                        end = Some(at + i + 1);
                        break;
                    }
                    _ => {}
                }
            }
            end
        } else {
            return false;
        };
        match next {
            Some(next) => at = next,
            None => return false,
        }
    }
}

/// The deepest element nesting of `bytes`, counted as the XML reader would recurse, stopping
/// as soon as it passes [`MAX_SVG_DEPTH`]. Comments, CDATA, processing instructions,
/// declarations and quoted attribute values are skipped.
fn too_deep(bytes: &[u8]) -> bool {
    let skip_to = |from: usize, end: &[u8]| {
        bytes[from..]
            .windows(end.len())
            .position(|w| w == end)
            .map_or(bytes.len(), |at| from + at + end.len())
    };
    let mut depth = 0_usize;
    let mut at = 0;
    while let Some(offset) = bytes[at..].iter().position(|b| *b == b'<') {
        at += offset;
        let rest = &bytes[at..];
        if rest.starts_with(b"<!--") {
            at = skip_to(at + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            at = skip_to(at + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            at = skip_to(at + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            at = skip_to(at + 2, b">");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            at = skip_to(at + 2, b">");
        } else {
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
                depth += 1;
                if depth > MAX_SVG_DEPTH {
                    return true;
                }
            }
            at = i + 1;
        }
        if at >= bytes.len() {
            break;
        }
    }
    false
}

/// A finite number within [`MAX_SVG_COORDINATE`] from an attribute; `Err` for a stated value
/// that is not one, `Ok(None)` when absent.
fn number(node: Node<'_, '_>, name: &str) -> Result<Option<f64>, ()> {
    let Some(text) = node.attribute(name) else {
        return Ok(None);
    };
    let value: f64 = text.trim().parse().map_err(|_| ())?;
    if value.is_finite() && value.abs() <= MAX_SVG_COORDINATE {
        Ok(Some(value))
    } else {
        Err(())
    }
}

/// The style name from `font-style` and `font-weight`: Regular, Bold, Italic, Bold Italic.
fn style_of(face: Option<Node<'_, '_>>) -> String {
    let attribute = |name| {
        face.and_then(|f| f.attribute(name))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
    };
    let weight = attribute("font-weight");
    let bold =
        weight == "bold" || weight == "bolder" || weight.parse::<u32>().is_ok_and(|w| w >= 600);
    let slant = attribute("font-style");
    let italic = slant.contains("italic") || slant.contains("oblique");
    FaceStyle {
        weight: if bold { 700 } else { 400 },
        width: NORMAL_WIDTH,
        slant: if italic {
            Slant::Italic
        } else {
            Slant::Upright
        },
    }
    .name()
}

/// Parses the bytes' XML after the byte checks; `None` for anything refused.
fn parse_document(bytes: &[u8]) -> Option<Document<'_>> {
    if bytes.len() > MAX_SVG_FONT_BYTES || has_internal_subset(bytes) || too_deep(bytes) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let options = ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_SVG_NODES,
    };
    Document::parse_with_options(text, options).ok()
}

/// The `<font>` elements of the document in order.
fn fonts<'a, 'input>(doc: &'a Document<'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    doc.descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "font")
}

/// Whether `font` has a `<glyph>` with a `d` (it is a face, criterion 1).
fn has_drawn_glyph(font: Node<'_, '_>) -> bool {
    font.children()
        .any(|n| n.tag_name().name() == "glyph" && n.attribute("d").is_some())
}

fn font_face<'a, 'input>(font: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    font.children()
        .find(|n| n.is_element() && n.tag_name().name() == "font-face")
}

/// The family of `font`: `font-face font-family`, else the font's `id`.
fn family_of(font: Node<'_, '_>) -> Option<String> {
    font_face(font)
        .and_then(|face| face.attribute("font-family"))
        .or_else(|| font.attribute("id"))
        .map(|name| name.trim().trim_matches(['"', '\'']).trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// The faces of an SVG font file: `(index of the <font>, family, style)` for every `<font>` that
/// holds a `<glyph>` with a `d` and parses in full. Empty for a file that is not one.
pub(crate) fn svg_faces(bytes: &[u8]) -> Vec<(u32, String, String)> {
    let Some(doc) = parse_document(bytes) else {
        return Vec::new();
    };
    fonts(&doc)
        .enumerate()
        .filter_map(|(index, font)| {
            let index = u32::try_from(index).ok()?;
            read_font(font)?;
            Some((index, family_of(font)?, style_of(font_face(font))))
        })
        .collect()
}

/// Parses `<font>` number `index` of `bytes`.
pub(crate) fn parse_svg_font(bytes: &[u8], index: u32) -> Option<SvgFont> {
    let doc = parse_document(bytes)?;
    let font = fonts(&doc).nth(usize::try_from(index).ok()?)?;
    read_font(font)
}

fn read_font(font: Node<'_, '_>) -> Option<SvgFont> {
    if !has_drawn_glyph(font) {
        return None;
    }
    let face = font_face(font);
    let face_number = |name| face.map_or(Ok(None), |f| number(f, name));
    let units_per_em = face_number("units-per-em")
        .ok()?
        .unwrap_or(DEFAULT_UNITS_PER_EM);
    if !(16.0..=16_384.0).contains(&units_per_em) {
        return None;
    }
    let default_advance = number(font, "horiz-adv-x").ok()?.unwrap_or(units_per_em);
    let to_em = Affine::scale_non_uniform(1.0 / units_per_em, -1.0 / units_per_em);
    let tolerance = ARC_TOLERANCE_EM * units_per_em;
    let mut total_points = 0_usize;
    let mut outline_of = |node: Node<'_, '_>| -> Option<(f64, Arc<GlyphOutline>)> {
        let advance = number(node, "horiz-adv-x").ok()?.unwrap_or(default_advance);
        let path = match node.attribute("d") {
            Some(d) => {
                let left = MAX_SVG_FONT_POINTS.saturating_sub(total_points);
                let path = parse_path_data(d, tolerance, MAX_GLYPH_POINTS.min(left)).ok()?;
                total_points += path
                    .elements()
                    .iter()
                    .map(|e| match e {
                        kurbo::PathEl::CurveTo(..) => 3,
                        kurbo::PathEl::QuadTo(..) => 2,
                        kurbo::PathEl::ClosePath => 0,
                        _ => 1,
                    })
                    .sum::<usize>();
                to_em * path
            }
            None => BezPath::new(),
        };
        Some((advance, Arc::new(GlyphOutline { path })))
    };
    let missing = font
        .children()
        .find(|n| n.tag_name().name() == "missing-glyph");
    let mut glyphs = vec![match missing {
        Some(node) => outline_of(node)?,
        None => (default_advance, Arc::new(GlyphOutline::default())),
    }];
    let mut by_char = BTreeMap::new();
    let mut by_name: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for node in font.children().filter(|n| n.tag_name().name() == "glyph") {
        if glyphs.len() > MAX_SVG_GLYPHS {
            return None;
        }
        let id = u32::try_from(glyphs.len()).ok()?;
        glyphs.push(outline_of(node)?);
        let mut chars = node.attribute("unicode").unwrap_or_default().chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            by_char.entry(c).or_insert(id);
        }
        if let Some(name) = node.attribute("glyph-name") {
            for name in name.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                by_name.entry(name).or_default().push(id);
            }
        }
    }
    let kerning = read_kerning(font, &by_char, &by_name)?;
    Some(SvgFont {
        units_per_em,
        ascent: face_number("ascent").ok()?,
        descent: face_number("descent").ok()?.map(f64::abs),
        cap_height: face_number("cap-height").ok()?,
        glyphs,
        by_char,
        kerning,
    })
}

/// The glyphs a `u` (characters, comma list; ranges ignored) and `g` (glyph names, comma list)
/// pair names.
fn named_glyphs(
    chars: Option<&str>,
    names: Option<&str>,
    by_char: &BTreeMap<char, u32>,
    by_name: &BTreeMap<&str, Vec<u32>>,
) -> Vec<u32> {
    let mut ids: Vec<u32> = Vec::new();
    for item in chars.unwrap_or_default().split(',') {
        let item = item.trim();
        let mut it = item.chars();
        if let (Some(c), None) = (it.next(), it.next())
            && let Some(id) = by_char.get(&c)
        {
            ids.push(*id);
        }
    }
    for name in names.unwrap_or_default().split(',').map(str::trim) {
        if let Some(found) = by_name.get(name) {
            ids.extend(found);
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn read_kerning(
    font: Node<'_, '_>,
    by_char: &BTreeMap<char, u32>,
    by_name: &BTreeMap<&str, Vec<u32>>,
) -> Option<BTreeMap<(u32, u32), f64>> {
    let mut kerning = BTreeMap::new();
    let mut pairs = 0_usize;
    for node in font.children().filter(|n| n.tag_name().name() == "hkern") {
        let k = number(node, "k").ok()?.unwrap_or(0.0);
        let first = named_glyphs(node.attribute("u1"), node.attribute("g1"), by_char, by_name);
        let second = named_glyphs(node.attribute("u2"), node.attribute("g2"), by_char, by_name);
        pairs = pairs.saturating_add(first.len().saturating_mul(second.len()));
        if pairs > MAX_SVG_KERN_PAIRS {
            return None;
        }
        for a in &first {
            for b in &second {
                kerning.entry((*a, *b)).or_insert(k);
            }
        }
    }
    Some(kerning)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_checks() {
        assert!(looks_like_xml(b"\xEF\xBB\xBF \n<svg/>"));
        assert!(!looks_like_xml(b"\x00\x01\x00\x00"));
        assert!(!has_internal_subset(
            br#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://x/svg11.dtd" ><svg/>"#
        ));
        assert!(has_internal_subset(
            br#"<!DOCTYPE svg [<!ENTITY a "b">]><svg/>"#
        ));
        assert!(has_internal_subset(
            br#"<!DOCTYPE svg PUBLIC "a>b" [ ]><svg/>"#
        ));
        assert!(has_internal_subset(
            br#"<?xml version="1.0"?><!-- <!DOCTYPE svg> --><!DOCTYPE svg [ ]><svg/>"#
        ));
        assert!(!has_internal_subset(
            b"<svg><!-- <!DOCTYPE svg [ ]> --></svg>"
        ));
        let deep = "<a>".repeat(MAX_SVG_DEPTH + 1);
        assert!(too_deep(deep.as_bytes()));
        let wide = "<a/><b x='<>'></b>".repeat(10_000);
        assert!(!too_deep(format!("<svg>{wide}</svg>").as_bytes()));
    }
}
