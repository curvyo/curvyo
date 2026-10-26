//! The textured pipeline of the image objects (ADR 0017 §5, `specs/0058-bitmap-import` criteria
//! 27 to 30): one texture per asset with its premultiplied mip chain, a quad per image drawn at
//! the depth of its own artwork layer, sampled trilinear when shrunk, bilinear when enlarged and
//! nearest from 8 screen pixels per image pixel. Compiles for `wasm32` only, like `gpu.rs`.

use std::collections::HashMap;

use curvyo_document_core::{AssetHash, Point, ViewTransform};
use curvyo_render_core::{DrawList, ImageQuad};

use crate::gpu_pipeline::DEPTH_FORMAT;
use crate::session::FrameImages;

/// From this many screen pixels per image pixel the picture shows its pixel grid
/// (`specs/0058-bitmap-import` criterion 28).
const NEAREST_FROM_PX: f64 = 8.0;

const SHADER_SOURCE: &str = r"
struct ScreenTransform {
    scale_x: f32,
    offset_x: f32,
    scale_y: f32,
    offset_y: f32,
    checker_phase: vec2<f32>,
    checker_cell: f32,
    checker_pad: f32,
    checker_a: vec4<f32>,
    checker_b: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: ScreenTransform;
@group(1) @binding(0)
var picture: texture_2d<f32>;
@group(1) @binding(1)
var picture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) depth: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@vertex
fn vs_image(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(
        input.position.x * transform.scale_x + transform.offset_x,
        input.position.y * transform.scale_y + transform.offset_y,
        input.depth,
        1.0,
    );
    out.uv = input.uv;
    out.opacity = input.opacity;
    return out;
}

// The texture holds premultiplied alpha, so the opacity scales all four channels.
@fragment
fn fs_image(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(picture, picture_sampler, input.uv) * input.opacity;
}
";

/// One vertex of an image quad.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    depth: f32,
}

/// One uploaded picture.
struct Texture {
    width: u32,
    height: u32,
    smooth: wgpu::BindGroup,
    sharp: wgpu::BindGroup,
}

/// The image pipeline, its samplers and the textures of the pictures on screen.
pub(crate) struct GpuImages {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    smooth: wgpu::Sampler,
    sharp: wgpu::Sampler,
    textures: HashMap<AssetHash, Texture>,
}

/// The quads of one frame, ready to draw.
pub(crate) struct ImageDraws {
    buffer: Option<wgpu::Buffer>,
    /// Per quad in layer order: the vertex index its layer starts at in the artwork, and the
    /// asset and sampler to draw it with.
    quads: Vec<(u32, AssetHash, bool)>,
}

impl ImageDraws {
    /// The artwork vertex indices at which an image is drawn, in order.
    pub(crate) fn splits(&self) -> impl Iterator<Item = u32> + '_ {
        self.quads.iter().map(|(split, ..)| *split)
    }
}

fn sampler(device: &wgpu::Device, magnify: wgpu::FilterMode) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("curvyo image sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: magnify,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..wgpu::SamplerDescriptor::default()
    })
}

impl GpuImages {
    /// The pipeline for `surface_format` at `sample_count`, sharing the frame's transform layout.
    pub(crate) fn new(
        device: &wgpu::Device,
        transform_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("curvyo image layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline = create_image_pipeline(
            device,
            &[transform_layout, &layout],
            surface_format,
            sample_count,
        );
        Self {
            pipeline,
            layout,
            smooth: sampler(device, wgpu::FilterMode::Linear),
            sharp: sampler(device, wgpu::FilterMode::Nearest),
            textures: HashMap::new(),
        }
    }

