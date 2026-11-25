//! Manufacturings (ADR 0032 §4 to §8, ADR 0033): a top-level node that owns an output page sized
//! to the machine's work area, with live placements of drawing pages and installed fixtures as
//! its children. This module holds their reads and the commands that write them.

use loro::{TreeID, TreeParentId};
use serde::Serialize;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::document_size::{DocumentSizeError, validated_document_side};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{
    KEY_MACHINE_NAME, KEY_MANUFACTURING, KEY_PAGE, KEY_PROCESS, KEY_PROFILE, PAGE_OWNER_FIELD,
    PlacementLink, SHAPE_PLACEMENT, is_manufacturing_node, manufacturing_nodes, owner_text,
    read_placement_link, string, write_placement_link, write_plain_map,
};
use crate::manufacturing_fixture_sets::FixtureSetSnapshot;
use crate::manufacturing_fixtures::{FixtureSetRecord, FixtureSnapshot};
use crate::node_registers::read_registers;
use crate::page_frame::{Frame, write_frame};
use crate::pages::{Page, owned_page, write};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;
use crate::units::DocumentSize;
use crate::{Point, Vec2};

/// The label of Add page with an output kind (`specs/0098-page-kinds` criterion 4).
pub const ADD_OUTPUT_PAGE_COMMIT_LABEL: &str = "add_output_page";
/// The label of the delete of a placement (criterion 12).
pub const DELETE_PLACEMENT_COMMIT_LABEL: &str = "delete_placement";
/// The label of a machine profile pick (`specs/0026-machine-profile` criterion 5).
pub const PICK_PROFILE_COMMIT_LABEL: &str = "pick_profile";
/// The label of Reset from profile (criterion 7).
pub const RESET_PROFILE_COMMIT_LABEL: &str = "reset_profile";
/// The label of Save as profile pointing the copy at the new record (criterion 3).
pub const SAVE_AS_PROFILE_COMMIT_LABEL: &str = "save_as_profile";
/// The label of an edit of the copy's name.
pub const EDIT_PROFILE_NAME_COMMIT_LABEL: &str = "edit_profile_name";
/// The label of an edit of the copy's origin corner.
pub const EDIT_ORIGIN_CORNER_COMMIT_LABEL: &str = "edit_origin_corner";
/// The label of an edit of the copy's max feed.
pub const EDIT_MAX_FEED_COMMIT_LABEL: &str = "edit_max_feed";
/// The label of an edit of the copy's max power.
pub const EDIT_MAX_POWER_COMMIT_LABEL: &str = "edit_max_power";
/// The label of an edit of the copy's Default power (`specs/0150` criterion 1).
pub const EDIT_DEFAULT_POWER_COMMIT_LABEL: &str = "edit_default_power";
/// The label of an edit of the copy's Default speed (`specs/0150` criterion 1).
pub const EDIT_DEFAULT_SPEED_COMMIT_LABEL: &str = "edit_default_speed";
/// The label of an edit of the copy's laser mode.
pub const EDIT_LASER_MODE_COMMIT_LABEL: &str = "edit_laser_mode";
/// The label of the copy's motorised Z axis switched on (`specs/0111` criterion 22).
pub const MOTORISED_Z_ON_COMMIT_LABEL: &str = "motorised_z_on";
/// The label of the copy's motorised Z axis switched off (`specs/0111` criterion 22).
pub const MOTORISED_Z_OFF_COMMIT_LABEL: &str = "motorised_z_off";
/// The label of the copy's Air assist switched on (`specs/0029` criterion 25).
pub const AIR_ASSIST_ON_COMMIT_LABEL: &str = "air_assist_on";
/// The label of the copy's Air assist switched off (`specs/0029` criterion 25).
pub const AIR_ASSIST_OFF_COMMIT_LABEL: &str = "air_assist_off";

