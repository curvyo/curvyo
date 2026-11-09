//! Manufacturings (ADR 0032 §4 to §8, ADR 0033): a top-level node that owns an output page sized
//! to the machine's work area, with live placements of drawing pages and installed fixtures as
//! its children. This module holds their reads and the commands that write them.

use loro::{LoroMap, TreeID, TreeParentId};
use serde::Serialize;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::document_size::{DocumentSizeError, validated_document_side};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{
    KEY_FIXTURE_KIND, KEY_MACHINE_NAME, KEY_MANUFACTURING, KEY_PAGE, KEY_PROCESS, KEY_PROFILE,
    PAGE_OWNER_FIELD, PlacementLink, SHAPE_FIXTURE, SHAPE_PLACEMENT, is_manufacturing_node,
    manufacturing_nodes, owner_text, read_fixture_params, read_placement_link, string,
    write_fixture_params, write_placement_link, write_plain_map,
};
use crate::node_registers::{KEY_NAME, KEY_VISIBLE, read_registers};
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
/// The label of Add fixture's commit.
pub const ADD_FIXTURE_COMMIT_LABEL: &str = "add_fixture";
/// The label of Delete fixture's commit.
pub const DELETE_FIXTURE_COMMIT_LABEL: &str = "delete_fixture";
/// The label of a fixture's hide or show.
pub const SET_FIXTURE_VISIBLE_COMMIT_LABEL: &str = "set_fixture_visible";
/// The label of a fixture parameter edit.
pub const SET_FIXTURE_PARAMS_COMMIT_LABEL: &str = "set_fixture_params";
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
/// The label of an edit of the copy's laser mode.
pub const EDIT_LASER_MODE_COMMIT_LABEL: &str = "edit_laser_mode";

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
    /// The laser mode switch.
    EditLaserMode,
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
            Self::EditLaserMode => EDIT_LASER_MODE_COMMIT_LABEL,
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
    /// The installed fixtures, in tree order.
    pub fixtures: Vec<FixtureSnapshot>,
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

