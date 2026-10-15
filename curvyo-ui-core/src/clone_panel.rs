//! What the panel says about linked clones (`specs/0049-linked-clone` criteria 21 to 23, 25 and
//! 56, `specs/0057-clone-overrides` criteria 1, 10, 26 and 42): the names of originals, the clone
//! block with its two override rows and the origin line, and the selections that Select original,
//! Select root original and Select clones make (view state, no commit).

use std::collections::HashSet;

use curvyo_document_core::{CloneSnapshot, MAX_CLONE_DEPTH, NodeId, ObjectRead, ObjectSnapshot};

use crate::style_scope::object_kind_name;

/// How an object is named in the panel: its kind and the six characters of its id
/// ("Path 3f9a1c"), and for a clone "Clone of " and the name of its direct original, so a
/// clone of a clone is "Clone of Clone of Path 3f9a1c" (at most eight times "Clone of ").
#[must_use]
pub fn object_name(read: &ObjectRead, id: NodeId) -> String {
    let mut prefix = String::new();
    let mut current = id;
    for _ in 0..=MAX_CLONE_DEPTH {
        match read.get(current) {
            Some(ObjectSnapshot::Clone(clone)) => {
                prefix.push_str("Clone of ");
                current = clone.chain.origin;
            }
            Some(object) => {
                return format!(
                    "{prefix}{} {}",
                    object_kind_name(object),
                    current.short_id()
                );
            }
            None => return format!("{prefix}Object {}", current.short_id()),
        }
    }
    format!("{prefix}…")
}

/// The common state of one override row over the selected clones (`0057` criteria 1 and 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideState {
    /// Every selected clone follows its original (placement) or shows its original's style.
    Original,
    /// Every selected clone has its own.
    Own,
    /// The selected clones differ: no segment is pressed.
    Mixed,
}

impl OverrideState {
    fn of(mut states: impl Iterator<Item = bool>) -> Self {
        let Some(first) = states.next() else {
            return Self::Original;
        };
        if states.all(|state| state == first) {
            if first { Self::Own } else { Self::Original }
        } else {
            Self::Mixed
        }
    }

    /// The code the host keys on: `"original"`, `"own"` or `"mixed"`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Own => "own",
            Self::Mixed => "mixed",
        }
    }
}

/// The words a selected clone's tag adds for its overrides (`0057` criterion 29): "own
/// placement", "own style" or "own placement and style"; `None` without any.
#[must_use]
pub fn overrides_words(clone: &CloneSnapshot) -> Option<&'static str> {
    match (clone.chain.own_placement, clone.chain.own_style) {
        (true, true) => Some("own placement and style"),
        (true, false) => Some("own placement"),
        (false, true) => Some("own style"),
        (false, false) => None,
    }
}

/// The second muted line of a clone's Layers tooltip (`0057` criterion 28): "Own placement, own
/// style", only the parts that apply; empty without overrides.
#[must_use]
pub fn overrides_line(clone: &CloneSnapshot) -> String {
    let parts: Vec<&str> = [
        clone.chain.own_placement.then_some("Own placement"),
        clone.chain.own_style.then_some("own style"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let line = parts.join(", ");
    let mut chars = line.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The clone block of the Style tab, for a selection that holds only clones (criteria 22, 56).
#[derive(Debug, Clone, PartialEq)]
pub struct CloneBlock {
    /// How many clones are selected.
    pub count: usize,
    /// The name of the first clone's direct original ("Path 3f9a1c").
    pub original: String,
    /// The depth of the deepest selected clone (criterion 56: the chain line from 2 on).
    pub deepest: usize,
    /// The name of the deepest selected clone's root original.
    pub root: String,
    /// The Placement row (`0057` criterion 1).
    pub placement: OverrideState,
    /// The Style row (`0057` criterion 10).
    pub style: OverrideState,
    /// The effective style of the first selected clone, for the greyed Stroke, Fill and Markers
    /// headers of a selection with the original's style (`0057` UX notes).
    pub summary: curvyo_document_core::Style,
}

/// The block for `selected`, or `None` unless every selected object is a clone.
#[must_use]
pub fn clone_block(read: &ObjectRead, selected: &[NodeId]) -> Option<CloneBlock> {
    let held: Vec<&ObjectSnapshot> = selected.iter().filter_map(|id| read.get(*id)).collect();
    let clones: Vec<&CloneSnapshot> = held
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(clone) => Some(clone),
            _ => None,
        })
        .collect();
    if clones.is_empty() || clones.len() != held.len() {
        return None;
    }
    let first = clones
        .iter()
        .min_by_key(|clone| read.index.get(&clone.id()))
        .copied()?;
    let deepest = clones.iter().max_by_key(|clone| clone.chain.depth)?;
    Some(CloneBlock {
        count: clones.len(),
        original: object_name(read, first.chain.origin),
        deepest: deepest.chain.depth,
        root: object_name(read, deepest.chain.root),
        placement: OverrideState::of(clones.iter().map(|clone| clone.chain.own_placement)),
        style: OverrideState::of(clones.iter().map(|clone| clone.chain.own_style)),
        summary: first.resolved.style.clone(),
    })
}

/// How many linked clones follow the selected objects directly, and how many of the selected
/// objects have one (criterion 23); 0 hides the line. Read from `clones_of`.
#[must_use]
pub fn followers_of(read: &ObjectRead, selected: &[NodeId]) -> (usize, usize) {
    let unique: HashSet<NodeId> = selected.iter().copied().collect();
    unique
        .iter()
        .filter_map(|id| read.clones_of.get(id).map(Vec::len))
        .fold((0, 0), |(clones, objects), n| {
            (clones + n, objects + usize::from(n > 0))
        })
}

/// Select original: the direct originals of the selected clones, each once, in stacking order
/// (criterion 25).
#[must_use]
pub fn select_originals(read: &ObjectRead, selected: &[NodeId]) -> Vec<NodeId> {
    in_stacking_order(
        read,
        selected_clones(read, selected)
            .map(|clone| clone.chain.origin)
            .collect(),
    )
}

/// Select root original: the root originals of the selected clones, each once (criterion 56).
#[must_use]
pub fn select_root_originals(read: &ObjectRead, selected: &[NodeId]) -> Vec<NodeId> {
    in_stacking_order(
        read,
        selected_clones(read, selected)
            .map(|clone| clone.chain.root)
            .collect(),
    )
}

/// Select clones: the clones that follow the selected objects directly, in stacking order
/// (criterion 25: for O it selects A, not B).
#[must_use]
pub fn select_clones(read: &ObjectRead, selected: &[NodeId]) -> Vec<NodeId> {
    let unique: HashSet<NodeId> = selected.iter().copied().collect();
    in_stacking_order(
        read,
        unique
            .iter()
            .filter_map(|id| read.clones_of.get(id))
            .flatten()
            .copied()
            .collect(),
    )
}

fn selected_clones<'a>(
    read: &'a ObjectRead,
    selected: &'a [NodeId],
) -> impl Iterator<Item = &'a CloneSnapshot> + 'a {
    selected.iter().filter_map(|id| match read.get(*id) {
        Some(ObjectSnapshot::Clone(clone)) => Some(clone),
        _ => None,
    })
}

