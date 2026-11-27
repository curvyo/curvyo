//! Where the tabs of one straight edge stand: their count, size and pattern, and what fits (spec
//! `0155` criteria 19 to 23, ADR 0089 decision 3).
//!
//! Pure 1-D arithmetic along an edge of a given length; the footprint code (`tab_layout`) calls it
//! once per straight. Everything is millimetres.

use curvyo_extension_core::{ParamValue, Warning};

use crate::fingers::Split;
use crate::values::Values;

/// How the tabs of an edge are spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pattern {
    /// Equal pitch along the whole edge (today's layout).
    Even,
    /// Grouped about the middle, the gaps widening toward the ends.
    Centred,
}

/// What the maker asked for; `None` is automatic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Joints {
    pub(crate) pattern: Pattern,
    /// Tabs per edge (a whole loop when there is no straight).
    pub(crate) count: Option<u32>,
    /// The tab width along the edge; automatic is twice each plate's own thickness.
    pub(crate) width: Option<f64>,
    /// Edge-to-edge gap of the middle pair, Centred only; automatic is twice the width.
    pub(crate) gap: Option<f64>,
    /// 0 is the Centred layout, 1 the Even one.
    pub(crate) spread: f64,
}

/// Every key automatic and Even: the layout of before.
#[cfg(test)]
pub(crate) const AUTO: Joints = Joints {
    pattern: Pattern::Even,
    count: None,
    width: None,
    gap: None,
    spread: 0.0,
};

/// The distance between the centres of two neighbours is at least this times the width: tab and
/// bridge are equal (**Proposal**, ADR 0089 decision 3).
const LEAST_GAP: f64 = 1.0;
/// The aimed distance between two tabs on an automatic count.
const AUTO_PITCH: f64 = 30.0;

impl Joints {
    /// The values as stored (a stored 0 is automatic).
    pub(crate) fn read(v: &Values<'_>, width_key: &str) -> Self {
        let some = |mm: f64| (mm > 0.0).then_some(mm);
        Self {
            pattern: if v.choice("joint_pattern") == "centred" {
                Pattern::Centred
            } else {
                Pattern::Even
            },
            count: Some(v.count("joint_count")).filter(|n| *n > 0),
            width: some(v.mm(width_key)),
            gap: some(v.mm("joint_gap")),
            spread: f64::from(v.count("joint_spread").min(100)) / 100.0,
        }
    }

    /// The half width of a tab of a plate whose own half width is `own`.
    pub(crate) fn half(&self, own: f64) -> f64 {
        self.width.map_or(own, |width| width / 2.0)
    }

    /// Whether a typed count decides the tabs (it also suppresses windows on the bends).
    pub(crate) fn typed(&self) -> bool {
        self.count.is_some()
    }
}

/// The sum of the gap multipliers of `count` tabs: the gap at rank k, counted outward from the
/// middle, is k times the Joint gap.
fn gap_units(count: u32) -> u32 {
    let n = i64::from(count);
    (0..n - 1)
        .map(|j| {
            let d = (2 * j - (n - 2)).abs();
            u32::try_from(if n % 2 == 0 { d / 2 + 1 } else { (d + 1) / 2 }).unwrap_or(0)
        })
        .sum()
}

/// The Even centres of `count` tabs on `length`.
fn even_centres(length: f64, count: u32) -> Vec<f64> {
    #[allow(clippy::cast_precision_loss)]
    let pitch = length / f64::from(count);
    (0..count)
        .map(|index| pitch * (f64::from(index) + 0.5))
        .collect()
}

