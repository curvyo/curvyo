//! Tester acceptance cases for `specs/0121-print-and-cut` slice 1 through the public session:
//! Add registration marks (criterion 1), export and job (4, 5), the Print and cut group (7 to
//! 10, 12, 14), Apply and Reset as one undo step each with the drawing unchanged (16, 18), Save
//! and reopen (19). Written from the specification before the implementation was read; every
//! job is read back by the 0029 tester's own GRBL reader.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_grbl_0029/mod.rs"]
mod grbl;

use curvyo_document_core::{
    Affine, Angle, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Page, Point,
    RectBounds, Timestamp, Vec2, pack, registration_mark_corners, unpack,
};
use curvyo_editor_wasm::{Session, SvgExportResult, Tool};
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::Frame;
use curvyo_laser_core::print_and_cut::machine_mm;
use curvyo_ui_core::SvgExportOptions;
use serde_json::Value;

const A4: (Point, Point) = (Point::new(0.0, 0.0), Point::new(210.0, 297.0));

struct Fx {
    s: Session,
    drawing: Page,
    laser: Page,
    placement: NodeId,
}

/// A drawing page with a 40 mm square at (60, 60) and (optionally) marks, placed on a
/// 600 x 400 mm laser page at (20, 0), with one Cut over the placed sheet.
fn fixture(with_marks: bool) -> Fx {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(60.0, 60.0),
        Point::new(100.0, 100.0),
    ));
    if with_marks {
        d.insert_registration_marks(registration_mark_corners(
            A4,
            Some((Point::new(60.0, 60.0), Point::new(100.0, 100.0))),
        ))
        .unwrap();
    }
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap();
    let laser = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let placement = d
        .place_page(
            &laser.id,
            &drawing.id,
            Affine::translation(Vec2::new(20.0, 0.0)),
        )
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    // Cut over everything: the placed sheet, and (when present) the marks object and each mark.
    let mut keys = vec![vec![placement]];
    if with_marks {
        let object = d.marks_objects_on(&drawing).remove(0);
        keys.push(vec![placement, object.id]);
        for mark in &object.marks {
            keys.push(vec![placement, mark.id]);
        }
    }
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &keys,
        )
        .or_else(|_| {
            d.add_operation(
                m,
                &NewOperation {
                    kind: "cut",
                    name: "Cut 1",
                },
                0,
                &[vec![placement]],
            )
        })
        .unwrap();
    let mut s = Session::open(7, &pack(&d, "0121-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    s.load_materials(&[], None);
    assert!(s.activate_page(&laser.id));
    let _ = s.edit_operation_setting(op, "main", "power", 60.0);
    let _ = s.edit_operation_setting(op, "main", "speed", 600.0);
    s.wipe_history();
    Fx {
        s,
        drawing,
        laser,
        placement,
    }
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.print_and_cut_view().expect("the Print and cut group")).unwrap()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn svg_of_page(s: &Session) -> String {
    let SvgExportResult::Written { text, .. } = s.export_svg(SvgExportOptions {
        text_as_paths: false,
        strokes_as_paths: false,
        selection_only: false,
        active_page_only: true,
    }) else {
        panic!("written");
    };
    text
}

fn frame() -> Frame {
    Frame {
        corner: OriginCorner::BottomLeft,
        width: 600_000,
        height: 400_000,
    }
}

/// Types in every mark where `fit` (sheet page-local to laser page-local) puts it, plus noise.
fn measure_by(fx: &mut Fx, fit: Affine, which: &[u8], noise: &[(u8, f64, f64)]) {
    let d = doc(&fx.s);
    let object = d.marks_objects_on(&fx.drawing).remove(0);
    for mark in object.marks {
        if !which.contains(&mark.number) {
            continue;
        }
        let local = Point::new(
            mark.centre.x - fx.drawing.origin.x,
            mark.centre.y - fx.drawing.origin.y,
        );
        let m = machine_mm(frame(), fit.apply_point(local));
        let (dx, dy) = noise
            .iter()
            .find(|e| e.0 == mark.number)
            .map_or((0.0, 0.0), |e| (e.1, e.2));
        assert!(fx.s.print_and_cut_measure(mark.number, m.x + dx, m.y + dy));
    }
}

fn the_fit() -> Affine {
    Affine::translation(Vec2::new(83.4, 41.2)).compose(Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(1.8_f64.to_radians()),
    ))
}

// ---- criterion 1 ------------------------------------------------------------------------------

#[test]
fn c1_add_registration_marks_on_a_drawing_page_is_one_step_and_absent_on_a_laser_page() {
    let mut s = Session::new(121_901);
    assert!(s.registration_marks_offered());
    let n = s.undo_len();
    assert!(s.add_registration_marks());
    assert_eq!(s.undo_len(), n + 1);
    let v: Value = serde_json::from_str(&s.registration_marks_view().unwrap()).unwrap();
    assert_eq!(v["count"], 4, "{v}");
    let _ = s.undo();
    let d = unpack(2, &s.pack("0.1.0").unwrap()).unwrap();
    assert!(d.marks_objects_on(&d.pages()[0]).is_empty(), "undone whole");
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(!s.registration_marks_offered(), "only on a Drawing page");
    assert!(!s.add_registration_marks());
}

