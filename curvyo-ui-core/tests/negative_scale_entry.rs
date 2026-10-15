//! `curvyo-ui-core` tests of `specs/0056-mirror-and-negative-scale/` Part C: the size
//! chip takes any finite W and H, a negative one mirrors about the chip's fixed point, a
//! zero one becomes 0.01 mm, linked fields carry the sign; and criterion 28 e, the same
//! signed map from a drag, a typed size and the flip button. Scale is 4 px/mm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, Document, Length, NewAnchor, NodeId, ObjectSnapshot, Point, PrimitiveSnapshot,
    RectBounds, Shape, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, EntryKey, EntryOutcome, FlipAxis, InvalidReason, Modifiers, ObjectSelection,
    SelectTool, TransformHandleTolerances, ZeroRecovery, flip_selection,
};

const SCALE: f64 = 4.0;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

struct Rig {
    document: Document,
    selection: ObjectSelection,
    tool: SelectTool,
}

impl Rig {
    fn new() -> Self {
        Self {
            document: Document::new(1),
            selection: ObjectSelection::new(),
            tool: SelectTool::new(),
        }
    }

    fn rect(&mut self, x: f64, w: f64) -> NodeId {
        self.document.create_rect(RectBounds {
            origin: pt(x, 0.0),
            width: Length::from_mm(w),
            height: Length::from_mm(10.0),
        })
    }

    fn triangle(&mut self) -> NodeId {
        let corner = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), pt(x, y));
        self.document.create_path(
            &[
                corner(1, 0.0, 0.0),
                corner(2, 30.0, 0.0),
                corner(3, 0.0, 10.0),
            ],
            true,
        )
    }

    fn objects(&self) -> Vec<ObjectSnapshot> {
        self.document
            .object_ids()
            .into_iter()
            .filter_map(|id| self.document.object(id))
            .collect()
    }

    fn select(&mut self, ids: &[NodeId]) {
        self.selection.set(ids);
    }

    /// The size chip of a double-click on the handle at `at`, `(shift, ctrl)` at the
    /// second press.
    fn double_click(&mut self, at: Point, modifiers: (bool, bool)) {
        let objects = self.objects();
        // The first press grabs the handle and its release writes nothing.
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            at,
            Tolerance::from_mm(2.0),
            TransformHandleTolerances::at_scale(SCALE),
            Modifiers::NONE,
        );
        self.tool.pointer_up(
            &self.document,
            &objects,
            &mut self.selection,
            at,
            Modifiers::NONE,
            &mut AnchorIdMinter::new(9),
        );
        let _ = self.tool.double_click(
            &objects,
            &self.selection,
            at,
            Tolerance::from_mm(2.0),
            TransformHandleTolerances::at_scale(SCALE),
            modifiers,
        );
    }

    /// The size chip of the key S.
    fn key_s(&mut self) {
        let objects = self.objects();
        self.tool
            .open_entry_for_key(&objects, &self.selection, EntryKey::Size)
            .unwrap();
    }

    fn commit(&self, texts: [&str; 2], last: usize) -> EntryOutcome {
        if let Some(entry) = self.tool.entry() {
            entry.commit(&self.document, texts, last)
        } else {
            self.tool
                .group_entry()
                .unwrap()
                .commit(&self.document, texts, last)
        }
    }

    fn zero_recovery(&self, texts: [&str; 2], last: usize) -> ZeroRecovery {
        if let Some(entry) = self.tool.entry() {
            entry.zero_recovery(texts, last)
        } else {
            self.tool.group_entry().unwrap().zero_recovery(texts, last)
        }
    }

    fn bounds(&self, id: NodeId) -> (f64, f64, f64, f64) {
        let Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { bounds, .. },
            ..
        })) = self.document.object(id)
        else {
            panic!("a rectangle");
        };
        (
            bounds.origin.x,
            bounds.origin.y,
            bounds.origin.x + bounds.width.as_mm(),
            bounds.origin.y + bounds.height.as_mm(),
        )
    }

    fn nodes(&self, id: NodeId) -> Vec<(f64, f64)> {
        let Some(ObjectSnapshot::Path(path)) = self.document.object(id) else {
            panic!("a path");
        };
        path.anchors
            .iter()
            .map(|a| (a.point.x, a.point.y))
            .collect()
    }
}

