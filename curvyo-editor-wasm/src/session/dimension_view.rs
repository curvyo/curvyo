//! Lays out and draws the Vector dimensions for the current view and hands their texts, the
//! Measure tool's readout and its mode label to the frontend overlay
//! (`specs/0070-measure-and-dimension` criteria 1, 2, 7, 8 and 12).

use std::collections::HashMap;

use curvyo_document_core::{
    DimensionKind, DimensionPlace, DimensionSnapshot, DisplayUnit, Measured, NodeId,
    ObjectSnapshot, Point, ViewTransform,
};
use curvyo_render_core::{DimensionLook, DimensionMarkDrawing, DrawList, build_dimension_marks};
use curvyo_ui_core::{DimensionMarks, MeasurePick, VECTOR_DECIMALS};
use serde::Serialize;

use super::{Session, Tool};

/// Hit target of a dimension text: at least 24px high, the estimated text width wide.
const TEXT_HIT_HEIGHT_PX: f64 = 24.0;
const TEXT_CHAR_PX: f64 = 7.0;
const TEXT_PAD_PX: f64 = 8.0;
/// Lines are hit within 4px on either side (an 8px band).
const LINE_HIT_PX: f64 = 4.0;

/// One Vector dimension laid out for the current view.
#[derive(Debug, Clone)]
pub(super) struct DimensionRecord {
    /// The node; `None` for the preview of the Measure tool.
    pub(super) id: Option<NodeId>,
    pub(super) kind: DimensionKind,
    pub(super) place: DimensionPlace,
    pub(super) points: Vec<Point>,
    /// "12.5 mm", "R 25 mm", "⌀ 50 mm", "45°".
    pub(super) text: String,
    /// "Dimension 12.5 millimetres", with ", not linked" when unlinked.
    pub(super) name: String,
    pub(super) linked: bool,
    pub(super) selected: bool,
    pub(super) marks: DimensionMarks,
    /// Screen position of the text centre, CSS pixels.
    pub(super) screen: (f64, f64),
}

impl DimensionRecord {
    fn half_width(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // a short label's character count
        let chars = self.text.chars().count() as f64;
        // The `Unlink` glyph of an unlinked dimension widens the label.
        let glyph = if self.linked { 0.0 } else { 7.0 };
        f64::midpoint(TEXT_CHAR_PX * chars, TEXT_PAD_PX) + glyph
    }

    /// Whether the screen point `(x, y)` is on the text or within 4px of a line.
    pub(super) fn hit(&self, x: f64, y: f64, view: ViewTransform) -> bool {
        let on_text = (x - self.screen.0).abs() <= self.half_width()
            && (y - self.screen.1).abs() <= TEXT_HIT_HEIGHT_PX / 2.0;
        on_text
            || self.marks.lines.iter().any(|&(a, b)| {
                let (a, b) = (view.document_to_screen(a), view.document_to_screen(b));
                segment_distance((x, y), a, b) <= LINE_HIT_PX
            })
    }
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.mul_add(dx, dy * dy);
    let t = if length > 0.0 {
        ((p.0 - a.0).mul_add(dx, (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - t.mul_add(dx, a.0)).hypot(p.1 - t.mul_add(dy, a.1))
}

/// The text and accessible name of a dimension of `kind` showing `value`.
pub(super) fn dimension_words(
    kind: DimensionKind,
    value: curvyo_document_core::DimensionValue,
    linked: bool,
    unit: DisplayUnit,
) -> (String, String) {
    let text = curvyo_ui_core::dimension_text(kind, value, unit, VECTOR_DECIMALS);
    let name = format!(
        "Dimension {}{}",
        curvyo_ui_core::dimension_speech(kind, value, unit),
        if linked { "" } else { ", not linked" }
    );
    (text, name)
}

/// One label of the overlay, CSS pixels.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct LabelJson {
    id: String,
    text: String,
    name: String,
    x: f64,
    y: f64,
    linked: bool,
    selected: bool,
    preview: bool,
}

/// A chip at a screen point.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct ChipJson {
    lines: Vec<String>,
    x: f64,
    y: f64,
}

/// What the overlay shows.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
struct MeasureViewJson {
    dimensions: Vec<LabelJson>,
    mode: Option<ChipJson>,
    readout: Option<ChipJson>,
    /// What the polite live region says: the value of the last pin or placement.
    speech: String,
}

/// A Select-tool drag of a dimension's text or line (criterion 8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DimensionDrag {
    pub(super) id: NodeId,
    pub(super) start: Point,
    pub(super) place: DimensionPlace,
    pub(super) moved: bool,
}

