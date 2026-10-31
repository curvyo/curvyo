//! Recognise text over the face tiers (`specs/0088-trace-recognition` criteria 3, 4, 6, 8 and 9;
//! `adrs.md` decisions 3a, 5, 6): the tier stop, the earliest tier winning, faces asked for only
//! when a line needs their tier, and the write plan.
#![allow(clippy::unwrap_used, missing_docs)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Document, FontKind, FontRef, NewAnchor, NodeId, ObjectSnapshot, Point, StyleEdit, Vec2,
};
use curvyo_text_core::{Face, FaceKey, FaceStore, LayoutOptions, Mm, layout};
use curvyo_ui_core::{
    AnchorIdMinter, FaceState, FaceTier, FontBlock, JobStep, ListedFace, RecogniseJob,
    plan_recognise_write, recognise_available, recognise_subjects, recognise_tiers,
};
use kurbo::{Affine, BezPath, PathEl};

fn font_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts")
        .join(file)
}

/// A listing and a store: `(block, family, style, file)`.
struct Library {
    store: FaceStore,
    listed: Vec<ListedFace>,
}

fn library(entries: &[(FontBlock, &str, &str, &str)]) -> Library {
    let mut store = FaceStore::new();
    let listed = entries
        .iter()
        .map(|(block, family, style, file)| {
            let key = store
                .add(std::fs::read(font_path(file)).unwrap(), 0)
                .unwrap();
            ListedFace {
                block: *block,
                family: (*family).to_owned(),
                style: (*style).to_owned(),
                host_id: (*file).to_owned(),
                index: 0,
                key: Some(key),
                kind: FontKind::Outline,
            }
        })
        .collect();
    Library { store, listed }
}

impl Library {
    fn face(&self, index: usize) -> Arc<Face> {
        let key: FaceKey = self.listed[index].key.unwrap();
        self.store.get(&key).unwrap().clone()
    }

    fn font(&self, index: usize) -> FontRef {
        let face = &self.listed[index];
        let key = face.key.unwrap();
        FontRef {
            family: face.family.clone(),
            style: face.style.clone(),
            face_hash: key.hash.to_hex(),
            face_index: 0,
            kind: FontKind::Outline,
        }
    }
}

/// `text` laid out in `face` at cap height `size`, one closed subpath per outline.
fn traced(face: &Face, text: &str, size: f64, at: (f64, f64)) -> Vec<BezPath> {
    let laid = layout(face, text, Mm(size), &LayoutOptions::default()).unwrap();
    let mut out: Vec<BezPath> = Vec::new();
    for glyph in &laid.lines[0].glyphs {
        let local = Affine::translate((at.0 + glyph.origin.x, at.1 + glyph.origin.y))
            * Affine::scale(laid.mm_per_em);
        for element in (local * face.outline(glyph.glyph).path.clone()).elements() {
            if let PathEl::MoveTo(_) = element {
                out.push(BezPath::new());
            }
            if let Some(last) = out.last_mut() {
                last.push(*element);
            }
        }
    }
    out
}

/// Runs a job to its end with every face ready, recording the faces asked about.
fn run(job: &mut RecogniseJob, lib: &Library) -> Vec<usize> {
    let asked = RefCell::new(Vec::new());
    loop {
        let step = job.step(&|index| {
            asked.borrow_mut().push(index);
            FaceState::Ready(lib.face(index))
        });
        match step {
            JobStep::Finished => break,
            JobStep::NeedFaces(_) => panic!("every face is ready"),
            JobStep::Running { .. } => {}
        }
    }
    let mut asked = asked.into_inner();
    asked.sort_unstable();
    asked.dedup();
    asked
}

const SANS_REGULAR: &str = "source-sans-3/SourceSans3-Regular.otf";
const SANS_LIGHT: &str = "source-sans-3/SourceSans3-Light.otf";
const SERIF_REGULAR: &str = "source-serif-4/SourceSerif4-Regular.ttf";
const SERIF_BOLD: &str = "source-serif-4/SourceSerif4-Bold.ttf";

#[test]
fn criterion_8_a_line_in_a_document_face_never_reads_a_later_tier() {
    let lib = library(&[
        (
            FontBlock::Included,
            "Source Sans 3",
            "Regular",
            SANS_REGULAR,
        ),
        (
            FontBlock::Included,
            "Source Serif 4",
            "Regular",
            SERIF_REGULAR,
        ),
        (FontBlock::System, "Source Serif 4", "Bold", SERIF_BOLD),
    ]);
    let outlines = traced(&lib.face(1), "Recognise", 10.0, (10.0, 20.0));
    let tiers = recognise_tiers(&lib.listed, &[lib.font(1)], &[]);
    assert_eq!(tiers.faces[0], vec![1]);
    let mut job = RecogniseJob::new(&outlines, Mm(0.3), tiers);
    let asked = run(&mut job, &lib);
    assert_eq!(asked, vec![1], "only the document's face is read");
    let accepted = job.accepted();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].tier, FaceTier::Document);
    assert_eq!(accepted[0].run.text, "Recognise");
}

