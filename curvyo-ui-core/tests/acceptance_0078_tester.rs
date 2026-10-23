//! Independent tester cases for `specs/0078-font-style-buttons/specification.md` on the pure
//! rules in `curvyo-ui-core` (criteria 3 to 9, 11 to 13, 22 to 24, 28): button states, greyed
//! reasons, press targets and the family pick that keeps the style.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(clippy::unwrap_used, clippy::assert_is_empty, missing_docs)]

use curvyo_document_core::{FontKind, FontRef};
use curvyo_text_core::{FaceHash, FaceKey, WeightBand};
use curvyo_ui_core::{
    FontBlock, Greyed, ListedFace, Pressed, StyleButtons, WEIGHT_STEPS, family_target, family_tile,
    italic_target, press_italic, press_weight, style_buttons, weight_target,
};

const XL: usize = 0;
const L: usize = 1;
const R: usize = 2;
const B: usize = 3;
const XB: usize = 4;

fn hash(n: u8) -> FaceHash {
    FaceHash::from_hex(&format!("{n:02x}").repeat(32)).unwrap()
}

fn face(block: FontBlock, family: &str, style: &str, id: &str) -> ListedFace {
    ListedFace {
        block,
        family: family.to_owned(),
        style: style.to_owned(),
        host_id: id.to_owned(),
        index: 0,
        key: None,
        kind: FontKind::Outline,
    }
}

fn inc(family: &str, styles: &[&str]) -> Vec<ListedFace> {
    styles
        .iter()
        .map(|s| face(FontBlock::Included, family, s, &format!("{family}/{s}")))
        .collect()
}

/// A text reference by family and style name; its hash matches no listed file.
fn text(family: &str, style: &str) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: style.to_owned(),
        face_hash: "ab".repeat(32),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn styles_of(faces: &[ListedFace], targets: &[Option<usize>]) -> Vec<Option<String>> {
    targets
        .iter()
        .map(|t| t.map(|i| faces[i].style.clone()))
        .collect()
}

fn source_sans() -> Vec<ListedFace> {
    inc(
        "Source Sans 3",
        &[
            "Regular",
            "Italic",
            "Bold",
            "Bold Italic",
            "Extra Light",
            "Extra Light Italic",
            "Light",
            "Light Italic",
        ],
    )
}

fn pressed_weights(b: &StyleButtons) -> Vec<usize> {
    (0..5)
        .filter(|&i| b.weights[i].pressed == Pressed::On)
        .collect()
}

#[test]
fn weight_steps_are_the_five_buttons_in_order() {
    assert_eq!(
        WEIGHT_STEPS,
        [
            WeightBand::ExtraLight,
            WeightBand::Light,
            WeightBand::Regular,
            WeightBand::Bold,
            WeightBand::ExtraBold
        ]
    );
}

/// Criterion 3: a face in a step presses that step; Thin, Medium, Semibold, Black press none;
/// an italic face presses Italic.
#[test]
fn ac03_pressed_state_follows_the_band() {
    let faces = inc(
        "Inter",
        &[
            "Thin",
            "Extra Light",
            "Light",
            "Regular",
            "Medium",
            "Semibold",
            "Bold",
            "Extra Bold",
            "Black",
            "Bold Italic",
        ],
    );
    for (style, want) in [
        ("Extra Light", Some(XL)),
        ("Light", Some(L)),
        ("Regular", Some(R)),
        ("Bold", Some(B)),
        ("Extra Bold", Some(XB)),
        ("Thin", None),
        ("Medium", None),
        ("Semibold", None),
        ("Black", None),
    ] {
        let b = style_buttons(&faces, &[text("Inter", style)]);
        assert_eq!(
            pressed_weights(&b),
            want.into_iter().collect::<Vec<_>>(),
            "{style}"
        );
        assert_eq!(b.italic.pressed, Pressed::Off, "{style}");
    }
    let b = style_buttons(&faces, &[text("Inter", "Bold Italic")]);
    assert_eq!(pressed_weights(&b), vec![B]);
    assert_eq!(b.italic.pressed, Pressed::On);
}