/// The Centred centres: tabs of `width` with the gap `gap` between the middle pair (or either
/// side of the middle tab) and `gap` more at each rank outward.
fn centred_centres(length: f64, count: u32, width: f64, gap: f64) -> Vec<f64> {
    let n = i64::from(count);
    let total = width * f64::from(count) + gap * f64::from(gap_units(count));
    let mut at = (length - total) / 2.0;
    let mut out = Vec::new();
    for index in 0..n {
        out.push(at + width / 2.0);
        at += width;
        if index < n - 1 {
            let d = (2 * index - (n - 2)).abs();
            let rank = if n % 2 == 0 { d / 2 + 1 } else { (d + 1) / 2 };
            #[allow(clippy::cast_precision_loss)]
            {
                at += gap * rank as f64;
            }
        }
    }
    out
}

/// The centres of `count` tabs of `width` on an edge of `length`, with `gap` between the middle
/// pair (Centred) already limited to what fits.
pub(crate) fn centres(length: f64, count: u32, width: f64, joints: &Joints, gap: f64) -> Vec<f64> {
    let even = even_centres(length, count);
    if joints.pattern == Pattern::Even || count == 0 {
        return even;
    }
    if count == 1 {
        return vec![length / 2.0];
    }
    let centred = centred_centres(length, count, width, gap);
    centred
        .iter()
        .zip(&even)
        .map(|(c, e)| c + (e - c) * joints.spread)
        .collect()
}

/// One straight edge a tab may stand on: the whole straight's length and whether the seam cuts it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Edge {
    pub(crate) length: f64,
    pub(crate) seam: bool,
}

impl Edge {
    /// The tabs of this edge for the typed or automatic count (the seam takes an even count).
    fn effective(self, count: u32) -> u32 {
        if self.seam && count % 2 == 1 {
            count + 1
        } else {
            count
        }
    }

    /// Whether `count` tabs of `width` hold with the margins and neighbours apart.
    fn holds(self, count: u32, width: f64, margin: f64, joints: &Joints) -> bool {
        let n = f64::from(count);
        let even = self.length / (2.0 * n) - width / 2.0 >= margin - 1e-9
            && self.length / n >= (1.0 + LEAST_GAP) * width - 1e-9;
        let need_even = joints.pattern == Pattern::Even || (count >= 2 && joints.spread > 0.0);
        let centred = n * width + f64::from(gap_units(count)) * LEAST_GAP * width
            <= self.length - 2.0 * margin + 1e-9;
        (!need_even || even) && (joints.pattern == Pattern::Even || centred)
    }

    /// The most tabs a typed count may ask for here (0 when not even one fits).
    pub(crate) fn most(self, width: f64, margin: f64, joints: &Joints) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let upper = (self.length / width).floor().max(0.0) as u32;
        (1..=upper)
            .rev()
            .find(|n| self.holds(self.effective(*n), width, margin, joints))
            .unwrap_or(0)
    }

    /// The count on this edge: typed, or automatic (about one per 30 mm, at most what fits when
    /// Centred).
    pub(crate) fn count(self, width: f64, margin: f64, joints: &Joints) -> u32 {
        if let Some(typed) = joints.count {
            return self.effective(typed);
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let auto = self.effective(((self.length / AUTO_PITCH).round() as u32).max(1));
        if joints.pattern == Pattern::Centred {
            let most = self.most(width, margin, joints);
            return self.effective(auto.min(most.max(1)));
        }
        auto
    }

    /// The gap of the middle pair on this edge: asked for (automatic is twice the width), lowered
    /// to what the edge holds.
    pub(crate) fn gap(self, count: u32, width: f64, margin: f64, joints: &Joints) -> f64 {
        let asked = joints.gap.unwrap_or(2.0 * width);
        let units = gap_units(count);
        if units == 0 {
            return asked;
        }
        let room = (self.length - 2.0 * margin - f64::from(count) * width) / f64::from(units);
        asked.min(room.max(width * LEAST_GAP))
    }
}

/// A value moved to what the edges allow, to tell the maker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Limit {
    WidthMost(f64),
    CountMost(u32),
    GapMost(f64),
    GapLeast(f64),
}

/// The typed values after the fit, what was changed, and the count the edges use.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fitted {
    pub(crate) joints: Joints,
    pub(crate) limits: Vec<Limit>,
    /// The most tabs on one edge (the muted Auto count).
    pub(crate) used_count: Option<u32>,
}

