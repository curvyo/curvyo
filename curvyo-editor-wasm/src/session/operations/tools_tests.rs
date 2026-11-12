#![allow(clippy::assert_is_empty, clippy::many_single_char_names)]

//! Session-level tests of `specs/0111-laser-operation-tools`: the kind press (criteria 3 to 8),
//! the group rule and its tick (9, 10, 14), the read-only entered group (11 to 13), the Operations
//! view toggle (15), and the unfolded row (17); the edits of its body live in `tools_option_tests`.

use curvyo_document_core::{Direction, NodeId, OperationSnapshot, Point, RectBounds};

use super::super::{Session, Tool};
use super::view_json::{RowView, TickView};
use crate::KeyInput;

/// A laser page made active with squares A, B and C on it (A and B grouped when `group`), the
/// Select tool and an empty history.
pub(super) struct Fx {
    pub(super) s: Session,
    pub(super) a: NodeId,
    pub(super) b: NodeId,
    pub(super) c: NodeId,
    pub(super) group: Option<NodeId>,
    pub(super) origin: Point,
}

pub(super) fn fixture(group: bool) -> Fx {
    let mut s = Session::new(300_111);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .origin;
    let square = |x: f64| {
        RectBounds::from_corners(
            Point::new(origin.x + x, origin.y + 10.0),
            Point::new(origin.x + x + 10.0, origin.y + 20.0),
        )
    };
    let (a, b, c, g) = s.recording_unguarded(|this| {
        let a = this.document.create_rect(square(10.0));
        let b = this.document.create_rect(square(30.0));
        let c = this.document.create_rect(square(60.0));
        let g = group.then(|| this.document.group_objects(&[a, b]).unwrap());
        (a, b, c, g)
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[]);
    s.wipe_history();
    Fx {
        s,
        a,
        b,
        c,
        group: g,
        origin,
    }
}

impl Fx {
    /// The top edge midpoint of the square at `x`: a press there hits it.
    fn edge(&self, x: f64) -> Point {
        Point::new(self.origin.x + x + 5.0, self.origin.y + 10.0)
    }

    fn click(&mut self, at: Point) {
        self.s.pointer_hover(at, false, false);
        self.s.pointer_down(at, false);
        self.s.pointer_up(at, false, false);
    }

    fn double_click(&mut self, at: Point) {
        self.click(at);
        self.s.double_click(at, false, false);
    }

    fn key(&mut self, k: &str) {
        self.s.key_down(KeyInput {
            key: k,
            ..KeyInput::default()
        });
    }
}

pub(super) fn ops(s: &Session) -> Vec<OperationSnapshot> {
    let manufacturing = s.active_manufacturing().unwrap();
    s.document.operations(manufacturing.id).unwrap()
}

pub(super) fn op_named(s: &Session, name: &str) -> OperationSnapshot {
    ops(s).into_iter().find(|op| op.name == name).unwrap()
}

pub(super) fn row(s: &Session, name: &str) -> RowView {
    s.operations_view()
        .unwrap()
        .rows
        .into_iter()
        .find(|r| r.name == name)
        .unwrap()
}

/// Member keys in a fixed order: the stored order is the key order.
pub(super) fn sorted(mut keys: Vec<Vec<NodeId>>) -> Vec<String> {
    keys.sort_by_key(|key| format!("{key:?}"));
    keys.iter().map(|key| format!("{key:?}")).collect()
}

pub(super) fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// Makes an operation of `kind` named `name` with no member, as a test grid or an old file
/// leaves one (no command of the tab does since criterion 8).
pub(super) fn empty_op(s: &mut Session, kind: &'static str, name: &'static str) -> NodeId {
    let manufacturing = s.active_manufacturing().unwrap().id;
    let index = ops(s).len();
    s.recording_unguarded(|this| {
        this.document
            .add_operation(
                manufacturing,
                &curvyo_document_core::NewOperation { kind, name },
                index,
                &[],
            )
            .unwrap()
    })
}

// ---- criteria 3 to 8: the kind press -------------------------------------------------------

#[test]
fn criterion_4_a_press_with_no_operation_of_the_kind_creates_one_and_assigns_in_one_step() {
    let mut f = fixture(false);
    f.s.selection.set(&[f.a]);
    let id = f.s.press_operation_kind("engrave", false).unwrap();
    let op = op_named(&f.s, "Engrave 1");
    assert_eq!(op.id, id);
    assert_eq!(op.members, vec![vec![f.a]]);
    assert!(op.show && op.output);
    assert_eq!(f.s.undo_len(), 1);
    assert_eq!(undo_label(&mut f.s), "Assign to Engrave 1");
    assert_eq!(f.s.take_operations_status(), "Engrave 1 added.");
    assert_eq!(f.s.take_operations_status(), "", "the status is read once");
}

#[test]
fn criterion_3_a_second_press_reuses_the_last_operation_of_the_kind() {
    let mut f = fixture(false);
    f.s.selection.set(&[f.a]);
    f.s.press_operation_kind("engrave", false).unwrap();
    f.s.selection.set(&[f.b]);
    f.s.press_operation_kind("engrave", false).unwrap();
    assert_eq!(ops(&f.s).len(), 1, "still one row");
    assert_eq!(
        sorted(op_named(&f.s, "Engrave 1").members),
        sorted(vec![vec![f.a], vec![f.b]])
    );
    assert_eq!(f.s.take_operations_status(), "Assigned to Engrave 1.");
    assert_eq!(f.s.undo_len(), 2);
}

#[test]
fn criterion_5_shift_always_creates_and_criterion_3_reuses_the_lowest() {
    let mut f = fixture(false);
    f.s.selection.set(&[f.b]);
    f.s.press_operation_kind("engrave", false).unwrap();
    f.s.press_operation_kind("engrave", true).unwrap();
    assert_eq!(ops(&f.s).len(), 2);
    assert_eq!(op_named(&f.s, "Engrave 2").members, vec![vec![f.b]]);
    f.s.selection.set(&[f.a]);
    f.s.press_operation_kind("engrave", false).unwrap();
    assert!(op_named(&f.s, "Engrave 2").members.contains(&vec![f.a]));
    assert!(!op_named(&f.s, "Engrave 1").members.contains(&vec![f.a]));
}

#[test]
fn criterion_7_a_press_that_changes_nothing_writes_no_step() {
    let mut f = fixture(false);
    f.s.selection.set(&[f.a]);
    f.s.press_operation_kind("cut", false).unwrap();
    let steps = f.s.undo_len();
    assert_eq!(f.s.press_operation_kind("cut", false), None);
    assert_eq!(f.s.undo_len(), steps);
    assert_eq!(f.s.take_operations_status(), "");
}

#[test]
fn criterion_7_only_guides_create_nothing_and_name_themselves() {
    let mut f = fixture(false);
    let a = f.a;
    f.s.recording_unguarded(|this| this.document.set_guide(&[a], true).unwrap());
    let steps = f.s.undo_len();
    f.s.selection.set(&[a]);
    assert_eq!(f.s.press_operation_kind("cut", false), None);
    assert!(ops(&f.s).is_empty());
    assert_eq!(f.s.undo_len(), steps);
    assert_eq!(
        f.s.operations_notice().as_deref(),
        Some("Guides never run.")
    );
    f.s.selection.set(&[a, f.b]);
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(op_named(&f.s, "Cut 1").members, vec![vec![f.b]]);
    assert_eq!(
        f.s.operations_notice().as_deref(),
        Some("1 guide not assigned. Guides never run.")
    );
}

#[test]
fn criterion_8_nothing_creates_an_empty_operation() {
    let mut f = fixture(false);
    assert_eq!(f.s.press_operation_kind("cut", false), None);
    assert_eq!(f.s.press_operation_kind("cut", true), None);
    assert!(ops(&f.s).is_empty());
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn criterion_1_and_2_the_bar_kinds_show_for_a_selection_with_what_a_press_does() {
    let mut f = fixture(false);
    assert_eq!(f.s.operations_view().unwrap().select_bar_kinds, None);
    f.s.selection.set(&[f.a]);
    let kinds = f.s.operations_view().unwrap().select_bar_kinds.unwrap();
    assert_eq!(kinds.len(), 5);
    assert_eq!(kinds[0].description, "Creates Cut 1");
    f.s.press_operation_kind("engrave", false).unwrap();
    let kinds = f.s.operations_view().unwrap().select_bar_kinds.unwrap();
    assert_eq!(kinds[2].description, "Assigns to Engrave 1");
}

// ---- criteria 9, 10: groups ----------------------------------------------------------------

#[test]
fn criterion_9_a_group_press_reaches_every_leaf_and_keeps_other_memberships() {
    let mut f = fixture(true);
    let g = f.group.unwrap();
    let engrave = empty_op(&mut f.s, "engrave", "Engrave 1");
    f.s.selection.set(&[f.b]);
    // B selected inside the group's unit: select it on its own by key.
    let b = f.b;
    f.s.recording_unguarded(|this| {
        this.document
            .set_operation_members(engrave, &[vec![b]], true)
            .unwrap()
    });
    f.click(f.edge(10.0));
    assert_eq!(f.s.selection_targets().targets, vec![g]);
    let steps = f.s.undo_len();
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(f.s.undo_len(), steps + 1);
    let cut = op_named(&f.s, "Cut 1");
    assert_eq!(sorted(cut.members.clone()), sorted(vec![vec![g], vec![b]]));
    assert_eq!(op_named(&f.s, "Engrave 1").members, vec![vec![b]]);
    // Criterion 10: Cut checked, Engrave mixed with the group named in line 2.
    assert_eq!(row(&f.s, "Cut 1").tick, TickView::All);
    let mixed = row(&f.s, "Engrave 1");
    assert_eq!(mixed.tick, TickView::Some);
    assert_eq!(mixed.group_in_part, Some(format!("Group {}", g.short_id())));
    assert_eq!(
        row(&f.s, "Cut 1").items,
        1,
        "the group is one item (criterion 10)"
    );
    // The mixed tick adds the rest; the untick removes the group and its members' keys.
    assert!(f.s.set_selection_in_operation(engrave, true));
    assert_eq!(row(&f.s, "Engrave 1").tick, TickView::All);
    assert!(f.s.set_selection_in_operation(cut.id, false));
    assert_eq!(undo_label(&mut f.s), "Remove from Cut 1");
    assert!(op_named(&f.s, "Cut 1").members.is_empty());
    assert_eq!(op_named(&f.s, "Engrave 1").members, vec![vec![b], vec![g]]);
}

#[test]
fn criterion_10_two_groups_in_part_are_counted() {
    let mut f = fixture(true);
    let g = f.group.unwrap();
    let c = f.c;
    let (h, d) = f.s.recording_unguarded(|this| {
        let d = this.document.create_rect(RectBounds::from_corners(
            Point::new(f.origin.x + 80.0, f.origin.y + 10.0),
            Point::new(f.origin.x + 90.0, f.origin.y + 20.0),
        ));
        (this.document.group_objects(&[c, d]).unwrap(), d)
    });
    let a = f.a;
    let cut = empty_op(&mut f.s, "cut", "Cut 1");
    f.s.recording_unguarded(|this| {
        this.document
            .set_operation_members(cut, &[vec![a], vec![d]], true)
            .unwrap()
    });
    f.s.selection.set(&[f.a, f.b, c, d]);
    assert_eq!(f.s.selection_targets().targets.len(), 2, "{g:?} {h:?}");
    let cut_row = row(&f.s, "Cut 1");
    assert_eq!(cut_row.tick, TickView::Some);
    assert_eq!(cut_row.group_in_part.as_deref(), Some("2 groups"));
}

#[test]
fn criterion_8_the_number_key_adds_a_mixed_group_and_removes_a_checked_one() {
    let mut f = fixture(true);
    let g = f.group.unwrap();
    empty_op(&mut f.s, "cut", "Cut 1");
    f.click(f.edge(10.0));
    assert!(f.s.toggle_selection_by_number(1));
    assert_eq!(op_named(&f.s, "Cut 1").members, vec![vec![g]]);
    assert!(f.s.toggle_selection_by_number(1));
    assert!(op_named(&f.s, "Cut 1").members.is_empty());
}

// ---- criteria 11 to 13: the read-only entered group ---------------------------------------

#[test]
fn criterion_11_a_double_click_enters_the_group_read_only_and_nothing_moves() {
    let mut f = fixture(true);
    let g = f.group.unwrap();
    f.double_click(f.edge(10.0));
    assert_eq!(f.s.context_depth(), 1);
    assert!(f.s.in_read_only_group());
    let crumb = f.s.operations_view().unwrap().entered_group.unwrap();
    assert_eq!(crumb.groups, vec![format!("Group {}", g.short_id())]);
    assert_eq!(crumb.page, f.s.active_page().name);
    // A press selects the member; a drag moves nothing; arrows, Delete and S do nothing.
    let before = f.s.document.object(f.a);
    let at = f.edge(10.0);
    let to = Point::new(at.x + 40.0, at.y + 40.0);
    f.s.pointer_hover(at, false, false);
    f.s.pointer_down(at, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(f.s.selection.ids(), &[f.a]);
    f.key("ArrowRight");
    f.key("Delete");
    f.key("s");
    assert_eq!(f.s.document.object(f.a), before);
    assert_eq!(f.s.undo_len(), 0, "entering and refusing write no step");
    assert!(!f.s.clipboard_allowed(false));
}

#[test]
fn criterion_11_the_entered_group_has_no_handles_and_an_outline_around_its_members() {
    let mut f = fixture(true);
    f.s.pointer_hover(f.edge(10.0), false, false);
    assert!(
        f.s.context_outline_box(&f.s.objects()).is_none(),
        "no outline at the root"
    );
    f.double_click(f.edge(10.0));
    assert!(f.s.in_read_only_group());
    assert_eq!(f.s.selection.ids(), &[f.a]);
    let handles = f.s.select_transform_decoration_input();
    assert!(handles.handles.is_empty() && handles.pivot_marker.is_none());
    let outline = f.s.context_outline_box(&f.s.objects()).unwrap();
    let a = f.s.document.object(f.a).unwrap();
    let b = f.s.document.object(f.b).unwrap();
    let (a, b) = (
        f.s.cached_oriented_bounds(&a).unwrap().document_corners(),
        f.s.cached_oriented_bounds(&b).unwrap().document_corners(),
    );
    let xs = |c: &[Point; 4]| c.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    assert!((outline[0].x - xs(&a).min(xs(&b))).abs() < 1e-6);
    f.key("Escape");
    assert!(f.s.context_outline_box(&f.s.objects()).is_none());
    f.click(f.edge(10.0));
    assert!(
        !f.s.select_transform_decoration_input().handles.is_empty(),
        "the group has its handles again at the root"
    );
}

#[test]
fn criterion_11_everything_outside_the_entered_group_is_drawn_at_35_percent_in_its_look() {
    let mut f = fixture(true);
    f.s.selection.set(&[f.c]);
    f.s.press_operation_kind("engrave", false).unwrap();
    f.s.press_operation_kind("cut", false).unwrap();
    let strength = |f: &mut Fx| {
        let objects = f.s.render_objects();
        let (looked, overlay) = f.s.operation_looked_artwork(&objects, f.s.view()).unwrap();
        let style = looked.iter().find(|o| o.id() == f.c).unwrap().style();
        let hatch = overlay.triangles.iter().map(|v| u32::from(v.color.a)).max();
        (
            style.fill.opacity.get().max(style.stroke.opacity.get()),
            hatch.unwrap_or(0),
        )
    };
    let (full, full_hatch) = strength(&mut f);
    assert!(full > 0.99 && full_hatch > 0, "{full} {full_hatch}");
    f.double_click(f.edge(10.0));
    assert!(f.s.in_read_only_group());
    let (dim, dim_hatch) = strength(&mut f);
    assert!((dim - full * 0.35).abs() < 1e-6, "{dim}");
    assert!(
        f64::from(dim_hatch) <= f64::from(full_hatch) * 0.35 + 1.0,
        "{dim_hatch} vs {full_hatch}: the hatch and chips of outside items are dimmed too"
    );
}

#[test]
fn criterion_11_a_dropped_file_in_the_read_only_group_is_refused_with_a_short_notice() {
    let mut f = fixture(true);
    f.double_click(f.edge(10.0));
    assert!(f.s.in_read_only_group());
    f.s.import_begin(Some((50.0, 50.0)));
    f.s.import_add(
        include_bytes!("../../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png"),
        "photo.png",
    );
    let outcome = f.s.import_finish(false);
    assert_eq!(outcome.imported, 0);
    assert!(outcome.alert);
    assert!(!outcome.notice.is_empty() && outcome.notice.chars().count() <= 70);
    assert_eq!(f.s.undo_len(), 0, "a refused drop writes no step");
}

#[test]
fn criterion_12_a_member_inside_takes_its_own_operation() {
    let mut f = fixture(true);
    f.double_click(f.edge(10.0));
    f.click(f.edge(10.0));
    assert_eq!(f.s.selection.ids(), &[f.a]);
    f.s.press_operation_kind("score", false).unwrap();
    assert_eq!(op_named(&f.s, "Score 1").members, vec![vec![f.a]]);
    assert_eq!(row(&f.s, "Score 1").tick, TickView::All);
}

#[test]
fn criterion_13_escape_leaves_in_one_press_with_the_group_selected() {
    let mut f = fixture(true);
    let g = f.group.unwrap();
    f.double_click(f.edge(10.0));
    f.click(f.edge(10.0));
    f.key("Escape");
    assert_eq!(f.s.context_depth(), 0);
    assert_eq!(
        f.s.selection_targets().targets,
        vec![g],
        "the group is selected"
    );
    assert_eq!(f.s.operations_view().unwrap().entered_group, None);
}

#[test]
fn criterion_13_a_press_outside_leaves_and_selects_what_it_hits() {
    let mut f = fixture(true);
    f.double_click(f.edge(10.0));
    f.click(f.edge(60.0));
    assert_eq!(f.s.context_depth(), 0);
    assert_eq!(f.s.selection.ids(), &[f.c]);
}

#[test]
fn criterion_13_page_activation_leaves_the_group() {
    let mut f = fixture(true);
    f.double_click(f.edge(10.0));
    assert!(f.s.activate_page(curvyo_document_core::FIRST_PAGE_ID));
    assert_eq!(f.s.context_depth(), 0);
}

// ---- criterion 15: the Operations view toggle ----------------------------------------------

#[test]
fn criterion_15_the_view_toggle_is_per_page_session_state_and_no_step() {
    let mut f = fixture(false);
    let page = f.s.active_page().id;
    assert!(f.s.operations_view().unwrap().view_pressed);
    assert!(f.s.toggle_operations_view(&page));
    assert!(!f.s.operations_view().unwrap().view_pressed);
    assert!(!f.s.operations_view_on());
    let pages: serde_json::Value = serde_json::from_str(&f.s.pages_json()).unwrap();
    let label = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == page.as_str())
        .unwrap();
    assert_eq!(label["operationsView"], false);
    assert!(!f.s.toggle_operations_view(curvyo_document_core::FIRST_PAGE_ID));
    assert_eq!(f.s.undo_len(), 0);
}

// ---- criterion 17: the unfolded row --------------------------------------------------------

#[test]
fn criterion_17_one_row_unfolds_at_a_time_and_a_second_press_folds_it() {
    let mut f = fixture(false);
    let cut = empty_op(&mut f.s, "cut", "Cut 1");
    let score = empty_op(&mut f.s, "score", "Score 1");
    let steps = f.s.undo_len();
    assert!(f.s.unfold_operation_row(cut));
    assert!(row(&f.s, "Cut 1").unfolded);
    assert!(f.s.unfold_operation_row(score));
    assert!(!row(&f.s, "Cut 1").unfolded);
    assert!(row(&f.s, "Score 1").unfolded);
    assert!(f.s.unfold_operation_row(score));
    assert!(!row(&f.s, "Score 1").unfolded);
    assert_eq!(f.s.undo_len(), steps, "unfolding is not a step");
}
