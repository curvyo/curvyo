//! The typed move entry of one object or of a multi-selection
//! (`specs/0010-edit-interaction-polish/specification.md`, criteria 15 to 22, 25,
//! 56; `specs/0019-multi-object-transform/` criterion 35; `specs/0047-transform-polish/`
//! criteria 37 to 50): two fields X and Y, read as a relative offset or as the absolute
//! position of a chosen [`ReferencePoint`] of the drawn bounds (of the group box for a
//! selection), the centre until another is chosen. The mode is the chip's (the DOM holds it
//! and passes it with every commit), the chosen point is view state of this entry, so the rule
//! exists once, here. The DOM chip only holds the text, the caret and the focus
//! (`adrs.md`, decision 3).

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, Vec2};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::group_box::GroupSelection;
use crate::object_bounds::object_outline_bounds;
use crate::oriented_box::OrientedBox;
use crate::reference_point::ReferencePoint;
use crate::transform_commit::{
    MAX_COORDINATE_MM, MOVE_EQUAL_EPSILON_MM, axis_within_limit, commit_move,
};
use crate::transform_entry::{
    EntryField, EntryOutcome, InvalidReason, format_mm, parse_entry_number,
};

/// An open move entry: the object (or objects) and the drawn bounds as they
/// were when it opened (an entry whose object changed or went away since
/// writes nothing).
#[derive(Debug, Clone)]
pub struct MoveEntry {
    start: Vec<ObjectSnapshot>,
    ids: Vec<NodeId>,
    start_box: OrientedBox,
    /// The tight bounds of the drawn outline: corners `(min, max)`.
    bounds: (Point, Point),
    /// X and Y as the chip shows them in Relative mode: prefill "0".
    fields: [EntryField; 2],
    /// Whether the chip shows its Absolute mode right now. View state the chip reports, so that
    /// the marks are drawn and clickable in Absolute only (criteria 39 and 40).
    absolute: bool,
    /// The point of the bounds that Absolute measures from: the centre on every open
    /// (criteria 37 and 49), never stored.
    reference: ReferencePoint,
    /// The point whose radio has the pointer or the keyboard focus: its mark is drawn in its
    /// hover state (criterion 41).
    radio_hover: Option<ReferencePoint>,
    /// Whether the chip opens with its Copy check on: Ctrl was held at the
    /// second press of the double-click (criterion 23).
    copy_preset: bool,
}

impl MoveEntry {
    /// An entry for `object` with oriented box `box_`, opening in Relative
    /// mode.
    #[must_use]
    pub fn new(object: &ObjectSnapshot, box_: &OrientedBox) -> Self {
        Self::over(vec![object.clone()], *box_, object_outline_bounds(object))
    }

    /// An entry for the selected `objects` (in selection order) with their
    /// `group` box: relative offsets move all of them, an absolute position
    /// puts the chosen point of the group box there, and a copy copies all
    /// of them (`specs/0019-multi-object-transform/` criterion 35).
    #[must_use]
    pub fn for_group(objects: &[ObjectSnapshot], group: &GroupSelection) -> Self {
        let box_ = *group.bounds();
        Self::over(objects.to_vec(), box_, (box_.min, box_.max))
    }

    fn over(start: Vec<ObjectSnapshot>, start_box: OrientedBox, bounds: (Point, Point)) -> Self {
        let field = |label, name| EntryField::for_param(label, name, "0".to_string());
        Self {
            ids: start.iter().map(ObjectSnapshot::id).collect(),
            start,
            start_box,
            bounds,
            fields: [
                field("X", "Horizontal offset"),
                field("Y", "Vertical offset"),
            ],
            absolute: false,
            reference: ReferencePoint::Centre,
            radio_hover: None,
            copy_preset: false,
        }
    }

    /// The same entry opening with its Copy check on or off: on when Ctrl was
    /// held at the second press of the double-click (criterion 23); the key M
    /// cannot carry a Ctrl and opens it off.
    #[must_use]
    pub const fn with_copy_preset(mut self, copy: bool) -> Self {
        self.copy_preset = copy;
        self
    }

    /// Whether the Copy check opens on.
    #[must_use]
    pub const fn copy_preset(&self) -> bool {
        self.copy_preset
    }

    /// The ids of the objects being moved, in selection order: one for a
    /// single object.
    #[must_use]
    pub fn ids(&self) -> &[NodeId] {
        &self.ids
    }

    /// The oriented box the object had when the entry opened: its centre is
    /// where the chip is anchored.
    #[must_use]
    pub const fn start_box(&self) -> &OrientedBox {
        &self.start_box
    }