/// Criterion 4: several texts press a weight only when all are in it; Italic on, off, mixed.
#[test]
fn ac04_several_texts_pressed_and_mixed() {
    let faces = source_sans();
    let both_bold = style_buttons(
        &faces,
        &[
            text("Source Sans 3", "Bold"),
            text("Source Sans 3", "Bold Italic"),
        ],
    );
    assert_eq!(pressed_weights(&both_bold), vec![B]);
    assert_eq!(both_bold.italic.pressed, Pressed::Mixed);
    let differ = style_buttons(
        &faces,
        &[
            text("Source Sans 3", "Bold"),
            text("Source Sans 3", "Light"),
        ],
    );
    assert!(pressed_weights(&differ).is_empty());
    assert_eq!(differ.italic.pressed, Pressed::Off);
    let all_it = style_buttons(
        &faces,
        &[
            text("Source Sans 3", "Italic"),
            text("Source Sans 3", "Light Italic"),
        ],
    );
    assert_eq!(all_it.italic.pressed, Pressed::On);
}

/// Criteria 5 and 6: greyed for a missing target, with the reason naming the family; a pressed
/// weight button is never greyed; italic texts get "as italic", Italic while pressed "as upright".
#[test]
fn ac05_ac06_greyed_and_reasons() {
    let vibes = inc("Great Vibes", &["Regular"]);
    let b = style_buttons(&vibes, &[text("Great Vibes", "Regular")]);
    assert_eq!(pressed_weights(&b), vec![R]);
    assert!(b.weights[R].greyed.is_none(), "pressed is never greyed");
    for i in [XL, L, B, XB] {
        assert_eq!(
            b.weights[i].greyed,
            Some(Greyed::NotIn("Great Vibes".into()))
        );
    }
    assert_eq!(b.italic.greyed, Some(Greyed::NotIn("Great Vibes".into())));

    let serif = inc("Source Serif 4", &["Regular", "Bold", "Italic"]);
    let b = style_buttons(&serif, &[text("Source Serif 4", "Italic")]);
    assert!(b.weights[R].greyed.is_none());
    assert_eq!(
        b.weights[B].greyed,
        Some(Greyed::NotInAsItalic("Source Serif 4".into())),
        "no Bold Italic"
    );

    let lobster_it = inc("Lobster", &["Italic"]);
    let b = style_buttons(&lobster_it, &[text("Lobster", "Italic")]);
    assert_eq!(b.italic.pressed, Pressed::On);
    assert_eq!(
        b.italic.greyed,
        Some(Greyed::NotInAsUpright("Lobster".into()))
    );
}

/// Criterion 6: several families, none with the face: "Not in any of the 3 fonts".
#[test]
fn ac06_not_in_any_counts_families() {
    let mut faces = inc("Great Vibes", &["Regular"]);
    faces.extend(inc("Lobster", &["Regular"]));
    faces.extend(inc("Pacifico", &["Regular"]));
    let b = style_buttons(
        &faces,
        &[
            text("Great Vibes", "Regular"),
            text("Lobster", "Regular"),
            text("Pacifico", "Regular"),
        ],
    );
    assert_eq!(b.weights[B].greyed, Some(Greyed::NotInAny(3)));
    assert_eq!(b.italic.greyed, Some(Greyed::NotInAny(3)));
}

/// Criterion 7: with several texts a button is greyed only when none has the face.
#[test]
fn ac07_greyed_only_when_none_has_it() {
    let mut faces = inc("Great Vibes", &["Regular"]);
    faces.extend(source_sans());
    let fonts = [
        text("Great Vibes", "Regular"),
        text("Source Sans 3", "Regular"),
    ];
    let b = style_buttons(&faces, &fonts);
    assert!(b.weights[B].greyed.is_none());
    assert!(b.italic.greyed.is_none());
    assert!(b.weights[XB].greyed.is_some());
    // Italic: only the texts it would change count. Mixed (one italic Source Sans, one
    // upright Great Vibes): pressing changes the Great Vibes text only, which has none.
    let fonts = [
        text("Great Vibes", "Regular"),
        text("Source Sans 3", "Italic"),
    ];
    let b = style_buttons(&faces, &fonts);
    assert_eq!(b.italic.pressed, Pressed::Mixed);
    assert!(
        b.italic.greyed.is_some(),
        "the only text it would change has no italic"
    );
}

