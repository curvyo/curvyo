//! The three commands that grow one open outline of a path: extend it, connect it to an open
//! outline of another path, close it (`specs/0034-pen-path-extension`; on compound paths
//! `specs/0055-compound-path-node-editing` criteria 7 to 9). Each takes one resolved
//! [`PathGrowth`] and runs one private funnel that resolves and checks everything before its first
//! write, writes the grown outline's anchor list, and ends in exactly one commit labelled
//! `extend_path`, `connect_paths` or `close_path` (ADR 0002 §9). The survivor's id, style, place in
//! the stacking order, its other outlines and the ids of its existing nodes are never written.

use std::collections::HashSet;

use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::{
    KEY_HANDLE_IN, KEY_HANDLE_OUT, KEY_POINT, anchor_index, anchor_map_at, insert_anchor_at,
    read_vec2, write_closed, write_kind, write_point, write_vec2,
};
use crate::path_model::{AnchorId, AnchorSnapshot, NewAnchor, NodeId, PathEditError, PathSnapshot};
use crate::path_reverse::reversed_anchors;
use crate::subpath_codec::{anchor_positions, append_outlines};

/// One end of an open outline's anchor list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathEnd {
    /// The first anchor; growth here is placed before the existing anchors.
    First,
    /// The last anchor; growth here is placed after the existing anchors.
    Last,
}

/// What one command does to one path, resolved by the caller (ADR 0002: commands carry resolved
/// geometry, never geometric intent).
#[derive(Debug, Clone, PartialEq)]
pub struct PathGrowth {
    /// The surviving object: it keeps its id, style, place and the ids of its existing anchors.
    pub path: NodeId,
    /// The end of the grown outline where `added` and `absorb` attach.
    pub end: PathEnd,
    /// The node of `path` at `end` of the outline that grows: it names that outline, since a
    /// compound path has several (`specs/0055-compound-path-node-editing` criterion 8). The
    /// outline must be open and this node its first (`PathEnd::First`) or last (`PathEnd::Last`)
    /// node.
    pub at: AnchorId,
    /// New anchors in stored order. At the first end they are inserted before every existing
    /// anchor, so the caller passes them already in the order they are to be stored (the node
    /// drawn last first).
    pub added: Vec<NewAnchor>,
    /// For a connect: the other path, the end of its outline that meets `path`, and the node at
    /// that end (which names the outline, as `at` does). That outline's anchors follow `added` (at
    /// the first end: precede it), reversed when both joined ends are the same kind of end, so the
    /// two ends are adjacent. The other path's other outlines are appended to `path` in their
    /// order (`0055` criterion 9) and the other object is removed.
    pub absorb: Option<(NodeId, PathEnd, AnchorId)>,
    /// Resolved values of existing anchors of `path` or of the absorbed path that change (a
    /// merged junction, a closing node with its join applied). Only the fields that differ are
    /// written.
    pub replace: Vec<AnchorSnapshot>,
    /// Existing anchors of `path` or of the absorbed path that are removed (the end a merge
    /// replaces).
    pub drop: Vec<AnchorId>,
}

/// A growth that passed every check: the survivor's snapshot, the absorbed path's final anchors
/// in the order they are inserted and its other outlines, and whether the result closes.
struct Plan {
    growth: PathGrowth,
    survivor: PathSnapshot,
    /// The number (storage order) of the survivor's outline that grows.
    outline: usize,
    absorbed: Option<Absorbed>,
    close: bool,
}

/// What a connect takes from the other path.
struct Absorbed {
    path: NodeId,
    /// The joined outline's anchors, in the order they are inserted.
    anchors: Vec<NewAnchor>,
    /// The other path's other outlines, appended to the survivor in their order.
    others: Vec<(Vec<NewAnchor>, bool)>,
}

impl Document {
    /// Adds `growth.added` at `growth.end` of the open outline that ends at `growth.at` as one
    /// commit labelled `extend_path` (`0034` criteria 4 and 5, `0055` criterion 8). Refuses
    /// `absorb`.
    ///
    /// # Errors
    /// [`PathEditError::NoSuchPath`] / [`PathEditError::NoSuchAnchor`] if a path or an anchor
    /// named is gone; [`PathEditError::NotExtendable`] if the outline is closed, `at` is not its
    /// node at `end`, or the growth absorbs another; [`PathEditError::DuplicateAnchorId`] if an
    /// added id is used twice.
    pub fn extend_path(&self, growth: &PathGrowth) -> Result<(), PathEditError> {
        if growth.absorb.is_some() {
            return Err(PathEditError::NotExtendable);
        }
        let plan = self.plan_growth(growth, false)?;
        self.apply_growth(&plan);
        self.commit_with_label("extend_path");
        Ok(())
    }