/// Fits the typed values to the tightest of `edges` (every straight that carries tabs); `auto`
/// is the width of an automatic tab (twice the widest plate).
pub(crate) fn fit(edges: &[Edge], margin: f64, auto: f64, asked: Joints) -> Fitted {
    let mut joints = asked;
    let mut limits = Vec::new();
    let usable: Vec<Edge> = edges
        .iter()
        .copied()
        .filter(|edge| edge.length - 2.0 * margin >= 1.0)
        .collect();
    if let Some(width) = joints.width
        && let Some(room) = usable
            .iter()
            .map(|e| e.length - 2.0 * margin)
            .reduce(f64::min)
        && width > room + 1e-9
    {
        joints.width = Some(room);
        limits.push(Limit::WidthMost(room));
    }
    let width = joints.width.unwrap_or(auto);
    if let Some(typed) = joints.count
        && let Some(most) = usable
            .iter()
            .map(|e| e.most(width, margin, &joints))
            .filter(|n| *n > 0)
            .min()
        && typed > most
    {
        joints.count = Some(most);
        limits.push(Limit::CountMost(most));
    }
    if joints.pattern == Pattern::Centred
        && let Some(gap) = joints.gap
    {
        {
            let least = LEAST_GAP * width;
            let tightest = usable
                .iter()
                .map(|e| {
                    let n = e.count(width, margin, &joints);
                    e.gap(n, width, margin, &joints)
                })
                .reduce(f64::min);
            if gap < least - 1e-9 {
                joints.gap = Some(least);
                limits.push(Limit::GapLeast(least));
            } else if let Some(most) = tightest
                && gap > most + 1e-9
            {
                joints.gap = Some(most);
                limits.push(Limit::GapMost(most));
            }
        }
    }
    let used_count = usable.iter().map(|e| e.count(width, margin, &joints)).max();
    Fitted {
        joints,
        limits,
        used_count,
    }
}

/// Fits the typed values to the tightest finger joint (`spans`, mm): nothing changes at Even with
/// an automatic count, so a default box never gets a notice. The owner keeps at least half a
/// finger at each end. `auto` is the automatic finger width.
pub(crate) fn fit_fingers(spans: &[f64], auto: f64, asked: Joints) -> Fitted {
    if asked.pattern == Pattern::Even && !asked.typed() {
        return Fitted {
            joints: asked,
            limits: Vec::new(),
            used_count: None,
        };
    }
    let edges: Vec<Edge> = spans
        .iter()
        .map(|length| Edge {
            length: *length,
            seam: false,
        })
        .collect();
    fit(&edges, asked.width.unwrap_or(auto) / 2.0, auto, asked)
}

/// Moves the stored values `limits` name and reports each as a dependent clamp.
pub(crate) fn apply(v: &mut Values<'_>, limits: &[Limit], width_key: &'static str) {
    for limit in limits {
        match *limit {
            Limit::WidthMost(mm) => v.limit_mm(width_key, 0.0, mm),
            Limit::CountMost(n) => v.limit_count("joint_count", n),
            Limit::GapMost(mm) => v.limit_mm("joint_gap", 0.0, mm),
            Limit::GapLeast(mm) => v.limit_mm("joint_gap", mm, f64::MAX),
        }
    }
}

/// The pieces of the finger joint along a span of `span` mm for the plates that meet there, whose
/// automatic finger width is `auto`: today's equal pieces at Even with an automatic count, else
/// the fingers at the centres a tab layout would use, with the owner's pieces between and at the
/// ends (spec `0155` criterion 26). Both plates call it with the same numbers.
pub(crate) fn finger_split(span: f64, auto: f64, joints: &Joints) -> Split {
    let width = joints.width.unwrap_or(auto);
    let Some(centres) = places(span, width, joints) else {
        return Split::equal(span, width);
    };
    let mut widths = Vec::new();
    let mut at = 0.0;
    for c in centres {
        widths.push(c - width / 2.0 - at);
        widths.push(width);
        at = c + width / 2.0;
    }
    widths.push(span - at);
    Split {
        segments: u32::try_from(widths.len()).unwrap_or(1),
        widths,
    }
}

