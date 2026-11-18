//! The CPU twin of the artwork fragment shader's gradient lookup (`fs_main` in
//! `curvyo-editor-wasm/src/gpu_shader.rs`), step by step in `f32`: the GPU path only runs in a
//! browser, so the host tests check the maths through this copy. A change to one changes the
//! other.

/// The texel at flat index `i` of `texels`; zero past the end, as WGSL's robust `textureLoad`.
fn texel(texels: &[[f32; 4]], i: usize) -> [f32; 4] {
    texels.get(i).copied().unwrap_or([0.0; 4])
}

/// The straight (not premultiplied) colour the shader writes for one fragment with gradient
/// attribute `g` (interpolated `u, v`; flat first stop and signed count) and vertex alpha
/// `alpha`, over the stop texture `texels`.
pub(crate) fn shade(g: [f32; 4], alpha: f32, texels: &[[f32; 4]]) -> [f32; 4] {
    #[allow(clippy::cast_possible_truncation)]
    let count = g[3].round() as i32;
    if count == 0 {
        return [1.0, 1.0, 1.0, alpha];
    }
    let t = if count < 0 { g[0].hypot(g[1]) } else { g[0] }.clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (first, n) = (g[2].round() as usize, count.unsigned_abs() as usize);
    let offset = |k: usize| texel(texels, 2 * (first + k))[0];
    let colour = |k: usize| texel(texels, 2 * (first + k) + 1);
    // The first stop above `t`: the later stop wins on a hard edge.
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        if offset(mid) > t {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    let c = if lo >= n {
        colour(n - 1)
    } else {
        let below = lo.max(1) - 1;
        let (o0, o1) = (offset(below), offset(lo));
        let f = ((t - o0) / (o1 - o0)).clamp(0.0, 1.0);
        let (c0, c1) = (colour(below), colour(lo));
        std::array::from_fn(|i| c0[i] + (c1[i] - c0[i]) * f)
    };
    if c[3] <= 0.0 {
        return [0.0; 4];
    }
    [c[0] / c[3], c[1] / c[3], c[2] / c[3], c[3] * alpha]
}