    /// Uploads the pictures of `images` that have no texture yet and drops the textures of the
    /// pictures no longer drawn.
    pub(crate) fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        images: &FrameImages,
    ) {
        self.textures
            .retain(|hash, _| images.iter().any(|(drawn, _)| drawn == hash));
        for (hash, working) in images {
            if self.textures.contains_key(hash) {
                continue;
            }
            let texture = self.upload(device, queue, working);
            self.textures.insert(hash.clone(), texture);
        }
    }

    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        working: &curvyo_image_core::WorkingImage,
    ) -> Texture {
        // The device may hold less than the working limit (WebGL2 guarantees 2048): the top
        // levels are skipped (ADR 0017 §5).
        let limit = device.limits().max_texture_dimension_2d.max(1);
        let chain: Vec<_> = working
            .premultiplied_mips()
            .into_iter()
            .skip_while(|level| level.width.max(level.height) > limit)
            .collect();
        let (width, height) = chain
            .first()
            .map_or((1, 1), |level| (level.width, level.height));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("curvyo image"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: u32::try_from(chain.len().max(1)).unwrap_or(1),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, mip) in chain.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: u32::try_from(level).unwrap_or(0),
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * mip.width),
                    rows_per_image: Some(mip.height),
                },
                wgpu::Extent3d {
                    width: mip.width,
                    height: mip.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = |sampler: &wgpu::Sampler| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("curvyo image bind group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        Texture {
            width: working.width,
            height: working.height,
            smooth: bind(&self.smooth),
            sharp: bind(&self.sharp),
        }
    }

    /// The quads of `list` that have a texture, as one vertex buffer, relative to `origin` (the
    /// document point at screen pixel (0, 0)), in layer order.
    pub(crate) fn prepare(
        &self,
        device: &wgpu::Device,
        list: &DrawList,
        view: ViewTransform,
        origin: Point,
    ) -> ImageDraws {
        let mut quads: Vec<&ImageQuad> = list
            .images
            .iter()
            .filter(|quad| self.textures.contains_key(&quad.asset))
            .collect();
        quads.sort_by_key(|quad| quad.layer);
        let mut vertices = Vec::with_capacity(quads.len() * 6);
        let mut draws = Vec::with_capacity(quads.len());
        for quad in quads {
            let Some(texture) = self.textures.get(&quad.asset) else {
                continue;
            };
            let depth = list.layer_depth(quad.layer);
            #[allow(clippy::cast_possible_truncation)]
            let opacity = quad.opacity.get() as f32;
            let corner = |i: usize, uv: [f32; 2]| {
                let p = quad.corners[i];
                #[allow(clippy::cast_possible_truncation)]
                let position = [(p.x - origin.x) as f32, (p.y - origin.y) as f32];
                ImageVertex {
                    position,
                    uv,
                    opacity,
                    depth,
                }
            };
            let (tl, tr, br, bl) = (
                corner(0, [0.0, 0.0]),
                corner(1, [1.0, 0.0]),
                corner(2, [1.0, 1.0]),
                corner(3, [0.0, 1.0]),
            );
            vertices.extend([tl, tr, br, tl, br, bl]);
            let split = u32::try_from(list.layer_start(quad.layer)).unwrap_or(u32::MAX);
            draws.push((split, quad.asset.clone(), sharp(quad, texture, view)));
        }
        let buffer = (!vertices.is_empty()).then(|| {
            wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("curvyo image vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            )
        });
        ImageDraws {
            buffer,
            quads: draws,
        }
    }

    /// Draws quad `index` of `draws` (the transform bind group is set by the caller).
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, draws: &ImageDraws, index: usize) {
        let (Some(buffer), Some((_, asset, sharp))) = (&draws.buffer, draws.quads.get(index))
        else {
            return;
        };
        let Some(texture) = self.textures.get(asset) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(
            1,
            if *sharp {
                &texture.sharp
            } else {
                &texture.smooth
            },
            &[],
        );
        pass.set_vertex_buffer(0, buffer.slice(..));
        let first = u32::try_from(index * 6).unwrap_or(u32::MAX);
        pass.draw(first..first + 6, 0..1);
    }
}

/// Whether every image pixel of `quad` covers at least [`NEAREST_FROM_PX`] screen pixels.
fn sharp(quad: &ImageQuad, texture: &Texture, view: ViewTransform) -> bool {
    let c = quad.corners;
    let across = c[0].vector_to(c[1]).length() * view.scale() / f64::from(texture.width.max(1));
    let down = c[0].vector_to(c[3]).length() * view.scale() / f64::from(texture.height.max(1));
    across.min(down) >= NEAREST_FROM_PX
}

fn create_image_pipeline(
    device: &wgpu::Device,
    layouts: &[&wgpu::BindGroupLayout],
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("curvyo image shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });
    let layouts: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().copied().map(Some).collect();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("curvyo image pipeline layout"),
        bind_group_layouts: &layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("curvyo image pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_image"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32
                ],
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_image"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        // The artwork depth rule: drawn at its layer's depth, in list order.
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
