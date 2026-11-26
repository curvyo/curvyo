//! A library item put into a slot as a fitted copy (`specs/0154-library-into-slots` criteria 2 to
//! 8, ADR 0088 decisions 2, 3 and 5): one slot, the replace of a filled slot, or every empty slot
//! of a holder, each one undo step, the copies selected.

use curvyo_document_core::{
    AnchorId, Angle, CameFrom, Fragment, FragmentReport, FragmentTarget, NodeId, Point, Vec2,
};
use curvyo_laser_core::{FitMode, SlotFit, slot_fit_matrix};
use curvyo_ui_core::library_index_text::clones_became_paths_text;
use curvyo_ui_core::slot_drop_text::{
    fill_notice, fit_notice, no_empty_slot_text, slot_filled_text,
};

use super::super::library_import::joint_box;
use super::super::{Session, Tool};
use super::into_slot::{SlotTarget, UnitBox};

/// The commit label of a copy put into a slot ("Put into slot").
pub(in crate::session) const PUT_INTO_SLOT_LABEL: &str = "put_into_slot";
/// The commit label of a replace of a slot's content ("Replace slot content").
pub(in crate::session) const REPLACE_SLOT_CONTENT_LABEL: &str = "replace_slot_content";
/// The commit label of a fill of every empty slot of a holder ("Fill slots").
pub(in crate::session) const FILL_SLOTS_LABEL: &str = "fill_slots";

/// Two fits whose scale and angle differ by less than this share one mapped fragment.
const SAME_FIT: f64 = 1e-9;

/// What goes into a slot: a page (all its members) or picked elements of a read-only source.
#[derive(Clone, Copy)]
pub struct SlotSource<'a> {
    /// The read-only library session.
    pub source: &'a Session,
    /// The picked top-level objects, when no page is given.
    pub roots: &'a [NodeId],
    /// The whole page, when given.
    pub page: Option<&'a str>,
    /// The Came from record of the copies.
    pub came_from: &'a CameFrom,
}

/// The unit a source makes: its roots, its box and the name of its wrapping group.
pub(super) struct Unit {
    roots: Vec<NodeId>,
    pub(super) box_: UnitBox,
    name: String,
}

/// What a put or a fill did, for the page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SlotDrop {
    /// The refusal ("Slot 2 is filled.", "No empty slot in Holder 1."): nothing was written.
    pub refused: Option<String>,
    /// The notice to show after a write, when the result is not a plain centre.
    pub notice: Option<String>,
}

impl SlotSource<'_> {
    /// The box of the unit this source makes, for the drag cue and the Into slot list.
    #[must_use]
    pub fn unit_box(&self) -> Option<UnitBox> {
        self.unit().map(|unit| unit.box_)
    }

    /// The unit's box and its roots in the source.
    pub(super) fn unit_parts(&self) -> Option<(UnitBox, Vec<NodeId>)> {
        self.unit().map(|unit| (unit.box_, unit.roots))
    }

    /// The unit: a page's members with the page frame as the box, or the picks with their joint
    /// box; named after the source page. `None` when there is nothing to put.
    pub(super) fn unit(&self) -> Option<Unit> {
        let pages = self.source.document.pages();
        let page_named = |id: &str| pages.iter().find(|p| p.id == id);
        let (roots, box_, name) = if let Some(id) = self.page {
            let page = page_named(id)?;
            let min = page.origin;
            let max = Point::new(
                min.x + page.size.width.as_mm(),
                min.y + page.size.height.as_mm(),
            );
            let roots = self.source.page_top_level(id);
            (roots, UnitBox::between(min, max), page.name.clone())
        } else {
            let (min, max) = joint_box(&self.source.document, self.roots);
            let name = page_named(&self.came_from.page_id)
                .map_or_else(|| self.came_from.page.clone(), |p| p.name.clone());
            (self.roots.to_vec(), UnitBox::between(min, max), name)
        };
        (!roots.is_empty()).then_some(Unit { roots, box_, name })
    }

    /// The unit's private copy with the linear part of `fit` in `target` applied about its centre.
    fn mapped(&self, unit: &Unit, fit: SlotFit, target: &SlotTarget) -> Result<Fragment, String> {
        let fragment = self
            .source
            .document
            .extract_fragment_placing(&unit.roots, None, &self.source.library_placed_members())
            .map_err(|error| error.to_string())?;
        if curvyo_ui_core::map_uniform(
            fragment.source(),
            &fragment.roots(),
            unit.box_.centre,
            fit.scale,
            fit_angle(fit, target),
        ) {
            Ok(fragment)
        } else {
            Err("the piece cannot be scaled to fit".to_owned())
        }
    }
}

