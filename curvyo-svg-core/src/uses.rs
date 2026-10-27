//! `<use>` (`specs/0024-svg-import-export` criterion 47, USE-CLONE B): a linked clone of the
//! object its target was read as when the relation is a move, rotation or flip only and the
//! copy would look the same, else an ordinary copy of the target at the place of the `use`.

use kurbo::Affine;
use roxmltree::Node;

use crate::drawing::{LeafKind, SvgNode};
use crate::limits::{MAX_CLONE_CHAIN, MAX_COORDINATE_MM, MAX_USE_DEPTH};
use crate::page::SODIPODI_NS;
use crate::reader::{Ctx, Reader};
use crate::shapes::attr;
use crate::style::{Computed, href};
use crate::units::{Axis, Units, aspect, view_box, view_box_map};

/// How far an entry of the linear part may be from an exact move, rotation or flip.
const ORTHOGONAL_TOLERANCE: f64 = 1e-6;

/// The elements a drawn original may not sit in.
const NOT_DRAWN: [&str; 6] = ["defs", "symbol", "clipPath", "mask", "pattern", "marker"];

impl Reader<'_, '_, '_> {
    /// A `use` under `ctx` (its own style and transform applied): a clone leaf, a copy, or
    /// nothing (a missing or foreign target, a cycle, a limit).
    pub(crate) fn use_element(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
        let target_id = href(node)?.strip_prefix('#')?;
        let target = *self.ids.get(target_id)?;
        let x = attr(node, "x")
            .and_then(|t| ctx.units.parse(t, Axis::X))
            .unwrap_or(0.0);
        let y = attr(node, "y")
            .and_then(|t| ctx.units.parse(t, Axis::Y))
            .unwrap_or(0.0);
        let placed = ctx.matrix * Affine::translate((x, y));
        // Inside a copy every `use` is a copy too: a copy is an ordinary leaf with no link, so a
        // chain past its clone depth continues as copies (criterion 47 (b) and (v)).
        let clone = if ctx.copy_depth == 0 {
            self.clone_of(node, target, placed, &ctx.style, &ctx.units)
        } else {
            None
        };
        if let Some((matrix, _)) = clone {
            let index = self.leaves;
            let style = ctx.style.leaf_style(1.0);
            let leaf = self.push_leaf(
                node,
                ctx,
                LeafKind::Clone {
                    original: usize::MAX,
                    matrix,
                },
                style,
            );
            self.pending.push((index, target.id().get()));
            return Some(leaf);
        }
        self.copy(node, target, placed, ctx)
    }

    /// The clone matrix M and chain depth when `node` (a `use` placed by `placed`, user units of
    /// the target to millimetres, with the computed `style`) can be a linked clone of `target`.
    fn clone_of(
        &mut self,
        node: Node<'_, '_>,
        target: Node<'_, '_>,
        placed: Affine,
        style: &Computed,
        units: &Units,
    ) -> Option<(Affine, usize)> {
        if style.opacity < 1.0 {
            return None;
        }
        let depth = if self.is(target, "path") {
            if target.attribute((SODIPODI_NS, "type")) == Some("star") {
                return None;
            }
            1
        } else if self.is(target, "use") {
            self.natural_clone_depth(target)? + 1
        } else {
            return None;
        };
        if depth > MAX_CLONE_CHAIN || node.id() == target.id() {
            return None;
        }
        let (ancestors, parent_style, target_units) = self.natural_context(target)?;
        let matrix = placed * ancestors.inverse();
        if !is_isometry(matrix) {
            return None;
        }
        let in_place = self.cascade.computed(target, &parent_style, &target_units);
        let in_use = self.cascade.computed(target, style, units);
        (in_place == in_use).then_some((matrix, depth))
    }

    /// The chain depth of the `use` element `node` at its own place, when it is a clone there.
    fn natural_clone_depth(&mut self, node: Node<'_, '_>) -> Option<usize> {
        if let Some(known) = self.clone_depths.get(&node.id().get()) {
            return *known;
        }
        // A cycle reads as a copy: the entry stands while the decision is made.
        self.clone_depths.insert(node.id().get(), None);
        let decision = (|| {
            let target = *self.ids.get(href(node)?.strip_prefix('#')?)?;
            let (ancestors, parent_style, units) = self.natural_context(node)?;
            let style = self.cascade.computed(node, &parent_style, &units);
            let own = attr(node, "transform")
                .and_then(|t| t.parse::<svgtypes::Transform>().ok())
                .map_or(Affine::IDENTITY, |t| {
                    Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
                });
            let x = attr(node, "x")
                .and_then(|t| units.parse(t, Axis::X))
                .unwrap_or(0.0);
            let y = attr(node, "y")
                .and_then(|t| units.parse(t, Axis::Y))
                .unwrap_or(0.0);
            let placed = ancestors * own * Affine::translate((x, y));
            self.clone_of(node, target, placed, &style, &units)
                .map(|(_, depth)| depth)
        })();
        self.clone_depths.insert(node.id().get(), decision);
        decision
    }

    /// The map, computed style and units of the parent of `node` at its own place in the file,
    /// or `None` when it is not drawn there (inside `defs`, a `symbol` and the like, or under an
    /// element the import does not read).
    pub(crate) fn natural_context(&self, node: Node<'_, '_>) -> Option<(Affine, Computed, Units)> {
        let chain: Vec<Node> = node.ancestors().skip(1).filter(Node::is_element).collect();
        let (root, containers) = chain.split_last()?;
        let view = crate::page::root_view(self.doc, *root);
        let mut matrix = view.matrix;
        let mut units = view.units;
        let mut style = self.cascade.computed(*root, &Computed::default(), &units);
        for container in containers.iter().rev() {
            let name = container.tag_name().name();
            if !self.is_svg(*container) || NOT_DRAWN.contains(&name) {
                return None;
            }
            if !matches!(name, "g" | "a" | "switch" | "svg") {
                return None;
            }
            style = self.cascade.computed(*container, &style, &units);
            if let Some(t) =
                attr(*container, "transform").and_then(|t| t.parse::<svgtypes::Transform>().ok())
            {
                matrix *= Affine::new([t.a, t.b, t.c, t.d, t.e, t.f]);
            }
            if name == "svg" {
                let (map, viewport) = crate::reader::viewport_map(*container, &units);
                matrix *= map;
                units.viewport = viewport;
            }
        }
        Some((matrix, style, units))
    }

    /// The map of the parent of `node` at its own place, when it is drawn there.
    pub(crate) fn natural_parent_matrix(&self, node: Node<'_, '_>) -> Option<Affine> {
        self.natural_context(node).map(|(matrix, _, _)| matrix)
    }

    /// The target copied to the place of the `use` (criterion 47 (b)), with the `use`'s name,
    /// hidden and locked state and opacity taken over.
    fn copy(
        &mut self,
        node: Node<'_, '_>,
        target: Node<'_, '_>,
        placed: Affine,
        ctx: &Ctx,
    ) -> Option<SvgNode> {
        let cycle = ctx.expanding.contains(&target.id())
            || ctx.expanding.contains(&node.id())
            || node.ancestors().any(|a| a.id() == target.id());
        if cycle || ctx.copy_depth >= MAX_USE_DEPTH || self.copies == 0 {
            return None;
        }
        self.copies -= 1;
        let mut inner = Ctx {
            matrix: placed,
            copy_depth: ctx.copy_depth + 1,
            at_root: false,
            ..ctx.clone()
        };
        inner.expanding.push(node.id());
        let copied = if self.is(target, "symbol") {
            if let Some((map, viewport)) = use_viewport(node, target, &ctx.units) {
                inner.matrix *= map;
                inner.units.viewport = viewport;
            }
            let style = self.cascade.computed(target, &inner.style, &inner.units);
            inner.style = style;
            let children = self.children(target, &inner);
            (!children.is_empty()).then_some(SvgNode::Group {
                state: Reader::state(target, &inner.style, false),
                layer: false,
                children,
            })
        } else {
            self.element(target, &inner)
        };
        let use_state = Reader::state(node, &ctx.style, true);
        copied.map(|mut copy| {
            let state = copy.state_mut();
            if use_state.name.is_some() {
                state.name = use_state.name;
            }
            state.hidden |= use_state.hidden;
            state.locked |= use_state.locked;
            state.opacity *= use_state.opacity;
            copy
        })
    }
}

