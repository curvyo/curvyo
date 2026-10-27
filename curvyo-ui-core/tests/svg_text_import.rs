//! Imported texts as Curvyo text objects (`specs/0024-svg-import-export` slice 2, criteria 80 to
//! 95, 106): Size, transform, positions to kerns, rotation, spacing, alignment, baselines,
//! writing modes, splits, missing faces and text on path, with the included faces loaded.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod svg_fonts;

use curvyo_document_core::{
    DrawingNode, DrawingText, MISSING_FACE_HASH, TextAlign, TextSnapshot, TextWritingMode,
    UNCREATED_TEXT_ID,
};
use curvyo_ui_core::{SvgImport, SvgPlacement, text_layout};
use kurbo::{Affine, Point};
use svg_fonts::Fonts;

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;

/// A millimetre document: one user unit is 1 mm.
fn import(fonts: &Fonts, body: &str) -> SvgImport {
    let svg =
        format!(r#"<svg {NS} width="200mm" height="200mm" viewBox="0 0 200 200">{body}</svg>"#);
    fonts.import(svg.as_bytes(), SvgPlacement::File)
}

/// Every text object of the import, groups entered, with its carrier for a text on path.
fn texts(nodes: &[DrawingNode]) -> Vec<(DrawingText, Option<usize>)> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            DrawingNode::Group { children, .. } => out.extend(texts(children)),
            DrawingNode::Text { text, .. } => out.push((text.clone(), None)),
            DrawingNode::TextOnPath { text, carrier, .. } => {
                out.push((text.clone(), Some(*carrier)));
            }
            _ => {}
        }
    }
    out
}

fn snapshot(text: &DrawingText) -> TextSnapshot {
    TextSnapshot {
        id: UNCREATED_TEXT_ID,
        string: text.string.clone(),
        font: text.font.clone(),
        size: text.size,
        layout: text.layout,
        chars: text.chars.clone(),
        placement: text.placement,
        style: text.style.clone(),
        outline: None,
        on_path: None,
        source: None,
    }
}

/// The face a text is drawn in (the substitute for a missing face).
fn face<'a>(fonts: &'a Fonts, text: &DrawingText) -> &'a curvyo_text_core::Face {
    let listed = fonts
        .faces
        .iter()
        .find(|f| f.key.unwrap().hash.to_hex() == text.font.face_hash)
        .or_else(|| {
            fonts
                .faces
                .iter()
                .find(|f| f.family == "Source Sans 3" && f.style == "Regular")
        })
        .unwrap();
    fonts.store.get(&listed.key.unwrap()).unwrap()
}

/// The document position of each glyph's origin as drawn (turned glyphs about their centre),
/// with the glyph's cluster byte.
fn origins(fonts: &Fonts, text: &DrawingText) -> Vec<(usize, Point)> {
    let laid = text_layout(&snapshot(text), face(fonts, text)).unwrap();
    let placement = Affine::new(text.placement.to_array());
    let frame = laid.frame();
    let mut out = Vec::new();
    for line in &laid.lines {
        for glyph in &line.glyphs {
            let turn = laid.rotation_rad + glyph.rotation_rad;
            let centre = Point::new(glyph.centre_x.0, glyph.centre_y.0);
            let drawn =
                centre + (Affine::rotate(turn) * (Point::ZERO + (glyph.origin - centre))).to_vec2();
            out.push((glyph.cluster, placement * (frame * drawn)));
        }
    }
    out
}

fn near(a: Point, b: Point, tolerance: f64) -> bool {
    (a - b).hypot() <= tolerance
}

