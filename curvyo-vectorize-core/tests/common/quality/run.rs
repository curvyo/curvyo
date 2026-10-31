//! Traces one reference case with one preset, measuring it (`specs/0089-trace-quality-and-colour`
//! criterion 3: everything but peak memory or the wasm time, which need their own process).

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{Preset, Selection, TraceParams, auto_threshold, selected};
use kurbo::{BezPath, Point};

use super::super::Picture;
use super::reference::Edge;
use super::report::Row;
use super::shape::{
    corner_errors, deviation, measure_edge, samples_along, self_intersections, truth_segments,
};
use super::topology::{compare, without_specks};
use super::{measure_micro, rasterise_4x};

/// What is known about a case beyond its picture.
pub struct Truth {
    pub raster_4x: Vec<bool>,
    pub paths: Option<Vec<BezPath>>,
    pub corners: Vec<Point>,
    pub edges: Vec<Edge>,
}

/// One picture to trace with every preset.
pub struct Case {
    pub key: String,
    pub group: Option<super::reference::Group>,
    pub picture: Picture,
    pub mm_per_px: f64,
    pub truth: Option<Truth>,
    /// The selection method (Brightness for all but the slice C fixtures).
    pub selection: Selection,
}

/// The settings of `picture` with `preset`: Brightness at the automatic Threshold.
pub fn params_for(preset: &Preset, picture: &Picture) -> TraceParams {
    preset.apply(TraceParams::with_threshold(auto_threshold(
        picture.raster(),
    )))
}

/// The settings of `case` with `preset`: its method, the automatic Threshold.
pub fn case_params(preset: &Preset, case: &Case) -> TraceParams {
    TraceParams {
        selection: case.selection,
        ..params_for(preset, &case.picture)
    }
}

/// Traces `case` with `preset` and measures everything of criterion 3 but memory and wasm.
pub fn measure(case: &Case, preset: &Preset) -> Row {
    measure_with(case, &case_params(preset, case), preset.specks_mm)
}

/// Traces `case` with `p` and measures it as [`measure`] does, the reference mask without the
/// specks below `specks_mm`.
pub fn measure_with(case: &Case, p: &TraceParams, specks_mm: f64) -> Row {
    let p = *p;
    let mm = case.mm_per_px;
    let t0 = super::super::thread_cpu_seconds();
    let result = super::super::run(&case.picture, Affine::scale(mm, mm), &p);
    let cpu = super::super::thread_cpu_seconds() - t0;
    let outlines: Vec<BezPath> = result
        .outlines
        .iter()
        .map(|o| kurbo::Affine::scale(1.0 / mm) * o.clone())
        .collect();
    let (w, h) = (case.picture.width as usize, case.picture.height as usize);
    let mask = selected(case.picture.raster(), &p);
    let speck_px = specks_mm * specks_mm / (mm * mm);
    let reference = without_specks(&mask, w, h, speck_px);
    let micro = 0.95 * specks_mm * specks_mm;
    let base = measure_micro(&outlines, &reference, w, h, mm, micro);
    let mut row = Row::new();
    row.insert("paths", result.counts.paths as f64);
    row.insert("nodes", result.counts.nodes as f64);
    row.insert("node_density", base.node_density);
    row.insert("wobble", base.wobble);
    row.insert("max_deviation_px", base.max_deviation_px);
    row.insert("mean_deviation_px", base.mean_deviation_px);
    row.insert(
        "self_intersections",
        self_intersections(&outlines, w, h) as f64,
    );
    row.insert("cpu_native_ms", cpu * 1e3);
    if let Some(truth) = &case.truth {
        measure_truth(&mut row, &outlines, truth, w, h);
    }
    row
}

