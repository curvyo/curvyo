//! The WGSL source of the draw-list pipelines (artwork, checkerboard, overlay). Compiled on every
//! target so the host tests can parse and validate it for WebGL2 (GLSL ES 3.00) without a GPU;
//! only `gpu_pipeline.rs` (wasm32) builds pipelines from it.
//!
//! The gradient lookup of `fs_main` (ADR 0067 §2) has a CPU twin in `curvyo-render-core`
//! (`gradient_fill/shader_reference.rs`); a change to one changes the other.

/// The draw-list shader: group 0 the screen transform, group 1 the gradient stop table.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) const SHADER_SOURCE: &str = r"
struct ScreenTransform {
    scale_x: f32,
    offset_x: f32,
    scale_y: f32,
    offset_y: f32,
    // The checkerboard of the document area (`specs/0040-document-background`): the
    // document corner in device pixels, reduced to the pattern's period, the cell
    // side in device pixels and the two tones.
    checker_phase: vec2<f32>,
    checker_cell: f32,
    checker_pad: f32,
    checker_a: vec4<f32>,
    checker_b: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: ScreenTransform;

// The gradient stop table (`curvyo_render_core::DrawList::stop_texture`): stop i at texels
// 2i (offset in red) and 2i + 1 (premultiplied colour), rows of STOP_ROW texels.
@group(1) @binding(0)
var stop_table: texture_2d<f32>;

const STOP_ROW: i32 = 2048;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) depth: f32,
    // Vertex buffer 1: (u, v, first stop, signed count); count 0 is a solid vertex.
    @location(3) gradient: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) stops: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(
        input.position.x * transform.scale_x + transform.offset_x,
        input.position.y * transform.scale_y + transform.offset_y,
        input.depth,
        1.0,
    );
    out.color = input.color;
    out.uv = input.gradient.xy;
    out.stops = input.gradient.zw;
    return out;
}

fn stop_texel(i: i32) -> vec4<f32> {
    return textureLoad(stop_table, vec2<i32>(i % STOP_ROW, i / STOP_ROW), 0);
}

// The colour of a gradient at its coordinate: pad, the first stop above t (a hard edge takes
// the later stop), a premultiplied blend, then straight alpha for ALPHA_BLENDING.
fn shade(uv: vec2<f32>, stops: vec2<f32>, alpha: f32) -> vec4<f32> {
    let count = i32(round(stops.y));
    var t = uv.x;
    if (count < 0) {
        t = length(uv);
    }
    t = clamp(t, 0.0, 1.0);
    let first = i32(round(stops.x));
    let n = abs(count);
    var lo = 0;
    var hi = n;
    loop {
        if (lo >= hi) {
            break;
        }
        let mid = (lo + hi) / 2;
        if (stop_texel(2 * (first + mid)).x > t) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    var c: vec4<f32>;
    if (lo >= n) {
        c = stop_texel(2 * (first + n - 1) + 1);
    } else {
        let below = max(lo, 1) - 1;
        let o0 = stop_texel(2 * (first + below)).x;
        let o1 = stop_texel(2 * (first + lo)).x;
        let f = clamp((t - o0) / (o1 - o0), 0.0, 1.0);
        let c0 = stop_texel(2 * (first + below) + 1);
        let c1 = stop_texel(2 * (first + lo) + 1);
        c = c0 + (c1 - c0) * f;
    }
    if (c.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(c.rgb / c.a, c.a * alpha);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (round(input.stops.y) == 0.0) {
        return input.color;
    }
    return shade(input.uv, input.stops, input.color.a);
}

// The tone is the parity of floor((p - corner) / cell) over both axes, even is
// tone A: `curvyo_render_core::checker_tone` is the same formula. `position` is the
// pixel centre in device pixels, so the cell edges fall on whole pixels.
@fragment
fn fs_checker(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let cell = floor((frag.xy - transform.checker_phase) / transform.checker_cell);
    if ((i32(cell.x) + i32(cell.y)) & 1) == 0 {
        return transform.checker_a;
    }
    return transform.checker_b;
}
";

#[cfg(test)]
mod tests {
    use super::SHADER_SOURCE;

    /// The shader parses, validates and translates to GLSL ES 3.00 (WebGL2) for every entry point
    /// the pipelines use: a WGSL error would otherwise show only in the browser.
    #[test]
    fn the_shader_compiles_for_webgl2() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE).expect("parses");
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("validates");
        for (stage, entry) in [
            (naga::ShaderStage::Vertex, "vs_main"),
            (naga::ShaderStage::Fragment, "fs_main"),
            (naga::ShaderStage::Fragment, "fs_checker"),
        ] {
            let options = naga::back::glsl::Options {
                version: naga::back::glsl::Version::Embedded {
                    version: 300,
                    is_webgl: true,
                },
                ..naga::back::glsl::Options::default()
            };
            let pipeline = naga::back::glsl::PipelineOptions {
                shader_stage: stage,
                entry_point: entry.to_owned(),
                multiview: None,
            };
            let mut out = String::new();
            let mut writer = naga::back::glsl::Writer::new(
                &mut out,
                &module,
                &info,
                &options,
                &pipeline,
                naga::proc::BoundsCheckPolicies::default(),
            )
            .expect("a GLSL writer");
            writer.write().unwrap_or_else(|e| panic!("{entry}: {e}"));
            assert!(out.contains("void main"), "{entry}");
        }
    }
}