/// Criterion 83's example: Inkscape's 30 pt text in a millimetre document.
#[test]
fn ac83_size_from_the_font_size_by_the_cap_height() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text x="20" y="50" style="font-size:10.5833px;font-family:'Source Sans 3'">Boho</text>"#,
    );
    let (text, _) = &texts(&import.nodes)[0];
    // The measured H of the included Source Sans 3 is 656 of 1000 units (the spec's example
    // assumes OS/2's 660, which would give 6.985 mm).
    let cap = face(&fonts, text).metrics().cap_height;
    assert!((cap - 0.656).abs() < 1e-3, "{cap}");
    assert!((text.size.as_mm() - 10.5833 * cap).abs() < 0.001);
    assert_eq!(text.font.family, "Source Sans 3");
    assert_eq!(text.string, "Boho");
    assert_eq!(text.placement.to_array()[4..], [20.0, 50.0]);
    assert_eq!(import.leaves, 1);
}

/// Criterion 84: a rotation stays in the transform, a stretch keeps its non-uniform part.
#[test]
fn ac84_the_uniform_part_goes_into_size() {
    let fonts = Fonts::included();
    let size = |transform: &str| {
        let import = import(
            &fonts,
            &format!(r#"<text transform="{transform}" style="font-size:10px">H</text>"#),
        );
        let (text, _) = texts(&import.nodes)[0].clone();
        (text.size.as_mm(), text.placement.to_array())
    };
    let (plain, _) = size("translate(0 0)");
    let (turned, m) = size("rotate(30)");
    assert!((plain - turned).abs() < 1e-9);
    assert!((m[1].atan2(m[0]).to_degrees() - 30.0).abs() < 1e-9);
    let (stretched, m) = size("scale(2 1)");
    let (root2, _) = size("scale(1.41421356237)");
    assert!((stretched - root2).abs() < 1e-6);
    assert!((m[0] - 2_f64.sqrt()).abs() < 1e-9 && (m[3] - 1.0 / 2_f64.sqrt()).abs() < 1e-9);
}

/// Criterion 86: absolute and relative positions land where SVG puts them, as kerns.
#[test]
fn ac86_every_character_lands_where_the_file_puts_it() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text x="10 16 22 30" y="50 50 52 50" style="font-size:8px">ABCD</text>"#,
    );
    let all = texts(&import.nodes);
    assert_eq!(all.len(), 1, "one text holds the kerns");
    let (text, _) = &all[0];
    assert!(!text.chars.is_empty());
    let want = [(10.0, 50.0), (16.0, 50.0), (22.0, 52.0), (30.0, 50.0)];
    let got = origins(&fonts, text);
    for (k, (x, y)) in want.iter().enumerate() {
        assert!(
            near(got[k].1, Point::new(*x, *y), 0.01),
            "{k}: {:?}",
            got[k].1
        );
    }
}

/// Criterion 88: `textLength` spreads the gaps so the extent is its length: the last H ends at
/// x + textLength; within the ranges one text, beyond them the pieces of criterion 92 (b).
#[test]
fn ac88_text_length_spreads_the_gaps() {
    let fonts = Fonts::included();
    let plain = import(
        &fonts,
        r#"<text x="10" y="20" style="font-size:5px">HH</text>"#,
    );
    let pens: Vec<f64> = origins(&fonts, &texts(&plain.nodes)[0].0)
        .iter()
        .map(|(_, p)| p.x)
        .collect();
    let advance = pens[1] - pens[0];
    for (length, objects) in [(40.0, 1), (60.0, 4)] {
        let import = import(
            &fonts,
            &format!(
                r#"<text x="10" y="20" textLength="{length}" style="font-size:5px">HHHH</text>"#
            ),
        );
        let all = texts(&import.nodes);
        assert_eq!(all.len(), objects, "textLength {length}");
        let firsts: Vec<f64> = all
            .iter()
            .flat_map(|(t, _)| origins(&fonts, t))
            .map(|(_, p)| p.x)
            .collect();
        assert!((firsts[0] - 10.0).abs() <= 0.01, "{firsts:?} {pens:?}");
        assert!(
            (firsts[3] + advance - (10.0 + length)).abs() <= 0.01,
            "textLength {length}: {firsts:?} advance {advance}"
        );
    }
}

