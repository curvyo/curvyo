//! `Session`'s glue for the Select bar's ellipse group and "Curve" field
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 3, 26, 44, 59 and 65): each control
//! is one commit through `curvyo_ui_core`'s bar edits, followed by its polite
//! announcement. No rule lives here.

use curvyo_document_core::{Angle, ArcKind};
use curvyo_ui_core::{
    ArcEnd, EntryOutcome, InvalidReason, ParamValue, arc_announcement, curve_announcement,
    parse_entry_number,
};

use super::{Session, Tool};

impl Session {
    /// One bar commit of `value` on the selected objects it acts on, then the
    /// announcement of the first one that changed.
    fn commit_arc_or_curve(&mut self, value: ParamValue) -> EntryOutcome {
        self.recording(|this| -> EntryOutcome {
            if this.tool != Tool::Select {
                return EntryOutcome::Unchanged;
            }
            let objects = this.objects();
            let before = this.document.version();
            let _ = this
                .select
                .commit_bar_value(&this.document, &objects, &this.selection, value);
            if this.document.version() == before {
                return EntryOutcome::Unchanged;
            }
            this.transform_announcement = this.bar_announcement(value);
            EntryOutcome::Committed
        })
    }

    /// The announcement of the first selected object `value` acts on.
    fn bar_announcement(&self, value: ParamValue) -> Option<String> {
        self.selection.ids().iter().find_map(|id| {
            let shape = self.document.primitive(*id)?.shape;
            match value {
                ParamValue::Curve(_) => curve_announcement(&shape),
                _ => arc_announcement(&shape),
            }
        })
    }

    /// "Pie", "Arc" or "Chord" (criterion 3): one commit for every selected arc.
    pub fn set_selected_arc_kind(&mut self, kind: ArcKind) -> EntryOutcome {
        self.commit_arc_or_curve(ParamValue::ArcKind(kind))
    }

    /// "Whole ellipse" (criterion 3): one commit for every selected arc.
    pub fn make_selected_ellipses_whole(&mut self) -> EntryOutcome {
        self.commit_arc_or_curve(ParamValue::WholeEllipse)
    }

    /// Enter in "Start" or "End" (criteria 3 and 64): any number of degrees, wrapped into
    /// `[0°, 360°)`; text that is not a number keeps the field open, marked invalid.
    pub fn set_selected_arc_angle_text(&mut self, end: ArcEnd, text: &str) -> EntryOutcome {
        let Some(degrees) = parse_entry_number(text, true) else {
            return EntryOutcome::Invalid {
                field: 0,
                reason: InvalidReason::NotANumber,
            };
        };
        self.commit_arc_or_curve(ParamValue::ArcAngle(
            end,
            Angle::from_radians(degrees.to_radians()),
        ))
    }

    /// Enter in "Curve" (criteria 26 and 44): percent, limited per shape; text that is not
    /// a number keeps the field open, marked invalid.
    pub fn set_selected_curve_text(&mut self, text: &str) -> EntryOutcome {
        let Some(percent) = parse_entry_number(text, false) else {
            return EntryOutcome::Invalid {
                field: 0,
                reason: InvalidReason::NotANumber,
            };
        };
        self.commit_arc_or_curve(ParamValue::Curve(percent / 100.0))
    }

    /// A drag of the "Curve" field, on every tick: nothing is written; the shapes show in
    /// blue over their old outline. Released by [`Session::commit_selected_ratio`] (the
    /// one pending bar edit) or dropped by [`Session::cancel_selected_ratio`].
    pub fn preview_selected_curve(&mut self, percent: f64) {
        if !percent.is_finite() {
            return;
        }
        let objects = self.objects();
        self.select.preview_bar_value(
            &objects,
            &self.selection,
            ParamValue::Curve(percent / 100.0),
        );
    }

    /// An arrow-key step of "Start" or "End" (criterion 65), on every key: nothing is
    /// written; committed once by [`Session::commit_selected_curve`] (the one pending bar
    /// edit) on key-up.
    pub fn preview_selected_arc_angle(&mut self, end: ArcEnd, degrees: f64) {
        if !degrees.is_finite() {
            return;
        }
        let objects = self.objects();
        self.select.preview_bar_value(
            &objects,
            &self.selection,
            ParamValue::ArcAngle(end, Angle::from_radians(degrees.to_radians())),
        );
    }

    /// Commits a pending "Curve", "Start" or "End" step as one commit and announces it.
    pub fn commit_selected_curve(&mut self) {
        let pending = self.select.bar_preview().map(|preview| preview.value);
        let before = self.document.version();
        self.commit_selected_ratio();
        if let Some(value @ (ParamValue::Curve(_) | ParamValue::ArcAngle(..))) = pending
            && self.document.version() != before
        {
            self.transform_announcement = self.bar_announcement(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{ArcKind, Point, Shape};
    use curvyo_ui_core::{ArcEnd, EntryOutcome};

    use super::super::{Session, Tool};

    fn session_with_a_circle() -> (Session, curvyo_document_core::NodeId) {
        let mut session = Session::new(1);
        session.set_tool(Tool::Ellipse);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(40.0, 40.0), false, false);
        let id = session.document.object_ids()[0];
        (session, id)
    }

    /// Criteria 3, 59 and 65: a typed End on a whole circle makes an open arc in one
    /// step, announced; Pie and Whole ellipse are one step each.
    #[test]
    fn the_bar_makes_and_changes_an_arc() {
        let (mut session, id) = session_with_a_circle();
        assert_eq!(session.tool, Tool::Select);
        assert_eq!(
            session.set_selected_arc_angle_text(ArcEnd::End, "180"),
            EntryOutcome::Committed
        );
        assert_eq!(
            session.take_transform_announcement(),
            "Arc start 45.0\u{b0}, end 180.0\u{b0}, arc"
        );
        assert_eq!(
            session.set_selected_arc_kind(ArcKind::Pie),
            EntryOutcome::Committed
        );
        let Some(Shape::Ellipse { arc: Some(arc), .. }) =
            session.document.primitive(id).map(|p| p.shape)
        else {
            panic!("an arc")
        };
        assert_eq!(arc.kind, ArcKind::Pie);
        assert_eq!(
            session.set_selected_arc_kind(ArcKind::Pie),
            EntryOutcome::Unchanged
        );
        assert!(matches!(
            session.set_selected_arc_angle_text(ArcEnd::Start, "x"),
            EntryOutcome::Invalid { .. }
        ));
        assert_eq!(
            session.make_selected_ellipses_whole(),
            EntryOutcome::Committed
        );
        assert_eq!(session.take_transform_announcement(), "Whole ellipse");
    }

    /// Criteria 26 and 65: a typed curve, limited and announced; a field drag commits once.
    #[test]
    fn the_curve_field_types_and_drags() {
        let (mut session, id) = session_with_a_circle();
        assert_eq!(
            session.set_selected_curve_text("-150"),
            EntryOutcome::Committed
        );
        assert_eq!(
            session.take_transform_announcement(),
            "Curve \u{2212}100.0%"
        );
        let before = session.document.version();
        session.preview_selected_curve(20.0);
        session.preview_selected_curve(35.0);
        assert_eq!(
            session.document.version(),
            before,
            "a preview writes nothing"
        );
        session.commit_selected_curve();
        let Some(Shape::Ellipse { curve, .. }) = session.document.primitive(id).map(|p| p.shape)
        else {
            panic!("an ellipse")
        };
        assert!((curve.get() - 0.35).abs() < 1e-12);
        assert_eq!(session.take_transform_announcement(), "Curve 35.0%");
    }
}
