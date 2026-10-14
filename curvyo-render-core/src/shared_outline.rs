//! The tessellation of a linked clone's root, made once per artwork build and mapped onto each
//! clone that may share it (`specs/0049-linked-clone/adrs.md` decision 3, criterion 44): a clone
//! whose composed map is rigid, whose root stroke has no dashes and no markers, and whose style is
//! the root's apart from the paint opacities. Every other clone is tessellated from its resolved
//! outline as before.

use std::collections::HashMap;

use curvyo_document_core::{Affine, CloneSnapshot, PathSnapshot, Style, ViewTransform};

use crate::artwork::{fill_layer, paint, solid_stroke_layer, stroke_params};
use crate::color::RgbaColor;
use crate::glyphs::DrawList;
use crate::stroke::OutlineRef;

/// How far from orthonormal a rigid map may be.
const RIGID_EPSILON: f64 = 1e-9;

/// One root's layers in root coordinates, painted opaque; `None` where the root has none.
struct RootLayers {
    fill: Option<DrawList>,
    stroke: Option<DrawList>,
}

/// The roots tessellated in one artwork build, by the address of their shared outline.
#[derive(Default)]
pub(crate) struct SharedOutlines {
    roots: HashMap<*const PathSnapshot, RootLayers>,
}

#[cfg(test)]
thread_local! {
    /// How many roots this thread has tessellated for sharing.
    static ROOT_TESSELLATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl SharedOutlines {
    /// Draws `clone` from its root's shared tessellation and returns `true`, or returns `false`
    /// (nothing drawn) when the clone may not share it.
    pub(crate) fn draw(
        &mut self,
        list: &mut DrawList,
        clone: &CloneSnapshot,
        view: ViewTransform,
    ) -> bool {
        let root = &clone.chain.root_outline;
        let composed = clone.chain.composed;
        if !is_rigid(composed)
            || !plain_root(&root.style)
            || !same_but_opacity(&clone.resolved.style, &root.style)
        {
            return false;
        }
        let layers = self
            .roots
            .entry(std::sync::Arc::as_ptr(root))
            .or_insert_with(|| tessellate(root, view));
        let style = &clone.resolved.style;
        if style.fill.paints() {
            let color = paint(style.fill.color, style.fill.opacity);
            if color.a > 0
                && let Some(fill) = &layers.fill
            {
                list.extend_artwork_mapped(fill, composed, color);
            }
        }
        if style.stroke.enabled {
            let color = paint(style.stroke.color, style.stroke.opacity);
            if color.a > 0
                && let Some(stroke) = &layers.stroke
            {
                list.extend_artwork_mapped(stroke, composed, color);
            }
        }
        true
    }
}

/// The number of roots tessellated for sharing on this thread (test builds only).
#[cfg(test)]
pub(crate) fn root_tessellations() -> usize {
    ROOT_TESSELLATIONS.with(std::cell::Cell::get)
}

/// Whether the linear part of `m` is orthonormal (a rotation or a reflection) within
/// [`RIGID_EPSILON`]: distances are kept, so the flattening tolerance holds after the map.
fn is_rigid(m: Affine) -> bool {
    let near = |v: f64, target: f64| (v - target).abs() <= RIGID_EPSILON;
    near(m.a.mul_add(m.a, m.b * m.b), 1.0)
        && near(m.c.mul_add(m.c, m.d * m.d), 1.0)
        && near(m.a.mul_add(m.c, m.b * m.d), 0.0)
}

/// Whether a root's stroke draws no dashes and no markers, so the frame budgets of both are
/// charged as before.
fn plain_root(style: &Style) -> bool {
    style.stroke.dash.is_solid() && style.stroke.markers == curvyo_document_core::Markers::default()
}

/// Whether two styles differ at most in the fill and stroke opacity.
fn same_but_opacity(a: &Style, b: &Style) -> bool {
    let mut a = a.clone();
    a.fill.opacity = b.fill.opacity;
    a.stroke.opacity = b.stroke.opacity;
    a == *b
}

/// The root's fill and solid stroke in root coordinates, painted opaque.
fn tessellate(root: &PathSnapshot, view: ViewTransform) -> RootLayers {
    #[cfg(test)]
    ROOT_TESSELLATIONS.with(|count| count.set(count.get() + 1));
    let outlines: Vec<OutlineRef<'_>> = root
        .subpaths()
        .map(|subpath| OutlineRef {
            anchors: subpath.anchors,
            closed: subpath.closed,
        })
        .collect();
    let opaque = |color| RgbaColor::from(color).with_alpha(255);
    let fill = fill_layer(&outlines, opaque(root.style.fill.color), view);
    let stroke = stroke_params(&root.style.stroke, opaque(root.style.stroke.color), view)
        .and_then(|params| solid_stroke_layer(&outlines, &params));
    RootLayers { fill, stroke }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{
        AnchorId, AnchorKind, Color, Document, NewAnchor, NodeId, ObjectSnapshot, Opacity, Point,
        StyleEdit, Vec2,
    };

    use super::*;
    use crate::artwork::build_artwork;

    fn view() -> ViewTransform {
        ViewTransform::new(3.0, Point::new(0.0, 0.0))
    }

    fn circle(d: &Document, n: u64, cx: f64, closed: bool) -> NodeId {
        let k = 5.0 * 0.552_284_749_8;
        let node = |i: u64, x: f64, y: f64, h: Vec2| NewAnchor {
            id: AnchorId::new(1, n * 10 + i),
            point: Point::new(x, y),
            handle_in: Vec2::new(-h.x, -h.y),
            handle_out: h,
            kind: AnchorKind::Symmetric,
        };
        d.create_path(
            &[
                node(0, cx + 5.0, 10.0, Vec2::new(0.0, k)),
                node(1, cx, 15.0, Vec2::new(-k, 0.0)),
                node(2, cx - 5.0, 10.0, Vec2::new(0.0, -k)),
                node(3, cx, 5.0, Vec2::new(k, 0.0)),
            ],
            closed,
        )
    }

    /// Layer count, colours, bounds and area per layer of a draw list.
    fn summary(list: &DrawList) -> Vec<(usize, Vec<RgbaColor>, [f64; 4], f64)> {
        let mut out = Vec::new();
        let mut start = 0;
        for &end in list.layers() {
            let triangles = &list.triangles[start..end];
            let mut colors: Vec<RgbaColor> = triangles.iter().map(|v| v.color).collect();
            colors.dedup();
            let mut bounds = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            let mut area = 0.0;
            for t in triangles.chunks(3) {
                for v in t {
                    bounds = [
                        bounds[0].min(v.position.x),
                        bounds[1].min(v.position.y),
                        bounds[2].max(v.position.x),
                        bounds[3].max(v.position.y),
                    ];
                }
                let (a, b, c) = (t[0].position, t[1].position, t[2].position);
                area += ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0;
            }
            out.push((end - start, colors, bounds, area));
            start = end;
        }
        out
    }

    fn unshared(objects: &[ObjectSnapshot]) -> Vec<ObjectSnapshot> {
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Clone(clone) => ObjectSnapshot::Path(clone.resolved.clone()),
                other => other.clone(),
            })
            .collect()
    }

    /// The tessellator works in `f32`, so coordinates agree to its precision: about 1e-5 of
    /// their size (8e-6 mm at 60 mm), not to 1e-6 mm absolute.
    fn assert_same(shared: &DrawList, plain: &DrawList) {
        let (a, b) = (summary(shared), summary(plain));
        assert_eq!(a.len(), b.len(), "layer count");
        for ((_, ca, ba, aa), (_, cb, bb, ab)) in a.iter().zip(&b) {
            assert_eq!(ca, cb, "colours");
            for (x, y) in ba.iter().zip(bb) {
                assert!(
                    (x - y).abs() < 1e-5 * y.abs().max(1.0),
                    "bounds {ba:?} {bb:?}"
                );
            }
            assert!((aa - ab).abs() < 1e-5 * ab.max(1.0), "area {aa} {ab}");
        }
    }

    #[test]
    fn shared_and_unshared_builds_agree_for_rigid_maps() {
        let d = Document::new(1);
        let root = circle(&d, 1, 20.0, true);
        d.edit_style(&[root], &StyleEdit::FillEnabled(true))
            .unwrap();
        d.edit_style(
            &[root],
            &StyleEdit::FillColor(Color {
                r: 200,
                g: 10,
                b: 10,
            }),
        )
        .unwrap();
        let moved = d.linked_clone(&[root], &[]).unwrap()[0];
        d.translate_objects(&[moved], Vec2::new(30.0, 4.0)).unwrap();
        let turned = d.linked_clone(&[root], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(clone) = d.object(turned).unwrap() else {
            panic!("a clone")
        };
        d.rotate_object(&ObjectSnapshot::Clone(clone.mapped_by(
            Affine::rotation_about(
                Point::new(0.0, 0.0),
                curvyo_document_core::Angle::from_radians(0.6),
            ),
        )))
        .unwrap();
        let mirrored = d.linked_clone(&[root], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(clone) = d.object(mirrored).unwrap() else {
            panic!("a clone")
        };
        d.rotate_object(&ObjectSnapshot::Clone(
            clone.mapped_by(Affine::new(-1.0, 0.0, 0.0, 1.0, 80.0, 0.0)),
        ))
        .unwrap();
        let open = circle(&d, 2, 60.0, false);
        d.edit_style(&[open], &StyleEdit::FillEnabled(true))
            .unwrap();
        let open_clone = d.linked_clone(&[open], &[]).unwrap()[0];
        d.translate_objects(&[open_clone], Vec2::new(0.0, 40.0))
            .unwrap();
        let mut objects = d.read_objects().objects.to_vec();
        // A faded clone (0050 changes only the opacities).
        if let ObjectSnapshot::Clone(clone) = &mut objects[1] {
            clone.resolved.style.fill.opacity = Opacity::new(0.5).unwrap();
        }
        let shared = build_artwork(&objects, view());
        let plain = build_artwork(&unshared(&objects), view());
        assert_same(&shared, &plain);
    }

    #[test]
    fn a_compound_root_keeps_its_hole_when_shared() {
        let d = Document::new(1);
        let outer = circle(&d, 1, 20.0, true);
        let ObjectSnapshot::Path(outer_path) = d.object(outer).unwrap() else {
            panic!("a path")
        };
        let ring: Vec<NewAnchor> = outer_path
            .anchors
            .iter()
            .enumerate()
            .map(|(k, a)| NewAnchor {
                id: AnchorId::new(5, k as u64),
                ..*a
            })
            .collect();
        let hole: Vec<NewAnchor> = outer_path
            .anchors
            .iter()
            .rev()
            .enumerate()
            .map(|(k, a)| NewAnchor {
                id: AnchorId::new(6, k as u64),
                point: Point::new(
                    20.0 + (a.point.x - 20.0) * 0.5,
                    10.0 + (a.point.y - 10.0) * 0.5,
                ),
                handle_in: Vec2::new(a.handle_out.x * 0.5, a.handle_out.y * 0.5),
                handle_out: Vec2::new(a.handle_in.x * 0.5, a.handle_in.y * 0.5),
                kind: a.kind,
            })
            .collect();
        let compound = d
            .replace_with_path(&[outer], outer, &[(ring, true), (hole, true)], "combine")
            .unwrap();
        d.edit_style(&[compound], &StyleEdit::FillEnabled(true))
            .unwrap();
        let c = d.linked_clone(&[compound], &[]).unwrap()[0];
        d.translate_objects(&[c], Vec2::new(40.0, 0.0)).unwrap();
        let objects = d.read_objects().objects;
        assert_same(
            &build_artwork(&objects, view()),
            &build_artwork(&unshared(&objects), view()),
        );
    }

    #[test]
    fn one_root_tessellation_serves_a_thousand_rigid_clones_and_others_fall_back() {
        let d = Document::new(1);
        let root = circle(&d, 1, 20.0, true);
        for _ in 0..1_000 {
            d.linked_clone(&[root], &[]).unwrap();
        }
        let objects = d.read_objects().objects;
        let before = root_tessellations();
        let _ = build_artwork(&objects, view());
        assert_eq!(root_tessellations() - before, 1);
        // A scaled clone, a dashed root and a clone of another style do not share.
        let scaled = d.linked_clone(&[root], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(clone) = d.object(scaled).unwrap() else {
            panic!("a clone")
        };
        let mut shared = SharedOutlines::default();
        let mut list = DrawList::default();
        assert!(!shared.draw(&mut list, &clone.mapped_by(Affine::scale(2.0, 1.0)), view()));
        let mut restyled = clone.clone();
        restyled.resolved.style.stroke.width = curvyo_document_core::Length::from_mm(3.0);
        assert!(!shared.draw(&mut list, &restyled, view()));
        d.edit_style(
            &[root],
            &StyleEdit::StrokeDash(curvyo_document_core::DashPattern::new(vec![2.0, 1.0]).unwrap()),
        )
        .unwrap();
        let ObjectSnapshot::Clone(dashed) = d.object(scaled).unwrap() else {
            panic!("a clone")
        };
        assert!(!shared.draw(&mut list, &dashed, view()));
    }
}