/// Criterion 87's test: B turned a quarter clockwise about its origin, A and C unchanged.
#[test]
fn ac87_turned_characters_keep_their_origin() {
    let fonts = Fonts::included();
    let plain = import(
        &fonts,
        r#"<text x="10" y="50" style="font-size:8px">ABC</text>"#,
    );
    let turned = import(
        &fonts,
        r#"<text x="10" y="50" rotate="0 90 0" style="font-size:8px">ABC</text>"#,
    );
    let a = origins(&fonts, &texts(&plain.nodes)[0].0);
    let (text, _) = &texts(&turned.nodes)[0];
    assert_eq!(text.chars.get(1).rotation, 90.0);
    let b = origins(&fonts, text);
    for k in 0..3 {
        assert!(near(a[k].1, b[k].1, 0.01), "{k}: {:?} {:?}", a[k].1, b[k].1);
    }
}

/// Criteria 85, 89: Inkscape's centred lines keep Centre, one text, one anchor, the line spacing
/// of `line-height`.
#[test]
fn ac85_ac89_inkscape_centred_lines() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text xml:space="preserve" style="font-size:10px;line-height:1.25;text-anchor:middle;font-family:'Source Sans 3'" x="100" y="40"><tspan sodipodi:role="line" x="100" y="40">First line</tspan><tspan sodipodi:role="line" x="100" y="52.5">2</tspan><tspan sodipodi:role="line" x="100" y="65">The third line</tspan></text>"#,
    );
    let all = texts(&import.nodes);
    assert_eq!(all.len(), 1);
    let (text, _) = &all[0];
    assert_eq!(text.layout.align, TextAlign::Centre);
    assert_eq!(text.string, "First line\n2\nThe third line");
    let factor = text.layout.line_spacing.unwrap();
    assert!((factor * text.size.as_mm() - 12.5).abs() < 1e-9);
    let laid = text_layout(&snapshot(text), face(&fonts, text)).unwrap();
    let placement = Affine::new(text.placement.to_array());
    for line in &laid.lines {
        let middle =
            placement * Point::new(f64::midpoint(line.extent.start.0, line.extent.end.0), 0.0);
        assert!((middle.x - 100.0).abs() < 0.01, "{middle:?}");
    }
}

/// Criterion 90's test: `dominant-baseline="middle"` centres an x on y = 50.
#[test]
fn ac90_middle_baseline_centres_the_x() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text x="10" y="50" dominant-baseline="middle" style="font-size:20px">x</text>"#,
    );
    let (text, _) = &texts(&import.nodes)[0];
    let face = face(&fonts, text);
    let laid = text_layout(&snapshot(text), face).unwrap();
    let glyph = &laid.lines[0].glyphs[0];
    let outline = face.outline(glyph.glyph);
    let bounds = (Affine::new(text.placement.to_array())
        * Affine::translate(glyph.origin.to_vec2())
        * Affine::scale(laid.mm_per_em))
        * outline.path.clone();
    let b = kurbo::Shape::bounding_box(&bounds);
    assert!((f64::midpoint(b.y0, b.y1) - 50.0).abs() < 0.05, "{b:?}");
}

/// Criterion 92's test: a bold word splits the text into a group of two, side by side.
#[test]
fn ac92_mixed_styles_split_into_a_group() {
    let fonts = Fonts::included();
    let mixed = import(
        &fonts,
        r#"<text x="10" y="50" style="font-size:10px;font-family:'Source Sans 3'">Hello <tspan style="font-weight:bold">world</tspan></text>"#,
    );
    let DrawingNode::Group {
        children,
        registers,
        ..
    } = &mixed.nodes[0]
    else {
        panic!("{:?}", mixed.nodes)
    };
    assert_eq!(registers.name.as_deref(), Some("Hello world"));
    let all = texts(children);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].0.string, "Hello ");
    assert_eq!(all[0].0.font.style, "Regular");
    assert_eq!(
        (all[1].0.string.as_str(), all[1].0.font.style.as_str()),
        ("world", "Bold")
    );
    let hello = text_layout(&snapshot(&all[0].0), face(&fonts, &all[0].0)).unwrap();
    let end = hello.lines[0].start.0 + hello.lines[0].width.0;
    let world = origins(&fonts, &all[1].0)[0].1;
    assert!((world.x - (10.0 + end)).abs() < 0.01, "{world:?} {end}");
    // Criterion 92 (b): a far tspan starts a new text.
    let far = import(
        &fonts,
        r#"<text x="10" y="50" style="font-size:10px">Near<tspan x="150">Far</tspan></text>"#,
    );
    assert_eq!(texts(&far.nodes).len(), 2);
}