/// The centres of the joints of width `width` on a span of `span` mm when the layout is not
/// today's (Centred or a typed count); `None` at the defaults and where not even one fits. The
/// owner keeps at least half a finger at each end.
pub(crate) fn places(span: f64, width: f64, joints: &Joints) -> Option<Vec<f64>> {
    if joints.pattern == Pattern::Even && !joints.typed() {
        return None;
    }
    let edge = Edge {
        length: span,
        seam: false,
    };
    let margin = width / 2.0;
    let most = edge.most(width, margin, joints);
    if most == 0 {
        return None;
    }
    let count = edge.count(width, margin, joints).min(most);
    let gap = edge.gap(count, width, margin, joints);
    Some(centres(span, count, width, joints, gap))
}

/// The notice of an unknown Joint pattern id, which reads Even (spec `0155` criterion 19).
pub(crate) fn pattern_warnings(v: &Values<'_>) -> Vec<Warning> {
    notice(
        "unknown_joint_pattern",
        v.unknown_choice("joint_pattern"),
        None,
    )
}

/// The notice of `count` tabs that keep their flat window on a bend although Bend through tabs
/// is `curve` (criterion 16); none when `count` is 0.
pub(crate) fn window_warnings(count: u32) -> Vec<Warning> {
    notice("tab_window", count > 0, Some(ParamValue::Count(count)))
}

fn notice(id: &'static str, told: bool, value: Option<ParamValue>) -> Vec<Warning> {
    told.then(|| Warning {
        id,
        pieces: Vec::new(),
        value,
        limit: None,
        param: None,
        subject: None,
    })
    .into_iter()
    .collect()
}

/// The least tab width: 1.5 times the kerf plus 1 mm (criterion 23).
pub(crate) fn least_width(kerf: f64) -> f64 {
    1.5 * kerf + 1.0
}

