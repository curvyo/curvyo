//! Replaces the job fixtures of one laser page by copies of another's (ADR 0059 §6, `specs/0113` criteria 35
//! to 37): Apply fixtures from page.

use std::collections::HashMap;

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::fixture_job_codec::{
    FixturePartKey, TARGET_PREFIX, is_job_meta, parse_target_key, target_key,
};
use crate::group_tree::node_id;
use crate::manufacturing::ManufacturingError;
use crate::manufacturing_codec::{SHAPE_FIXTURE_SLOT, has_shape};
use crate::manufacturing_fixture_sets::MAX_JOB_FIXTURES;
use crate::operation_codec::KEY_OP_OUTPUT;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of Apply fixtures from page.
pub const APPLY_PAGE_FIXTURES_COMMIT_LABEL: &str = "apply_page_fixtures";

impl Document {
    /// Replaces the job fixtures of the manufacturing `target` by copies of every job fixture of
    /// `source`, in one step "Apply fixtures from page": `target`'s own job fixtures are deleted
    /// (its sets stay), and each copy keeps its slots, jobs and every register (name, kind,
    /// values, matrix, Lock, eye, Active) as it is stored, with fresh ids, in the source's order,
    /// never mirrored and never renamed. Every job's Output is off and every tick names the
    /// copy's own part. Returns the copies' ids; `Ok` of none and no step when the source has no
    /// job fixture.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `target` or `source` is not a live manufacturing;
    /// [`ManufacturingError::TooManyJobFixtures`], changing nothing, when the source holds more
    /// than 64 job fixtures (ADR 0059 §8).
    pub fn apply_page_fixtures(
        &self,
        target: NodeId,
        source: NodeId,
    ) -> Result<Vec<NodeId>, ManufacturingError> {
        let held: Vec<TreeID> = self
            .manufacturing(target)
            .ok_or(ManufacturingError::NotFound)?
            .job_fixtures()
            .map(|fixture| tree_id_of(fixture.id))
            .collect();
        let sources: Vec<TreeID> = self
            .manufacturing(source)
            .ok_or(ManufacturingError::NotFound)?
            .job_fixtures()
            .map(|fixture| tree_id_of(fixture.id))
            .collect();
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        if sources.len() > MAX_JOB_FIXTURES {
            return Err(ManufacturingError::TooManyJobFixtures);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let mut copies = Vec::with_capacity(sources.len());
        for fixture in sources {
            copies.push(node_id(copy_fixture(&tree, fixture, tree_id_of(target))?));
        }
        for fixture in held {
            tree.delete(fixture)
                .map_err(|_| ManufacturingError::NoSuchFixture)?;
        }
        // The rule of `specs/0023-groups`: a fixture holds no group, so this finds nothing.
        crate::groups::remove_emptied_groups(&tree, &[None]);
        self.commit_with_label(APPLY_PAGE_FIXTURES_COMMIT_LABEL);
        Ok(copies)
    }
}

/// Copies, uncommitted, the fixture `fixture` with its slot and job children to the end of
/// `parent`; returns the copy.
fn copy_fixture(
    tree: &LoroTree,
    fixture: TreeID,
    parent: TreeID,
) -> Result<TreeID, ManufacturingError> {
    let copy = new_node(tree, parent)?;
    copy_registers(&meta_of(tree, fixture)?, &meta_of(tree, copy)?, |_, _| true);
    let mut ids = HashMap::from([(fixture, copy)]);
    let mut jobs = Vec::new();
    for child in tree
        .children(TreeParentId::Node(fixture))
        .unwrap_or_default()
    {
        let meta = meta_of(tree, child)?;
        if has_shape(&meta, SHAPE_FIXTURE_SLOT) {
            let slot = new_node(tree, copy)?;
            copy_registers(&meta, &meta_of(tree, slot)?, |_, _| true);
            ids.insert(child, slot);
        } else if is_job_meta(&meta) {
            jobs.push(meta);
        }
    }
    for meta in jobs {
        let job = new_node(tree, copy)?;
        let into = meta_of(tree, job)?;
        copy_registers(&meta, &into, |key, _| {
            !key.starts_with(TARGET_PREFIX) && key != KEY_OP_OUTPUT
        });
        write_ticks(&meta, &into, &ids);
        // invariant: a plain value under a known key on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        into.insert(KEY_OP_OUTPUT, false).unwrap();
    }
    Ok(copy)
}

/// The ticks of `from` that name a copied part, written under the copy's ids into `into`.
fn write_ticks(from: &LoroMap, into: &LoroMap, ids: &HashMap<TreeID, TreeID>) {
    let mut ticks = Vec::new();
    from.for_each(|key, value| {
        let Some((part, suffix)) = parse_target_key(key) else {
            return;
        };
        let (Some(copy), ValueOrContainer::Value(value)) = (ids.get(&part), value) else {
            return;
        };
        let key = target_key(&FixturePartKey {
            node: node_id(*copy),
            suffix: suffix.map(str::to_owned),
        });
        ticks.push((key, value));
    });
    for (key, value) in ticks {
        // invariant: a plain value under a fresh key on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        into.insert(&key, value).unwrap();
    }
}

/// Copies every plain register of `from` that `keep` accepts into `into`.
fn copy_registers(from: &LoroMap, into: &LoroMap, keep: impl Fn(&str, &LoroValue) -> bool) {
    let mut values = Vec::new();
    from.for_each(|key, value| {
        if let ValueOrContainer::Value(value) = value
            && keep(key, &value)
        {
            values.push((key.to_owned(), value));
        }
    });
    for (key, value) in values {
        // invariant: a plain value on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        into.insert(&key, value).unwrap();
    }
}

fn new_node(tree: &LoroTree, parent: TreeID) -> Result<TreeID, ManufacturingError> {
    tree.create(TreeParentId::Node(parent))
        .map_err(|_| ManufacturingError::NotFound)
}

fn meta_of(tree: &LoroTree, node: TreeID) -> Result<LoroMap, ManufacturingError> {
    tree.get_meta(node)
        .map_err(|_| ManufacturingError::NoSuchFixture)
}
