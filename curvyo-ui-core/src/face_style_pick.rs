//! Which face of a family a weight step, the italic toggle or a family pick gives a text
//! (`specs/0078-font-style-buttons` criteria 3 to 13 and 22 to 28, `adrs.md` decision 2).

use std::collections::BTreeSet;

use curvyo_document_core::{FontKind, FontRef};
use curvyo_text_core::{FaceStyle, Slant, WeightBand};

use crate::family_key::family_key;
use crate::font_match::{FaceMatch, FontBlock, ListedFace, match_face};

/// The weight buttons in their order.
pub const WEIGHT_STEPS: [WeightBand; 5] = [
    WeightBand::ExtraLight,
    WeightBand::Light,
    WeightBand::Regular,
    WeightBand::Bold,
    WeightBand::ExtraBold,
];

/// A family in the listing: its block, family key and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Family {
    block: FontBlock,
    key: String,
    kind: FontKind,
}

/// A text's face as the buttons see it.
#[derive(Debug, Clone)]
struct Subject {
    /// Its family in the listing; `None` when no block lists the family (criterion 9).
    family: Option<Family>,
    /// The family name as the maker reads it.
    name: String,
    style: FaceStyle,
}

fn style_of(name: &str) -> FaceStyle {
    FaceStyle::parse(name).unwrap_or(FaceStyle::REGULAR)
}

fn family_of(face: &ListedFace) -> Family {
    Family {
        block: face.block,
        key: family_key(&face.family),
        kind: face.kind,
    }
}

/// The listed face `font` resolves to; for a missing face, the family of the stored name in the
/// first block that lists it, with the stored style (criterion 9).
fn subject(faces: &[ListedFace], font: &FontRef) -> Subject {
    let listed = match match_face(font, faces) {
        FaceMatch::Face(key) => faces
            .iter()
            .position(|face| face.key == Some(key) && face.kind == font.kind),
        FaceMatch::NeedsLoad(index) => Some(index),
        FaceMatch::Missing => None,
    };
    if let Some(face) = listed.and_then(|index| faces.get(index)) {
        return Subject {
            family: Some(family_of(face)),
            name: face.family.clone(),
            style: style_of(&face.style),
        };
    }
    let key = family_key(&font.family);
    let family = faces
        .iter()
        .filter(|face| face.kind == font.kind && family_key(&face.family) == key)
        .min_by_key(|face| face.block)
        .map(family_of);
    Subject {
        family,
        name: font.family.clone(),
        style: style_of(&font.style),
    }
}

/// The face of `family` whose style `fits`, by `rank` (lower first), then a loaded face, then
/// the listing's order.
fn find(
    faces: &[ListedFace],
    family: &Family,
    fits: impl Fn(&FaceStyle) -> bool,
    rank: impl Fn(&FaceStyle) -> u8,
) -> Option<usize> {
    faces
        .iter()
        .enumerate()
        .filter(|(_, face)| family_of(face) == *family)
        .filter_map(|(index, face)| FaceStyle::parse(&face.style).map(|s| (index, face, s)))
        .filter(|(_, _, style)| fits(style))
        .min_by_key(|(index, face, style)| (rank(style), face.key.is_none(), *index))
        .map(|(index, ..)| index)
}

/// The face of `family` in `band` with the width and italic state of `like`, the same slant
/// first (the "target face" of the spec's Terms).
fn in_band(
    faces: &[ListedFace],
    family: &Family,
    band: WeightBand,
    like: FaceStyle,
) -> Option<usize> {
    find(
        faces,
        family,
        |s| s.band() == band && s.width == like.width && s.is_slanted() == like.is_slanted(),
        |s| u8::from(s.slant != like.slant),
    )
}

/// The target face of `font` for the weight `band` (criteria 10, 13): its family's face in that
/// band with its width and italic state.
#[must_use]
pub fn weight_target(faces: &[ListedFace], font: &FontRef, band: WeightBand) -> Option<usize> {
    let subject = subject(faces, font);
    in_band(faces, subject.family.as_ref()?, band, subject.style)
}

