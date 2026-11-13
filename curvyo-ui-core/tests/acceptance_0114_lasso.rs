//! `specs/0114-image-trace-mask` criteria 10, 11 and 23 at the mask: the lasso L1 (corners
//! (0, 0), (60, 0), (0, 100) mm on M1, `curvyo-ui-core/tests/support_0094`) stored in the image's
//! unit square, made into the worker's mask by `part_region` and traced as the worker traces it;
//! a lasso that crosses itself paints by non-zero winding, deterministically.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

#[path = "support_0094/mod.rs"]
mod support;

use curvyo_document_core::{
    Affine, Point, Tolerance, TraceMaskPart, TraceMaskRegion, TracePartSettings, TraceRegionKind,
    UnitPoint, Vec2,
};
use curvyo_geometry_core::{
    BooleanError, BooleanOp, Outline, OutlineTriple, boolean, signed_area_mm2,
};
use curvyo_ui_core::{TraceRegion, part_region, read_trace_wire};
use kurbo::{BezPath, PathEl};
use support::{M1, draw, map_100mm};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster, TraceParams};

const N: u32 = 1000;
/// The image M1 over 100 x 100 mm: the unit square to document mm.
const PLACEMENT: Affine = Affine {
    a: 100.0,
    b: 0.0,
    c: 0.0,
    d: 100.0,
    e: 0.0,
    f: 0.0,
};
/// L1's corners in mm.
const L1: [(f64, f64); 3] = [(0.0, 0.0), (60.0, 0.0), (0.0, 100.0)];

fn lasso(corners: &[(f64, f64)], outside: bool) -> TraceMaskPart {
    TraceMaskPart {
        id: "0".into(),
        order: 0,
        outside,
        settings: TracePartSettings::new(),
        regions: vec![TraceMaskRegion {
            id: "1@1".into(),
            order: 0,
            kind: TraceRegionKind::Lasso,
            subtract: false,
            points: corners
                .iter()
                .map(|&(x, y)| UnitPoint::new(x / 100.0, y / 100.0))
                .collect(),
        }],
    }
}

fn trace(mask: Option<&TraceRegion>) -> Vec<f64> {
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &draw(M1, N),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&TraceParams::with_threshold(50)),
            lines: &[],
            mask: mask.map(|r| r as &dyn vectrace::mask::Mask),
            shapes: false,
        },
        &mut |_| {},
    )
}

fn outlines(wire: &[f64]) -> Vec<BezPath> {
    read_trace_wire(wire).expect("a readable wire").outlines
}

/// Every anchor of `paths`.
fn nodes(paths: &[BezPath]) -> Vec<Point> {
    paths
        .iter()
        .flat_map(|p| p.elements().iter().copied())
        .filter_map(|el| match el {
            PathEl::MoveTo(q)
            | PathEl::LineTo(q)
            | PathEl::QuadTo(_, q)
            | PathEl::CurveTo(_, _, q) => Some(Point::new(q.x, q.y)),
            PathEl::ClosePath => None,
        })
        .collect()
}

/// `paths` flattened to polygons the kernel reads.
fn polygons(paths: &[BezPath]) -> Vec<Vec<OutlineTriple>> {
    let mut out: Vec<Vec<OutlineTriple>> = Vec::new();
    for p in paths {
        let mut current: Vec<OutlineTriple> = Vec::new();
        kurbo::flatten(p.elements().iter().copied(), 0.0005, |el| match el {
            PathEl::MoveTo(q) => {
                if current.len() >= 3 {
                    out.push(std::mem::take(&mut current));
                }
                current.clear();
                current.push((Point::new(q.x, q.y), Vec2::ZERO, Vec2::ZERO));
            }
            PathEl::LineTo(q) => current.push((Point::new(q.x, q.y), Vec2::ZERO, Vec2::ZERO)),
            _ => {}
        });
        if current.len() >= 3 {
            out.push(current);
        }
    }
    out
}

