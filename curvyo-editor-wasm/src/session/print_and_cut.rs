//! The Print and cut group of a laser page in the session (`specs/0121-print-and-cut`
//! criteria 7 to 20, ADR 0081 §4 to §8): which placed sheets have marks, the reading of the
//! chosen sheet as the group's JSON, and its commands, one undo step each.

use curvyo_document_core::{MarksObject, NodeId, Page, PlacedPage, Point, PrintAndCutRegister};
use curvyo_laser_core::job::{Frame, Start, read_output_settings, um_of_mm};
use curvyo_laser_core::print_and_cut::text;
use curvyo_laser_core::print_and_cut::{
    FitState, PrintAndCut, RowKind, SheetInput, head_position_um, read_print_and_cut,
};
use serde_json::{Value, json};

use super::Session;

/// The session's Print and cut facts: the sheet the picker chose (view state, not stored).
#[derive(Debug, Default)]
pub(super) struct PrintAndCutState {
    sheet: Option<NodeId>,
}

/// One placed sheet with marks: the placement, its drawing page and its marks objects in tree
/// order (the first is used, criterion 6).
pub(super) struct Sheet {
    pub(super) placed: PlacedPage,
    pub(super) source: Page,
    pub(super) objects: Vec<MarksObject>,
}

impl Sheet {
    /// The first marks object's marks as number and centre in sheet page-local mm.
    pub(super) fn marks(&self) -> Vec<(u8, Point)> {
        self.objects.first().map_or_else(Vec::new, |object| {
            object
                .marks
                .iter()
                .map(|m| {
                    (
                        m.number,
                        Point::new(
                            m.centre.x - self.source.origin.x,
                            m.centre.y - self.source.origin.y,
                        ),
                    )
                })
                .collect()
        })
    }
}

impl Session {
    /// Every placed sheet with a marks object on the laser page `manufacturing`, in tree order.
    pub(super) fn print_and_cut_sheets(&self, manufacturing: NodeId) -> Vec<Sheet> {
        self.document
            .placed_pages()
            .into_iter()
            .filter(|p| p.output.kind.manufacturing() == Some(manufacturing))
            .filter_map(|placed| {
                let source = placed.source.clone()?;
                let objects = self.document.marks_objects_on(&source);
                (!objects.is_empty()).then_some(Sheet {
                    placed,
                    source,
                    objects,
                })
            })
            .collect()
    }

    /// The sheet the group shows: the picker's, else the selected placement, else the first.
    fn chosen_sheet(&self) -> Option<(Sheet, usize)> {
        let manufacturing = self.active_manufacturing()?;
        let sheets = self.print_and_cut_sheets(manufacturing.id);
        let wanted = self
            .print_and_cut
            .sheet
            .filter(|id| sheets.iter().any(|s| s.placed.id == *id))
            .or_else(|| self.selected_placement())
            .filter(|id| sheets.iter().any(|s| s.placed.id == *id));
        let count = sheets.len();
        let sheet = match wanted {
            Some(id) => sheets.into_iter().find(|s| s.placed.id == id),
            None => sheets.into_iter().next(),
        }?;
        Some((sheet, count))
    }

    /// The placement the group shows now, for the canvas overlay.
    pub(super) fn print_and_cut_view_sheet(&self) -> Option<NodeId> {
        self.chosen_sheet().map(|(sheet, _)| sheet.placed.id)
    }

    /// The frame and Start of the active laser page.
    pub(super) fn print_and_cut_frame(&self) -> Option<(Frame, Start)> {
        let manufacturing = self.active_manufacturing()?;
        let page = manufacturing.page.as_ref()?;
        let (corner, size) = self.active_copy().map_or(
            (curvyo_laser_core::OriginCorner::default(), page.size),
            |copy| (copy.read.copy.values.origin, copy.page),
        );
        let frame = Frame {
            corner,
            width: um_of_mm(size.width.as_mm()),
            height: um_of_mm(size.height.as_mm()),
        };
        Some((frame, read_output_settings(&manufacturing).start))
    }