/// The turn of the unit in document space: the fit's quarter turn plus the board's own turn.
fn fit_angle(fit: SlotFit, target: &SlotTarget) -> Angle {
    let turn = if fit.turned {
        std::f64::consts::FRAC_PI_2
    } else {
        0.0
    };
    Angle::from_radians(turn + target.board.b.atan2(target.board.a))
}

/// The move that takes the mapped unit's centre onto the slot's centre.
fn fit_offset(unit: &Unit, fit: SlotFit, target: &SlotTarget) -> Vec2 {
    let matrix = slot_fit_matrix(unit.box_.centre, fit, target.slot.centre, target.board);
    let at = matrix.apply_point(unit.box_.centre);
    Vec2::new(at.x - unit.box_.centre.x, at.y - unit.box_.centre.y)
}

/// One mapped fragment per distinct fit (ADR 0088 decision 3).
struct Mapped {
    scale: f64,
    angle: Angle,
    fragment: Fragment,
}

impl Mapped {
    /// Whether this fragment serves a fit of `scale` and `angle`.
    fn serves(&self, scale: f64, angle: Angle) -> bool {
        (self.scale - scale).abs() < SAME_FIT
            && (self.angle.as_radians() - angle.as_radians()).abs() < SAME_FIT
    }
}

/// What a fill writes: one mapped fragment per distinct fit, one copy per empty slot, and the
/// scale of each copy for the notice.
struct FillPlan {
    mapped: Vec<Mapped>,
    copies: Vec<Copy>,
    scales: Vec<f64>,
}

/// One copy to insert: the index of its fragment, its move and its fresh anchor ids.
struct Copy {
    fragment: usize,
    offset: Vec2,
    anchors: Vec<AnchorId>,
}

impl Session {
    /// Puts `from` into the slot `slot` of the active laser page, fitted by `mode`, as one step
    /// "Put into slot" (criteria 4 to 6). A filled slot refuses (criterion 7) unless `replace`
    /// is set: then its content is removed in the same step "Replace slot content".
    ///
    /// # Errors
    /// The sentence to show when nothing was written for another reason.
    pub fn put_into_slot(
        &mut self,
        from: &SlotSource<'_>,
        slot: NodeId,
        mode: FitMode,
        replace: bool,
    ) -> Result<SlotDrop, String> {
        let targets = self.slot_targets();
        let target = targets
            .iter()
            .find(|t| t.id == slot)
            .ok_or_else(|| "the slot is gone".to_owned())?;
        let content = self
            .slot_contents(std::slice::from_ref(target))
            .pop()
            .unwrap_or_default();
        if !content.is_empty() && !replace {
            return Ok(SlotDrop {
                refused: Some(slot_filled_text(&target.name)),
                notice: None,
            });
        }
        let unit = from.unit().ok_or_else(|| "nothing to put".to_owned())?;
        let fit = target.fit(unit.box_, mode);
        let fragment = from.mapped(&unit, fit, target)?;
        let copy = Copy {
            fragment: 0,
            offset: fit_offset(&unit, fit, target),
            anchors: self.mint(fragment.anchor_count()),
        };
        let label = if content.is_empty() {
            PUT_INTO_SLOT_LABEL
        } else {
            REPLACE_SLOT_CONTENT_LABEL
        };
        let mapped = [Mapped {
            scale: fit.scale,
            angle: fit_angle(fit, target),
            fragment,
        }];
        let reports =
            self.write_copies(label, &unit, &mapped, &[copy], from.came_from, &content)?;
        let real = mode == FitMode::RealSize;
        let notice = join_notices(fit_notice(&target.name, &fit, real), &reports);
        self.select_copies(&reports);
        Ok(SlotDrop {
            refused: None,
            notice,
        })
    }

    /// Puts one copy of `from` into every empty slot of `holder`, each fitted to its own slot, in
    /// slot order, as one step "Fill slots" (criterion 8). Filled slots are skipped and counted.
    ///
    /// # Errors
    /// The sentence to show when nothing was written for another reason.
    pub fn fill_empty_slots(
        &mut self,
        from: &SlotSource<'_>,
        holder: NodeId,
        mode: FitMode,
    ) -> Result<SlotDrop, String> {
        let targets: Vec<SlotTarget> = self
            .slot_targets()
            .into_iter()
            .filter(|t| t.holder == holder)
            .collect();
        let holder_name = targets
            .first()
            .map(|t| t.holder_name.clone())
            .ok_or_else(|| "the holder has no slot".to_owned())?;
        let contents = self.slot_contents(&targets);
        let empty: Vec<&SlotTarget> = targets
            .iter()
            .zip(&contents)
            .filter(|(_, content)| content.is_empty())
            .map(|(target, _)| target)
            .collect();
        if empty.is_empty() {
            return Ok(SlotDrop {
                refused: Some(no_empty_slot_text(&holder_name)),
                notice: None,
            });
        }
        let unit = from.unit().ok_or_else(|| "nothing to put".to_owned())?;
        let plan = self.plan_fill(from, &unit, &empty, mode)?;
        let reports = self.write_copies(
            FILL_SLOTS_LABEL,
            &unit,
            &plan.mapped,
            &plan.copies,
            from.came_from,
            &[],
        )?;
        let scales = plan.scales;
        let skipped = targets.len() - empty.len();
        let real = mode == FitMode::RealSize;
        let notice = join_notices(fill_notice(empty.len(), skipped, &scales, real), &reports);
        self.select_copies(&reports);
        Ok(SlotDrop {
            refused: None,
            notice,
        })
    }

