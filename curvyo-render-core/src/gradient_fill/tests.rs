//! The gradient spans, the stop table and the shader maths against `colour_at`
//! (`specs/0124-gradient-fill` criteria 5, 9 to 11, 24 e). Tolerance: the shader works in `f32`
//! on 8-bit premultiplied stops, `colour_at` in integers on a `1 / 65,536` grid, so a channel may
//! differ by 1/255 plus `f32` rounding ([`TOLERANCE`]); never on the far side of a hard edge.

use curvyo_document_core::{
    AnchorId, Color, Document, FillPaint, GradientKind, GradientStop, GradientStops, Length,
    NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds, StopOffset, StyleEdit, ViewTransform,
    over_white_grey,
};

use super::shader_reference::shade;
use super::*;
use crate::RgbaColor;
use crate::artwork::build_artwork;

/// One 8-bit step plus `f32` rounding, in `0..=255` units.
const TOLERANCE: f32 = 1.0 + 1e-3;

/// [`TOLERANCE`] on stops 1/255 apart: `colour_at` quantises `t` to `1 / 65,536`, which on such
/// a gap moves a channel by up to `255 * 255 / 65,536`, about one more step.
const STEEP_TOLERANCE: f32 = 2.0 + 1e-3;

fn stop(r: u8, g: u8, b: u8, alpha: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn rect(document: &Document, x: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn shaded(document: &Document, id: NodeId, kind: GradientKind, stops: Vec<GradientStop>) {
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    document
        .edit_style(&[id], &StyleEdit::GradientKind(kind))
        .unwrap();
    document
        .edit_style(
            &[id],
            &StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
        )
        .unwrap();
}

/// What the GPU writes at `p`: the attributes interpolated over the triangle of the span that
/// holds `p`, then the shader's lookup; premultiplied, `0..=255`.
#[allow(clippy::many_single_char_names)] // barycentric weights over a triangle a, b, c
fn gpu_at(list: &DrawList, p: Point) -> [f32; 4] {
    let attributes = list.gradient_attributes();
    let texels = list.stop_texture().texels;
    for span in list.gradients() {
        for tri in list.triangles[span.vertices()].chunks(3).enumerate() {
            let (i, v) = tri;
            let (a, b, c) = (v[0].position, v[1].position, v[2].position);
            let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
            if det.abs() < 1e-12 {
                continue;
            }
            let l0 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / det;
            let l1 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / det;
            let l2 = 1.0 - l0 - l1;
            if l0 < -1e-9 || l1 < -1e-9 || l2 < -1e-9 {
                continue;
            }
            let base = span.vertices().start + 3 * i;
            let at = |k: usize| attributes[base + k];
            #[allow(clippy::cast_possible_truncation)]
            let g: [f32; 4] = std::array::from_fn(|j| {
                if j < 2 {
                    (l0 * f64::from(at(0)[j]) + l1 * f64::from(at(1)[j]) + l2 * f64::from(at(2)[j]))
                        as f32
                } else {
                    at(0)[j]
                }
            });
            let alpha = f32::from(v[0].color.a) / 255.0;
            let s = shade(g, alpha, &texels);
            return [
                s[0] * s[3] * 255.0,
                s[1] * s[3] * 255.0,
                s[2] * s[3] * 255.0,
                s[3] * 255.0,
            ];
        }
    }
    panic!("{p:?} is in no gradient triangle");
}

fn close(gpu: [f32; 4], cpu: PremultipliedRgba8) -> bool {
    within(gpu, cpu, TOLERANCE)
}

fn within(gpu: [f32; 4], cpu: PremultipliedRgba8, tolerance: f32) -> bool {
    let cpu = [cpu.r, cpu.g, cpu.b, cpu.a].map(f32::from);
    gpu.iter().zip(cpu).all(|(g, c)| (g - c).abs() <= tolerance)
}

fn object(document: &Document, id: NodeId) -> ObjectSnapshot {
    document.object(id).unwrap()
}

/// Criterion 5, first test, through the drawing: black to white over 100 mm reads 50 % grey at
/// x = 50, and every sample agrees with `colour_at`.
#[test]
fn black_to_white_draws_half_grey_in_the_middle() {
    let document = Document::new(1);
    let id = rect(&document, 0.0, 100.0, 40.0);
    shaded(
        &document,
        id,
        GradientKind::Linear,
        vec![stop(0, 0, 0, 255, 0.0), stop(255, 255, 255, 255, 1.0)],
    );
    let o = object(&document, id);
    let list = build_artwork(std::slice::from_ref(&o), ViewTransform::identity());
    assert_eq!(list.gradients().len(), 1);
    let middle = gpu_at(&list, Point::new(50.0, 13.0));
    assert!((middle[0] - 127.5).abs() <= TOLERANCE, "{middle:?}");
    let placed = placed_gradient(&o).unwrap();
    for i in 1..20 {
        let p = Point::new(f64::from(i) * 5.0 - 0.3, 3.0 + f64::from(i));
        assert!(close(gpu_at(&list, p), placed.colour_at(p)), "{p:?}");
    }
}

/// Criterion 5, second test: black, red, white read pure red at x = 50.
#[test]
fn three_stops_draw_the_middle_stop_exactly() {
    let document = Document::new(1);
    let id = rect(&document, 0.0, 100.0, 40.0);
    shaded(
        &document,
        id,
        GradientKind::Linear,
        vec![
            stop(0, 0, 0, 255, 0.0),
            stop(255, 0, 0, 255, 0.5),
            stop(255, 255, 255, 255, 1.0),
        ],
    );
    let list = build_artwork(&[object(&document, id)], ViewTransform::identity());
    let red = gpu_at(&list, Point::new(50.0, 20.0));
    assert!(
        (red[0] - 255.0).abs() <= TOLERANCE && red[1].abs() <= TOLERANCE,
        "{red:?}"
    );
}

/// Criterion 5, third test: a radial draws one colour on every point of a circle, the same as
/// `colour_at`, translucent stops included.
#[test]
fn a_radial_draws_one_colour_on_a_circle() {
    let document = Document::new(1);
    let id = rect(&document, 0.0, 100.0, 100.0);
    shaded(
        &document,
        id,
        GradientKind::Radial,
        vec![stop(10, 200, 30, 255, 0.0), stop(250, 0, 90, 100, 1.0)],
    );
    let o = object(&document, id);
    let list = build_artwork(std::slice::from_ref(&o), ViewTransform::identity());
    assert!(list.gradients()[0].is_radial());
    let placed = placed_gradient(&o).unwrap();
    let first = gpu_at(&list, Point::new(80.0, 50.0));
    for (dx, dy) in [
        (0.0, 30.0),
        (-30.0, 0.0),
        (0.0, -30.0),
        (21.213_2, 21.213_2),
    ] {
        let p = Point::new(50.0 + dx, 50.0 + dy);
        let here = gpu_at(&list, p);
        assert!(close(here, placed.colour_at(p)), "{p:?}");
        assert!(
            here.iter()
                .zip(first)
                .all(|(a, b)| (a - b).abs() <= TOLERANCE),
            "{p:?}"
        );
    }
}

/// A hard edge is hard: the later stop wins from its offset on.
#[test]
fn a_hard_edge_stays_hard() {
    let texels = |stops: &[GradientStop]| {
        let stops = GradientStops::new(stops.to_vec()).unwrap();
        let mut list = DrawList::default();
        list.gradient_stops = stop_texels(&stops, false);
        list.stop_texture().texels
    };
    let table = texels(&[
        stop(0, 0, 0, 255, 0.0),
        stop(0, 0, 0, 255, 0.5),
        stop(255, 255, 255, 255, 0.5),
        stop(255, 255, 255, 255, 1.0),
    ]);
    assert_eq!(shade([0.5, 0.0, 0.0, 4.0], 1.0, &table)[0], 1.0);
    assert_eq!(shade([0.499, 0.0, 0.0, 4.0], 1.0, &table)[0], 0.0);
}

/// Criterion 24 e: 256 stops draw as `colour_at` reads them, and the binary search needs no
/// more than eight halvings.
#[test]
fn two_hundred_fifty_six_stops_agree_with_colour_at() {
    let stops: Vec<GradientStop> = (0..256_u32)
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)]
            let c = (i * 97 % 256) as u8;
            stop(c, 255 - c, c / 2, 255 - c / 3, f64::from(i) / 255.0)
        })
        .collect();
    let list_stops = GradientStops::new(stops).unwrap();
    let mut list = DrawList::default();
    list.gradient_stops = stop_texels(&list_stops, false);
    let table = list.stop_texture().texels;
    for i in 0..=1000 {
        let t = f64::from(i) / 1000.0 + 0.000_37;
        #[allow(clippy::cast_possible_truncation)]
        let s = shade([t as f32, 0.0, 0.0, 256.0], 1.0, &table);
        let gpu = [
            s[0] * s[3] * 255.0,
            s[1] * s[3] * 255.0,
            s[2] * s[3] * 255.0,
            s[3] * 255.0,
        ];
        assert!(
            within(gpu, list_stops.colour_at(t), STEEP_TOLERANCE),
            "{t}: {gpu:?} {:?}",
            list_stops.colour_at(t)
        );
    }
}