/// Criterion 8: single-line font with one face; Source Sans 3 included faces.
#[test]
fn ac08_single_line_and_source_sans() {
    let mut hershey = inc("Hershey Sans 1-stroke", &["Regular"]);
    hershey[0].kind = FontKind::SingleLine;
    let mut r = text("Hershey Sans 1-stroke", "Regular");
    r.kind = FontKind::SingleLine;
    let b = style_buttons(&hershey, &[r]);
    assert_eq!(pressed_weights(&b), vec![R]);
    for i in [XL, L, B, XB] {
        assert!(b.weights[i].greyed.is_some());
    }
    assert!(b.italic.greyed.is_some());

    let b = style_buttons(&source_sans(), &[text("Source Sans 3", "Regular")]);
    for i in [XL, L, R, B] {
        assert!(b.weights[i].greyed.is_none(), "{i}");
    }
    assert!(b.italic.greyed.is_none());
    assert_eq!(
        b.weights[XB].greyed,
        Some(Greyed::NotIn("Source Sans 3".into()))
    );
}

/// Criterion 8, desktop part: a system family with an Extra Bold face offers it.
#[test]
fn ac08_system_extra_bold_available() {
    let faces: Vec<_> = ["Regular", "Extra Bold"]
        .iter()
        .map(|s| face(FontBlock::System, "Noto Sans", s, &format!("/f/{s}")))
        .collect();
    let font = text("Noto Sans", "Regular");
    assert!(
        style_buttons(&faces, std::slice::from_ref(&font)).weights[XB]
            .greyed
            .is_none()
    );
    assert_eq!(weight_target(&faces, &font, WeightBand::ExtraBold), Some(1));
}

/// Criterion 9: a missing face uses the first block that lists the family; pressed states follow
/// the stored style name; no family anywhere greys all six with "Font missing".
#[test]
fn ac09_missing_face() {
    let mut faces: Vec<_> = ["Regular"]
        .iter()
        .map(|s| face(FontBlock::Curvyo, "Lato", s, "c/lato"))
        .collect();
    faces.extend(
        ["Regular", "Bold", "Italic"]
            .iter()
            .map(|s| face(FontBlock::System, "Lato", s, &format!("s/{s}"))),
    );
    // Stored "Bold" is not in the Curvyo block: the face is missing there, yet that block is
    // the one used, so Bold is pressed (stored style) and the others follow Curvyo's faces.
    let b = style_buttons(&faces, &[text("Lato", "Bold")]);
    assert_eq!(pressed_weights(&b), vec![B]);
    assert!(b.weights[R].greyed.is_none());
    assert!(b.weights[L].greyed.is_some());
    assert!(b.italic.greyed.is_some(), "Curvyo block has no italic");

    let b = style_buttons(&faces, &[text("Nowhere Grotesk", "Bold")]);
    assert_eq!(pressed_weights(&b), Vec::<usize>::new());
    assert_eq!(b.italic.pressed, Pressed::Off);
    for w in &b.weights {
        assert_eq!(w.greyed, Some(Greyed::FontMissing));
    }
    assert_eq!(b.italic.greyed, Some(Greyed::FontMissing));
}

