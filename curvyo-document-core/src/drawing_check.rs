//! The checks of an imported drawing before its first write (`specs/0024-svg-import-export`
//! criteria 2, 3, 78, 93, 104): anchor ids, outlines, clone chains, group depth, texts, carriers
//! and assets, so a drawing that fails writes nothing.

use std::collections::HashSet;
use std::sync::Arc;

use loro::LoroTree;

use crate::clone_model::MAX_CLONE_DEPTH;
use crate::document::Document;
use crate::drawing_content::DrawingText;
use crate::drawing_insert::DrawingNode;
use crate::group_tree::MAX_GROUP_DEPTH;
use crate::objects::ObjectEditError;
use crate::path_model::AnchorId;

/// What the leaves of a drawing are, in document order: `Some(original)` for a clone.
fn leaf_kinds(nodes: &[DrawingNode], out: &mut Vec<(bool, Option<usize>)>) {
    for node in nodes {
        match node {
            DrawingNode::Group { children, .. } => leaf_kinds(children, out),
            DrawingNode::Path { .. } => out.push((true, None)),
            DrawingNode::Primitive { .. } => out.push((false, None)),
            DrawingNode::Clone { original, .. } => out.push((false, Some(*original))),
            DrawingNode::Text { .. }
            | DrawingNode::TextOnPath { .. }
            | DrawingNode::Image { .. } => {}
        }
    }
}

/// The deepest chain of groups in `nodes`.
fn group_depth(nodes: &[DrawingNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            DrawingNode::Group { children, .. } => 1 + group_depth(children),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

/// The texts, carriers and asset numbers of a drawing's content.
fn collect_content<'a>(
    nodes: &'a [DrawingNode],
    texts: &mut Vec<&'a DrawingText>,
    carriers: &mut Vec<usize>,
    assets: &mut Vec<usize>,
) {
    for node in nodes {
        match node {
            DrawingNode::Group { children, .. } => {
                collect_content(children, texts, carriers, assets);
            }
            DrawingNode::Text { text, .. } => texts.push(text),
            DrawingNode::TextOnPath { text, carrier, .. } => {
                texts.push(text);
                carriers.push(*carrier);
            }
            DrawingNode::Image {
                asset, placement, ..
            } => {
                assets.push(*asset);
                if !placement.is_within_limit() {
                    carriers.push(usize::MAX);
                }
            }
            _ => {}
        }
    }
}

/// Every check of a drawing before the first write: anchor ids, outlines, clone references and
/// chains, group depth (`extra_groups` above the drawing's own), texts, carriers and assets.
pub(crate) fn validate(
    document: &Document,
    tree: &LoroTree,
    nodes: &[DrawingNode],
    anchor_ids: &[AnchorId],
    extra_groups: usize,
    assets: &[Arc<[u8]>],
) -> Result<(), ObjectEditError> {
    let needed = DrawingNode::anchor_count(nodes);
    let seen: HashSet<AnchorId> = anchor_ids.iter().copied().collect();
    if anchor_ids.len() != needed || seen.len() != needed {
        return Err(ObjectEditError::AnchorIds);
    }
    crate::replace::check_new_anchor_ids(tree, &[], &seen)?;
    if group_depth(nodes) + extra_groups > MAX_GROUP_DEPTH {
        return Err(ObjectEditError::TooDeep);
    }
    let mut empty = false;
    check_outlines(nodes, &mut empty);
    if empty {
        return Err(ObjectEditError::NoOutlines);
    }
    let mut leaves = Vec::new();
    leaf_kinds(nodes, &mut leaves);
    for start in 0..leaves.len() {
        let mut at = start;
        let mut depth = 0;
        while let Some(original) = leaves[at].1 {
            depth += 1;
            if depth > MAX_CLONE_DEPTH || original >= leaves.len() {
                return Err(ObjectEditError::NoSuchObject);
            }
            at = original;
        }
        if depth > 0 && !leaves[at].0 {
            return Err(ObjectEditError::NoSuchObject);
        }
    }
    let (mut texts, mut carriers, mut named) = (Vec::new(), Vec::new(), Vec::new());
    collect_content(nodes, &mut texts, &mut carriers, &mut named);
    if texts.iter().any(|text| !text.is_valid()) {
        return Err(ObjectEditError::InvalidText);
    }
    // A carrier is a path or a primitive, never a clone.
    if carriers
        .iter()
        .any(|&at| leaves.get(at).is_none_or(|leaf| leaf.1.is_some()))
    {
        return Err(ObjectEditError::NoSuchObject);
    }
    crate::drawing_content::check_assets(document, assets, &named)
}

fn check_outlines(nodes: &[DrawingNode], empty: &mut bool) {
    for node in nodes {
        match node {
            DrawingNode::Group { children, .. } => check_outlines(children, empty),
            DrawingNode::Path { outlines, .. } => {
                *empty |= outlines.is_empty() || outlines.iter().any(|o| o.anchors.is_empty());
            }
            _ => {}
        }
    }
}
