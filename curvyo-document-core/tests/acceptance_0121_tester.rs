//! Tester acceptance cases for `specs/0121-print-and-cut` slice 1 on the document side: the
//! marks object (criteria 1 to 3, 6), the measures register (9, 10, 19), Apply and Reset (16,
//! 18) and format 55 (ADR 0081 §9), written from the specification before the implementation
//! was read. Black box through the public `Document` API.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    missing_docs
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, Document, DocumentSize, Length, ManufacturingError,
    NewOutputPage, NodeId, ObjectEditError, ObjectSnapshot, Point, PrintAndCutMeasures,
    PrintAndCutRegister, RectBounds, RegistrationMarkSnapshot, Vec2, pack,
    registration_mark_corners, unpack,
};

const A4: (Point, Point) = (Point::new(0.0, 0.0), Point::new(210.0, 297.0));

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let m: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    m["format_version"].as_u64().unwrap()
}

fn steps(d: &Document) -> usize {
    d.own_steps().len()
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo(d: &Document) {
    undo_nth(d, 0);
}

/// Undoes the `k`-th last own step that is not itself an undo.
fn undo_nth(d: &Document, k: usize) {
    let own: Vec<_> = d
        .own_steps()
        .into_iter()
        .filter(|s| s.reverts.is_none())
        .collect();
    d.undo_step(own[own.len() - 1 - k].step).unwrap();
}

fn mark_centres(d: &Document) -> Vec<(u8, f64, f64)> {
    d.marks_objects_on(&d.pages()[0])
        .remove(0)
        .marks
        .iter()
        .map(|m| (m.number, m.centre.x, m.centre.y))
        .collect()
}

/// A drawing page with a small design and marks, placed on a 400 x 300 laser page.
struct Sheet {
    d: Document,
    laser: String,
    placement: NodeId,
    marks: NodeId,
}

fn sheet() -> Sheet {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(60.0, 60.0),
        Point::new(100.0, 100.0),
    ));
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap()
        .id;
    let marks = d
        .insert_registration_marks(registration_mark_corners(
            A4,
            Some((Point::new(60.0, 60.0), Point::new(100.0, 100.0))),
        ))
        .unwrap();
    let placement = d
        .place_page(
            &laser,
            &drawing.id,
            Affine::translation(Vec2::new(20.0, 0.0)),
        )
        .unwrap();
    Sheet {
        d,
        laser,
        placement,
        marks,
    }
}

fn measures(d: &Document, id: NodeId) -> PrintAndCutMeasures {
    match d.print_and_cut(id).unwrap() {
        PrintAndCutRegister::Measures(m) => m,
        other => panic!("{other:?}"),
    }
}

/// Every object except placements, and the drawing page, as text: what "the drawing" is.
fn drawing_text(d: &Document) -> String {
    let placed: Vec<NodeId> = d.placed_pages().iter().map(|p| p.id).collect();
    let objects: Vec<String> = d
        .read_objects()
        .objects
        .iter()
        .filter(|o| !placed.contains(&o.id()))
        .map(|o| format!("{o:?}"))
        .collect();
    format!("{:?}\n{}", d.pages()[0], objects.join("\n"))
}

// ---- criterion 1: where the marks go --------------------------------------------------------

#[test]
fn c1_an_empty_page_and_a_small_design_get_the_page_corners_inset_10_mm() {
    let page = [
        Point::new(10.0, 10.0),
        Point::new(200.0, 10.0),
        Point::new(200.0, 287.0),
        Point::new(10.0, 287.0),
    ];
    assert_eq!(registration_mark_corners(A4, None), page, "empty page");
    let small = (Point::new(80.0, 100.0), Point::new(130.0, 180.0));
    assert_eq!(
        registration_mark_corners(A4, Some(small)),
        page,
        "small design"
    );
}

#[test]
fn c1_a_full_page_design_gets_its_box_grown_and_clamped_to_10_mm_inside_the_page() {
    // A full-page design: the grown box leaves the page everywhere, so it clamps to the inset.
    let full = (Point::new(2.0, 2.0), Point::new(208.0, 295.0));
    assert_eq!(
        registration_mark_corners(A4, Some(full)),
        [
            Point::new(10.0, 10.0),
            Point::new(200.0, 10.0),
            Point::new(200.0, 287.0),
            Point::new(10.0, 287.0),
        ]
    );
    // A design in the lower right quarter that reaches the corner: the candidate (200, 287)
    // lies inside it, so all four go to its box grown by 10 mm, clamped where it leaves.
    let quarter = (Point::new(100.0, 100.0), Point::new(205.0, 292.0));
    assert_eq!(
        registration_mark_corners(A4, Some(quarter)),
        [
            Point::new(90.0, 90.0),
            Point::new(200.0, 90.0),
            Point::new(200.0, 287.0),
            Point::new(90.0, 287.0),
        ]
    );
}

