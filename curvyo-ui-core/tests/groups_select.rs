//! The Select tool and groups (`specs/0023-groups` criteria 8 to 10, 13 and 15): a click, a
//! Shift-click, a marquee, a lasso and the Alt-click cycle choose among the objects of the context,
//! a group counting as one; a double-click on a group asks to enter it; a Ctrl-copy of a group is a
//! group of copies.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use curvyo_document_core::{
    Document, Length, NodeId, ObjectSnapshot, Point, RectBounds, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, Modifiers, ObjectSelection, SelectDoubleClickOutcome, SelectTool,
    TransformHandleTolerances, Units,
};

const SCALE: f64 = 4.0;
const TOLERANCE_MM: f64 = 2.0;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::new(true, false);
const CTRL: Modifiers = Modifiers::new(false, true);
const ALT: Modifiers = Modifiers::NONE.with_alt(true);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

struct Rig {
    document: Document,
    selection: ObjectSelection,
    tool: SelectTool,
    minter: AnchorIdMinter,
    context: Option<NodeId>,
}

impl Rig {
    /// Four 10 mm squares at x = 0, 20, 40, 60 (y 0 to 10); the second and third are one group
    /// when `group` is set.
    fn new(group: bool) -> (Self, [NodeId; 4], Option<NodeId>) {
        let document = Document::new(1);
        let ids = [0.0, 20.0, 40.0, 60.0].map(|x| {
            document.create_rect(RectBounds {
                origin: pt(x, 0.0),
                width: Length::from_mm(10.0),
                height: Length::from_mm(10.0),
            })
        });
        let group = group.then(|| document.group_objects(&[ids[1], ids[2]]).unwrap());
        let mut rig = Self {
            document,
            selection: ObjectSelection::new(),
            tool: SelectTool::new(),
            minter: AnchorIdMinter::new(9),
            context: None,
        };
        rig.sync();
        (rig, ids, group)
    }

    fn sync(&mut self) {
        self.tool
            .set_units(Arc::new(Units::build(&self.document, self.context)));
    }

    fn objects(&self) -> Vec<ObjectSnapshot> {
        self.document
            .object_ids()
            .into_iter()
            .filter_map(|id| self.document.object(id))
            .collect()
    }

    fn press(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            at,
            Tolerance::from_mm(TOLERANCE_MM),
            TransformHandleTolerances::at_scale(SCALE),
            modifiers,
        );
    }

    fn release(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_up(
            &self.document,
            &objects,
            &mut self.selection,
            at,
            modifiers,
            &mut self.minter,
        );
    }

    fn click(&mut self, at: Point, modifiers: Modifiers) {
        self.press(at, modifiers);
        self.release(at, modifiers);
    }

    fn drag(&mut self, from: Point, to: Point, press: Modifiers, release: Modifiers) {
        self.press(from, press);
        self.tool.pointer_moved(
            pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
            release,
            &mut self.selection,
        );
        self.tool.pointer_moved(to, release, &mut self.selection);
        self.release(to, release);
    }

    fn selected(&self) -> Vec<NodeId> {
        let mut ids = self.selection.ids().to_vec();
        ids.sort_by_key(|id| self.document.object_ids().iter().position(|o| o == id));
        ids
    }
}

/// Criterion 8: a click on a leaf inside a group selects the group, not the leaf; a click on a leaf
/// that is a direct child of the context selects the leaf.
#[test]
fn a_click_on_a_member_selects_the_whole_group() {
    let (mut rig, ids, _) = Rig::new(true);
    rig.click(pt(5.0, 0.0), NONE);
    assert_eq!(rig.selected(), vec![ids[0]]);
    rig.click(pt(45.0, 0.0), NONE);
    assert_eq!(
        rig.selected(),
        vec![ids[1], ids[2]],
        "the third square selects its group"
    );
    rig.click(pt(25.0, 0.0), NONE);
    assert_eq!(rig.selected(), vec![ids[1], ids[2]]);
}

