//! The slit pattern of a bending wall as pure geometry: slots across a flat strip, parallel to
//! the bend axis, that run out through the edges so no uncut band crosses a bend (ADR 0082 §6).
//!
//! The strip is developed flat: `x` along it, `y` from the top edge (0) to the bottom edge
//! (`height`). Millimetres.

use curvyo_document_core::Length;

/// A bend needs at least two columns, else a band at the bridge height stays uncut.
const MIN_COLUMNS: usize = 2;

/// Which pattern the columns follow. An open set: a new kind is one arm of `slots`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlitKind {
    /// One slot per column, running out through alternately the top and the bottom edge.
    Straight,
    /// Columns of `rows` slots with bridges, every other column shifted by half a period.
    Offset,
    /// The Straight layout with each slot a chain of tangent arcs (`wave_slot.rs`).
    Wave,
}

/// The pattern's measures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SlitPattern {
    pub(crate) kind: SlitKind,
    /// The slot's width across the column (before the kerf).
    pub(crate) width: Length,
    /// The distance between two columns.
    pub(crate) spacing: Length,
    /// The material left between a slot and an edge or another slot.
    pub(crate) bridge: Length,
    /// The slots of an even column of the Offset pattern.
    pub(crate) rows: u32,
}

/// A flat stretch of the strip that bends: `from..to` along it, `height` across.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Band {
    pub(crate) from: Length,
    pub(crate) to: Length,
    pub(crate) height: Length,
}

/// A stretch along the strip that holds no column (a tab slot and its bridges).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Window {
    pub(crate) from: Length,
    pub(crate) to: Length,
}

/// One slot: a rectangle `width` wide centred on `x`, from `y0` to `y1`. It runs out through the
/// top edge when `y0` is 0 and through the bottom edge when `y1` is the band's height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Slot {
    pub(crate) x: f64,
    pub(crate) y0: f64,
    pub(crate) y1: f64,
    pub(crate) width: f64,
    /// For a wave slot: the length of one half wave along the slot.
    pub(crate) wave: Option<f64>,
}

/// The slots of `band`, none within `pattern.bridge` of a window.
pub(crate) fn slots(band: &Band, pattern: &SlitPattern, windows: &[Window]) -> Vec<Slot> {
    let (from, to) = (band.from.as_mm(), band.to.as_mm());
    let spacing = pattern.spacing.as_mm();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = (((to - from) / spacing).round() as usize).max(MIN_COLUMNS);
    #[allow(clippy::cast_precision_loss)]
    let pitch = (to - from) / columns as f64;
    let mut slots = Vec::new();
    for column in 0..columns {
        #[allow(clippy::cast_precision_loss)]
        let x = from + pitch * (column as f64 + 0.5);
        let swing = if pattern.kind == SlitKind::Wave {
            spacing / 8.0
        } else {
            0.0
        };
        let half = pattern.width.as_mm() / 2.0 + pattern.bridge.as_mm() + swing;
        if windows
            .iter()
            .any(|w| x + half > w.from.as_mm() && x - half < w.to.as_mm())
        {
            continue;
        }
        match pattern.kind {
            SlitKind::Straight => slots.push(straight(column, x, band, pattern)),
            SlitKind::Wave => {
                let mut slot = straight(column, x, band, pattern);
                slot.wave = Some(spacing);
                slots.push(slot);
            }
            SlitKind::Offset => slots.extend(offset(column, x, band, pattern)),
        }
    }
    slots
}

fn straight(column: usize, x: f64, band: &Band, pattern: &SlitPattern) -> Slot {
    let (height, bridge) = (band.height.as_mm(), pattern.bridge.as_mm());
    let (y0, y1) = if column.is_multiple_of(2) {
        (0.0, height - bridge)
    } else {
        (bridge, height)
    };
    Slot {
        x,
        y0,
        y1,
        width: pattern.width.as_mm(),
        wave: None,
    }
}