/// The truth-based metrics of criterion 3.
fn measure_truth(row: &mut Row, outlines: &[BezPath], truth: &Truth, w: usize, h: usize) {
    let traced_4x = rasterise_4x(outlines, w, h);
    let fg = truth.raster_4x.iter().filter(|t| **t).count();
    let xor = traced_4x
        .iter()
        .zip(&truth.raster_4x)
        .filter(|(a, b)| a != b)
        .count();
    row.insert("area_error_true_pct", xor as f64 / fg.max(1) as f64 * 100.0);
    let segments = truth_segments(truth.paths.as_deref(), &truth.raster_4x, w, h);
    let dev = deviation(outlines, &segments, &samples_along(&segments), w, h);
    row.insert("dev_trace_max_px", dev.trace_max);
    row.insert("dev_trace_mean_px", dev.trace_mean);
    row.insert("dev_truth_max_px", dev.truth_max);
    row.insert("dev_truth_mean_px", dev.truth_mean);
    // Axis-aligned features on half pixels are reported only: a binary mask puts them 0.5 px
    // off whatever the tracer does (`specs/0089`, frozen targets).
    let half_edges: Vec<&Edge> = truth.edges.iter().filter(|e| half_pixel_edge(e)).collect();
    let on_half = |c: &Point| {
        half_edges
            .iter()
            .any(|e| e.points.iter().any(|p| p.distance(*c) < 1e-6))
    };
    let (half_corners, corners): (Vec<Point>, Vec<Point>) =
        truth.corners.iter().partition(|c| on_half(c));
    let half_errors = corner_errors(outlines, &half_corners);
    if !half_errors.is_empty() {
        row.insert(
            "half_px_corner_err_max_px",
            half_errors.iter().copied().fold(0.0, f64::max),
        );
    }
    let errors = corner_errors(outlines, &corners);
    row.insert("corners", errors.len() as f64);
    if !errors.is_empty() {
        let max = errors.iter().copied().fold(0.0, f64::max);
        row.insert("corner_err_max_px", max);
        row.insert(
            "corner_err_mean_px",
            errors.iter().sum::<f64>() / errors.len() as f64,
        );
        let within = errors.iter().filter(|e| **e <= 0.5).count();
        row.insert(
            "corners_within_0_5_pct",
            within as f64 / errors.len() as f64 * 100.0,
        );
    }
    let half_dev = half_edges
        .iter()
        .map(|e| measure_edge(outlines, e))
        .filter(|r| !r.lost)
        .map(|r| r.deviation)
        .fold(f64::NAN, f64::max);
    if half_dev.is_finite() {
        row.insert("half_px_straight_dev_max_px", half_dev);
    }
    let edges: Vec<Edge> = truth
        .edges
        .iter()
        .filter(|e| !half_pixel_edge(e))
        .cloned()
        .collect();
    measure_edges(row, outlines, &edges);
    let topo = compare(&truth.raster_4x, &traced_4x, w * 4, h * 4);
    row.insert("truth_components", topo.truth_components as f64);
    row.insert("truth_holes", topo.truth_holes as f64);
    row.insert("components_lost", topo.components_lost as f64);
    row.insert("components_extra", topo.components_extra as f64);
    row.insert("connections_lost", topo.connections_lost as f64);
    row.insert("connections_extra", topo.connections_extra as f64);
    row.insert("holes_lost", topo.holes_lost as f64);
    row.insert("holes_extra", topo.holes_extra as f64);
}

/// Whether `v` lies within 0.1 px of a half pixel.
fn on_half_pixel(v: f64) -> bool {
    ((v - v.floor()) - 0.5).abs() < 0.1
}

/// An axis-aligned straight edge on (or within 0.1 px of) a half pixel.
pub fn half_pixel_edge(edge: &Edge) -> bool {
    let [a, b] = edge.points[..] else {
        return false;
    };
    edge.straight
        && (((a.x - b.x).abs() < 1e-6 && on_half_pixel(a.x))
            || ((a.y - b.y).abs() < 1e-6 && on_half_pixel(a.y)))
}

/// Straightness of the annotated straight edges longer than 20 px and corner-freeness of the
/// annotated smooth curves (criteria 3, 6, 7).
fn measure_edges(row: &mut Row, outlines: &[BezPath], edges: &[Edge]) {
    let long = |e: &Edge| {
        e.points
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum::<f64>()
            > 20.0
    };
    let (mut straight, mut smooth) = (Vec::new(), Vec::new());
    for edge in edges.iter().filter(|e| long(e)) {
        let r = measure_edge(outlines, edge);
        if edge.straight {
            straight.push(r);
        } else {
            smooth.push(r);
        }
    }
    if !straight.is_empty() {
        let kept: Vec<_> = straight.iter().filter(|r| !r.lost).collect();
        row.insert("straight_edges", straight.len() as f64);
        row.insert("straight_lost", (straight.len() - kept.len()) as f64);
        row.insert(
            "straight_dev_max_px",
            kept.iter().map(|r| r.deviation).fold(0.0, f64::max),
        );
        row.insert(
            "straight_segments_max",
            kept.iter().map(|r| r.segments).max().unwrap_or(0) as f64,
        );
        row.insert(
            "straight_segments_mean",
            kept.iter().map(|r| r.segments as f64).sum::<f64>() / kept.len().max(1) as f64,
        );
        row.insert(
            "straight_flips",
            kept.iter().map(|r| r.flips).sum::<usize>() as f64,
        );
    }
    if !smooth.is_empty() {
        let kept: Vec<_> = smooth.iter().filter(|r| !r.lost).collect();
        row.insert("smooth_edges", smooth.len() as f64);
        row.insert("smooth_lost", (smooth.len() - kept.len()) as f64);
        row.insert(
            "smooth_dev_max_px",
            kept.iter().map(|r| r.deviation).fold(0.0, f64::max),
        );
        row.insert(
            "false_corners",
            kept.iter().map(|r| r.corners).sum::<usize>() as f64,
        );
    }
}