/// Criterion 11: a weight press counts set and left texts and leaves the others alone.
#[test]
fn ac11_press_weight_counts() {
    let mut faces = inc("Great Vibes", &["Regular"]);
    faces.extend(source_sans());
    let fonts = [
        text("Source Sans 3", "Regular"),
        text("Great Vibes", "Regular"),
        text("Source Sans 3", "Light Italic"),
    ];
    let p = press_weight(&faces, &fonts, WeightBand::Bold);
    assert_eq!((p.set, p.left), (2, 1));
    assert_eq!(
        styles_of(&faces, &p.targets),
        vec![Some("Bold".into()), None, Some("Bold Italic".into())]
    );
    assert!(!p.upright);
}

/// Criterion 12: Italic on mixed makes the upright ones italic, italic ones stay; on all-italic
/// makes all upright; texts without counterpart are left.
#[test]
fn ac12_press_italic() {
    let mut faces = inc("Great Vibes", &["Regular"]);
    faces.extend(source_sans());
    let fonts = [
        text("Source Sans 3", "Bold"),
        text("Source Sans 3", "Italic"),
        text("Great Vibes", "Regular"),
    ];
    let p = press_italic(&faces, &fonts);
    assert!(!p.upright);
    assert_eq!(p.left, 1);
    assert_eq!(p.set, 2, "Bold Italic and the italic that stays");
    let s = styles_of(&faces, &p.targets);
    assert_eq!(s[0].as_deref(), Some("Bold Italic"));
    assert!(
        s[1].is_none() || s[1].as_deref() == Some("Italic"),
        "italic text stays"
    );
    assert!(s[2].is_none());

    let fonts = [
        text("Source Sans 3", "Bold Italic"),
        text("Source Sans 3", "Light Italic"),
    ];
    let p = press_italic(&faces, &fonts);
    assert!(p.upright);
    assert_eq!((p.set, p.left), (2, 0));
    assert_eq!(
        styles_of(&faces, &p.targets),
        vec![Some("Bold".into()), Some("Light".into())]
    );
}

/// Criterion 13: weight keeps italic; Bold, then Italic, then Light ends Light Italic.
#[test]
fn ac13_weight_and_italic_compose() {
    let faces = source_sans();
    let mut font = text("Source Sans 3", "Regular");
    let to = |font: &FontRef, i: usize| text(&font.family, &faces[i].style);
    font = to(
        &font,
        weight_target(&faces, &font, WeightBand::Bold).unwrap(),
    );
    font = to(&font, italic_target(&faces, &font).unwrap());
    assert_eq!(font.style, "Bold Italic");
    font = to(
        &font,
        weight_target(&faces, &font, WeightBand::Light).unwrap(),
    );
    assert_eq!(font.style, "Light Italic");
}

/// Criterion 22: a family pick keeps the weight and slant where the family has them, else
/// Regular; works across the Thin..Black bands too (weight band).
#[test]
fn ac22_family_pick_keeps_style() {
    let mut faces = source_sans();
    let serif_start = faces.len();
    faces.extend(inc(
        "Source Serif 4",
        &["Regular", "Bold", "Italic", "Semibold"],
    ));
    let picked = serif_start; // family row picks its Regular face
    let style = |f: &FontRef| faces[family_target(&faces, picked, f)].style.clone();
    assert_eq!(style(&text("Source Sans 3", "Bold")), "Bold");
    assert_eq!(style(&text("Source Sans 3", "Italic")), "Italic");
    assert_eq!(
        style(&text("Source Sans 3", "Bold Italic")),
        "Regular",
        "no Bold Italic"
    );
    assert_eq!(style(&text("Source Sans 3", "Light")), "Regular");
    assert_eq!(
        style(&text("Inter", "Semibold")),
        "Semibold",
        "a band without button"
    );
}

/// Criterion 24: a tile shows the subject's style; several texts with different styles show
/// Regular.
#[test]
fn ac24_tile_face() {
    let mut faces = inc("Source Serif 4", &["Regular", "Bold", "Bold Italic"]);
    faces.extend(source_sans());
    let bi = family_tile(&faces, 0, &[text("Source Sans 3", "Bold Italic")]);
    assert_eq!(faces[bi].style, "Bold Italic");
    let mixed = family_tile(
        &faces,
        0,
        &[
            text("Source Sans 3", "Bold"),
            text("Source Sans 3", "Light"),
        ],
    );
    assert_eq!(faces[mixed].style, "Regular");
}