    /// The reading of `sheet` on the active laser page.
    pub(super) fn read_sheet(&self, sheet: &Sheet) -> Option<PrintAndCut> {
        let (frame, start) = self.print_and_cut_frame()?;
        let register = self
            .document
            .print_and_cut(sheet.placed.id)
            .unwrap_or(PrintAndCutRegister::Absent);
        let marks = sheet.marks();
        Some(read_print_and_cut(&SheetInput {
            marks: &marks,
            register: &register,
            frame,
            matrix: sheet.placed.matrix,
            sheet: (
                sheet.source.size.width.as_mm(),
                sheet.source.size.height.as_mm(),
            ),
            start,
        }))
    }

    /// The Print and cut group of the active laser page as JSON, or `None` when no placed sheet
    /// there has marks (criterion 7: the group is absent).
    #[must_use]
    pub fn print_and_cut_view(&self) -> Option<String> {
        let manufacturing = self.active_manufacturing()?;
        let (sheet, count) = self.chosen_sheet()?;
        let reading = self.read_sheet(&sheet)?;
        let sheets: Vec<Value> = self
            .print_and_cut_sheets(manufacturing.id)
            .iter()
            .map(|s| {
                json!({
                    "id": s.placed.id,
                    "name": s.source.name,
                    "marks": s.objects[0].marks.len(),
                })
            })
            .collect();
        Some(view_json(&sheet, count, &sheets, &reading).to_string())
    }

    /// Chooses the sheet of the group (the Sheet picker, criterion 7).
    pub fn print_and_cut_choose_sheet(&mut self, placement: NodeId) {
        self.print_and_cut.sheet = Some(placement);
    }

    fn with_chosen_sheet<R: Default>(
        &mut self,
        body: impl FnOnce(&mut Self, &Sheet, &PrintAndCut) -> R,
    ) -> R {
        let Some((sheet, _)) = self.chosen_sheet() else {
            return R::default();
        };
        let Some(reading) = self.read_sheet(&sheet) else {
            return R::default();
        };
        self.recording(|this| body(this, &sheet, &reading))
    }

    /// Sets mark `number`'s measured position in machine mm (a typed row), rounded to µm, one
    /// step (criterion 9). `false` when out of range.
    pub fn print_and_cut_measure(&mut self, number: u8, x_mm: f64, y_mm: f64) -> bool {
        if !x_mm.is_finite() || !y_mm.is_finite() {
            return false;
        }
        let position = head_position_um(x_mm, y_mm);
        self.with_chosen_sheet(|this, sheet, _| {
            this.document
                .set_print_and_cut_measure(sheet.placed.id, number, Some(position))
                .is_ok()
        })
    }

    /// Take head position (criterion 8): the connector's last work position in mm, as a measure.
    /// The host passes the position it holds; nothing is sent to the machine (criterion 20).
    pub fn print_and_cut_take_head_position(&mut self, number: u8, x_mm: f64, y_mm: f64) -> bool {
        self.print_and_cut_measure(number, x_mm, y_mm)
    }

    /// Clear on a row (criterion 9), one step.
    pub fn print_and_cut_clear(&mut self, number: u8) -> bool {
        self.with_chosen_sheet(|this, sheet, _| {
            this.document
                .set_print_and_cut_measure(sheet.placed.id, number, None)
                .is_ok()
        })
    }

    /// The Tolerance field in mm (criterion 13), one step.
    pub fn print_and_cut_tolerance(&mut self, mm: f64) -> bool {
        if !mm.is_finite() {
            return false;
        }
        let um = um_of_mm(mm);
        self.with_chosen_sheet(|this, sheet, _| {
            this.document
                .set_print_and_cut_tolerance(sheet.placed.id, um)
                .is_ok()
        })
    }