fn close4(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)]
        .iter()
        .all(|(p, q)| (p - q).abs() < 1e-9)
}

fn close_nodes(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p.0 - q.0).abs() < 1e-9 && (p.1 - q.1).abs() < 1e-9)
}

/// Criterion 14: the E handle's chip (fixed edge x = 0), W typed −30: a mirror about
/// x = 0; the reopened chip shows the positive size again.
#[test]
fn a_negative_width_mirrors_about_the_fixed_edge() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 30.0);
    rig.select(&[id]);
    rig.double_click(pt(30.0, 5.0), (false, false));
    assert_eq!(rig.commit(["-30", "10.0"], 0), EntryOutcome::Committed);
    assert!(
        close4(rig.bounds(id), (-30.0, 0.0, 0.0, 10.0)),
        "{:?}",
        rig.bounds(id)
    );
    rig.double_click(pt(-30.0, 5.0), (false, false));
    assert_eq!(rig.tool.entry().unwrap().fields()[0].prefill, "30.0");
}

/// Criterion 14 with the key S: the centre line is the fixed line; W and H negative is a
/// half turn about the centre; the real minus sign parses.
#[test]
fn the_key_s_mirrors_about_the_centre() {
    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.key_s();
    assert_eq!(
        rig.commit(["\u{2212}30", "10.0"], 0),
        EntryOutcome::Committed
    );
    assert!(close_nodes(
        &rig.nodes(id),
        &[(30.0, 0.0), (0.0, 0.0), (30.0, 10.0)]
    ));

    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.key_s();
    assert_eq!(rig.commit(["-30", "-10"], 1), EntryOutcome::Committed);
    assert!(close_nodes(
        &rig.nodes(id),
        &[(30.0, 10.0), (0.0, 10.0), (30.0, 0.0)]
    ));
}

/// Criterion 15: a typed 0 becomes +0.01 mm and the entry reports it for the notice.
#[test]
fn a_typed_zero_becomes_a_hundredth() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 30.0);
    rig.select(&[id]);
    rig.double_click(pt(30.0, 5.0), (false, false));
    assert_eq!(
        rig.zero_recovery(["0", "10.0"], 0),
        ZeroRecovery {
            width: true,
            height: false
        }
    );
    assert_eq!(rig.commit(["0", "10.0"], 0), EntryOutcome::Committed);
    assert!(
        close4(rig.bounds(id), (0.0, 0.0, 0.01, 10.0)),
        "{:?}",
        rig.bounds(id)
    );
}

/// Criterion 17: the remaining refusals; the same signed value writes nothing, the same
/// magnitude negative is a change; linked fields carry the sign.
#[test]
fn the_remaining_refusals_and_the_linked_sign() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 30.0);
    rig.select(&[id]);
    rig.double_click(pt(30.0, 5.0), (false, false));
    assert_eq!(
        rig.commit(["x", "10.0"], 0),
        EntryOutcome::Invalid {
            field: 0,
            reason: InvalidReason::NotANumber
        }
    );
    assert_eq!(
        rig.commit(["-20000000", "10.0"], 0),
        EntryOutcome::Invalid {
            field: 0,
            reason: InvalidReason::TooLarge
        }
    );
    assert_eq!(rig.commit(["+30", "10.0"], 0), EntryOutcome::Unchanged);

    let mut rig = Rig::new();
    let id = rig.rect(0.0, 30.0);
    rig.select(&[id]);
    rig.double_click(pt(30.0, 10.0), (false, true));
    let entry = rig.tool.entry().unwrap();
    assert!(entry.linked());
    assert_eq!(entry.linked_text(0, "-30").as_deref(), Some("-10.0"));
    assert_eq!(rig.commit(["-30", "-10.0"], 0), EntryOutcome::Committed);
    assert!(
        close4(rig.bounds(id), (-30.0, -10.0, 0.0, 0.0)),
        "{:?}",
        rig.bounds(id)
    );
}

