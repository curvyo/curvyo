//! The costs of tiling (`specs/0069-tiling` criterion 40): a source of one path with 50 segments
//! tiled 100 x 100 with every variance channel on: the layout and preview build after one option
//! change (target 50 ms), an Apply that creates 10,000 instances and a seed change on 10,000
//! existing instances (target 500 ms each). Thread CPU time (`cpu_time`). The preview budget is
//! asserted in release builds only; the two Apply targets are Proposals the customer did not set,
//! so they are printed and tracked in `docs/technical-debt.md`, not gated (`CLAUDE.md` §3,
//! criterion 40). A debug build prints every number.
//! A frame of a drag of a tiling's source (criterion 29, live during the gesture, customer
//! 2026-11-02), a scale with the Select handles and a node drag with the Node tool, for a 10 x 10
//! and a 100 x 100 tiling: no existing budget covers it, so the numbers are printed against a
//! 16 ms target (Proposal) and not gated (`CLAUDE.md` §3).
//! Run: `cargo test --release -p curvyo-editor-wasm --test tiling_budget -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;

use curvyo_document_core::{
    AnchorId, Angle, Document, Length, NewAnchor, NodeId, Percent, Point, TilingRecipe, pack,
};
use curvyo_editor_wasm::{KeyInput, RowPress, Session, Tool};

const PREVIEW_BUDGET: Duration = Duration::from_millis(50);
const APPLY_TARGET: Duration = Duration::from_millis(500);
const DRAG_FRAME_TARGET: Duration = Duration::from_millis(16);

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

/// A Proposal target: printed with the measured value, never asserted.
fn report(label: &str, took: Duration, target: Duration) {
    println!("{label}: {took:?} (target {target:?}, reported, not gated)");
}

fn session() -> Session {
    session_with_source().0
}

/// A session with the source selected: a 50-segment polygon of radius 8 about (10, 10).
fn session_with_source() -> (Session, NodeId) {
    let document = Document::new(1);
    let anchors: Vec<NewAnchor> = (0..50_u32)
        .map(|n| {
            let a = f64::from(n) / 50.0 * std::f64::consts::TAU;
            NewAnchor::corner(
                AnchorId::new(1, u64::from(n)),
                Point::new(10.0 + 8.0 * a.cos(), 10.0 + 8.0 * a.sin()),
            )
        })
        .collect();
    let source = document.create_path(&anchors, true);
    let mut session = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    (session, source)
}

fn hundred_by_hundred(mut recipe: TilingRecipe) -> TilingRecipe {
    recipe.grid.columns = 100;
    recipe.grid.rows = 100;
    let v = &mut recipe.variance;
    v.rotation.random = Angle::from_radians(0.2);
    v.scale_x.random = Percent::new(10.0);
    v.offset_x.random = Length::from_mm(1.0);
    v.offset_y.random = Length::from_mm(1.0);
    v.opacity.random = Percent::new(20.0);
    v.hue.random = Angle::from_radians(0.5);
    recipe
}

#[test]
fn ac40_preview_apply_and_seed_change_of_ten_thousand() {
    let mut session = session();
    assert!(session.tiling_enter(42));
    let recipe = hundred_by_hundred(session.tiling_recipe().unwrap());
    assert!(session.tiling_set_recipe(recipe));
    let _ = session.draw_list();
    let mut changed = recipe;
    changed.grid.spacing_x = Length::from_mm(6.0);
    let ((), took) = cpu_time_of(|| {
        assert!(session.tiling_set_recipe(changed));
        let _ = session.draw_list();
    });
    check("Layout and preview of 10,000", took, PREVIEW_BUDGET);
    let (applied, took) = cpu_time_of(|| session.tiling_apply());
    assert!(applied);
    report("Apply creating 10,000", took, APPLY_TARGET);
    assert!(session.tiling_enter(1));
    let mut reseeded = session.tiling_recipe().unwrap();
    reseeded.seed = 4242;
    assert!(session.tiling_set_recipe(reseeded));
    let (applied, took) = cpu_time_of(|| session.tiling_apply());
    assert!(applied);
    report("Seed change on 10,000", took, APPLY_TARGET);
}

fn hold(s: &mut Session, at: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(at, false, false);
}

/// A session with the source tiled `n` x `n` as clones (Keep source in place on), the tiling bar
/// closed and the source selected.
fn tiled(n: u32) -> (Session, NodeId) {
    let (mut s, source) = session_with_source();
    assert!(s.tiling_enter(7));
    let mut recipe = s.tiling_recipe().unwrap();
    recipe.grid.columns = n;
    recipe.grid.rows = n;
    assert!(s.tiling_set_recipe(recipe));
    assert!(s.tiling_apply());
    let _ = s.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
    s.press_layer_row(&serde_json::to_string(&source).unwrap(), RowPress::Plain);
    (s, source)
}

/// The CPU time of one frame in the middle of a drag from `a` to `b`: the pointer move and the
/// draw list. The frame before it is drawn first, so the drag's one-time reads are not counted.
fn drag_frame(s: &mut Session, a: Point, b: Point) -> Duration {
    hold(s, a);
    s.pointer_down(a, false);
    hold(
        s,
        Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
    );
    let _ = s.draw_list();
    let ((), took) = cpu_time_of(|| {
        hold(s, b);
        let _ = s.draw_list();
    });
    s.pointer_up(b, false, false);
    took
}

#[test]
fn ac29_a_frame_of_a_source_drag_with_a_live_tiling() {
    for n in [10, 100] {
        let (mut s, _) = tiled(n);
        // The bottom-right scale handle of the source's box (2..18 x 2..18).
        let took = drag_frame(&mut s, Point::new(18.0, 18.0), Point::new(24.0, 22.0));
        report(
            &format!("Scale frame, {n} x {n} tiling"),
            took,
            DRAG_FRAME_TARGET,
        );
        let (mut s, _) = tiled(n);
        s.set_tool(Tool::Node);
        let took = drag_frame(&mut s, Point::new(18.0, 10.0), Point::new(21.0, 13.0));
        report(
            &format!("Node drag frame, {n} x {n} tiling"),
            took,
            DRAG_FRAME_TARGET,
        );
    }
}
