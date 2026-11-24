//! The Print and cut state of a placed sheet, from its measures to the fit with its warnings
//! (ADR 0081 §5, `specs/0121-print-and-cut` criteria 10 to 14, 16, 18). Text-free; the words
//! are in `print_and_cut_text.rs`.
//!
//! Measures are machine work coordinates in µm; each is turned into laser page-local mm by the
//! page's [`Frame`] (its own inverse), so an origin-corner flip is never read as a mirrored sheet.

use curvyo_document_core::{
    Affine, Length, PRINT_AND_CUT_DEFAULT_TOLERANCE_UM, Point, PrintAndCutMeasures,
    PrintAndCutRegister,
};
use curvyo_geometry_core::{FitError, PointPair, fit_similarity};

use crate::job_input::{PointUm, mm_of_um, um_of_mm};
use crate::job_moves::Frame;
use crate::output_settings::Start;

/// Two points closer than this are one place; no fit below it (criterion 15).
pub const MIN_SPREAD: Length = Length::from_mm(1.0);
/// The reflected fit must beat the plain fit by this much to call the sheet mirrored, so
/// collinear marks never warn.
pub const MIRROR_MARGIN_MM: f64 = 0.01;
/// The scale band without a warning (criterion 14).
pub const SCALE_WARN: (f64, f64) = (0.98, 1.02);
/// The scale band Apply is offered in (ADR 0081 §7).
pub const SCALE_USABLE: (f64, f64) = (0.5, 2.0);
/// The placement is on the fit while the sheet frame's corners differ by at most this, in mm.
pub const SAME_PLACEMENT_MM: f64 = 0.001;

/// What the reading needs of one placed sheet.
#[derive(Debug, Clone, Copy)]
pub struct SheetInput<'a> {
    /// The marks of the sheet's first marks object: number and centre in sheet page-local mm.
    pub marks: &'a [(u8, Point)],
    /// The placement's register.
    pub register: &'a PrintAndCutRegister,
    /// The laser page's frame (origin corner, bed size).
    pub frame: Frame,
    /// The placement's current matrix (sheet page-local to laser page-local mm).
    pub matrix: Affine,
    /// The sheet's width and height in mm.
    pub sheet: (f64, f64),
    /// The laser page's Start.
    pub start: Start,
}

/// What a row of the group is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A mark of the sheet.
    Mark,
    /// A measure whose mark is not on the sheet ("Mark 3 is gone from the sheet"); not counted.
    Gone,
    /// A number the sheet holds twice; not counted.
    Twice,
}

/// One row: a mark number and what was measured for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkRow {
    /// The mark number.
    pub number: u8,
    /// The measured position in µm, machine work coordinates.
    pub measured: Option<[i64; 2]>,
    /// What the row is.
    pub kind: RowKind,
}

/// The fit as the group shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct FitSummary {
    /// The fitted matrix, sheet page-local to laser page-local mm.
    pub map: Affine,
    /// How many pairs it fits.
    pub pairs: usize,
    /// The machine position of the sheet's top-left corner under the fit, in mm.
    pub moved_mm: [f64; 2],
    /// The turn in degrees, counter-clockwise on the screen positive.
    pub turned_deg: f64,
    /// The scale in percent.
    pub scale_pct: f64,
    /// The largest residual in mm.
    pub largest_mm: f64,
    /// The RMS residual in mm.
    pub rms_mm: f64,
    /// The mark with the largest residual.
    pub worst_mark: u8,
    /// Whether the largest residual is above the tolerance (criterion 13).
    pub over_tolerance: bool,
}

/// The state line of the group.
#[derive(Debug, Clone, PartialEq)]
pub enum FitState {
    /// Fewer than 2 usable pairs.
    NotEnough,
    /// The marks or the measures lie within [`MIN_SPREAD`].
    TooClose,
    /// A fit.
    Fit(FitSummary),
}

/// A warning under the fit; every one permits Apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Warning {
    /// The scale is outside [`SCALE_WARN`]; in percent.
    Scale(f64),
    /// The marks look mirrored.
    Mirrored,
    /// Two measured positions within [`MIN_SPREAD`] of each other.
    SamePlace(u8, u8),
    /// Start is Head position.
    HeadPosition,
    /// Start is User origin: the job moves away from the measured bed positions (`specs/0118` O4).
    UserOrigin,
}

