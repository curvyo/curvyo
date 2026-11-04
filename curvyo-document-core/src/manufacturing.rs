//! Manufacturings (ADR 0032 §4 to §8, ADR 0033): a top-level node that owns an output page sized
//! to the machine's work area, with live placements of drawing pages and installed fixtures as
//! its children. This module holds their reads and the commands that write them.

use loro::{LoroMap, TreeID, TreeParentId};
use serde::Serialize;

use crate::Point;
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

/// The label of Add manufacturing's commit.
pub const ADD_MANUFACTURING_COMMIT_LABEL: &str = "add_manufacturing";
/// The label of Delete manufacturing's commit.
pub const DELETE_MANUFACTURING_COMMIT_LABEL: &str = "delete_manufacturing";
/// The label of a work area edit's commit.
pub const SET_WORK_AREA_COMMIT_LABEL: &str = "set_work_area";
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
    /// Its name ("Laser 1").
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

/// What Add manufacturing creates.
#[derive(Debug, Clone, Copy)]
pub struct NewManufacturing<'a> {
    /// The process id.
    pub process: &'a str,
    /// The process name the new manufacturing's name starts with ("Laser").
    pub label: &'a str,
    /// The machine profile copy, written whole; `None` writes no copy.
    pub profile: Option<&'a FixtureParams>,
    /// The work area, the output page's size.
    pub work_area: DocumentSize,
    /// The drawing page placed at the output page's top-left corner, if any (criterion 5).
    pub place: Option<&'a str>,
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
            page: owned_page(&self.loro().get_map(ROOT_MAP), &page_id),
            placements,
            fixtures,
            process_values: crate::manufacturing_codec::process_values(&meta),
        })
    }

    /// Adds a manufacturing named "`label` n" with its owned output page of the work area's size
    /// and, with `place`, a placement of that drawing page at the top-left corner, scale 1,
    /// rotation 0, in one history step (criteria 2, 3, 5). The node goes to the end of the
    /// root, after every drawing node.
    ///
    /// # Errors
    /// [`ManufacturingError::Size`] for a work area outside the page size limits.
    ///
    /// # Panics
    /// Does not panic in practice: the tree operations act on the root and nodes just created.
    pub fn add_manufacturing(
        &self,
        new: NewManufacturing<'_>,
    ) -> Result<NodeId, ManufacturingError> {
        let width = validated_document_side(new.work_area.width)?;
        let height = validated_document_side(new.work_area.height)?;
        let taken: Vec<String> = self.manufacturings().into_iter().map(|m| m.name).collect();
        let name = next_numbered(new.label, &taken, false);
        let place = new
            .place
            .filter(|page| self.pages().iter().any(|p| p.id == *page));
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        // invariant: the root always accepts a new child.
        #[allow(clippy::unwrap_used)]
        let node = tree.create(TreeParentId::Root).unwrap();
        let page_id = format!("mfg-{}-{}", node.peer, node.counter);
        // invariant: the node was just created, so its meta map exists; plain values only.
        #[allow(clippy::unwrap_used)]
        {
            let meta = tree.get_meta(node).unwrap();
            meta.insert(KEY_MANUFACTURING, true).unwrap();
            meta.insert(KEY_NAME, name.as_str()).unwrap();
            meta.insert(KEY_PROCESS, new.process).unwrap();
            meta.insert(KEY_PAGE, page_id.as_str()).unwrap();
            if let Some(profile) = new.profile {
                write_plain_map(&meta, KEY_PROFILE, profile);
            }
        }
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, &page_id, "name", name.as_str());
        write(&root, &page_id, "order", 0.0);
        write_frame(
            &root,
            &page_id,
            Frame {
                origin: Point::new(0.0, 0.0),
                size: DocumentSize { width, height },
            },
        );
        write(&root, &page_id, "deleted", false);
        write(&root, &page_id, PAGE_OWNER_FIELD, owner_text(node));
        if let Some(page) = place {
            // invariant: the manufacturing node was just created and is live.
            #[allow(clippy::unwrap_used)]
            let placement = tree.create(TreeParentId::Node(node)).unwrap();
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(placement).unwrap();
            // invariant: a fresh meta map takes a plain value.
            #[allow(clippy::unwrap_used)]
            meta.insert(KEY_SHAPE, SHAPE_PLACEMENT).unwrap();
            write_placement_link(
                &meta,
                &PlacementLink {
                    page: page.to_owned(),
                    matrix: Affine::IDENTITY,
                },
            );
        }
        self.commit_with_label(ADD_MANUFACTURING_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Deletes the manufacturing `id` with its output page, placements and fixtures in one
    /// history step; the node goes in one tree delete, so an undo revives everything (criterion
    /// 13). The drawing is untouched. Allowed for an unknown process too (ADR 0033 §4).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `id` is not a live manufacturing.
    pub fn delete_manufacturing(&self, id: NodeId) -> Result<(), ManufacturingError> {
        let manufacturing = self.manufacturing(id).ok_or(ManufacturingError::NotFound)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let page = string(
            &tree
                .get_meta(tree_id_of(id))
                .map_err(|_| ManufacturingError::NotFound)?,
            KEY_PAGE,
        );
        tree.delete(tree_id_of(manufacturing.id))
            .map_err(|_| ManufacturingError::NotFound)?;
        // The rule of `specs/0023-groups` that every delete sweeps emptied groups; a
        // manufacturing sits at the root and holds no group, so this finds nothing of its own.
        crate::groups::remove_emptied_groups(&tree, &[None]);
        if let Some(page) = page {
            write(&self.loro().get_map(ROOT_MAP), &page, "deleted", true);
        }
        self.commit_with_label(DELETE_MANUFACTURING_COMMIT_LABEL);
        Ok(())
    }

    /// Sets the work area, the output page's size, in one history step; placed content keeps its
    /// place relative to the top-left corner, since it is page-local (criterion 3). `Ok(false)`
    /// and nothing written when the size is unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing or one without its page;
    /// [`ManufacturingError::Size`] for a side outside the page size limits.
    pub fn set_work_area(
        &self,
        id: NodeId,
        size: DocumentSize,
    ) -> Result<bool, ManufacturingError> {
        let manufacturing = self.manufacturing(id).ok_or(ManufacturingError::NotFound)?;
        let page = manufacturing.page.ok_or(ManufacturingError::NotFound)?;
        let width = validated_document_side(size.width)?;
        let height = validated_document_side(size.height)?;
        #[allow(clippy::float_cmp)] // exact register equality avoids an empty step
        if page.size.width.as_mm() == width.as_mm() && page.size.height.as_mm() == height.as_mm() {
            return Ok(false);
        }
        let root = self.loro().get_map(ROOT_MAP);
        write_frame(
            &root,
            &page.id,
            Frame {
                origin: Point::new(0.0, 0.0),
                size: DocumentSize { width, height },
            },
        );
        self.commit_with_label(SET_WORK_AREA_COMMIT_LABEL);
        Ok(true)
    }

    /// Writes the machine profile copy `profile` whole and sets the work area, the output page's
    /// size, in one history step labelled by `change` (ADR 0039 §4); the format-35 machine name
    /// goes in the same step. Placed content keeps its place relative to the top-left corner.
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
            write_frame(
                &self.loro().get_map(ROOT_MAP),
                &page.id,
                Frame {
                    origin: Point::new(0.0, 0.0),
                    size: DocumentSize { width, height },
                },
            );
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