// ---- criterion 4: export and job ------------------------------------------------------------

#[test]
fn c4_the_svg_export_of_the_sheet_has_every_mark() {
    let mut fx = fixture(true);
    assert!(fx.s.activate_page(&fx.drawing.id));
    let text = svg_of_page(&fx.s);
    for n in 1..=4 {
        assert!(
            text.contains(&format!("data-curvyo-mark=\"{n}\"")),
            "mark {n}: {text}"
        );
    }
    assert_eq!(
        text.matches("data-curvyo-kind=\"registration-mark\"")
            .count(),
        4
    );
}

#[test]
fn c4_no_mark_contributes_a_burn_move_under_a_cut_over_everything() {
    let mut with = fixture(true);
    let mut without = fixture(false);
    let a = with.s.export_job("Untitled.curvyo").expect("export").bytes;
    let b = without
        .s
        .export_job("Untitled.curvyo")
        .expect("export")
        .bytes;
    let pa = grbl::read(&a).unwrap();
    let pb = grbl::read(&b).unwrap();
    assert!(pa.burn_mm() > 150.0, "the square burns: {}", pa.burn_mm());
    assert_eq!(pa.burn_mm(), pb.burn_mm(), "marks add no burn");
    assert_eq!(pa.burn_box(), pb.burn_box());
    // A Cut over the placed sheet burns its outline and the square; the box is the sheet's
    // (page x 20..230), never grown by a mark.
    let (x0, _, x1, _) = pa.burn_box().unwrap();
    assert_eq!((x0, x1), (20_000, 230_000), "{:?}", pa.burn_box());
    let _ = &with.laser;
    let _ = &mut without.placement;
}

#[test]
fn c5_operations_name_the_marks_not_cut() {
    let fx = fixture(true);
    assert_eq!(view(&fx.s)["not_cut"], "Registration marks (not cut)");
}

// ---- criteria 7 to 10 ----------------------------------------------------------------------

#[test]
fn c7_the_group_is_absent_without_marks_and_lists_rows_in_number_order() {
    let none = fixture(false);
    assert!(
        none.s.print_and_cut_view().is_none(),
        "absent, not disabled"
    );
    let fx = fixture(true);
    let v = view(&fx.s);
    let numbers: Vec<i64> = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, [1, 2, 3, 4], "{v:#}");
    assert_eq!(v["state"], "Measure at least 2 marks");
}

#[test]
fn c9_c10_each_edit_is_one_step_and_one_mark_is_not_enough() {
    let mut fx = fixture(true);
    let n = fx.s.undo_len();
    measure_by(&mut fx, the_fit(), &[1], &[]);
    assert_eq!(fx.s.undo_len(), n + 1);
    assert_eq!(view(&fx.s)["state"], "Measure at least 2 marks");
    assert!(fx.s.print_and_cut_take_head_position(2, 250.0, 200.0));
    assert_eq!(fx.s.undo_len(), n + 2, "Take head position is one step");
    assert!(fx.s.print_and_cut_clear(2));
    assert_eq!(fx.s.undo_len(), n + 3, "Clear is one step");
    assert!(view(&fx.s)["rows"][1]["x"].is_null());
    let _ = fx.s.undo();
    assert!(
        !view(&fx.s)["rows"][1]["x"].is_null(),
        "undo brings the row back"
    );
}

// ---- criteria 11 to 16, 18, 19 ----------------------------------------------------------------

#[test]
fn c11_two_marks_say_no_error_check() {
    let mut fx = fixture(true);
    measure_by(&mut fx, the_fit(), &[1, 3], &[]);
    let v = view(&fx.s);
    assert!(v.to_string().contains("2 marks: no error check"), "{v:#}");
}

#[test]
fn c13_c14_three_marks_with_one_off_warn_and_still_apply() {
    let mut fx = fixture(true);
    measure_by(&mut fx, the_fit(), &[1, 2, 3, 4], &[(3, 2.0, 0.0)]);
    let v = view(&fx.s);
    let text = v.to_string();
    assert!(text.contains("check mark 3"), "{v:#}");
    assert!(v["apply"]["reason"].is_null(), "Apply still works: {v:#}");
}