    /// The fill of `empty` with `unit`: the fit of each slot, a fragment mapped once per distinct
    /// fit (ADR 0088 decision 3), each copy's move and anchor ids.
    fn plan_fill(
        &mut self,
        from: &SlotSource<'_>,
        unit: &Unit,
        empty: &[&SlotTarget],
        mode: FitMode,
    ) -> Result<FillPlan, String> {
        let mut plan = FillPlan {
            mapped: Vec::new(),
            copies: Vec::new(),
            scales: Vec::new(),
        };
        for target in empty {
            let fit = target.fit(unit.box_, mode);
            let angle = fit_angle(fit, target);
            let index =
                if let Some(index) = plan.mapped.iter().position(|m| m.serves(fit.scale, angle)) {
                    index
                } else {
                    let fragment = from.mapped(unit, fit, target)?;
                    plan.mapped.push(Mapped {
                        scale: fit.scale,
                        angle,
                        fragment,
                    });
                    plan.mapped.len() - 1
                };
            let count = plan.mapped[index].fragment.anchor_count();
            plan.copies.push(Copy {
                fragment: index,
                offset: fit_offset(unit, fit, target),
                anchors: self.mint(count),
            });
            plan.scales.push(fit.scale);
        }
        Ok(plan)
    }

    /// `count` fresh anchor ids.
    fn mint(&mut self, count: usize) -> Vec<AnchorId> {
        (0..count).map(|_| self.minter.mint()).collect()
    }

    /// Inserts every copy and then removes `remove`, as one step `label`. Every copy is checked
    /// before the first write, so a refusal writes nothing (`Document::batch` keeps what was
    /// written before an error); `remove` was read from this document in the same command, so its
    /// delete has no stale id to refuse, and an error of it is still reported, not dropped.
    fn write_copies(
        &mut self,
        label: &'static str,
        unit: &Unit,
        mapped: &[Mapped],
        copies: &[Copy],
        came_from: &CameFrom,
        remove: &[NodeId],
    ) -> Result<Vec<FragmentReport>, String> {
        let page = self.active_page().id;
        self.document.set_draw_parent(self.draw_parent(None));
        let wrap = unit.roots.len() > 1;
        let target_of = |copy: &Copy| {
            if wrap {
                FragmentTarget::Group {
                    page: &page,
                    offset: copy.offset,
                    name: &unit.name,
                }
            } else {
                FragmentTarget::Elements {
                    page: &page,
                    offset: copy.offset,
                }
            }
        };
        for copy in copies {
            let fragment = &mapped[copy.fragment].fragment;
            self.document
                .check_insert(fragment, target_of(copy), &copy.anchors)
                .map_err(|error| error.to_string())?;
        }
        self.recording_unguarded(|this| {
            this.document.batch(label, |document| {
                let mut reports = Vec::new();
                for copy in copies {
                    let fragment = &mapped[copy.fragment].fragment;
                    let report = document.insert_fragment(
                        fragment,
                        target_of(copy),
                        came_from,
                        &copy.anchors,
                    );
                    reports.push(report.map_err(|error| error.to_string())?);
                }
                if !remove.is_empty() {
                    document.delete_objects(remove).map_err(|error| {
                        format!("the slot's content could not be removed: {error}")
                    })?;
                }
                Ok(reports)
            })
        })
    }

    /// Selects every copy and takes the Select tool (criterion 4).
    fn select_copies(&mut self, reports: &[FragmentReport]) {
        let roots: Vec<NodeId> = reports
            .iter()
            .flat_map(|r| r.roots.iter().copied())
            .collect();
        self.tool = Tool::Select;
        self.selection.set(&roots);
    }
}

/// The fit sentence first, then the `0137` clone sentence (UX notes V21).
fn join_notices(fit: Option<String>, reports: &[FragmentReport]) -> Option<String> {
    let clones = reports
        .first()
        .and_then(|r| clones_became_paths_text(r.clones_became_paths));
    match (fit, clones) {
        (Some(a), Some(b)) => Some(format!("{a} {b}")),
        (a, b) => a.or(b),
    }
}
