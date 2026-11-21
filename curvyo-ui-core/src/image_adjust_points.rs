//! The writes of the Curves graph's points and the Channel preset cells (`specs/0135` criterion
//! 5, `ux-notes.md` §3.8): a point added, removed or moved, a Channel preset picked, each one
//! step "Edit adjustments".

use curvyo_document_core::AdjustEntry;
use curvyo_laser_core::adjust::{MAX_CURVE_POINTS, ManualStep};

/// The Curves row `index` as read: its points and whether it is switched on.
fn curve_row(entries: &[AdjustEntry], index: usize) -> Option<(Vec<(u8, u8)>, bool)> {
    let step = entries.get(index)?.step()?;
    let read = ManualStep::read(&step.kind, |key| step.fields.get(key).copied()).ok()?;
    let ManualStep::Curves { points, count } = read else {
        return None;
    };
    Some((points[..usize::from(count)].to_vec(), step.enabled))
}

/// The stack with Curves row `index` holding `points`, or `None` when they are no curve.
fn with_curve(
    entries: &[AdjustEntry],
    index: usize,
    points: &[(u8, u8)],
    enabled: bool,
) -> Option<Vec<AdjustEntry>> {
    let step = ManualStep::curve(points)?;
    let mut next = entries.to_vec();
    next[index] = step.entry(enabled);
    Some(next)
}

/// A click on the curve of row `index` at (`input`, `output`) in % (criterion 5: at most 8
/// points): a point there, in Input order; `None` when the row holds 8 already, a point has that
/// Input, or the row is no Curves row.
#[must_use]
pub fn add_curve_point(
    entries: &[AdjustEntry],
    index: usize,
    (input, output): (u8, u8),
) -> Option<Vec<AdjustEntry>> {
    let (mut points, enabled) = curve_row(entries, index)?;
    if points.len() >= MAX_CURVE_POINTS || points.iter().any(|p| p.0 == input) {
        return None;
    }
    let at = points.iter().position(|p| p.0 > input)?;
    points.insert(at, (input, output.min(100)));
    with_curve(entries, index, &points, enabled)
}

/// Delete on point `point` of Curves row `index`; `None` for an end point, which stays.
#[must_use]
pub fn remove_curve_point(
    entries: &[AdjustEntry],
    index: usize,
    point: usize,
) -> Option<Vec<AdjustEntry>> {
    let (mut points, enabled) = curve_row(entries, index)?;
    if point == 0 || point + 1 >= points.len() {
        return None;
    }
    points.remove(point);
    with_curve(entries, index, &points, enabled)
}

/// A drag of point `point` of Curves row `index` to (`input`, `output`) in %: the end points
/// move only vertically, the others stay between their neighbours (one Input apart); `None`
/// when nothing changes.
#[must_use]
pub fn move_curve_point(
    entries: &[AdjustEntry],
    index: usize,
    point: usize,
    (input, output): (u8, u8),
) -> Option<Vec<AdjustEntry>> {
    let (mut points, enabled) = curve_row(entries, index)?;
    let last = points.len().checked_sub(1)?;
    let old = *points.get(point)?;
    let input = if point == 0 || point == last {
        old.0
    } else {
        input.clamp(points[point - 1].0 + 1, points[point + 1].0 - 1)
    };
    let moved = (input, output.min(100));
    if moved == old {
        return None;
    }
    points[point] = moved;
    with_curve(entries, index, &points, enabled)
}

/// The Straight curve button of Curves row `index`: the two end points only; `None` when it is
/// straight already.
#[must_use]
pub fn straighten_curve(entries: &[AdjustEntry], index: usize) -> Option<Vec<AdjustEntry>> {
    let (points, enabled) = curve_row(entries, index)?;
    let straight = [(0, 0), (100, 100)];
    (points != straight).then(|| with_curve(entries, index, &straight, enabled))?
}

/// A Channel preset cell of Channel mix row `index`: the three weights `weights`; `None` when
/// the row holds them already or is no Channel mix row.
#[must_use]
pub fn set_channel_mix(
    entries: &[AdjustEntry],
    index: usize,
    weights: [i16; 3],
) -> Option<Vec<AdjustEntry>> {
    let step = entries.get(index)?.step()?;
    let read = ManualStep::read(&step.kind, |key| step.fields.get(key).copied()).ok()?;
    let ManualStep::ChannelMix { weights: old } = read else {
        return None;
    };
    if old == weights {
        return None;
    }
    let mut next = entries.to_vec();
    next[index] = ManualStep::ChannelMix { weights }.entry(step.enabled);
    Some(next)
}

#[cfg(test)]
mod tests {
    use curvyo_laser_core::adjust::{CHANNEL_PRESETS, StepKind};

    use super::*;
    use crate::image_adjust_edit::add_step;

    fn points(entries: &[AdjustEntry]) -> Vec<(u8, u8)> {
        curve_row(entries, 0).unwrap().0
    }

    #[test]
    fn a_click_adds_a_point_in_order_up_to_eight() {
        let mut stack = add_step(&[], StepKind::Curves, 100);
        assert_eq!(points(&stack), [(0, 0), (100, 100)]);
        stack = add_curve_point(&stack, 0, (60, 70)).unwrap();
        stack = add_curve_point(&stack, 0, (30, 20)).unwrap();
        assert_eq!(points(&stack), [(0, 0), (30, 20), (60, 70), (100, 100)]);
        assert!(
            add_curve_point(&stack, 0, (30, 50)).is_none(),
            "one point per Input"
        );
        for x in [10, 20, 40, 50] {
            stack = add_curve_point(&stack, 0, (x, x)).unwrap();
        }
        assert_eq!(points(&stack).len(), 8);
        assert!(add_curve_point(&stack, 0, (90, 90)).is_none(), "at most 8");
    }

    #[test]
    fn end_points_stay_and_inner_points_stay_between_their_neighbours() {
        let stack = add_curve_point(&add_step(&[], StepKind::Curves, 100), 0, (50, 50)).unwrap();
        assert!(remove_curve_point(&stack, 0, 0).is_none());
        assert!(remove_curve_point(&stack, 0, 2).is_none());
        let moved = move_curve_point(&stack, 0, 0, (40, 10)).unwrap();
        assert_eq!(points(&moved)[0], (0, 10), "the end moves only vertically");
        let moved = move_curve_point(&stack, 0, 1, (100, 80)).unwrap();
        assert_eq!(points(&moved)[1], (99, 80), "stays below the next Input");
        let fewer = remove_curve_point(&stack, 0, 1).unwrap();
        assert_eq!(points(&fewer), [(0, 0), (100, 100)]);
        assert!(straighten_curve(&fewer, 0).is_none());
        assert_eq!(
            points(&straighten_curve(&moved, 0).unwrap()),
            [(0, 0), (100, 100)]
        );
    }

    #[test]
    fn a_channel_preset_sets_the_three_weights() {
        let stack = add_step(&[], StepKind::ChannelMix, 100);
        let red = CHANNEL_PRESETS[1].1;
        let next = set_channel_mix(&stack, 0, red).unwrap();
        assert_eq!(
            next[0].step().unwrap().fields["red_pct"],
            100,
            "{:?}",
            next[0]
        );
        assert!(set_channel_mix(&next, 0, red).is_none());
        assert!(set_channel_mix(&add_step(&[], StepKind::Levels, 100), 0, red).is_none());
    }
}