    /// Apply (criterion 16): the sheet's placement to the fit, one step. The refusal text, or an
    /// empty string on success or when Apply is not offered.
    pub fn print_and_cut_apply(&mut self) -> String {
        self.with_chosen_sheet(|this, sheet, reading| {
            let FitState::Fit(fit) = &reading.fit else {
                return String::new();
            };
            if text::apply_button(reading).is_none_or(|(_, reason)| reason.is_some()) {
                return String::new();
            }
            match this.document.apply_print_and_cut(sheet.placed.id, fit.map) {
                Err(curvyo_document_core::ManufacturingError::OffPage) => text::OFF_PAGE.to_owned(),
                Ok(()) | Err(_) => String::new(),
            }
        })
    }

    /// Reset placement (criterion 16), one step; only while offered.
    pub fn print_and_cut_reset(&mut self) -> bool {
        self.with_chosen_sheet(|this, sheet, reading| {
            reading.reset_offered && this.document.reset_print_and_cut(sheet.placed.id).is_ok()
        })
    }
}

/// A measured value in mm with two decimals.
fn mm_text(um: i64) -> String {
    #[allow(clippy::cast_precision_loss)]
    text::fixed(um as f64 / 1000.0, 2)
}

/// The rows of the group.
fn rows_json(reading: &PrintAndCut) -> Vec<Value> {
    reading
        .rows
        .iter()
        .map(|row| {
            json!({
                "number": row.number,
                "x": row.measured.map(|[x, _]| mm_text(x)),
                "y": row.measured.map(|[_, y]| mm_text(y)),
                "kind": match row.kind {
                    RowKind::Mark => "mark",
                    RowKind::Gone => "gone",
                    RowKind::Twice => "twice",
                },
                "note": text::row_note(row),
            })
        })
        .collect()
}

/// The fit block, `null` without a fit.
fn fit_json(reading: &PrintAndCut) -> Value {
    let FitState::Fit(fit) = &reading.fit else {
        return Value::Null;
    };
    let (first, second) = text::error_lines(reading).unwrap_or_default();
    json!({
        "moved": text::pair_mm_text(fit.moved_mm[0], fit.moved_mm[1]),
        "turned": text::degrees_text(fit.turned_deg),
        "scale": text::percent_text(fit.scale_pct),
        "error": first,
        "error_detail": second,
        "warn": fit.over_tolerance && fit.pairs > 2,
    })
}

fn view_json(sheet: &Sheet, count: usize, sheets: &[Value], reading: &PrintAndCut) -> Value {
    let measured = reading
        .rows
        .iter()
        .filter(|r| r.kind == RowKind::Mark && r.measured.is_some())
        .count();
    let apply = text::apply_button(reading)
        .map(|(label, reason)| json!({"label": label, "reason": reason}));
    let object_count = sheet.objects.first().map_or(0, |o| o.marks.len());
    #[allow(clippy::cast_precision_loss)]
    let tolerance_mm = reading.tolerance_um as f64 / 1000.0;
    json!({
        "sheet": sheet.placed.id,
        "sheet_name": sheet.source.name,
        "not_cut": text::NOT_CUT,
        "sheets": if count > 1 { sheets.to_vec() } else { Vec::new() },
        "marks_line": format!("Registration marks, {object_count} marks"),
        "second_line": (sheet.objects.len() > 1).then(|| format!("Registration marks 2: {}", text::NOT_USED)),
        "unreadable": reading.unreadable.then_some(text::UNREADABLE),
        "summary": text::header_summary(reading),
        "warn_summary": matches!(&reading.fit, FitState::Fit(f) if f.over_tolerance && f.pairs > 2),
        "rows": rows_json(reading),
        "state": text::state_line(reading),
        "fit": fit_json(reading),
        "tolerance": tolerance_mm,
        "show_tolerance": measured >= 2,
        "warnings": reading.warnings.iter().map(text::warning_text).collect::<Vec<_>>(),
        "apply": apply,
        "not_usable": text::not_usable(reading),
        "reset": {
            "enabled": reading.reset_offered,
            "reason": (!reading.reset_offered).then_some(text::NOTHING_TO_RESET),
        },
        "placement_line": reading.on_fit.map(|on| if on { text::ON_FIT } else { text::DIFFERS }),
    })
}

#[cfg(test)]
#[path = "print_and_cut_tests.rs"]
mod tests;