/// Criterion 82: a missing family keeps its name and the reserved hash; the substitute draws it.
#[test]
fn ac82_missing_face() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text x="10" y="50" style="font-family:'Nonexistent Sans', sans-serif;font-weight:bold">Hi</text>"#,
    );
    let (text, _) = &texts(&import.nodes)[0];
    assert_eq!(text.font.family, "Nonexistent Sans");
    assert_eq!(text.font.style, "Bold");
    assert_eq!(text.font.face_hash, MISSING_FACE_HASH);
}

/// Criteria 88, 91: spacing as the text's values, vertical modes.
#[test]
fn ac88_ac91_spacing_and_vertical() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r#"<text x="10" y="50" style="font-size:10px;letter-spacing:1px;word-spacing:2px">A B</text>
        <text x="100" y="10" style="font-size:10px;writing-mode:vertical-rl;text-orientation:upright">ABC</text>"#,
    );
    let all = texts(&import.nodes);
    let size = all[0].0.size.as_mm();
    assert!((all[0].0.layout.letter_spacing * size - 1.0).abs() < 1e-9);
    assert!((all[0].0.layout.word_spacing * size - 2.0).abs() < 1e-9);
    assert!(
        all[0].0.chars.is_empty(),
        "no kerns where the layout agrees"
    );
    assert_eq!(all[1].0.layout.writing_mode, TextWritingMode::VerticalRl);
}

/// Criteria 93, 94: a text on a drawn ring names it as carrier with its Position.
#[test]
fn ac93_text_on_path() {
    let fonts = Fonts::included();
    let import = import(
        &fonts,
        r##"<path id="ring" d="M60 100 A40 40 0 1 1 140 100 A40 40 0 1 1 60 100" fill="none" stroke="#000"/>
        <text style="font-size:8px;text-anchor:middle"><textPath href="#ring" startOffset="75%">CURVYO</textPath></text>"##,
    );
    let all = texts(&import.nodes);
    let (text, carrier) = &all[0];
    assert_eq!(*carrier, Some(0));
    assert_eq!(text.string, "CURVYO");
    let DrawingNode::TextOnPath { values, .. } = &import.nodes[1] else {
        panic!()
    };
    let length = 2.0 * std::f64::consts::PI * 40.0;
    assert!(
        (values.position.as_mm() - 0.75 * length).abs() < 0.05,
        "{:?}",
        values.position
    );
}

/// Criterion 106: the import is centred on the box that includes the texts.
#[test]
fn ac106_centring_includes_texts() {
    let fonts = Fonts::included();
    let svg = format!(
        r#"<svg {NS} width="200mm" height="200mm" viewBox="0 0 200 200"><text x="10" y="50" style="font-size:10px">Only</text></svg>"#
    );
    let import = fonts.import(
        svg.as_bytes(),
        SvgPlacement::CentredOn(curvyo_document_core::Point::new(0.0, 0.0)),
    );
    let found = texts(&import.nodes);
    let (text, _) = &found[0];
    let laid = text_layout(&snapshot(text), face(&fonts, text)).unwrap();
    let line = &laid.lines[0];
    let middle = Affine::new(text.placement.to_array())
        * Point::new(f64::midpoint(line.extent.start.0, line.extent.end.0), 0.0);
    assert!(middle.x.abs() < 0.5, "{middle:?}");
}