#[test]
fn c1_insert_is_one_step_of_four_numbered_marks_in_one_group() {
    let d = Document::new(1);
    let before = steps(&d);
    let group = d
        .insert_registration_marks(registration_mark_corners(A4, None))
        .unwrap();
    assert_eq!(steps(&d), before + 1, "one undoable step");
    let object = d.marks_object(group).expect("the marks object");
    assert_eq!(object.id, group);
    let numbers: Vec<u8> = object.marks.iter().map(|m| m.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    assert_eq!(mark_centres(&d)[0], (1, 10.0, 10.0));
    for m in &object.marks {
        assert_eq!(m.diameter.as_mm(), 6.0, "criterion 2 default");
        assert_eq!(m.line.as_mm(), 0.25);
        assert_eq!(d.marks_object_of(m.id).unwrap().id, group);
    }
    undo(&d);
    assert!(d.marks_objects_on(&d.pages()[0]).is_empty());
}

// ---- criterion 2: size, line, move only ----------------------------------------------------

#[test]
fn c2_diameter_and_line_change_every_mark_in_one_step_within_their_ranges() {
    let s = sheet();
    let before = steps(&s.d);
    s.d.set_registration_mark_size(s.marks, Length::from_mm(10.0), Length::from_mm(0.5))
        .unwrap();
    assert_eq!(steps(&s.d), before + 1);
    for m in s.d.marks_object(s.marks).unwrap().marks {
        assert_eq!((m.diameter.as_mm(), m.line.as_mm()), (10.0, 0.5));
    }
    for (dia, line) in [(2.9, 0.25), (20.1, 0.25), (6.0, 0.04), (6.0, 1.01)] {
        assert!(
            s.d.set_registration_mark_size(s.marks, Length::from_mm(dia), Length::from_mm(line))
                .is_err(),
            "{dia} / {line}"
        );
    }
    for (dia, line) in [(3.0, 0.05), (20.0, 1.0)] {
        assert!(
            s.d.set_registration_mark_size(s.marks, Length::from_mm(dia), Length::from_mm(line))
                .is_ok(),
            "{dia} / {line}"
        );
    }
    assert_eq!(steps(&s.d), before + 3, "refused writes add no step");
}

#[test]
fn c2_a_transform_moves_the_centre_only() {
    let s = sheet();
    let any = s.d.marks_object(s.marks).unwrap().marks[0].id;
    let mark = RegistrationMarkSnapshot::new(
        any,
        2,
        Point::new(50.0, 40.0),
        Length::from_mm(6.0),
        Length::from_mm(0.25),
    );
    let turned_scaled = Affine::rotation_about(Point::new(0.0, 0.0), Angle::from_radians(0.7))
        .compose(Affine::scale(3.0, 0.5));
    let mapped = mark.mapped_by(turned_scaled);
    let want = turned_scaled.apply_point(mark.centre);
    assert!((mapped.centre.x - want.x).abs() < 1e-9 && (mapped.centre.y - want.y).abs() < 1e-9);
    assert_eq!(mapped.diameter, mark.diameter, "never scaled");
    assert_eq!(mapped.line, mark.line);
    assert_eq!(mapped.number, 2, "the number stays");
    // A move of one mark in the document moves only its centre and keeps its number.
    let first = s.d.marks_object(s.marks).unwrap().marks[0].clone();
    s.d.translate_objects(&[first.id], Vec2::new(3.0, 4.0))
        .unwrap();
    let moved = s.d.marks_object(s.marks).unwrap().marks[0].clone();
    assert_eq!(moved.number, first.number);
    assert_eq!(
        (moved.centre.x, moved.centre.y),
        (first.centre.x + 3.0, first.centre.y + 4.0)
    );
    assert_eq!(moved.diameter, first.diameter);
}

// ---- criterion 3: 2 to 4 marks, the delete rule -------------------------------------------

#[test]
fn c3_delete_below_two_marks_is_refused_and_the_object_deletes_whole() {
    let s = sheet();
    let ids: Vec<NodeId> =
        s.d.marks_object(s.marks)
            .unwrap()
            .marks
            .iter()
            .map(|m| m.id)
            .collect();
    s.d.delete_objects(&ids[..2]).unwrap();
    assert_eq!(s.d.marks_object(s.marks).unwrap().marks.len(), 2);
    let before = steps(&s.d);
    let refused = s.d.delete_objects(&ids[2..3]);
    assert_eq!(refused, Err(ObjectEditError::MarksBelowTwo));
    assert_eq!(
        ObjectEditError::MarksBelowTwo.to_string(),
        "Registration marks need at least 2 marks."
    );
    assert_eq!(steps(&s.d), before, "nothing written");
    assert_eq!(s.d.marks_object(s.marks).unwrap().marks.len(), 2);
    // Add mark brings back the lowest free number, up to 4.
    assert_eq!(
        s.d.marks_object(s.marks).unwrap().lowest_free_number(),
        Some(1)
    );
    s.d.add_registration_mark(s.marks, registration_mark_corners(A4, None))
        .unwrap();
    s.d.add_registration_mark(s.marks, registration_mark_corners(A4, None))
        .unwrap();
    assert_eq!(s.d.marks_object(s.marks).unwrap().marks.len(), 4);
    assert!(
        s.d.add_registration_mark(s.marks, registration_mark_corners(A4, None))
            .is_err(),
        "never a 5th"
    );
    s.d.delete_objects(&[s.marks]).unwrap();
    assert!(s.d.marks_objects_on(&s.d.pages()[0]).is_empty());
    let remaining = s.d.read_objects();
    assert!(
        !remaining
            .objects
            .iter()
            .any(|o| matches!(o, ObjectSnapshot::RegistrationMark(_))),
        "no orphan marks"
    );
}

// ---- criteria 9, 10, 19: the register -------------------------------------------------------

#[test]
fn c9_each_measure_and_clear_is_one_step() {
    let s = sheet();
    assert_eq!(
        s.d.print_and_cut(s.placement),
        Some(PrintAndCutRegister::Absent),
        "a project without measures"
    );
    let before = steps(&s.d);
    s.d.set_print_and_cut_measure(s.placement, 1, Some([30_000, 290_000]))
        .unwrap();
    s.d.set_print_and_cut_measure(s.placement, 2, Some([220_000, -5_250]))
        .unwrap();
    assert_eq!(steps(&s.d), before + 2);
    assert_eq!(measures(&s.d, s.placement).marks[&2], [220_000, -5_250]);
    s.d.set_print_and_cut_measure(s.placement, 1, None).unwrap();
    assert_eq!(steps(&s.d), before + 3, "Clear is one step");
    assert!(!measures(&s.d, s.placement).marks.contains_key(&1));
    undo(&s.d);
    assert_eq!(measures(&s.d, s.placement).marks[&1], [30_000, 290_000]);
    assert!(
        s.d.set_print_and_cut_measure(s.placement, 5, Some([0, 0]))
            .is_err()
    );
    s.d.set_print_and_cut_tolerance(s.placement, 400).unwrap();
    assert_eq!(measures(&s.d, s.placement).tolerance_um, 400);
    assert!(s.d.set_print_and_cut_tolerance(s.placement, 49).is_err());
    assert!(s.d.set_print_and_cut_tolerance(s.placement, 5001).is_err());
}

#[test]
fn c10_an_unreadable_stored_measure_is_replaced_by_the_next_write() {
    let bytes = std::fs::read(fixture("print_and_cut_damaged_v55.curvyo")).unwrap();
    let d = unpack(2, &bytes).unwrap();
    let unreadable = d
        .placed_pages()
        .iter()
        .map(|p| p.id)
        .find(|&id| d.print_and_cut(id) == Some(PrintAndCutRegister::Unreadable))
        .expect("the damaged golden has an unreadable register");
    d.set_print_and_cut_measure(unreadable, 2, Some([1_000, 2_000]))
        .unwrap();
    let m = measures(&d, unreadable);
    assert_eq!(m.marks.len(), 1);
    assert_eq!(m.marks[&2], [1_000, 2_000]);
}

#[test]
fn c19_measures_survive_save_and_reopen_and_saves_are_format_55() {
    let s = sheet();
    s.d.set_print_and_cut_measure(s.placement, 1, Some([30_000, 290_000]))
        .unwrap();
    s.d.set_print_and_cut_measure(s.placement, 3, Some([210_000, 10_000]))
        .unwrap();
    s.d.set_print_and_cut_tolerance(s.placement, 700).unwrap();
    let fit = Affine::translation(Vec2::new(25.0, 3.0));
    s.d.apply_print_and_cut(s.placement, fit).unwrap();
    let bytes = pack(&s.d, "0.1.0").unwrap();
    assert_eq!(CURRENT_FORMAT_VERSION, 55);
    assert_eq!(manifest_version(&bytes), 55);
    let back = unpack(3, &bytes).unwrap();
    let m = measures(&back, s.placement);
    assert_eq!(m, measures(&s.d, s.placement));
    assert_eq!(m.tolerance_um, 700);
    assert_eq!(m.reset, Some(Affine::translation(Vec2::new(20.0, 0.0))));
    assert_eq!(back.placed_page(s.placement).unwrap().matrix, fit);
}

#[test]
fn c19_a_copy_of_a_placed_sheet_does_not_copy_its_measures() {
    let s = sheet();
    s.d.set_print_and_cut_measure(s.placement, 1, Some([30_000, 290_000]))
        .unwrap();
    let copies =
        s.d.duplicate_placements(&[s.placement], Vec2::new(5.0, 5.0), "duplicate")
            .unwrap();
    assert_eq!(
        s.d.print_and_cut(copies[0]),
        Some(PrintAndCutRegister::Absent)
    );
    assert_eq!(
        measures(&s.d, s.placement).marks.len(),
        1,
        "the original keeps"
    );
}

#[test]
fn c19_the_goldens_are_format_55_and_open() {
    for name in [
        "print_and_cut_v55.curvyo",
        "print_and_cut_damaged_v55.curvyo",
    ] {
        let bytes = std::fs::read(fixture(name)).unwrap();
        assert_eq!(manifest_version(&bytes), 55, "{name}");
        let d = unpack(2, &bytes).unwrap();
        assert!(!d.marks_objects_on(&d.pages()[0]).is_empty(), "{name}");
        assert_eq!(
            members(&pack(&d, "0.1.0").unwrap())["document.json"],
            members(&bytes)["document.json"],
            "{name} saves back unchanged"
        );
    }
}

// ---- criterion 16: Apply and Reset ---------------------------------------------------------

#[test]
fn c16_apply_and_reset_are_one_step_each_and_leave_the_drawing_untouched() {
    let s = sheet();
    s.d.set_print_and_cut_measure(s.placement, 1, Some([30_000, 290_000]))
        .unwrap();
    let drawing = drawing_text(&s.d);
    let before = s.d.placed_page(s.placement).unwrap().matrix;
    let fit = Affine::translation(Vec2::new(83.4, 41.2)).compose(Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(1.8_f64.to_radians()),
    ));
    let n = steps(&s.d);
    s.d.apply_print_and_cut(s.placement, fit).unwrap();
    assert_eq!(steps(&s.d), n + 1, "Apply is one step");
    assert_eq!(last_label(&s.d), "apply_print_and_cut");
    assert_eq!(s.d.placed_page(s.placement).unwrap().matrix, fit);
    assert_eq!(measures(&s.d, s.placement).reset, Some(before));
    assert_eq!(drawing_text(&s.d), drawing, "the drawing is not changed");
    s.d.reset_print_and_cut(s.placement).unwrap();
    assert_eq!(steps(&s.d), n + 2, "Reset is one step");
    assert_eq!(last_label(&s.d), "reset_print_and_cut");
    assert_eq!(
        s.d.placed_page(s.placement).unwrap().matrix,
        before,
        "exactly"
    );
    assert_eq!(measures(&s.d, s.placement).reset, None);
    assert_eq!(measures(&s.d, s.placement).marks.len(), 1, "measures kept");
    assert_eq!(drawing_text(&s.d), drawing);
    undo_nth(&s.d, 0);
    assert_eq!(s.d.placed_page(s.placement).unwrap().matrix, fit);
    undo_nth(&s.d, 1);
    assert_eq!(s.d.placed_page(s.placement).unwrap().matrix, before);
    assert_eq!(measures(&s.d, s.placement).reset, None);
    assert_eq!(drawing_text(&s.d), drawing);
}

#[test]
fn c16_a_fit_off_the_laser_page_is_refused_and_writes_nothing() {
    let s = sheet();
    let before = s.d.placed_page(s.placement).unwrap().matrix;
    let n = steps(&s.d);
    let far = Affine::translation(Vec2::new(5_000.0, 5_000.0));
    assert!(matches!(
        s.d.apply_print_and_cut(s.placement, far),
        Err(ManufacturingError::OffPage)
    ));
    assert_eq!(steps(&s.d), n);
    assert_eq!(s.d.placed_page(s.placement).unwrap().matrix, before);
    let _ = &s.laser;
}
