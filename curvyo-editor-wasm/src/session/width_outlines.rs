//! Memoises the variable-width artwork of each path on its exact source snapshot, computed in
//! the session when it is first asked for (ADR 0053, amendment of 2026-11-10, item 6).

use curvyo_document_core::{CurveKind, NodeId, ObjectSnapshot, PathSnapshot, Tolerance};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A path's fill without its stroke, and the filled outline of its variable stroke.
pub(super) type Artwork = (PathSnapshot, Option<PathSnapshot>);

/// How many results each path id keeps, least recently used first out: a drag shows the moved
/// path while the original is still asked for (snapping, the ghost), and every linked clone of a
/// group draws the group's leaves under their own ids, so a few clones of one group keep theirs
/// without recomputing every frame.
const KEPT_PER_PATH: usize = 8;

/// The tolerance of the canvas, hit testing and export, millimetres.
const TOLERANCE_MM: f64 = 0.01;

/// One result and the exact input it was made for, compared by value.
#[derive(Debug)]
struct Entry {
    source: PathSnapshot,
    tolerance_bits: u64,
    result: Option<Arc<Artwork>>,
}

/// The memo: the latest results per path, newest last.
#[derive(Debug, Default)]
pub(super) struct WidthOutlines {
    entries: HashMap<NodeId, Vec<Entry>>,
}

impl WidthOutlines {
    /// The artwork of `source` at `tolerance_mm`, computed now unless memoised; `None` when the
    /// outline is refused (degenerate input).
    pub fn get(&mut self, source: &PathSnapshot, tolerance_mm: f64) -> Option<Arc<Artwork>> {
        let bits = tolerance_mm.to_bits();
        let kept = self.entries.entry(source.id).or_default();
        if let Some(at) = kept
            .iter()
            .position(|entry| entry.tolerance_bits == bits && entry.source == *source)
        {
            let entry = kept.remove(at);
            let result = entry.result.clone();
            kept.push(entry);
            return result;
        }
        let result = curvyo_ui_core::verified_variable_stroke_artwork_with_tolerance(
            source,
            Tolerance::from_mm(tolerance_mm),
        )
        .ok()
        .map(Arc::new);
        if kept.len() >= KEPT_PER_PATH {
            kept.remove(0);
        }
        kept.push(Entry {
            source: source.clone(),
            tolerance_bits: bits,
            result: result.clone(),
        });
        result
    }

    /// Forgets every path not in `ids`.
    pub fn retain(&mut self, ids: &HashSet<NodeId>) {
        self.entries.retain(|id, _| ids.contains(id));
    }
}

impl super::Session {
    /// The ids of the selection's objects and of the leaves of its units.
    pub(super) fn transform_selection_ids(&self) -> HashSet<NodeId> {
        let mut ids: HashSet<_> = self.selection.ids().iter().copied().collect();
        ids.extend(
            self.units()
                .selected(self.selection.ids())
                .into_iter()
                .flat_map(|unit| unit.leaves),
        );
        ids
    }

    /// The artwork of the drawn `path` (a Spiro evaluated first), at the canvas tolerance;
    /// `None` when the outline is refused.
    pub(super) fn width_artwork(&self, path: &PathSnapshot) -> Option<Arc<Artwork>> {
        if path.curve_kind != CurveKind::Spiro {
            return self.width_outlines.borrow_mut().get(path, TOLERANCE_MM);
        }
        let evaluated = self.spiro_selection_objects(&[ObjectSnapshot::Path(path.clone())]);
        let evaluated = evaluated.first()?.drawn_path()?;
        if evaluated.curve_kind == CurveKind::Spiro {
            return None;
        }
        self.width_outlines
            .borrow_mut()
            .get(evaluated, TOLERANCE_MM)
    }

    /// The artwork of `source` at `tolerance_mm`, for a placement drawn through a scaling map.
    pub(super) fn width_artwork_at(
        &self,
        source: &PathSnapshot,
        tolerance_mm: f64,
    ) -> Option<Arc<Artwork>> {
        self.width_outlines.borrow_mut().get(source, tolerance_mm)
    }

    /// Whether every variable stroke among `objects` has an outline: a transform that would
    /// leave one without is refused.
    pub(super) fn width_outlines_exist(&self, objects: &[ObjectSnapshot]) -> bool {
        objects.iter().all(|object| {
            curvyo_ui_core::profiled_paths(object)
                .into_iter()
                .all(|path| self.width_artwork(path).is_some())
        })
    }

