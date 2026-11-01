//! Writes an align, distribute or grid plan as one commit.
//!
//! `specs/0061-align-and-distribute/` criteria 3, 10 to 17: the reference
//! rule by the selected count, the units' true boxes, the per-leaf results
//! and the one commit through the group move writer.

use curvyo_document_core::{Document, Length, NodeId, ObjectSnapshot, Vec2};

use crate::align_labels::distribute_commit_label;
use crate::align_plan::{
    AlignAxis, AlignBox, AlignCommand, DistributeMode, align_translations,
    distribute_on_frame_translations, distribute_translations,
};
use crate::group_transform::commit_group;
use crate::object_bounds::object_outline_bounds;
use crate::object_selection::ObjectSelection;
use crate::transform_drag::StrokeScaling;
use crate::units::{SelectedUnit, Units};

/// What an align or distribute press acts on and relative to what
/// (criteria 3, 14 to 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignReference {
    /// Nothing selected: the active page's visible, unlocked objects,
    /// relative to the page (criterion 15).
    PageObjects,
    /// The selected objects, relative to the active page (one object for
    /// align, up to two for distribute; criteria 14, 16).
    Page,
    /// The selected objects, relative to the key object or the selection box
    /// (criteria 3, 6).
    Selection,
}

impl AlignReference {
    /// Whether the press is relative to the active page.
    #[must_use]
    pub const fn on_page(self) -> bool {
        !matches!(self, Self::Selection)
    }
}

/// The reference of an align press with `selected` units: the page's
/// objects with none, the page with one, else the selection (criteria 3,
/// 14, 15).
#[must_use]
pub const fn align_reference(selected: usize) -> AlignReference {
    match selected {
        0 => AlignReference::PageObjects,
        1 => AlignReference::Page,
        _ => AlignReference::Selection,
    }
}

/// The reference of a distribute press with `selected` units: the page's
/// objects with none, the page with one or two, else the selection
/// (criteria 6, 15, 16).
#[must_use]
pub const fn distribute_reference(selected: usize) -> AlignReference {
    match selected {
        0 => AlignReference::PageObjects,
        1 | 2 => AlignReference::Page,
        _ => AlignReference::Selection,
    }
}

/// What the align and distribute commands did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignOutcome {
    /// One commit was written.
    Aligned,
    /// Every translation is zero: nothing was written (criteria 3 and 7).
    Unchanged,
    /// Nothing to act on: nothing was written.
    NothingSelected,
    /// A grid with fewer cells than objects (criterion 19): nothing was
    /// written.
    TooFewCells,
}

/// The one read the commands work from: the document to write and the
/// objects of the current context (the leaves the selection names).
pub struct AlignInput<'a> {
    /// The document the commit is written to.
    pub document: &'a Document,
    /// The objects of the context.
    pub objects: &'a [ObjectSnapshot],
    /// The grouping of the context's leaves.
    pub units: &'a Units,
    /// The current selection.
    pub selection: &'a ObjectSelection,
}

/// Every `unit` with its true bounding box, in its order: the union of its
/// leaves' outline bounds — the same box the selection draws (criterion 9).
/// A unit with no leaf in `objects` is left out together with its box, so a
/// plan over the boxes always lines up with its units.
#[must_use]
pub fn unit_boxes(objects: &[ObjectSnapshot], units: &[SelectedUnit]) -> BoxedUnits {
    let mut found = BoxedUnits::default();
    for unit in units {
        let boxes: Vec<AlignBox> = unit
            .leaves
            .iter()
            .filter_map(|leaf| objects.iter().find(|object| object.id() == *leaf))
            .map(|object| {
                let (min, max) = object_outline_bounds(object);
                AlignBox::new(min, max)
            })
            .collect();
        if let Some(union) = AlignBox::union(&boxes) {
            found.units.push(unit.clone());
            found.boxes.push(union);
        }
    }
    found
}

/// Units and their true boxes, index for index (the result of
/// [`unit_boxes`]).
#[derive(Debug, Clone, Default)]
pub struct BoxedUnits {
    /// The units that have at least one leaf in the objects read.
    pub units: Vec<SelectedUnit>,
    /// The true box of `units[i]` at `i`.
    pub boxes: Vec<AlignBox>,
}

/// The leaves of the context whose true box overlaps `page` (the active
/// page's rectangle, edges included): the objects a page-wise align or
/// distribute acts on when nothing is selected (the customer's extension of
/// 2026-10-30). Hidden and locked objects are not in `objects` (the pickable
/// read), so they never move and never count.
#[must_use]
pub fn page_object_ids(objects: &[ObjectSnapshot], page: &AlignBox) -> Vec<NodeId> {
    objects
        .iter()
        .filter(|object| {
            let (min, max) = object_outline_bounds(object);
            let box_ = AlignBox::new(min, max);
            box_.start(AlignAxis::Horizontal) <= page.end(AlignAxis::Horizontal)
                && box_.end(AlignAxis::Horizontal) >= page.start(AlignAxis::Horizontal)
                && box_.start(AlignAxis::Vertical) <= page.end(AlignAxis::Vertical)
                && box_.end(AlignAxis::Vertical) >= page.start(AlignAxis::Vertical)
        })
        .map(ObjectSnapshot::id)
        .collect()
}

