//! Which listed face an imported text asks for (`specs/0024-svg-import-export` criteria 80 to 82):
//! the `font-family` list against the families, full names and PostScript names of the listed
//! faces, block by block; the face of a family by italic state, width and weight (`0078` rules);
//! a family that no block lists is a missing face that keeps the file's name.

use std::cell::RefCell;
use std::collections::BTreeMap;

use curvyo_svg_core::{FontFamily, TextStyle};
use curvyo_text_core::FaceStyle;

use crate::family_key::family_key;
use crate::font_match::{FontBlock, ListedFace};

/// The included family the generic `sans-serif`, `system-ui`, `monospace` and `fantasy` name.
pub const GENERIC_SANS: &str = "Source Sans 3";
/// The included family `serif` names.
pub const GENERIC_SERIF: &str = "Source Serif 4";
/// The included family `cursive` names.
pub const GENERIC_CURSIVE: &str = "Great Vibes";

/// The face a text style resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgFace {
    /// The listed face of this index.
    Listed(usize),
    /// No named entry matched (criterion 82): the first named family and the requested style
    /// name, drawn in the substitute face.
    Missing {
        /// The first named family of the list.
        family: String,
        /// The style name made from the requested weight and italic state.
        style: String,
    },
}

/// What [`svg_face`] reads of a style: the family list, weight, italic state and width.
type FaceQuery = (Vec<FontFamily>, u16, bool, u16);

/// [`svg_face`] over one import: each distinct family list and style is matched once (a file of
/// 100,000 texts names a handful of faces).
pub(crate) struct FaceMemo<'a> {
    faces: &'a [ListedFace],
    seen: RefCell<BTreeMap<FaceQuery, SvgFace>>,
}

impl<'a> FaceMemo<'a> {
    pub(crate) fn new(faces: &'a [ListedFace]) -> Self {
        Self {
            faces,
            seen: RefCell::new(BTreeMap::new()),
        }
    }

    /// The face `style` asks for, as [`svg_face`].
    pub(crate) fn face(&self, style: &TextStyle) -> SvgFace {
        let query = (
            style.families.clone(),
            style.weight,
            style.italic,
            style.stretch,
        );
        if let Some(face) = self.seen.borrow().get(&query) {
            return face.clone();
        }
        let face = svg_face(style, self.faces);
        self.seen.borrow_mut().insert(query, face.clone());
        face
    }
}

/// The full name a listing implies: "Family Style", or the family alone for Regular.
fn full_name(face: &ListedFace) -> String {
    if face.style.eq_ignore_ascii_case("regular") {
        face.family.clone()
    } else {
        format!("{} {}", face.family, face.style)
    }
}

/// The PostScript name a listing implies: "FamilyNoSpaces-StyleNoSpaces" (Illustrator's
/// `MyriadPro-Regular`).
fn postscript_name(face: &ListedFace) -> String {
    let squeeze = |s: &str| s.split_whitespace().collect::<String>();
    format!("{}-{}", squeeze(&face.family), squeeze(&face.style))
}

/// The face `style` asks for among `faces` (criteria 80 to 82): the first entry that matches
/// wins; a list whose named entries all fail is missing even with a generic entry after them.
#[must_use]
pub fn svg_face(style: &TextStyle, faces: &[ListedFace]) -> SvgFace {
    let found: Vec<(bool, Option<usize>)> = style
        .families
        .iter()
        .map(|entry| match entry {
            FontFamily::Named(name) => (true, named_face(style, faces, name)),
            FontFamily::SansSerif => (false, generic_face(style, faces, GENERIC_SANS)),
            FontFamily::Serif => (false, generic_face(style, faces, GENERIC_SERIF)),
            FontFamily::Cursive => (false, generic_face(style, faces, GENERIC_CURSIVE)),
        })
        .collect();
    let named_fail = found.iter().any(|(named, _)| *named)
        && found.iter().all(|(named, at)| !named || at.is_none());
    if !named_fail && let Some(at) = found.iter().find_map(|(_, at)| *at) {
        return SvgFace::Listed(at);
    }
    let family = style
        .families
        .iter()
        .find_map(|f| match f {
            FontFamily::Named(name) => Some(name.clone()),
            _ => None,
        })
        .unwrap_or_else(|| GENERIC_SANS.to_owned());
    let requested = FaceStyle {
        weight: style.weight,
        width: curvyo_text_core::NORMAL_WIDTH,
        slant: if style.italic {
            curvyo_text_core::Slant::Italic
        } else {
            curvyo_text_core::Slant::Upright
        },
    };
    SvgFace::Missing {
        family,
        style: requested.name(),
    }
}

