//! The HPGL writer (`specs/0159-cutting-plotter` criteria 13 to 16, `adrs.md` PL7): speed and
//! force per operation, pen-up and pen-down runs in plotter units of 0.025 mm with the profile's
//! origin, repeats after rounding dropped.

use std::fmt::Write as _;

use curvyo_document_core::DocumentSize;
use curvyo_geometry_core::{PointUm, um_of_mm};

use crate::job::CutterJob;
use crate::numbers::SpeedMmS;
use crate::profile::{ForceRange, OriginCorner};

/// Plotter units per mm (0.025 mm each).
pub const UNITS_PER_MM: i64 = 40;

/// A coordinate in whole plotter units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlotterUnit(pub i64);

/// A point in plotter units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlotterPoint {
    x: PlotterUnit,
    y: PlotterUnit,
}

/// `um` micrometres in plotter units, rounded half away from zero (25 µm each).
#[must_use]
pub fn plotter_units(um: i64) -> PlotterUnit {
    let step = 1_000 / UNITS_PER_MM;
    let half = step / 2;
    let rounded = if um >= 0 {
        (um + half) / step
    } else {
        -((-um + half) / step)
    };
    PlotterUnit(rounded)
}

/// The HPGL of `job` on a page of `page` size, with the plotter's `origin` and `force` range.
#[must_use]
pub fn write_hpgl(
    job: &CutterJob,
    page: DocumentSize,
    origin: OriginCorner,
    force: ForceRange,
) -> String {
    let height_um = um_of_mm(page.height.as_mm());
    let to_plotter = |p: PointUm| PlotterPoint {
        x: plotter_units(p.x),
        y: match origin {
            OriginCorner::TopLeft => plotter_units(p.y),
            OriginCorner::BottomLeft => plotter_units(height_um - p.y),
        },
    };
    let mut out = String::from("IN;\n");
    for run in &job.operations {
        if run.paths.is_empty() {
            continue;
        }
        let _ = writeln!(out, "VS{};", cm_per_s(run.settings.speed));
        let _ = writeln!(out, "FS{};", force.force_of(run.settings.pressure));
        for path in run.runs() {
            let mut points: Vec<PlotterPoint> = Vec::new();
            for p in path.iter().copied().map(to_plotter) {
                if points.last() != Some(&p) {
                    points.push(p);
                }
            }
            let Some((first, rest)) = points.split_first() else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            let _ = writeln!(out, "PU{},{};", first.x.0, first.y.0);
            let coords: Vec<String> = rest
                .iter()
                .map(|p| format!("{},{}", p.x.0, p.y.0))
                .collect();
            let _ = writeln!(out, "PD{};", coords.join(","));
        }
    }
    out.push_str("PU;\nSP0;\n");
    out
}

/// `speed` in cm/s as HPGL `VS` takes it: whole, or with one decimal (Proposal).
fn cm_per_s(speed: SpeedMmS) -> String {
    let mm = speed.get();
    if mm.is_multiple_of(10) {
        (mm / 10).to_string()
    } else {
        format!("{}.{}", mm / 10, mm % 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micrometres_round_half_away_from_zero_to_plotter_units() {
        assert_eq!(plotter_units(0), PlotterUnit(0));
        assert_eq!(plotter_units(12), PlotterUnit(0));
        assert_eq!(plotter_units(13), PlotterUnit(1));
        assert_eq!(plotter_units(50_000), PlotterUnit(2_000));
        assert_eq!(plotter_units(-13), PlotterUnit(-1));
        assert_eq!(plotter_units(-12), PlotterUnit(0));
    }

    #[test]
    fn the_speed_is_written_in_cm_per_second() {
        assert_eq!(cm_per_s(SpeedMmS::new(200).unwrap()), "20");
        assert_eq!(cm_per_s(SpeedMmS::new(25).unwrap()), "2.5");
    }
}