    /// The chip's two fields, X then Y, as they read in Relative mode.
    #[must_use]
    pub fn fields(&self) -> &[EntryField] {
        &self.fields
    }

    /// The tight bounds of the drawn outline, corners `(min, max)`: the box the reference
    /// points lie on and the pale red box shows while the chip is open (criterion 34).
    #[must_use]
    pub const fn bounds(&self) -> (Point, Point) {
        self.bounds
    }

    /// What an untouched X and Y field show in Absolute mode for the chosen reference point:
    /// its current position, one decimal (criterion 37).
    #[must_use]
    pub fn absolute_prefill(&self) -> [String; 2] {
        self.absolute_prefill_for(self.reference)
    }

    /// The same for `reference`: also what the fields show while the pointer is on its
    /// mark (criterion 44).
    #[must_use]
    pub fn absolute_prefill_for(&self, reference: ReferencePoint) -> [String; 2] {
        let at = reference.point(self.bounds);
        [format_mm(at.x), format_mm(at.y)]
    }

    /// The chosen reference point of Absolute: the centre until another is chosen.
    #[must_use]
    pub const fn reference(&self) -> ReferencePoint {
        self.reference
    }

    /// Chooses the reference point. Nothing is applied (criterion 41).
    pub const fn set_reference(&mut self, reference: ReferencePoint) {
        self.reference = reference;
    }

    /// Whether the chip shows Absolute right now.
    #[must_use]
    pub const fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// The chip reports its mode: the marks exist in Absolute only (criterion 39).
    pub const fn set_absolute(&mut self, absolute: bool) {
        self.absolute = absolute;
    }

    /// The point whose radio has the pointer or the focus, if any.
    #[must_use]
    pub const fn radio_hover(&self) -> Option<ReferencePoint> {
        self.radio_hover
    }

    /// The chip reports the radio under the pointer or with the focus.
    pub const fn set_radio_hover(&mut self, hover: Option<ReferencePoint>) {
        self.radio_hover = hover;
    }

    /// The offset the typed `texts` mean in the given mode, `Ok(None)` when it
    /// is no move (an unedited chip, relative 0 and 0, an absolute position
    /// equal to the current one), or the first refused field. A field whose
    /// text equals the prefill of the mode in force is untouched and means "no
    /// change on that axis". Absolute reads the chosen reference point.
    ///
    /// # Errors
    /// The index of the first offending field with
    /// [`InvalidReason::NotANumber`]: text that is not a number, or a value or
    /// a result beyond the document's coordinate limit on any of the four edges.
    pub fn resolve(
        &self,
        texts: [&str; 2],
        absolute: bool,
    ) -> Result<Option<Vec2>, (usize, InvalidReason)> {
        self.resolve_at(texts, absolute, self.reference)
    }

    /// [`MoveEntry::resolve`] with `reference` instead of the chosen point: the click on a
    /// mark moves that point (criterion 40), and "untouched" is decided against its prefill
    /// (criterion 44).
    ///
    /// # Errors
    /// As [`MoveEntry::resolve`].
    pub fn resolve_at(
        &self,
        texts: [&str; 2],
        absolute: bool,
        reference: ReferencePoint,
    ) -> Result<Option<Vec2>, (usize, InvalidReason)> {
        let (min, max) = self.bounds;
        let (origin, far) = ([min.x, min.y], [max.x, max.y]);
        let at = reference.point(self.bounds);
        let current = [at.x, at.y];
        let absolute_prefill = self.absolute_prefill_for(reference);
        let mut offset = [0.0_f64; 2];
        for index in 0..2 {
            let prefill = if absolute {
                absolute_prefill[index].as_str()
            } else {
                self.fields[index].prefill.as_str()
            };
            if texts[index] == prefill {
                continue;
            }
            let value = parse_entry_number(texts[index], false)
                .filter(|value| value.abs() <= MAX_COORDINATE_MM)
                .ok_or((index, InvalidReason::NotANumber))?;
            offset[index] = if absolute {
                value - current[index]
            } else {
                value
            };
            if !axis_within_limit(origin[index], far[index], offset[index]) {
                return Err((index, InvalidReason::NotANumber));
            }
        }
        let offset = Vec2::new(offset[0], offset[1]);
        Ok((offset.length() > MOVE_EQUAL_EPSILON_MM).then_some(offset))
    }

