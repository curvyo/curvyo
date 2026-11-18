//! The time of each phase of the drawn frames, for the performance probe (`tools/perf/`,
//! `specs/0126-performance-pass` phase 4): off until the probe turns it on, so a normal
//! session reads no clock.

use curvyo_render_core::DrawList;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// The page's `performance.now()`, in milliseconds.
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

/// The phases of one drawn frame, in the order they run.
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    /// `Session::frame_draw_list`: the draw list built in Rust, tessellation included.
    DrawList = 0,
    /// `Session::frame_images`: the pictures of the frame.
    Images = 1,
    /// `Gpu::prepare`: GPU vertices built and uploaded.
    Upload = 2,
    /// `Gpu::draw`: encode, submit and present.
    Submit = 3,
}

/// Sums of the phase times of the frames drawn since the last [`FramePhases::take`].
#[derive(Default)]
pub(crate) struct FramePhases {
    on: bool,
    frames: u32,
    ms: [f64; 4],
    layers: usize,
    vertices: usize,
    last: f64,
}

impl FramePhases {
    /// Turns the timing on or off; the sums start again.
    pub(crate) fn set_on(&mut self, on: bool) {
        *self = Self {
            on,
            ..Self::default()
        };
    }

    /// Opens a frame: the clock starts.
    pub(crate) fn start(&mut self) {
        if self.on {
            self.frames += 1;
            self.last = performance_now();
        }
    }

    /// Closes `phase` at the present moment.
    pub(crate) fn mark(&mut self, phase: Phase) {
        if self.on {
            let now = performance_now();
            self.ms[phase as usize] += now - self.last;
            self.last = now;
        }
    }

    /// Counts the layers and vertices of the frame's `list`.
    pub(crate) fn count(&mut self, list: &DrawList) {
        if self.on {
            self.layers += list.layers().len();
            self.vertices += list.triangles.len();
        }
    }

    /// Frames, the milliseconds of the four phases, artwork layers and vertices, summed since
    /// the last call; the sums start again.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn take(&mut self) -> Vec<f64> {
        let taken = vec![
            f64::from(self.frames),
            self.ms[0],
            self.ms[1],
            self.ms[2],
            self.ms[3],
            self.layers as f64,
            self.vertices as f64,
        ];
        self.set_on(self.on);
        taken
    }
}
