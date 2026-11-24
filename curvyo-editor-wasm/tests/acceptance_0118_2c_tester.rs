//! Independent acceptance tests for `specs/0118-machine-control` slice 2c through the public
//! session (criteria 29 to 31; `adrs.md` O2, O7; ux-notes W41, W44): Set origin here as one undo
//! step, Start User origin in the exported job, the job's bytes after an undo. Written from the
//! specification before the implementation was read. The job is compared as bytes, so no GRBL
//! reader is needed here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    missing_docs
)]

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewOperation, NewOutputPage, Point, RectBounds, Timestamp,
    Vec2, pack,
};
use curvyo_editor_wasm::Session;

/// A drawing with a rectangle, placed on a 400 x 300 mm laser page at the origin, with one Cut
/// over the placed sheet (as the 0121 fixture does). The sheet is 210 x 297 mm.
fn session() -> Session {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 20.0),
        Point::new(110.0, 70.0),
    ));
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let laser = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let placement = d
        .place_page(
            &laser.id,
            &drawing.id,
            Affine::translation(Vec2::new(0.0, 0.0)),
        )
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[vec![placement]],
        )
        .unwrap();
    let mut s = Session::open(7, &pack(&d, "0118-2c-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(
        s.activate_page(&laser.id),
        "the laser page is the active page"
    );
    s.load_materials(&[], None);
    let _ = s.edit_operation_setting(op, "main", "power", 60.0);
    let _ = s.edit_operation_setting(op, "main", "speed", 600.0);
    s.wipe_history();
    s
}

fn job(s: &mut Session) -> Vec<u8> {
    s.export_job("Untitled.curvyo").expect("export").bytes
}

#[test]
fn c29_set_origin_here_is_one_undo_step_and_undo_restores_the_machine_origin_job() {
    let mut s = session();
    let before_job = job(&mut s);
    let before_steps = s.undo_len();
    assert!(s.set_user_origin(100.0, 2.0), "the point is stored");
    assert_eq!(
        s.undo_len(),
        before_steps + 1,
        "one step for Set origin here"
    );
    assert_ne!(job(&mut s), before_job, "the job moved to the point");
    let _ = s.undo();
    assert_eq!(
        job(&mut s),
        before_job,
        "one undo takes back the point and the User origin pick"
    );
}

#[test]
fn c29_a_stored_point_is_kept_when_another_start_is_picked_and_the_job_follows_the_pick() {
    let mut s = session();
    let machine = job(&mut s);
    assert!(s.set_user_origin(100.0, 2.0));
    let user = job(&mut s);
    assert!(s.set_start("machine_origin"), "another pick");
    assert_eq!(job(&mut s), machine, "Machine origin burns where it did");
    assert!(
        s.set_start("user_origin"),
        "User origin picks the kept point again"
    );
    assert_eq!(job(&mut s), user, "the same point gives the same job");
}

#[test]
fn c30_user_origin_cannot_be_picked_before_a_point_is_stored() {
    let mut s = session();
    assert!(
        !s.set_start("user_origin"),
        "refused without a stored point"
    );
}

#[test]
fn c31_a_user_origin_that_leaves_the_bed_blocks_the_export() {
    let mut s = session();
    // The Cut runs over the placed A4 sheet (210 x 297 mm, from the origin): at x 200 its right
    // edge lands at 410 mm, past the 400 mm bed.
    assert!(s.set_user_origin(200.0, 2.0));
    assert!(
        s.export_job("Untitled.curvyo").is_none(),
        "the work-area check blocks the export"
    );
}

#[test]
fn c31_a_user_origin_inside_the_bed_exports() {
    let mut s = session();
    assert!(s.set_user_origin(100.0, 2.0));
    assert!(s.export_job("Untitled.curvyo").is_some());
}