    /// Grows `growth.path` at `growth.end`, then absorbs the other path at its joined end and
    /// removes it, as one commit labelled `connect_paths` (`0034` criteria 8 to 11). `absorb` may
    /// be left out: a new path that ends on another one becomes that other one.
    ///
    /// # Errors
    /// As [`Document::extend_path`]; [`PathEditError::NotExtendable`] also if the absorbed path
    /// is the survivor itself, or its outline is closed or does not end at the named node.
    pub fn connect_paths(&self, growth: &PathGrowth) -> Result<(), PathEditError> {
        let plan = self.plan_growth(growth, false)?;
        self.apply_growth(&plan);
        self.commit_with_label("connect_paths");
        Ok(())
    }

    /// Grows each path (with its `added`, if any) and closes it, all in one commit labelled
    /// `close_path` (`0034` criteria 14 and 19). Every growth is checked before the first write,
    /// so a refusal changes nothing.
    ///
    /// # Errors
    /// As [`Document::extend_path`]; [`PathEditError::TooFewToClose`] if a result would have
    /// fewer than three anchors; [`PathEditError::NotExtendable`] also if two growths name the
    /// same outline.
    pub fn close_paths(&self, growths: &[PathGrowth]) -> Result<(), PathEditError> {
        let plans = growths
            .iter()
            .map(|growth| self.plan_growth(growth, true))
            .collect::<Result<Vec<_>, _>>()?;
        // One growth per outline: a second one would name a node the first may drop or move
        // (`specs/0055-compound-path-node-editing` criterion 7).
        let mut grown = HashSet::new();
        if !plans
            .iter()
            .all(|plan| grown.insert((plan.growth.path, plan.outline)))
        {
            return Err(PathEditError::NotExtendable);
        }
        for plan in &plans {
            self.apply_growth(plan);
        }
        self.commit_with_label("close_path");
        Ok(())
    }

    /// Resolves and checks one growth without writing anything.
    fn plan_growth(&self, growth: &PathGrowth, close: bool) -> Result<Plan, PathEditError> {
        let survivor = self.path(growth.path).ok_or(PathEditError::NoSuchPath)?;
        let (outline, grown) = open_outline_at(&survivor, growth.end, growth.at)?;
        let absorbed_snapshot = match growth.absorb {
            Some((other, end, at)) => {
                if other == growth.path {
                    return Err(PathEditError::NotExtendable);
                }
                let snapshot = self.path(other).ok_or(PathEditError::NoSuchPath)?;
                let outline = outline_number(&snapshot, end, at)?;
                Some((other, end, snapshot, outline))
            }
            None => None,
        };
        check_ids(
            growth,
            &survivor,
            absorbed_snapshot.as_ref().map(|(_, _, s, _)| s),
        )?;
        let absorbed = absorbed_snapshot.map(|(other, end, snapshot, number)| {
            let mut outlines = snapshot.subpaths().enumerate();
            let mut anchors = Vec::new();
            let mut others = Vec::new();
            for (index, outline) in &mut outlines {
                if index == number {
                    anchors = outline
                        .anchors
                        .iter()
                        .filter(|anchor| !growth.drop.contains(&anchor.id))
                        .map(|anchor| resolved(anchor, &growth.replace))
                        .collect();
                } else {
                    others.push((outline.anchors.to_vec(), outline.closed));
                }
            }
            if end == growth.end {
                anchors = reversed_anchors(&anchors);
            }
            Absorbed {
                path: other,
                anchors,
                others,
            }
        });
        let kept = grown
            .iter()
            .filter(|anchor| !growth.drop.contains(&anchor.id))
            .count();
        let total = kept + growth.added.len() + absorbed.as_ref().map_or(0, |a| a.anchors.len());
        if close && total < 3 {
            return Err(PathEditError::TooFewToClose);
        }
        Ok(Plan {
            growth: growth.clone(),
            survivor,
            outline,
            absorbed,
            close,
        })
    }

