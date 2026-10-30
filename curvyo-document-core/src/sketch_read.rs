//! Sketch reads validate checkpoints without writing document operations.
use crate::sketch_codec;
use crate::{NodeId, SketchSnapshot};
use loro::LoroMap;

#[cfg(test)]
thread_local! { static SOLVES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(crate) fn solve_count() -> usize {
    SOLVES.with(std::cell::Cell::get)
}

pub(crate) fn read(id: NodeId, meta: &LoroMap) -> Result<SketchSnapshot, String> {
    let (graph, placement, checkpoint) = sketch_codec::read(meta)?;
    let style = crate::style_codec::read_style(meta);
    let mut status = None;
    let mut fully_constrained = None;
    let mut conflicting = std::collections::BTreeSet::new();
    let mut redundant = std::collections::BTreeSet::new();
    let diagnosis;
    let mut outdated = false;
    #[cfg(test)]
    SOLVES.with(|count| count.set(count.get() + 1));
    let candidate = {
        let seed = seed_for(&graph, checkpoint.as_ref());
        match curvyo_sketch_core::solve_seeded(&graph, &seed, &curvyo_sketch_core::SolveMode::Edit)
        {
            Ok(result) => {
                status = Some(result.status);
                fully_constrained = result.fully_constrained;
                conflicting = result.conflicting;
                redundant = result.redundant;
                diagnosis = result.message;
                if let Some(candidate) = result.candidate {
                    candidate
                } else {
                    outdated = true;
                    checkpoint.as_ref().map_or_else(
                        || curvyo_sketch_core::Sketch::new("empty".into()),
                        |saved| saved.candidate.clone(),
                    )
                }
            }
            Err(error) => {
                diagnosis = Some(error.to_string());
                outdated = true;
                checkpoint.as_ref().map_or_else(
                    || curvyo_sketch_core::Sketch::new("empty".into()),
                    |saved| saved.candidate.clone(),
                )
            }
        }
    };
    let resolved = crate::sketch_outline::outline(id, &candidate, placement, &style)?;
    Ok(SketchSnapshot {
        id,
        graph,
        placement,
        style,
        resolved,
        solution: candidate,
        checkpoint,
        status,
        fully_constrained,
        conflicting,
        redundant,
        stale: outdated,
        diagnosis,
    })
}

/// A changed authoritative entity wins over coordinates from an older checkpoint.
pub(crate) fn seed_for(
    graph: &curvyo_sketch_core::Sketch,
    checkpoint: Option<&crate::SketchCheckpoint>,
) -> curvyo_sketch_core::Sketch {
    let Some(saved) = checkpoint else {
        return graph.clone();
    };
    let mut seed = saved.candidate.clone();
    for (id, entity) in &graph.entities {
        if saved.intent.entities.get(id) != Some(entity) {
            seed.entities.insert(id.clone(), entity.clone());
        }
    }
    seed
}
