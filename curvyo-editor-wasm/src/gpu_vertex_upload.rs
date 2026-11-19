//! Puts a frame's draw-list vertices into a GPU vertex buffer the way the current
//! [`GpuVariant`] says: a new buffer per frame (production), one kept buffer, or a kept buffer
//! written only when the vertices changed. `wasm32`-only, like `gpu.rs`.

use curvyo_document_core::{Point, ViewTransform};
use curvyo_render_core::DrawList;

use crate::gpu_variant::{GpuVariant, Upload, note};
use crate::gpu_vertex::{GpuVertex, gpu_vertices};

/// How far, in screen pixels, a cached anchor may lie from the view's origin before the
/// vertices are rebuilt around the origin again (the `f32` positions stay exact well below).
const MAX_ANCHOR_SHIFT_PX: f64 = 50_000.0;

/// One frame's vertices on the GPU.
pub(crate) struct UploadedVertices {
    /// `None` when the draw list has no triangles.
    pub(crate) buffer: Option<wgpu::Buffer>,
    /// The document point the positions are relative to: the view's origin, or a cached anchor
    /// whose shift from it the screen transform has to add.
    pub(crate) anchor: Point,
    /// The vertex count.
    pub(crate) count: usize,
}

/// The vertex buffer kept across frames and the vertices last written into it.
#[derive(Default)]
pub(crate) struct VertexUpload {
    /// The kept buffer and its size in bytes.
    kept: Option<(wgpu::Buffer, u64)>,
    /// The anchor and the bytes last written (`cache_vertices`).
    cached: Option<(Point, Vec<u8>)>,
}

impl VertexUpload {
    /// The vertices of `draw_list` relative to `origin` (or to the cached anchor) on the GPU.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw_list: &DrawList,
        view: ViewTransform,
        variant: GpuVariant,
    ) -> UploadedVertices {
        let origin = view.screen_to_document(0.0, 0.0);
        if variant.cache_vertices
            && let Some(frame) = self.unchanged(draw_list, view, origin)
        {
            return frame;
        }
        let vertices = gpu_vertices(draw_list, origin);
        let count = vertices.len();
        let bytes: &[u8] = bytemuck::cast_slice(&vertices);
        let buffer = if vertices.is_empty() {
            None
        } else if variant.reuse_vertex_buffer {
            Some(self.write(device, queue, bytes))
        } else {
            note(Upload::Created);
            note(Upload::Written);
            Some(wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("curvyo vertex buffer"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ))
        };
        if variant.cache_vertices {
            self.cached = Some((origin, bytes.to_vec()));
        }
        UploadedVertices {
            buffer,
            anchor: origin,
            count,
        }
    }

    /// The kept buffer as it is, when the vertices relative to the cached anchor are the bytes
    /// written last time and the anchor is still near the view.
    fn unchanged(
        &self,
        draw_list: &DrawList,
        view: ViewTransform,
        origin: Point,
    ) -> Option<UploadedVertices> {
        let (anchor, written) = self.cached.as_ref()?;
        let shift = (anchor.x - origin.x).hypot(anchor.y - origin.y) * view.scale();
        if shift > MAX_ANCHOR_SHIFT_PX {
            return None;
        }
        let vertices: Vec<GpuVertex> = gpu_vertices(draw_list, *anchor);
        if bytemuck::cast_slice::<GpuVertex, u8>(&vertices) != written.as_slice() {
            return None;
        }
        note(Upload::Skipped);
        Some(UploadedVertices {
            buffer: (!vertices.is_empty())
                .then(|| self.kept.as_ref().map(|(buffer, _)| buffer.clone()))
                .flatten(),
            anchor: *anchor,
            count: vertices.len(),
        })
    }

    /// Writes `bytes` into the kept buffer, growing it to the next power of two first.
    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> wgpu::Buffer {
        let needed = bytes.len() as u64;
        let buffer = match &self.kept {
            Some((buffer, size)) if *size >= needed => buffer.clone(),
            _ => {
                let size = needed.next_power_of_two().max(1024);
                note(Upload::Created);
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("curvyo kept vertex buffer"),
                    size,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.kept = Some((buffer.clone(), size));
                buffer
            }
        };
        note(Upload::Written);
        queue.write_buffer(&buffer, 0, bytes);
        buffer
    }
}