#[test]
fn criterion_8_a_face_only_in_the_system_fonts_is_found_in_tier_5() {
    let lib = library(&[
        (
            FontBlock::Included,
            "Source Sans 3",
            "Regular",
            SANS_REGULAR,
        ),
        (FontBlock::System, "Source Serif 4", "Bold", SERIF_BOLD),
    ]);
    let outlines = traced(&lib.face(1), "Recognise", 10.0, (10.0, 20.0));
    let mut job = RecogniseJob::new(&outlines, Mm(0.3), recognise_tiers(&lib.listed, &[], &[]));
    run(&mut job, &lib);
    let accepted = job.accepted();
    assert_eq!(accepted.len(), 1, "{accepted:#?}");
    assert_eq!(accepted[0].tier, FaceTier::System);
    assert_eq!(accepted[0].face, 1);
    assert_eq!(job.last_tier(), Some(FaceTier::System));
}

#[test]
fn criterion_9_the_earlier_tier_wins_when_both_reach_80_percent() {
    // Source Sans 3 Regular at 5 mm: its Light is within the tolerance too (criterion 3).
    let lib = library(&[
        (
            FontBlock::Included,
            "Source Sans 3",
            "Regular",
            SANS_REGULAR,
        ),
        (
            FontBlock::Included,
            "Source Sans 3 Light",
            "Light",
            SANS_LIGHT,
        ),
    ]);
    let outlines = traced(&lib.face(0), "Laser 3 mm", 5.0, (10.0, 20.0));
    // The Light family is a favourite: tier 2 before the included tier 4.
    let tiers = recognise_tiers(&lib.listed, &[], &["Source Sans 3 Light".to_owned()]);
    assert_eq!(tiers.faces[1], vec![1]);
    assert_eq!(tiers.faces[3], vec![0]);
    let mut job = RecogniseJob::new(&outlines, Mm(0.3), tiers);
    let asked = run(&mut job, &lib);
    assert_eq!(asked, vec![1], "the included tier is not searched");
    assert_eq!(job.accepted()[0].tier, FaceTier::Favourites);
    assert_eq!(job.accepted()[0].face, 1);
}

#[test]
fn faces_not_loaded_are_asked_for_before_the_tier_is_searched() {
    let lib = library(&[(
        FontBlock::Included,
        "Source Sans 3",
        "Regular",
        SANS_REGULAR,
    )]);
    let outlines = traced(&lib.face(0), "Hello", 10.0, (10.0, 20.0));
    let mut job = RecogniseJob::new(&outlines, Mm(0.3), recognise_tiers(&lib.listed, &[], &[]));
    assert_eq!(
        job.step(&|_| FaceState::Loading),
        JobStep::NeedFaces(vec![0])
    );
    // A cold face is made ready on a step of its own (architect review: the host paints and can
    // stop between faces); the next step searches the line.
    assert!(!lib.face(0).recognition_ready());
    assert_eq!(
        job.step(&|_| FaceState::Ready(lib.face(0))),
        JobStep::Running { done: 0, total: 1 }
    );
    assert!(lib.face(0).recognition_ready());
    assert_eq!(
        job.step(&|_| FaceState::Ready(lib.face(0))),
        JobStep::Running { done: 1, total: 1 }
    );
    assert_eq!(job.step(&|_| FaceState::Unavailable), JobStep::Finished);
    assert_eq!(job.accepted()[0].run.text, "Hello");
}

#[test]
fn criterion_4_nothing_within_the_tolerance_names_the_nearest_face() {
    let lib = library(&[
        (
            FontBlock::Included,
            "Source Sans 3",
            "Regular",
            SANS_REGULAR,
        ),
        (FontBlock::Included, "Source Serif 4", "Bold", SERIF_BOLD),
    ]);
    let great_vibes = {
        let mut store = FaceStore::new();
        let key = store
            .add(
                std::fs::read(font_path("great-vibes/GreatVibes-Regular.ttf")).unwrap(),
                0,
            )
            .unwrap();
        store.get(&key).unwrap().clone()
    };
    let outlines = traced(&great_vibes, "Curvyo", 10.0, (10.0, 20.0));
    let mut job = RecogniseJob::new(&outlines, Mm(0.3), recognise_tiers(&lib.listed, &[], &[]));
    run(&mut job, &lib);
    assert_eq!(job.accepted().len(), 0);
    let (_, deviation) = job.nearest().unwrap();
    assert!(deviation > 0.3);
}

