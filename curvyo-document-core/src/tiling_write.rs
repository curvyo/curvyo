//! The writes of one tiling member (ADR 0030 §2 and §3): a new linked clone, text clone or copy of
//! a source leaf placed by its cell, and the minimal rewrite of an existing one.

use loro::LoroMap;

use crate::affine::Affine;
use crate::clone_codec::{
    CloneLink, KEY_OWN_PLACEMENT, KEY_OWN_STYLE, is_clone_meta, read_clone_link, read_overrides,
    write_clone, write_clone_link, write_flag,
};
use crate::clone_read::ObjectRead;
use crate::image_model::ImageSnapshot;
use crate::path_model::{AnchorId, NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;
use crate::style_model::Style;
use crate::text_model::TextSnapshot;
use crate::tiling_model::{TILING_WRITE_TOLERANCE_MM, TilingCell, TilingResult, hue_shifted};

/// What a source leaf is, as far as tiling it goes.
#[derive(Debug, Clone)]
pub(crate) enum LeafSource {
    /// A path, compound path, primitive (its conversion), clone (resolved) or sketch (drawn):
    /// its outline and style in document space.
    Outline(PathSnapshot),
    /// A text or text clone as shown; `root` is the text a text clone follows.
    Text {
        text: Box<TextSnapshot>,
        root: NodeId,
    },
    /// An image.
    Image(ImageSnapshot),
}

/// One source leaf with what its instances need.
#[derive(Debug, Clone)]
pub(crate) struct Leaf {
    /// The leaf node.
    pub(crate) id: NodeId,
    /// Its geometry and style.
    pub(crate) source: LeafSource,
    /// Its `opacity` register.
    pub(crate) opacity: f64,
}

/// What one member is made as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberKind {
    Clone,
    TextClone,
    PathCopy,
    TextCopy,
    ImageCopy,
}

impl Leaf {
    /// The kind of this leaf's members under `result`: images are always copies.
    pub(crate) const fn kind(&self, result: TilingResult) -> MemberKind {
        match (&self.source, result) {
            (LeafSource::Outline(_), TilingResult::Clones) => MemberKind::Clone,
            (LeafSource::Outline(_), TilingResult::Copies) => MemberKind::PathCopy,
            (LeafSource::Text { .. }, TilingResult::Clones) => MemberKind::TextClone,
            (LeafSource::Text { .. }, TilingResult::Copies) => MemberKind::TextCopy,
            (LeafSource::Image(_), _) => MemberKind::ImageCopy,
        }
    }

    fn style(&self) -> Style {
        match &self.source {
            LeafSource::Outline(path) => path.style.clone(),
            LeafSource::Text { text, .. } => text.style.clone(),
            LeafSource::Image(image) => image.style.clone(),
        }
    }

    /// The style and opacity of a member in `cell`: the leaf's, with the cell's hue and opacity
    /// shift when `vary` is on (criterion 15).
    fn varied(&self, cell: &TilingCell, vary: bool) -> (Style, f64) {
        let mut style = self.style();
        if !vary {
            return (style, self.opacity);
        }
        style.fill.color = hue_shifted(style.fill.color, cell.hue);
        style.stroke.color = hue_shifted(style.stroke.color, cell.hue);
        let opacity = (self.opacity + cell.opacity.get() / 100.0).clamp(0.0, 1.0);
        (style, opacity)
    }
}

/// Whether the node `meta` (with id `id`) is a member of `kind` made from `leaf`.
pub(crate) fn is_member_of(
    read: &ObjectRead,
    meta: &LoroMap,
    id: NodeId,
    leaf: &Leaf,
    kind: MemberKind,
) -> bool {
    match kind {
        MemberKind::Clone => {
            is_clone_meta(meta) && read_clone_link(meta).is_some_and(|link| link.origin == leaf.id)
        }
        MemberKind::TextClone => {
            let LeafSource::Text { root, .. } = &leaf.source else {
                return false;
            };
            crate::text_clone_codec::read_source(meta) == Some(*root)
        }
        MemberKind::PathCopy => matches!(read.get(id), Some(ObjectSnapshot::Path(_))),
        MemberKind::TextCopy => {
            matches!(read.get(id), Some(ObjectSnapshot::Text(t)) if t.source.is_none())
        }
        MemberKind::ImageCopy => crate::image_codec::is_image_meta(meta),
    }
}