/// Whether the linear part of `m` is orthogonal (a move, rotation or flip) within
/// [`ORTHOGONAL_TOLERANCE`] and its translation within the coordinate limit.
#[allow(clippy::many_single_char_names)] // the SVG names of the six entries
fn is_isometry(m: Affine) -> bool {
    let [a, b, c, d, e, f] = m.as_coeffs();
    let close = |value: f64, wanted: f64| (value - wanted).abs() <= ORTHOGONAL_TOLERANCE;
    close(a.mul_add(a, b * b), 1.0)
        && close(c.mul_add(c, d * d), 1.0)
        && close(a.mul_add(c, b * d), 0.0)
        && e.abs() <= MAX_COORDINATE_MM
        && f.abs() <= MAX_COORDINATE_MM
}

/// A `symbol`'s viewBox mapped to the `use`'s `width` × `height`.
fn use_viewport(
    node: Node<'_, '_>,
    symbol: Node<'_, '_>,
    units: &Units,
) -> Option<(Affine, (f64, f64))> {
    let w = attr(node, "width").and_then(|t| units.parse(t, Axis::X))?;
    let h = attr(node, "height").and_then(|t| units.parse(t, Axis::Y))?;
    let vb = view_box(symbol.attribute("viewBox"))?;
    (w > 0.0 && h > 0.0).then(|| {
        (
            view_box_map(vb, aspect(symbol.attribute("preserveAspectRatio")), w, h),
            (vb.w, vb.h),
        )
    })
}
