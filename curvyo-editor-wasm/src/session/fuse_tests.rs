//! Bridge in the session (`specs/0156-fused-lettering` criteria 1, 2, 4, 11, 14, 18 and 20): the
//! laser-page gate, the one-page refusal, the result group in one step that undo takes back, the
//! Outline's operations, the refusal under an assigned group, run-twice determinism and
//! save-and-reopen.

use curvyo_document_core::{Direction, NodeId, Point, RectBounds, unpack};
use curvyo_ui_core::{FuseMode, FuseSettings};

use curvyo_laser_core::operations::{ChainScope, OperationList};

use super::fuse::{FuseAvailability, FuseOutcome, FuseSessionRefusal};
use super::{Session, Tool};

const LETTERING: FuseSettings = FuseSettings {
    mode: FuseMode::Lettering,
    width_mm: 1.0,
    reach_mm: 5.0,
    join_all: false,
    bridges_per_island: 2.0,
};

/// Two bars 3 mm apart drawn on the laser page; returns them.
fn bars(s: &mut Session) -> (NodeId, NodeId) {
    let o = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .origin;
    let bar = |x: f64| {
        RectBounds::from_corners(
            Point::new(o.x + x, o.y + 10.0),
            Point::new(o.x + x + 2.0, o.y + 20.0),
        )
    };
    s.recording_unguarded(|this| {
        (
            this.document.create_rect(bar(10.0)),
            this.document.create_rect(bar(15.0)),
        )
    })
}

/// A laser page made active and two bars 3 mm apart drawn on it, both selected; returns them.
fn laser_with_bars(seed: u64) -> (Session, NodeId, NodeId) {
    let mut s = Session::new(seed);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let (a, b) = bars(&mut s);
    s.set_tool(Tool::Select);
    s.selection.set(&[a, b]);
    (s, a, b)
}

fn name(s: &Session, id: NodeId) -> Option<String> {
    s.document.node_registers(id).and_then(|r| r.name)
}

/// Criteria 1 and 2: Bridge exists on a laser page only; dimmed without a selection.
#[test]
fn bridge_is_present_on_a_laser_page_only() {
    let mut s = Session::new(310_001);
    assert_eq!(s.fuse_availability(), FuseAvailability::Absent);
    assert_eq!(s.apply_fuse(LETTERING), FuseOutcome::Ignored);
    let (mut s, _, _) = laser_with_bars(310_002);
    assert_eq!(s.fuse_availability(), FuseAvailability::Ready);
    s.selection.set(&[]);
    assert_eq!(s.fuse_availability(), FuseAvailability::NeedsOne);
    s.activate_page(curvyo_document_core::FIRST_PAGE_ID);
    assert_eq!(s.fuse_availability(), FuseAvailability::Absent);
}

/// Criteria 11 and 18: one step "Bridge", the group selected, Outline above Bridges, the names;
/// undo restores the operands.
#[test]
fn bridge_writes_one_group_in_one_step_and_undo_restores() {
    let (mut s, a, b) = laser_with_bars(310_003);
    let steps = s.document.own_step_count();
    assert_eq!(s.apply_fuse(LETTERING), FuseOutcome::Applied { bridges: 1 });
    assert_eq!(s.document.own_step_count(), steps + 1);
    let group = s.selection.ids()[0];
    assert_eq!(name(&s, group).as_deref(), Some("Bridged lettering"));
    let children = s.document.children(Some(group));
    assert_eq!(children.len(), 2);
    assert_eq!(name(&s, children[1]).as_deref(), Some("Outline"));
    assert_eq!(name(&s, children[0]).as_deref(), Some("Bridges"));
    let bridges = s.document.children(Some(children[0]));
    assert_eq!(name(&s, bridges[0]).as_deref(), Some("Bridge 1"));
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Bridge")
    );
    s.undo();
    assert!(s.document.object(a).is_some() && s.document.object(b).is_some());
}

