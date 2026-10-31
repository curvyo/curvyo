//! Dimension label positions remain presentation records outside solver intent.
use crate::{Document, NodeId, SketchEditError, sketch_codec};
use curvyo_sketch_core::{ConstraintId, Position};
use loro::LoroMap;
use std::collections::BTreeMap;
pub(crate) const KEY: &str = "sketch_labels";

pub(crate) fn read(meta: &LoroMap) -> Result<BTreeMap<ConstraintId, Position>, String> {
    if meta.get(KEY).is_none() {
        return Ok(BTreeMap::new());
    }
    let map = sketch_codec::collection(meta, KEY)?;
    let mut result = BTreeMap::new();
    let mut error = None;
    map.for_each(|key, _| match sketch_codec::atomic::<Position>(&map, key) {
        Ok(point) if point.x.0.is_finite() && point.y.0.is_finite() && !key.is_empty() => {
            result.insert(key.into(), point);
        }
        _ => error = Some("invalid sketch label position".into()),
    });
    error.map_or(Ok(result), Err)
}
impl Document {
    /// Places an existing dimension label in sketch-local millimetres.
    /// # Errors
    /// Refuses nonfinite positions, missing dimensions or a competing page owner.
    pub fn set_sketch_label(
        &self,
        id: NodeId,
        constraint: &ConstraintId,
        position: Position,
    ) -> Result<bool, SketchEditError> {
        if !position.x.0.is_finite()
            || !position.y.0.is_finite()
            || self.sketch_ownership_conflict(id)
        {
            return Err(SketchEditError(
                "Invalid sketch label target or position".into(),
            ));
        }
        let meta = self.sketch_meta(id)?;
        let (graph, _, _) = sketch_codec::read(&meta)?;
        if graph
            .constraints
            .get(constraint)
            .is_none_or(|c| c.relation.expression().is_none())
        {
            return Err(SketchEditError("Dimension no longer exists".into()));
        }
        if read(&meta)?.get(constraint) == Some(&position) {
            return Ok(false);
        }
        let labels = if meta.get(KEY).is_some() {
            sketch_codec::collection(&meta, KEY)?
        } else {
            meta.insert_container(KEY, LoroMap::new())
                .map_err(|e| SketchEditError(e.to_string()))?
        };
        sketch_codec::write(&labels, &constraint.0, &position)?;
        self.commit_with_label("edit_sketch");
        Ok(true)
    }
}

pub(crate) fn remove(meta: &LoroMap, constraint: &ConstraintId) -> Result<(), String> {
    if meta.get(KEY).is_some() {
        sketch_codec::collection(meta, KEY)?
            .delete(&constraint.0)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn remap(meta: &LoroMap, namespace: &str) -> Result<(), String> {
    let records = read(meta)?;
    if meta.get(KEY).is_none() {
        return Ok(());
    }
    let map = meta
        .insert_container(KEY, LoroMap::new())
        .map_err(|e| e.to_string())?;
    for (id, point) in records {
        sketch_codec::write(&map, &format!("{namespace}:{}", id.0), &point)?;
    }
    Ok(())
}
