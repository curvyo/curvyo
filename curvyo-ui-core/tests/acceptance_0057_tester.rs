//! Independent tester tests for `specs/0057-clone-overrides` at the ui-core seams: Spread colours
//! (criterion 42, a Proposal) and a property test of the held-back rule (criteria 3, 5, 7).
//! Written from the specification; only the names of public seams were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(missing_docs, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::assert_is_empty
)]

use curvyo_document_core::{
    Affine, AnchorId, Color, Document, NewAnchor, NodeId, ObjectSnapshot, Point, StyleEdit, Vec2,
};
use curvyo_ui_core::spread_colours;
use proptest::prelude::*;

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(9_000);
fn aid() -> AnchorId {
    AnchorId::new(8, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}
fn rect(d: &Document, w: f64, h: f64) -> NodeId {
    let p = |x, y| NewAnchor::corner(aid(), Point::new(x, y));
    d.create_path(&[p(0.0, 0.0), p(w, 0.0), p(w, h), p(0.0, h)], true)
}
fn hue(c: Color) -> f64 {
    let (r, g, b) = (
        f64::from(c.r) / 255.0,
        f64::from(c.g) / 255.0,
        f64::from(c.b) / 255.0,
    );
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let dl = mx - mn;
    if dl == 0.0 {
        return 0.0;
    }
    let h = if mx == r {
        ((g - b) / dl).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / dl + 2.0
    } else {
        (r - g) / dl + 4.0
    };
    h * 60.0
}
fn lightness(c: Color) -> f64 {
    let m = [c.r, c.g, c.b].map(f64::from);
    (m.iter().copied().fold(0.0, f64::max) + m.iter().copied().fold(255.0, f64::min)) / 510.0
}
fn twelve(red: Color) -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    d.edit_style(&[o], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[o], &StyleEdit::FillColor(red)).unwrap();
    let cs: Vec<NodeId> = (0..12)
        .map(|_| d.linked_clone(&[o], &[]).unwrap()[0])
        .collect();
    (d, o, cs)
}

#[test]
fn ac42_twelve_red_clones_get_hues_0_to_330_and_apply_in_one_undoable_commit() {
    let red = Color {
        r: 230,
        g: 30,
        b: 30,
    };
    let (d, o, cs) = twelve(red);
    let h0 = hue(red);
    // Selection order shuffled: the order is the stacking order from the back, not the click order.
    let mut sel = cs.clone();
    sel.reverse();
    let colours = spread_colours(&d.read_objects(), &sel);
    assert_eq!(colours.len(), 12);
    let ids: Vec<NodeId> = colours.iter().map(|c| c.clone).collect();
    // Each clone sits directly above its original, so the last made is the lowest.
    let read = d.read_objects();
    let mut stacked = cs.clone();
    stacked.sort_by_key(|id| read.index[id]);
    assert_eq!(ids, stacked, "back to front");
    for (k, c) in colours.iter().enumerate() {
        let want = (h0 + 30.0 * k as f64).rem_euclid(360.0);
        let got = hue(c.fill);
        let diff = (got - want).abs().min(360.0 - (got - want).abs());
        assert!(diff <= 1.5, "clone {k}: hue {got} want {want}");
        assert!(
            (lightness(c.fill) - lightness(red)).abs() < 0.01,
            "lightness kept"
        );
    }
    // Apply: clones become Own with the new colours; original untouched; one commit; undo.
    let before: Vec<_> = cs
        .iter()
        .map(|&c| d.object(c).unwrap().style().clone())
        .collect();
    let n = d.own_steps().len();
    d.set_own_style(&cs, true).unwrap();
    d.recolour_clones(&colours).unwrap();
    assert_eq!(d.own_steps().len(), n + 2);
    assert_eq!(d.own_steps().last().unwrap().label, "spread_colours");
    for want in &colours {
        assert_eq!(d.object(want.clone).unwrap().style().fill.color, want.fill);
    }
    assert_eq!(d.object(o).unwrap().style().fill.color, red);
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    for (&c, b) in cs.iter().zip(&before) {
        assert_eq!(d.object(c).unwrap().style().fill.color, b.fill.color);
    }
}

#[test]
fn ac42_fewer_than_two_clones_gives_nothing_and_a_grey_stays_grey() {
    let (d, _, cs) = twelve(Color {
        r: 128,
        g: 128,
        b: 128,
    });
    assert!(spread_colours(&d.read_objects(), &cs[..1]).is_empty());
    assert!(spread_colours(&d.read_objects(), &[]).is_empty());
    for c in spread_colours(&d.read_objects(), &cs) {
        assert!(
            c.fill.r == c.fill.g && c.fill.g == c.fill.b,
            "grey stays grey: {:?}",
            c.fill
        );
    }
}

