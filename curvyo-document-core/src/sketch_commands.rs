//! Sketch commands commit explicit intent changes with a validated checkpoint.
use crate::sketch_codec::{self, KEYS};
use crate::{Affine, Document, NodeId, SketchCheckpoint, SketchDelta, SketchSnapshot, Style};
use curvyo_sketch_core::{Sketch, SolveMode};
use loro::{LoroMap, TreeID};

/// A sketch edit failed without accepting invalid geometry.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SketchEditError(pub String);
impl From<String> for SketchEditError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Document {
    /// Creates one parametric object and one history step.
    /// # Errors
    /// Rejects invalid placement, malformed intent or an unsatisfied graph.
    pub fn create_sketch(
        &self,
        graph: &Sketch,
        placement: Affine,
        style: &Style,
    ) -> Result<NodeId, SketchEditError> {
        if !sketch_codec::rigid(placement) {
            return Err(SketchEditError("invalid sketch placement".into()));
        }
        let checkpoint = checkpoint(graph, graph)?;
        let tree = self.loro().get_tree(crate::document::OBJECTS_TREE);
        let leaf = tree
            .create(self.drawing_parent(&tree))
            .map_err(|e| SketchEditError(e.to_string()))?;
        let meta = tree
            .get_meta(leaf)
            .map_err(|e| SketchEditError(e.to_string()))?;
        crate::shape_codec::write_shape_tag(&meta, "sketch");
        crate::style_codec::write_creation_style(&meta);
        crate::style_codec::write_changes(&meta, &crate::style_codec::read_style(&meta), style);
        sketch_codec::write_graph(&meta, graph)?;
        sketch_codec::write(&meta, KEYS[4], &placement.to_array())?;
        sketch_codec::write(&meta, KEYS[5], &checkpoint)?;
        self.commit_with_label("add_sketch");
        Ok(NodeId::from_parts(leaf.peer, leaf.counter))
    }

    /// Reads a sketch without changing history or stored solver records.
    #[must_use]
    pub fn sketch(&self, id: NodeId) -> Option<SketchSnapshot> {
        // Pending transactions bypass revision caching. In particular, never
        // publish a partially edited graph as the cached durable snapshot.
        let cacheable = self.loro.get_pending_txn_len() == 0;
        let version = self.version();
        if cacheable
            && let Some((revision, saved)) = self.sketch_cache.borrow().get(&id)
            && *revision == version
        {
            return Some(saved.clone());
        }
        let Ok(meta) = self.sketch_meta(id) else {
            self.sketch_cache.borrow_mut().remove(&id);
            return None;
        };
        let (graph, placement, checkpoint) = sketch_codec::read(&meta).ok()?;
        let reused = self.sketch_cache.borrow().get(&id).and_then(|(_, saved)| {
            if saved.graph != graph || saved.checkpoint != checkpoint {
                return None;
            }
            let mut view = saved.mapped_by(placement.compose(saved.placement.inverse()?));
            view.style = crate::style_codec::read_style(&meta);
            view.resolved.style = view.style.clone();
            Some(view)
        });
        let snapshot = reused.or_else(|| crate::sketch_read::read(id, &meta).ok())?;
        if cacheable {
            self.sketch_cache
                .borrow_mut()
                .insert(id, (version, snapshot.clone()));
        }
        Some(snapshot)
    }

    /// Applies only explicitly listed intent records in one transaction.
    /// # Errors
    /// Rejects missing objects, invalid references or unsatisfied constraints.
    pub fn apply_sketch_delta(
        &self,
        id: NodeId,
        delta: &SketchDelta,
    ) -> Result<bool, SketchEditError> {
        self.apply_sketch_change(id, delta, None)
    }

    /// Commits an explicit intent delta from a validated live drag pose.
    /// Dependent positions are checkpoint-only; unlisted entity records stay untouched.
    /// # Errors
    /// Rejects a preview that changes intent or violates the resulting hard constraints.
    pub fn apply_sketch_preview(
        &self,
        id: NodeId,
        delta: &SketchDelta,
        preview: &Sketch,
    ) -> Result<bool, SketchEditError> {
        self.apply_sketch_change(id, delta, Some(preview))
    }