/// The label of the copy's focus pulse switched on (`specs/0118` 35 a).
pub const FOCUS_PULSE_ON_COMMIT_LABEL: &str = "focus_pulse_on";
/// The label of the copy's focus pulse switched off (`specs/0118` 35 a).
pub const FOCUS_PULSE_OFF_COMMIT_LABEL: &str = "focus_pulse_off";
/// The label of an edit of the copy's pulse power (`specs/0118` 35 a).
pub const EDIT_PULSE_POWER_COMMIT_LABEL: &str = "edit_pulse_power";
/// The label of an edit of the copy's pulse length (`specs/0118` 35 a).
pub const EDIT_PULSE_LENGTH_COMMIT_LABEL: &str = "edit_pulse_length";

/// The label of the copy and page taking the size the controller reports (`specs/0131`).
pub const USE_MACHINE_SIZE_COMMIT_LABEL: &str = "use_machine_size";

/// Which change writes the machine profile copy; each is one history step with its own label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileChange {
    /// A pick from the library.
    Pick,
    /// Reset from profile.
    Reset,
    /// Save as profile (the copy now points at the new record).
    SaveAs,
    /// The Name field.
    EditName,
    /// The origin corner.
    EditOrigin,
    /// The max feed field.
    EditMaxFeed,
    /// The max power field.
    EditMaxPower,
    /// The Default power field (`specs/0150` criterion 1).
    EditDefaultPower,
    /// The Default speed field (`specs/0150` criterion 1).
    EditDefaultSpeed,
    /// The laser mode switch.
    EditLaserMode,
    /// The Motorised Z axis switch, to `on` (ADR 0056 §7).
    MotorisedZ {
        /// The new state.
        on: bool,
    },
    /// The Air assist switch, to `on` (ADR 0065 §3).
    AirAssist {
        /// The new state.
        on: bool,
    },
    /// Use machine size: the work area the controller reports (`specs/0131`).
    UseMachineSize,
    /// The Focus pulse switch, to `on` (`specs/0118` 35 a).
    FocusPulse {
        /// The new state.
        on: bool,
    },
    /// The Pulse power field.
    EditPulsePower,
    /// The Pulse length field.
    EditPulseLength,
}

impl ProfileChange {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pick => PICK_PROFILE_COMMIT_LABEL,
            Self::Reset => RESET_PROFILE_COMMIT_LABEL,
            Self::SaveAs => SAVE_AS_PROFILE_COMMIT_LABEL,
            Self::EditName => EDIT_PROFILE_NAME_COMMIT_LABEL,
            Self::EditOrigin => EDIT_ORIGIN_CORNER_COMMIT_LABEL,
            Self::EditMaxFeed => EDIT_MAX_FEED_COMMIT_LABEL,
            Self::EditMaxPower => EDIT_MAX_POWER_COMMIT_LABEL,
            Self::EditDefaultPower => EDIT_DEFAULT_POWER_COMMIT_LABEL,
            Self::EditDefaultSpeed => EDIT_DEFAULT_SPEED_COMMIT_LABEL,
            Self::EditLaserMode => EDIT_LASER_MODE_COMMIT_LABEL,
            Self::MotorisedZ { on: true } => MOTORISED_Z_ON_COMMIT_LABEL,
            Self::MotorisedZ { on: false } => MOTORISED_Z_OFF_COMMIT_LABEL,
            Self::AirAssist { on: true } => AIR_ASSIST_ON_COMMIT_LABEL,
            Self::AirAssist { on: false } => AIR_ASSIST_OFF_COMMIT_LABEL,
            Self::UseMachineSize => USE_MACHINE_SIZE_COMMIT_LABEL,
            Self::FocusPulse { on: true } => FOCUS_PULSE_ON_COMMIT_LABEL,
            Self::FocusPulse { on: false } => FOCUS_PULSE_OFF_COMMIT_LABEL,
            Self::EditPulsePower => EDIT_PULSE_POWER_COMMIT_LABEL,
            Self::EditPulseLength => EDIT_PULSE_LENGTH_COMMIT_LABEL,
        }
    }
}

/// A fixture's parameter values as stored: opaque plain values (ADR 0033 §3).
pub type FixtureParams = serde_json::Map<String, serde_json::Value>;