/// Criterion 4: a selection with a leaf off the laser page is refused.
#[test]
fn a_leaf_off_the_laser_page_refuses() {
    let (mut s, a, _) = laser_with_bars(310_004);
    let far = s.recording_unguarded(|this| {
        this.document.create_rect(RectBounds::from_corners(
            Point::new(-5_000.0, -5_000.0),
            Point::new(-4_990.0, -4_990.0),
        ))
    });
    s.selection.set(&[a, far]);
    assert_eq!(
        s.apply_fuse(LETTERING),
        FuseOutcome::Refused(FuseSessionRefusal::OnePage)
    );
}

/// Criterion 14: the Outline carries the bottom operand's operation, the Bridges none of their
/// own; under an assigned layer Bridge runs and the Bridges inherit the layer's operation by the
/// nearest rule (Frage 404 A).
#[test]
fn the_outline_takes_the_operation_and_the_bridges_inherit_the_layer() {
    let (mut s, a, b) = laser_with_bars(310_005);
    s.selection.set(&[a]);
    let cut = s.add_test_operation("cut").unwrap();
    s.selection.set(&[a, b]);
    assert!(matches!(
        s.apply_fuse(LETTERING),
        FuseOutcome::Applied { .. }
    ));
    let group = s.selection.ids()[0];
    let outline = s.document.children(Some(group))[1];
    let manufacturing = s.active_manufacturing().unwrap().id;
    let op = s
        .document
        .operations(manufacturing)
        .unwrap()
        .into_iter()
        .find(|op| op.id == cut)
        .unwrap();
    assert_eq!(op.members, vec![vec![outline]]);

    let mut s = Session::new(310_006);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let layer = s.recording_unguarded(|this| this.document.new_layer(None));
    let manufacturing = s.active_manufacturing().unwrap().id;
    s.recording_unguarded(|this| {
        this.document
            .add_operation(
                manufacturing,
                &curvyo_document_core::NewOperation {
                    kind: "cut",
                    name: "Cut 1",
                    settings: None,
                },
                0,
                &[vec![layer]],
            )
            .unwrap()
    });
    s.document.set_draw_parent(Some(layer));
    let (a, b) = bars(&mut s);
    s.set_tool(Tool::Select);
    s.selection.set(&[a, b]);
    assert_eq!(s.apply_fuse(LETTERING), FuseOutcome::Applied { bridges: 1 });
    let group = s.selection.ids()[0];
    let parts = s.document.children(Some(group))[0];
    let bridge = s.document.children(Some(parts))[0];
    let active = s.active_operations().unwrap();
    let layer_cut = active.operations[0].id;
    assert!(
        active.operations[0]
            .members
            .iter()
            .all(|m| m != &vec![bridge]),
        "the Bridge has no assignment of its own"
    );
    let list = OperationList::new(&active.operations, active.cuts_last);
    let inherited = list
        .nearest(&s.document.key_chain(&[bridge]), ChainScope::Whole)
        .unwrap();
    assert_eq!(inherited.target, vec![layer]);
    assert_eq!(active.operations[inherited.operations[0]].id, layer_cut);
}

/// Criteria 18 and 20: two runs give the same bytes; save and reopen keeps the group.
#[test]
fn bridge_is_deterministic_and_survives_save_and_reopen() {
    let run = || {
        let (mut s, _, _) = laser_with_bars(310_007);
        s.apply_fuse(LETTERING);
        let group = s.selection.ids()[0];
        (s, group)
    };
    let (first, group) = run();
    let (second, _) = run();
    assert_eq!(first.objects(), second.objects());
    assert_eq!(
        first.document.children(Some(group)),
        second.document.children(Some(group))
    );
    let reopened = unpack(5, &first.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.children(Some(group)).len(), 2);
    let outline = reopened.children(Some(group))[1];
    assert_eq!(reopened.object(outline), first.document.object(outline));
}