/// `ids` once each, in the stacking order of the read.
fn in_stacking_order(read: &ObjectRead, ids: Vec<NodeId>) -> Vec<NodeId> {
    let unique: HashSet<NodeId> = ids.into_iter().collect();
    let mut placed: Vec<(usize, NodeId)> = unique
        .into_iter()
        .filter_map(|id| read.index.get(&id).map(|&at| (at, id)))
        .collect();
    placed.sort_unstable_by_key(|&(at, _)| at);
    placed.into_iter().map(|(_, id)| id).collect()
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point};

    use super::*;

    fn chain() -> (Document, NodeId, NodeId, NodeId) {
        let d = Document::new(1);
        let corner =
            |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
        let o = d.create_path(
            &[
                corner(0, 0.0, 0.0),
                corner(1, 10.0, 0.0),
                corner(2, 0.0, 10.0),
            ],
            true,
        );
        let a = d.linked_clone(&[o], &[]).unwrap()[0];
        let b = d.linked_clone(&[a], &[]).unwrap()[0];
        (d, o, a, b)
    }

    #[test]
    fn criterion_21_names_follow_the_chain() {
        let (d, o, a, b) = chain();
        let objects = d.read_objects();
        let path = format!("Path {}", o.short_id());
        assert_eq!(object_name(&objects, o), path);
        assert_eq!(object_name(&objects, a), format!("Clone of {path}"));
        assert_eq!(
            object_name(&objects, b),
            format!("Clone of Clone of {path}")
        );
    }

    #[test]
    fn criterion_56_the_block_of_a_chain_names_its_depth_and_root() {
        let (d, o, a, b) = chain();
        let objects = d.read_objects();
        let block = clone_block(&objects, &[b]).unwrap();
        assert_eq!(block.count, 1);
        assert_eq!(block.deepest, 2);
        assert_eq!(block.original, object_name(&objects, a));
        assert_eq!(block.root, object_name(&objects, o));
        assert_eq!(clone_block(&objects, &[a]).unwrap().deepest, 1);
        assert_eq!(clone_block(&objects, &[a, o]), None, "only clones");
    }

    #[test]
    fn criterion_25_select_original_root_and_clones() {
        let (d, o, a, b) = chain();
        let objects = d.read_objects();
        assert_eq!(select_originals(&objects, &[b]), vec![a]);
        assert_eq!(select_root_originals(&objects, &[b, a]), vec![o]);
        assert_eq!(select_clones(&objects, &[o]), vec![a]);
        assert_eq!(followers_of(&objects, &[o]), (1, 1));
        assert_eq!(followers_of(&objects, &[b]), (0, 0));
    }

    /// `specs/0057-clone-overrides` criteria 1, 10, 28 and 29: the rows' common state, mixed
    /// when the clones differ, and the words of the tag and the Layers line.
    #[test]
    fn the_override_rows_read_the_common_state() {
        let (d, _, a, b) = chain();
        let objects = d.read_objects();
        let block = clone_block(&objects, &[a, b]).unwrap();
        assert_eq!(
            (block.placement, block.style),
            (OverrideState::Original, OverrideState::Original)
        );
        d.set_own_placement(&[a], true).unwrap();
        d.set_own_style(&[a, b], true).unwrap();
        let objects = d.read_objects();
        let block = clone_block(&objects, &[a, b]).unwrap();
        assert_eq!(block.placement, OverrideState::Mixed);
        assert_eq!(block.style, OverrideState::Own);
        let ObjectSnapshot::Clone(clone_a) = objects.get(a).unwrap() else {
            panic!("a clone")
        };
        assert_eq!(overrides_words(clone_a), Some("own placement and style"));
        assert_eq!(overrides_line(clone_a), "Own placement, own style");
        let ObjectSnapshot::Clone(clone_b) = objects.get(b).unwrap() else {
            panic!("a clone")
        };
        assert_eq!(overrides_line(clone_b), "Own style");
    }
}
