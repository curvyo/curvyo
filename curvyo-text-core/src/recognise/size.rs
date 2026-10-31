//! Finds a line's size and baseline for one face (`adrs.md` decision 3, stage 5).

use super::candidates::Prepared;
use super::lines::median;
use super::reference::ReferenceGlyph;
use super::verify::FaceFit;

/// Scales within this share of each other count as one size in the vote (Proposal).
const SCALE_AGREEMENT: f64 = 0.03;

/// The size and baseline of the line (stage 5), by a vote: every shortlisted character of every
/// single-shape candidate proposes the scale that makes its height the candidate's; the scale
/// most candidates agree with (within [`SCALE_AGREEMENT`]) wins, ties by the lower summed
/// descriptor distance of the agreeing characters (so an 'o' read as a small 'O' loses to the
/// 'o'), and a few wrong first choices of a short word do not set the size. The window then moves
/// to the median of what agrees with it; the baseline is the median of the agreeing proposals.
pub(crate) fn size_and_baseline(
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
    lists: &[Vec<(usize, f64)>],
) -> Option<(f64, f64)> {
    // Per candidate, its proposals: (scale, baseline, descriptor distance).
    let proposals: Vec<Vec<(f64, f64, f64)>> = prepared
        .candidates
        .iter()
        .zip(lists)
        .filter(|(candidate, _)| candidate.len == 1)
        .map(|(candidate, list)| {
            list.iter()
                .filter(|&&(g, _)| glyphs[g].bbox.height() >= 0.05)
                .map(|&(g, d)| {
                    let reference = &glyphs[g];
                    let scale = candidate.bbox.height() / reference.bbox.height();
                    (scale, candidate.bbox.y1 - reference.bbox.y1 * scale, d)
                })
                .collect()
        })
        .collect();
    // Per candidate, its agreeing proposal nearest in descriptor distance.
    let support = |scale: f64| -> Vec<(f64, f64, f64)> {
        proposals
            .iter()
            .filter_map(|own| {
                own.iter()
                    .copied()
                    .filter(|p| (p.0 - scale).abs() <= SCALE_AGREEMENT * scale)
                    .min_by(|a, b| a.2.total_cmp(&b.2))
            })
            .collect()
    };
    let mut best: Option<(usize, f64, f64)> = None;
    for &(scale, _, _) in proposals.iter().flatten() {
        let agreeing = support(scale);
        let votes = agreeing.len();
        let distance: f64 = agreeing.iter().map(|p| p.2).sum();
        if best.is_none_or(|(n, d, s)| {
            votes > n || (votes == n && distance.total_cmp(&d).then(scale.total_cmp(&s)).is_lt())
        }) {
            best = Some((votes, distance, scale));
        }
    }
    let (_, _, mut guess) = best?;
    for _ in 0..4 {
        let mut scales: Vec<f64> = support(guess).iter().map(|p| p.0).collect();
        let centre = median(&mut scales);
        if centre.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
            || (centre - guess).abs() <= 1e-9 * guess
        {
            break;
        }
        guess = centre;
    }
    let agreeing = support(guess);
    let mut scales: Vec<f64> = agreeing.iter().map(|p| p.0).collect();
    let mut baselines: Vec<f64> = agreeing.iter().map(|p| p.1).collect();
    let scale = median(&mut scales);
    let baseline = median(&mut baselines);
    (scale.is_finite() && scale > 0.0).then_some((scale, baseline))
}

/// The scale and baseline the fitted letters themselves give: the median of their candidates'
/// heights over their glyphs' heights, over the letters within the tolerance (all letters that
/// are text when fewer than two are). A vote over first choices is a few per cent off; at large
/// sizes that alone is more than the tolerance (tester finding 2026-10-31).
pub(crate) fn refined(
    fit: &FaceFit,
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
    tolerance: f64,
) -> Option<(f64, f64)> {
    let pairs = |within: bool| -> Vec<(f64, f64)> {
        fit.letters
            .iter()
            .filter(|l| !l.stays_path && (!within || l.best.distance.max <= tolerance))
            .filter_map(|l| {
                let candidate = &prepared.candidates[l.candidate];
                let reference = &glyphs[l.best.glyph];
                (candidate.len == 1 && reference.bbox.height() >= 0.05).then(|| {
                    let scale = candidate.bbox.height() / reference.bbox.height();
                    (scale, candidate.bbox.y1 - reference.bbox.y1 * scale)
                })
            })
            .collect()
    };
    let mut found = pairs(true);
    if found.len() < 2 {
        found = pairs(false);
    }
    if found.is_empty() {
        return None;
    }
    let mut scales: Vec<f64> = found.iter().map(|p| p.0).collect();
    let mut baselines: Vec<f64> = found.iter().map(|p| p.1).collect();
    let scale = median(&mut scales);
    (scale.is_finite() && scale > 0.0).then(|| (scale, median(&mut baselines)))
}