#[test]
fn shift_click_toggles_a_group_in_and_out_as_one() {
    let (mut rig, ids, _) = Rig::new(true);
    rig.click(pt(5.0, 0.0), NONE);
    rig.click(pt(25.0, 0.0), SHIFT);
    assert_eq!(rig.selected(), vec![ids[0], ids[1], ids[2]]);
    rig.click(pt(45.0, 0.0), SHIFT);
    assert_eq!(rig.selected(), vec![ids[0]], "the group leaves together");
    rig.click(pt(45.0, 0.0), SHIFT);
    assert_eq!(rig.selected(), vec![ids[0], ids[1], ids[2]]);
}

/// Criterion 8 and 18 of `0004`: dragging a member of a selected group moves the whole selection;
/// dragging a member of an unselected group selects and moves the group.
#[test]
fn dragging_a_member_moves_the_whole_group() {
    let (mut rig, ids, _) = Rig::new(true);
    rig.drag(pt(45.0, 0.0), pt(45.0, 20.0), NONE, NONE);
    for id in [ids[1], ids[2]] {
        let ObjectSnapshot::Primitive(moved) = rig.document.object(id).unwrap() else {
            panic!("a rectangle");
        };
        assert!((moved.rotation.as_radians()).abs() < 1e-12);
    }
    let bounds = |id: NodeId| match rig.document.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => bounds.origin,
            _ => panic!("a rectangle"),
        },
        ObjectSnapshot::Path(_) => panic!("a rectangle"),
    };
    assert_eq!(bounds(ids[1]), pt(20.0, 20.0));
    assert_eq!(bounds(ids[2]), pt(40.0, 20.0));
    assert_eq!(bounds(ids[0]), pt(0.0, 0.0), "the others stay");
}

/// Criterion 9: a marquee chooses among the objects of the context; a group counts as one object
/// whose box is the bounding box of its leaves, for the touch and the contain rule alike.
#[test]
fn a_marquee_sees_a_group_as_one_object_with_the_box_of_all_its_leaves() {
    let (mut rig, ids, _) = Rig::new(true);
    // Contain (rightward): the rectangle holds the first square and only the first of the group's
    // two squares: the group box (20 to 50) is not inside, so only the first square is chosen.
    rig.drag(pt(-2.0, -12.0), pt(32.0, 14.0), NONE, NONE);
    assert_eq!(rig.selected(), vec![ids[0]]);
    // Contain, wide enough for the whole group box: both members are chosen together.
    rig.selection.clear();
    rig.drag(pt(-2.0, -12.0), pt(52.0, 14.0), NONE, NONE);
    assert_eq!(rig.selected(), vec![ids[0], ids[1], ids[2]]);
    // Touch (leftward): reaching one member chooses the group, the square beyond is not touched.
    rig.selection.clear();
    rig.drag(pt(32.0, 14.0), pt(22.0, -2.0), NONE, NONE);
    assert_eq!(rig.selected(), vec![ids[1], ids[2]]);
}

#[test]
fn a_lasso_that_touches_one_member_chooses_the_group() {
    let (mut rig, ids, _) = Rig::new(true);
    rig.press(pt(41.0, 5.0), ALT);
    rig.tool
        .pointer_moved(pt(44.0, 5.0), ALT, &mut rig.selection);
    rig.tool
        .pointer_moved(pt(48.0, 5.0), ALT, &mut rig.selection);
    rig.release(pt(48.0, 5.0), ALT);
    assert_eq!(rig.selected(), vec![ids[1], ids[2]]);
}

