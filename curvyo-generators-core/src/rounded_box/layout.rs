//! The flat layout of the rounded box: the strip in the first row, the plates in a second row in
//! runs of equal thickness, everything centred on the origin (ADR 0082 §5 "Layout").

use curvyo_extension_core::{ClosedPath, Part};

use crate::contour::Contour;
use crate::outline::translated;

/// One part before it is placed: its contours in the part's own coordinates (outline first) and
/// the thickness that decides its run.
pub(crate) struct Piece {
    pub(crate) contours: Vec<Contour>,
    pub(crate) thickness: f64,
    /// Which part it is: `strip`, `cover`, `lid`, `base` or `ring`.
    pub(crate) role: &'static str,
    /// The number of a wall piece of a path box (1 based), 0 for any other part.
    pub(crate) number: u32,
    /// Holes already offset by the kerf, in the part's own coordinates (the ring's opening of a
    /// path box, which comes from the boolean offset and not from a contour).
    pub(crate) holes: Vec<ClosedPath>,
}

impl Piece {
    /// A part of the rounded box: contours, thickness and role.
    pub(crate) fn new(contours: Vec<Contour>, thickness: f64, role: &'static str) -> Self {
        Self {
            contours,
            thickness,
            role,
            number: 0,
            holes: Vec::new(),
        }
    }
}

/// A placed part and the offset its own coordinates were moved by.
pub(crate) struct Placed {
    pub(crate) part: Part,
    pub(crate) offset: (f64, f64),
    pub(crate) role: &'static str,
    pub(crate) number: u32,
}

/// The plates in the order of their runs: parts of equal thickness together, runs in the order of
/// their first part.
fn by_runs(plates: Vec<Piece>) -> Vec<Piece> {
    let mut runs: Vec<(f64, Vec<Piece>)> = Vec::new();
    for plate in plates {
        match runs
            .iter_mut()
            .find(|(thickness, _)| (thickness - plate.thickness).abs() < 1e-9)
        {
            Some((_, run)) => run.push(plate),
            None => runs.push((plate.thickness, vec![plate])),
        }
    }
    runs.into_iter().flat_map(|(_, run)| run).collect()
}

fn size(piece: &Piece) -> (f64, f64, f64, f64) {
    piece.contours[0].bounds()
}

/// Places the strip pieces (one for the rounded box) in the first row and the plates in the
/// second; each part carries the kerf-offset paths. `spacing` is the gap plus the kerf.
pub(crate) fn place(strips: &[Piece], plates: Vec<Piece>, kerf: f64, spacing: f64) -> Vec<Placed> {
    let plates = by_runs(plates);
    let row = |pieces: &[&Piece]| -> f64 {
        let widths: f64 = pieces
            .iter()
            .map(|piece| {
                let b = size(piece);
                b.2 - b.0
            })
            .sum();
        widths + spacing * gaps(pieces.len())
    };
    let height = |pieces: &[&Piece]| {
        pieces
            .iter()
            .map(|piece| {
                let b = size(piece);
                b.3 - b.1
            })
            .fold(0.0, f64::max)
    };
    let first: Vec<&Piece> = strips.iter().collect();
    let second: Vec<&Piece> = plates.iter().collect();
    let rows = [first, second];
    let filled: Vec<&Vec<&Piece>> = rows.iter().filter(|r| !r.is_empty()).collect();
    let total_height: f64 =
        filled.iter().map(|r| height(r)).sum::<f64>() + spacing * gaps(filled.len());
    let mut y = -total_height / 2.0;
    let mut placed = Vec::new();
    for row_pieces in &rows {
        if row_pieces.is_empty() {
            continue;
        }
        let mut x = -row(row_pieces) / 2.0;
        for piece in row_pieces {
            let b = size(piece);
            let offset = (x - b.0, y - b.1);
            placed.push(Placed {
                part: Part {
                    key: part_key(piece.role, piece.number),
                    paths: piece
                        .contours
                        .iter()
                        .map(|c| translated(&c.offset(kerf / 2.0).to_path(), offset.0, offset.1))
                        .chain(
                            piece
                                .holes
                                .iter()
                                .map(|hole| translated(hole, offset.0, offset.1)),
                        )
                        .collect::<Vec<ClosedPath>>(),
                },
                offset,
                role: piece.role,
                number: piece.number,
            });
            x += b.2 - b.0 + spacing;
        }
        y += height(row_pieces) + spacing;
    }
    placed
}

/// The part key of a piece: its role, a wall piece of a path box `wall-<n>` (ADR 0082 §10).
fn part_key(role: &'static str, number: u32) -> std::borrow::Cow<'static, str> {
    if role == "wall" {
        format!("wall-{number}").into()
    } else {
        role.into()
    }
}

/// The number of gaps between `count` neighbours, as a factor.
fn gaps(count: usize) -> f64 {
    f64::from(u32::try_from(count.saturating_sub(1)).unwrap_or(0))
}
