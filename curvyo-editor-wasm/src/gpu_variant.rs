//! The perf build's switches on the `wgpu` frame path (`tools/perf/measure.py --gpu-variants`),
//! read at every attach and frame: the constant baseline outside the `perf` feature.

// On the host only the tests use this module (`gpu.rs`, its reader, is `wasm32`-only).
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

/// One variant of the GPU path, from a name like `samples-1+no-fences`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GpuVariant {
    /// The multisample count asked for instead of `gpu.rs`'s preference list, when set.
    pub(crate) samples: Option<u32>,
    /// wgpu's GL fences short-circuited (`GlFenceBehavior::AutoFinish`): no `fenceSync` or
    /// `getSyncParameter` per submit; wgpu-hal's `flush` after every submit stays.
    pub(crate) no_fences: bool,
    /// One vertex buffer kept across frames and written into, not one created per frame.
    pub(crate) reuse_vertex_buffer: bool,
    /// The vertices kept relative to a fixed anchor (the view's shift goes into the transform
    /// uniform) and uploaded only when they changed; implies `reuse_vertex_buffer`.
    pub(crate) cache_vertices: bool,
}

impl GpuVariant {
    /// The production path.
    pub(crate) const BASELINE: Self = Self {
        samples: None,
        no_fences: false,
        reuse_vertex_buffer: false,
        cache_vertices: false,
    };

    /// The variant `text` names: parts joined by `+` (`baseline`, `samples-N`, `no-fences`,
    /// `reuse-vertex-buffer`, `cache-vertices`); empty is the baseline.
    #[cfg(any(test, feature = "perf"))]
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut variant = Self::BASELINE;
        for part in text
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part {
                "baseline" => {}
                "no-fences" => variant.no_fences = true,
                "reuse-vertex-buffer" => variant.reuse_vertex_buffer = true,
                "cache-vertices" => {
                    variant.cache_vertices = true;
                    variant.reuse_vertex_buffer = true;
                }
                _ => {
                    let samples = part
                        .strip_prefix("samples-")
                        .and_then(|count| count.parse::<u32>().ok())
                        .filter(|count| [1, 2, 4, 8, 16].contains(count))
                        .ok_or_else(|| format!("unknown gpu variant part {part:?}"))?;
                    variant.samples = Some(samples);
                }
            }
        }
        Ok(variant)
    }

    /// The variant as one line for `Gpu::info` (`key=value` pairs).
    pub(crate) fn describe(self) -> String {
        format!(
            "samples={} fences={} vertex buffer={}",
            self.samples
                .map_or_else(|| "preferred".to_owned(), |n| n.to_string()),
            if self.no_fences {
                "auto-finish"
            } else {
                "normal"
            },
            if self.cache_vertices {
                "kept, upload only when changed"
            } else if self.reuse_vertex_buffer {
                "kept"
            } else {
                "new per frame"
            },
        )
    }
}

#[cfg(feature = "perf")]
mod store {
    use std::cell::Cell;

    use super::GpuVariant;

    thread_local! {
        static CURRENT: Cell<GpuVariant> = const { Cell::new(GpuVariant::BASELINE) };
        /// Uploads written and skipped (`cache_vertices`), vertex buffers created.
        static COUNTS: Cell<[u64; 3]> = const { Cell::new([0; 3]) };
    }

    pub(crate) fn current() -> GpuVariant {
        CURRENT.with(Cell::get)
    }

    pub(crate) fn set(variant: GpuVariant) {
        CURRENT.with(|current| current.set(variant));
    }

    pub(crate) fn count(index: usize) {
        COUNTS.with(|counts| {
            let mut value = counts.get();
            value[index] += 1;
            counts.set(value);
        });
    }

    pub(crate) fn take_counts() -> [u64; 3] {
        COUNTS.with(|counts| counts.replace([0; 3]))
    }
}

/// The variant the next attach and every frame use.
#[cfg(feature = "perf")]
pub(crate) fn current() -> GpuVariant {
    store::current()
}

/// The variant the next attach and every frame use: the baseline outside the perf build.
#[cfg(not(feature = "perf"))]
pub(crate) const fn current() -> GpuVariant {
    GpuVariant::BASELINE
}

/// Which upload event [`note`] counts.
#[derive(Clone, Copy)]
pub(crate) enum Upload {
    /// The vertices were written to the GPU.
    Written = 0,
    /// The vertices were unchanged and not written (`cache_vertices`).
    Skipped = 1,
    /// A vertex buffer was created.
    Created = 2,
}

/// Counts `event` for the perf build's stats; nothing outside it.
pub(crate) fn note(event: Upload) {
    #[cfg(feature = "perf")]
    store::count(event as usize);
    #[cfg(not(feature = "perf"))]
    let _ = event;
}

/// Sets the GPU variant the next `attach_canvas` and every frame use (`tools/perf/measure.py
/// --gpu-variants`, perf build only) and returns it as `Gpu::info` will print it.
///
/// # Errors
/// A `JsValue` string naming an unknown part.
#[cfg(all(target_arch = "wasm32", feature = "perf"))]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn perf_set_gpu_variant(text: &str) -> Result<String, wasm_bindgen::JsValue> {
    let variant = GpuVariant::parse(text).map_err(|err| wasm_bindgen::JsValue::from_str(&err))?;
    store::set(variant);
    Ok(variant.describe())
}

/// The vertex uploads since the last call (perf build only): written, skipped, buffers created.
#[cfg(all(target_arch = "wasm32", feature = "perf"))]
#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub fn perf_take_gpu_counts() -> Vec<f64> {
    #[allow(clippy::cast_precision_loss)]
    store::take_counts().iter().map(|&n| n as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::GpuVariant;

    #[test]
    fn an_empty_name_and_baseline_are_the_production_path() {
        assert_eq!(GpuVariant::parse(""), Ok(GpuVariant::BASELINE));
        assert_eq!(GpuVariant::parse("baseline"), Ok(GpuVariant::BASELINE));
        assert_eq!(super::current(), GpuVariant::BASELINE);
    }

    #[test]
    fn parts_combine_and_cache_vertices_keeps_the_buffer() {
        let variant = GpuVariant::parse("samples-1+no-fences+cache-vertices").unwrap();
        assert_eq!(variant.samples, Some(1));
        assert!(variant.no_fences && variant.cache_vertices && variant.reuse_vertex_buffer);
        assert_eq!(
            variant.describe(),
            "samples=1 fences=auto-finish vertex buffer=kept, upload only when changed"
        );
    }

    #[test]
    fn unknown_parts_and_odd_sample_counts_are_refused() {
        assert!(GpuVariant::parse("samples-3").is_err());
        assert!(GpuVariant::parse("ctx-no-depth").is_err());
        assert!(GpuVariant::parse("baseline+bogus").is_err());
    }
}