/// Criterion 10: the Alt-click candidates are the objects of the context under the point; a group
/// is one candidate, so a click cycle over a group and a square below it has two steps.
#[test]
fn the_alt_click_cycle_steps_over_objects_of_the_context_not_over_leaves() {
    let document = Document::new(1);
    let mk = |x: f64, w: f64| {
        document.create_rect(RectBounds {
            origin: pt(x, 0.0),
            width: Length::from_mm(w),
            height: Length::from_mm(10.0),
        })
    };
    // A wide square at the bottom, then a group of two squares on top of its middle.
    let wide = mk(0.0, 60.0);
    let m1 = mk(10.0, 10.0);
    let m2 = mk(12.0, 10.0);
    let group = document.group_objects(&[m1, m2]).unwrap();
    let _ = group;
    let mut rig = Rig {
        document,
        selection: ObjectSelection::new(),
        tool: SelectTool::new(),
        minter: AnchorIdMinter::new(9),
        context: None,
    };
    rig.sync();
    rig.click(pt(15.0, 0.0), NONE);
    assert_eq!(
        rig.selected(),
        vec![m1, m2],
        "the plain click takes the group on top"
    );
    rig.click(pt(15.0, 0.0), ALT);
    assert_eq!(
        rig.selected(),
        vec![wide],
        "the next candidate is the square below"
    );
    rig.click(pt(15.0, 0.0), ALT);
    assert_eq!(
        rig.selected(),
        vec![m1, m2],
        "and the cycle wraps to the group"
    );
}

/// Criterion 15: a double-click on a member of a group asks the host to enter the group, naming the
/// leaf that was hit; on a leaf that is a direct child of the context nothing of that kind happens.
#[test]
fn a_double_click_on_a_group_asks_to_enter_it() {
    let (mut rig, ids, group) = Rig::new(true);
    let objects = rig.objects();
    let outcome = rig.tool.double_click(
        &objects,
        &rig.selection,
        pt(45.0, 0.0),
        Tolerance::from_mm(TOLERANCE_MM),
        TransformHandleTolerances::at_scale(SCALE),
        (false, false),
    );
    assert_eq!(
        outcome,
        SelectDoubleClickOutcome::EnterGroup {
            group: group.unwrap(),
            leaf: ids[2]
        }
    );
    let outcome = rig.tool.double_click(
        &objects,
        &rig.selection,
        pt(5.0, 0.0),
        Tolerance::from_mm(TOLERANCE_MM),
        TransformHandleTolerances::at_scale(SCALE),
        (false, false),
    );
    assert_eq!(
        outcome,
        SelectDoubleClickOutcome::EditHint,
        "a rectangle outside any group"
    );
}

/// Inside the group the same click selects the leaf: the group is the context.
#[test]
fn inside_the_entered_group_a_click_selects_a_leaf() {
    let (mut rig, ids, group) = Rig::new(true);
    rig.context = group;
    rig.sync();
    // The host passes only the objects of the context.
    let inside: Vec<ObjectSnapshot> = [ids[1], ids[2]]
        .iter()
        .filter_map(|&id| rig.document.object(id))
        .collect();
    rig.tool.pointer_down(
        &inside,
        &mut rig.selection,
        pt(45.0, 0.0),
        Tolerance::from_mm(TOLERANCE_MM),
        TransformHandleTolerances::at_scale(SCALE),
        NONE,
    );
    assert_eq!(rig.selection.ids(), &[ids[2]]);
}

/// Criterion 13: a Ctrl-copy of a group is a group of copies with new ids, above the original, and
/// the copies' leaves are selected.
#[test]
fn a_ctrl_copy_of_a_group_copies_the_group() {
    let (mut rig, ids, group) = Rig::new(true);
    rig.click(pt(45.0, 0.0), NONE);
    rig.drag(pt(40.0, 5.0), pt(40.0, 35.0), NONE, CTRL);
    let roots = rig.document.children(None);
    assert_eq!(roots.len(), 4, "three objects and the copy: {roots:?}");
    assert_eq!(roots[3], ids[3]);
    assert_eq!(roots[0], ids[0]);
    assert_eq!(roots[1], group.unwrap());
    let copy = roots[2];
    assert!(rig.document.is_group(copy), "the copy is a group");
    assert_ne!(copy, group.unwrap());
    let copied = rig.document.leaves_of(copy);
    assert_eq!(copied.len(), 2);
    assert!(copied.iter().all(|c| !ids.contains(c)), "new ids");
    assert_eq!(rig.selected().len(), 2);
    assert!(
        copied.iter().all(|c| rig.selection.contains(*c)),
        "the copy is selected"
    );
}
