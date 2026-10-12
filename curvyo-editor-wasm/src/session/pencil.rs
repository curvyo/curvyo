//! `Session`'s Pencil glue (`specs/0032-pen-tablet-input`): pointer samples in, the session
//! settings and the readout of the Pencil bar, the blue preview of the stroke in progress, and
//! the one commit of a finished stroke. The stroke rules are `curvyo_ui_core`'s, the geometry
//! `curvyo_geometry_core`'s. Samples, preview and settings are ephemeral (ADR 0009 §2): none of
//! them reaches the document (criteria 14, 16).

use curvyo_document_core::{
    AnchorKind, Angle, Document, Length, NewAnchor, NodeId, Point, Style, StyleEdit, SubpathRef,
    Vec2, ViewTransform,
};
use curvyo_render_core::{DrawList, build_outline_preview};
use curvyo_ui_core::{
    AnchorIdMinter, PENCIL_COMMIT_LABEL, PencilSettings, PencilTool, PointerKind, PointerSample,
    PressureCurve, StrokeShape,
};

use super::{Session, Tool, sanitized_point};

/// Which part of a pointer gesture a batch of samples belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PencilPhase {
    /// `pointerdown`.
    Down,
    /// `pointermove`, with or without contact.
    Move,
    /// `pointerup`.
    Up,
}

/// The values of the muted readout "Pen: pressure 0.62, tilt 12° / -3°" (criterion 15).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenReadout {
    /// The pressure from 0 to 1.
    pub pressure: f64,
    /// The tilt along X.
    pub tilt_x: Angle,
    /// The tilt along Y.
    pub tilt_y: Angle,
}

/// What the Pencil bar shows (criteria 14, 15).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PencilBar {
    /// A pen has been seen in this session: the bar is in the tree.
    pub pen_seen: bool,
    /// A stroke with pressure has been drawn in this session: Min, Max and Curve show.
    pub pressure_seen: bool,
    /// The readout while the last reading came from a pen.
    pub readout: Option<PenReadout>,
    /// The settings for the next stroke.
    pub settings: PencilSettings,
}

/// The Pencil's part of the session.
#[derive(Debug, Default)]
pub(super) struct PencilState {
    tool: PencilTool,
    settings: PencilSettings,
    pen_seen: bool,
    pressure_seen: bool,
    readout: Option<PenReadout>,
}

impl PencilState {
    /// Drops a stroke in progress; returns whether there was one.
    pub(super) fn cancel(&mut self) -> bool {
        self.tool.cancel()
    }

    /// Whether a stroke is in progress.
    pub(super) const fn is_drawing(&self) -> bool {
        self.tool.is_drawing()
    }
}

/// Node kinds of a fitted centre line: the ends are corners, the inner nodes are smooth with
/// handles of their own lengths.
fn centre_anchors(minter: &mut AnchorIdMinter, centre: &[(Point, Vec2, Vec2)]) -> Vec<NewAnchor> {
    let last = centre.len().saturating_sub(1);
    centre
        .iter()
        .enumerate()
        .map(|(index, &(point, handle_in, handle_out))| NewAnchor {
            id: minter.mint(),
            point,
            handle_in,
            handle_out,
            kind: if index == 0 || index == last {
                AnchorKind::Corner
            } else {
                AnchorKind::Asymmetric
            },
        })
        .collect()
}

/// Corner nodes on the vertices of an outline.
fn outline_anchors(minter: &mut AnchorIdMinter, outline: &[Point]) -> Vec<NewAnchor> {
    outline
        .iter()
        .map(|point| NewAnchor::corner(minter.mint(), *point))
        .collect()
}

/// The anchors and closed flag of every outline of a shape, ids from `minter`.
fn shape_outlines(minter: &mut AnchorIdMinter, shape: &StrokeShape) -> Vec<(Vec<NewAnchor>, bool)> {
    match shape {
        StrokeShape::Centre(centre) => vec![(centre_anchors(minter, centre), false)],
        StrokeShape::Outlines(outlines) => outlines
            .iter()
            .map(|outline| (outline_anchors(minter, outline), true))
            .collect(),
    }
}