/// The italic counterpart of `font` (criterion 12): its family's face with its width and weight
/// band and the other italic state, an Italic face before an Oblique one.
#[must_use]
pub fn italic_target(faces: &[ListedFace], font: &FontRef) -> Option<usize> {
    let subject = subject(faces, font);
    let style = subject.style;
    find(
        faces,
        subject.family.as_ref()?,
        |s| {
            s.band() == style.band()
                && s.width == style.width
                && s.is_slanted() != style.is_slanted()
        },
        |s| u8::from(s.slant == Slant::Oblique),
    )
}

/// The face a press on the family of the listed face `picked` gives `font` (criterion 22): the
/// family's face in its weight band with its width and italic state, else `picked` (the family's
/// Regular or first face).
#[must_use]
pub fn family_target(faces: &[ListedFace], picked: usize, font: &FontRef) -> usize {
    let Some(face) = faces.get(picked) else {
        return picked;
    };
    let style = subject(faces, font).style;
    in_band(faces, &family_of(face), style.band(), style).unwrap_or(picked)
}

/// The face a preview tile of the family of `picked` shows (criterion 24): [`family_target`]
/// when every subject text has one style (weight band, width, italic state), else `picked`.
#[must_use]
pub fn family_tile(faces: &[ListedFace], picked: usize, fonts: &[FontRef]) -> usize {
    let key = |font: &FontRef| {
        let style = subject(faces, font).style;
        (style.band(), style.width, style.is_slanted())
    };
    match fonts.split_first() {
        Some((first, rest)) if rest.iter().all(|font| key(font) == key(first)) => {
            family_target(faces, picked, first)
        }
        _ => picked,
    }
}

/// Whether a button is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pressed {
    /// Not pressed.
    #[default]
    Off,
    /// Pressed.
    On,
    /// Some subject texts are, some are not (Italic only).
    Mixed,
}

/// Why a button is greyed: the tooltip's second line (criterion 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Greyed {
    /// "Not in Great Vibes".
    NotIn(String),
    /// "Not in Source Serif 4 as italic" (a weight button for italic texts).
    NotInAsItalic(String),
    /// "Not in Lobster as upright" (Italic while pressed).
    NotInAsUpright(String),
    /// "Not in any of the 3 fonts".
    NotInAny(usize),
    /// "Font missing": no block lists the family (criterion 9).
    FontMissing,
}

/// One button of the Style group.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleButton {
    /// Pressed, mixed or not.
    pub pressed: Pressed,
    /// Greyed, with the reason.
    pub greyed: Option<Greyed>,
}

/// The Style group: the five weight buttons of [`WEIGHT_STEPS`] and Italic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleButtons {
    /// Extra Light, Light, Regular, Bold, Extra Bold.
    pub weights: [StyleButton; 5],
    /// The Italic toggle.
    pub italic: StyleButton,
}

/// The reason for a button no text in `concerned` has a face for.
fn reason(concerned: &[&Subject], as_italic: bool, as_upright: bool) -> Greyed {
    if concerned.iter().all(|s| s.family.is_none()) {
        return Greyed::FontMissing;
    }
    let names: BTreeSet<String> = concerned.iter().map(|s| family_key(&s.name)).collect();
    match concerned.first() {
        Some(one) if names.len() == 1 => {
            let name = one.name.clone();
            if as_upright {
                Greyed::NotInAsUpright(name)
            } else if as_italic {
                Greyed::NotInAsItalic(name)
            } else {
                Greyed::NotIn(name)
            }
        }
        _ => Greyed::NotInAny(names.len()),
    }
}