    fn apply_sketch_change(
        &self,
        id: NodeId,
        delta: &SketchDelta,
        preview: Option<&Sketch>,
    ) -> Result<bool, SketchEditError> {
        let meta = self.sketch_meta(id)?;
        let (mut graph, _, saved) = sketch_codec::read(&meta)?;
        let before = graph.clone();
        let mut seed = crate::sketch_read::seed_for(&graph, saved.as_ref());
        apply(&mut graph, delta);
        if graph == before {
            return Ok(false);
        }
        apply(&mut seed, delta);
        if let Err(error) = graph.validate()
            && !crate::sketch_parameter_repair::allowed(&before, &graph, delta)
        {
            return Err(SketchEditError(error.to_string()));
        }
        if let Some(preview) = preview {
            curvyo_sketch_core::validate_candidate(&graph, preview)
                .map_err(|error| SketchEditError(error.to_string()))?;
            seed = preview.clone();
        }
        // Well-formed contradictory intent remains editable; the old checkpoint stays visible.
        let checkpoint = checkpoint(&graph, &seed).ok();
        let entities = sketch_codec::collection(&meta, KEYS[1])?;
        let constraints = sketch_codec::collection(&meta, KEYS[2])?;
        let parameters = sketch_codec::collection(&meta, KEYS[3])?;
        for (id, value) in &delta.entities {
            change(&entities, &id.0, value.as_ref())?;
        }
        for (id, value) in &delta.constraints {
            change(&constraints, &id.0, value.as_ref())?;
        }
        for (id, value) in &delta.parameters {
            change(&parameters, &id.0, value.as_ref())?;
        }
        if let Some(checkpoint) = checkpoint {
            sketch_codec::write(&meta, KEYS[5], &checkpoint)?;
        }
        self.commit_with_label("edit_sketch");
        Ok(true)
    }
    pub(crate) fn sketch_meta(&self, id: NodeId) -> Result<LoroMap, SketchEditError> {
        let tree = self.loro().get_tree(crate::document::OBJECTS_TREE);
        let leaf = TreeID::new(id.peer, id.counter);
        if !crate::path_codec::leaf_exists(&tree, leaf) {
            return Err(SketchEditError("missing sketch".into()));
        }
        let meta = tree
            .get_meta(leaf)
            .map_err(|e| SketchEditError(e.to_string()))?;
        if crate::shape_codec::read_shape_tag(&meta).as_deref() != Some("sketch") {
            return Err(SketchEditError("not a sketch".into()));
        }
        Ok(meta)
    }
}
fn checkpoint(graph: &Sketch, seed: &Sketch) -> Result<SketchCheckpoint, SketchEditError> {
    let result = curvyo_sketch_core::solve_seeded(graph, seed, &SolveMode::Edit)
        .map_err(|e| SketchEditError(e.to_string()))?;
    let candidate = result.candidate.ok_or_else(|| {
        SketchEditError(
            result
                .message
                .unwrap_or_else(|| "unsatisfied sketch".into()),
        )
    })?;
    let mut intent = graph.clone();
    intent.contact_seeds.clear();
    Ok(SketchCheckpoint {
        fingerprint: sketch_codec::fingerprint(&intent)?,
        intent,
        candidate,
        fully_constrained: result.fully_constrained,
        conflicting: result.conflicting,
        redundant: result.redundant,
        status: result.status,
    })
}
fn apply(graph: &mut Sketch, delta: &SketchDelta) {
    for (id, record) in &delta.entities {
        if let Some(record) = record {
            graph.entities.insert(id.clone(), record.entity.clone());
            if record.construction {
                graph.construction.insert(id.clone());
            } else {
                graph.construction.remove(id);
            }
        } else {
            graph.entities.remove(id);
            graph.construction.remove(id);
        }
    }
    for (id, record) in &delta.constraints {
        if let Some(record) = record {
            graph.constraints.insert(id.clone(), record.clone());
        } else {
            graph.constraints.remove(id);
            graph.contact_seeds.remove(id);
        }
    }
    for (id, record) in &delta.parameters {
        if let Some(record) = record {
            graph.parameters.0.insert(id.clone(), record.clone());
        } else {
            graph.parameters.0.remove(id);
        }
    }
}
fn change<T: serde::Serialize>(map: &LoroMap, key: &str, value: Option<&T>) -> Result<(), String> {
    if let Some(value) = value {
        sketch_codec::write(map, key, value)
    } else {
        map.delete(key).map_err(|e| e.to_string())
    }
}

impl Document {
    /// Explicitly replaces a sketch by its currently visible compound path.
    /// The caller supplies fresh anchor IDs, as for other path creation commands.
    /// # Errors
    /// Rejects missing sketches or duplicate/incorrect anchor identities.
    pub fn convert_sketch_to_path(
        &self,
        id: NodeId,
        ids: &[crate::AnchorId],
    ) -> Result<(), SketchEditError> {
        let snapshot = self
            .sketch(id)
            .ok_or_else(|| SketchEditError("missing sketch".into()))?;
        let mut path = snapshot.resolved;
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len() || ids.len() != path.all_anchors().count() {
            return Err(SketchEditError("conversion needs fresh anchor IDs".into()));
        }
        if self
            .read_objects()
            .objects
            .iter()
            .filter_map(|o| match o {
                crate::ObjectSnapshot::Path(path) => Some(path),
                _ => None,
            })
            .any(|path| path.all_anchors().any(|a| unique.contains(&a.id)))
        {
            return Err(SketchEditError("anchor ID already used".into()));
        }
        for (anchor, id) in path.all_anchors_mut().zip(ids) {
            anchor.id = *id;
        }
        let meta = self.sketch_meta(id)?;
        let opacity = crate::node_registers::read_registers(&meta).opacity;
        crate::clone_write::unlink_meta(&meta, &path, opacity.get());
        for key in KEYS {
            meta.delete(key)
                .map_err(|e| SketchEditError(e.to_string()))?;
        }
        crate::path_codec::write_rotation(&meta, path.rotation);
        self.commit_with_label("convert_sketch_to_path");
        Ok(())
    }
}