/// The node registers a text-copy rewrite keeps (ADR 0030 §5: Apply owns placement and the
/// variance's own style, not visibility, lock or guide).
const KEPT_NODE_REGISTERS: [&str; 5] = [
    crate::node_registers::KEY_NAME,
    crate::node_registers::KEY_VISIBLE,
    crate::node_registers::KEY_LOCKED,
    crate::node_registers::KEY_GUIDE,
    crate::node_registers::KEY_LAYER,
];

fn mapped_outline(path: &PathSnapshot, affine: Affine, ids: &[AnchorId]) -> PathSnapshot {
    let mut mapped = path.mapped(affine);
    for (anchor, id) in mapped.all_anchors_mut().zip(ids) {
        anchor.id = *id;
    }
    mapped
}

fn mapped_text(text: &TextSnapshot, affine: Affine, style: Style) -> TextSnapshot {
    let mut copy = text.clone();
    copy.placement = affine.compose(text.placement);
    copy.style = style;
    copy.on_path = None;
    copy.source = None;
    copy
}

fn write_text_copy(meta: &LoroMap, text: &TextSnapshot, opacity: f64) {
    crate::shape_codec::write_shape_tag(meta, crate::text_codec::SHAPE_TEXT);
    crate::style_codec::write_style(meta, &text.style);
    crate::text_codec::write_text(meta, text);
    crate::clone_write::write_opacity_register(meta, opacity);
}

/// Writes a new member of `leaf` into the fresh node `meta`. `mint` gives the anchor ids of a
/// path copy. Uncommitted.
pub(crate) fn write_new(
    meta: &LoroMap,
    leaf: &Leaf,
    kind: MemberKind,
    cell: &TilingCell,
    vary: bool,
    mint: &mut dyn FnMut() -> AnchorId,
) {
    let (style, opacity) = leaf.varied(cell, vary);
    match (&leaf.source, kind) {
        (LeafSource::Outline(_), MemberKind::Clone) => {
            write_clone(
                meta,
                CloneLink {
                    origin: leaf.id,
                    matrix: cell.affine,
                },
            );
            write_flag(meta, KEY_OWN_PLACEMENT, true);
            if vary {
                crate::clone_overrides::write_own_style(meta, &style, opacity);
            }
        }
        (LeafSource::Outline(path), _) => {
            let ids: Vec<AnchorId> = (0..path.all_anchors().count()).map(|_| mint()).collect();
            let mut copy = mapped_outline(path, cell.affine, &ids);
            copy.style = style;
            crate::clone_write::unlink_meta(meta, &copy, opacity);
        }
        (LeafSource::Text { text, root }, MemberKind::TextClone) => {
            crate::shape_codec::write_shape_tag(meta, crate::text_codec::SHAPE_TEXT);
            crate::text_clone_codec::write_source(meta, *root);
            crate::text_codec::write_placement(meta, cell.affine.compose(text.placement));
            if vary {
                crate::style_codec::write_style(meta, &style);
                write_flag(meta, KEY_OWN_STYLE, true);
                crate::clone_write::write_opacity_register(meta, opacity);
            }
        }
        (LeafSource::Text { text, .. }, _) => {
            write_text_copy(meta, &mapped_text(text, cell.affine, style), opacity);
        }
        (LeafSource::Image(image), _) => {
            crate::image_codec::write_image(
                meta,
                &image.asset,
                cell.affine.compose(image.placement),
            );
            crate::clone_write::write_opacity_register(meta, opacity);
        }
    }
}