/// The Style group's states for the subject texts' `fonts` (criteria 3 to 9): a weight button is
/// pressed when every text is in its band, greyed when none has a target face; Italic is pressed,
/// mixed or not, greyed when none of the texts it would change has a counterpart. Texts whose
/// family no block lists grey all six and press none.
#[must_use]
pub fn style_buttons(faces: &[ListedFace], fonts: &[FontRef]) -> StyleButtons {
    // A thousand selected texts usually share a few faces: each is resolved once (UX notes §8).
    let mut distinct: Vec<&FontRef> = Vec::new();
    for font in fonts {
        if !distinct.contains(&font) {
            distinct.push(font);
        }
    }
    let fonts: Vec<FontRef> = distinct.into_iter().cloned().collect();
    let fonts = fonts.as_slice();
    let subjects: Vec<Subject> = fonts.iter().map(|font| subject(faces, font)).collect();
    if subjects.is_empty() {
        return StyleButtons::default();
    }
    let all: Vec<&Subject> = subjects.iter().collect();
    let all_slanted = subjects.iter().all(|s| s.style.is_slanted());
    let weights = WEIGHT_STEPS.map(|band| {
        if subjects.iter().all(|s| s.family.is_none()) {
            return StyleButton {
                pressed: Pressed::Off,
                greyed: Some(Greyed::FontMissing),
            };
        }
        if subjects.iter().all(|s| s.style.band() == band) {
            return StyleButton {
                pressed: Pressed::On,
                greyed: None,
            };
        }
        let any = fonts
            .iter()
            .any(|font| weight_target(faces, font, band).is_some());
        StyleButton {
            pressed: Pressed::Off,
            greyed: (!any).then(|| reason(&all, all_slanted, false)),
        }
    });
    let slanted = subjects.iter().filter(|s| s.style.is_slanted()).count();
    let pressed = match slanted {
        0 => Pressed::Off,
        n if n == subjects.len() => Pressed::On,
        _ => Pressed::Mixed,
    };
    let changed: Vec<(&FontRef, &Subject)> = fonts
        .iter()
        .zip(&subjects)
        .filter(|(_, s)| pressed == Pressed::On || !s.style.is_slanted())
        .collect();
    let any = changed
        .iter()
        .any(|(font, _)| italic_target(faces, font).is_some());
    let concerned: Vec<&Subject> = changed.iter().map(|(_, s)| *s).collect();
    let italic = if concerned.iter().all(|s| s.family.is_none()) {
        StyleButton {
            pressed: Pressed::Off,
            greyed: Some(Greyed::FontMissing),
        }
    } else {
        StyleButton {
            pressed,
            greyed: (!any).then(|| reason(&concerned, false, pressed == Pressed::On)),
        }
    };
    StyleButtons { weights, italic }
}

/// What a press does to the subject texts (criteria 10 to 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StylePress {
    /// Per text, the listed face it gets; `None` keeps its face.
    pub targets: Vec<Option<usize>>,
    /// The texts in the pressed style afterwards.
    pub set: usize,
    /// The texts left as they were for want of a face.
    pub left: usize,
    /// Italic was pressed while every text was italic: the texts go upright.
    pub upright: bool,
}

/// A press on the weight button of `band` (criteria 10, 11, 13).
#[must_use]
pub fn press_weight(faces: &[ListedFace], fonts: &[FontRef], band: WeightBand) -> StylePress {
    let targets: Vec<Option<usize>> = fonts
        .iter()
        .map(|font| weight_target(faces, font, band))
        .collect();
    let set = targets.iter().filter(|t| t.is_some()).count();
    StylePress {
        left: targets.len() - set,
        set,
        targets,
        upright: false,
    }
}