/// The anchors of one closed subpath with its handles.
fn anchors_of(outline: &BezPath, minter: &mut AnchorIdMinter) -> Vec<NewAnchor> {
    let mut anchors: Vec<NewAnchor> = Vec::new();
    let mut start = kurbo::Point::ORIGIN;
    for element in outline.elements() {
        match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                start = if anchors.is_empty() { p } else { start };
                anchors.push(NewAnchor::corner(minter.mint(), Point::new(p.x, p.y)));
            }
            PathEl::QuadTo(c, p) => {
                let from = anchors.last().unwrap().point;
                let from = kurbo::Point::new(from.x, from.y);
                let c1 = from + (c - from) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                push_curve(&mut anchors, minter, c1, c2, p);
            }
            PathEl::CurveTo(c1, c2, p) => push_curve(&mut anchors, minter, c1, c2, p),
            PathEl::ClosePath => {}
        }
    }
    // A closing segment that ends on the start: its handle goes to the first anchor.
    if anchors.len() > 2 {
        let last = anchors[anchors.len() - 1];
        if (last.point.x - start.x).abs() < 1e-9 && (last.point.y - start.y).abs() < 1e-9 {
            anchors[0].handle_in = last.handle_in;
            anchors.pop();
        }
    }
    anchors
}

fn push_curve(
    anchors: &mut Vec<NewAnchor>,
    minter: &mut AnchorIdMinter,
    c1: kurbo::Point,
    c2: kurbo::Point,
    p: kurbo::Point,
) {
    let previous = anchors.last_mut().unwrap();
    previous.handle_out = Vec2::new(c1.x - previous.point.x, c1.y - previous.point.y);
    let mut anchor = NewAnchor::corner(minter.mint(), Point::new(p.x, p.y));
    anchor.handle_in = Vec2::new(c2.x - p.x, c2.y - p.y);
    anchors.push(anchor);
}

/// A document with one filled compound path of `outlines` (one subpath each).
fn document_with(outlines: &[BezPath]) -> (Document, NodeId) {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let subpaths: Vec<(Vec<NewAnchor>, bool)> = outlines
        .iter()
        .map(|o| (anchors_of(o, &mut minter), true))
        .collect();
    let seed = document.create_path(
        &[
            NewAnchor::corner(minter.mint(), Point::new(900.0, 900.0)),
            NewAnchor::corner(minter.mint(), Point::new(901.0, 900.0)),
        ],
        false,
    );
    let id = document
        .replace_with_path(&[seed], seed, &subpaths, "boolean_union")
        .unwrap();
    document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    (document, id)
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

#[test]
fn the_write_keeps_what_was_not_recognised_and_takes_the_path_style() {
    let lib = library(&[(
        FontBlock::Included,
        "Source Sans 3",
        "Regular",
        SANS_REGULAR,
    )]);
    let mut outlines = traced(&lib.face(0), "Hello", 10.0, (10.0, 20.0));
    // A square far below the line: no letter.
    let mut square = BezPath::new();
    square.move_to((10.0, 80.0));
    square.line_to((40.0, 80.0));
    square.line_to((40.0, 110.0));
    square.line_to((10.0, 110.0));
    square.close_path();
    outlines.push(square);
    let (document, id) = document_with(&outlines);
    let objects = objects(&document);
    assert!(recognise_available(&objects, &[id], &|_| false));
    let subjects = recognise_subjects(&objects, &[id], &|_| false);
    assert_eq!(subjects.outlines.len(), outlines.len());
    let mut job = RecogniseJob::new(
        &subjects.outlines,
        Mm(0.3),
        recognise_tiers(&lib.listed, &[], &[]),
    );
    run(&mut job, &lib);
    let runs: Vec<_> = job
        .accepted()
        .iter()
        .map(|line| (line.run.clone(), lib.font(line.face)))
        .collect();
    let write = plan_recognise_write(&objects, &subjects, &runs);
    assert_eq!(write.texts.len(), 1);
    assert_eq!(write.texts[0].string, "Hello");
    assert_eq!(write.texts[0].source, id);
    assert_eq!(&write.texts[0].style, objects[0].style());
    assert_eq!(write.removed, vec![id]);
    assert_eq!(write.pieces.len(), 1);
    assert_eq!(write.pieces[0].1.len(), 1, "the square stays");
    assert_eq!(write.left, 1);
}