    /// The artwork of every variable stroke in the document read, by path id.
    pub(super) fn frozen_width_artwork(&self) -> HashMap<NodeId, (PathSnapshot, Arc<Artwork>)> {
        let read = self.document_read();
        read.clones
            .objects
            .iter()
            .filter_map(|object| {
                let path = object.drawn_path()?;
                if path.width_points.is_empty() {
                    return None;
                }
                Some((path.id, (path.clone(), self.width_artwork(path)?)))
            })
            .collect()
    }

    /// The oriented bounds of `object`, a variable stroke by its outline.
    pub(super) fn cached_oriented_bounds(
        &self,
        object: &ObjectSnapshot,
    ) -> Option<curvyo_ui_core::OrientedBox> {
        curvyo_ui_core::oriented_bounds_with_artwork(object, &|path| self.width_artwork(path))
    }
}

/// `object` as it is drawn with its variable strokes filled, pushed onto `out`: a profiled path as
/// its fill and the outline of its stroke, a clone of a group, text or image with each profiled
/// leaf so (ADR 0045 §3), anything else as it is. Returns whether a stroke had no outline: its
/// path's fill is drawn alone then (ADR 0053 amendment item 5), never a stand-in stroke.
pub(super) fn push_width_expanded(
    object: &ObjectSnapshot,
    artwork: &dyn Fn(&PathSnapshot) -> Option<Arc<Artwork>>,
    out: &mut Vec<ObjectSnapshot>,
) -> bool {
    if let ObjectSnapshot::Clone(clone) = object
        && let Some(picture) = clone.chain.picture.as_deref()
        && !curvyo_ui_core::profiled_paths(object).is_empty()
    {
        let mut leaves = Vec::with_capacity(picture.len() + 1);
        let mut unsupported = false;
        for leaf in picture {
            unsupported |= push_width_expanded(leaf, artwork, &mut leaves);
        }
        let mut expanded = clone.clone();
        expanded.chain.picture = Some(leaves.into());
        out.push(ObjectSnapshot::Clone(expanded));
        return unsupported;
    }
    let Some(path) = object
        .drawn_path()
        .filter(|path| !path.width_points.is_empty())
    else {
        out.push(object.clone());
        return false;
    };
    if let Some(computed) = artwork(path) {
        let (fill, outline) = computed.as_ref();
        out.push(ObjectSnapshot::Path(fill.clone()));
        if let Some(outline) = outline {
            out.push(ObjectSnapshot::Path(outline.clone()));
        }
        return false;
    }
    let mut fill = path.clone();
    fill.style.stroke.enabled = false;
    fill.width_points.clear();
    out.push(ObjectSnapshot::Path(fill));
    true
}

/// The artwork lookup over the memo alone, for calls that also borrow other parts of the
/// session; its callers pass objects whose Spiro paths are evaluated already.
pub(super) fn memo_lookup(
    outlines: &std::cell::RefCell<WidthOutlines>,
) -> impl Fn(&PathSnapshot) -> Option<Arc<Artwork>> + '_ {
    move |path| outlines.borrow_mut().get(path, TOLERANCE_MM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point};

    /// One profiled line under one id, shifted by `dy`: what a group and two of its clones draw.
    fn shifted(dy: f64) -> PathSnapshot {
        let doc = Document::new(5);
        let id = doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(5, 1), Point::new(0.0, dy)),
                NewAnchor::corner(AnchorId::new(5, 2), Point::new(30.0, dy)),
            ],
            false,
        );
        doc.set_width_point(id, AnchorId::new(5, 3), 0.5, 3.0)
            .unwrap();
        doc.path(id).unwrap()
    }

    #[test]
    fn a_group_and_two_clones_of_it_keep_their_outlines_across_frames() {
        let sources = [shifted(0.0), shifted(50.0), shifted(100.0)];
        assert!(sources.iter().all(|s| s.id == sources[0].id), "one id");
        let mut memo = WidthOutlines::default();
        let first: Vec<_> = sources
            .iter()
            .map(|s| memo.get(s, TOLERANCE_MM).unwrap())
            .collect();
        for _frame in 0..3 {
            for (source, kept) in sources.iter().zip(&first) {
                let again = memo.get(source, TOLERANCE_MM).unwrap();
                assert!(Arc::ptr_eq(&again, kept), "memoised, not recomputed");
            }
        }
    }
}