/// Criterion 28: a family with Semibold and Black only presses nothing and greys Bold and
/// Extra Bold.
#[test]
fn ac28_semibold_and_black_only() {
    let faces = inc("Heavy", &["Semibold", "Black"]);
    let b = style_buttons(&faces, &[text("Heavy", "Semibold")]);
    assert!(pressed_weights(&b).is_empty());
    assert!(b.weights[B].greyed.is_some());
    assert!(b.weights[XB].greyed.is_some());
}

/// Criterion 28 and the tie rule: the same face as OTF and TTF; a loaded one wins, else the
/// first in listing order.
#[test]
fn ac28_otf_and_ttf_tie() {
    let mut faces = vec![
        face(FontBlock::System, "Lato", "Regular", "lato.otf"),
        face(FontBlock::System, "Lato", "Bold", "lato-bold.otf"),
        face(FontBlock::System, "Lato", "Bold", "lato-bold.ttf"),
    ];
    let font = text("Lato", "Regular");
    assert_eq!(weight_target(&faces, &font, WeightBand::Bold), Some(1));
    faces[2].key = Some(FaceKey {
        hash: hash(7),
        index: 0,
    });
    assert_eq!(
        weight_target(&faces, &font, WeightBand::Bold),
        Some(2),
        "loaded wins"
    );
}

/// Criterion 28: a Condensed text stays Condensed; a normal-width face does not qualify.
#[test]
fn ac28_condensed_family_keeps_width() {
    let faces = inc(
        "Roboto",
        &["Regular", "Bold", "Condensed", "Condensed Light"],
    );
    let font = text("Roboto", "Condensed");
    assert_eq!(
        weight_target(&faces, &font, WeightBand::Light).map(|i| faces[i].style.as_str()),
        Some("Condensed Light")
    );
    assert_eq!(
        weight_target(&faces, &font, WeightBand::Bold),
        None,
        "no Condensed Bold"
    );
    let b = style_buttons(&faces, &[font]);
    assert_eq!(pressed_weights(&b), vec![R]);
}

/// Criterion 28 and Terms: an Oblique-only family has an italic counterpart; Italic is
/// preferred over Oblique.
#[test]
fn ac28_oblique_only_and_italic_preferred() {
    let faces = inc("Mono", &["Regular", "Oblique"]);
    let font = text("Mono", "Regular");
    assert_eq!(italic_target(&faces, &font), Some(1));
    let b = style_buttons(&faces, &[text("Mono", "Oblique")]);
    assert_eq!(b.italic.pressed, Pressed::On);
    let faces = inc("Both", &["Regular", "Oblique", "Italic"]);
    assert_eq!(italic_target(&faces, &text("Both", "Regular")), Some(2));
}

/// Terms: a family is the faces of the same block; a weight in another block does not count.
#[test]
fn family_is_per_block() {
    let faces = vec![
        face(FontBlock::Included, "Lato", "Regular", "i/r"),
        face(FontBlock::System, "Lato", "Bold", "s/b"),
    ];
    let mut font = text("Lato", "Regular");
    // Resolve the text to the Included face by its hash-independent name match.
    font.face_hash = "cd".repeat(32);
    let b = style_buttons(&faces, &[font.clone()]);
    assert!(b.weights[B].greyed.is_some(), "Bold is in another block");
    assert_eq!(weight_target(&faces, &font, WeightBand::Bold), None);
}

/// Edge: an empty listing and no subject fonts do not panic.
#[test]
fn degenerate_inputs() {
    let b = style_buttons(&[], &[]);
    assert!(pressed_weights(&b).is_empty());
    let p = press_weight(&[], &[], WeightBand::Bold);
    assert_eq!((p.set, p.left), (0, 0));
    let p = press_italic(&[], &[text("X", "Regular")]);
    assert_eq!(p.set, 0);
}
