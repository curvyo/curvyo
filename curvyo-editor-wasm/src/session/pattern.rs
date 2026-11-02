//! `Session`'s glue for the Pattern block of the Fill section (`specs/0071-pattern-fill` criteria
//! 1 to 4, 16, 18, 23, 24): what the block shows, the commands its controls issue, the over-limit
//! refusal, and Convert to paths. Every rule lives in `curvyo_ui_core` (`PatternField`,
//! `PatternCache`, `pattern_convert_plan`); this keeps the session's state and the commits.

use curvyo_document_core::{
    FillPaint, GeneratorKind, NodeId, ObjectSnapshot, PatternFill, PatternMode, PatternSeed,
    StyleEdit,
};
use curvyo_ui_core::{
    Grid, PatternError, PatternField, PatternFieldView, PatternState, object_pattern_pieces,
    pattern_convert_plan,
};
use serde::Serialize;

use super::Session;

/// The hard limit as the state line words it.
const LIMIT_TEXT: &str = "500,000";
/// The state line of a pattern over the work limit (acceptance report finding 2, UI review
/// 2026-11-02 item 4): it names no lever, as not every generator has spacing, size or lines.
const TOO_DETAILED_TEXT: &str =
    "Not generated: too detailed for this shape. Make the pattern coarser.";

/// The state line of a refusal.
fn refusal_text(error: PatternError) -> String {
    match error {
        PatternError::TooMany { estimate } => format!(
            "Not generated: {} pieces. The limit is {LIMIT_TEXT}.",
            grouped(estimate)
        ),
        PatternError::TooDetailed => TOO_DETAILED_TEXT.to_owned(),
    }
}

/// One value field of the block, as the host reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternFieldJson {
    name: &'static str,
    label: &'static str,
    text: String,
    value: f64,
    mixed: bool,
    bar: f64,
    bar_from: f64,
    zero: Option<f64>,
    unit: &'static str,
    unit_words: &'static str,
    value_min: f64,
    value_max: f64,
    default_text: String,
    resettable: bool,
}

impl From<PatternFieldView> for PatternFieldJson {
    fn from(view: PatternFieldView) -> Self {
        Self {
            name: view.name,
            label: view.label,
            text: view.text,
            value: view.value,
            mixed: view.mixed,
            bar: view.bar,
            bar_from: view.bar_from,
            zero: view.zero,
            unit: view.unit,
            unit_words: view.unit_words,
            value_min: view.value_min,
            value_max: view.value_max,
            default_text: view.default_text,
            resettable: view.resettable,
        }
    }
}

/// What the Pattern block and the fill kind group show.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // independent shown flags, read by name
pub struct PatternPanelJson {
    /// Some edited object can take a pattern: the Pattern item is in the kind group.
    pub(super) offered: bool,
    /// `"none"`, `"solid"`, `"pattern"` or `"mixed"`.
    pub(super) kind: String,
    /// Every edited object paints a pattern: the block is shown.
    pub(super) shown: bool,
    /// The generator's id, empty when they differ.
    pub(super) generator: String,
    /// The generator's fields (none while generators differ).
    pub(super) fields: Vec<PatternFieldJson>,
    /// The margin field.
    pub(super) margin: Option<PatternFieldJson>,
    /// The generator is seeded: the Seed row shows.
    pub(super) seeded: bool,
    /// The seed as text, empty when mixed.
    pub(super) seed: String,
    /// `"lines"`, `"shapes"` or `"mixed"`.
    pub(super) mode: String,
    /// The Whole pieces only row shows: the generator's pieces are shapes (criterion 29).
    pub(super) whole_offered: bool,
    /// `"on"`, `"off"` or `"mixed"`.
    pub(super) whole_pieces: String,
    /// The fill colour rows show (every edited pattern draws Shapes).
    pub(super) colour_rows: bool,
    /// The state lines.
    pub(super) state: Vec<String>,
    /// The first state line is the over-limit alert.
    pub(super) alert: bool,
    /// Convert to paths is in the tree.
    pub(super) convert: bool,
}

/// Formats `n` with a comma every three digits ("52,300").
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The kind of one object's fill.
fn kind_of(object: &ObjectSnapshot) -> &'static str {
    let fill = &object.style().fill;
    if !fill.enabled {
        "none"
    } else if fill.paint == FillPaint::Pattern {
        "pattern"
    } else {
        "solid"
    }
}

