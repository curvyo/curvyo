//! Criterion 27 of `specs/0091-trace-text-recognition`: a line drawn with a single-line face and
//! traced is matched by its centre line (`Recogniser::fit_strokes`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_text_core::{Face, FaceStore, LayoutOptions, Mm, Recogniser, layout};
use kurbo::{BezPath, Stroke, StrokeOpts, Vec2};

fn face(file: &str) -> Arc<Face> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts")
        .join(file);
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn readability() -> Arc<Face> {
    face("ems-readability/EMSReadability.svg")
}

/// `text` in `face` at 10 mm, its strokes drawn `width` mm wide and traced: the closed
/// outlines of the drawing.
fn drawn(face: &Face, text: &str, width: f64) -> Vec<BezPath> {
    let laid = layout(face, text, Mm(10.0), &LayoutOptions::default()).unwrap();
    laid.glyph_paths(face)
        .iter()
        .map(|p| {
            let stroked =
                kurbo::stroke(p.iter(), &Stroke::new(width), &StrokeOpts::default(), 0.005);
            // A trace gives closed outlines.
            let mut closed = BezPath::new();
            for el in stroked.elements() {
                if matches!(el, kurbo::PathEl::MoveTo(_))
                    && !matches!(
                        closed.elements().last(),
                        None | Some(kurbo::PathEl::ClosePath)
                    )
                {
                    closed.close_path();
                }
                closed.push(*el);
            }
            if !matches!(
                closed.elements().last(),
                None | Some(kurbo::PathEl::ClosePath)
            ) {
                closed.close_path();
            }
            closed
        })
        .collect()
}

/// Criterion 27: a line drawn with a single-line face is matched by its centre line, every
/// letter sure in its own face, and comes nearer than another single-line face.
#[test]
fn a_stroke_line_is_set_in_its_single_line_face() {
    let own = readability();
    let outlines = drawn(&own, "Embedded", 0.4);
    let rec = Recogniser::with_hint(&outlines, Mm(0.3), "Embedded");
    let fit = rec
        .fit_strokes(&own, "Embedded", Vec2::new(1.0, 0.0))
        .expect("thin strokes");
    assert_eq!((fit.run.sure, fit.run.letters), (8, 8), "{fit:?}");
    assert_eq!(fit.run.guessed, Vec::<usize>::new());
    assert!(
        (fit.run.size.get() - 10.0).abs() < 0.5,
        "{}",
        fit.run.size.get()
    );
    let other = face("hershey-sans-1/HersheySans1.svg");
    let theirs = rec
        .fit_strokes(&other, "Embedded", Vec2::new(1.0, 0.0))
        .expect("thin strokes");
    assert!(
        fit.score < theirs.score,
        "{} vs {}",
        fit.score,
        theirs.score
    );
    // The stroke it is drawn with is the traced width, within a grid cell.
    assert!((fit.width.get() - 0.4).abs() < 0.1, "{:?}", fit.width);
}

/// Criterion 27: outline faces and thick strokes are no stroke match.
#[test]
fn outline_faces_and_thick_strokes_are_no_stroke_text() {
    let own = readability();
    let thin = Recogniser::with_hint(&drawn(&own, "Embedded", 0.4), Mm(0.3), "Embedded");
    let sans = face("source-sans-3/SourceSans3-Regular.otf");
    assert!(
        thin.fit_strokes(&sans, "Embedded", Vec2::new(1.0, 0.0))
            .is_none()
    );
    let thick = Recogniser::with_hint(&drawn(&own, "Embedded", 2.0), Mm(0.3), "Embedded");
    assert!(
        thick
            .fit_strokes(&own, "Embedded", Vec2::new(1.0, 0.0))
            .is_none()
    );
}

/// `text` in `face` at 10 mm as closed outlines, one per glyph.
fn outlines(face: &Face, text: &str) -> Vec<BezPath> {
    let laid = layout(face, text, Mm(10.0), &LayoutOptions::default()).unwrap();
    laid.glyph_paths(face)
}

/// Round 3: a bolder cut has wider strokes, and only an italic leans clearly.
#[test]
fn stroke_width_and_slant_tell_weight_and_italic() {
    let text = "Embedded Freelancer";
    let width = |file: &str| {
        let f = face(file);
        Recogniser::new(&outlines(&f, text), Mm(0.3))
            .stroke_width(Vec2::new(1.0, 0.0))
            .unwrap()
            .get()
    };
    let (light, regular, bold) = (
        width("source-sans-3/SourceSans3-Light.otf"),
        width("source-sans-3/SourceSans3-Regular.otf"),
        width("source-sans-3/SourceSans3-Bold.otf"),
    );
    assert!(
        light < regular && regular < bold,
        "{light} {regular} {bold}"
    );
    let slant = |file: &str| {
        let f = face(file);
        Recogniser::new(&outlines(&f, text), Mm(0.3)).slant(Vec2::new(1.0, 0.0))
    };
    let upright = slant("source-sans-3/SourceSans3-Regular.otf");
    assert!(upright.abs() < 2.0, "{upright}");
    let italic = slant("source-sans-3/SourceSans3-It.otf");
    assert!(italic > 6.0, "{italic}");
}