#[test]
fn c16_apply_and_reset_are_one_undo_step_each_and_the_drawing_stays_byte_identical() {
    let mut fx = fixture(true);
    assert!(fx.s.activate_page(&fx.drawing.id));
    let drawing_svg = svg_of_page(&fx.s);
    assert!(fx.s.activate_page(&fx.laser.id));
    let before = doc(&fx.s).placed_page(fx.placement).unwrap().matrix;
    measure_by(&mut fx, the_fit(), &[1, 2, 3, 4], &[]);
    let n = fx.s.undo_len();
    assert_eq!(fx.s.print_and_cut_apply(), "", "applied");
    assert_eq!(fx.s.undo_len(), n + 1, "Apply is one step");
    let placed = doc(&fx.s).placed_page(fx.placement).unwrap().matrix;
    assert!(placed.approx_eq(the_fit(), 1e-3), "{placed:?}");
    let v = view(&fx.s);
    assert_eq!(
        v["apply"]["reason"], "The sheet is on the fit",
        "greyed: {v:#}"
    );
    assert_eq!(v["reset"]["enabled"], true);
    assert!(fx.s.activate_page(&fx.drawing.id));
    assert_eq!(
        svg_of_page(&fx.s),
        drawing_svg,
        "the drawing is not changed"
    );
    assert!(fx.s.activate_page(&fx.laser.id));
    // The job cuts the square where the fit puts it.
    let job = grbl::read(&fx.s.export_job("Untitled.curvyo").unwrap().bytes).unwrap();
    let corner = machine_mm(frame(), the_fit().apply_point(Point::new(60.0, 60.0)));
    let (x0, y0, x1, y1) = job.burn_box().unwrap();
    let um = |v: f64| (v * 1000.0).round() as i64;
    assert!(
        (x0..=x1).contains(&um(corner.x)) && (y0..=y1).contains(&um(corner.y)),
        "{corner:?} in {:?}",
        job.burn_box()
    );
    // Reset is one step and restores exactly.
    assert!(fx.s.print_and_cut_reset());
    assert_eq!(fx.s.undo_len(), n + 2);
    assert_eq!(doc(&fx.s).placed_page(fx.placement).unwrap().matrix, before);
    let _ = fx.s.undo();
    assert!(
        doc(&fx.s)
            .placed_page(fx.placement)
            .unwrap()
            .matrix
            .approx_eq(the_fit(), 1e-3)
    );
    let _ = fx.s.undo();
    assert_eq!(doc(&fx.s).placed_page(fx.placement).unwrap().matrix, before);
    assert!(fx.s.activate_page(&fx.drawing.id));
    assert_eq!(svg_of_page(&fx.s), drawing_svg);
}

#[test]
fn c16_a_fit_off_the_laser_page_is_refused() {
    let mut fx = fixture(true);
    let far = Affine::translation(Vec2::new(3_000.0, 3_000.0));
    measure_by(&mut fx, far, &[1, 2, 3], &[]);
    let before = doc(&fx.s).placed_page(fx.placement).unwrap().matrix;
    let n = fx.s.undo_len();
    let answer = fx.s.print_and_cut_apply();
    assert_eq!(answer, "The fit puts the sheet off the laser page.");
    assert_eq!(fx.s.undo_len(), n);
    assert_eq!(doc(&fx.s).placed_page(fx.placement).unwrap().matrix, before);
}

#[test]
fn c14_start_is_head_position_warns_in_the_group() {
    let mut fx = fixture(true);
    measure_by(&mut fx, the_fit(), &[1, 2, 3], &[]);
    assert!(fx.s.set_start("head_position"));
    let text = view(&fx.s).to_string();
    assert!(
        text.contains("Start is Head position: the cut will not follow the sheet"),
        "{text}"
    );
    assert!(
        view(&fx.s)["apply"]["reason"].is_null(),
        "Apply still works"
    );
}

#[test]
fn c18_a_hand_move_after_apply_offers_apply_again_and_keeps_measures() {
    let mut fx = fixture(true);
    measure_by(&mut fx, the_fit(), &[1, 2, 3], &[]);
    assert_eq!(fx.s.print_and_cut_apply(), "");
    let d = doc(&fx.s);
    d.translate_objects(&[fx.placement], Vec2::new(4.0, 0.0))
        .unwrap();
    let mut s = Session::open(8, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert!(s.activate_page(&fx.laser.id));
    let v = view(&s);
    assert_eq!(v["apply"]["label"], "Apply again", "{v:#}");
    assert!(v.to_string().contains("Placement differs from the fit"));
    assert!(!v["rows"][0]["x"].is_null(), "measures kept");
}

#[test]
fn c19_save_and_reopen_shows_the_same_group() {
    let mut fx = fixture(true);
    measure_by(&mut fx, the_fit(), &[1, 2, 4], &[(4, 0.3, 0.0)]);
    assert!(fx.s.print_and_cut_tolerance(0.4));
    assert_eq!(fx.s.print_and_cut_apply(), "");
    let v = view(&fx.s);
    let mut back = Session::open(9, &fx.s.pack("0.1.0").unwrap()).unwrap();
    assert!(back.activate_page(&fx.laser.id));
    let w = view(&back);
    assert_eq!(w["rows"], v["rows"]);
    assert_eq!(w["fit"], v["fit"]);
    assert_eq!(w["tolerance"], 0.4);
    assert_eq!(w["reset"], v["reset"]);
}