    /// Validates the typed values and, if valid and not a no-op, moves the
    /// object by them as one commit, the one a drag of the same offset
    /// writes; with `copy`, makes one copy displaced as typed and leaves the
    /// object untouched. An entry whose object changed or vanished since it
    /// opened writes nothing. The ids of the copy come back with a committed
    /// copy: the caller selects them (criterion 35).
    #[must_use]
    pub fn commit(
        &self,
        document: &Document,
        texts: [&str; 2],
        absolute: bool,
        copy: bool,
        minter: &mut AnchorIdMinter,
    ) -> (EntryOutcome, Option<Vec<NodeId>>) {
        self.commit_at(document, texts, (absolute, self.reference), copy, minter)
    }

    /// [`MoveEntry::commit`] with `reference` instead of the chosen point: the click on a
    /// mark (criterion 40). `mode` is the chip's mode and the point to move.
    #[must_use]
    pub fn commit_at(
        &self,
        document: &Document,
        texts: [&str; 2],
        (absolute, reference): (bool, ReferencePoint),
        copy: bool,
        minter: &mut AnchorIdMinter,
    ) -> (EntryOutcome, Option<Vec<NodeId>>) {
        let unchanged = self
            .start
            .iter()
            .all(|object| document.object(object.id()).as_ref() == Some(object));
        if !unchanged {
            return (EntryOutcome::Unchanged, None);
        }
        match self.resolve_at(texts, absolute, reference) {
            Err((field, reason)) => (EntryOutcome::Invalid { field, reason }, None),
            Ok(None) => (EntryOutcome::Unchanged, None),
            Ok(Some(offset)) => {
                let copies = commit_move(document, &self.ids, offset, copy, minter);
                (EntryOutcome::Committed, copies)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{
        AnchorId, Angle, InnerRatio, Length, NewAnchor, NodeId, PointCount, RectBounds, StarFrame,
    };

    use super::*;

    fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        })
    }

    /// An entry over one object, measuring Absolute from the **top-left**: the reference
    /// the examples of `0010` criteria 18 to 25 were written for (`0047` criterion 37: their
    /// numbers hold for the top-left reference). [`fresh_entry_of`] has the default centre.
    fn entry_of(document: &Document, id: NodeId) -> MoveEntry {
        let mut entry = fresh_entry_of(document, id);
        entry.set_reference(ReferencePoint::TopLeft);
        entry
    }

    fn fresh_entry_of(document: &Document, id: NodeId) -> MoveEntry {
        let object = document.object(id).expect("exists");
        MoveEntry::new(&object, &crate::oriented_box::oriented_bounds(&object))
    }

    fn top_left(document: &Document, id: NodeId) -> Point {
        object_outline_bounds(&document.object(id).expect("exists")).0
    }

    fn near(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
    }

