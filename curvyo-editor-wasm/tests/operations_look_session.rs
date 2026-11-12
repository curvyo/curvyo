//! `specs/0101-laser-operations` criteria 14 to 16 and 18 at the session, as ux-notes section 13
//! draws them (customer 2026-11-10: no colour per operation; red for cut lines, pale orange for the
//! hatch, the symbols as chips). The tests read the triangles the frame submits, by their colour.

#![allow(clippy::unwrap_used)]

mod legacy_key;
mod operation_shims;
use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, OperationSwitch, Point,
    RectBounds, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_render_core::{
    DrawList, OP_CHIP_GROUND, OP_CUT, OP_ENGRAVE_HATCH, OP_INK, OP_LANE_LINE, RgbaColor,
};
use operation_shims::{OperationsViewMode, set_view};

const BLACK: RgbaColor = RgbaColor::opaque(0, 0, 0);

/// A laser page with one 10 mm square; when `kind` is given the square is the one member of a
/// single operation of that kind. Returns the file, the laser page id and the square's id.
fn page(kind: Option<&str>) -> (Vec<u8>, String, NodeId) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let o = laser.origin;
    let square = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + 10.0, o.y + 10.0),
        Point::new(o.x + 20.0, o.y + 20.0),
    ));
    if let Some(kind) = kind {
        let m = laser.kind.manufacturing().unwrap();
        d.add_operation(m, &NewOperation { kind, name: "Op 1" }, 0, &[vec![square]])
            .unwrap();
    }
    (pack(&d, "look").unwrap(), laser.id, square)
}

fn open(bytes: &[u8], laser: &str, mode: OperationsViewMode) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(laser));
    set_view(&mut s, mode);
    s
}

fn triangles_in(list: &DrawList, color: RgbaColor) -> usize {
    list.triangles.iter().filter(|v| v.color == color).count()
}

#[test]
fn cut_draws_a_red_line_under_view_operations_and_the_items_own_black_is_gone() {
    let (bytes, laser, _) = page(Some("cut"));
    let list = open(&bytes, &laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&list, OP_CUT) > 0);
    assert_eq!(triangles_in(&list, BLACK), 0);
}

#[test]
fn view_drawn_keeps_the_items_own_style_and_draws_no_red_or_chips() {
    let (bytes, laser, _) = page(Some("cut"));
    let list = open(&bytes, &laser, OperationsViewMode::Drawn).draw_list();
    assert!(triangles_in(&list, BLACK) > 0);
    assert_eq!(triangles_in(&list, OP_CUT), 0);
    assert_eq!(
        triangles_in(&list, OP_CHIP_GROUND),
        0,
        "no symbols under View: Drawn"
    );
}

#[test]
fn an_unassigned_item_is_in_the_not_assigned_look_and_has_no_symbol() {
    let (bytes, laser, _) = page(None);
    let list = open(&bytes, &laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&list, OP_LANE_LINE) > 0);
    assert_eq!(triangles_in(&list, BLACK), 0);
    assert_eq!(triangles_in(&list, OP_CUT), 0);
    assert_eq!(triangles_in(&list, OP_CHIP_GROUND), 0);
}

#[test]
fn show_off_hides_the_items_under_both_views() {
    for mode in [OperationsViewMode::Operations, OperationsViewMode::Drawn] {
        let (bytes, laser, _) = page(Some("cut"));
        let mut s = open(&bytes, &laser, mode);
        let op = s.operations_view().unwrap().rows[0].id;
        assert!(s.set_operation_switch(op, OperationSwitch::Show, false));
        let list = s.draw_list();
        assert_eq!(triangles_in(&list, OP_CUT), 0);
        assert_eq!(triangles_in(&list, BLACK), 0);
        assert_eq!(triangles_in(&list, OP_LANE_LINE), 0);
        assert_eq!(triangles_in(&list, OP_CHIP_GROUND), 0);
    }
}