/// Criterion 10: grey stops draw the grey of the colour over white, within one step.
#[test]
fn grey_stops_draw_the_grey_of_the_shade() {
    let stops = GradientStops::new(vec![
        stop(200, 30, 90, 180, 0.0),
        stop(20, 240, 10, 60, 0.4),
        stop(255, 255, 0, 255, 1.0),
    ])
    .unwrap();
    let mut list = DrawList::default();
    list.gradient_stops = stop_texels(&stops, true);
    let table = list.stop_texture().texels;
    for i in 0..=50 {
        let t = f64::from(i) / 50.0;
        #[allow(clippy::cast_possible_truncation)]
        let s = shade([t as f32, 0.0, 0.0, 3.0], 1.0, &table);
        assert!((s[3] - 1.0).abs() < 1e-6, "opaque");
        assert!((s[0] - s[1]).abs() < 1e-6 && (s[1] - s[2]).abs() < 1e-6);
        let expected = f32::from(over_white_grey(stops.colour_at(t)));
        assert!(
            (s[0] * 255.0 - expected).abs() <= 1.5,
            "{t}: {s:?} {expected}"
        );
    }
}

#[test]
fn the_greyed_build_greys_only_the_named_leaves() {
    let document = Document::new(1);
    let (a, b) = (
        rect(&document, 0.0, 10.0, 10.0),
        rect(&document, 20.0, 10.0, 10.0),
    );
    for id in [a, b] {
        shaded(
            &document,
            id,
            GradientKind::Linear,
            vec![stop(255, 0, 0, 255, 0.0), stop(0, 0, 255, 255, 1.0)],
        );
    }
    let objects = [object(&document, a), object(&document, b)];
    let list = crate::build_artwork_greyed(
        &objects,
        &std::collections::HashSet::new(),
        &|id| id == b,
        ViewTransform::identity(),
    );
    let spans = list.gradients();
    let colour = |span: &GradientSpan| list.gradient_stops()[span.stops().start as usize].colour;
    assert_eq!(colour(&spans[0]), [1.0, 0.0, 0.0, 1.0]);
    let grey = colour(&spans[1]);
    assert_eq!([grey[0], grey[0], grey[0], 1.0], grey);
    assert!(grey[0] > 0.0);
}

