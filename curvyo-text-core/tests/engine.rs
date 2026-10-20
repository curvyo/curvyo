//! Acceptance tests of the text engine against `specs/0074-text-objects` (criteria 4, 5, 6, 10,
//! 12, 21, 23, 25), with the included fonts as fixture faces.
#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

mod common;

use common::{SANS, SERIF, face, font_bytes, fonts_dir, store_with};
use curvyo_text_core::{FaceStore, Mm, TextError, layout};
use kurbo::{CubicBez, ParamCurve, PathEl, Point, QuadBez, Shape};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

const TOLERANCE_MM: f64 = 0.01;

/// Criterion 4: lines are left-aligned at x = 0 and each next baseline is the face's ascender
/// minus descender plus line gap lower, scaled to the size.
#[test]
fn ac04_line_height_from_the_face_metrics() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let laid = layout(face, "Ab\nCd\nEf", Mm(10.0)).unwrap();
    let bytes = font_bytes(SERIF);
    let stated = FontRef::new(&bytes)
        .unwrap()
        .metrics(Size::unscaled(), LocationRef::default());
    let em = f64::from(stated.units_per_em);
    let line_em = f64::from(stated.ascent) / em - f64::from(stated.descent) / em
        + f64::from(stated.leading) / em;
    let expected = line_em * laid.mm_per_em;
    for (index, line) in laid.lines.iter().enumerate() {
        assert!(
            (line.baseline.0 - expected * f64::from(u8::try_from(index).unwrap())).abs() < 1e-9
        );
        assert!(
            line.glyphs[0].origin.x.abs() < 1.0,
            "left-aligned at the anchor"
        );
    }
}

/// Criterion 5: GPOS kerning and the fi ligature of Source Serif 4.
#[test]
fn ac05_kerning_and_ligature() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let size = Mm(20.0);
    let width = |text| layout(face, text, size).unwrap().lines[0].width.0;
    let kerned = width("AV");
    let unkerned = width("A") + width("V");
    // Source Serif 4 kerns A-V by -119 units of its 1000-unit em.
    let kern = 119.0 / 1000.0 * layout(face, "A", size).unwrap().mm_per_em;
    assert!((unkerned - kerned - kern).abs() < TOLERANCE_MM);
    let fi = layout(face, "fi", size).unwrap();
    assert_eq!(fi.lines[0].glyphs.len(), 1, "fi draws one ligature glyph");
    assert_ne!(fi.lines[0].glyphs[0].glyph, 0);
}

/// Criterion 6: a character the face lacks draws `.notdef` in its place.
#[test]
fn ac06_missing_character_draws_notdef() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let laid = layout(face, "a\u{E000}b", Mm(10.0)).unwrap();
    let glyphs: Vec<u32> = laid.lines[0].glyphs.iter().map(|g| g.glyph).collect();
    assert_eq!(glyphs.len(), 3);
    assert_eq!(glyphs[1], 0);
    assert_eq!(
        laid.glyph_paths(face).len(),
        3,
        ".notdef has an outline and is kept"
    );
}

/// Criteria 10 and 25: the line box spans ascender to descender and anchor to the longest line;
/// caret positions are cluster boundaries, so a ligature is not split.
#[test]
fn ac10_ac25_line_box_and_caret_boundaries() {
    let (store, key) = store_with(SERIF);
    let face = face(&store, &key);
    let laid = layout(face, "fit\nH", Mm(10.0)).unwrap();
    let line_box = laid.line_box();
    assert!((line_box.y0 + laid.ascender.0).abs() < 1e-9);
    assert!((line_box.y1 - (laid.line_height.0 + laid.descender.0)).abs() < 1e-9);
    assert!((line_box.x1 - laid.lines[0].width.0).abs() < 1e-9);
    let first: Vec<usize> = laid.lines[0].boundaries.iter().map(|(b, _)| *b).collect();
    assert_eq!(first, vec![0, 2, 3], "no caret inside the fi ligature");
    assert_eq!(laid.next_boundary(0), 2);
    assert_eq!(laid.previous_boundary(4), 3);
    let on_h = Point::new(laid.lines[1].width.0 + 5.0, laid.line_height.0 - 1.0);
    assert_eq!(laid.nearest_boundary(on_h), 5);
}