#[test]
fn output_off_draws_the_item_as_not_assigned_without_a_symbol() {
    let (bytes, laser, _) = page(Some("cut"));
    let mut s = open(&bytes, &laser, OperationsViewMode::Operations);
    let op = s.operations_view().unwrap().rows[0].id;
    assert!(s.set_operation_switch(op, OperationSwitch::Output, false));
    let list = s.draw_list();
    assert_eq!(triangles_in(&list, OP_CUT), 0);
    assert!(triangles_in(&list, OP_LANE_LINE) > 0);
}

#[test]
fn engrave_is_hatched_in_the_pale_orange_and_engrave_outline_adds_the_ink_outline() {
    let (fill_bytes, fill_laser, _) = page(Some("engrave"));
    let fill = open(&fill_bytes, &fill_laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&fill, OP_ENGRAVE_HATCH) > 0);
    let (both_bytes, both_laser, _) = page(Some("engrave_outline"));
    let both = open(&both_bytes, &both_laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&both, OP_ENGRAVE_HATCH) > 0);
    assert!(triangles_in(&both, OP_INK) > triangles_in(&fill, OP_INK));
}

#[test]
fn score_is_a_dotted_ink_line_and_cut_out_a_dashed_red_outline() {
    let (score_bytes, score_laser, _) = page(Some("score"));
    let score = open(&score_bytes, &score_laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&score, OP_INK) > 0);
    assert_eq!(
        triangles_in(&score, OP_CUT),
        0,
        "red is for lines to be cut only"
    );
    let (out_bytes, out_laser, _) = page(Some("cut_out"));
    let out = open(&out_bytes, &out_laser, OperationsViewMode::Operations).draw_list();
    assert!(triangles_in(&out, OP_CUT) > 0);
}

#[test]
fn each_object_with_an_operation_gets_its_symbol_chip() {
    let (bytes, laser, _) = page(Some("cut"));
    let list = open(&bytes, &laser, OperationsViewMode::Operations).draw_list();
    assert!(
        triangles_in(&list, OP_CHIP_GROUND) > 0,
        "the chip ground is drawn"
    );
}

#[test]
fn switching_the_view_back_and_forth_changes_the_drawing() {
    let (bytes, laser, _) = page(Some("cut"));
    let mut s = open(&bytes, &laser, OperationsViewMode::Operations);
    assert!(triangles_in(&s.draw_list(), OP_CUT) > 0);
    assert!(set_view(&mut s, OperationsViewMode::Drawn));
    assert_eq!(triangles_in(&s.draw_list(), OP_CUT), 0);
    assert!(set_view(&mut s, OperationsViewMode::Operations));
    assert!(triangles_in(&s.draw_list(), OP_CUT) > 0);
}

#[test]
fn a_placed_members_draws_in_the_look_of_its_placement_operation() {
    let (bytes, laser) = placed_page(Some("cut"));
    let s = open(&bytes, &laser, OperationsViewMode::Operations);
    let list = s.frame_draw_list();
    assert!(triangles_in(&list, OP_CUT) > 0);
}

#[test]
fn an_unassigned_placed_member_draws_in_the_not_assigned_look() {
    let (bytes, laser) = placed_page(None);
    let s = open(&bytes, &laser, OperationsViewMode::Operations);
    let list = s.frame_draw_list();
    assert!(triangles_in(&list, OP_LANE_LINE) > 0);
    assert_eq!(triangles_in(&list, OP_CUT), 0);
}

#[test]
fn show_off_on_the_placement_hides_its_members() {
    let (bytes, laser) = placed_page(Some("cut"));
    let mut s = open(&bytes, &laser, OperationsViewMode::Operations);
    let op = s.operations_view().unwrap().rows[0].id;
    assert!(s.set_operation_switch(op, OperationSwitch::Show, false));
    let list = s.frame_draw_list();
    assert_eq!(triangles_in(&list, OP_CUT), 0);
    assert_eq!(triangles_in(&list, OP_LANE_LINE), 0);
}

