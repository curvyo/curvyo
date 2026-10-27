//! The import's entry point: the byte checks, the root, the walk, the clone links resolved and
//! the page decided (`specs/0024-svg-import-export` criteria 6, 15 to 20, 47 and 69).

use std::collections::BTreeMap;

use kurbo::{Affine, BezPath, Rect, Shape};

use crate::css::{Sheets, style_text};
use crate::drawing::{LeafKind, SvgDrawing, SvgNode};
use crate::limits::{MAX_ELEMENTS, MAX_FILE_BYTES, MAX_XML_NODES, PAGE_MM};
use crate::page::{background, display_unit, root_view};
use crate::paint::Ids;
use crate::reader::{Ctx, Reader, SVG_NS};
use crate::style::{Cascade, Computed};
use crate::xml_guard::{self, DoctypeRule, GuardError, GuardLimits};

/// Why a file gives no drawing at all (criterion 6). Everything else is read or ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgError {
    /// Larger than [`MAX_FILE_BYTES`].
    TooLarge,
    /// gzip bytes (an `.svgz`).
    Compressed,
    /// Not UTF-8 text.
    NotUtf8,
    /// A DOCTYPE with entities the literal rule does not allow (criterion 68).
    Entities,
    /// Not XML, nested too deep, or a root that is not `<svg>`; with the parse error's line and
    /// column when there is one.
    NotSvg(Option<(u32, u32)>),
}

/// The guard's limits for an import.
const IMPORT_LIMITS: GuardLimits = GuardLimits {
    max_bytes: MAX_FILE_BYTES,
    max_nodes: MAX_XML_NODES,
    max_elements: Some(MAX_ELEMENTS),
    doctype: DoctypeRule::LiteralEntities,
};

/// Reads the SVG `bytes` into a drawing. A pure function: nothing is fetched and nothing runs
/// (criterion 67).
///
/// # Errors
/// [`SvgError`] for the five refusals of criterion 6.
pub fn read_svg(bytes: &[u8]) -> Result<SvgDrawing, SvgError> {
    let checked = xml_guard::check(bytes, IMPORT_LIMITS).map_err(refusal)?;
    let doc = checked.parse().map_err(refusal)?;
    let root = doc.root_element();
    let plain_names = root.tag_name().namespace().is_none();
    if root.tag_name().name() != "svg"
        || !(plain_names || root.tag_name().namespace() == Some(SVG_NS))
    {
        return Err(SvgError::NotSvg(None));
    }
    let mut ids: Ids = BTreeMap::new();
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        if let Some(id) = node.attribute("id") {
            ids.entry(id).or_insert(node);
        }
    }
    let css = style_text(&doc);
    let sheets = Sheets::new(&css);
    let cascade = Cascade {
        sheets: &sheets,
        ids: &ids,
    };
    let view = root_view(&doc, root);
    let style = cascade.computed(root, &Computed::default(), &view.units);
    let ctx = Ctx {
        matrix: view.matrix,
        style,
        units: view.units,
        copy_depth: 0,
        expanding: Vec::new(),
        at_root: true,
    };
    let mut reader = Reader::new(&doc, cascade, &ids, plain_names);
    let nodes = reader.children(root, &ctx);
    let originals: BTreeMap<usize, usize> = reader
        .pending
        .iter()
        .filter_map(|(clone, target)| Some((*clone, *reader.leaf_of.get(target)?)))
        .collect();
    // Text path carriers: (text number, path slot) to the carrier's leaf number in the walk.
    let carriers: BTreeMap<(usize, usize), usize> = reader
        .carriers
        .iter()
        .filter_map(|(text, slot, target)| Some(((*text, *slot), *reader.leaf_of.get(target)?)))
        .collect();
    let nodes = resolve_clones(nodes, reader.leaves, &originals, &carriers);
    let page = view
        .page
        .or_else(|| content_page(&nodes))
        .map(|(w, h)| (w.clamp(PAGE_MM.0, PAGE_MM.1), h.clamp(PAGE_MM.0, PAGE_MM.1)));
    Ok(SvgDrawing {
        page,
        display_unit: display_unit(root),
        background: background(root),
        nodes,
        truncated: checked.truncated || reader.stopped,
    })
}

fn refusal(error: GuardError) -> SvgError {
    match error {
        GuardError::TooLarge => SvgError::TooLarge,
        GuardError::Compressed => SvgError::Compressed,
        GuardError::NotUtf8 => SvgError::NotUtf8,
        GuardError::Entities => SvgError::Entities,
        GuardError::TooDeep => SvgError::NotSvg(None),
        GuardError::NotXml { line, column } => SvgError::NotSvg(Some((line, column))),
    }
}

/// Gives every clone leaf its original's final index, dropping the clones whose original was not
/// read (and their own clones), and the groups that become empty. `originals` maps a clone's
/// index during the walk to its original's.
fn resolve_clones(
    nodes: Vec<SvgNode>,
    leaves: usize,
    originals: &BTreeMap<usize, usize>,
    carriers: &BTreeMap<(usize, usize), usize>,
) -> Vec<SvgNode> {
    let mut clones = vec![false; leaves];
    let mut walk_index = 0;
    mark_clones(&nodes, &mut walk_index, &mut clones);
    // A clone is kept when its chain ends on a leaf that is not a clone.
    let kept: Vec<bool> = (0..leaves)
        .map(|index| {
            let mut at = index;
            for _ in 0..=crate::limits::MAX_CLONE_CHAIN {
                if !clones[at] {
                    return true;
                }
                match originals.get(&at) {
                    Some(&next) => at = next,
                    None => return false,
                }
            }
            false
        })
        .collect();
    let mut new_index = vec![None; leaves];
    let mut next = 0;
    for (old, slot) in new_index.iter_mut().enumerate() {
        if kept[old] {
            *slot = Some(next);
            next += 1;
        }
    }
    let mut rebuilt = Rebuild {
        index: 0,
        text: 0,
        kept: &kept,
        new_index: &new_index,
        originals,
        carriers,
        carrier_leaves: carriers.values().copied().collect(),
    };
    rebuilt.nodes(nodes)
}