/// Writes a stroke's outlines as one new path in one step labelled `pencil_stroke`: an open
/// path in the new-path style, or a closed (possibly compound) path filled with the new-path
/// stroke colour and no stroke (criteria 7, 8, 13). Returns the new object.
fn write_stroke(document: &Document, outlines: &[(Vec<NewAnchor>, bool)], filled: bool) -> NodeId {
    document.batch(PENCIL_COMMIT_LABEL, |document| {
        // invariant: `shape_outlines` gives one outline for a centre line and one per outline
        // of a variable-width result, which `variable_outline` never returns empty.
        let (first, closed) = &outlines[0];
        let id = document.create_path(first, *closed);
        if !filled {
            return id;
        }
        let colour = Style::default().stroke.color;
        // The object was created in this batch, so it exists; a style edit of a fill or
        // stroke switch cannot be refused for a path.
        for edit in [
            StyleEdit::FillColor(colour),
            StyleEdit::FillEnabled(true),
            StyleEdit::StrokeEnabled(false),
        ] {
            let _ = document.edit_style(&[id], &edit);
        }
        if outlines.len() == 1 {
            return id;
        }
        // Holes: the path is replaced by one compound path with its style, in the same step.
        // invariant: `id` was created above and is its own base, no outline is empty, and
        // every anchor id is freshly minted (the first outline's ids belong to `id`, which is
        // replaced), so none of the refusals of `replace_with_path` can apply.
        #[allow(clippy::expect_used)]
        document
            .replace_with_path(&[id], id, outlines, PENCIL_COMMIT_LABEL)
            .expect("replacing a path created in the same batch by its outlines")
    })
}

impl Session {
    /// One DOM pointer event of the Pencil with every sample it carries (the coalesced ones
    /// first), in document millimetres (criteria 1 to 5). A press with the tip or the left button
    /// starts a stroke, a move extends it or, without contact, only moves the pointer, and the
    /// release commits it (criteria 7, 8, 12, 13). Touch is ignored (criterion 4).
    pub fn pencil_input(&mut self, phase: PencilPhase, buttons: u32, samples: &[PointerSample]) {
        let samples: Vec<PointerSample> = samples
            .iter()
            .filter_map(|sample| {
                sanitized_point(sample.position).map(|position| PointerSample {
                    position,
                    ..*sample
                })
            })
            .collect();
        let Some(last) = samples.last().copied() else {
            return;
        };
        if last.kind == PointerKind::Touch {
            return;
        }
        self.note_reading(&last);
        self.pointer_position = Some(last.position);
        match phase {
            PencilPhase::Down => {
                self.flush_select_bar_preview();
                self.flush_style_preview();
                if self.pencil.tool.press(samples[0], buttons) {
                    self.button_down = true;
                    self.pencil.tool.extend(&samples[1..]);
                }
            }
            PencilPhase::Move => self.pencil.tool.extend(&samples),
            PencilPhase::Up => {
                self.button_down = false;
                self.recording(|this| this.commit_pencil_stroke(&samples));
            }
        }
    }

    /// Updates the "pen seen" flag and the readout from the last reading of an event.
    fn note_reading(&mut self, sample: &PointerSample) {
        if sample.kind == PointerKind::Pen {
            self.pencil.pen_seen = true;
            self.pencil.readout = Some(PenReadout {
                pressure: sample.pressure,
                tilt_x: sample.tilt_x,
                tilt_y: sample.tilt_y,
            });
        } else {
            self.pencil.readout = None;
        }
    }

    /// The release: one commit for a stroke that has a shape, the new object selected, the Pencil
    /// still active (criterion 13); nothing for a stroke that is too short (criterion 12).
    fn commit_pencil_stroke(&mut self, samples: &[PointerSample]) {
        let Some(stroke) = self.pencil.tool.release(samples) else {
            return;
        };
        let filled = stroke.has_pressure();
        let Some(shape) = stroke.shape(&self.pencil.settings) else {
            return;
        };
        if filled {
            self.pencil.pressure_seen = true;
        }
        let outlines = shape_outlines(&mut self.minter, &shape);
        let id = write_stroke(&self.document, &outlines, filled);
        self.selection.select_single(id);
    }

    /// What the Pencil bar shows (criteria 14, 15).
    #[must_use]
    pub fn pencil_bar(&self) -> PencilBar {
        PencilBar {
            pen_seen: self.pencil.pen_seen,
            pressure_seen: self.pencil.pressure_seen,
            readout: self.pencil.readout,
            settings: self.pencil.settings,
        }
    }