    /// Writes a checked plan: no commit.
    fn apply_growth(&self, plan: &Plan) {
        let growth = &plan.growth;
        // invariant: `plan_growth` resolved this path and the outline that ends at `at` moments
        // ago, and `close_paths` refuses a second plan for the same outline, so the plans before
        // this one in the same call changed other outlines only.
        #[allow(clippy::unwrap_used)]
        let (meta, _) = self.path_parts(growth.path).unwrap();
        #[allow(clippy::unwrap_used)]
        let outline = anchor_positions(&meta).at(growth.at).unwrap();
        let anchors = outline.list;
        for new in &growth.replace {
            if let Some(old) = plan.survivor.anchor(new.id)
                && let Ok(index) = anchor_index(&anchors, new.id)
            {
                write_changed(&anchor_map_at(&anchors, index), old, new);
            }
        }
        let mut doomed: Vec<usize> = growth
            .drop
            .iter()
            .filter_map(|&id| anchor_index(&anchors, id).ok())
            .collect();
        doomed.sort_unstable();
        for index in doomed.into_iter().rev() {
            // invariant: the index was just found in this same list.
            #[allow(clippy::unwrap_used)]
            anchors.delete(index, 1).unwrap();
        }
        let absorbed: &[NewAnchor] = plan.absorbed.as_ref().map_or(&[], |a| &a.anchors);
        match growth.end {
            PathEnd::Last => {
                let at = anchors.len();
                for (offset, anchor) in growth.added.iter().chain(absorbed).enumerate() {
                    insert_anchor_at(&anchors, at + offset, anchor);
                }
            }
            PathEnd::First => {
                for (offset, anchor) in absorbed.iter().chain(&growth.added).enumerate() {
                    insert_anchor_at(&anchors, offset, anchor);
                }
            }
        }
        if let Some(absorbed) = &plan.absorbed {
            append_outlines(&meta, &absorbed.others);
            let tree = self.loro().get_tree(OBJECTS_TREE);
            let other = crate::paths::tree_id_of(absorbed.path);
            self.repair_clones_of_removed(&tree, &[other]);
            let parents = crate::group_tree::parents_of(&tree, &[other]);
            // invariant: `plan_growth` resolved this object moments ago.
            #[allow(clippy::unwrap_used)]
            tree.delete(other).unwrap();
            crate::groups::remove_emptied_groups(&tree, &parents);
        }
        if plan.close {
            write_closed(&outline.map, true);
        }
    }
}

/// The number (storage order) of the open outline of `path` whose node at `end` is `at`.
///
/// # Errors
/// [`PathEditError::NoSuchAnchor`] if no outline holds `at`; [`PathEditError::NotExtendable`] if
/// its outline is closed or `at` is not its node at `end`.
fn outline_number(path: &PathSnapshot, end: PathEnd, at: AnchorId) -> Result<usize, PathEditError> {
    let (number, outline) = path
        .subpaths()
        .enumerate()
        .find(|(_, outline)| outline.anchors.iter().any(|a| a.id == at))
        .ok_or(PathEditError::NoSuchAnchor)?;
    let at_end = match end {
        PathEnd::First => outline.anchors.first(),
        PathEnd::Last => outline.anchors.last(),
    };
    if outline.closed || at_end.is_none_or(|anchor| anchor.id != at) {
        return Err(PathEditError::NotExtendable);
    }
    Ok(number)
}

/// The number and the anchors of the open outline of `path` whose node at `end` is `at`; errors
/// as [`outline_number`].
fn open_outline_at(
    path: &PathSnapshot,
    end: PathEnd,
    at: AnchorId,
) -> Result<(usize, &[AnchorSnapshot]), PathEditError> {
    let number = outline_number(path, end, at)?;
    // invariant: `outline_number` found this outline among the same subpaths.
    #[allow(clippy::unwrap_used)]
    Ok((number, path.subpaths().nth(number).unwrap().anchors))
}

/// `anchor` with the replacement for its id applied, if there is one.
fn resolved(anchor: &AnchorSnapshot, replace: &[AnchorSnapshot]) -> NewAnchor {
    replace
        .iter()
        .find(|new| new.id == anchor.id)
        .copied()
        .unwrap_or(*anchor)
}

/// Writes the fields of `new` that differ from `old`: a redundant write would win against a
/// collaborator's concurrent edit of the same register.
fn write_changed(map: &loro::LoroMap, old: &AnchorSnapshot, new: &AnchorSnapshot) {
    if old.point != new.point {
        write_point(map, KEY_POINT, new.point);
    }
    if old.handle_in != new.handle_in || read_vec2(map, KEY_HANDLE_IN) != new.handle_in {
        write_vec2(map, KEY_HANDLE_IN, new.handle_in);
    }
    if old.handle_out != new.handle_out || read_vec2(map, KEY_HANDLE_OUT) != new.handle_out {
        write_vec2(map, KEY_HANDLE_OUT, new.handle_out);
    }
    if old.kind != new.kind {
        write_kind(map, new.kind);
    }
}

/// The ids of a growth: added ids are unique and belong to no anchor of the two paths, and every
/// replaced or dropped id is an anchor of the survivor or of the absorbed path.
fn check_ids(
    growth: &PathGrowth,
    survivor: &PathSnapshot,
    absorbed: Option<&PathSnapshot>,
) -> Result<(), PathEditError> {
    let existing: HashSet<AnchorId> = survivor
        .all_anchors()
        .chain(absorbed.iter().flat_map(|path| path.all_anchors()))
        .map(|anchor| anchor.id)
        .collect();
    let mut seen = HashSet::new();
    for anchor in &growth.added {
        if existing.contains(&anchor.id) || !seen.insert(anchor.id) {
            return Err(PathEditError::DuplicateAnchorId);
        }
    }
    let named = growth
        .replace
        .iter()
        .map(|anchor| anchor.id)
        .chain(growth.drop.iter().copied());
    for id in named {
        if !existing.contains(&id) {
            return Err(PathEditError::NoSuchAnchor);
        }
    }
    Ok(())
}