/// An installed fixture (ADR 0032 §8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name ("Engraved grid", "Engraved grid 2").
    pub name: String,
    /// `<process id>/<kind>`.
    pub kind: String,
    /// The stored parameter values.
    pub params: FixtureParams,
    /// Whether it is shown on the output page.
    pub visible: bool,
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
fn next_numbered(label: &str, taken: &[String], first_bare: bool) -> String {
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
        let mut fixtures = Vec::new();
        for child in tree.children(TreeParentId::Node(node)).unwrap_or_default() {
            let Ok(child_meta) = tree.get_meta(child) else {
                continue;
            };
            match string(&child_meta, KEY_SHAPE).as_deref() {
                Some(SHAPE_PLACEMENT) => {
                    if let Some(link) = read_placement_link(&child_meta) {
                        placements.push(PlacementSnapshot {
                            id: node_id(child),
                            page: link.page,
                            matrix: link.matrix,
                        });
                    }
                }
                Some(SHAPE_FIXTURE) => fixtures.push(read_fixture(child, &child_meta)),
                _ => {}
            }
        }
        Some(ManufacturingSnapshot {
            id: node_id(node),
            name: read_registers(&meta).name.unwrap_or_default(),
            process: string(&meta, KEY_PROCESS)?,
            profile: crate::manufacturing_codec::read_profile_map(&meta),
            legacy_machine_name: string(&meta, KEY_MACHINE_NAME),
            page: owned_page(&self.loro().get_map(ROOT_MAP), &tree, &page_id),
            placements,
            fixtures,
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
            origin: crate::pages::next_page_origin(&pages, size),
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
    /// own items are not written (ADR 0042 §6, `specs/0098-page-kinds` criterion 14).
    /// `Ok(false)` and nothing written when the copy, the size and the old key are unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing or one without its page;
    /// [`ManufacturingError::Size`] for a side outside the page size limits.
    pub fn set_manufacturing_profile(
        &self,
        id: NodeId,
        profile: &FixtureParams,
        work_area: DocumentSize,
        anchor: PageCorner,
        change: ProfileChange,
    ) -> Result<bool, ManufacturingError> {
        let manufacturing = self.manufacturing(id).ok_or(ManufacturingError::NotFound)?;
        let page = manufacturing.page.ok_or(ManufacturingError::NotFound)?;
        let width = validated_document_side(work_area.width)?;
        let height = validated_document_side(work_area.height)?;
        #[allow(clippy::float_cmp)] // exact register equality avoids an empty step
        let same_size =
            page.size.width.as_mm() == width.as_mm() && page.size.height.as_mm() == height.as_mm();
        let same_profile = manufacturing.profile.as_ref() == Some(profile);
        if same_size && same_profile && manufacturing.legacy_machine_name.is_none() {
            return Ok(false);
        }
        let meta = self
            .loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(tree_id_of(id))
            .map_err(|_| ManufacturingError::NotFound)?;
        if !same_profile {
            write_plain_map(&meta, KEY_PROFILE, profile);
        }
        if manufacturing.legacy_machine_name.is_some() {
            meta.delete(KEY_MACHINE_NAME)
                .map_err(|_| ManufacturingError::NotFound)?;
        }
        if !same_size {
            let shift = Vec2::new(
                if anchor.right {
                    page.size.width.as_mm() - width.as_mm()
                } else {
                    0.0
                },
                if anchor.bottom {
                    page.size.height.as_mm() - height.as_mm()
                } else {
                    0.0
                },
            );
            write_frame(
                &self.loro().get_map(ROOT_MAP),
                &page.id,
                Frame {
                    origin: Point::new(page.origin.x + shift.x, page.origin.y + shift.y),
                    size: DocumentSize { width, height },
                },
            );
            let back = Affine::translation(Vec2::new(-shift.x, -shift.y));
            let tree = self.loro().get_tree(OBJECTS_TREE);
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
        self.commit_with_label(change.label());
        Ok(true)
    }

    /// Installs a fixture of `kind` (`<process>/<kind>`) named `label`, numbered when the name is
    /// taken ("Engraved grid 2"), with the stored `params`, at the end of the setup, in one
    /// history step (criterion 9).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing.
    ///
    /// # Panics
    /// Does not panic in practice: the meta map of a node just created exists.
    pub fn add_fixture(
        &self,
        manufacturing: NodeId,
        kind: &str,
        label: &str,
        params: &FixtureParams,
    ) -> Result<NodeId, ManufacturingError> {
        let snapshot = self
            .manufacturing(manufacturing)
            .ok_or(ManufacturingError::NotFound)?;
        let taken: Vec<String> = snapshot.fixtures.into_iter().map(|f| f.name).collect();
        let name = next_numbered(label, &taken, true);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let fixture = tree
            .create(TreeParentId::Node(tree_id_of(manufacturing)))
            .map_err(|_| ManufacturingError::NotFound)?;
        // invariant: the node was just created, so its meta map exists; plain values only.
        #[allow(clippy::unwrap_used)]
        {
            let meta = tree.get_meta(fixture).unwrap();
            meta.insert(KEY_SHAPE, SHAPE_FIXTURE).unwrap();
            meta.insert(KEY_FIXTURE_KIND, kind).unwrap();
            meta.insert(KEY_NAME, name.as_str()).unwrap();
            write_fixture_params(&meta, params);
        }
        self.commit_with_label(ADD_FIXTURE_COMMIT_LABEL);
        Ok(node_id(fixture))
    }

    /// Removes the fixture `id` in one history step.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn delete_fixture(&self, id: NodeId) -> Result<(), ManufacturingError> {
        self.fixture_meta(id)?;
        self.loro()
            .get_tree(OBJECTS_TREE)
            .delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        self.commit_with_label(DELETE_FIXTURE_COMMIT_LABEL);
        Ok(())
    }

    /// Hides or shows the fixture `id` through the ordinary `visible` register, saved and
    /// undoable (`adrs.md` decision 4). `Ok(false)` when it already is.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_visible(
        &self,
        id: NodeId,
        visible: bool,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if read_registers(&meta).visible == visible {
            return Ok(false);
        }
        meta.insert(KEY_VISIBLE, visible)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        self.commit_with_label(SET_FIXTURE_VISIBLE_COMMIT_LABEL);
        Ok(true)
    }

    /// Writes the fixture's parameter values whole, so a merge never pairs one peer's spacing with
    /// another's origin (ADR 0033 §2). `Ok(false)` when they are unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_params(
        &self,
        id: NodeId,
        params: &FixtureParams,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if read_fixture_params(&meta).as_ref() == Some(params) {
            return Ok(false);
        }
        write_fixture_params(&meta, params);
        self.commit_with_label(SET_FIXTURE_PARAMS_COMMIT_LABEL);
        Ok(true)
    }

    fn fixture_meta(&self, id: NodeId) -> Result<LoroMap, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let parent = match tree.parent(node) {
            Some(TreeParentId::Node(parent)) if node_exists(&tree, node) => parent,
            _ => return Err(ManufacturingError::NoSuchFixture),
        };
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        if !is_manufacturing_node(&tree, parent)
            || string(&meta, KEY_SHAPE).as_deref() != Some(SHAPE_FIXTURE)
        {
            return Err(ManufacturingError::NoSuchFixture);
        }
        Ok(meta)
    }
}

fn read_fixture(node: TreeID, meta: &LoroMap) -> FixtureSnapshot {
    let registers = read_registers(meta);
    FixtureSnapshot {
        id: node_id(node),
        name: registers.name.unwrap_or_default(),
        kind: string(meta, KEY_FIXTURE_KIND).unwrap_or_default(),
        params: read_fixture_params(meta).unwrap_or_default(),
        visible: registers.visible,
    }
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