/// The per-unit translation results of `plan`: every leaf of a moved unit is
/// translated by its unit's delta, so a group moves as one whole (criterion 2).
/// A linked clone's own matrix takes only the part its link does not already
/// carry (criterion 11): a clone whose original moves in the same commit
/// rides along, a clone with own placement (`0057`) never does.
fn translated_results(
    objects: &[ObjectSnapshot],
    units: &[SelectedUnit],
    plan: &[Vec2],
) -> Vec<ObjectSnapshot> {
    let deltas: std::collections::HashMap<NodeId, Vec2> = units
        .iter()
        .zip(plan)
        .filter(|(_, delta)| **delta != Vec2::ZERO)
        .map(|(unit, delta)| (unit.id, *delta))
        .collect();
    units
        .iter()
        .zip(plan)
        .filter(|(_, delta)| **delta != Vec2::ZERO)
        .flat_map(|(unit, delta)| {
            let deltas = &deltas;
            unit.leaves.iter().filter_map(move |leaf| {
                let object = objects.iter().find(|o| o.id() == *leaf)?;
                let offset = match object {
                    ObjectSnapshot::Clone(clone) => {
                        let carried = if clone.chain.own_placement {
                            Vec2::ZERO
                        } else {
                            deltas
                                .get(&clone.chain.origin)
                                .copied()
                                .unwrap_or(Vec2::ZERO)
                        };
                        *delta - carried
                    }
                    _ => *delta,
                };
                (offset != Vec2::ZERO).then(|| object.translated(offset))
            })
        })
        .collect()
}

/// Writes `plan` for `units` as one commit named `label`, through the same
/// writer as a group move. `Unchanged` when every translation is zero.
pub(crate) fn commit_plan(
    document: &Document,
    objects: &[ObjectSnapshot],
    units: &[SelectedUnit],
    plan: &[Vec2],
    label: &'static str,
) -> AlignOutcome {
    let results = translated_results(objects, units, plan);
    if results.is_empty() {
        return AlignOutcome::Unchanged;
    }
    if commit_group(document, &results, StrokeScaling::Keep, label, None) {
        AlignOutcome::Aligned
    } else {
        AlignOutcome::NothingSelected
    }
}

/// Aligns the selected units of `input`'s selection by `command` onto the
/// key unit's box when it is still selected, else onto the selection's union
/// box, as one commit (criteria 2, 3, 12). A linked clone moves through its
/// own placement matrix (criterion 11). The caller routes fewer than two
/// units to [`align_page`] ([`align_reference`]).
#[must_use]
pub fn align_selection(
    input: &AlignInput,
    key: Option<NodeId>,
    command: AlignCommand,
) -> AlignOutcome {
    let selected = input.units.selected(input.selection.ids());
    let BoxedUnits { units, boxes } = unit_boxes(input.objects, &selected);
    let key_box = key.and_then(|id| {
        units
            .iter()
            .position(|unit| unit.id == id)
            .map(|at| boxes[at])
    });
    let Some(reference) = key_box.or_else(|| AlignBox::union(&boxes)) else {
        return AlignOutcome::NothingSelected;
    };
    let plan = align_translations(&boxes, &reference, command);
    commit_plan(
        input.document,
        input.objects,
        &units,
        &plan,
        command.commit_label(false),
    )
}

/// Aligns every unit of `units` onto the active `page` — the page is the
/// reference whatever the count (criteria 14, 15).
#[must_use]
pub fn align_page(
    input: &AlignInput,
    units: &[SelectedUnit],
    page: AlignBox,
    command: AlignCommand,
) -> AlignOutcome {
    let BoxedUnits { units, boxes } = unit_boxes(input.objects, units);
    if boxes.is_empty() {
        return AlignOutcome::NothingSelected;
    }
    let plan = align_translations(&boxes, &page, command);
    commit_plan(
        input.document,
        input.objects,
        &units,
        &plan,
        command.commit_label(true),
    )
}

/// Distributes the selected units of `input`'s selection on `axis` by `mode`
/// and the typed `gap`, as one commit (criteria 6 to 8, 12). Fewer than three
/// units change nothing (criterion 7), two with a typed gap are enough (the
/// amendment of 2026-10-30).
#[must_use]
pub fn distribute_selection(
    input: &AlignInput,
    axis: AlignAxis,
    mode: DistributeMode,
    gap: Option<Length>,
) -> AlignOutcome {
    let selected = input.units.selected(input.selection.ids());
    let BoxedUnits { units, boxes } = unit_boxes(input.objects, &selected);
    if boxes.is_empty() {
        return AlignOutcome::NothingSelected;
    }
    let plan = distribute_translations(&boxes, axis, mode, gap);
    let label = distribute_commit_label(axis, false);
    commit_plan(input.document, input.objects, &units, &plan, label)
}

/// Distributes every unit of `units` evenly across the active `page`
/// (criterion 16): n + 1 equal gaps, or the centres cutting the page into
/// n + 1 equal parts; a typed `gap` spaces from the page's start edge.
#[must_use]
pub fn distribute_page(
    input: &AlignInput,
    units: &[SelectedUnit],
    page: AlignBox,
    axis: AlignAxis,
    mode: DistributeMode,
    gap: Option<Length>,
) -> AlignOutcome {
    let BoxedUnits { units, boxes } = unit_boxes(input.objects, units);
    if boxes.is_empty() {
        return AlignOutcome::NothingSelected;
    }
    let plan = distribute_on_frame_translations(&boxes, &page, axis, mode, gap);
    let label = distribute_commit_label(axis, true);
    commit_plan(input.document, input.objects, &units, &plan, label)
}