fn offset(column: usize, x: f64, band: &Band, pattern: &SlitPattern) -> Vec<Slot> {
    let (height, bridge) = (band.height.as_mm(), pattern.bridge.as_mm());
    let rows = f64::from(pattern.rows.max(1));
    // The slot length of an even column: `rows` slots and `rows + 1` bridges fill the height.
    let length = (height - (rows + 1.0) * bridge) / rows;
    let slot = |y0: f64, y1: f64| Slot {
        x,
        y0,
        y1,
        width: pattern.width.as_mm(),
        wave: None,
    };
    let mut out = Vec::new();
    if column.is_multiple_of(2) {
        for row in 0..pattern.rows.max(1) {
            let y0 = bridge + f64::from(row) * (length + bridge);
            out.push(slot(y0, y0 + length));
        }
    } else {
        // The bridges sit at the middle of the even column's slots; the end slots run out.
        let mut start = 0.0;
        for row in 0..pattern.rows.max(1) {
            let centre = bridge + f64::from(row) * (length + bridge) + length / 2.0;
            out.push(slot(start, centre - bridge / 2.0));
            start = centre + bridge / 2.0;
        }
        out.push(slot(start, height));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(value: f64) -> Length {
        Length::from_mm(value)
    }

    fn pattern(kind: SlitKind) -> SlitPattern {
        SlitPattern {
            kind,
            width: mm(0.3),
            spacing: mm(3.3),
            bridge: mm(3.0),
            rows: 2,
        }
    }

    fn band() -> Band {
        Band {
            from: mm(100.0),
            to: mm(125.0),
            height: mm(56.0),
        }
    }

    fn assert_no_uncut_band(slots: &[Slot], height: f64) {
        let mut y = 0.0;
        while y <= height + 1e-9 {
            assert!(
                slots.iter().any(|s| s.y0 - 1e-9 <= y && y <= s.y1 + 1e-9),
                "an uncut band at {y} mm"
            );
            y += 0.05;
        }
    }

    #[test]
    fn straight_slots_alternate_out_through_both_edges_and_leave_no_uncut_band() {
        let slots = slots(&band(), &pattern(SlitKind::Straight), &[]);
        assert_eq!(slots.len(), 8, "25 / 3.3 rounds to 8 columns");
        assert_eq!(slots[0].y0, 0.0);
        assert_eq!(slots[0].y1, 53.0);
        assert_eq!(slots[1].y0, 3.0);
        assert_eq!(slots[1].y1, 56.0);
        assert_no_uncut_band(&slots, 56.0);
        let pitch = 25.0 / 8.0;
        assert!(
            (slots[0].x - (100.0 + pitch / 2.0)).abs() < 1e-9,
            "centred columns"
        );
    }

    #[test]
    fn offset_columns_hold_rows_slots_and_the_odd_ones_run_out_through_both_edges() {
        let slots = slots(&band(), &pattern(SlitKind::Offset), &[]);
        assert_no_uncut_band(&slots, 56.0);
        let first: Vec<&Slot> = slots
            .iter()
            .filter(|s| (s.x - slots[0].x).abs() < 1e-9)
            .collect();
        assert_eq!(first.len(), 2, "an even column holds `rows` slots");
        assert!(
            first
                .iter()
                .all(|s| s.y0 >= 3.0 - 1e-9 && s.y1 <= 53.0 + 1e-9)
        );
        let second_x = slots
            .iter()
            .map(|s| s.x)
            .find(|x| *x > slots[0].x + 1e-9)
            .unwrap();
        let second: Vec<&Slot> = slots
            .iter()
            .filter(|s| (s.x - second_x).abs() < 1e-9)
            .collect();
        assert_eq!(second.len(), 3, "an odd column holds rows + 1 slots");
        assert_eq!(second[0].y0, 0.0);
        assert_eq!(second[2].y1, 56.0);
        // Bridges between slots of one column are one wall thickness.
        for pair in first.windows(2) {
            assert!((pair[1].y0 - pair[0].y1 - 3.0).abs() < 1e-9);
        }
    }

    #[test]
    fn no_column_comes_within_a_bridge_of_a_window() {
        let window = Window {
            from: mm(110.0),
            to: mm(116.0),
        };
        let slots = slots(&band(), &pattern(SlitKind::Straight), &[window]);
        for slot in &slots {
            let distance =
                (window.from.as_mm() - slot.x - 0.15).max(slot.x - 0.15 - window.to.as_mm());
            assert!(
                distance >= 3.0 - 1e-9,
                "slot at {} is {distance} from the window",
                slot.x
            );
        }
    }

    #[test]
    fn a_bend_shorter_than_one_spacing_still_gets_two_columns() {
        let short = Band {
            from: mm(0.0),
            to: mm(2.0),
            height: mm(56.0),
        };
        let slots = slots(&short, &pattern(SlitKind::Straight), &[]);
        assert_eq!(slots.len(), 2);
        assert_no_uncut_band(&slots, 56.0);
    }
}
