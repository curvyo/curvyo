//! Sketch copies preflight ownership before creating independent graph identities.
use crate::document::{OBJECTS_TREE, ROOT_MAP};
use crate::{CopySource, Document, NodeId, ObjectEditError, Vec2};
use loro::{LoroMap, TreeID};
use std::collections::HashMap;

impl Document {
    /// Copies artwork into a named page, refusing a second owning sketch atomically.
    /// # Errors
    /// Refuses an occupied/missing target page, multiple originals or invalid source geometry.
    pub fn duplicate_objects_to_page(
        &self,
        sources: &[CopySource],
        offset: Vec2,
        page: &str,
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        if !self.pages().iter().any(|p| p.id == page) {
            return Err(ObjectEditError::NoSuchObject);
        }
        self.duplicate_objects_in_page(sources, offset, &HashMap::new(), Some(page))
    }

    pub(crate) fn prepare_sketch_copies(
        &self,
        sources: &[&CopySource],
        page: Option<&str>,
    ) -> Result<HashMap<NodeId, String>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let selected: Vec<_> = sources
            .iter()
            .map(|s| TreeID::new(s.id.peer, s.id.counter))
            .collect();
        let mut claims = HashMap::new();
        for id in self.object_ids() {
            let node = TreeID::new(id.peer, id.counter);
            if !selected.contains(&node)
                && !crate::group_tree::has_ancestor_in(&tree, node, &selected)
            {
                continue;
            }
            let Ok(meta) = self.sketch_meta(id) else {
                continue;
            };
            crate::sketch_codec::read(&meta).map_err(|_| ObjectEditError::InvalidConversion)?;
            let Some(source_page) = self.sketch_page(id) else {
                continue;
            };
            let target = page.unwrap_or(&source_page);
            if !self.pages().iter().any(|p| p.id == target) {
                return Err(ObjectEditError::NoSuchObject);
            }
            if self.page_sketch(target).is_some() || claims.values().any(|p| p == target) {
                return Err(ObjectEditError::SketchPageOccupied);
            }
            claims.insert(id, target.to_owned());
        }
        Ok(claims)
    }

    pub(crate) fn assign_sketch_copies(
        &self,
        made: &[(TreeID, TreeID)],
        claims: &HashMap<NodeId, String>,
    ) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for &(source, copied) in made {
            let Some(page) = claims.get(&NodeId::from_parts(source.peer, source.counter)) else {
                continue;
            };
            // invariant: these nodes and page claims were created and preflighted by this command.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(copied).unwrap();
            #[allow(clippy::unwrap_used)]
            crate::sketch_codec::write(&meta, crate::sketch_page::PAGE_KEY, page).unwrap();
            #[allow(clippy::unwrap_used)]
            crate::sketch_codec::write(
                &self.loro().get_map(ROOT_MAP),
                &crate::pages::key(page, "sketch_owner"),
                &NodeId::from_parts(copied.peer, copied.counter),
            )
            .unwrap();
        }
    }
}

pub(crate) fn remap(meta: &LoroMap, id: NodeId) -> Result<(), String> {
    let namespace = id.full_id();
    crate::sketch_labels::remap(meta, &namespace)?;
    let (graph, _, checkpoint) = crate::sketch_codec::read(meta)?;
    crate::sketch_codec::write_graph(meta, &graph.with_fresh_namespace(&namespace))?;
    if let Some(mut saved) = checkpoint {
        saved.intent = saved.intent.with_fresh_namespace(&namespace);
        saved.candidate = saved.candidate.with_fresh_namespace(&namespace);
        saved.fingerprint = crate::sketch_codec::fingerprint(&saved.intent)?;
        saved.fully_constrained = saved.fully_constrained.map(|ids| {
            ids.into_iter()
                .map(|id| format!("{namespace}:{}", id.0).into())
                .collect()
        });
        saved.conflicting = saved
            .conflicting
            .into_iter()
            .map(|id| format!("{namespace}:{}", id.0).into())
            .collect();
        saved.redundant = saved
            .redundant
            .into_iter()
            .map(|id| format!("{namespace}:{}", id.0).into())
            .collect();
        crate::sketch_codec::write(meta, crate::sketch_codec::KEYS[5], &saved)?;
    }
    Ok(())
}