/// The own style of a clone or text clone: written while `vary` is on, cleared when the style
/// variance was on before and is off now, a hand-set one kept otherwise (criterion 28).
fn sync_own_style(
    meta: &LoroMap,
    style: &Style,
    opacity: f64,
    vary: bool,
    varied_before: bool,
) -> bool {
    let overrides = read_overrides(meta);
    if vary {
        let stored_opacity = crate::node_registers::read_registers(meta).opacity.get();
        #[allow(clippy::float_cmp)] // a register written from the same computation
        let same = overrides.own_style
            && crate::style_codec::read_style(meta) == *style
            && stored_opacity == opacity;
        if !same {
            crate::clone_overrides::write_own_style(meta, style, opacity);
        }
        return !same;
    }
    if varied_before && overrides.own_style {
        crate::clone_overrides::clear_style_keys(meta);
        write_flag(meta, KEY_OWN_STYLE, false);
        return true;
    }
    false
}

fn path_differs(stored: &PathSnapshot, expected: &PathSnapshot, opacity: (f64, f64)) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() <= TILING_WRITE_TOLERANCE_MM;
    let anchors_match = stored.all_anchors().count() == expected.all_anchors().count()
        && stored
            .all_anchors()
            .zip(expected.all_anchors())
            .all(|(s, e)| {
                near(s.point.x, e.point.x)
                    && near(s.point.y, e.point.y)
                    && near(s.handle_in.x, e.handle_in.x)
                    && near(s.handle_in.y, e.handle_in.y)
                    && near(s.handle_out.x, e.handle_out.x)
                    && near(s.handle_out.y, e.handle_out.y)
                    && s.kind == e.kind
            });
    let closed_match = stored.closed == expected.closed
        && stored
            .extra_subpaths
            .iter()
            .map(|s| s.closed)
            .eq(expected.extra_subpaths.iter().map(|s| s.closed));
    !anchors_match || !closed_match || stored.style != expected.style || !near(opacity.0, opacity.1)
}

fn text_differs(stored: &TextSnapshot, expected: &TextSnapshot) -> bool {
    stored.string != expected.string
        || stored.font != expected.font
        || stored.size != expected.size
        || stored.layout != expected.layout
        || stored.chars != expected.chars
        || stored.style != expected.style
        || !stored
            .placement
            .approx_eq(expected.placement, TILING_WRITE_TOLERANCE_MM)
}

/// What one member should be after an Apply: its cell's placement, its varied style and
/// opacity, and whether the style variance is on now and was on before.
struct Target {
    affine: Affine,
    style: Style,
    opacity: f64,
    vary: bool,
    varied_before: bool,
}

impl Target {
    fn sync_own_style(&self, meta: &LoroMap) -> bool {
        sync_own_style(
            meta,
            &self.style,
            self.opacity,
            self.vary,
            self.varied_before,
        )
    }
}

/// Rewrites the existing member `meta` (id `id`) of `leaf` where it differs from what `cell`
/// gives; returns whether anything was written. Uncommitted.
pub(crate) fn update(
    read: &ObjectRead,
    (meta, id): (&LoroMap, NodeId),
    leaf: &Leaf,
    kind: MemberKind,
    cell: &TilingCell,
    (vary, varied_before): (bool, bool),
    mint: &mut dyn FnMut() -> AnchorId,
) -> bool {
    let (style, opacity) = leaf.varied(cell, vary);
    let target = Target {
        affine: cell.affine,
        style,
        opacity,
        vary,
        varied_before,
    };
    match (&leaf.source, kind) {
        (LeafSource::Outline(_), MemberKind::Clone) => update_clone(meta, leaf.id, &target),
        (LeafSource::Outline(path), _) => update_path_copy(read, (meta, id), path, target, mint),
        (LeafSource::Text { text, .. }, MemberKind::TextClone) => {
            update_text_clone(meta, text, &target)
        }
        (LeafSource::Text { text, .. }, _) => update_text_copy(read, (meta, id), text, target),
        (LeafSource::Image(image), _) => update_image_copy((meta, id), image, &target),
    }
}