/// A press on Italic (criterion 12): all italic go upright, else every upright text goes italic
/// and italic texts stay.
#[must_use]
pub fn press_italic(faces: &[ListedFace], fonts: &[FontRef]) -> StylePress {
    let slanted: Vec<bool> = fonts
        .iter()
        .map(|font| subject(faces, font).style.is_slanted())
        .collect();
    let upright = !slanted.is_empty() && slanted.iter().all(|s| *s);
    let targets: Vec<Option<usize>> = fonts
        .iter()
        .zip(&slanted)
        .map(|(font, slanted)| {
            if upright || !slanted {
                italic_target(faces, font)
            } else {
                None
            }
        })
        .collect();
    let left = targets
        .iter()
        .zip(&slanted)
        .filter(|(target, slanted)| target.is_none() && (upright || !**slanted))
        .count();
    StylePress {
        set: targets.len() - left,
        left,
        targets,
        upright,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_text_core::{FaceHash, FaceKey};

    fn face(family: &str, style: &str, file: &str) -> ListedFace {
        ListedFace {
            block: FontBlock::System,
            family: family.to_owned(),
            style: style.to_owned(),
            host_id: file.to_owned(),
            index: 0,
            key: None,
            kind: FontKind::Outline,
        }
    }

    fn family(name: &str, styles: &[&str]) -> Vec<ListedFace> {
        styles
            .iter()
            .map(|style| face(name, style, &format!("{name}-{style}.otf")))
            .collect()
    }

    fn font(family: &str, style: &str) -> FontRef {
        FontRef {
            family: family.to_owned(),
            style: style.to_owned(),
            face_hash: FaceHash([0; 32]).to_hex(),
            face_index: 0,
            kind: FontKind::Outline,
        }
    }

    fn style_at(faces: &[ListedFace], index: Option<usize>) -> Option<&str> {
        index.map(|i| faces[i].style.as_str())
    }

    fn greyed(buttons: &StyleButtons) -> Vec<bool> {
        buttons
            .weights
            .iter()
            .chain([&buttons.italic])
            .map(|b| b.greyed.is_some())
            .collect()
    }

    /// Criterion 28: a family with all five steps.
    #[test]
    fn ac28_all_five_steps() {
        let faces = family(
            "Inter",
            &[
                "Regular",
                "Italic",
                "Extra Light",
                "Light",
                "Bold",
                "Extra Bold",
                "Extra Bold Italic",
            ],
        );
        let text = font("Inter", "Regular");
        for (band, style) in
            WEIGHT_STEPS
                .iter()
                .zip(["Extra Light", "Light", "Regular", "Bold", "Extra Bold"])
        {
            assert_eq!(
                style_at(&faces, weight_target(&faces, &text, *band)),
                Some(style)
            );
        }
        let buttons = style_buttons(&faces, &[text]);
        assert_eq!(buttons.weights[2].pressed, Pressed::On);
        assert_eq!(greyed(&buttons), [false; 6]);
        // Criterion 13: a weight press on an italic text stays italic.
        let italic = font("Inter", "Italic");
        assert_eq!(
            style_at(
                &faces,
                weight_target(&faces, &italic, WeightBand::ExtraBold)
            ),
            Some("Extra Bold Italic")
        );
        assert_eq!(
            weight_target(&faces, &italic, WeightBand::Bold),
            None,
            "no Bold Italic"
        );
    }

    /// Criterion 28: Semibold and Black only: no step pressed, Bold and Extra Bold greyed.
    #[test]
    fn ac28_semibold_and_black_only() {
        let faces = family("Heavy", &["Semibold", "Black"]);
        let buttons = style_buttons(&faces, &[font("Heavy", "Semibold")]);
        assert!(buttons.weights.iter().all(|b| b.pressed == Pressed::Off));
        assert_eq!(greyed(&buttons), [true, true, true, true, true, true]);
        assert_eq!(
            buttons.weights[3].greyed,
            Some(Greyed::NotIn("Heavy".into()))
        );
    }

    /// Criterion 28: the same face as OTF and TTF: a loaded one wins, then the listing's order.
    #[test]
    fn ac28_otf_and_ttf_of_one_face() {
        let mut faces = vec![
            face("Dup", "Regular", "Dup-Regular.otf"),
            face("Dup", "Bold", "Dup-Bold.otf"),
            face("Dup", "Bold", "Dup-Bold.ttf"),
        ];
        let text = font("Dup", "Regular");
        assert_eq!(weight_target(&faces, &text, WeightBand::Bold), Some(1));
        faces[2].key = Some(FaceKey {
            hash: FaceHash([2; 32]),
            index: 0,
        });
        assert_eq!(weight_target(&faces, &text, WeightBand::Bold), Some(2));
    }

    /// Criterion 28: a Condensed family keeps the width.
    #[test]
    fn ac28_condensed_keeps_the_width() {
        let mut faces = family(
            "Narrow",
            &["Regular", "Bold", "Condensed", "Condensed Light"],
        );
        faces.extend(family("Other", &["Condensed Bold"]));
        let text = font("Narrow", "Condensed");
        assert_eq!(
            style_at(&faces, weight_target(&faces, &text, WeightBand::Light)),
            Some("Condensed Light")
        );
        assert_eq!(weight_target(&faces, &text, WeightBand::Bold), None);
        assert_eq!(
            style_at(
                &faces,
                weight_target(&faces, &font("Narrow", "Regular"), WeightBand::Bold)
            ),
            Some("Bold")
        );
    }

    /// Criterion 28: an Oblique-only family: Italic finds the Oblique face, and an Italic face is
    /// preferred where both exist.
    #[test]
    fn ac28_oblique_only() {
        let faces = family("Slope", &["Regular", "Oblique"]);
        let text = font("Slope", "Regular");
        assert_eq!(
            style_at(&faces, italic_target(&faces, &text)),
            Some("Oblique")
        );
        let both = family("Both", &["Regular", "Oblique", "Italic"]);
        assert_eq!(
            style_at(&both, italic_target(&both, &font("Both", "Regular"))),
            Some("Italic")
        );
        let buttons = style_buttons(&faces, &[font("Slope", "Oblique")]);
        assert_eq!(buttons.italic.pressed, Pressed::On);
        assert_eq!(buttons.italic.greyed, None);
    }

    /// Criteria 8 and 28: a single-face SVG font: Regular pressed, the rest greyed; an outline
    /// face of the same name is another family.
    #[test]
    fn ac08_ac28_single_face_svg_font() {
        let mut svg = face("Hershey Sans 1-stroke", "Regular", "HersheySans1.svg");
        svg.kind = FontKind::SingleLine;
        let faces = vec![svg, face("Hershey Sans 1-stroke", "Bold", "fake.otf")];
        let mut text = font("Hershey Sans 1-stroke", "Regular");
        text.kind = FontKind::SingleLine;
        let buttons = style_buttons(&faces, &[text]);
        assert_eq!(buttons.weights[2].pressed, Pressed::On);
        assert_eq!(greyed(&buttons), [true, true, false, true, true, true]);
    }

    /// Criteria 6 and 8: the reasons.
    #[test]
    fn ac06_reasons() {
        let mut faces = family("Source Serif 4", &["Regular", "Italic", "Bold"]);
        faces.extend(family("Lobster", &["Italic"]));
        faces.extend(family("Great Vibes", &["Regular"]));
        let serif_italic = style_buttons(&faces, &[font("Source Serif 4", "Italic")]);
        assert_eq!(
            serif_italic.weights[3].greyed,
            Some(Greyed::NotInAsItalic("Source Serif 4".into()))
        );
        let lobster = style_buttons(&faces, &[font("Lobster", "Italic")]);
        assert_eq!(lobster.italic.pressed, Pressed::On);
        assert_eq!(
            lobster.italic.greyed,
            Some(Greyed::NotInAsUpright("Lobster".into()))
        );
        let several = style_buttons(
            &faces,
            &[
                font("Source Serif 4", "Italic"),
                font("Lobster", "Italic"),
                font("Great Vibes", "Regular"),
            ],
        );
        assert_eq!(several.weights[0].greyed, Some(Greyed::NotInAny(3)));
    }

    /// Criterion 4: several texts: a weight pressed only when all are in it; Italic mixed.
    #[test]
    fn ac04_several_texts() {
        let faces = family(
            "Source Sans 3",
            &["Regular", "Italic", "Bold", "Bold Italic"],
        );
        let buttons = style_buttons(
            &faces,
            &[
                font("Source Sans 3", "Bold"),
                font("Source Sans 3", "Bold Italic"),
            ],
        );
        assert_eq!(buttons.weights[3].pressed, Pressed::On);
        assert_eq!(buttons.italic.pressed, Pressed::Mixed);
        let mixed = style_buttons(
            &faces,
            &[
                font("Source Sans 3", "Bold"),
                font("Source Sans 3", "Regular"),
            ],
        );
        assert!(mixed.weights.iter().all(|b| b.pressed == Pressed::Off));
        assert_eq!(mixed.italic.pressed, Pressed::Off);
    }

    /// Criterion 7: greyed only when none of the texts has the face.
    #[test]
    fn ac07_permit_over_refuse() {
        let mut faces = family("Source Sans 3", &["Regular", "Bold"]);
        faces.extend(family("Great Vibes", &["Regular"]));
        let fonts = [
            font("Source Sans 3", "Regular"),
            font("Great Vibes", "Regular"),
        ];
        let buttons = style_buttons(&faces, &fonts);
        assert_eq!(buttons.weights[3].greyed, None);
        let press = press_weight(&faces, &fonts, WeightBand::Bold);
        assert_eq!((press.set, press.left), (1, 1));
        assert_eq!(press.targets, vec![Some(1), None]);
    }

    /// Criterion 9: a missing face uses the stored family in the first block that lists it;
    /// with none, all six are greyed with "Font missing".
    #[test]
    fn ac09_missing_face() {
        let mut faces = family("Source Serif 4", &["Regular", "Bold"]);
        let mut curvyo = family("Source Serif 4", &["Regular", "Light"]);
        for face in &mut curvyo {
            face.block = FontBlock::Curvyo;
        }
        faces.extend(curvyo);
        // The stored style "Medium" is not listed, so the face is missing.
        let text = font("Source Serif 4", "Medium");
        assert_eq!(
            style_at(&faces, weight_target(&faces, &text, WeightBand::Light)),
            Some("Light")
        );
        assert_eq!(weight_target(&faces, &text, WeightBand::Bold), None);
        let gone = style_buttons(&faces, &[font("Lost Font", "Bold")]);
        assert!(gone.weights.iter().all(|b| b.pressed == Pressed::Off));
        assert!(
            gone.weights
                .iter()
                .chain([&gone.italic])
                .all(|b| b.greyed == Some(Greyed::FontMissing))
        );
    }

    /// Criteria 12 and 13: Italic adds the italic to upright texts, all italic go upright;
    /// Bold, then Italic, then Light gives Light Italic.
    #[test]
    fn ac12_ac13_italic_press() {
        let faces = family(
            "Source Sans 3",
            &[
                "Regular",
                "Italic",
                "Bold",
                "Bold Italic",
                "Light",
                "Light Italic",
            ],
        );
        let fonts = [
            font("Source Sans 3", "Regular"),
            font("Source Sans 3", "Bold Italic"),
        ];
        let press = press_italic(&faces, &fonts);
        assert!(!press.upright);
        assert_eq!(press.targets, vec![Some(1), None]);
        assert_eq!((press.set, press.left), (2, 0));
        let back = press_italic(&faces, &[font("Source Sans 3", "Italic")]);
        assert!(back.upright);
        assert_eq!(back.targets, vec![Some(0)]);
        let mut text = font("Source Sans 3", "Regular");
        for step in ["bold", "italic", "light"] {
            let target = match step {
                "bold" => weight_target(&faces, &text, WeightBand::Bold),
                "italic" => italic_target(&faces, &text),
                _ => weight_target(&faces, &text, WeightBand::Light),
            };
            text = font("Source Sans 3", &faces[target.unwrap_or(0)].style);
        }
        assert_eq!(text.style, "Light Italic");
    }

    /// Criteria 22 to 24: a family pick keeps the style where the family has it; tiles show it
    /// for one style, the picked face for several.
    #[test]
    fn ac22_ac24_family_pick_and_tiles() {
        let mut faces = family("Source Sans 3", &["Regular", "Bold Italic"]);
        faces.extend(family(
            "Source Serif 4",
            &["Regular", "Italic", "Bold", "Bold Italic"],
        ));
        faces.extend(family("Great Vibes", &["Regular"]));
        let text = font("Source Sans 3", "Bold Italic");
        assert_eq!(
            style_at(&faces, Some(family_target(&faces, 2, &text))),
            Some("Bold Italic")
        );
        assert_eq!(family_target(&faces, 6, &text), 6);
        assert_eq!(family_tile(&faces, 2, &[text.clone(), text.clone()]), 5);
        assert_eq!(
            family_tile(&faces, 2, &[text, font("Source Sans 3", "Regular")]),
            2
        );
    }
}