/// The one value all `items` share, or `None` when they differ (or there are none).
fn common<T: PartialEq + Clone>(items: impl IntoIterator<Item = T>) -> Option<T> {
    let mut items = items.into_iter();
    let first = items.next()?;
    items.all(|item| item == first).then_some(first)
}

/// Whether `object` can take a pattern.
const fn takes_pattern(object: &ObjectSnapshot) -> bool {
    matches!(
        object,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_)
    )
}

impl Session {
    /// The edited objects that can take a pattern, as shown (a panel drag previewed).
    fn pattern_objects(&self) -> Vec<ObjectSnapshot> {
        let ids = self.style_scope().ids;
        let mut shown: Vec<ObjectSnapshot> = self
            .objects()
            .iter()
            .filter(|object| ids.contains(&object.id()) && takes_pattern(object))
            .cloned()
            .collect();
        if self.style.is_active() {
            self.style.apply_to(&mut shown);
        }
        shown
    }

    /// The patterns of the edited objects when they all paint one.
    fn edited_patterns(&self) -> Vec<PatternFill> {
        self.pattern_objects()
            .iter()
            .filter_map(|object| object.style().fill.painted_pattern())
            .collect()
    }

    /// The generator the edited patterns share.
    fn edited_generator(&self) -> Option<GeneratorKind> {
        common(self.edited_patterns().iter().map(|p| p.generator.kind()))
    }

    /// What the Pattern block shows (`specs/0071-pattern-fill` UX notes §2 to §10).
    #[must_use]
    pub fn pattern_panel(&self) -> PatternPanelJson {
        let objects = self.pattern_objects();
        if objects.is_empty() {
            return PatternPanelJson::default();
        }
        let kind = common(objects.iter().map(kind_of)).unwrap_or("mixed");
        let mut panel = PatternPanelJson {
            offered: true,
            kind: kind.to_owned(),
            shown: kind == "pattern",
            ..PatternPanelJson::default()
        };
        if !panel.shown {
            return panel;
        }
        let patterns: Vec<PatternFill> = objects
            .iter()
            .filter_map(|o| o.style().fill.painted_pattern())
            .collect();
        let generator = common(patterns.iter().map(|p| p.generator.kind()));
        if let Some(generator) = generator {
            generator.id().clone_into(&mut panel.generator);
            panel.seeded = generator.is_seeded();
            panel.whole_offered = generator.has_whole_pieces();
            panel.fields = curvyo_document_core::pattern_params(generator)
                .iter()
                .map(|spec| {
                    let field = PatternField::Param(spec.param);
                    let values: Vec<f64> = patterns.iter().filter_map(|p| field.shown(p)).collect();
                    field.view(generator, &values).into()
                })
                .collect();
        }
        let margins: Vec<f64> = patterns.iter().map(|p| p.margin.as_mm()).collect();
        panel.margin = Some(
            PatternField::Margin
                .view(generator.unwrap_or(GeneratorKind::Hatch), &margins)
                .into(),
        );
        panel.seed = common(patterns.iter().map(|p| p.seed.get()))
            .map_or_else(String::new, |s| s.to_string());
        let mode = common(patterns.iter().map(|p| p.mode));
        match mode {
            Some(PatternMode::Lines) => "lines",
            Some(PatternMode::Shapes) => "shapes",
            None => "mixed",
        }
        .clone_into(&mut panel.mode);
        panel.colour_rows = mode == Some(PatternMode::Shapes);
        match common(patterns.iter().map(|p| p.whole_pieces)) {
            Some(true) => "on",
            Some(false) => "off",
            None => "mixed",
        }
        .clone_into(&mut panel.whole_pieces);
        panel.convert = self.pattern_state_lines(&objects, &mut panel);
        panel
    }