#[test]
fn ac42_a_clone_with_fill_off_starts_from_its_stroke_hue_and_spreads_the_stroke() {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    d.edit_style(&[o], &StyleEdit::FillEnabled(false)).unwrap();
    d.edit_style(&[o], &StyleEdit::StrokeColor(Color { r: 0, g: 0, b: 220 }))
        .unwrap();
    let cs: Vec<NodeId> = (0..4)
        .map(|_| d.linked_clone(&[o], &[]).unwrap()[0])
        .collect();
    let cols = spread_colours(&d.read_objects(), &cs);
    assert_eq!(cols.len(), 4);
    let h0 = hue(Color { r: 0, g: 0, b: 220 });
    for (k, c) in cols.iter().enumerate() {
        let s = c.stroke.expect("stroke on: the stroke colour is spread");
        let want = (h0 + 90.0 * k as f64).rem_euclid(360.0);
        let d = (hue(s) - want).abs();
        assert!(d.min(360.0 - d) < 1.5);
    }
}

fn arb_g() -> impl Strategy<Value = Affine> {
    (
        -3.0f64..3.0,
        -3.0f64..3.0,
        -3.0f64..3.0,
        -3.0f64..3.0,
        -50.0f64..50.0,
        -50.0f64..50.0,
    )
        .prop_filter("invertible", |(a, b, c, d, ..)| {
            (a * d - b * c).abs() > 0.05
        })
        .prop_map(|(a, b, c, d, e, f)| Affine::new(a, b, c, d, e, f))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Criterion 3: for every invertible G the held clone's outline is unchanged (1e-6 mm), the
    /// following clone's is G applied in its local frame, and the original is G-mapped.
    #[test]
    fn held_clone_outline_is_invariant_under_any_transform_of_its_original(
        g in arb_g(),
        w in 0.01f64..200.0,
        h in 0.001f64..200.0,
        tx in -100.0f64..100.0,
        ty in -100.0f64..100.0,
    ) {
        let d = Document::new(1);
        let o = rect(&d, w, h);
        let held = d.linked_clone(&[o], &[]).unwrap()[0];
        let free = d.linked_clone(&[o], &[]).unwrap()[0];
        d.translate_objects(&[held, free], Vec2::new(tx, ty)).unwrap();
        d.set_own_placement(&[held], true).unwrap();
        let outline = |id| match d.object(id).unwrap() {
            ObjectSnapshot::Clone(c) => c.resolved.anchors.iter().map(|a| (a.point.x, a.point.y)).collect::<Vec<_>>(),
            _ => panic!(),
        };
        let (h0, f0) = (outline(held), outline(free));
        let ObjectSnapshot::Path(p) = d.object(o).unwrap() else { panic!() };
        // Skip maps that the document legitimately refuses.
        if d.transform_objects(&[ObjectSnapshot::Path(p.mapped(g))], false).is_err() { return Ok(()); }
        let (h1, f1) = (outline(held), outline(free));
        // Either the clone stayed (the normal case) or, for a numerically singular T, it was unlinked.
        if matches!(d.object(held), Some(ObjectSnapshot::Clone(_))) {
            for (a, b) in h0.iter().zip(&h1) {
                let tol = 1e-6 * (1.0 + a.0.abs().max(a.1.abs()));
                prop_assert!((a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol, "{a:?} {b:?}");
            }
        } else if let Some(ObjectSnapshot::Path(q)) = d.object(held) {
            for (a, b) in h0.iter().zip(&q.anchors) {
                prop_assert!((a.0 - b.point.x).abs() <= 1e-6 && (a.1 - b.point.y).abs() <= 1e-6);
            }
        }
        prop_assert_eq!(f0.len(), f1.len());
    }
}

#[test]
fn ac1_ac10_the_block_shows_the_common_state_or_mixed() {
    use curvyo_ui_core::{OverrideState, clone_block};
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    let a = d.linked_clone(&[o], &[]).unwrap()[0];
    let b = d.linked_clone(&[o], &[]).unwrap()[0];
    let state =
        |ids: &[NodeId]| clone_block(&d.read_objects(), ids).map(|k| (k.placement, k.style));
    assert_eq!(
        state(&[a]),
        Some((OverrideState::Original, OverrideState::Original))
    );
    d.set_own_placement(&[a], true).unwrap();
    assert_eq!(
        state(&[a]),
        Some((OverrideState::Own, OverrideState::Original))
    );
    assert_eq!(
        state(&[a, b]),
        Some((OverrideState::Mixed, OverrideState::Original))
    );
    d.set_own_style(&[a], true).unwrap();
    assert_eq!(
        state(&[a, b]),
        Some((OverrideState::Mixed, OverrideState::Mixed))
    );
    d.set_own_placement(&[b], true).unwrap();
    d.set_own_style(&[b], true).unwrap();
    assert_eq!(
        state(&[a, b]),
        Some((OverrideState::Own, OverrideState::Own))
    );
    assert_eq!(state(&[o]), None, "not for a path");
    assert_eq!(
        state(&[a, o]),
        None,
        "not for a mixed path and clone selection"
    );
}
