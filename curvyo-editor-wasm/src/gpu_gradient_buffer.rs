//! Vertex buffer 1, the per-vertex gradient attributes (ADR 0067 §1, `specs/0124-gradient-fill`
//! criterion 24 b): built from the draw list only when it shades a gradient, else one buffer of
//! zeros kept across frames. Compiles for `wasm32` only, like `gpu.rs`.

use curvyo_render_core::DrawList;

/// Bytes per vertex of the attributes: `(u, v, first stop, signed count)` in `f32`.
const BYTES_PER_VERTEX: u64 = 16;

/// The zero attributes kept for frames without gradients, grown to the largest list seen.
pub(crate) struct GradientAttributes {
    zeros: wgpu::Buffer,
    zero_vertices: u64,
}

/// A vertex buffer of `vertices` zero attributes (a new `wgpu` buffer is zero-filled).
fn zero_buffer(device: &wgpu::Device, vertices: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("curvyo gradient zeros"),
        size: vertices * BYTES_PER_VERTEX,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    })
}

impl GradientAttributes {
    /// Zeros for 1,024 vertices to start with.
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            zeros: zero_buffer(device, 1_024),
            zero_vertices: 1_024,
        }
    }

    /// This frame's attributes for `vertices` vertices of `draw_list`: a new buffer when the list
    /// shades a gradient, else `None` and [`GradientAttributes::zeros`] covers them (grown here
    /// when the list is longer than any before).
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        draw_list: &DrawList,
        vertices: usize,
    ) -> Option<wgpu::Buffer> {
        if draw_list.has_gradients() {
            return Some(wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("curvyo gradient buffer"),
                    contents: bytemuck::cast_slice(&draw_list.gradient_attributes()),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
        let needed = u64::try_from(vertices).unwrap_or(u64::MAX);
        if needed > self.zero_vertices {
            self.zero_vertices = needed.next_power_of_two();
            self.zeros = zero_buffer(device, self.zero_vertices);
        }
        None
    }

    /// The kept zeros, at least as long as the last [`GradientAttributes::prepare`] asked for.
    pub(crate) const fn zeros(&self) -> &wgpu::Buffer {
        &self.zeros
    }
}