/// Criterion 12: Size is the height of the capital H above the baseline, exactly, in every
/// included face (Great Vibes' H has a swash below the baseline, so its top is measured).
#[test]
fn ac12_size_is_the_cap_height_in_every_included_face() {
    let mut checked = 0;
    for family in std::fs::read_dir(fonts_dir()).unwrap() {
        let family = family.unwrap().path();
        if !family.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(family).unwrap() {
            let path = file.unwrap().path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if !matches!(ext, "ttf" | "otf") {
                continue;
            }
            let mut store = FaceStore::new();
            let key = store.add(std::fs::read(&path).unwrap(), 0).unwrap();
            let face = store.get(&key).unwrap();
            let laid = layout(face, "H", Mm(20.0)).unwrap();
            let paths = laid.glyph_paths(face);
            let top = -paths[0].bounding_box().y0;
            assert!(
                (top - 20.0).abs() < TOLERANCE_MM,
                "{}: {top}",
                path.display()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 9);
}

/// Criterion 21: every face of a collection is read; WOFF is not.
#[test]
fn ac21_collection_faces_and_no_woff() {
    let collection = collection_of(&[font_bytes(SERIF), font_bytes(SANS)]);
    let mut store = FaceStore::new();
    let serif = store.add(collection.clone(), 0).unwrap();
    let sans = store.add(collection.clone(), 1).unwrap();
    assert_eq!(serif.hash, sans.hash);
    let cap = |key| store.get(&key).unwrap().metrics().cap_height;
    assert!((cap(serif) - 0.67).abs() < 1e-9);
    assert!((cap(sans) - 0.656).abs() < 1e-9);
    assert_eq!(store.add(collection, 2), Err(TextError::Unreadable(2)));
    let mut woff = font_bytes(SERIF);
    woff[..4].copy_from_slice(b"wOFF");
    assert_eq!(store.add(woff, 0), Err(TextError::Unreadable(0)));
}

/// Criterion 23: TrueType quadratics become cubics of the same curve, CFF cubics are kept; both
/// within 0.001 mm of the font's outline points scaled to the size.
#[test]
fn ac23_outlines_reproduce_the_font_points() {
    for (path, expect_quads) in [(SERIF, true), (SANS, false)] {
        let (store, key) = store_with(path);
        let face = face(&store, &key);
        let laid = layout(face, "O", Mm(20.0)).unwrap();
        let converted = &laid.glyph_paths(face)[0];
        let bytes = font_bytes(path);
        let font = FontRef::new(&bytes).unwrap();
        let glyph = font.charmap().map('O').unwrap();
        let mut recorder = Recorder::default();
        font.outline_glyphs()
            .get(glyph)
            .unwrap()
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut recorder,
            )
            .unwrap();
        let scale = laid.mm_per_em / face.units_per_em();
        let to_mm = |(x, y): (f32, f32)| Point::new(f64::from(x) * scale, -f64::from(y) * scale);
        assert_eq!(recorder.segments.len(), converted.elements().len());
        assert_eq!(recorder.saw_quad, expect_quads);
        let mut current = Point::ZERO;
        for (segment, element) in recorder.segments.iter().zip(converted.elements()) {
            match (segment, element) {
                (Seg::Move(p), PathEl::MoveTo(q)) | (Seg::Line(p), PathEl::LineTo(q)) => {
                    assert!(to_mm(*p).distance(*q) < 0.001);
                }
                (Seg::Quad(c, p), PathEl::CurveTo(a, b, q)) => {
                    let quad = QuadBez::new(current, to_mm(*c), to_mm(*p));
                    let cubic = CubicBez::new(current, *a, *b, *q);
                    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                        assert!(quad.eval(t).distance(cubic.eval(t)) < 0.001);
                    }
                }
                (Seg::Cubic(c0, c1, p), PathEl::CurveTo(a, b, q)) => {
                    assert!(to_mm(*c0).distance(*a) < 0.001);
                    assert!(to_mm(*c1).distance(*b) < 0.001);
                    assert!(to_mm(*p).distance(*q) < 0.001);
                }
                (Seg::Close, PathEl::ClosePath) => {}
                other => panic!("element mismatch {other:?}"),
            }
            if let Some(end) = element.end_point() {
                current = end;
            }
        }
    }
}

#[derive(Debug)]
enum Seg {
    Move((f32, f32)),
    Line((f32, f32)),
    Quad((f32, f32), (f32, f32)),
    Cubic((f32, f32), (f32, f32), (f32, f32)),
    Close,
}

#[derive(Default)]
struct Recorder {
    segments: Vec<Seg>,
    saw_quad: bool,
}

impl OutlinePen for Recorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.segments.push(Seg::Move((x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.segments.push(Seg::Line((x, y)));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.saw_quad = true;
        self.segments.push(Seg::Quad((cx, cy), (x, y)));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.segments.push(Seg::Cubic((a, b), (c, d), (x, y)));
    }
    fn close(&mut self) {
        self.segments.push(Seg::Close);
    }
}

/// Builds a TrueType collection (`ttcf` 1.0) from whole font files.
fn collection_of(fonts: &[Vec<u8>]) -> Vec<u8> {
    let read_u16 = |b: &[u8], at: usize| usize::from(u16::from_be_bytes([b[at], b[at + 1]]));
    let read_u32 = |b: &[u8], at: usize| {
        usize::try_from(u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])).unwrap()
    };
    let header = 12 + 4 * fonts.len();
    let directories: Vec<usize> = fonts.iter().map(|f| 12 + 16 * read_u16(f, 4)).collect();
    let mut data_at = header + directories.iter().sum::<usize>();
    let mut heads = Vec::new();
    let mut tables = Vec::new();
    let u32_bytes = |v: usize| u32::try_from(v).unwrap().to_be_bytes();
    let mut out = b"ttcf\x00\x01\x00\x00".to_vec();
    out.extend_from_slice(&u32_bytes(fonts.len()));
    let mut dir_at = header;
    for (font, size) in fonts.iter().zip(&directories) {
        out.extend_from_slice(&u32_bytes(dir_at));
        dir_at += size;
        let mut directory = font[..12].to_vec();
        for record in 0..read_u16(font, 4) {
            let at = 12 + 16 * record;
            let (offset, length) = (read_u32(font, at + 8), read_u32(font, at + 12));
            directory.extend_from_slice(&font[at..at + 8]);
            directory.extend_from_slice(&u32_bytes(data_at));
            directory.extend_from_slice(&u32_bytes(length));
            tables.extend_from_slice(&font[offset..offset + length]);
            while tables.len() % 4 != 0 {
                tables.push(0);
            }
            data_at = header + directories.iter().sum::<usize>() + tables.len();
        }
        heads.push(directory);
    }
    for directory in heads {
        out.extend_from_slice(&directory);
    }
    out.extend_from_slice(&tables);
    out
}