/// One manufacturing as read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManufacturingSnapshot {
    /// Its node.
    pub id: NodeId,
    /// The node's old name ("Laser 1"), left as is in old files and no longer written: the page
    /// name is the one shown (ADR 0042 §5).
    pub name: String,
    /// The process id, equal to the extension's id (`curvyo.laser`).
    pub process: String,
    /// The machine profile copy `mfg_profile`, opaque here and typed by the process's extension
    /// (ADR 0039 §1); `None` for a format-35 manufacturing.
    pub profile: Option<FixtureParams>,
    /// The format-35 placeholder machine name, read for the migration (ADR 0039 §6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_machine_name: Option<String>,
    /// The owned output page; `None` when its record is missing (damaged by a merge).
    pub page: Option<Page>,
    /// The placements, in tree order.
    pub placements: Vec<PlacementSnapshot>,
    /// The named machine sets, in child order (ADR 0059 §1).
    pub sets: Vec<FixtureSetSnapshot>,
    /// Every fixture in listing order (ADR 0059 §3): the sets' fixtures, set by set in child
    /// order, then the job fixtures in child order. A fixture's `set` names its set.
    pub fixtures: Vec<FixtureSnapshot>,
    /// The profile record's `modified` (Unix ms) the set mirror was last aligned to or written
    /// with (`mfg_fixture_sets_modified`, ADR 0059 §7 amended 2026-11-16); 0 when absent or not
    /// a whole number.
    pub sets_modified: i64,
    /// The raw process registers `x.<process id>.<key>`, opaque here (ADR 0033 §3).
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub process_values: FixtureParams,
}

impl ManufacturingSnapshot {
    /// The work area: the output page's size.
    #[must_use]
    pub fn work_area(&self) -> DocumentSize {
        self.page
            .as_ref()
            .map_or_else(DocumentSize::default, |page| page.size)
    }

    /// The job fixtures (in no set), in child order.
    pub fn job_fixtures(&self) -> impl Iterator<Item = &FixtureSnapshot> {
        self.fixtures.iter().filter(|fixture| fixture.set.is_none())
    }

    /// The fixtures of the set `set`, in child order.
    pub fn set_fixtures(&self, set: NodeId) -> impl Iterator<Item = &FixtureSnapshot> {
        self.fixtures
            .iter()
            .filter(move |fixture| fixture.set == Some(set))
    }

    /// The On this page check of the set that holds `fixture`; `None` for a job fixture.
    #[must_use]
    pub fn set_on_page(&self, fixture: &FixtureSnapshot) -> Option<bool> {
        let set = fixture.set?;
        self.sets.iter().find(|s| s.id == set).map(|s| s.on_page)
    }

    /// The sets as a profile record stores them: ids, names, locks and fixtures, no On this
    /// page, no On the bed, no Output and no job fixture (ADR 0060 §1, `0113` criterion 10). A
    /// node without a profile id gives an empty id.
    #[must_use]
    pub fn set_records(&self) -> Vec<FixtureSetRecord> {
        self.sets
            .iter()
            .map(|set| FixtureSetRecord {
                id: set.uid.clone().unwrap_or_default(),
                name: set.name.clone(),
                locked: set.locked,
                fixtures: self
                    .set_fixtures(set.id)
                    .map(FixtureSnapshot::record)
                    .collect(),
            })
            .collect()
    }
}

/// The sets Pick profile writes as the page's fresh mirror, and the record `modified` they come
/// with (ADR 0059 §6, §7 amended 2026-11-16).
#[derive(Debug, Clone, Copy)]
pub struct ProfileSets<'a> {
    /// The record's sets, in order; none deletes the mirror (Generic laser).
    pub sets: &'a [FixtureSetRecord],
    /// The record's `modified`, written as `mfg_fixture_sets_modified`.
    pub modified: i64,
}

