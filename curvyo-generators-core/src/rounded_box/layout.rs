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
}

/// A placed part and the offset its own coordinates were moved by.
pub(crate) struct Placed {
    pub(crate) part: Part,
    pub(crate) offset: (f64, f64),
    pub(crate) role: &'static str,
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

/// Places the strip and the plates; each part carries the kerf-offset paths. `spacing` is the
/// gap plus the kerf.
pub(crate) fn place(strip: &Piece, plates: Vec<Piece>, kerf: f64, spacing: f64) -> Vec<Placed> {
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
    let first = [strip];
    let second: Vec<&Piece> = plates.iter().collect();
    let rows = [first.to_vec(), second];
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
                    paths: piece
                        .contours
                        .iter()
                        .map(|c| translated(&c.offset(kerf / 2.0).to_path(), offset.0, offset.1))
                        .collect::<Vec<ClosedPath>>(),
                },
                offset,
                role: piece.role,
            });
            x += b.2 - b.0 + spacing;
        }
        y += height(row_pieces) + spacing;
    }
    placed
}

/// The number of gaps between `count` neighbours, as a factor.
fn gaps(count: usize) -> f64 {
    f64::from(u32::try_from(count.saturating_sub(1)).unwrap_or(0))
}
