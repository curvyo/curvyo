//! Which listed face a text's font reference names (ADR 0020 §1, "Font matching";
//! `specs/0074-text-objects` criteria 29 and 30): only faces of the text's stored kind count
//! (`specs/0076-single-line-fonts/adrs.md` decision 1).

use curvyo_document_core::{FontKind, FontRef};
use curvyo_text_core::FaceKey;

use crate::family_key::family_key;

/// The block of the Fonts tab a face is listed in, in the order of preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontBlock {
    /// The Curvyo font folder.
    Curvyo,
    /// Shipped with the app.
    Included,
    /// Installed on the computer (desktop only).
    System,
}

/// One face the host listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedFace {
    /// Its block.
    pub block: FontBlock,
    /// The family name.
    pub family: String,
    /// The style name.
    pub style: String,
    /// The host's id of its file.
    pub host_id: String,
    /// The face's index in its file.
    pub index: u32,
    /// Its key in the engine's store once its bytes were handed in.
    pub key: Option<FaceKey>,
    /// Outline or single-line, as the host listed it.
    pub kind: FontKind,
}

/// How a font reference resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceMatch {
    /// A loaded face: the stored one (same hash), or one of the same family and style.
    Face(FaceKey),
    /// A listed face of the same family and style whose bytes are not loaded yet: the host loads
    /// the listed face of this index, then the text is laid out again.
    NeedsLoad(usize),
    /// No face matches: the text is drawn with the included sans face and marked (criterion 29).
    Missing,
}

/// Resolves `font` over the faces of its kind among `faces`: (a) a loaded face with the stored
/// hash and index; (b) a face of
/// the same family (by [`family_key`]) and style, case-insensitive, preferring the Curvyo folder, then the included
/// fonts, then the computer's, without a notice (criterion 30); (c) missing.
#[must_use]
pub fn match_face(font: &FontRef, faces: &[ListedFace]) -> FaceMatch {
    let exact = faces
        .iter()
        .filter(|face| face.kind == font.kind)
        .find_map(|face| {
            face.key
                .filter(|key| key.index == font.face_index && key.hash.to_hex() == font.face_hash)
        });
    if let Some(key) = exact {
        return FaceMatch::Face(key);
    }
    let same_name = |face: &&ListedFace| {
        face.kind == font.kind
            && family_key(&face.family) == family_key(&font.family)
            && face.style.to_lowercase() == font.style.to_lowercase()
    };
    let best = faces
        .iter()
        .enumerate()
        .filter(|(_, face)| same_name(face))
        .min_by_key(|(_, face)| (face.block, face.key.is_none()));
    match best {
        Some((_, ListedFace { key: Some(key), .. })) => FaceMatch::Face(*key),
        Some((index, _)) => FaceMatch::NeedsLoad(index),
        None => FaceMatch::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_text_core::FaceHash;

    fn listed(block: FontBlock, family: &str, style: &str, hash: Option<u8>) -> ListedFace {
        ListedFace {
            block,
            family: family.to_owned(),
            style: style.to_owned(),
            host_id: format!("{block:?}{family}"),
            index: 0,
            key: hash.map(|h| FaceKey {
                hash: FaceHash([h; 32]),
                index: 0,
            }),
            kind: FontKind::Outline,
        }
    }

    fn font(family: &str, style: &str, hash: u8) -> FontRef {
        FontRef {
            family: family.to_owned(),
            style: style.to_owned(),
            face_hash: FaceHash([hash; 32]).to_hex(),
            face_index: 0,
            kind: FontKind::Outline,
        }
    }

    /// Criteria 29 and 30: the stored face first; else the same family and style, Curvyo
    /// before Included before This computer, without a notice; else missing.
    #[test]
    fn ac29_ac30_matching_rule() {
        let faces = vec![
            listed(FontBlock::System, "Source Serif 4", "Bold", Some(3)),
            listed(FontBlock::Included, "Source Serif 4", "Bold", Some(2)),
            listed(FontBlock::Curvyo, "source serif 4", "bold", None),
        ];
        let key = |h| {
            FaceMatch::Face(FaceKey {
                hash: FaceHash([h; 32]),
                index: 0,
            })
        };
        assert_eq!(
            match_face(&font("Source Serif 4", "Bold", 3), &faces),
            key(3)
        );
        assert_eq!(
            match_face(&font("Source Serif 4", "Bold", 9), &faces),
            FaceMatch::NeedsLoad(2)
        );
        assert_eq!(
            match_face(&font("Source Serif 4", "Bold", 9), &faces[..2]),
            key(2)
        );
        assert_eq!(
            match_face(&font("Great Vibes", "Regular", 1), &faces),
            FaceMatch::Missing
        );
    }

    /// `specs/0076-single-line-fonts` criterion 22: a face of the other kind never matches, by
    /// hash or by name.
    #[test]
    fn ac22_matching_is_kind_aware() {
        let mut single = listed(
            FontBlock::Curvyo,
            "Hershey Sans 1-stroke",
            "Regular",
            Some(5),
        );
        single.kind = FontKind::SingleLine;
        let outline = listed(
            FontBlock::Curvyo,
            "Hershey Sans 1-stroke",
            "Regular",
            Some(6),
        );
        let mut text = font("Hershey Sans 1-stroke", "Regular", 6);
        text.kind = FontKind::SingleLine;
        assert_eq!(
            match_face(&text, std::slice::from_ref(&outline)),
            FaceMatch::Missing
        );
        assert!(matches!(
            match_face(&text, &[outline, single]),
            FaceMatch::Face(FaceKey {
                hash: FaceHash([5, ..]),
                ..
            })
        ));
    }
}