/// A live reference to a drawing page (ADR 0032 §6).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlacementSnapshot {
    /// Its node.
    pub id: NodeId,
    /// The drawing page it shows.
    pub page: String,
    /// Maps the page's local mm (origin at its top-left) to the output page's local mm.
    pub matrix: Affine,
}

/// What Add page with an output kind creates.
#[derive(Debug, Clone, Copy)]
pub struct NewOutputPage<'a> {
    /// The process id.
    pub process: &'a str,
    /// The word the page's name starts with ("Laser").
    pub label: &'a str,
    /// The machine profile copy, written whole; `None` writes no copy.
    pub profile: Option<&'a FixtureParams>,
    /// The work area, the page's size.
    pub work_area: DocumentSize,
}

/// The corner of an output page that keeps its place in the plane when the work area changes
/// (the machine origin, ADR 0042 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageCorner {
    /// The corner is on the right edge.
    pub right: bool,
    /// The corner is on the bottom edge.
    pub bottom: bool,
}

/// A refused manufacturing command; it writes nothing.
#[derive(Debug, thiserror::Error)]
pub enum ManufacturingError {
    /// The id names no live manufacturing.
    #[error("no such manufacturing")]
    NotFound,
    /// The id names no live fixture.
    #[error("no such fixture")]
    NoSuchFixture,
    /// The id names no live slot of a fixture.
    #[error("no such slot")]
    NoSuchSlot,
    /// The holder holds 50 slots already (`specs/0102-laser-fixtures` criterion 19).
    #[error("a holder holds up to 50 slots")]
    TooManySlots,
    /// The id names no live fixture set of a manufacturing.
    #[error("no such fixture set")]
    NoSuchSet,
    /// The page holds 16 sets already (`0113` criterion 17).
    #[error("a laser page holds up to 16 fixture sets")]
    TooManySets,
    /// The page holds 64 job fixtures already, or an Apply would pass 64 (`0113` criterion 17,
    /// ADR 0059 §8).
    #[error("a laser page holds up to 64 job fixtures")]
    TooManyJobFixtures,
    /// The work area is not a valid page size.
    #[error(transparent)]
    Size(#[from] DocumentSizeError),
    /// A placed page would have no overlap with its own laser page (`0100` criterion 18).
    #[error("A placed page stays on its laser page.")]
    OffPage,
    /// A placement matrix would pass ±1e7 or is not finite.
    #[error("the place is too far out")]
    OutOfRange,
}

/// "`label` n" with the smallest n from 1 that no name of `taken` uses.
pub(crate) fn next_numbered(label: &str, taken: &[String], first_bare: bool) -> String {
    let free = |name: &String| !taken.contains(name);
    if first_bare && free(&label.to_owned()) {
        return label.to_owned();
    }
    let start = if first_bare { 2 } else { 1 };
    // One of the first `taken.len() + 1` numbers from `start` is free.
    (start..=start + taken.len())
        .map(|n| format!("{label} {n}"))
        .find(free)
        .unwrap_or_else(|| label.to_owned())
}

impl Document {
    /// Every manufacturing in creation order.
    #[must_use]
    pub fn manufacturings(&self) -> Vec<ManufacturingSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        manufacturing_nodes(&tree)
            .into_iter()
            .filter_map(|id| self.read_manufacturing(id))
            .collect()
    }

