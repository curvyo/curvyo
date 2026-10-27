//! A `<text>` or `<flowRoot>` read into an [`SvgText`] (`specs/0024-svg-import-export` criteria
//! 78, 79, 85 to 98): the element tree below it walked into runs of character data with their
//! styles, position lists and text paths; Inkscape lines and flowed paragraphs as line breaks.

use kurbo::{Affine, ParamCurveArclen, Shape};
use roxmltree::Node;

use crate::drawing::{LeafKind, LeafStyle, NodeState, SvgLeaf, SvgNode};
use crate::drawing_text::{FlowBox, SvgText, SvgTextPath, TextStyle};
use crate::page::SODIPODI_NS;
use crate::reader::{Ctx, Reader};
use crate::shapes::{attr, leaf_kind};
use crate::style::{Computed, href};
use crate::text_chars::{Lists, MAX_LIST_NUMBERS, Segment, Space, characters, placed};
use crate::text_style::families;
use crate::units::{Axis, Units};

/// The namespace of `xml:space`.
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// The elements a `textPath` may point to (criterion 93).
const CARRIERS: [&str; 7] = [
    "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
];

/// How a text path's carrier is found once the text is known to be drawn.
enum CarrierRequest {
    /// No drawn shape: a straight text.
    None,
    /// The element drawn at its own place in the same user space, by its node id.
    Drawn(u32),
    /// A new guide path with this outline in millimetres.
    Guide(kurbo::BezPath),
}

/// The walk below one text element.
struct Walk {
    segments: Vec<Segment>,
    lists: Vec<Lists>,
    styles: Vec<TextStyle>,
    paths: Vec<(SvgTextPath, CarrierRequest)>,
    stack: Vec<usize>,
    /// Whether a tspan below the text has both `x` and `y` (positioned lines, criterion 96 (a)).
    positioned: bool,
    /// Whether any character data was seen (an Inkscape line breaks only after some).
    content: bool,
}