fn mark_clones(nodes: &[SvgNode], index: &mut usize, clones: &mut [bool]) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => mark_clones(children, index, clones),
            SvgNode::Leaf { leaf, .. } => {
                clones[*index] = matches!(leaf.kind, LeafKind::Clone { .. });
                *index += 1;
            }
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
}

/// The renumbering after the walk: clones to their originals' final numbers, texts to their
/// carriers', carriers that SVG does not draw marked as guides (criterion 94).
struct Rebuild<'a> {
    /// The leaf number in the walk.
    index: usize,
    /// The text number in the walk.
    text: usize,
    kept: &'a [bool],
    new_index: &'a [Option<usize>],
    originals: &'a BTreeMap<usize, usize>,
    carriers: &'a BTreeMap<(usize, usize), usize>,
    carrier_leaves: std::collections::BTreeSet<usize>,
}

impl Rebuild<'_> {
    fn nodes(&mut self, nodes: Vec<SvgNode>) -> Vec<SvgNode> {
        let mut out = Vec::with_capacity(nodes.len());
        for node in nodes {
            match node {
                SvgNode::Group {
                    state,
                    layer,
                    children,
                } => {
                    let children = self.nodes(children);
                    if layer || !children.is_empty() {
                        out.push(SvgNode::Group {
                            state,
                            layer,
                            children,
                        });
                    }
                }
                SvgNode::Leaf {
                    mut state,
                    mut leaf,
                } => {
                    let old = self.index;
                    self.index += 1;
                    if !self.kept[old] {
                        continue;
                    }
                    if let LeafKind::Clone { original, .. } = &mut leaf.kind {
                        match self.originals.get(&old).and_then(|o| self.new_index[*o]) {
                            Some(found) => *original = found,
                            None => continue,
                        }
                    }
                    if self.carrier_leaves.contains(&old) {
                        let painted = |paint: Option<crate::drawing::Paint>| {
                            paint.is_some_and(|p| p.opacity > 0.0)
                        };
                        let drawn = (painted(leaf.style.fill) || painted(leaf.style.stroke))
                            && state.opacity > 0.0;
                        state.guide |= !drawn || state.hidden;
                    }
                    out.push(SvgNode::Leaf { state, leaf });
                }
                SvgNode::Text { state, mut text } => {
                    let number = self.text;
                    self.text += 1;
                    for (slot, path) in text.paths.iter_mut().enumerate() {
                        let walked = path
                            .carrier
                            .or_else(|| self.carriers.get(&(number, slot)).copied());
                        path.carrier = walked.and_then(|old| self.new_index[old]);
                    }
                    out.push(SvgNode::Text { state, text });
                }
                image @ SvgNode::Image { .. } => out.push(image),
            }
        }
        out
    }
}

/// The page of a file with no size and no viewBox (criterion 17): from (0, 0) to the right and
/// bottom edge of the drawing (stroke excluded, hidden objects included), rounded up to whole
/// millimetres, at least 1 mm; `None` for an empty drawing.
fn content_page(nodes: &[SvgNode]) -> Option<(f64, f64)> {
    let b = bounds_of(nodes)?;
    Some((b.x1.max(0.0).ceil().max(1.0), b.y1.max(0.0).ceil().max(1.0)))
}

/// The bounding box in millimetres of every leaf of `nodes` (clones through their chain), the
/// stroke excluded and hidden leaves included; `None` without a leaf.
pub(crate) fn bounds_of(nodes: &[SvgNode]) -> Option<Rect> {
    let mut outlines: Vec<Outline> = Vec::new();
    collect_outlines(nodes, &mut outlines);
    let mut bounds: Option<Rect> = None;
    for index in 0..outlines.len() {
        let mut matrix = Affine::IDENTITY;
        let mut at = index;
        let mut path = None;
        for _ in 0..=crate::limits::MAX_CLONE_CHAIN {
            match &outlines[at] {
                (Some(own), _) => {
                    path = Some(matrix * own.clone());
                    break;
                }
                (None, Some((original, m))) => {
                    matrix *= *m;
                    at = *original;
                }
                (None, None) => break,
            }
        }
        if let Some(path) = path {
            let b = path.bounding_box();
            bounds = Some(bounds.map_or(b, |all| all.union(b)));
        }
    }
    bounds
}

/// A leaf's own outline, or a clone's original and matrix.
type Outline = (Option<BezPath>, Option<(usize, Affine)>);

fn collect_outlines(nodes: &[SvgNode], out: &mut Vec<Outline>) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => collect_outlines(children, out),
            SvgNode::Leaf { leaf, .. } => out.push(match leaf.kind {
                LeafKind::Clone { original, matrix } => (None, Some((original, matrix))),
                ref kind => (kind.outline_mm(), None),
            }),
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
}