/// The automatic values the generator used, for the muted rows.
pub(crate) fn used(
    v: &Values<'_>,
    fitted: &Fitted,
    (width_key, auto_width): (&'static str, f64),
) -> Vec<(&'static str, ParamValue)> {
    let mut out = Vec::new();
    if v.count("joint_count") == 0
        && let Some(n) = fitted.used_count
    {
        out.push(("joint_count", ParamValue::Count(n)));
    }
    let length = |mm: f64| ParamValue::Length(curvyo_document_core::Length::from_mm(mm));
    if v.mm(width_key) == 0.0 {
        out.push((width_key, length(auto_width)));
    }
    if v.mm("joint_gap") == 0.0 {
        out.push((
            "joint_gap",
            length(2.0 * fitted.joints.width.unwrap_or(auto_width)),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centred(spread: f64, gap: Option<f64>) -> Joints {
        Joints {
            pattern: Pattern::Centred,
            gap,
            spread,
            ..AUTO
        }
    }

    #[test]
    fn count_one_is_the_middle_of_the_edge() {
        for length in [40.0, 100.0, 333.3] {
            let c = centres(length, 1, 6.0, &centred(0.0, None), 12.0);
            assert!((c[0] - length / 2.0).abs() < 0.01);
        }
    }

    #[test]
    fn a_pair_keeps_the_gap_between_its_edges() {
        let c = centres(200.0, 2, 6.0, &centred(0.0, Some(10.0)), 10.0);
        assert!((c[1] - c[0] - 16.0).abs() < 1e-9, "{c:?}");
        assert!((c[0] + c[1] - 200.0).abs() < 1e-9, "symmetric");
    }

    #[test]
    fn each_gap_outward_adds_the_joint_gap() {
        let c = centres(400.0, 5, 6.0, &centred(0.0, Some(8.0)), 8.0);
        let gaps: Vec<f64> = c.windows(2).map(|w| w[1] - w[0] - 6.0).collect();
        for (got, want) in gaps.iter().zip([16.0, 8.0, 8.0, 16.0]) {
            assert!((got - want).abs() < 1e-9, "{gaps:?}");
        }
        let c4 = centres(400.0, 4, 6.0, &centred(0.0, Some(8.0)), 8.0);
        let gaps4: Vec<f64> = c4.windows(2).map(|w| w[1] - w[0] - 6.0).collect();
        for (got, want) in gaps4.iter().zip([16.0, 8.0, 16.0]) {
            assert!((got - want).abs() < 1e-9, "{gaps4:?}");
        }
    }

    #[test]
    fn a_spread_of_100_percent_is_the_even_layout() {
        for count in 2..=7 {
            let even = centres(300.0, count, 6.0, &AUTO, 12.0);
            let blend = centres(300.0, count, 6.0, &centred(1.0, None), 12.0);
            for (a, b) in even.iter().zip(&blend) {
                assert!((a - b).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn the_blend_never_overlaps_and_keeps_the_margins() {
        let edge = Edge {
            length: 200.0,
            seam: false,
        };
        let joints = centred(0.0, None);
        for count in 2..=edge.most(6.0, 3.0, &joints) {
            for step in 0..=10 {
                let j = centred(f64::from(step) / 10.0, None);
                let gap = edge.gap(count, 6.0, 3.0, &j);
                let c = centres(200.0, count, 6.0, &j, gap);
                assert!(c[0] - 3.0 >= 3.0 - 1e-9 && c[c.len() - 1] + 3.0 <= 197.0 + 1e-9);
                assert!(c.windows(2).all(|w| w[1] - w[0] >= 6.0 - 1e-9), "{c:?}");
            }
        }
    }

    #[test]
    fn the_default_finger_joint_is_todays_equal_pieces() {
        for span in [20.0, 50.0, 120.0, 333.0] {
            let split = finger_split(span, 6.0, &AUTO);
            assert_eq!(split, Split::equal(span, 6.0));
            assert!(split.widths.is_empty() && split.segments % 2 == 1);
        }
    }

    #[test]
    fn a_centred_finger_joint_has_the_owner_at_both_ends_and_fills_the_span() {
        for span in [30.0, 50.0, 120.0] {
            for n in 1..=4 {
                let joints = Joints {
                    count: Some(n),
                    ..centred(0.0, None)
                };
                let split = finger_split(span, 6.0, &joints);
                let fingers = split.fingers();
                assert!(fingers >= 1 && fingers <= n, "{span} {n}: {fingers}");
                assert_eq!(split.widths.len(), split.segments as usize);
                assert!((split.widths.iter().sum::<f64>() - span).abs() < 1e-9);
                assert!(split.widths.iter().all(|w| *w >= 3.0 - 1e-9), "{split:?}");
                if fingers == 1 {
                    assert!((split.widths[0] - split.widths[2]).abs() < 1e-9, "centred");
                }
            }
        }
    }

    #[test]
    fn the_seam_edge_takes_an_even_count() {
        let edge = Edge {
            length: 100.0,
            seam: true,
        };
        assert_eq!(edge.count(6.0, 3.0, &AUTO) % 2, 0);
        let one = Joints {
            count: Some(1),
            ..centred(0.0, None)
        };
        assert_eq!(edge.count(6.0, 3.0, &one), 2);
    }

    #[test]
    fn a_count_too_high_is_lowered_to_what_fits() {
        let edge = Edge {
            length: 60.0,
            seam: false,
        };
        let asked = Joints {
            count: Some(50),
            ..AUTO
        };
        let fitted = fit(&[edge], 3.0, 6.0, asked);
        assert!(matches!(fitted.limits[0], Limit::CountMost(n) if (1..50).contains(&n)));
    }
}