impl Reader<'_, '_, '_> {
    /// A `<text>` under `ctx` (its own style and transform applied); `None` without a character.
    pub(crate) fn text(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
        let mut walk = Walk {
            segments: Vec::new(),
            lists: Vec::new(),
            styles: Vec::new(),
            paths: Vec::new(),
            stack: Vec::new(),
            positioned: false,
            content: false,
        };
        let space = space_of(node, Space::Normal, &ctx.style);
        let root_style = Self::enter(node, ctx, &ctx.style, &mut walk);
        self.walk_children(node, ctx, &ctx.style, space, None, &mut walk);
        let flow = if walk.positioned {
            None
        } else {
            self.frame_of(node, ctx)
        };
        self.finish(node, ctx, walk, root_style, flow)
    }

    /// A `<flowRoot>` (Inkscape 0.48 to 0.92): the first shape of its `flowRegion` is the box,
    /// each `flowPara` or `flowDiv` a paragraph (criterion 96 (b)).
    pub(crate) fn flow_root(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
        let mut walk = Walk {
            segments: Vec::new(),
            lists: Vec::new(),
            styles: Vec::new(),
            paths: Vec::new(),
            stack: Vec::new(),
            positioned: false,
            content: false,
        };
        let root_style = Self::enter(node, ctx, &ctx.style, &mut walk);
        let mut region = None;
        let children: Vec<_> = node.children().filter(|c| self.is_svg(*c)).collect();
        for child in children {
            match child.tag_name().name() {
                "flowRegion" if region.is_none() => {
                    region = child
                        .children()
                        .find_map(|shape| self.shape_box(shape, &ctx.units));
                }
                "flowPara" | "flowDiv" => {
                    if walk.content {
                        walk.segments.push(break_segment(root_style, &walk.stack));
                    }
                    walk.content = true;
                    let style = self.cascade.computed(child, &ctx.style, &ctx.units);
                    Self::enter(child, ctx, &style, &mut walk);
                    self.walk_children(child, ctx, &style, Space::Normal, None, &mut walk);
                    walk.stack.pop();
                }
                _ => {}
            }
        }
        let flow = region.map_or(
            FlowBox {
                left: 0.0,
                top: 0.0,
                width: 0.0,
                first_baseline: None,
                by_anchor: false,
            },
            |b| FlowBox {
                left: b.x0,
                top: b.y0,
                width: b.width(),
                first_baseline: None,
                by_anchor: false,
            },
        );
        self.finish(node, ctx, walk, root_style, Some(flow))
    }

    /// Adds the style and lists of `node` (computed as `style`) and enters it; returns its style.
    fn enter(node: Node<'_, '_>, ctx: &Ctx, style: &Computed, walk: &mut Walk) -> usize {
        let scale = ctx.matrix.determinant().abs().sqrt();
        let text = &style.text;
        let font_size = text.font_size;
        walk.styles.push(TextStyle {
            families: families(text.family.as_deref()),
            weight: text.weight,
            italic: text.italic,
            stretch: text.stretch,
            font_size,
            letter_spacing: text.letter_spacing,
            word_spacing: text.word_spacing,
            anchor: text.anchor,
            writing_mode: text.writing_mode,
            orientation: text.orientation,
            line_height: text.line_height,
            baseline: text.baseline(),
            baseline_shift: text.baseline_shift,
            flow_align: text.flow_align,
            paint: style.leaf_style(scale),
        });
        let list = |name: &str, axis| numbers(node, name, |t| ctx.units.parse(t, axis));
        walk.lists.push(Lists {
            x: list("x", Axis::X),
            y: list("y", Axis::Y),
            dx: list("dx", Axis::X),
            dy: list("dy", Axis::Y),
            rotate: numbers(node, "rotate", |t| t.parse::<f64>().ok()),
        });
        walk.stack.push(walk.lists.len() - 1);
        walk.styles.len() - 1
    }

    /// Walks the children of a text content element.
    fn walk_children(
        &mut self,
        node: Node<'_, '_>,
        ctx: &Ctx,
        style: &Computed,
        space: Space,
        path: Option<usize>,
        walk: &mut Walk,
    ) {
        let own_style = walk.styles.len() - 1;
        for child in node.children() {
            if child.is_text() {
                if let Some(text) = child.text() {
                    walk.content |= !text.is_empty();
                    walk.segments.push(Segment {
                        text: Some(text.to_owned()),
                        style: own_style,
                        path,
                        elements: walk.stack.clone(),
                        space,
                        transform: style.text.transform,
                    });
                }
                continue;
            }
            if !self.is_svg(child) {
                continue;
            }
            let name = child.tag_name().name();
            if !matches!(name, "tspan" | "a" | "textPath" | "flowSpan") {
                continue;
            }
            let child_style = self.cascade.computed(child, style, &ctx.units);
            if child_style.display_none {
                continue;
            }
            if name == "tspan" {
                walk.positioned |= child.attribute("x").is_some() && child.attribute("y").is_some();
                if child.attribute((SODIPODI_NS, "role")) == Some("line") {
                    if walk.content {
                        walk.segments.push(break_segment(own_style, &walk.stack));
                    }
                    walk.content = true;
                }
            }
            let child_path = if name == "textPath" && path.is_none() {
                let request = self.carrier_request(child, ctx);
                walk.paths.push(request);
                Some(walk.paths.len() - 1)
            } else {
                path
            };
            let child_space = space_of(child, space, &child_style);
            Self::enter(child, ctx, &child_style, walk);
            self.walk_children(child, ctx, &child_style, child_space, child_path, walk);
            walk.stack.pop();
        }
    }

    /// The characters made into a text node, with any new guide carrier placed before it.
    fn finish(
        &mut self,
        node: Node<'_, '_>,
        ctx: &Ctx,
        walk: Walk,
        root_style: usize,
        flow: Option<FlowBox>,
    ) -> Option<SvgNode> {
        if walk.styles[root_style].font_size <= 0.0 {
            return None;
        }
        let mut budget = self.text_chars.min(self.points);
        let before = budget;
        let raw = characters(&walk.segments, &mut budget);
        let used = before - budget;
        self.text_chars -= used;
        self.points -= used;
        if self.text_chars == 0 || self.points == 0 {
            self.stopped = true;
        }
        let chars: Vec<_> = placed(&raw, &walk.segments, &walk.lists)
            .into_iter()
            .filter(|c| walk.styles[c.style].font_size > 0.0)
            .collect();
        if !chars.iter().any(|c| c.ch != '\n') {
            return None;
        }
        let serial = self.texts;
        self.texts += 1;
        let mut paths = Vec::with_capacity(walk.paths.len());
        for (slot, (path, request)) in walk.paths.into_iter().enumerate() {
            match request {
                CarrierRequest::None => {}
                CarrierRequest::Drawn(target) => self.carriers.push((serial, slot, target)),
                CarrierRequest::Guide(outline) => {
                    let index = self.leaves;
                    self.leaves += 1;
                    self.before.push(SvgNode::Leaf {
                        state: NodeState {
                            guide: true,
                            ..NodeState::default()
                        },
                        leaf: SvgLeaf {
                            kind: LeafKind::Path {
                                path: outline,
                                node_types: None,
                            },
                            style: LeafStyle {
                                fill: None,
                                even_odd: false,
                                stroke: None,
                                stroke_width_mm: None,
                                dash: Vec::new(),
                                cap: None,
                                join: None,
                            },
                        },
                    });
                    paths.push(SvgTextPath {
                        carrier: Some(index),
                        ..path
                    });
                    continue;
                }
            }
            paths.push(path);
        }
        let text_length = attr(node, "textLength")
            .and_then(|t| ctx.units.parse(t, Axis::X))
            .filter(|l| *l > 0.0);
        Some(SvgNode::Text {
            state: Reader::state(node, &ctx.style, true),
            text: Box::new(SvgText {
                matrix: ctx.matrix,
                chars,
                styles: walk.styles,
                root_style,
                paths,
                flow,
                text_length,
            }),
        })
    }

    /// How the carrier of the `textPath` `node` will be found, and its offset (criteria 93 to 95).
    fn carrier_request(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> (SvgTextPath, CarrierRequest) {
        let side_right = node.attribute("side").map(str::trim) == Some("right");
        let offset_text = attr(node, "startOffset").map(str::trim);
        let straight = (
            SvgTextPath {
                carrier: None,
                start_offset: 0.0,
                side_right,
                length: 0.0,
            },
            CarrierRequest::None,
        );
        let Some(target) = href(node)
            .and_then(|h| h.strip_prefix('#'))
            .and_then(|id| self.ids.get(id).copied())
        else {
            return straight;
        };
        if !(self.is_svg(target) && CARRIERS.contains(&target.tag_name().name())) {
            return straight;
        }
        let own = attr(target, "transform")
            .and_then(|t| t.parse::<svgtypes::Transform>().ok())
            .map_or(Affine::IDENTITY, |t| {
                Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
            });
        let mut points = self.points;
        let Ok(kind) = leaf_kind(target, own, &ctx.units, &mut points) else {
            return straight;
        };
        let Some(user) = kind.outline_mm() else {
            return straight;
        };
        let first = first_outline(&user);
        let length = first.segments().map(|s| s.arclen(1e-6)).sum::<f64>();
        let path_length = attr(target, "pathLength")
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|l| l.is_finite() && *l > 0.0);
        let start_offset = match offset_text {
            Some(t) if t.ends_with('%') => t
                .trim_end_matches('%')
                .trim()
                .parse::<f64>()
                .map_or(0.0, |p| p / 100.0 * length),
            Some(t) => ctx
                .units
                .parse(t, Axis::X)
                .map_or(0.0, |v| path_length.map_or(v, |p| v * length / p)),
            None => 0.0,
        };
        let path = SvgTextPath {
            carrier: None,
            start_offset,
            side_right,
            length,
        };
        let same_space = self
            .natural_parent_matrix(target)
            .is_some_and(|parent| close(parent, ctx.matrix));
        if same_space && ctx.copy_depth == 0 {
            return (path, CarrierRequest::Drawn(target.id().get()));
        }
        (path, CarrierRequest::Guide(ctx.matrix * first))
    }

    /// The frame of an Inkscape 1.x flowed text without positioned lines: `shape-inside` (the
    /// shape's box less `shape-padding`) or `inline-size` (criterion 96 (a)).
    fn frame_of(&self, node: Node<'_, '_>, ctx: &Ctx) -> Option<FlowBox> {
        let text = &ctx.style.text;
        if let Some(id) = &text.shape_inside {
            let shape = self.ids.get(&**id).copied()?;
            let b = self.shape_box(shape, &ctx.units)?;
            let pad = text.shape_padding;
            return Some(FlowBox {
                left: b.x0 + pad,
                top: b.y0 + pad,
                width: b.width() - 2.0 * pad,
                first_baseline: None,
                by_anchor: false,
            });
        }
        let width = text.inline_size?;
        let first = |name: &str, axis| {
            attr(node, name)
                .and_then(|t| t.split([',', ' ']).find(|s| !s.is_empty()))
                .and_then(|t| ctx.units.parse(t, axis))
                .unwrap_or(0.0)
        };
        let y = first("y", Axis::Y);
        Some(FlowBox {
            left: first("x", Axis::X),
            top: y,
            width,
            first_baseline: Some(y),
            by_anchor: true,
        })
    }

    /// The box in user units of a drawn shape element (its own transform applied).
    fn shape_box(&self, shape: Node<'_, '_>, units: &Units) -> Option<kurbo::Rect> {
        if !(self.is_svg(shape) && CARRIERS.contains(&shape.tag_name().name())) {
            return None;
        }
        let own = attr(shape, "transform")
            .and_then(|t| t.parse::<svgtypes::Transform>().ok())
            .map_or(Affine::IDENTITY, |t| {
                Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
            });
        let mut points = usize::MAX;
        let kind = leaf_kind(shape, own, units, &mut points).ok()?;
        Some(kind.outline_mm()?.bounding_box())
    }
}