impl Session {
    /// Every dimension among `objects`, resolved against the same objects (the preview of a drag
    /// included, criterion 11) and laid out for the current view.
    pub(super) fn dimension_records(&self, objects: &[ObjectSnapshot]) -> Vec<DimensionRecord> {
        let has = objects
            .iter()
            .any(|o| matches!(o, ObjectSnapshot::Dimension(_)));
        if !has {
            return Vec::new();
        }
        let index: HashMap<NodeId, &ObjectSnapshot> = objects
            .iter()
            .filter(|o| !matches!(o, ObjectSnapshot::Dimension(_)))
            .map(|o| (o.id(), o))
            .collect();
        let selected = self.selection.ids();
        let dimensions: Vec<DimensionSnapshot> = objects
            .iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Dimension(d) => {
                    let mut d = (**d).clone();
                    curvyo_document_core::resolve_dimension_into(&mut d, |id| {
                        index.get(&id).copied()
                    });
                    if let Some(drag) = self.dimension_drag
                        && drag.id == d.id
                    {
                        d.place = drag.place;
                    }
                    if let Some((id, index, at)) = self.kept_point_drag
                        && id == d.id
                        && !d.linked
                        && let Some(point) = d.shown.points.get_mut(index)
                    {
                        *point = at;
                    }
                    Some(d)
                }
                _ => None,
            })
            .collect();
        dimensions
            .iter()
            .filter_map(|d| {
                self.layout_record(
                    Some(d.id),
                    d.kind,
                    &d.shown,
                    d.place,
                    d.linked,
                    self.tool == Tool::Select && selected.contains(&d.id),
                )
            })
            .collect()
    }

    /// One dimension laid out for the view.
    fn layout_record(
        &self,
        id: Option<NodeId>,
        kind: DimensionKind,
        shown: &Measured,
        place: DimensionPlace,
        linked: bool,
        selected: bool,
    ) -> Option<DimensionRecord> {
        let (text, name) = dimension_words(kind, shown.value, linked, self.document.display_unit());
        let item = curvyo_ui_core::DimensionItem {
            geometry: curvyo_ui_core::dimension_geometry(kind, &shown.points, place.sector)?,
            label: curvyo_ui_core::label_point(kind, &shown.points, place),
            text_chars: text.chars().count(),
        };
        let view = self.view();
        let marks = curvyo_ui_core::layout_dimensions(&[item], view)
            .into_iter()
            .next()?;
        Some(DimensionRecord {
            id,
            kind,
            place,
            points: shown.points.clone(),
            screen: view.document_to_screen(marks.text),
            text,
            name,
            linked,
            selected,
            marks,
        })
    }

    /// The marks of `records`: guide green, solid; dashed when unlinked; accent when selected.
    pub(super) fn dimension_draw_list(
        records: &[DimensionRecord],
        view: ViewTransform,
    ) -> DrawList {
        let mut list = DrawList::default();
        for record in records {
            let arrows: Vec<_> = record
                .marks
                .arrows
                .iter()
                .map(|a| (a.tip, a.back))
                .collect();
            list.extend(build_dimension_marks(
                &DimensionMarkDrawing {
                    lines: &record.marks.lines,
                    arrows: &arrows,
                    look: DimensionLook::Guide,
                    dashed: !record.linked,
                    hovered: false,
                    selected: record.selected,
                },
                view,
            ));
        }
        list
    }

    /// The Measure tool's preview dimension, when the gesture leaves one (criterion 4).
    fn measure_preview_record(&self) -> Option<DimensionRecord> {
        let pointer = self.pointer_position?;
        if self.tool != Tool::Measure || !self.measure_leaves_dimension() {
            return None;
        }
        let measuring = self.measuring(pointer)?;
        let place = self.measure_place(&measuring, pointer);
        let value = curvyo_document_core::measure(measuring.kind, &measuring.points, place.sector)?;
        let shown = Measured {
            points: measuring.points.clone(),
            value,
        };
        self.layout_record(None, measuring.kind, &shown, place, true, false)
    }

    /// The dimensions and the Measure tool's drawing over the artwork of `objects`; keeps the
    /// labels for the overlay of this frame.
    pub(super) fn dimension_layer(&self, objects: &[ObjectSnapshot]) -> DrawList {
        let view = self.view();
        let mut records = self.dimension_records(objects);
        let mut list = DrawList::default();
        if self.tool == Tool::Measure {
            if let Some(preview) = self.measure_preview_record() {
                records.push(preview);
                // Shift held after the first pick: its dot stays, as in the blue state (`0070`
                // UI review item 6).
                if let Some(first) = self.measure_open_anchor() {
                    list.extend(curvyo_render_core::build_measure_dots(&[first], view));
                }
            } else {
                list.extend(self.measure_line_list(view));
            }
            list.extend(self.measure_marker_list(view));
        }
        let mut drawn = Self::dimension_draw_list(&records, view);
        drawn.extend(list);
        *self.dimension_labels.borrow_mut() = records;
        drawn
    }

    /// The blue measurement: the line, its ticks and the anchor dots (UX notes §3).
    fn measure_line_list(&self, view: ViewTransform) -> DrawList {
        let mut list = DrawList::default();
        let Some(pointer) = self.pointer_position else {
            return list;
        };
        if let Some(MeasurePick::Curve { center, radius, .. }) = self.measure_first() {
            let radius = radius.as_mm();
            let d = center.vector_to(pointer);
            let rim = if d.length() > 0.0 {
                center.translated(d.normalized_to(radius))
            } else {
                Point::new(center.x + radius, center.y)
            };
            list.extend(curvyo_render_core::build_measure_radius(center, rim, view));
            list.extend(curvyo_render_core::build_measure_dots(&[center], view));
            return list;
        }
        let Some(measuring) = self.measuring(pointer) else {
            return list;
        };
        if measuring.kind == DimensionKind::Angle {
            for far in &measuring.points[1..] {
                list.extend(curvyo_render_core::build_measure_line(
                    measuring.points[0],
                    *far,
                    view,
                ));
            }
        } else if let [a, b, ..] = measuring.points[..] {
            list.extend(curvyo_render_core::build_measure_line(a, b, view));
        }
        list.extend(curvyo_render_core::build_measure_dots(
            &measuring.points,
            view,
        ));
        list
    }

    /// The snap marker on the point feature under the pointer, or the hover band along the
    /// segment or curve a press would pick (UX notes §6).
    fn measure_marker_list(&self, view: ViewTransform) -> DrawList {
        match self.measure_hover() {
            Some(MeasurePick::Point { slot, at }) if slot.node().is_some() => {
                curvyo_render_core::build_snap_marker(&[], Some(at), view)
            }
            _ => curvyo_render_core::build_pick_band(&self.measure_hover_band(), view),
        }
    }

    /// The polyline of the segment or curve under the pointer; empty over a point or nothing.
    pub(super) fn measure_hover_band(&self) -> Vec<Point> {
        match self.measure_hover() {
            Some(MeasurePick::Segment { start, end }) => vec![start.1, end.1],
            Some(MeasurePick::Curve {
                center,
                radius,
                arc: (start, sweep),
                ..
            }) => {
                let (radius, start, sweep) =
                    (radius.as_mm(), start.as_radians(), sweep.as_radians());
                // invariant: at most 64 pieces.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let steps = ((sweep.abs() / 0.1).ceil() as usize).clamp(2, 64);
                (0..=steps)
                    .map(|i| {
                        #[allow(clippy::cast_precision_loss)] // at most 64 steps
                        let a = start + sweep * i as f64 / steps as f64;
                        Point::new(center.x + radius * a.cos(), center.y + radius * a.sin())
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// The Measure tool's readout chip, when a measurement is shown (criterion 1).
    fn measure_readout(&self) -> Option<ChipJson> {
        let pointer = self.pointer_position?;
        if self.tool != Tool::Measure || !self.measure_active() || self.measure_leaves_dimension() {
            return None;
        }
        let unit = self.document.display_unit();
        let measuring = self.measuring(pointer)?;
        let lines = match measuring.kind {
            DimensionKind::Radius | DimensionKind::Diameter => {
                let radius = measuring.points[0].vector_to(measuring.points[1]).length();
                curvyo_ui_core::radius_readout(curvyo_document_core::Length::from_mm(radius), unit)
                    .to_vec()
            }
            DimensionKind::Angle => {
                let value = curvyo_document_core::measure(
                    measuring.kind,
                    &measuring.points,
                    measuring.sector,
                )?;
                vec![curvyo_ui_core::dimension_text(
                    DimensionKind::Angle,
                    value,
                    unit,
                    VECTOR_DECIMALS,
                )]
            }
            _ => curvyo_ui_core::readout_lines(measuring.points[0], measuring.points[1], unit)
                .to_vec(),
        };
        let at = match measuring.kind {
            _ if self.measure_pinned() => measuring.points[1],
            _ => pointer,
        };
        let (x, y) = self.view().document_to_screen(at);
        Some(ChipJson { lines, x, y })
    }

    /// The mode label at the pointer once the Measure tool has a first pick (UX notes §4).
    fn measure_mode_chip(&self) -> Option<ChipJson> {
        // No chip before the first pick: only the crosshair (customer feedback on PR #243).
        if self.tool != Tool::Measure || !self.measure_active() {
            return None;
        }
        let pointer = self.pointer_position?;
        let dimension = self.measure_leaves_dimension();
        let kind = self.measuring(pointer).map(|m| m.kind);
        let (x, y) = self.view().document_to_screen(pointer);
        Some(ChipJson {
            lines: vec![curvyo_ui_core::measure_mode_label(kind, dimension)],
            x,
            y,
        })
    }

    /// The overlay of the last frame drawn: dimension texts, the readout and the mode label.
    #[must_use]
    pub fn measure_view_json(&self) -> String {
        let dimensions = self
            .dimension_labels
            .borrow()
            .iter()
            .map(|record| LabelJson {
                id: record
                    .id
                    .map(|id| serde_json::to_string(&id).unwrap_or_default())
                    .unwrap_or_default(),
                text: record.text.clone(),
                name: record.name.clone(),
                x: record.screen.0,
                y: record.screen.1,
                linked: record.linked,
                selected: record.selected,
                preview: record.id.is_none(),
            })
            .collect();
        let view = MeasureViewJson {
            dimensions,
            mode: self.measure_mode_chip(),
            readout: self.measure_readout(),
            speech: self.measure_speech().to_owned(),
        };
        serde_json::to_string(&view).unwrap_or_default()
    }

    /// `objects` without the selected dimensions, when one is selected: the selection box, the
    /// group box and the handles leave them out.
    pub(super) fn without_selected_dimensions<'a>(
        &self,
        objects: std::borrow::Cow<'a, [ObjectSnapshot]>,
    ) -> std::borrow::Cow<'a, [ObjectSnapshot]> {
        let any = self
            .dimension_labels
            .borrow()
            .iter()
            .any(|record| record.selected);
        if !any {
            return objects;
        }
        std::borrow::Cow::Owned(
            objects
                .iter()
                .filter(|o| !matches!(o, ObjectSnapshot::Dimension(_)))
                .cloned()
                .collect(),
        )
    }

    /// The dimension under the screen point of `point`, front-most first, in the Select tool.
    /// A pointer hit reads the dimensions of the last drawn frame, the ones the pointer is over;
    /// a document write lays them out afresh (`Self::dimension_records`).
    pub(super) fn dimension_at(&self, point: Point) -> Option<DimensionRecord> {
        let view = self.view();
        let (x, y) = view.document_to_screen(point);
        self.dimension_labels
            .borrow()
            .iter()
            .rev()
            .find(|record| record.id.is_some() && record.hit(x, y, view))
            .cloned()
    }
}
