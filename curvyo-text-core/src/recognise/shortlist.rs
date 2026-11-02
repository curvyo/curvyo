//! Ranks the faces of a tier for a line by descriptor distance and shortlists each candidate's
//! nearest characters (`adrs.md` decision 3, stage 4).

use std::sync::Arc;

use super::candidates::Prepared;
use super::reference::{ReferenceGlyph, ReferenceGlyphs};

/// Characters per candidate and face that go on to the exact distance.
const SHORTLIST: usize = 4;
/// Faces per tier that go on to the exact distance.
const FACES_VERIFIED: usize = 8;
/// With an OCR hint (`specs/0091` decision 4) the shortlist holds only the read characters and
/// their look-alikes, so the descriptor ranks the faces and characters well enough that fewer go
/// on to the exact distance (Proposal, measured on the reference set).
const FACES_VERIFIED_HINTED: usize = 3;
const SHORTLIST_HINTED: usize = 3;
/// The shortlisted reference glyphs of every candidate, nearest descriptor first; with `allowed`,
/// only glyphs of those characters (an OCR hint, `specs/0091` `adrs.md` decision 4).
fn shortlist(
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
    allowed: Option<&[char]>,
) -> Vec<Vec<(usize, f64)>> {
    prepared
        .candidates
        .iter()
        .map(|candidate| {
            let mut scored: Vec<(usize, f64)> = glyphs
                .iter()
                .enumerate()
                .filter(|(_, g)| allowed.is_none_or(|chars| chars.contains(&g.c)))
                .map(|(i, g)| (i, candidate.descriptor.distance(&g.descriptor)))
                .collect();
            scored.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            scored.truncate(if allowed.is_some() {
                SHORTLIST_HINTED
            } else {
                SHORTLIST
            });
            scored
        })
        .collect()
}

/// The shortlist of one candidate: reference glyph indices and descriptor distances.
pub(crate) type Shortlist = Vec<(usize, f64)>;
/// A face of the tier as ranked: its index, descriptor sum and per-candidate shortlists.
pub(crate) type RankedFace = (usize, f64, Vec<Shortlist>);

/// Ranks the faces of a tier by their descriptor sums over single-shape candidates and keeps the
/// best [`FACES_VERIFIED`] (stage 4), with their sums and shortlists; `allowed` as in
/// [`shortlist`].
pub(crate) fn rank_faces(
    prepared: &Prepared,
    faces: &[Arc<ReferenceGlyphs>],
    allowed: Option<&[char]>,
) -> Vec<RankedFace> {
    let mut ranked: Vec<RankedFace> = faces
        .iter()
        .enumerate()
        .filter(|(_, glyphs)| !glyphs.glyphs.is_empty())
        .map(|(face, glyphs)| {
            let lists = shortlist(prepared, &glyphs.glyphs, allowed);
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
    ranked.truncate(if allowed.is_some() {
        FACES_VERIFIED_HINTED
    } else {
        FACES_VERIFIED
    });
    ranked
}