/// A line break in the walk.
fn break_segment(style: usize, stack: &[usize]) -> Segment {
    Segment {
        text: None,
        style,
        path: None,
        elements: stack.to_vec(),
        space: Space::Normal,
        transform: crate::text_style::TextTransform::None,
    }
}

/// The white-space handling of `node` under its parent's `parent`.
fn space_of(node: Node<'_, '_>, parent: Space, style: &Computed) -> Space {
    if style.text.pre {
        return Space::Pre;
    }
    match node.attribute((XML_NS, "space")) {
        Some("preserve") => Space::Preserve,
        Some("default") => Space::Normal,
        _ if parent == Space::Pre => Space::Normal,
        _ => parent,
    }
}

/// A list attribute of `node` read by `read`, at most [`MAX_LIST_NUMBERS`] values; an unreadable
/// item ends the list.
fn numbers(node: Node<'_, '_>, name: &str, read: impl Fn(&str) -> Option<f64>) -> Vec<f64> {
    let Some(text) = attr(node, name) else {
        return Vec::new();
    };
    text.split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|item| !item.is_empty())
        .take(MAX_LIST_NUMBERS)
        .map_while(|item| read(item).filter(|v| v.is_finite()))
        .collect()
}

/// The first outline of `path`.
fn first_outline(path: &kurbo::BezPath) -> kurbo::BezPath {
    let mut out = kurbo::BezPath::new();
    for (index, el) in path.elements().iter().enumerate() {
        if index > 0 && matches!(el, kurbo::PathEl::MoveTo(_)) {
            break;
        }
        out.push(*el);
    }
    out
}

/// Whether two maps are the same within rounding.
fn close(a: Affine, b: Affine) -> bool {
    a.as_coeffs()
        .iter()
        .zip(b.as_coeffs())
        .all(|(x, y)| (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0))
}
