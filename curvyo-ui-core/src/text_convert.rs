//! The one conversion of text objects into paths (`specs/0074-text-objects` criteria 22, 24, 29):
//! Object to path makes a group of one compound path per glyph; the path commands take a text
//! as the compound path of all its glyphs. A text drawn with a substitute face, or not laid out
//! yet, is refused, never converted to the wrong letters or to nothing. Commands that work on
//! areas also refuse single-line texts (`specs/0076-single-line-fonts` criterion 19).

use std::borrow::Cow;

use curvyo_document_core::{
    FontKind, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, TextConversion, TextSnapshot,
};

use crate::anchor_id_minter::AnchorIdMinter;

/// Why texts could not be converted; nothing was changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRefusal {
    /// The texts to blame, drawn with the refusal outline.
    pub offenders: Vec<NodeId>,
    /// The stored names of the missing faces, as "Source Serif 4 Bold", one per face.
    pub missing_faces: Vec<String>,
    /// Some offender is not laid out yet (its face is still loading).
    pub loading: bool,
    /// The offenders are single-line texts, which have no area (`0076` criterion 19).
    pub no_area: bool,
}

/// The compound path of all glyphs of `text`, or `None` when it uses a substitute face or is not
/// laid out (criterion 29).
#[must_use]
pub fn text_path(text: &TextSnapshot) -> Option<&PathSnapshot> {
    text.outline
        .as_deref()
        .filter(|outline| !outline.substitute)
        .map(|outline| &outline.glyphs)
}

/// The refusal for the texts among `ids`, or `None` when every one converts. All or nothing:
/// one text that cannot be converted refuses the whole command (UX notes §4).
#[must_use]
pub fn texts_refusal(objects: &[ObjectSnapshot], ids: &[NodeId]) -> Option<TextRefusal> {
    let mut refusal = TextRefusal {
        offenders: Vec::new(),
        missing_faces: Vec::new(),
        loading: false,
        no_area: false,
    };
    for object in objects.iter().filter(|o| ids.contains(&o.id())) {
        let ObjectSnapshot::Text(text) = object else {
            continue;
        };
        if text_path(text).is_some() {
            continue;
        }
        refusal.offenders.push(text.id);
        if text.outline.is_none() {
            refusal.loading = true;
        } else {
            let face = format!("{} {}", text.font.family, text.font.style);
            if !refusal.missing_faces.contains(&face) {
                refusal.missing_faces.push(face);
            }
        }
    }
    (!refusal.offenders.is_empty()).then_some(refusal)
}

/// The refusal of a command that works on areas (the Boolean operations, Fracture, Flatten,
/// Weld, Offset) for the texts among `ids`: a missing or loading face first ([`texts_refusal`]),
/// then every single-line text, by its stored kind. All or nothing.
#[must_use]
pub fn area_refusal(objects: &[ObjectSnapshot], ids: &[NodeId]) -> Option<TextRefusal> {
    if let Some(refusal) = texts_refusal(objects, ids) {
        return Some(refusal);
    }
    let offenders: Vec<NodeId> = objects
        .iter()
        .filter(|o| ids.contains(&o.id()))
        .filter_map(|object| match object {
            ObjectSnapshot::Text(text) if text.font.kind == FontKind::SingleLine => Some(text.id),
            _ => None,
        })
        .collect();
    (!offenders.is_empty()).then_some(TextRefusal {
        offenders,
        missing_faces: Vec::new(),
        loading: false,
        no_area: true,
    })
}

/// `objects` with every convertible text replaced by the compound path of its glyphs, under the
/// text's id, so a path command plans and writes it like a path (criterion 24); the write
/// deletes the text node in the command's own step. Texts that do not convert stay (the caller
/// refused them with [`texts_refusal`] first).
#[must_use]
pub fn texts_as_paths(objects: &[ObjectSnapshot]) -> Cow<'_, [ObjectSnapshot]> {
    if !objects.iter().any(|o| matches!(o, ObjectSnapshot::Text(_))) {
        return Cow::Borrowed(objects);
    }
    Cow::Owned(
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Text(text) => text_path(text)
                    .map_or_else(|| object.clone(), |path| ObjectSnapshot::Path(path.clone())),
                other => other.clone(),
            })
            .collect(),
    )
}

/// Object to path for the texts among `ids` (criterion 22): per text a group named by its name
/// or its first 24 characters, one compound path per drawn glyph with fresh anchor ids.
///
/// # Errors
/// The [`TextRefusal`] when a text does not convert.
pub fn build_text_conversions(
    objects: &[ObjectSnapshot],
    ids: &[NodeId],
    names: &dyn Fn(NodeId) -> Option<String>,
    minter: &mut AnchorIdMinter,
) -> Result<Vec<TextConversion>, TextRefusal> {
    if let Some(refusal) = texts_refusal(objects, ids) {
        return Err(refusal);
    }
    Ok(objects
        .iter()
        .filter(|o| ids.contains(&o.id()))
        .filter_map(|object| match object {
            ObjectSnapshot::Text(text) => Some(text),
            _ => None,
        })
        .filter_map(|text| {
            let outline = text.outline.as_deref()?;
            let path = &outline.glyphs;
            let mut subpaths = path.subpaths();
            let glyphs = outline
                .glyph_outlines
                .iter()
                .map(|&count| {
                    subpaths
                        .by_ref()
                        .take(count)
                        .map(|outline| {
                            let anchors: Vec<NewAnchor> = outline
                                .anchors
                                .iter()
                                .map(|anchor| NewAnchor {
                                    id: minter.mint(),
                                    ..*anchor
                                })
                                .collect();
                            (anchors, outline.closed)
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|glyph: &Vec<_>| !glyph.is_empty())
                .collect();
            Some(TextConversion {
                id: text.id,
                name: names(text.id).unwrap_or_else(|| text.default_name()),
                glyphs,
            })
        })
        .collect())
}