fn stored_opacity(meta: &LoroMap) -> f64 {
    crate::node_registers::read_registers(meta).opacity.get()
}

fn update_clone(meta: &LoroMap, origin: NodeId, target: &Target) -> bool {
    let mut changed = false;
    if read_clone_link(meta).is_none_or(|link| {
        !link
            .matrix
            .approx_eq(target.affine, TILING_WRITE_TOLERANCE_MM)
    }) {
        write_clone_link(
            meta,
            CloneLink {
                origin,
                matrix: target.affine,
            },
        );
        changed = true;
    }
    if !read_overrides(meta).own_placement {
        write_flag(meta, KEY_OWN_PLACEMENT, true);
        changed = true;
    }
    target.sync_own_style(meta) || changed
}

fn update_path_copy(
    read: &ObjectRead,
    (meta, id): (&LoroMap, NodeId),
    path: &PathSnapshot,
    target: Target,
    mint: &mut dyn FnMut() -> AnchorId,
) -> bool {
    let Some(ObjectSnapshot::Path(stored)) = read.get(id) else {
        return false;
    };
    let count = path.all_anchors().count();
    let ids: Vec<AnchorId> = if stored.all_anchors().count() == count {
        stored.all_anchors().map(|a| a.id).collect()
    } else {
        (0..count).map(|_| mint()).collect()
    };
    let mut expected = mapped_outline(path, target.affine, &ids);
    expected.style = target.style;
    if !path_differs(stored, &expected, (stored_opacity(meta), target.opacity)) {
        return false;
    }
    crate::clone_write::clear_clone_keys(meta);
    crate::clone_write::unlink_meta(meta, &expected, target.opacity);
    true
}

fn update_text_clone(meta: &LoroMap, text: &TextSnapshot, target: &Target) -> bool {
    let placement = target.affine.compose(text.placement);
    let mut changed = false;
    if crate::text_codec::read_placement(meta)
        .is_none_or(|p| !p.approx_eq(placement, TILING_WRITE_TOLERANCE_MM))
    {
        crate::text_codec::write_placement(meta, placement);
        changed = true;
    }
    target.sync_own_style(meta) || changed
}

fn update_text_copy(
    read: &ObjectRead,
    (meta, id): (&LoroMap, NodeId),
    text: &TextSnapshot,
    target: Target,
) -> bool {
    let expected = mapped_text(text, target.affine, target.style);
    let Some(ObjectSnapshot::Text(stored)) = read.get(id) else {
        return false;
    };
    #[allow(clippy::float_cmp)] // a register written from the same computation
    if !text_differs(stored, &expected) && stored_opacity(meta) == target.opacity {
        return false;
    }
    for key in meta.keys().map(|k| k.to_string()).collect::<Vec<_>>() {
        if !crate::tiling_codec::ALL_TILING_KEYS.contains(&key.as_str())
            && !KEPT_NODE_REGISTERS.contains(&key.as_str())
        {
            crate::tiling_codec::remove(meta, &key);
        }
    }
    write_text_copy(meta, &expected, target.opacity);
    true
}

fn update_image_copy(
    (meta, id): (&LoroMap, NodeId),
    image: &ImageSnapshot,
    target: &Target,
) -> bool {
    let placement = target.affine.compose(image.placement);
    let same = crate::image_codec::read_image_snapshot(id, meta).is_some_and(|s| {
        s.asset == image.asset && s.placement.approx_eq(placement, TILING_WRITE_TOLERANCE_MM)
    });
    #[allow(clippy::float_cmp)] // a register written from the same computation
    if same && stored_opacity(meta) == target.opacity {
        return false;
    }
    crate::image_codec::write_image(meta, &image.asset, placement);
    crate::tiling_codec::remove(meta, crate::node_registers::KEY_OPACITY);
    crate::clone_write::write_opacity_register(meta, target.opacity);
    true
}
