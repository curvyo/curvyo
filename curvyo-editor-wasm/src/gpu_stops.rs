//! The gradient stop table on the GPU (ADR 0067 §2, `specs/0124-gradient-fill` criteria 9, 24 b
//! and e): its `Rgba32Float` texture under its bind group layout, built and uploaded only when
//! the draw list's stops differ from the uploaded ones, and cut to the device's largest 2D
//! texture. Compiles for `wasm32` only, like `gpu.rs`.

use curvyo_render_core::{DrawList, StopTexel};

/// The layout of bind group 1: the gradient stop table, an unfilterable float texture read with
/// `textureLoad` (core WebGL2, no float-linear extension).
pub(crate) fn create_stop_table_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("curvyo stop table layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    })
}

/// The uploaded stop table: its texture's bind group and the stops it holds, so a frame with
/// the same stops builds and uploads nothing (ADR 0067 §2).
pub(crate) struct StopTable {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uploaded: Vec<StopTexel>,
    /// The device's largest 2D texture side: the most rows the table may have.
    max_rows: u32,
}

impl StopTable {
    /// An empty table (one row of zeros) under `layout`.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: wgpu::BindGroupLayout,
    ) -> Self {
        let max_rows = device.limits().max_texture_dimension_2d;
        let empty = DrawList::default().stop_texture_within(max_rows);
        let bind_group = upload_stop_texture(device, queue, &layout, &empty);
        Self {
            layout,
            bind_group,
            uploaded: Vec::new(),
            max_rows,
        }
    }

    /// The table of `draw_list`, built and uploaded only when its stops differ from the ones on
    /// the GPU; a frame without gradients after one without costs one empty comparison.
    pub(crate) fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw_list: &DrawList,
    ) {
        let stops = draw_list.gradient_stops();
        if stops == self.uploaded.as_slice() {
            return;
        }
        let texture = draw_list.stop_texture_within(self.max_rows);
        self.bind_group = upload_stop_texture(device, queue, &self.layout, &texture);
        self.uploaded = stops.to_vec();
    }

    pub(crate) const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// A new `Rgba32Float` texture holding `table`, and its bind group.
fn upload_stop_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    table: &curvyo_render_core::StopTexture,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width: table.width,
        height: table.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("curvyo stop table"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&table.texels),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(table.width * 16),
            rows_per_image: Some(table.height),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("curvyo stop table bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    })
}