/// A laser page with a placed drawing page (one 10 mm square on it); the placement is assigned to
/// `kind` when given. Returns the file and the laser page id.
fn placed_page(kind: Option<&str>) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let source = d.add_page();
    let o = source.origin;
    let _square = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + 2.0, o.y + 2.0),
        Point::new(o.x + 12.0, o.y + 12.0),
    ));
    let placement = d
        .place_page(
            &laser.id,
            &source.id,
            curvyo_document_core::Affine::translation(curvyo_document_core::Vec2::new(10.0, 10.0)),
        )
        .unwrap();
    if let Some(kind) = kind {
        let m = laser.kind.manufacturing().unwrap();
        d.add_operation(
            m,
            &NewOperation { kind, name: "Op 1" },
            0,
            &[vec![placement]],
        )
        .unwrap();
    }
    (pack(&d, "placed").unwrap(), laser.id)
}

/// A laser page with a group of two 10 mm squares and a linked clone of it. The clone's first
/// member is assigned to a Cut operation by its path, as a file of format 39 to 43 holds it (Frage
/// 212 A). Returns the file and the page.
fn clone_of_group(member_kind: Option<&str>) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let o = laser.origin;
    let first = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + 10.0, o.y + 10.0),
        Point::new(o.x + 20.0, o.y + 20.0),
    ));
    let second = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + 30.0, o.y + 10.0),
        Point::new(o.x + 40.0, o.y + 20.0),
    ));
    let group = d.group_objects(&[first, second]).unwrap();
    let clone = d.linked_clone(&[group], &[]).unwrap()[0];
    if let Some(kind) = member_kind {
        let m = laser.kind.manufacturing().unwrap();
        let path = d.clone_member_path(clone, first).unwrap();
        d.add_operation(m, &NewOperation { kind, name: "Op 1" }, 0, &[])
            .unwrap();
        // No command writes a member key any more (`specs/0111` criterion 14): an old file's.
        let d = legacy_key::with_legacy_key(&d, "Op 1", &path);
        return (pack(&d, "clone").unwrap(), laser.id);
    }
    (pack(&d, "clone").unwrap(), laser.id)
}

/// Superseded by `specs/0111` criterion 14 (Frage 212 A, ADR 0056 §2): a clone member's path key
/// is legacy and runs nothing; the members take the clone's operations (here none).
#[test]
fn a_legacy_clone_member_key_draws_nothing_and_the_members_follow_the_clone() {
    let (bytes, laser) = clone_of_group(Some("cut"));
    let s = open(&bytes, &laser, OperationsViewMode::Operations);
    let list = s.draw_list();
    assert_eq!(
        triangles_in(&list, OP_CUT),
        0,
        "the legacy key draws no Cut"
    );
    assert!(
        triangles_in(&list, OP_LANE_LINE) > 0,
        "both members are in the Not assigned look"
    );
}

/// A laser page with one curve of `kind` through four points, cut by one operation
/// (`specs/0110` criterion 2: Spiro and B-spline paths are drawn in every view).
fn curve_page(kind: curvyo_document_core::CurveKind) -> (Vec<u8>, String) {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let o = laser.origin;
    let controls: Vec<_> = [(10.0, 10.0), (30.0, 40.0), (50.0, 10.0), (70.0, 40.0)]
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let mut node = NewAnchor::corner(
                AnchorId::new(1, 100 + i as u64),
                Point::new(o.x + x, o.y + y),
            );
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect();
    let curve = d.create_editable_curve(&controls, false, kind).unwrap();
    let m = laser.kind.manufacturing().unwrap();
    d.add_operation(
        m,
        &NewOperation {
            kind: "cut",
            name: "Op 1",
        },
        0,
        &[vec![curve]],
    )
    .unwrap();
    (pack(&d, "look").unwrap(), laser.id)
}

#[test]
fn spiro_and_b_spline_curves_are_drawn_in_their_laser_look_and_as_drawn() {
    for kind in [
        curvyo_document_core::CurveKind::Spiro,
        curvyo_document_core::CurveKind::BSpline,
    ] {
        let (bytes, laser) = curve_page(kind);
        let looked = open(&bytes, &laser, OperationsViewMode::Operations).draw_list();
        assert!(triangles_in(&looked, OP_CUT) > 0, "{kind:?} in its look");
        let drawn = open(&bytes, &laser, OperationsViewMode::Drawn).draw_list();
        assert!(triangles_in(&drawn, BLACK) > 0, "{kind:?} as drawn");
    }
}