    /// The manufacturing `id`, `None` when it is not a live manufacturing.
    #[must_use]
    pub fn manufacturing(&self, id: NodeId) -> Option<ManufacturingSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        (node_exists(&tree, node) && is_manufacturing_node(&tree, node))
            .then(|| self.read_manufacturing(node))
            .flatten()
    }

    /// Whether `id` is a manufacturing or lies inside one: the skip predicate of ADR 0032 for
    /// callers that hold a node id (`specs/0093` `adrs.md` decision 2).
    #[must_use]
    pub fn is_in_manufacturing(&self, id: NodeId) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut node = tree_id_of(id);
        for _ in 0..=crate::MAX_GROUP_DEPTH * 4 {
            if is_manufacturing_node(&tree, node) {
                return true;
            }
            match tree.parent(node) {
                Some(TreeParentId::Node(parent)) => node = parent,
                _ => return false,
            }
        }
        false
    }

    fn read_manufacturing(&self, node: TreeID) -> Option<ManufacturingSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(node).ok()?;
        let page_id = string(&meta, KEY_PAGE)?;
        let mut placements = Vec::new();
        let children = tree.children(TreeParentId::Node(node)).unwrap_or_default();
        for &child in &children {
            let Ok(child_meta) = tree.get_meta(child) else {
                continue;
            };
            if string(&child_meta, KEY_SHAPE).as_deref() == Some(SHAPE_PLACEMENT)
                && let Some(link) = read_placement_link(&child_meta)
            {
                placements.push(PlacementSnapshot {
                    id: node_id(child),
                    page: link.page,
                    matrix: link.matrix,
                });
            }
        }
        let (sets, fixtures) =
            crate::manufacturing_fixture_sets::read_sets_and_fixtures(&tree, &children);
        Some(ManufacturingSnapshot {
            id: node_id(node),
            name: read_registers(&meta).name.unwrap_or_default(),
            process: string(&meta, KEY_PROCESS)?,
            profile: crate::manufacturing_codec::read_profile_map(&meta),
            legacy_machine_name: string(&meta, KEY_MACHINE_NAME),
            page: owned_page(&self.loro().get_map(ROOT_MAP), &tree, &page_id),
            placements,
            sets,
            fixtures,
            sets_modified: crate::manufacturing_codec::read_sets_modified(&meta),
            process_values: crate::manufacturing_codec::process_values(&meta),
        })
    }

    /// Adds an output page of `new.process` named "`label` N" with exactly the work area, its
    /// manufacturing node holding the machine copy, no fixture and no placement, placed by the
    /// Add page rule, in one history step (ADR 0042 §5, `specs/0098-page-kinds` criterion 4).
    /// The node goes to the end of the root, after every drawing node.
    ///
    /// # Errors
    /// [`ManufacturingError::Size`] for a work area outside the page size limits.
    ///
    /// # Panics
    /// Does not panic in practice: the tree operations act on the root and a node just created.
    pub fn add_output_page(&self, new: NewOutputPage<'_>) -> Result<Page, ManufacturingError> {
        let size = DocumentSize {
            width: validated_document_side(new.work_area.width)?,
            height: validated_document_side(new.work_area.height)?,
        };
        let pages = self.pages();
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        // invariant: the root always accepts a new child.
        #[allow(clippy::unwrap_used)]
        let node = tree.create(TreeParentId::Root).unwrap();
        let page = Page {
            id: format!("mfg-{}-{}", node.peer, node.counter),
            name: crate::pages::next_free_name(&pages, new.label),
            origin: crate::pages::add_page_origin(&pages, size),
            size,
            grid: crate::PageGrid::DEFAULT,
            kind: crate::PageKind::Output {
                manufacturing: node_id(node),
                process: new.process.to_owned(),
            },
        };
        // invariant: the node was just created, so its meta map exists; plain values only.
        #[allow(clippy::unwrap_used)]
        {
            let meta = tree.get_meta(node).unwrap();
            meta.insert(KEY_MANUFACTURING, true).unwrap();
            meta.insert(KEY_PROCESS, new.process).unwrap();
            meta.insert(KEY_PAGE, page.id.as_str()).unwrap();
            if let Some(profile) = new.profile {
                write_plain_map(&meta, KEY_PROFILE, profile);
            }
        }
        let root = self.loro().get_map(ROOT_MAP);
        Self::write_new_page(&root, &pages, &page);
        write(&root, &page.id, "deleted", false);
        write(&root, &page.id, PAGE_OWNER_FIELD, owner_text(node));
        self.commit_with_label(ADD_OUTPUT_PAGE_COMMIT_LABEL);
        Ok(page)
    }

    /// Removes the manufacturing node `id` with its placements and fixtures in one tree delete,
    /// uncommitted: the page that owns it is being deleted in the same step (ADR 0042 §5).
    pub(crate) fn remove_manufacturing_node(&self, id: NodeId) -> Result<(), ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NotFound)?;
        // The rule of `specs/0023-groups` that every delete sweeps emptied groups; a
        // manufacturing sits at the root and holds no group, so this finds nothing of its own.
        crate::groups::remove_emptied_groups(&tree, &[None]);
        Ok(())
    }

    /// Deletes, uncommitted, every placement of the page `page` on every laser page: the page is
    /// being deleted in the same step (criterion 24).
    pub(crate) fn remove_placements_of(&self, page: &str) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let doomed: Vec<loro::TreeID> = manufacturing_nodes(&tree)
            .into_iter()
            .flat_map(|m| tree.children(TreeParentId::Node(m)).unwrap_or_default())
            .filter(|&child| {
                tree.get_meta(child)
                    .ok()
                    .and_then(|meta| read_placement_link(&meta))
                    .is_some_and(|link| link.page == page)
            })
            .collect();
        for node in doomed {
            let _ = tree.delete(node);
        }
        crate::groups::remove_emptied_groups(&tree, &[None]);
    }
    /// Deletes the placements `ids`, uncommitted, inside the caller's step (Delete of placed pages
    /// in the one selection, `specs/0100` criterion 19).
    pub(crate) fn remove_placement_nodes(&self, ids: &[NodeId]) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for &id in ids {
            let _ = tree.delete(tree_id_of(id));
        }
        crate::groups::remove_emptied_groups(&tree, &[None]);
    }

    /// Removes the placement `id` (only the reference; the drawing page stays) in one history
    /// step (criterion 12).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `id` is not a live placement.
    pub fn delete_placement(&self, id: NodeId) -> Result<(), ManufacturingError> {
        let placed = self
            .manufacturings()
            .iter()
            .any(|m| m.placements.iter().any(|p| p.id == id));
        if !placed {
            return Err(ManufacturingError::NotFound);
        }
        self.loro()
            .get_tree(OBJECTS_TREE)
            .delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NotFound)?;
        crate::groups::remove_emptied_groups(&self.loro().get_tree(OBJECTS_TREE), &[None]);
        self.commit_with_label(DELETE_PLACEMENT_COMMIT_LABEL);
        Ok(())
    }

    /// Writes the machine profile copy `profile` whole and sets the work area, the output page's
    /// size, in one history step labelled by `change` (ADR 0039 §4); the format-35 machine name
    /// goes in the same step. The `anchor` corner (the copy's origin) keeps its place in the
    /// plane and the frame grows away from it; each placement keeps its place in the plane and
    /// own items are not written (ADR 0042 §6, `specs/0098-page-kinds` criterion 14). The pages
    /// beside the frame's moving edges move with them, carrying what `contents` says they own,
    /// in the same step (ADR 0051 §2, `specs/0109` criterion 32).
    /// `Ok(false)` and nothing written when the copy, the size and the old key are unchanged.
    /// The page's fixtures are not written; [`Document::set_manufacturing_profile_and_sets`]
    /// writes its sets too.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing or one without its page;
    /// [`ManufacturingError::Size`] for a side outside the page size limits;
    /// [`ManufacturingError::OutOfRange`] when a moved page's objects would leave the range.
    pub fn set_manufacturing_profile(
        &self,
        id: NodeId,
        profile: &FixtureParams,
        work_area: DocumentSize,
        anchor: PageCorner,
        change: ProfileChange,
        contents: &crate::PageContents,
    ) -> Result<bool, ManufacturingError> {
        self.write_profile(id, profile, work_area, anchor, change, None, &[], contents)
    }

    /// As [`Document::set_manufacturing_profile`], and in the same step the page's sets are
    /// replaced by a fresh mirror of the profile's (Pick profile, ADR 0059 §6 amended 2026-11-16,
    /// `0113` criteria 9 and 14): one new set node per record, in its order, with the record's
    /// ids and locks, every set On this page, every fixture On the bed, every job Output off;
    /// no set deletes the mirror (Generic laser). `mfg_fixture_sets_modified` takes the record's
    /// `modified`. The job fixtures are never written. The sets are not written when they already
    /// equal the records and every set and fixture is on.
    ///
    /// # Errors
    /// As [`Document::set_manufacturing_profile`].
    #[allow(clippy::too_many_arguments)] // a pick carries the copy, page, corner, change, sets and owned contents
    pub fn set_manufacturing_profile_and_sets(
        &self,
        id: NodeId,
        profile: &FixtureParams,
        work_area: DocumentSize,
        anchor: PageCorner,
        change: ProfileChange,
        sets: ProfileSets<'_>,
        contents: &crate::PageContents,
    ) -> Result<bool, ManufacturingError> {
        self.write_profile(
            id,
            profile,
            work_area,
            anchor,
            change,
            Some(sets),
            &[],
            contents,
        )
    }

    /// As [`Document::set_manufacturing_profile_and_sets`] (with `sets`) or
    /// [`Document::set_manufacturing_profile`] (without), and in the same step the `fills` are
    /// written before the new copy: the numbers the operations of this manufacturing without
    /// `op_settings` read under the copy before the change (`specs/0150` adrs.md decision 4 b),
    /// so an undo returns copy and registers together and no visible number changes.
    ///
    /// # Errors
    /// As [`Document::set_manufacturing_profile`]; [`ManufacturingError::NotFound`] when a fill
    /// names no live operation or a test grid row. Nothing is written then.
    #[allow(clippy::too_many_arguments)] // the copy, page, corner, change, sets, fills and owned contents
    pub fn set_manufacturing_profile_filled(
        &self,
        id: NodeId,
        profile: &FixtureParams,
        work_area: DocumentSize,
        anchor: PageCorner,
        change: ProfileChange,
        sets: Option<ProfileSets<'_>>,
        fills: &[crate::SettingsWrite],
        contents: &crate::PageContents,
    ) -> Result<bool, ManufacturingError> {
        self.write_profile(
            id, profile, work_area, anchor, change, sets, fills, contents,
        )
    }

    #[allow(clippy::too_many_arguments)] // a pick carries the copy, page, corner, change, sets, fills and owned contents
    fn write_profile(
        &self,
        id: NodeId,
        profile: &FixtureParams,
        work_area: DocumentSize,
        anchor: PageCorner,
        change: ProfileChange,
        sets: Option<ProfileSets<'_>>,
        fills: &[crate::SettingsWrite],
        contents: &crate::PageContents,
    ) -> Result<bool, ManufacturingError> {
        let manufacturing = self.manufacturing(id).ok_or(ManufacturingError::NotFound)?;
        let page = manufacturing
            .page
            .clone()
            .ok_or(ManufacturingError::NotFound)?;
        let size = DocumentSize {
            width: validated_document_side(work_area.width)?,
            height: validated_document_side(work_area.height)?,
        };
        #[allow(clippy::float_cmp)] // exact register equality avoids an empty step
        let same_size = page.size.width.as_mm() == size.width.as_mm()
            && page.size.height.as_mm() == size.height.as_mm();
        let same_profile = manufacturing.profile.as_ref() == Some(profile);
        let reset_start = user_origin_reset(&manufacturing, profile, change);
        let current = manufacturing.set_records();
        let all_on = manufacturing.sets.iter().all(|set| set.on_page)
            && manufacturing
                .fixtures
                .iter()
                .all(|fixture| fixture.set.is_none() || fixture.active);
        let sets = sets.filter(|new| {
            !all_on || new.sets != current.as_slice() || new.modified != manufacturing.sets_modified
        });
        if same_size
            && same_profile
            && sets.is_none()
            && manufacturing.legacy_machine_name.is_none()
        {
            return Ok(false);
        }
        let meta = self
            .loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(tree_id_of(id))
            .map_err(|_| ManufacturingError::NotFound)?;
        let (shift, frame) = anchored_frame(&page, size, anchor);
        let pages = self.pages();
        let layout = if same_size {
            crate::page_layout::Layout::default()
        } else {
            crate::page_layout::reflow(&pages, &page.id, frame)
        };
        self.check_layout(&layout, contents)
            .map_err(|_| ManufacturingError::OutOfRange)?;
        let fill_metas = fills
            .iter()
            .map(|fill| self.writable_operation_meta(fill.operation))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ManufacturingError::NotFound)?;
        self.batch(change.label(), |doc| {
            Self::write_settings(&fill_metas, fills);
            doc.write_layout(&pages, &layout, contents)
                .map_err(|_| ManufacturingError::OutOfRange)?;
            if !same_profile {
                write_plain_map(&meta, KEY_PROFILE, profile);
            }
            if let Some(key) = &reset_start {
                meta.insert(key, "machine_origin")
                    .map_err(|_| ManufacturingError::NotFound)?;
            }
            if manufacturing.legacy_machine_name.is_some() {
                meta.delete(KEY_MACHINE_NAME)
                    .map_err(|_| ManufacturingError::NotFound)?;
            }
            if !same_size {
                write_frame(&doc.loro().get_map(ROOT_MAP), &page.id, frame);
                let back = Affine::translation(Vec2::new(-shift.x, -shift.y));
                let tree = doc.loro().get_tree(OBJECTS_TREE);
                for placement in &manufacturing.placements {
                    if let Ok(meta) = tree.get_meta(tree_id_of(placement.id)) {
                        write_placement_link(
                            &meta,
                            &PlacementLink {
                                page: placement.page.clone(),
                                matrix: back.compose(placement.matrix),
                            },
                        );
                    }
                }
            }
            if let Some(new) = sets {
                doc.replace_sets(id, new.sets)?;
                // invariant: a plain value under a known key on an attached map cannot fail.
                #[allow(clippy::unwrap_used)]
                meta.insert(crate::manufacturing_codec::KEY_SETS_MODIFIED, new.modified)
                    .unwrap();
            }
            Ok(true)
        })
    }
}