/// What Apply offers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Apply {
    /// No fit: greyed.
    Unavailable,
    /// The placement is on the fit: greyed ("The sheet is on the fit").
    OnFit,
    /// Offered; `again` after an earlier Apply and a move by hand.
    Offered {
        /// Whether the label reads "Apply again".
        again: bool,
    },
    /// The scale is outside [`SCALE_USABLE`]: not offered; in percent.
    NotUsable(f64),
}

/// The Print and cut state of one placed sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintAndCut {
    /// Whether the stored register could not be read.
    pub unreadable: bool,
    /// The tolerance in µm.
    pub tolerance_um: i64,
    /// The rows in number order: the sheet's marks, then measures of marks that are gone.
    pub rows: Vec<MarkRow>,
    /// The fit or why there is none.
    pub fit: FitState,
    /// The warnings, in the order of ADR 0081 §5.
    pub warnings: Vec<Warning>,
    /// What Apply offers.
    pub apply: Apply,
    /// Whether Reset placement is offered (a Reset target and the placement on the fit).
    pub reset_offered: bool,
    /// After an Apply (a Reset target exists): whether the placement is on the fit.
    pub on_fit: Option<bool>,
}

/// The measured head position in mm rounded to whole µm (criterion 8).
#[must_use]
pub fn head_position_um(x_mm: f64, y_mm: f64) -> [i64; 2] {
    // invariant: a machine position is far inside ±9e12 mm, so the rounded µm fit an i64.
    #[allow(clippy::cast_possible_truncation)]
    let um = |v: f64| (v * 1000.0).round() as i64;
    [um(x_mm), um(y_mm)]
}

/// Laser page-local mm of a machine point in mm, and back (the map is its own inverse): the
/// job's own [`Frame::machine_point`], to the µm.
#[must_use]
pub fn machine_mm(frame: Frame, p: Point) -> Point {
    let m = frame.machine_point(PointUm::new(um_of_mm(p.x), um_of_mm(p.y)));
    Point::new(mm_of_um(m.x), mm_of_um(m.y))
}

/// The Print and cut state of one placed sheet (ADR 0081 §5).
#[must_use]
pub fn read_print_and_cut(input: &SheetInput<'_>) -> PrintAndCut {
    let (measures, unreadable) = match input.register {
        PrintAndCutRegister::Measures(m) => (m.clone(), false),
        PrintAndCutRegister::Absent => (PrintAndCutMeasures::default(), false),
        PrintAndCutRegister::Unreadable => (PrintAndCutMeasures::default(), true),
    };
    let rows = rows_of(input.marks, &measures);
    let pairs: Vec<(u8, PointPair)> = rows
        .iter()
        .filter(|r| r.kind == RowKind::Mark)
        .filter_map(|r| {
            let [x, y] = r.measured?;
            let centre = input.marks.iter().find(|(n, _)| *n == r.number)?.1;
            let page = input.frame.machine_point(PointUm::new(x, y));
            let to = Point::new(mm_of_um(page.x), mm_of_um(page.y));
            Some((r.number, PointPair { from: centre, to }))
        })
        .collect();
    let mut reading = PrintAndCut {
        unreadable,
        tolerance_um: if unreadable {
            PRINT_AND_CUT_DEFAULT_TOLERANCE_UM
        } else {
            measures.tolerance_um
        },
        rows,
        fit: FitState::NotEnough,
        warnings: Vec::new(),
        apply: Apply::Unavailable,
        reset_offered: false,
        on_fit: None,
    };
    let only: Vec<PointPair> = pairs.iter().map(|(_, p)| *p).collect();
    match fit_similarity(&only, MIN_SPREAD) {
        Err(FitError::TooFewPairs) => {}
        Err(FitError::TooClose) => reading.fit = FitState::TooClose,
        Ok(fit) => {
            let numbers: Vec<u8> = pairs.iter().map(|(n, _)| *n).collect();
            fill_fit(&mut reading, input, &fit, &numbers, &only, &measures);
        }
    }
    match input.start {
        Start::HeadPosition => reading.warnings.push(Warning::HeadPosition),
        Start::UserOrigin => reading.warnings.push(Warning::UserOrigin),
        Start::MachineOrigin | Start::HomeThenOrigin => {}
    }
    reading
}