/// Equal stop lists share one table range; the spans cover exactly the fill's vertices, the
/// stroke is a later layer over it.
#[test]
fn equal_lists_share_one_range_and_the_stroke_is_drawn_over() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|i| rect(&document, f64::from(i) * 20.0, 10.0, 10.0))
        .collect();
    for id in &ids {
        shaded(
            &document,
            *id,
            GradientKind::Linear,
            vec![stop(1, 2, 3, 255, 0.0), stop(4, 5, 6, 255, 1.0)],
        );
    }
    let objects: Vec<_> = ids.iter().map(|id| object(&document, *id)).collect();
    let list = build_artwork(&objects, ViewTransform::identity());
    assert_eq!(list.gradients().len(), 3);
    assert_eq!(list.gradient_stops().len(), 2, "one shared list");
    let span = &list.gradients()[0];
    assert_eq!(
        list.layers()[0],
        span.vertices().end,
        "the fill is its own layer"
    );
    assert!(list.layers()[1] > span.vertices().end, "the stroke follows");
    let attributes = list.gradient_attributes();
    assert_eq!(attributes.len(), list.triangles.len());
    let stroke_vertex = list.layers()[0];
    assert_eq!(attributes[stroke_vertex], [0.0; 4], "a solid vertex");
    assert!(
        list.triangles[span.vertices()]
            .iter()
            .all(|v| v.color == RgbaColor::WHITE),
        "white, the shader multiplies"
    );
}

