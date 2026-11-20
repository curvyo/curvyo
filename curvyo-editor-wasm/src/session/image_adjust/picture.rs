//! The selected picture as the Adjustments group reads it (`specs/0135` criteria 1, 60, 61): the
//! one image or image clone selected, the Image operations it runs in, in Operations-list order,
//! and its steps resolved at the shown operation's Interval.

use std::sync::Arc;

use curvyo_document_core::{ImageSnapshot, NodeId, ObjectSnapshot};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::adjust::{AdjustRegisters, ManualStep};
use curvyo_laser_core::job::{
    BlockSource, JobInput, NumbersContext, block_numbers, read_output_settings,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::Interval;

/// The Interval of a picture in no Image operation (criterion 60, the `0123` default).
pub(super) const UNASSIGNED_INTERVAL_UM: u32 = 100;

/// One Image operation the picture runs in.
#[derive(Debug, Clone)]
pub(in crate::session) struct PictureOperation {
    /// The operation.
    pub id: NodeId,
    /// Its name ("Image 1").
    pub name: String,
    /// Its Interval in µm (0.10 mm when it has no settings yet).
    pub interval_um: u32,
}

/// The steps that run on the picture at the shown operation's Interval.
#[derive(Debug, Clone)]
pub(super) struct ResolvedPicture {
    /// The steps in stored units.
    pub steps: Vec<ManualStep>,
    /// The Interval they were resolved at, in µm.
    pub interval_um: u32,
    /// The registers as read.
    pub registers: AdjustRegisters,
    /// The content key of the exact adjusted picture; `None` when its bytes cannot be read.
    pub key: Option<curvyo_laser_core::adjust::AdjustKey>,
}

/// The image leaf of `object`: an image, or the one image a clone of an image draws.
pub(super) fn image_leaf(object: &ObjectSnapshot) -> Option<&ImageSnapshot> {
    match object {
        ObjectSnapshot::Image(image) => Some(image),
        _ => match object.picture()? {
            [ObjectSnapshot::Image(image)] => Some(image),
            _ => None,
        },
    }
}

impl crate::session::Session {
    /// The picture the Adjustments group is for: exactly one image or image clone selected
    /// (`ux-notes.md` P2), `None` otherwise.
    pub fn adjust_target(&self) -> Option<NodeId> {
        let [id] = self.selection.ids() else {
            return None;
        };
        self.document.shown_adjustments(*id).ok().map(|_| *id)
    }

    /// The image leaf of picture `id` as drawn (a clone's carries its effective adjustments).
    pub(super) fn picture_leaf(&self, id: NodeId) -> Option<ImageSnapshot> {
        image_leaf(self.object_read().get(id)?).cloned()
    }

    /// The working image of picture `id`, `None` when its bytes cannot be read.
    pub(super) fn picture_pixels(&self, id: NodeId) -> Option<Arc<WorkingImage>> {
        self.job_pixels(&self.picture_leaf(id)?.asset)
    }

    /// The Image operations of the active laser page that run picture `id`, in Operations-list
    /// order (criterion 60).
    pub(in crate::session) fn picture_operations(&self, id: NodeId) -> Vec<PictureOperation> {
        self.picture_operations_in(&self.document_read(), id)
    }

    /// [`Self::picture_operations`] with the drawing read of now, `read`.
    pub(in crate::session) fn picture_operations_in(
        &self,
        read: &Arc<crate::session::object_cache::DocumentRead>,
        id: NodeId,
    ) -> Vec<PictureOperation> {
        self.with_page_job_input_in(read, |page| self.operations_in(page, id))
            .unwrap_or_default()
    }

    /// The Image operations of `page` that run picture `id`, in Operations-list order.
    fn operations_in(&self, page: &JobInput, id: NodeId) -> Vec<PictureOperation> {
        self.image_operations_in(page, |leaves| leaves.iter().any(|leaf| leaf.node == id))
    }

    /// The Interval of each picture's first Image operation in `page` (criterion 60), by the node
    /// its leaf is drawn for.
    pub(super) fn first_intervals(
        &self,
        page: &JobInput,
    ) -> std::collections::HashMap<NodeId, u32> {
        let mut intervals = std::collections::HashMap::new();
        let operations = self.image_operations_in(page, |_| true);
        for op in &operations {
            let Some(block) = page.blocks.iter().find(|b| b.id == op.id) else {
                continue;
            };
            for leaf in &block.leaves {
                intervals.entry(leaf.node).or_insert(op.interval_um);
            }
        }
        intervals
    }

    /// The Image operations of `page` whose leaves `runs` accepts, in Operations-list order.
    fn image_operations_in(
        &self,
        page: &JobInput,
        runs: impl Fn(&[curvyo_laser_core::job::LeafInput]) -> bool,
    ) -> Vec<PictureOperation> {
        let (Some(active), Some(manufacturing)) =
            (self.active_operations(), self.active_manufacturing())
        else {
            return Vec::new();
        };
        let fixture_material = read_output_settings(&manufacturing).fixture_material;
        let context = NumbersContext {
            materials: self.materials().reads(),
            profile_id: &page.machine.profile_id,
            fixture_material: fixture_material.as_deref(),
            max_feed: page.machine.values.max_feed,
        };
        let mut found: Vec<(usize, PictureOperation)> = Vec::new();
        for block in &page.blocks {
            let BlockSource::Operation { .. } = &block.source else {
                continue;
            };
            if block.kind.known() != Some(OperationKind::Image) || !runs(&block.leaves) {
                continue;
            }
            let Some(at) = active.operations.iter().position(|op| op.id == block.id) else {
                continue;
            };
            let op = &active.operations[at];
            let interval = block_numbers(block, context)
                .numbers()
                .map_or(Interval::DEFAULT, |n| n.picture_interval);
            found.push((
                at,
                PictureOperation {
                    id: op.id,
                    name: op.name.clone(),
                    interval_um: u32::from(interval.hundredths()) * 10,
                },
            ));
        }
        found.sort_by_key(|(at, _)| *at);
        found.into_iter().map(|(_, op)| op).collect()
    }

    /// The operation whose Interval the group and Make editable use: the Before and after pick
    /// when it is one of `operations`, else the first (criteria 29, 60).
    pub(super) fn shown_operation<'a>(
        &self,
        operations: &'a [PictureOperation],
    ) -> Option<&'a PictureOperation> {
        let picked = self.image_adjust.pane.operation;
        operations
            .iter()
            .find(|op| Some(op.id) == picked)
            .or_else(|| operations.first())
    }

    /// The shown operation's Interval of the selected picture, 0.10 mm without one.
    pub(super) fn shown_interval_um(&self) -> u32 {
        self.adjust_target().map_or(UNASSIGNED_INTERVAL_UM, |id| {
            let operations = self.picture_operations(id);
            self.shown_operation(&operations)
                .map_or(UNASSIGNED_INTERVAL_UM, |op| op.interval_um)
        })
    }

    /// The steps that run on picture `id` at the shown operation's Interval, with the held
    /// Strength (criteria 28, 29); `None` when its bytes cannot be read and a preset needs them.
    pub(super) fn resolved_picture(&self, id: NodeId) -> Option<ResolvedPicture> {
        let leaf = self.picture_leaf(id)?;
        let mut registers = AdjustRegisters::read(&leaf.adjustments);
        if let Some((held, strength)) = self.image_adjust.held_strength
            && held == id
        {
            registers.strength = strength;
        }
        let interval_um = self.shown_interval_um();
        let pixels = self.job_pixels(&leaf.asset);
        let (steps, key) = match pixels {
            Some(pixels) => {
                let resolved = self.adjust_pictures().resolve(
                    (&leaf.asset, &pixels, &leaf.corners()),
                    &registers,
                    interval_um,
                    0,
                );
                (resolved.steps, Some(resolved.key))
            }
            None if registers.preset == curvyo_laser_core::adjust::Preset::Expert => (
                registers.effective_steps(interval_um, || {
                    curvyo_image_core::Analysis::of_plane(&curvyo_image_core::GreyPlane::white(
                        1, 1,
                    ))
                }),
                None,
            ),
            None => return None,
        };
        Some(ResolvedPicture {
            steps,
            interval_um,
            registers,
            key,
        })
    }
}
