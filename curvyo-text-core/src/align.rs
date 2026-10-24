//! The per-line shift and the justify widths of a laid-out text (`specs/0079-text-layout-controls`
//! criteria 2 and 3).

use crate::units::Mm;

/// How the lines of a text sit on the anchor's x.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Each line's extent starts at the anchor (today's layout).
    #[default]
    Left,
    /// Each line's extent is centred on the anchor.
    Centre,
    /// Each line's extent ends at the anchor.
    Right,
    /// Every line but the last is spread to the widest line; lines start at the anchor.
    Justify,
}

/// What justify needs to know of one placed line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineMeasure {
    /// The extent's width.
    pub width: f64,
    /// The space clusters inside the extent.
    pub spaces: usize,
    /// The clusters inside the extent.
    pub clusters: usize,
}

/// The extra width a justified line gets: per space inside its extent, or, without one, per gap
/// between two clusters.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Extra {
    /// Added to each space cluster inside the extent.
    pub per_space: f64,
    /// Added between each pair of clusters.
    pub per_gap: f64,
}

/// The x shift of a line whose extent is `start..end` under `align` (criterion 2).
pub(crate) fn shift(align: Align, start: Mm, end: Mm) -> f64 {
    match align {
        Align::Left | Align::Justify => 0.0,
        Align::Centre => -(start.0 + end.0) / 2.0,
        Align::Right => -end.0,
    }
}

/// The extra widths of `lines` under Justify (criterion 3): every line but the last with at least
/// two clusters is spread to the widest extent, by its spaces, else by its cluster gaps.
pub(crate) fn justify(lines: &[LineMeasure]) -> Vec<Extra> {
    let widest = lines.iter().map(|line| line.width).fold(0.0, f64::max);
    let last = lines.len().saturating_sub(1);
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let extra = widest - line.width;
            if index == last || line.clusters < 2 || extra <= 0.0 {
                Extra::default()
            } else if line.spaces > 0 {
                Extra {
                    per_space: extra / count(line.spaces),
                    per_gap: 0.0,
                }
            } else {
                Extra {
                    per_space: 0.0,
                    per_gap: extra / count(line.clusters - 1),
                }
            }
        })
        .collect()
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shift_of_each_alignment() {
        assert!((shift(Align::Centre, Mm(0.0), Mm(10.0)) + 5.0).abs() < 1e-12);
        assert!((shift(Align::Right, Mm(0.0), Mm(10.0)) + 10.0).abs() < 1e-12);
        assert!(shift(Align::Left, Mm(0.0), Mm(10.0)).abs() < 1e-12);
        assert!(shift(Align::Justify, Mm(0.0), Mm(10.0)).abs() < 1e-12);
    }

    #[test]
    fn justify_by_spaces_then_by_gaps() {
        let lines = [
            LineMeasure {
                width: 6.0,
                spaces: 2,
                clusters: 5,
            },
            LineMeasure {
                width: 7.0,
                spaces: 0,
                clusters: 4,
            },
            LineMeasure {
                width: 10.0,
                spaces: 0,
                clusters: 1,
            },
            LineMeasure {
                width: 1.0,
                spaces: 1,
                clusters: 3,
            },
        ];
        let extra = justify(&lines);
        assert!((extra[0].per_space - 2.0).abs() < 1e-12);
        assert!((extra[1].per_gap - 1.0).abs() < 1e-12);
        assert_eq!(extra[2], Extra::default());
        assert_eq!(extra[3], Extra::default(), "the last line stays");
    }
}