/// Appending keeps each span on its vertices and stops; moving the vertices keeps the shade.
#[test]
fn extend_and_map_keep_the_shade() {
    let document = Document::new(1);
    let (a, b) = (
        rect(&document, 0.0, 10.0, 10.0),
        rect(&document, 20.0, 10.0, 10.0),
    );
    shaded(
        &document,
        a,
        GradientKind::Linear,
        vec![stop(0, 0, 0, 255, 0.0), stop(9, 9, 9, 255, 1.0)],
    );
    shaded(
        &document,
        b,
        GradientKind::Radial,
        vec![stop(1, 1, 1, 255, 0.0), stop(2, 2, 2, 255, 1.0)],
    );
    let first = build_artwork(&[object(&document, a)], ViewTransform::identity());
    let second = build_artwork(&[object(&document, b)], ViewTransform::identity());
    let before = second.gradient_attributes();
    let mut list = first.clone();
    let base = list.triangles.len();
    list.extend(second.clone());
    let span = &list.gradients()[1];
    assert_eq!(
        span.vertices().start,
        base + second.gradients()[0].vertices().start
    );
    assert_eq!(span.stops(), 2..4);
    let after = list.gradient_attributes();
    assert_eq!(
        after[base..]
            .iter()
            .map(|a| [a[0], a[1]])
            .collect::<Vec<_>>(),
        before.iter().map(|a| [a[0], a[1]]).collect::<Vec<_>>()
    );
    assert_eq!(after[span.vertices().start][2], 2.0, "first stop shifted");
    let mut moved = list.clone();
    moved.map_positions(|p| Point::new(p.x * 2.0 + 7.0, p.y));
    assert_eq!(moved.gradient_attributes(), after);
}

/// An open path's fill is closed by a chord and shaded; its vertices are those of its Colour fill.
#[test]
fn an_open_path_is_shaded_like_its_colour_fill() {
    let document = Document::new(1);
    let id = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(20.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(20.0, 20.0)),
        ],
        false,
    );
    document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    let colour = build_artwork(&[object(&document, id)], ViewTransform::identity());
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    let gradient = build_artwork(&[object(&document, id)], ViewTransform::identity());
    let span = &gradient.gradients()[0];
    assert_eq!(span.vertices().len(), colour.layers()[0]);
}

