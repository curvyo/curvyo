//! Editing Vector dimensions with the Select tool (`specs/0070-measure-and-dimension` criteria 8
//! and 10): a press on a dimension selects it, a drag moves its text and line, and the Guide
//! button turns it into paths and a text.

use curvyo_document_core::{
    Affine, ConvertedPath, DimensionConversion, FontKind, Length, NewAnchor, Point, Style,
    TextAlign, TextLayoutValues, TextSnapshot, Vec2,
};

use super::dimension_view::{DimensionDrag, DimensionRecord};
use super::{Session, Tool};

/// The canvas label is 12px high; its capitals are about 70% of that.
const LABEL_CAP_PX: f64 = 8.5;
/// A press that moves less than this many screen pixels does not move the dimension.
const CLICK_PX: f64 = 4.0;

impl Session {
    /// A Select-tool press on a dimension's text or line: selects it (Shift adds) and starts a
    /// drag of its text. Returns whether a dimension took the press.
    pub(super) fn dimension_pointer_down(&mut self, point: Point, shift: bool) -> bool {
        if self.tool != Tool::Select {
            return false;
        }
        let Some(record) = self.dimension_at(point) else {
            return false;
        };
        let Some(id) = record.id else {
            return false;
        };
        let mut ids = if shift {
            self.selection.ids().to_vec()
        } else {
            Vec::new()
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
        self.selection.set(&ids);
        self.dimension_drag = Some(DimensionDrag {
            id,
            start: point,
            place: record.place,
            moved: false,
        });
        true
    }

    /// The drag follows the pointer: the text goes where the pointer is (criterion 8).
    pub(super) fn dimension_pointer_move(&mut self, point: Point) -> bool {
        let Some(mut drag) = self.dimension_drag else {
            return false;
        };
        let scale = self.view().scale();
        if !drag.moved && drag.start.vector_to(point).length() * scale <= CLICK_PX {
            return true;
        }
        drag.moved = true;
        let records = self.dimension_labels.borrow().clone();
        if let Some(record) = records.iter().find(|r| r.id == Some(drag.id)) {
            drag.place = curvyo_ui_core::place_at(record.kind, &record.points, drag.place, point);
        }
        self.dimension_drag = Some(drag);
        true
    }

    /// The release: one step "Move dimension" when the text moved.
    pub(super) fn dimension_pointer_up(&mut self, point: Point) -> bool {
        if self.dimension_drag.is_none() {
            return false;
        }
        self.dimension_pointer_move(point);
        let Some(drag) = self.dimension_drag.take() else {
            return false;
        };
        if drag.moved {
            let _ = self.recording(|this| this.document.set_dimension_place(drag.id, drag.place));
        }
        true
    }

    /// Whether a dimension text drag is in flight.
    pub(super) fn dimension_drag_in_flight(&self) -> bool {
        self.dimension_drag.is_some()
    }

    /// Escape during a dimension drag: it ends without writing.
    pub(super) fn cancel_dimension_drag(&mut self) -> bool {
        self.dimension_drag.take().is_some()
    }

    /// What the selected dimensions become, laid out for the view at this moment from the
    /// document as it is now, never from the last drawn frame (criterion 10).
    fn dimension_conversions(&mut self) -> Vec<DimensionConversion> {
        let px = 1.0 / self.view().scale();
        let font = self.next_text_font();
        let selected = self.selection.ids().to_vec();
        let read = self.document_read();
        let records: Vec<DimensionRecord> = self
            .dimension_records(&read.leaves)
            .into_iter()
            .filter(|r| r.id.is_some_and(|id| selected.contains(&id)))
            .collect();
        let (stroke, fill) = converted_styles(px);
        let mut out = Vec::new();
        for record in records {
            let Some(id) = record.id else {
                continue;
            };
            out.push(DimensionConversion {
                id,
                paths: self.converted_paths(&record, px, (&stroke, &fill)),
                text: font
                    .clone()
                    .map(|font| converted_text(id, &record, font, (px, &fill))),
            });
        }
        out
    }

    /// The lines and arrowheads of `record` as plain paths, as the canvas draws them.
    fn converted_paths(
        &mut self,
        record: &DimensionRecord,
        px: f64,
        (stroke, fill): (&Style, &Style),
    ) -> Vec<ConvertedPath> {
        let mut corner = |p: Point| NewAnchor::corner(self.minter.mint(), p);
        let mut paths: Vec<ConvertedPath> = record
            .marks
            .lines
            .iter()
            .map(|&(a, b)| ConvertedPath {
                anchors: vec![corner(a), corner(b)],
                closed: false,
                style: stroke.clone(),
            })
            .collect();
        for arrow in &record.marks.arrows {
            let outline = curvyo_render_core::dimension_arrow_outline(arrow.tip, arrow.back, px);
            paths.push(ConvertedPath {
                anchors: outline.into_iter().map(&mut corner).collect(),
                closed: true,
                style: fill.clone(),
            });
        }
        paths
    }

    /// The Guide button on a selection with dimensions: converts them (and turns the other
    /// guides back) as one step "Convert to path", and selects what they became (criterion 10).
    /// Returns whether it acted.
    pub(super) fn convert_selected_dimensions(
        &mut self,
        others: &[curvyo_document_core::NodeId],
    ) -> bool {
        let conversions = self.dimension_conversions();
        if conversions.is_empty() {
            return false;
        }
        let made = self.recording(|this| {
            this.document.batch(
                curvyo_document_core::CONVERT_DIMENSION_COMMIT_LABEL,
                |document| {
                    if !others.is_empty() {
                        let _ = document.set_guide(others, false);
                    }
                    document.convert_dimensions(&conversions)
                },
            )
        });
        match made {
            Ok(groups) => {
                let mut ids = others.to_vec();
                ids.extend(groups);
                self.selection.set(&ids);
                true
            }
            Err(_) => false,
        }
    }
}

/// The guide-green stroke of a converted line (one screen pixel wide) and the fill of a
/// converted arrowhead or text.
fn converted_styles(px: f64) -> (Style, Style) {
    let guide = curvyo_render_core::GUIDE;
    let guide = curvyo_document_core::Color {
        r: guide.r,
        g: guide.g,
        b: guide.b,
    };
    let mut stroke = Style::default();
    stroke.stroke.color = guide;
    stroke.stroke.width = Length::from_mm(px);
    stroke.fill.enabled = false;
    let mut fill = Style::default();
    fill.stroke.enabled = false;
    fill.fill.enabled = true;
    fill.fill.color = guide;
    (stroke, fill)
}

/// The value text of the dimension `id` laid out as `record`, as an editable outline text in
/// `font`, centred where the canvas label sits.
fn converted_text(
    id: curvyo_document_core::NodeId,
    record: &DimensionRecord,
    font: curvyo_document_core::FontRef,
    (px, fill): (f64, &Style),
) -> TextSnapshot {
    let cap = LABEL_CAP_PX * px;
    let center = record.marks.text;
    TextSnapshot {
        id,
        string: record.text.clone(),
        font: curvyo_document_core::FontRef {
            kind: FontKind::Outline,
            ..font
        },
        size: Length::from_mm(cap),
        layout: TextLayoutValues {
            align: TextAlign::Centre,
            ..TextLayoutValues::DEFAULT
        },
        chars: curvyo_document_core::TextChars::default(),
        placement: Affine::translation(Vec2::new(center.x, center.y + cap / 2.0)),
        style: fill.clone(),
        outline: None,
        on_path: None,
        source: None,
    }
}
