//! Atomic sketch records encode intent independently of solution checkpoints.
use crate::{Affine, SketchCheckpoint, SketchEntityRecord};
use curvyo_sketch_core::{Parameters, Sketch};
use loro::{LoroMap, LoroValue, ValueOrContainer};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub(crate) const KEYS: [&str; 6] = [
    "sketch_origin",
    "sketch_entities",
    "sketch_constraints",
    "sketch_parameters",
    "sketch_placement",
    "sketch_checkpoint",
];

pub(crate) fn fingerprint(graph: &Sketch) -> Result<String, String> {
    let mut intent = graph.clone();
    intent.contact_seeds.clear();
    let bytes = serde_json::to_vec(&intent).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(b"curvyo-sketch-intent-v1\0");
    hash.update(bytes);
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn atomic<T: DeserializeOwned>(map: &LoroMap, key: &str) -> Result<T, String> {
    let Some(ValueOrContainer::Value(value)) = map.get(key) else {
        return Err(format!("missing atomic {key}"));
    };
    serde_json::from_value(serde_json::Value::from(value)).map_err(|e| format!("{key}: {e}"))
}
pub(crate) fn write<T: Serialize>(map: &LoroMap, key: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    map.insert(key, LoroValue::from(value))
        .map_err(|e| e.to_string())
}
pub(crate) fn collection(map: &LoroMap, key: &str) -> Result<LoroMap, String> {
    match map.get(key) {
        Some(ValueOrContainer::Container(loro::Container::Map(value))) => Ok(value),
        _ => Err(format!("missing collection {key}")),
    }
}
pub(crate) fn read(map: &LoroMap) -> Result<(Sketch, Affine, Option<SketchCheckpoint>), String> {
    if map.get(crate::sketch_page::PAGE_KEY).is_some() {
        let page: String = atomic(map, crate::sketch_page::PAGE_KEY)?;
        if page.is_empty() {
            return Err("invalid sketch page identity".into());
        }
    }
    crate::sketch_labels::read(map)?;
    let mut graph = Sketch::new(atomic(map, KEYS[0])?);
    graph.entities.clear();
    let entities = collection(map, KEYS[1])?;
    let mut failure = None;
    entities.for_each(
        |key, _| match atomic::<SketchEntityRecord>(&entities, key) {
            Ok(record) => {
                if record.construction {
                    graph.construction.insert(key.into());
                }
                graph.entities.insert(key.into(), record.entity);
            }
            Err(error) => failure = Some(error),
        },
    );
    let constraints = collection(map, KEYS[2])?;
    constraints.for_each(|key, _| match atomic(&constraints, key) {
        Ok(record) => {
            graph.constraints.insert(key.into(), record);
        }
        Err(error) => failure = Some(error),
    });
    let parameters = collection(map, KEYS[3])?;
    graph.parameters = Parameters::default();
    parameters.for_each(|key, _| match atomic(&parameters, key) {
        Ok(record) => {
            graph.parameters.0.insert(key.into(), record);
        }
        Err(error) => failure = Some(error),
    });
    if let Some(error) = failure {
        return Err(error);
    }
    curvyo_sketch_core::validate_records(&graph).map_err(|e| e.to_string())?;
    let placement = Affine::from_array(atomic(map, KEYS[4])?);
    if !placement.is_finite() || placement.inverse().is_none() {
        return Err("invalid sketch placement".into());
    }
    let checkpoint: Option<SketchCheckpoint> = if map.get(KEYS[5]).is_some() {
        Some(atomic(map, KEYS[5])?)
    } else {
        None
    };
    if let Some(checkpoint) = &checkpoint {
        if matches!(
            checkpoint.status,
            curvyo_sketch_core::SolveStatus::Conflicting
                | curvyo_sketch_core::SolveStatus::SolverFailure
        ) || checkpoint
            .conflicting
            .iter()
            .chain(&checkpoint.redundant)
            .any(|id| !checkpoint.intent.constraints.contains_key(id))
            || checkpoint.fully_constrained.as_ref().is_some_and(|ids| {
                ids.iter()
                    .any(|id| !checkpoint.intent.entities.contains_key(id))
            })
        {
            return Err("invalid checkpoint diagnostics".into());
        }
        if fingerprint(&checkpoint.intent)? != checkpoint.fingerprint {
            return Err("checkpoint fingerprint mismatch".into());
        }
        curvyo_sketch_core::validate_candidate(&checkpoint.intent, &checkpoint.candidate)
            .map_err(|e| e.to_string())?;
    }
    Ok((graph, placement, checkpoint))
}

pub(crate) fn write_graph(map: &LoroMap, graph: &Sketch) -> Result<(), String> {
    write(map, KEYS[0], &graph.origin)?;
    let entities = map
        .insert_container(KEYS[1], LoroMap::new())
        .map_err(|e| e.to_string())?;
    for (id, entity) in &graph.entities {
        write(
            &entities,
            &id.0,
            &SketchEntityRecord {
                entity: entity.clone(),
                construction: graph.construction.contains(id),
            },
        )?;
    }
    let constraints = map
        .insert_container(KEYS[2], LoroMap::new())
        .map_err(|e| e.to_string())?;
    for (id, constraint) in &graph.constraints {
        write(&constraints, &id.0, constraint)?;
    }
    let parameters = map
        .insert_container(KEYS[3], LoroMap::new())
        .map_err(|e| e.to_string())?;
    for (id, parameter) in &graph.parameters.0 {
        write(&parameters, &id.0, parameter)?;
    }
    Ok(())
}

pub(crate) fn map_placement(meta: &LoroMap, map: Affine) {
    if let Ok(value) = atomic::<[f64; 6]>(meta, KEYS[4]) {
        let placement = map.compose(Affine::from_array(value));
        // invariant: known attached map accepts a finite atomic placement value.
        #[allow(clippy::unwrap_used)]
        write(meta, KEYS[4], &placement.to_array()).unwrap();
    }
}

pub(crate) fn rigid(map: Affine) -> bool {
    const TOLERANCE: f64 = 1e-9;
    map.is_finite()
        && (map.a * map.a + map.b * map.b - 1.0).abs() <= TOLERANCE
        && (map.c * map.c + map.d * map.d - 1.0).abs() <= TOLERANCE
        && (map.a * map.c + map.b * map.d).abs() <= TOLERANCE
        && (map.determinant() - 1.0).abs() <= TOLERANCE
}