#[test]
fn the_texture_has_two_texels_per_stop_in_rows_of_2048() {
    let mut list = DrawList::default();
    assert_eq!(list.stop_texture().height, 1, "never empty");
    list.gradient_stops = vec![
        StopTexel {
            offset: 0.25,
            colour: [0.1, 0.2, 0.3, 0.4]
        };
        1025
    ];
    let texture = list.stop_texture();
    assert_eq!((texture.width, texture.height), (2048, 2));
    assert_eq!(texture.texels.len(), 4096);
    assert_eq!(
        texture.texels[2048],
        [0.25, 0.0, 0.0, 0.0],
        "stop 1024 opens row 2"
    );
    assert_eq!(texture.texels[2049], [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(texture.texels[2050], [0.0; 4]);
}

/// A faded leaf shows its gradient's alpha times its opacity.
#[test]
fn fading_a_gradient_fades_its_stops() {
    let mut fill = curvyo_document_core::Fill {
        gradient: Some(curvyo_document_core::GradientFill::first_for(
            Color { r: 1, g: 2, b: 3 },
            200,
        )),
        ..curvyo_document_core::Fill::default()
    };
    crate::fade_fill(&mut fill, 0.5);
    let stops = fill.gradient.unwrap().stops;
    assert_eq!(
        stops.as_slice().iter().map(|s| s.alpha).collect::<Vec<_>>(),
        [100, 128]
    );
}

/// A linked clone of a gradient root shares the root's tessellation and is shaded in its own box.
#[test]
fn a_rigid_clone_is_shaded_in_its_own_box() {
    let document = Document::new(1);
    let corner = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let id = document.create_path(
        &[
            corner(1, 0.0, 0.0),
            corner(2, 10.0, 0.0),
            corner(3, 10.0, 10.0),
            corner(4, 0.0, 10.0),
        ],
        true,
    );
    document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    shaded(
        &document,
        id,
        GradientKind::Linear,
        vec![stop(0, 0, 0, 255, 0.0), stop(255, 255, 255, 255, 1.0)],
    );
    let clone = document.linked_clone(&[id], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(snapshot) = object(&document, clone) else {
        panic!("a clone")
    };
    let c = ObjectSnapshot::Clone(
        snapshot.mapped_by(curvyo_document_core::Affine::translation(
            curvyo_document_core::Vec2::new(50.0, 0.0),
        )),
    );
    let list = build_artwork(std::slice::from_ref(&c), ViewTransform::identity());
    assert_eq!(list.gradients().len(), 1);
    let left = gpu_at(&list, Point::new(50.5, 5.0));
    let right = gpu_at(&list, Point::new(59.5, 5.0));
    assert!(left[0] < 20.0 && right[0] > 235.0, "{left:?} {right:?}");
}

/// The device limit (`max_texture_dimension_2d`): a table taller than it is cut to it, the rows
/// that fit keep their stops, and a list without gradients says so (the host then uploads
/// nothing, ADR 0067 §2).
#[test]
fn the_stop_texture_is_cut_to_the_device_limit() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|i| rect(&document, f64::from(i) * 20.0, 10.0, 10.0))
        .collect();
    let stops = |k: u8| -> Vec<GradientStop> {
        (0..1_200_u32)
            .map(|i| {
                stop(
                    k,
                    u8::try_from(i % 256).unwrap(),
                    0,
                    255,
                    f64::from(i) / 1_199.0,
                )
            })
            .collect()
    };
    for (k, id) in (0_u8..).zip(&ids) {
        shaded(&document, *id, GradientKind::Linear, stops(k));
    }
    let objects: Vec<_> = ids.iter().map(|id| object(&document, *id)).collect();
    let list = build_artwork(&objects, ViewTransform::identity());
    assert!(list.has_gradients());
    let full = list.stop_texture();
    assert_eq!(full.height, 4, "3,600 stops are 7,200 texels");
    let cut = list.stop_texture_within(2);
    assert_eq!(cut.height, 2);
    assert_eq!(cut.texels.len(), 2 * STOP_TEXTURE_WIDTH as usize);
    assert_eq!(cut.texels[..], full.texels[..cut.texels.len()]);
    assert!(!build_artwork(&[], ViewTransform::identity()).has_gradients());
}

/// Ten thousand shapes with one stop list share its rows: the table holds it once.
#[test]
fn ten_thousand_equal_lists_share_one_row() {
    let document = Document::new(1);
    let first = rect(&document, 0.0, 10.0, 10.0);
    let four = vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.3),
        stop(0, 255, 0, 255, 0.6),
        stop(0, 0, 255, 255, 1.0),
    ];
    shaded(&document, first, GradientKind::Linear, four);
    let base = object(&document, first);
    let objects: Vec<ObjectSnapshot> = (0..10_000)
        .map(|k| {
            base.translated(curvyo_document_core::Vec2::new(
                f64::from(k % 100) * 12.0,
                f64::from(k / 100) * 12.0,
            ))
        })
        .collect();
    let list = build_artwork(&objects, ViewTransform::identity());
    assert_eq!(list.gradients().len(), 10_000);
    assert_eq!(list.gradient_stops().len(), 4);
    assert_eq!(list.stop_texture().height, 1);
}
