//! Ranks the faces of a tier for a line by descriptor distance and shortlists each candidate's
//! nearest characters (`adrs.md` decision 3, stage 4).

use std::sync::Arc;

use super::candidates::Prepared;
use super::reference::{ReferenceGlyph, ReferenceGlyphs};

/// Characters per candidate and face that go on to the exact distance.
const SHORTLIST: usize = 4;
/// Faces per tier that go on to the exact distance.
const FACES_VERIFIED: usize = 8;
/// The shortlisted reference glyphs of every candidate, nearest descriptor first.
fn shortlist(prepared: &Prepared, glyphs: &[ReferenceGlyph]) -> Vec<Vec<(usize, f64)>> {
    prepared
        .candidates
        .iter()
        .map(|candidate| {
            let mut scored: Vec<(usize, f64)> = glyphs
                .iter()
                .enumerate()
                .map(|(i, g)| (i, candidate.descriptor.distance(&g.descriptor)))
                .collect();
            scored.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            scored.truncate(SHORTLIST);
            scored
        })
        .collect()
}

/// The shortlist of one candidate: reference glyph indices and descriptor distances.
pub(crate) type Shortlist = Vec<(usize, f64)>;
/// A face of the tier as ranked: its index, descriptor sum and per-candidate shortlists.
pub(crate) type RankedFace = (usize, f64, Vec<Shortlist>);

/// Ranks the faces of a tier by their descriptor sums over single-shape candidates and keeps the
/// best [`FACES_VERIFIED`] (stage 4), with their sums and shortlists.
pub(crate) fn rank_faces(prepared: &Prepared, faces: &[Arc<ReferenceGlyphs>]) -> Vec<RankedFace> {
    let mut ranked: Vec<RankedFace> = faces
        .iter()
        .enumerate()
        .filter(|(_, glyphs)| !glyphs.glyphs.is_empty())
        .map(|(face, glyphs)| {
            let lists = shortlist(prepared, &glyphs.glyphs);
            let sum = prepared
                .candidates
                .iter()
                .zip(&lists)
                .filter(|(c, _)| c.len == 1)
                .map(|(_, list)| list.first().map_or(256.0, |(_, d)| *d))
                .sum();
            (face, sum, lists)
        })
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    ranked.truncate(FACES_VERIFIED);
    ranked
}