/// The face of the family, full name or PostScript name `name`, block by block.
fn named_face(style: &TextStyle, faces: &[ListedFace], name: &str) -> Option<usize> {
    let key = family_key(name);
    let lower = name.to_lowercase();
    for block in [FontBlock::Curvyo, FontBlock::Included, FontBlock::System] {
        let family: Vec<usize> = faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.block == block && family_key(&f.family) == key)
            .map(|(i, _)| i)
            .collect();
        if !family.is_empty() {
            return Some(pick(style, faces, &family));
        }
        if let Some(index) = faces.iter().position(|f| {
            f.block == block
                && (full_name(f).to_lowercase() == lower
                    || postscript_name(f).to_lowercase() == lower)
        }) {
            return Some(index);
        }
    }
    None
}

/// The face of the included family `family` a generic name stands for.
fn generic_face(style: &TextStyle, faces: &[ListedFace], family: &str) -> Option<usize> {
    let key = family_key(family);
    let members: Vec<usize> = faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.block == FontBlock::Included && family_key(&f.family) == key)
        .map(|(i, _)| i)
        .collect();
    (!members.is_empty()).then(|| pick(style, faces, &members))
}

/// The face of a family (the listed indices `family`, one block) for `style` (criterion 81):
/// the italic state first, then the nearest width, then the nearest weight (the lighter on a tie
/// for a request of 500 or less, else the heavier), then a loaded face, then the listing order.
fn pick(style: &TextStyle, faces: &[ListedFace], family: &[usize]) -> usize {
    let styled: Vec<(usize, FaceStyle)> = family
        .iter()
        .map(|&i| {
            (
                i,
                FaceStyle::parse(&faces[i].style).unwrap_or(FaceStyle::REGULAR),
            )
        })
        .collect();
    let same_slant: Vec<&(usize, FaceStyle)> = styled
        .iter()
        .filter(|(_, s)| s.is_slanted() == style.italic)
        .collect();
    let pool: Vec<&(usize, FaceStyle)> = if same_slant.is_empty() {
        styled.iter().collect()
    } else {
        same_slant
    };
    let want_width = style.stretch;
    let narrower_first = want_width <= curvyo_text_core::NORMAL_WIDTH;
    let width_rank = |w: u16| {
        let distance = w.abs_diff(want_width);
        let wrong_side = if narrower_first {
            w > want_width
        } else {
            w < want_width
        };
        (distance, wrong_side)
    };
    let best_width = pool.iter().map(|(_, s)| width_rank(s.width)).min();
    let want = style.weight;
    let heavier_on_tie = want > 500;
    pool.iter()
        .filter(|(_, s)| Some(width_rank(s.width)) == best_width)
        .min_by_key(|(i, s)| {
            let distance = s.weight.abs_diff(want);
            let tie = if heavier_on_tie {
                s.weight < want
            } else {
                s.weight > want
            };
            (distance, tie, faces[*i].key.is_none(), *i)
        })
        .map_or(family[0], |(i, _)| *i)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use curvyo_document_core::FontKind;
    use curvyo_svg_core::{
        Baseline, BaselineShift, FlowAlign, LeafStyle, LineHeight, SvgOrientation, SvgWritingMode,
        TextAnchor,
    };

    pub(crate) fn style_of(families: Vec<FontFamily>, weight: u16, italic: bool) -> TextStyle {
        TextStyle {
            families,
            weight,
            italic,
            stretch: 5,
            font_size: 16.0,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            anchor: TextAnchor::Start,
            writing_mode: SvgWritingMode::Horizontal,
            orientation: SvgOrientation::Auto,
            line_height: LineHeight::Normal,
            baseline: Baseline::Alphabetic,
            baseline_shift: BaselineShift::None,
            flow_align: FlowAlign::Start,
            paint: LeafStyle {
                fill: None,
                even_odd: false,
                stroke: None,
                stroke_width_mm: None,
                dash: Vec::new(),
                cap: None,
                join: None,
            },
        }
    }

    fn listed(block: FontBlock, family: &str, style: &str) -> ListedFace {
        ListedFace {
            block,
            family: family.into(),
            style: style.into(),
            host_id: format!("{family}-{style}"),
            index: 0,
            key: None,
            kind: FontKind::Outline,
        }
    }

    fn sans() -> Vec<ListedFace> {
        [
            "Regular",
            "Bold",
            "Italic",
            "Bold Italic",
            "Light",
            "Light Italic",
            "Extra Light",
        ]
        .iter()
        .map(|s| listed(FontBlock::Included, "Source Sans 3", s))
        .chain([
            listed(FontBlock::Included, "Source Serif 4", "Regular"),
            listed(FontBlock::Included, "Great Vibes", "Regular"),
            listed(FontBlock::System, "Myriad Pro", "Regular"),
            listed(FontBlock::System, "Myriad Pro", "Bold"),
        ])
        .collect()
    }

    fn named(name: &str) -> FontFamily {
        FontFamily::Named(name.into())
    }

    /// Criterion 81's tests: 600 picks Bold, 300 italic picks Light Italic.
    #[test]
    fn ac81_nearest_face_of_the_family() {
        let faces = sans();
        let pick = |w, i| match svg_face(&style_of(vec![named("source sans 3")], w, i), &faces) {
            SvgFace::Listed(at) => faces[at].style.clone(),
            other @ SvgFace::Missing { .. } => panic!("{other:?}"),
        };
        assert_eq!(pick(600, false), "Bold");
        assert_eq!(pick(300, true), "Light Italic");
        assert_eq!(pick(400, true), "Italic");
        assert_eq!(pick(200, false), "Extra Light");
        assert_eq!(
            pick(550, false),
            "Bold",
            "a tie above 500 takes the heavier"
        );
        assert_eq!(
            pick(250, false),
            "Extra Light",
            "a tie at 500 or less takes the lighter"
        );
    }

    /// Criterion 80: the list in order, generic names, PostScript and full names.
    #[test]
    fn ac80_family_lists_and_names() {
        let faces = sans();
        let face = |families| match svg_face(&style_of(families, 400, false), &faces) {
            SvgFace::Listed(at) => (faces[at].family.clone(), faces[at].style.clone()),
            other @ SvgFace::Missing { .. } => panic!("{other:?}"),
        };
        assert_eq!(face(vec![FontFamily::Serif]).0, "Source Serif 4");
        assert_eq!(face(vec![FontFamily::Cursive]).0, "Great Vibes");
        assert_eq!(face(vec![FontFamily::SansSerif]).0, "Source Sans 3");
        assert_eq!(
            face(vec![named("MyriadPro-Bold")]),
            ("Myriad Pro".into(), "Bold".into())
        );
        assert_eq!(
            face(vec![named("Myriad Pro Bold")]),
            ("Myriad Pro".into(), "Bold".into())
        );
    }

    /// Criterion 82: named entries that all fail give a missing face named after the first one,
    /// also with a generic entry after it.
    #[test]
    fn ac82_missing_keeps_the_file_name() {
        let faces = sans();
        assert_eq!(
            svg_face(
                &style_of(vec![named("Lobster"), FontFamily::Cursive], 700, true),
                &faces
            ),
            SvgFace::Missing {
                family: "Lobster".into(),
                style: "Bold Italic".into()
            }
        );
        assert!(matches!(
            svg_face(
                &style_of(vec![named("Nope"), FontFamily::Serif], 400, false),
                &faces
            ),
            SvgFace::Missing { .. }
        ));
        assert!(matches!(
            svg_face(&style_of(vec![FontFamily::Cursive], 400, false), &faces),
            SvgFace::Listed(_)
        ));
    }
}