    /// Criterion 18, as `0047` criterion 37 changes it: both fields open at "0" in Relative
    /// mode and show the current centre in Absolute mode (the top-left with that reference).
    #[test]
    fn the_fields_open_at_zero_and_the_absolute_prefill_is_the_centre() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let fresh = fresh_entry_of(&document, id);
        assert_eq!(fresh.reference(), ReferencePoint::Centre);
        assert_eq!(fresh.absolute_prefill(), ["25.0", "40.0"]);
        assert!(!fresh.is_absolute(), "the chip opens in Relative");
        let entry = entry_of(&document, id);
        assert_eq!(entry.fields()[0].prefill, "0");
        assert_eq!(entry.fields()[1].prefill, "0");
        assert_eq!(entry.fields()[0].label, "X");
        assert_eq!(entry.fields()[1].label, "Y");
        assert_eq!(entry.absolute_prefill(), ["10.0", "20.0"]);
    }

    /// Criterion 20: relative (5, -3) moves the top-left from (10, 20) to
    /// (15, 17); X right, Y down.
    #[test]
    fn a_relative_entry_moves_by_the_offset() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        assert_eq!(
            entry
                .commit(
                    &document,
                    ["5", "-3"],
                    false,
                    false,
                    &mut AnchorIdMinter::new(99)
                )
                .0,
            EntryOutcome::Committed
        );
        assert!(near(top_left(&document, id), 15.0, 17.0));
    }

    /// Criterion 21: absolute (100, 50) puts the origin of a 30 x 40 rectangle
    /// at (100, 50).
    #[test]
    fn an_absolute_entry_puts_the_top_left_there() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        assert_eq!(
            entry
                .commit(
                    &document,
                    ["100", "50"],
                    true,
                    false,
                    &mut AnchorIdMinter::new(99)
                )
                .0,
            EntryOutcome::Committed
        );
        assert!(near(top_left(&document, id), 100.0, 50.0));
        let ObjectSnapshot::Primitive(p) = document.object(id).expect("exists") else {
            panic!("a primitive");
        };
        assert_eq!(p.rotation.as_radians(), 0.0, "rotation unchanged");
    }

    /// Criterion 21: for a rotated rectangle and for a star the reference is
    /// the top-left of the drawn outline's tight bounds, not of the frame box.
    #[test]
    fn absolute_uses_the_tight_bounds_of_a_rotated_rectangle_and_a_star() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 10.0);
        let turned = document
            .object(id)
            .expect("exists")
            .rotated(Point::new(25.0, 25.0), Angle::from_radians(0.5));
        document.rotate_object(&turned).expect("rotates");
        let star = document.create_star(
            StarFrame {
                center: Point::new(200.0, 100.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(-1.0),
            },
            PointCount::new(5).expect("count"),
            InnerRatio::new(0.5).expect("ratio"),
        );
        for id in [id, star] {
            let entry = entry_of(&document, id);
            assert_eq!(
                entry
                    .commit(
                        &document,
                        ["-12.5", "300.25"],
                        true,
                        false,
                        &mut AnchorIdMinter::new(99)
                    )
                    .0,
                EntryOutcome::Committed
            );
            assert!(near(top_left(&document, id), -12.5, 300.25), "{id:?}");
        }
    }

    /// Criterion 19: an untouched field means no change on its axis, in both
    /// modes, and the same text is read in the mode current at Enter.
    #[test]
    fn an_untouched_field_is_no_change_and_the_mode_decides_the_reading() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        // Relative: X typed, Y untouched.
        assert_eq!(
            entry.resolve(["5", "0"], false),
            Ok(Some(Vec2::new(5.0, 0.0)))
        );
        // Absolute: X untouched (shows the current 10.0), Y typed.
        assert_eq!(
            entry.resolve(["10.0", "50"], true),
            Ok(Some(Vec2::new(0.0, 30.0)))
        );
        // The same texts read in each mode.
        assert_eq!(
            entry.resolve(["100", "50"], false),
            Ok(Some(Vec2::new(100.0, 50.0)))
        );
        assert_eq!(
            entry.resolve(["100", "50"], true),
            Ok(Some(Vec2::new(90.0, 30.0)))
        );
    }

    /// Criterion 25: the result equal to the current position is no move.
    #[test]
    fn a_no_op_writes_nothing() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        let before = document.object(id);
        for (texts, absolute) in [
            (["0", "0"], false),
            (["0.0", "0.0"], false),
            (["0,0", "-0"], false),
            (["10.0", "20.0"], true),
            (["10", "20"], true),
        ] {
            assert_eq!(
                entry
                    .commit(
                        &document,
                        texts,
                        absolute,
                        false,
                        &mut AnchorIdMinter::new(99)
                    )
                    .0,
                EntryOutcome::Unchanged,
                "{texts:?} {absolute}"
            );
        }
        assert_eq!(document.object(id), before);
    }

    /// Criterion 22: text that is not a number, a value beyond the limit and a
    /// result beyond the limit are refused and write nothing; a comma, a point
    /// and a sign are accepted, also a negative absolute position.
    #[test]
    fn invalid_text_is_refused_and_writes_nothing() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let before = document.object(id);
        let entry = entry_of(&document, id);
        for (texts, field) in [
            (["", "1"], 0),
            (["1", "abc"], 1),
            (["1,2,3", "1"], 0),
            (["1", "1e8"], 1),
            (["NaN", "1"], 0),
            (["1", "100000000"], 1),
            (["1", "inf"], 1),
        ] {
            for absolute in [false, true] {
                assert_eq!(
                    entry
                        .commit(
                            &document,
                            texts,
                            absolute,
                            false,
                            &mut AnchorIdMinter::new(99)
                        )
                        .0,
                    EntryOutcome::Invalid {
                        field,
                        reason: InvalidReason::NotANumber
                    },
                    "{texts:?} {absolute}"
                );
            }
        }
        // Inside the limit as a number but the far edge lands beyond it.
        assert_eq!(
            entry.resolve(["9999999", "0"], false),
            Err((0, InvalidReason::NotANumber))
        );
        assert_eq!(document.object(id), before);
        assert!(
            entry
                .resolve(["1,5", "+2.5"], false)
                .expect("valid")
                .is_some()
        );
        assert_eq!(
            entry.resolve(["-100", "-50"], true),
            Ok(Some(Vec2::new(-110.0, -70.0)))
        );
    }

    /// Criterion 20: the typed relative move and a drag of the same offset
    /// leave identical registers, for a path and a primitive.
    #[test]
    fn the_typed_move_equals_a_drag_of_the_same_offset() {
        let build = || {
            let document = Document::new(1);
            let a = rect(&document, 10.0, 20.0, 30.0, 40.0);
            let b = document.create_path(
                &[
                    NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(1, 2), Point::new(7.0, 3.0)),
                ],
                false,
            );
            (document, a, b)
        };
        for pick in [0, 1] {
            let (typed_doc, a, b) = build();
            let (drag_doc, a2, b2) = build();
            let id = [a, b][pick];
            let id2 = [a2, b2][pick];
            let entry = entry_of(&typed_doc, id);
            assert_eq!(
                entry
                    .commit(
                        &typed_doc,
                        ["12.5", "-4.25"],
                        false,
                        false,
                        &mut AnchorIdMinter::new(99)
                    )
                    .0,
                EntryOutcome::Committed
            );
            drag_doc
                .translate_objects(&[id2], Vec2::new(12.5, -4.25))
                .expect("moves");
            assert_eq!(typed_doc.object(id), drag_doc.object(id2), "pick {pick}");
        }
    }

    /// Criterion 25: a collaborator's edit or delete while the chip is open
    /// makes Enter write nothing.
    #[test]
    fn a_stale_entry_writes_nothing() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        document
            .translate_objects(&[id], Vec2::new(1.0, 1.0))
            .expect("moves");
        let before = document.object(id);
        assert_eq!(
            entry
                .commit(
                    &document,
                    ["5", "5"],
                    false,
                    false,
                    &mut AnchorIdMinter::new(99)
                )
                .0,
            EntryOutcome::Unchanged
        );
        assert_eq!(document.object(id), before);
        document.delete_objects(&[id]).expect("deletes");
        assert_eq!(
            entry
                .commit(
                    &document,
                    ["5", "5"],
                    false,
                    false,
                    &mut AnchorIdMinter::new(99)
                )
                .0,
            EntryOutcome::Unchanged
        );
    }

    /// A value copied from the move readout ("\u{2212}3.0") is accepted in both
    /// modes, as is the ASCII sign.
    #[test]
    fn the_real_minus_sign_is_accepted() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = entry_of(&document, id);
        assert_eq!(
            entry.resolve(["\u{2212}3.0", "\u{2212}1"], false),
            entry.resolve(["-3.0", "-1"], false)
        );
        assert_eq!(
            entry.resolve(["\u{2212}3.0", "\u{2212}1"], false),
            Ok(Some(Vec2::new(-3.0, -1.0)))
        );
        assert_eq!(
            entry.resolve(["\u{2212}5", "20.0"], true),
            Ok(Some(Vec2::new(-15.0, 0.0)))
        );
    }

    /// `0047` criterion 38, worked example: typed 100 and 50 with the centre as the reference
    /// puts the centre of the 30 x 40 rectangle at (100, 50), the top-left corner at (85, 30).
    #[test]
    fn plain_enter_puts_the_centre_there() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = fresh_entry_of(&document, id);
        assert_eq!(
            entry
                .commit(
                    &document,
                    ["100", "50"],
                    true,
                    false,
                    &mut AnchorIdMinter::new(99)
                )
                .0,
            EntryOutcome::Committed
        );
        assert!(near(top_left(&document, id), 85.0, 30.0));
    }

    /// Criterion 40, the examples of the rectangle: typed 100 and 50 and each mark.
    #[test]
    fn each_reference_point_moves_that_point_to_the_typed_position() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = fresh_entry_of(&document, id);
        for (reference, origin) in [
            (ReferencePoint::TopLeft, (100.0, 50.0)),
            (ReferencePoint::BottomRight, (70.0, 10.0)),
            (ReferencePoint::Centre, (85.0, 30.0)),
            (ReferencePoint::LeftMiddle, (100.0, 30.0)),
            (ReferencePoint::BottomCentre, (85.0, 10.0)),
            (ReferencePoint::TopRight, (70.0, 50.0)),
            (ReferencePoint::TopCentre, (85.0, 50.0)),
            (ReferencePoint::RightMiddle, (70.0, 30.0)),
            (ReferencePoint::BottomLeft, (100.0, 10.0)),
        ] {
            let offset = entry
                .resolve_at(["100", "50"], true, reference)
                .expect("valid")
                .expect("moves");
            assert!(
                (10.0 + offset.x - origin.0).abs() < 1e-9
                    && (20.0 + offset.y - origin.1).abs() < 1e-9,
                "{reference:?}: {offset:?}"
            );
        }
    }

    /// Criterion 40: typed X = 100 only, then the top-right mark: the top-right corner's x
    /// becomes 100 and its y stays. The untouched field is compared with the prefill of the
    /// point that was clicked (criterion 44).
    #[test]
    fn an_untouched_field_is_untouched_for_the_point_clicked() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = fresh_entry_of(&document, id);
        // The top-right corner is at (40, 20); the fields showed its prefill "20.0" for Y.
        assert_eq!(
            entry.absolute_prefill_for(ReferencePoint::TopRight),
            ["40.0", "20.0"]
        );
        assert_eq!(
            entry.resolve_at(["100", "20.0"], true, ReferencePoint::TopRight),
            Ok(Some(Vec2::new(60.0, 0.0)))
        );
        // Nothing typed: both fields are the clicked point's prefill, nothing to apply
        // (criterion 45).
        assert_eq!(
            entry.resolve_at(["40.0", "20.0"], true, ReferencePoint::TopRight),
            Ok(None)
        );
        // A typed position equal to the point's current one is no move either.
        assert_eq!(
            entry.resolve_at(["40", "20"], true, ReferencePoint::TopRight),
            Ok(None)
        );
    }

    /// Criterion 40, rotated example: the 20 x 10 rectangle with its centre at (10, 5) turned by
    /// 30 degrees has the true bounds x -1.16 to 21.16, y -4.33 to 14.33; typed 0 and 0 and the
    /// top-left mark put the corner of those bounds at the origin, so the centre goes to
    /// (11.16, 9.33).
    #[test]
    fn the_marks_of_a_turned_rectangle_are_those_of_its_true_bounds() {
        let document = Document::new(1);
        let id = rect(&document, 0.0, 0.0, 20.0, 10.0);
        let turned = document.object(id).expect("exists").rotated(
            Point::new(10.0, 5.0),
            Angle::from_radians(30.0_f64.to_radians()),
        );
        document.rotate_object(&turned).expect("rotates");
        let entry = fresh_entry_of(&document, id);
        let (min, max) = entry.bounds();
        assert!((min.x + 1.160_254).abs() < 1e-5 && (max.x - 21.160_254).abs() < 1e-5);
        assert!((min.y + 4.330_127).abs() < 1e-5 && (max.y - 14.330_127).abs() < 1e-5);
        let (outcome, _) = entry.commit_at(
            &document,
            ["0", "0"],
            (true, ReferencePoint::TopLeft),
            false,
            &mut AnchorIdMinter::new(99),
        );
        assert_eq!(outcome, EntryOutcome::Committed);
        let centre = match document.object(id).expect("exists") {
            ObjectSnapshot::Primitive(p) => match p.shape {
                curvyo_document_core::Shape::Rect { bounds, .. } => Point::new(
                    bounds.origin.x + bounds.width.as_mm() / 2.0,
                    bounds.origin.y + bounds.height.as_mm() / 2.0,
                ),
                _ => panic!("a rectangle"),
            },
            ObjectSnapshot::Path(_) => panic!("a primitive"),
        };
        assert!(
            (centre.x - 11.160_254).abs() < 1e-5 && (centre.y - 9.330_127).abs() < 1e-5,
            "{centre:?}"
        );
    }

    /// Criterion 45: the coordinate limit is checked on all four edges, so it is the same for
    /// every reference point: a result that puts the far edge beyond it is refused, also for
    /// the near point.
    #[test]
    fn the_limit_holds_on_all_four_edges_for_every_reference() {
        let document = Document::new(1);
        let id = rect(&document, 10.0, 20.0, 30.0, 40.0);
        let entry = fresh_entry_of(&document, id);
        for reference in ReferencePoint::ALL {
            // The point goes to just inside the limit, the far edge lands beyond it.
            let result = entry.resolve_at(["9999990", "0"], true, reference);
            let (min, max) = entry.bounds();
            let at = reference.point((min, max));
            let moved_max = max.x + (9_999_990.0 - at.x);
            let moved_min = min.x + (9_999_990.0 - at.x);
            let inside = moved_max.abs() <= 1.0e7 && moved_min.abs() <= 1.0e7;
            assert_eq!(result.is_ok(), inside, "{reference:?}: {result:?}");
        }
    }
}
