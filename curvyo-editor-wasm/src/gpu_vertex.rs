//! The draw list in the GPU's vertex shape: positions relative to the view origin in `f32`,
//! normalised colours and layer depths. `gpu.rs` uploads what it builds; the wasm probe times it
//! (`specs/0126-performance-pass` phase 4).

use curvyo_document_core::Point;
use curvyo_render_core::DrawList;

/// One triangle-list vertex in the shape the GPU pipeline (`gpu_pipeline.rs`) expects:
/// a clip-ready `f32` position (before the per-frame screen transform),
/// a normalized `f32` color and the depth of the vertex's artwork layer.
#[repr(C)]
#[derive(Clone, Copy)]
#[cfg_attr(target_arch = "wasm32", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub(crate) struct GpuVertex {
    position: [f32; 2],
    color: [f32; 4],
    depth: f32,
}

/// Converts a draw-list vertex to the GPU's own vertex shape, **relative
/// to `origin`** (the document point currently at screen pixel `(0, 0)`)
/// — subtracted in `f64`, before the `f32` cast
/// (`specs/0004-canvas-navigation-and-selection/adrs.md`: "The GPU upload
/// subtracts the view origin in `f64` before the `f32` cast"). Casting
/// an *absolute* document-mm position straight to `f32` (the previous
/// behaviour) loses precision proportional to its distance from the
/// document origin; panning far from the origin at high zoom (pan is
/// unbounded, and nothing in this spec limits it) made that loss
/// catastrophic once it approached half the stroke width. Subtracting
/// `origin` first keeps the `f32` error proportional to distance from
/// the *viewport's* origin — i.e. roughly proportional to distance on
/// screen, in pixels — regardless of where the document origin is.
/// [`ScreenTransform`]'s own offset no longer needs `origin` at all,
/// since every vertex arrives already shifted.
#[must_use]
pub(crate) fn to_gpu_vertex(
    vertex: curvyo_render_core::Vertex,
    origin: Point,
    depth: f32,
) -> GpuVertex {
    let relative_x = vertex.position.x - origin.x;
    let relative_y = vertex.position.y - origin.y;
    #[allow(clippy::cast_possible_truncation)]
    let position = [relative_x as f32, relative_y as f32];
    let color = [
        f32::from(vertex.color.r) / 255.0,
        f32::from(vertex.color.g) / 255.0,
        f32::from(vertex.color.b) / 255.0,
        f32::from(vertex.color.a) / 255.0,
    ];
    GpuVertex {
        position,
        color,
        depth,
    }
}

/// Every draw-list vertex in the GPU's shape, shifted by `origin` (the
/// document point at screen pixel (0, 0)) in `f64` before its own `f32` cast
/// (`to_gpu_vertex`'s doc comment), tagged with its layer's depth.
#[must_use]
pub(crate) fn gpu_vertices(
    draw_list: &DrawList,
    origin: curvyo_document_core::Point,
) -> Vec<GpuVertex> {
    draw_list
        .triangles
        .iter()
        .copied()
        .zip(draw_list.vertex_depths())
        .map(|(vertex, depth)| to_gpu_vertex(vertex, origin, depth))
        .collect()
}
