//! Spread colours (`specs/0057-clone-overrides` criterion 42, Proposal): evenly spaced hues for
//! the selected clones in stacking order, each colour keeping its saturation and lightness.

use curvyo_document_core::{CloneColours, Color, NodeId, ObjectRead, ObjectSnapshot};

/// The colours Spread colours gives the selected clones of `selected`, back to front: the first
/// keeps its hue (that of its fill colour, or of its stroke colour when the fill is off), the
/// k-th of n is turned by k·360°/n; the fill colour always, the stroke colour where the stroke is
/// on. Empty with fewer than two selected clones (the button is disabled then).
#[must_use]
pub fn spread_colours(read: &ObjectRead, selected: &[NodeId]) -> Vec<CloneColours> {
    let mut clones: Vec<(usize, &curvyo_document_core::CloneSnapshot)> = selected
        .iter()
        .filter_map(|id| match read.get(*id) {
            Some(ObjectSnapshot::Clone(clone)) => Some((read.index[id], clone)),
            _ => None,
        })
        .collect();
    clones.sort_unstable_by_key(|(at, _)| *at);
    clones.dedup_by_key(|(at, _)| *at);
    if clones.len() < 2 {
        return Vec::new();
    }
    let first = &clones[0].1.resolved.style;
    let start = if first.fill.enabled {
        hue_of(first.fill.color)
    } else {
        hue_of(first.stroke.color)
    };
    #[allow(clippy::cast_precision_loss)] // a selection count far below 2^52
    let step = 360.0 / clones.len() as f64;
    clones
        .iter()
        .enumerate()
        .map(|(k, (_, clone))| {
            #[allow(clippy::cast_precision_loss)]
            let hue = (start + step * k as f64).rem_euclid(360.0);
            let style = &clone.resolved.style;
            CloneColours {
                clone: clone.id(),
                fill: with_hue(style.fill.color, hue),
                stroke: style
                    .stroke
                    .enabled
                    .then(|| with_hue(style.stroke.color, hue)),
            }
        })
        .collect()
}

/// Hue (degrees), saturation and lightness (0..=1) of an RGB colour.
fn hsl(color: Color) -> (f64, f64, f64) {
    let [r, g, b] = [color.r, color.g, color.b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = f64::midpoint(max, min);
    let delta = max - min;
    if delta <= f64::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - 2.0f64.mul_add(lightness, -1.0).abs());
    #[allow(clippy::float_cmp)] // `max` is one of the three
    let hue = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    } * 60.0;
    (hue, saturation, lightness)
}

fn hue_of(color: Color) -> f64 {
    hsl(color).0
}

/// `color` with its hue set to `hue`, its saturation and lightness kept (a grey stays grey).
#[allow(clippy::many_single_char_names)] // the channel names of the HSL formula
fn with_hue(color: Color, hue: f64) -> Color {
    let (_, saturation, lightness) = hsl(color);
    let chroma = (1.0 - 2.0f64.mul_add(lightness, -1.0).abs()) * saturation;
    let h = hue / 60.0;
    let x = chroma * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h {
        h if h < 1.0 => (chroma, x, 0.0),
        h if h < 2.0 => (x, chroma, 0.0),
        h if h < 3.0 => (0.0, chroma, x),
        h if h < 4.0 => (0.0, x, chroma),
        h if h < 5.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    // invariant: each channel lies in 0..=1 by construction, so the rounded byte fits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color {
        r: byte(r),
        g: byte(g),
        b: byte(b),
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, StyleEdit};

    use super::*;

    fn twelve_red_clones() -> (Document, Vec<NodeId>) {
        let d = Document::new(1);
        let corner =
            |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
        let p = d.create_path(
            &[
                corner(0, 0.0, 0.0),
                corner(1, 10.0, 0.0),
                corner(2, 0.0, 10.0),
            ],
            true,
        );
        d.edit_style(&[p], &StyleEdit::FillEnabled(true)).unwrap();
        d.edit_style(&[p], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
            .unwrap();
        let clones = (0..12)
            .map(|_| d.linked_clone(&[p], &[]).unwrap()[0])
            .collect();
        (d, clones)
    }

    #[test]
    fn criterion_42_twelve_red_clones_get_hues_in_steps_of_thirty() {
        let (d, clones) = twelve_red_clones();
        let read = d.read_objects();
        // Each new clone sits directly above the root, so the back-most is the last made.
        let back_to_front: Vec<NodeId> = clones.iter().rev().copied().collect();
        let colours = spread_colours(&read, &clones);
        assert_eq!(colours.len(), 12);
        for (k, colour) in colours.iter().enumerate() {
            assert_eq!(
                colour.clone, back_to_front[k],
                "stacking order, back to front"
            );
            #[allow(clippy::cast_precision_loss)]
            let wanted = 30.0 * k as f64;
            assert!(
                (hue_of(colour.fill) - wanted).abs() < 1.5,
                "{k}: {:?}",
                colour.fill
            );
            let (_, s, l) = hsl(colour.fill);
            assert!((s - 1.0).abs() < 0.01 && (l - 0.5).abs() < 0.01);
            assert!(colour.stroke.is_some(), "the default stroke is on");
        }
        assert_eq!(colours[0].fill, Color { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn a_grey_stays_grey_and_one_clone_gets_nothing() {
        let (d, clones) = twelve_red_clones();
        let colours = spread_colours(&d.read_objects(), &clones[..2]);
        assert_eq!(
            colours[1].stroke,
            Some(Color::BLACK),
            "black keeps its lightness"
        );
        assert_eq!(spread_colours(&d.read_objects(), &clones[..1]), Vec::new());
    }
}