    /// Sets the Minimum width for the next stroke (criterion 14).
    pub fn set_pencil_minimum(&mut self, width: Length) {
        self.pencil.settings.set_minimum(width);
    }

    /// Sets the Maximum width for the next stroke (criterion 14).
    pub fn set_pencil_maximum(&mut self, width: Length) {
        self.pencil.settings.set_maximum(width);
    }

    /// Sets the pressure curve for the next stroke (criterion 14).
    pub fn set_pencil_curve(&mut self, curve: PressureCurve) {
        self.pencil.settings.curve = curve;
    }

    /// The blue preview of the stroke in progress: its centre line, or its outline once it has
    /// pressure (criterion 11). Empty while no stroke is in progress.
    pub(super) fn pencil_preview_list(&self, view: ViewTransform) -> DrawList {
        if self.tool != Tool::Pencil {
            return DrawList::default();
        }
        let Some(shape) = self.pencil.tool.preview(&self.pencil.settings) else {
            return DrawList::default();
        };
        // Preview ids are never stored, so they come from a throwaway minter and the session's
        // own minter stays in step.
        let mut minter = AnchorIdMinter::new(0);
        let outlines = shape_outlines(&mut minter, &shape);
        let refs: Vec<SubpathRef<'_>> = outlines
            .iter()
            .map(|(anchors, closed)| SubpathRef {
                closed: *closed,
                anchors,
            })
            .collect();
        build_outline_preview(&refs, view)
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{ObjectSnapshot, PathSnapshot};
    use curvyo_ui_core::known_operation_name;

    use super::*;

    const TIP: u32 = 1;

    /// The unsigned area of a polygon, in square millimetres.
    fn signed_area_of(points: &[Point]) -> f64 {
        let mut twice = 0.0;
        for (index, a) in points.iter().enumerate() {
            let b = points[(index + 1) % points.len()];
            twice += a.x * b.y - b.x * a.y;
        }
        twice / 2.0
    }

    fn reading(kind: PointerKind, x: f64, y: f64, pressure: f64) -> PointerSample {
        PointerSample {
            position: Point::new(x, y),
            kind,
            pressure,
            tilt_x: Angle::from_radians(12.0_f64.to_radians()),
            tilt_y: Angle::from_radians(-3.0_f64.to_radians()),
            eraser: false,
            time_ms: x,
        }
    }

    /// A stroke along y = 20 from x = 10 to x = 60: a press, `moves` move events of one sample
    /// each and a release.
    fn draw(session: &mut Session, kind: PointerKind, pressure: impl Fn(f64) -> f64, moves: usize) {
        let x_at = |i: usize| {
            10.0 + 50.0 * f64::from(u32::try_from(i).unwrap())
                / f64::from(u32::try_from(moves + 1).unwrap())
        };
        session.pencil_input(
            PencilPhase::Down,
            TIP,
            &[reading(kind, x_at(0), 20.0, pressure(0.0))],
        );
        for i in 1..=moves {
            let x = x_at(i);
            session.pencil_input(
                PencilPhase::Move,
                TIP,
                &[reading(kind, x, 20.0, pressure((x - 10.0) / 50.0))],
            );
        }
        session.pencil_input(
            PencilPhase::Up,
            0,
            &[reading(kind, 60.0, 20.0, pressure(1.0))],
        );
    }

    fn pencil_session() -> Session {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pencil);
        session
    }

    fn only_path(session: &Session) -> PathSnapshot {
        let objects = session.objects();
        assert_eq!(objects.len(), 1, "one object");
        match &objects[0] {
            ObjectSnapshot::Path(path) => path.clone(),
            ObjectSnapshot::Primitive(_) => panic!("a path was expected"),
        }
    }

    #[test]
    fn criterion_7_a_mouse_stroke_is_one_open_path_in_the_new_path_style() {
        let mut session = pencil_session();
        draw(&mut session, PointerKind::Mouse, |_| 0.5, 98);
        let path = only_path(&session);
        assert!(!path.closed);
        assert_eq!(path.anchors.len(), 2);
        assert_eq!(path.style, Style::default());
    }

