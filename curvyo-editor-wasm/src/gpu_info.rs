//! Describes the `wgpu` adapter and surface configuration of an attached canvas as text, for the
//! performance probe (`tools/perf/`). `wasm32`-only, like `gpu.rs`.

use crate::gpu_variant::GpuVariant;

/// The adapter, the surface's capabilities and the configuration applied to
/// it, one `key: value` per line, for the performance probe (`tools/perf/`).
pub(crate) fn describe(
    adapter: &wgpu::Adapter,
    capabilities: &wgpu::SurfaceCapabilities,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
    variant: GpuVariant,
) -> String {
    let info = adapter.get_info();
    format!(
        "adapter: {}\nbackend: {:?}\ndevice type: {:?}\nvendor/device: {:#06x}/{:#06x}\n\
         driver: {} {}\nsurface format: {:?}\npresent mode: {:?}\nalpha mode: {:?}\n\
         frame latency: {}\nsample count: {sample_count}\nformats offered: {:?}\n\
         present modes offered: {:?}\nalpha modes offered: {:?}\nvariant: {}",
        info.name,
        info.backend,
        info.device_type,
        info.vendor,
        info.device,
        info.driver,
        info.driver_info,
        config.format,
        config.present_mode,
        config.alpha_mode,
        config.desired_maximum_frame_latency,
        capabilities.formats,
        capabilities.present_modes,
        capabilities.alpha_modes,
        variant.describe(),
    )
}