fn fill_fit(
    reading: &mut PrintAndCut,
    input: &SheetInput<'_>,
    fit: &curvyo_geometry_core::SimilarityFit,
    numbers: &[u8],
    pairs: &[PointPair],
    measures: &PrintAndCutMeasures,
) {
    let worst = fit
        .residuals
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.as_mm().total_cmp(&b.1.as_mm()))
        .map_or(0, |(i, _)| numbers[i]);
    #[allow(clippy::cast_precision_loss)]
    let tolerance_mm = reading.tolerance_um as f64 / 1000.0;
    let moved = machine_mm(input.frame, fit.map.apply_point(Point::new(0.0, 0.0)));
    let scale_pct = fit.scale * 100.0;
    reading.fit = FitState::Fit(FitSummary {
        map: fit.map,
        pairs: pairs.len(),
        moved_mm: [moved.x, moved.y],
        turned_deg: -fit.angle.as_radians().to_degrees(),
        scale_pct,
        largest_mm: fit.largest.as_mm(),
        rms_mm: fit.rms.as_mm(),
        worst_mark: worst,
        over_tolerance: fit.largest.as_mm() > tolerance_mm,
    });
    if !(SCALE_WARN.0..=SCALE_WARN.1).contains(&fit.scale) {
        reading.warnings.push(Warning::Scale(scale_pct));
    }
    if fit
        .mirrored_rms
        .is_some_and(|m| m.as_mm() + MIRROR_MARGIN_MM < fit.rms.as_mm())
    {
        reading.warnings.push(Warning::Mirrored);
    }
    if let Some((a, b)) = same_place(numbers, pairs) {
        reading.warnings.push(Warning::SamePlace(a, b));
    }
    let on_fit = same_placement(input, fit.map);
    let applied = measures.reset.is_some();
    reading.apply = if !(SCALE_USABLE.0..=SCALE_USABLE.1).contains(&fit.scale) {
        Apply::NotUsable(scale_pct)
    } else if on_fit {
        Apply::OnFit
    } else {
        Apply::Offered { again: applied }
    };
    // A fit that is not usable says so; no placement line beside it (UX review).
    let usable = !matches!(reading.apply, Apply::NotUsable(_));
    reading.reset_offered = applied && on_fit && usable;
    reading.on_fit = (applied && usable).then_some(on_fit);
}

/// The first two measured positions within [`MIN_SPREAD`], by mark number.
fn same_place(numbers: &[u8], pairs: &[PointPair]) -> Option<(u8, u8)> {
    (0..pairs.len()).find_map(|i| {
        (i + 1..pairs.len()).find_map(|j| {
            let (p, q) = (pairs[i].to, pairs[j].to);
            ((p.x - q.x).hypot(p.y - q.y) < MIN_SPREAD.as_mm()).then(|| (numbers[i], numbers[j]))
        })
    })
}

/// Whether the sheet frame's four corners under the current and the fitted matrix agree.
fn same_placement(input: &SheetInput<'_>, fit: Affine) -> bool {
    let (width, height) = input.sheet;
    [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)]
        .iter()
        .all(|&(x, y)| {
            let corner = Point::new(x, y);
            let (now, fitted) = (input.matrix.apply_point(corner), fit.apply_point(corner));
            (now.x - fitted.x).hypot(now.y - fitted.y) <= SAME_PLACEMENT_MM
        })
}

/// The rows: every number of the sheet in order (twice-held numbers marked), then each measure
/// whose number the sheet does not hold.
fn rows_of(marks: &[(u8, Point)], measures: &PrintAndCutMeasures) -> Vec<MarkRow> {
    let mut numbers: Vec<u8> = marks.iter().map(|(n, _)| *n).collect();
    numbers.sort_unstable();
    let mut rows: Vec<MarkRow> = Vec::new();
    for n in numbers {
        if let Some(row) = rows.iter_mut().find(|r| r.number == n) {
            row.kind = RowKind::Twice;
            continue;
        }
        rows.push(MarkRow {
            number: n,
            measured: measures.marks.get(&n).copied(),
            kind: RowKind::Mark,
        });
    }
    for (&n, &xy) in &measures.marks {
        if !rows.iter().any(|r| r.number == n) {
            rows.push(MarkRow {
                number: n,
                measured: Some(xy),
                kind: RowKind::Gone,
            });
        }
    }
    rows.sort_by_key(|r| r.number);
    rows
}

#[cfg(test)]
#[path = "print_and_cut_tests.rs"]
mod tests;