/// The area of `op` over the operands (nonzero), mm²; 0 for an empty result.
fn op_area(op: BooleanOp, operands: &[&[Vec<OutlineTriple>]]) -> f64 {
    let outl: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|operand| operand.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let refs: Vec<&[Outline<'_>]> = outl.iter().map(Vec::as_slice).collect();
    match boolean(op, &refs, Tolerance::from_mm(0.003)) {
        Ok(r) => r
            .outlines()
            .iter()
            .map(|p| signed_area_mm2(p))
            .sum::<f64>()
            .abs(),
        Err(BooleanError::EmptyResult) => 0.0,
        Err(e) => panic!("kernel refused {op:?}: {e:?}"),
    }
}

/// The signed distance of `p` from the line through `a` and `b`, positive on the side away from
/// the origin.
fn side_distance(a: Point, b: Point, p: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let cross = |q: Point| (q.x - a.x) * dy - (q.y - a.y) * dx;
    let sign = if cross(Point::new(0.0, 0.0)) > 0.0 {
        -1.0
    } else {
        1.0
    };
    sign * cross(p) / dx.hypot(dy)
}

/// Criterion 10: with L1 Inside every node lies inside L1 (at most 0.01 mm out), the disc is cut
/// on the slanted side with its new vertices within 0.005 mm of it; Inside and Outside together
/// are the unmasked trace (no gap, no overlap, under 0.01 mm² each).
#[test]
fn c10_l1_cuts_the_disc_on_its_slanted_side_without_gap_or_overlap() {
    let inside = part_region(&lasso(&L1, false), PLACEMENT).unwrap();
    let outside = part_region(&lasso(&L1, true), PLACEMENT).unwrap();
    let (a, b) = (Point::new(60.0, 0.0), Point::new(0.0, 100.0));
    let kept = outlines(&trace(Some(&inside)));
    assert!(!kept.is_empty(), "the disc is traced");
    let mut on_side = 0;
    for p in nodes(&kept) {
        let d = side_distance(a, b, p);
        assert!(d <= 0.01, "node {p:?} lies {d} mm beyond the slanted side");
        assert!(
            p.x >= -0.01 && p.y >= -0.01,
            "node {p:?} beyond L1's straight sides"
        );
        if d.abs() < 0.02 {
            assert!(
                d.abs() <= 0.005,
                "a cut vertex {p:?} lies {d} mm off the side"
            );
            on_side += 1;
        }
    }
    assert!(
        on_side >= 2,
        "the disc is cut by the slanted side ({on_side} vertices on it)"
    );
    let (pin, pout) = (polygons(&kept), polygons(&outlines(&trace(Some(&outside)))));
    let whole = polygons(&outlines(&trace(None)));
    let overlap = op_area(BooleanOp::Intersection, &[&pin, &pout]);
    assert!(
        overlap < 0.01,
        "Inside and Outside overlap by {overlap} mm²"
    );
    let joined_area = op_area(BooleanOp::Union, &[&pin, &pout]);
    let xor = {
        let o_in: Vec<Outline<'_>> = pin.iter().map(|x| Outline::new(x, true)).collect();
        let o_out: Vec<Outline<'_>> = pout.iter().map(|x| Outline::new(x, true)).collect();
        let joined: Vec<Vec<OutlineTriple>> = boolean(
            BooleanOp::Union,
            &[&o_in, &o_out],
            Tolerance::from_mm(0.003),
        )
        .unwrap()
        .outlines()
        .iter()
        .map(|p| p.iter().map(|&q| (q, Vec2::ZERO, Vec2::ZERO)).collect())
        .collect();
        op_area(BooleanOp::Exclusion, &[&joined, &whole])
    };
    let whole_area = op_area(BooleanOp::Union, &[&whole]);
    // No gap at the seam: within 0.05 mm of the slanted side, what the unmasked trace paints and
    // neither Inside nor Outside paints. (Over the whole picture the areas also differ by the
    // flattening of the cut shapes' curves, 0.003 mm, which `0094` criterion 4 bounds below.)
    let (ux, uy) = (
        (b.x - a.x) / 116.619_037_896_906,
        (b.y - a.y) / 116.619_037_896_906,
    );
    let (nx, ny) = (-uy * 0.05, ux * 0.05);
    let strip: Vec<OutlineTriple> = [
        Point::new(a.x + nx - ux, a.y + ny - uy),
        Point::new(b.x + nx + ux, b.y + ny + uy),
        Point::new(b.x - nx + ux, b.y - ny + uy),
        Point::new(a.x - nx - ux, a.y - ny - uy),
    ]
    .iter()
    .map(|&q| (q, Vec2::ZERO, Vec2::ZERO))
    .collect();
    // What the unmasked trace paints and neither part does, in one kernel call (both on one
    // grid), then cut to the strip.
    let missing: Vec<Vec<OutlineTriple>> = {
        let o: [Vec<Outline<'_>>; 3] = [&whole, &pin, &pout]
            .map(|shapes| shapes.iter().map(|x| Outline::new(x, true)).collect());
        match boolean(
            BooleanOp::Difference,
            &[&o[0], &o[1], &o[2]],
            Tolerance::from_mm(0.003),
        ) {
            Ok(r) => r
                .outlines()
                .iter()
                .map(|p| p.iter().map(|&q| (q, Vec2::ZERO, Vec2::ZERO)).collect())
                .collect(),
            Err(BooleanError::EmptyResult) => Vec::new(),
            Err(e) => panic!("{e:?}"),
        }
    };
    let gap = if missing.is_empty() {
        0.0
    } else {
        op_area(
            BooleanOp::Intersection,
            &[&missing, std::slice::from_ref(&strip)],
        )
    };
    assert!(gap < 0.01, "a gap of {gap} mm² along the slanted side");
    assert!(
        (joined_area - whole_area).abs() <= 0.001 * whole_area,
        "the union of Inside and Outside ({joined_area} mm²) against the unmasked trace ({whole_area} mm²)"
    );
    // And in shape no more than `0094` criterion 4 allows (the cut outlines are polylines).
    assert!(xor <= 0.001 * whole_area, "symmetric difference {xor} mm²");
}

/// Criteria 11 and 23: a self-crossing lasso masks its non-zero winding area, and the same
/// stored lasso traces to the same numbers every time.
#[test]
fn c23_a_self_crossing_lasso_traces_by_non_zero_winding_and_deterministically() {
    // A bow tie over the disc: (0, 30) to (50, 70) to (50, 30) to (0, 70) mm.
    let bow = [(0.0, 30.0), (50.0, 70.0), (50.0, 30.0), (0.0, 70.0)];
    let region = part_region(&lasso(&bow, false), PLACEMENT).unwrap();
    assert!(region.keeps(Point::new(10.0, 50.0)) && region.keeps(Point::new(40.0, 50.0)));
    assert!(!region.keeps(Point::new(25.0, 35.0)), "above the crossing");
    let one = trace(Some(&region));
    let again = part_region(&lasso(&bow, false), PLACEMENT).unwrap();
    assert_eq!(region.to_numbers(), again.to_numbers());
    assert_eq!(one, trace(Some(&again)), "byte-identical trace");
    for p in nodes(&outlines(&one)) {
        assert!(
            region.keeps(p) || {
                // On the stencil edge: within 0.01 mm of a kept point.
                [(0.01, 0.0), (-0.01, 0.0), (0.0, 0.01), (0.0, -0.01)]
                    .iter()
                    .any(|&(dx, dy)| region.keeps(Point::new(p.x + dx, p.y + dy)))
            },
            "node {p:?} outside the bow tie"
        );
    }
}