/// The shift of the page's origin and its new frame when the page takes `size` and keeps `anchor`.
fn anchored_frame(page: &Page, size: DocumentSize, anchor: PageCorner) -> (Vec2, Frame) {
    let shift = Vec2::new(
        if anchor.right {
            page.size.width.as_mm() - size.width.as_mm()
        } else {
            0.0
        },
        if anchor.bottom {
            page.size.height.as_mm() - size.height.as_mm()
        } else {
            0.0
        },
    );
    let frame = Frame {
        origin: Point::new(page.origin.x + shift.x, page.origin.y + shift.y),
        size,
    };
    (shift, frame)
}

/// The Start register of `manufacturing` when `change` puts another machine on the page while
/// Start is User origin: a point taken from one head is no place on another, so Start returns to
/// Machine origin in the same step and the point stays (`specs/0118` O9, Frage 345 C). Edits and
/// a pick of the same record keep it.
fn user_origin_reset(
    manufacturing: &ManufacturingSnapshot,
    profile: &FixtureParams,
    change: ProfileChange,
) -> Option<String> {
    let key = format!("x.{}.start", manufacturing.process);
    let other = manufacturing.profile.as_ref().and_then(|p| p.get("id")) != profile.get("id");
    let user_origin =
        manufacturing.process_values.get(&key) == Some(&serde_json::Value::from("user_origin"));
    (change == ProfileChange::Pick && other && user_origin).then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_take_the_smallest_free_number() {
        let taken = vec!["Laser 1".to_owned(), "Laser 3".to_owned()];
        assert_eq!(next_numbered("Laser", &taken, false), "Laser 2");
        assert_eq!(next_numbered("Laser", &[], false), "Laser 1");
        assert_eq!(next_numbered("Engraved grid", &[], true), "Engraved grid");
        let grids = vec!["Engraved grid".to_owned()];
        assert_eq!(
            next_numbered("Engraved grid", &grids, true),
            "Engraved grid 2"
        );
    }
}