/// Criterion 14 for a multi-selection: the typed size is about the chip's fixed point, a
/// typed 0 becomes 0.01 mm.
#[test]
fn a_group_entry_mirrors_and_recovers() {
    let mut rig = Rig::new();
    let a = rig.rect(0.0, 10.0);
    let b = rig.rect(20.0, 10.0);
    rig.select(&[a, b]);
    rig.key_s();
    assert_eq!(rig.commit(["-30", "10.0"], 0), EntryOutcome::Committed);
    assert!(close4(rig.bounds(a), (20.0, 0.0, 30.0, 10.0)));
    assert!(close4(rig.bounds(b), (0.0, 0.0, 10.0, 10.0)));

    let mut rig = Rig::new();
    let a = rig.rect(0.0, 10.0);
    let b = rig.rect(20.0, 10.0);
    rig.select(&[a, b]);
    rig.key_s();
    assert!(rig.zero_recovery(["0", "10.0"], 0).width);
    assert_eq!(rig.commit(["0", "10.0"], 0), EntryOutcome::Committed);
    let (x0, _, x1, _) = rig.bounds(b);
    assert!((x1 - x0 - 10.0 * 0.01 / 30.0).abs() < 1e-9);
}

/// Criterion 28 e: for an unrotated selection with the centre fixed, a Shift drag of the
/// E handle to the mirror position, the size typed with the key S and the flip button give
/// the same snapshots.
#[test]
fn a_drag_a_typed_size_and_the_flip_button_agree() {
    let build = || {
        let mut rig = Rig::new();
        let a = rig.rect(0.0, 10.0);
        let b = rig.triangle();
        let c = rig.rect(40.0, 5.0);
        rig.select(&[a, b, c]);
        rig
    };
    let mut dragged = build();
    let objects = dragged.objects();
    let shift = Modifiers::new(true, false);
    dragged.tool.pointer_down(
        &objects,
        &mut dragged.selection,
        pt(45.0, 5.0),
        Tolerance::from_mm(2.0),
        TransformHandleTolerances::at_scale(SCALE),
        shift,
    );
    dragged
        .tool
        .pointer_moved(pt(0.0, 5.0), shift, &mut dragged.selection);
    dragged.tool.pointer_up(
        &dragged.document,
        &objects,
        &mut dragged.selection,
        pt(0.0, 5.0),
        shift,
        &mut AnchorIdMinter::new(9),
    );
    let mut typed = build();
    typed.key_s();
    assert_eq!(typed.commit(["-45", "10.0"], 0), EntryOutcome::Committed);
    let flipped = build();
    flip_selection(
        &flipped.document,
        &flipped.objects(),
        &flipped.selection,
        FlipAxis::Horizontal,
    );
    let text = |rig: &Rig| format!("{:?}", rig.objects());
    assert_eq!(text(&dragged), text(&typed));
    let (d, f) = (dragged.objects(), flipped.objects());
    for (x, y) in d.iter().zip(&f) {
        assert_eq!(format!("{x:.6?}"), format!("{y:.6?}"));
    }
}

/// The UX notes' chip note: the mirror a negative typed size means, single and group.
#[test]
fn the_chip_names_the_mirror() {
    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.key_s();
    let entry = rig.tool.entry().unwrap();
    assert_eq!(entry.mirror_line(["30.0", "10.0"], 0), None);
    assert_eq!(
        entry.mirror_line(["-30", "10.0"], 0),
        Some("Mirrored left to right")
    );
    assert_eq!(
        entry.mirror_line(["30.0", "-10"], 1),
        Some("Mirrored top to bottom")
    );
    assert_eq!(entry.mirror_line(["-30", "-10"], 1), Some("Turned 180°"));

    let mut rig = Rig::new();
    let a = rig.rect(0.0, 10.0);
    let b = rig.rect(20.0, 10.0);
    rig.select(&[a, b]);
    rig.key_s();
    let entry = rig.tool.group_entry().unwrap();
    assert_eq!(
        entry.mirror_line(["-30", "10.0"], 0),
        Some("Mirrored left to right")
    );
}