    /// The state lines: the over-limit alert, the count note, open shapes. Read from the memo, as
    /// is whether Convert to paths has anything to convert: a selected path or primitive with
    /// pieces (`review-architecture.md` change 2; a clone is styled by its source).
    fn pattern_state_lines(
        &self,
        objects: &[ObjectSnapshot],
        panel: &mut PatternPanelJson,
    ) -> bool {
        let mut cache = self.patterns.borrow_mut();
        let selected = &self.style_scope().ids;
        let (mut pieces, mut open, mut over) = (0_usize, 0_usize, 0_usize);
        let (mut detailed, mut convert) = (false, false);
        for object in objects {
            match cache.pieces_of(object, false).map(|(_, state)| state) {
                Some(PatternState::Pieces(n)) => {
                    pieces += n;
                    convert |= n > 0
                        && selected.contains(&object.id())
                        && matches!(
                            object,
                            ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)
                        );
                }
                Some(PatternState::Open) => open += 1,
                Some(PatternState::TooMany(estimate)) => over += estimate,
                Some(PatternState::TooDetailed) => detailed = true,
                None => {}
            }
        }
        if over > 0 {
            panel.alert = true;
            panel
                .state
                .push(refusal_text(PatternError::TooMany { estimate: over }));
        } else if detailed {
            panel.alert = true;
            panel.state.push(refusal_text(PatternError::TooDetailed));
        } else if pieces >= curvyo_ui_core::PATTERN_NOTE_PIECES {
            panel.state.push(format!("{} pieces", grouped(pieces)));
        }
        if open > 0 {
            panel.state.push(if open == objects.len() {
                "Open shapes show no pattern.".to_owned()
            } else if open == 1 {
                "1 open shape shows no pattern.".to_owned()
            } else {
                format!("{open} open shapes show no pattern.")
            });
        }
        convert
    }

    /// Why a pattern `edit` would not be generated on some edited object (over a limit).
    fn pattern_edit_over_limit(&self, ids: &[NodeId], edit: &StyleEdit) -> Option<PatternError> {
        if !edit.is_pattern_edit() {
            return None;
        }
        self.objects()
            .iter()
            .filter(|object| ids.contains(&object.id()))
            .find_map(|object| {
                let mut changed = object.clone();
                edit.apply_to_object(changed.style_mut(), object.id())
                    .ok()?;
                object_pattern_pieces(&changed)?.err()
            })
    }

    /// A pattern edit as one commit, refused over the hard limit (criterion 18): `false` when
    /// nothing was written.
    fn apply_pattern_edit(&mut self, edit: &StyleEdit) -> bool {
        let ids = self.style_scope().targets(&self.objects(), edit);
        if let Some(error) = self.pattern_edit_over_limit(&ids, edit) {
            self.pattern_refusal = Some(error);
            return false;
        }
        self.pattern_refusal = None;
        self.apply_style_edit(edit)
    }

    /// Before a panel drag's release: a previewed pattern edit over the hard limit is dropped, so
    /// the release writes nothing (criterion 18).
    pub(super) fn drop_pattern_preview_over_limit(&mut self) {
        let Some(edit) = self.style.pending_edit().cloned() else {
            return;
        };
        let ids = self.style_scope().targets(&self.objects(), &edit);
        if let Some(error) = self.pattern_edit_over_limit(&ids, &edit) {
            self.pattern_refusal = Some(error);
            self.style.cancel();
        }
    }

    /// The refusal of the last pattern commit over the limit, taken once by the host.
    pub fn take_pattern_refusal(&mut self) -> Option<String> {
        self.pattern_refusal.take().map(refusal_text)
    }

    /// The fill kind group: `"none"`, `"solid"` or `"pattern"`, one commit.
    pub fn set_fill_kind(&mut self, kind: &str) -> bool {
        let edit = match kind {
            "none" => StyleEdit::FillEnabled(false),
            "solid" => StyleEdit::FillPaint(FillPaint::Colour),
            "pattern" => StyleEdit::FillPaint(FillPaint::Pattern),
            _ => return false,
        };
        self.apply_pattern_edit(&edit)
    }

    /// A tile of the generator picker, one commit.
    pub fn set_pattern_generator(&mut self, id: &str) -> bool {
        GeneratorKind::from_id(id)
            .is_some_and(|kind| self.apply_pattern_edit(&StyleEdit::PatternGenerator(kind)))
    }

    /// Draw as Lines or Shapes, one commit.
    pub fn set_pattern_mode(&mut self, name: &str) -> bool {
        let mode = match name {
            "lines" => PatternMode::Lines,
            "shapes" => PatternMode::Shapes,
            _ => return false,
        };
        self.apply_pattern_edit(&StyleEdit::PatternMode(mode))
    }

    /// Whole pieces only on or off, one commit (criterion 29).
    pub fn set_pattern_whole_pieces(&mut self, on: bool) -> bool {
        self.apply_pattern_edit(&StyleEdit::PatternWholePieces(on))
    }

    /// Re-roll: each shape's next seed, one commit.
    pub fn reroll_pattern_seed(&mut self) -> bool {
        self.apply_pattern_edit(&StyleEdit::PatternReroll)
    }

    /// The typed seed: a whole number from 0 to 4294967295, one commit.
    ///
    /// # Errors
    /// `"seed"` for anything else; nothing is written.
    pub fn set_pattern_seed_text(&mut self, text: &str) -> Result<bool, &'static str> {
        let seed: u32 = text.trim().parse().map_err(|_| "seed")?;
        Ok(self.apply_pattern_edit(&StyleEdit::PatternSeed(PatternSeed::new(seed))))
    }

    fn pattern_field(&self, name: &str) -> Option<(PatternField, GeneratorKind)> {
        let kind = self.edited_generator().unwrap_or(GeneratorKind::Hatch);
        let field = PatternField::from_name(kind, name)?;
        if matches!(field, PatternField::Param(_)) && self.edited_generator().is_none() {
            return None;
        }
        Some((field, kind))
    }

    /// A tick of a field drag: position `p` on its scale, previewed without writing; the gesture
    /// ends with [`Session::commit_style_preview`] or [`Session::cancel_style_preview`].
    pub fn preview_pattern_field(&mut self, name: &str, p: f64, grid: Grid) {
        if let Some((field, kind)) = self.pattern_field(name) {
            let value = field.value_at(kind, p, grid);
            self.preview_style_edit(field.edit(kind, value));
        }
    }

    /// An arrow key on a field: `steps` steps from the shown value, previewed; the key-up commits.
    pub fn step_pattern_field(&mut self, name: &str, steps: i32, grid: Grid) {
        let Some((field, kind)) = self.pattern_field(name) else {
            return;
        };
        let Some(shown) = self.edited_patterns().first().and_then(|p| field.shown(p)) else {
            return;
        };
        self.preview_style_edit(field.edit(kind, field.step(kind, shown, steps, grid)));
    }

    /// The reset icon or `Ctrl+Backspace`: the field's default, one commit.
    pub fn reset_pattern_field(&mut self, name: &str) -> bool {
        self.pattern_field(name).is_some_and(|(field, kind)| {
            self.apply_pattern_edit(&field.edit(kind, field.default_value(kind)))
        })
    }

    /// Enter or Tab in a typed field: one commit, or the reason the text was refused.
    ///
    /// # Errors
    /// `"range"` or `"number"` from the field; for a Joint or Wall not smaller than its cell, the
    /// field it collides with ([`PatternField::misfit`]); nothing is written.
    pub fn set_pattern_field_text(&mut self, name: &str, text: &str) -> Result<bool, &'static str> {
        let Some((field, kind)) = self.pattern_field(name) else {
            return Ok(false);
        };
        let value = field.parse(kind, text)?;
        let edit = field.edit(kind, value);
        if let Some(pattern) = self
            .edited_patterns()
            .iter()
            .find(|pattern| edit.apply_to(&mut pattern_style(pattern)).is_err())
        {
            return Err(field.misfit(pattern));
        }
        Ok(self.apply_pattern_edit(&edit))
    }

    /// Convert to paths on the edited shapes: one commit; the new groups are selected after.
    pub fn convert_pattern_to_paths(&mut self) -> bool {
        self.recording(|this| -> bool {
            this.flush_style_preview();
            let ids = this.style_scope().ids;
            let objects = this.objects();
            let plan = pattern_convert_plan(&objects, &ids, &mut this.minter);
            if plan.is_empty() {
                return false;
            }
            let Ok(groups) = this.document.convert_patterns(&plan) else {
                return false;
            };
            this.node.clear_selection();
            // The selection holds leaves; a group is selected through its leaves, as after Group.
            let leaves: Vec<NodeId> = groups
                .iter()
                .flat_map(|group| this.document.leaves_of(*group))
                .collect();
            this.selection.set(&leaves);
            this.hovered_object = None;
            true
        })
    }
}

/// A style holding `pattern` as its painted pattern, to check an edit against.
fn pattern_style(pattern: &PatternFill) -> curvyo_document_core::Style {
    let mut style = curvyo_document_core::Style::default();
    style.fill.enabled = true;
    style.fill.paint = FillPaint::Pattern;
    style.fill.pattern = Some(Box::new(*pattern));
    style
}