    #[test]
    fn criterion_8_a_pressure_stroke_is_one_filled_closed_path_without_stroke() {
        let mut session = pencil_session();
        draw(&mut session, PointerKind::Pen, |t| 0.2 + 0.6 * t, 98);
        let path = only_path(&session);
        assert!(path.closed);
        assert!(path.style.fill.enabled);
        assert_eq!(path.style.fill.color, Style::default().stroke.color);
        assert!(!path.style.stroke.enabled);
        assert!(path.anchors.len() > 4);
        let points: Vec<Point> = path.anchors.iter().map(|a| a.point).collect();
        assert!(signed_area_of(&points).abs() > 10.0);
    }

    #[test]
    fn criterion_13_one_commit_named_pencil_stroke_the_object_selected_the_tool_kept() {
        let mut session = pencil_session();
        let before = session.document.own_step_count();
        draw(&mut session, PointerKind::Pen, |t| 0.3 + 0.5 * t, 20);
        let steps = session.document.own_steps_from(before);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, PENCIL_COMMIT_LABEL);
        assert_eq!(
            known_operation_name(PENCIL_COMMIT_LABEL),
            Some("Pencil stroke")
        );
        assert_eq!(session.selection.ids(), &[only_path(&session).id]);
        assert_eq!(session.tool(), Tool::Pencil);
        assert_eq!(session.undo_len(), 1);
    }

    #[test]
    fn criterion_12_escape_mid_stroke_writes_nothing() {
        let mut session = pencil_session();
        let pen = |x: f64| reading(PointerKind::Pen, x, 20.0, 0.4);
        session.pencil_input(PencilPhase::Down, TIP, &[pen(0.0)]);
        session.pencil_input(PencilPhase::Move, TIP, &[pen(5.0), pen(10.0)]);
        assert_eq!(session.escape(), super::super::EscapeStep::CancelledDrag);
        session.pencil_input(PencilPhase::Up, 0, &[pen(15.0)]);
        assert!(session.objects().is_empty());
        assert_eq!(session.tool(), Tool::Pencil);
        // The next Escape leaves the tool.
        assert_eq!(session.escape(), super::super::EscapeStep::LeftTool);
    }

    #[test]
    fn criterion_12_a_tiny_stroke_creates_nothing() {
        let mut session = pencil_session();
        let before = session.document.own_step_count();
        let pen = |x: f64| reading(PointerKind::Pen, x, 20.0, 0.4);
        session.pencil_input(PencilPhase::Down, TIP, &[pen(0.0)]);
        session.pencil_input(PencilPhase::Up, 0, &[pen(0.05)]);
        session.pencil_input(PencilPhase::Down, TIP, &[pen(0.0)]);
        session.pencil_input(PencilPhase::Up, 0, &[]);
        assert!(session.objects().is_empty());
        assert_eq!(session.document.own_step_count(), before);
    }

    #[test]
    fn criteria_2_and_4_coalesced_samples_count_and_touch_is_ignored() {
        let mut session = pencil_session();
        let pen = |x: f64| reading(PointerKind::Pen, x, 20.0, 0.4);
        session.pencil_input(PencilPhase::Down, TIP, &[pen(0.0)]);
        session.pencil_input(
            PencilPhase::Move,
            TIP,
            &[pen(1.0), pen(2.0), pen(3.0), pen(4.0), pen(5.0)],
        );
        assert!(session.pencil.is_drawing());
        assert!(session.pencil.cancel());

        let touch = |x: f64| reading(PointerKind::Touch, x, 20.0, 0.5);
        session.pencil_input(PencilPhase::Down, TIP, &[touch(0.0)]);
        session.pencil_input(PencilPhase::Move, TIP, &[touch(30.0)]);
        session.pencil_input(PencilPhase::Up, 0, &[touch(60.0)]);
        assert!(session.objects().is_empty());
        assert!(!session.pencil.is_drawing());
    }

    #[test]
    fn criterion_5_a_hovering_pen_starts_nothing_but_moves_the_pointer() {
        let mut session = pencil_session();
        let hover = reading(PointerKind::Pen, 33.0, 44.0, 0.0);
        session.pencil_input(PencilPhase::Move, 0, &[hover]);
        session.pencil_input(PencilPhase::Down, 0, &[hover]);
        assert!(!session.pencil.is_drawing());
        assert_eq!(session.pointer_position, Some(Point::new(33.0, 44.0)));
    }

    #[test]
    fn criteria_14_and_15_the_bar_appears_with_a_pen_and_its_controls_after_pressure() {
        let mut session = pencil_session();
        draw(&mut session, PointerKind::Mouse, |_| 0.5, 10);
        let bar = session.pencil_bar();
        assert!(!bar.pen_seen && !bar.pressure_seen && bar.readout.is_none());

        session.pencil_input(
            PencilPhase::Move,
            0,
            &[reading(PointerKind::Pen, 5.0, 5.0, 0.0)],
        );
        let bar = session.pencil_bar();
        assert!(bar.pen_seen && !bar.pressure_seen);
        let readout = bar.readout.unwrap();
        assert!((readout.tilt_x.as_radians().to_degrees() - 12.0).abs() < 1e-9);
        assert!((readout.tilt_y.as_radians().to_degrees() + 3.0).abs() < 1e-9);

        // A pen without pressure shows the readout but not the controls.
        draw(&mut session, PointerKind::Pen, |_| 0.5, 10);
        assert!(!session.pencil_bar().pressure_seen);
        draw(&mut session, PointerKind::Pen, |t| 0.1 + 0.8 * t, 10);
        assert!(session.pencil_bar().pressure_seen);

        // A mouse reading hides the readout again.
        session.pencil_input(
            PencilPhase::Move,
            0,
            &[reading(PointerKind::Mouse, 5.0, 5.0, 0.0)],
        );
        assert!(session.pencil_bar().readout.is_none());
    }

    #[test]
    fn criterion_14_a_change_applies_from_the_next_stroke() {
        let mut thin = pencil_session();
        draw(&mut thin, PointerKind::Pen, |_| 0.9, 40);
        let mut wide = pencil_session();
        wide.set_pencil_maximum(Length::from_mm(6.0));
        wide.set_pencil_curve(PressureCurve::Soft);
        draw(&mut wide, PointerKind::Pen, |_| 0.9, 40);
        let area = |session: &Session| {
            let points: Vec<Point> = only_path(session).anchors.iter().map(|a| a.point).collect();
            signed_area_of(&points).abs()
        };
        assert!(
            area(&wide) > 2.5 * area(&thin),
            "{} vs {}",
            area(&wide),
            area(&thin)
        );
        assert!((wide.pencil_bar().settings.maximum().as_mm() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn criterion_16_no_sample_and_no_setting_reaches_the_saved_file() {
        let mut session = pencil_session();
        session.set_pencil_minimum(Length::from_mm(0.0573));
        session.set_pencil_maximum(Length::from_mm(7.3141));
        draw(&mut session, PointerKind::Pen, |t| 0.2 + 0.7 * t, 30);
        let bytes = session.pack("0.1.0").unwrap();
        let reopened = curvyo_document_core::unpack(9, &bytes).unwrap();
        let objects: Vec<_> = reopened
            .object_ids()
            .into_iter()
            .filter_map(|id| reopened.object(id))
            .collect();
        assert_eq!(objects.len(), 1);
        let json = serde_json::to_string(&objects).unwrap();
        // The widths have four decimals, which no coordinate on the 0.001 mm grid has.
        for forbidden in ["pressure", "tilt", "pointer", "0.0573", "7.3141"] {
            assert!(
                !json.contains(forbidden),
                "{forbidden} found in the document"
            );
        }
    }

    #[test]
    fn criterion_11_the_preview_is_drawn_while_a_stroke_is_in_progress_and_matches_the_object() {
        let mut session = pencil_session();
        let view = session.view();
        assert_eq!(session.pencil_preview_list(view).triangles.len(), 0);
        let pen = |x: f64, p: f64| reading(PointerKind::Pen, x, 20.0, p);
        session.pencil_input(PencilPhase::Down, TIP, &[pen(10.0, 0.3)]);
        let middle: Vec<_> = (1..40)
            .map(|i| pen(10.0 + f64::from(i), 0.3 + f64::from(i) / 100.0))
            .collect();
        session.pencil_input(PencilPhase::Move, TIP, &middle);
        assert_ne!(session.pencil_preview_list(view).triangles.len(), 0);
        let preview = session
            .pencil
            .tool
            .preview(&session.pencil.settings)
            .unwrap();
        session.pencil_input(PencilPhase::Up, 0, &[pen(50.0, 0.7)]);
        assert_eq!(session.pencil_preview_list(view).triangles.len(), 0);
        // The preview of the samples so far and the object agree within 0.15 mm wherever both
        // exist: every preview vertex short of the last sample lies on the object's outline.
        let StrokeShape::Outlines(outlines) = preview else {
            panic!("an outline preview was expected");
        };
        let object: Vec<Point> = only_path(&session)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect();
        let distance = |p: &Point| {
            (0..object.len())
                .map(|i| {
                    let (a, b) = (object[i], object[(i + 1) % object.len()]);
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy))
                        .clamp(0.0, 1.0);
                    (a.x + t * dx - p.x).hypot(a.y + t * dy - p.y)
                })
                .fold(f64::INFINITY, f64::min)
        };
        let near_start = outlines[0].iter().filter(|p| p.x < 40.0);
        for point in near_start {
            assert!(
                distance(point) <= 0.15,
                "{point:?} is {} mm off",
                distance(point)
            );
        }
    }

    #[test]
    fn the_pencil_cancels_its_stroke_when_the_tool_changes() {
        let mut session = pencil_session();
        session.pencil_input(
            PencilPhase::Down,
            TIP,
            &[reading(PointerKind::Pen, 0.0, 0.0, 0.4)],
        );
        session.set_tool(Tool::Select);
        assert!(!session.pencil.is_drawing());
    }

    /// Criterion 6: `P` picks the Pencil in every tool, and not while a stroke is in progress.
    #[test]
    fn criterion_6_the_key_p_picks_the_pencil() {
        use super::super::{KeyInput, KeyOutcome};
        for tool in [
            Tool::Select,
            Tool::Pen,
            Tool::Node,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::PolygonStar,
        ] {
            let mut session = Session::new(1);
            session.set_tool(tool);
            let outcome = session.key_down(KeyInput {
                key: "P",
                ..KeyInput::default()
            });
            assert_eq!(outcome, KeyOutcome::ToolChanged, "{tool:?}");
            assert_eq!(session.tool(), Tool::Pencil);
        }
        let mut session = pencil_session();
        session.pencil_input(
            PencilPhase::Down,
            TIP,
            &[reading(PointerKind::Mouse, 0.0, 0.0, 0.5)],
        );
        session.set_tool(Tool::Pencil);
        assert!(session.pencil.is_drawing());
        let outcome = session.key_down(KeyInput {
            key: "b",
            ..KeyInput::default()
        });
        assert_eq!(
            outcome,
            KeyOutcome::Ignored,
            "no tool letter during a stroke"
        );
    }

    /// Architect review: a pressure stroke that crosses itself around an empty area keeps its
    /// hole as a compound path in exactly one step.
    #[test]
    fn criterion_8_a_pressure_stroke_that_crosses_itself_keeps_its_hole_in_one_step() {
        let mut session = pencil_session();
        let before = session.document.own_step_count();
        // A loop of radius 15 mm drawn 1.2 times round, so the stroke crosses its own start.
        let at = |i: u32| {
            let angle = std::f64::consts::TAU * 1.2 * f64::from(i) / 300.0;
            reading(
                PointerKind::Pen,
                50.0 + 15.0 * angle.cos(),
                50.0 + 15.0 * angle.sin(),
                0.6,
            )
        };
        session.pencil_input(PencilPhase::Down, TIP, &[at(0)]);
        let middle: Vec<_> = (1..300).map(at).collect();
        session.pencil_input(PencilPhase::Move, TIP, &middle);
        session.pencil_input(PencilPhase::Up, 0, &[at(300)]);
        let steps = session.document.own_steps_from(before);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, PENCIL_COMMIT_LABEL);
        let path = only_path(&session);
        assert!(path.closed);
        assert_eq!(
            path.extra_subpaths.len(),
            1,
            "an outer outline and one hole"
        );
        let area = |anchors: &[NewAnchor]| {
            let points: Vec<Point> = anchors.iter().map(|a| a.point).collect();
            signed_area_of(&points)
        };
        let (outer, hole) = (area(&path.anchors), area(&path.extra_subpaths[0].anchors));
        assert!(
            outer * hole < 0.0,
            "the hole is wound against the outer outline"
        );
        assert!(hole.abs() > 400.0, "hole area {hole}");
        assert_eq!(session.selection.ids(), &[path.id]);
    }
}
