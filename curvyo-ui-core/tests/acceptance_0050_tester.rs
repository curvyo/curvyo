//! Independent tester tests for `specs/0050-layers-and-objects-panel` in the interaction layer:
//! the panel strip, the words, and the restacking plans checked against a reference model with a
//! fixed-seed generator. Written from the specification before the implementation was read.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::too_many_lines)]

use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_ui_core::{
    PanelTab, PanelTabs, Reorder, deleted_layer_notice, moved_announcement, objects_count_text,
    reorder_plans, tab_entries,
};

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

#[test]
fn the_strip_has_layers_fourth_and_never_dimmed() {
    for active in [
        PanelTab::File,
        PanelTab::Document,
        PanelTab::Style,
        PanelTab::Layers,
        PanelTab::History,
    ] {
        for non_empty in [false, true] {
            let entries = tab_entries(active, non_empty);
            let names: Vec<_> = entries.iter().map(|e| e.name).collect();
            assert_eq!(
                names,
                ["file", "document", "style", "layers", "history", "fonts"]
            );
            let l = &entries[3];
            assert_eq!((l.label, l.tooltip), ("Layers", "Layers (Shift+Ctrl+L)"));
            assert!(l.enabled);
            assert_eq!(l.selected, active == PanelTab::Layers);
        }
    }
    assert_eq!(PanelTab::from_name("layers"), Some(PanelTab::Layers));
    assert_eq!(PanelTab::from_name("Layers"), None);
}

#[test]
fn layers_is_sticky_like_history_whatever_the_selection_does() {
    let mut tabs = PanelTabs::new(false);
    tabs.shortcut_layers();
    for non_empty in [true, false, true, true, false] {
        tabs.observe(non_empty);
        assert_eq!(tabs.active(), PanelTab::Layers);
    }
}

#[test]
fn the_words_of_the_spec() {
    assert_eq!(objects_count_text(40), "40 objects");
    assert_eq!(objects_count_text(1), "1 object");
    assert_eq!(
        deleted_layer_notice("Cut", 12),
        "Deleted layer Cut and 12 objects."
    );
    assert!(!deleted_layer_notice("Cut", 12).contains("No undo yet."));
    assert_eq!(
        moved_announcement("Rectangle 3f9a1c", 2, 5),
        "Moved Rectangle 3f9a1c to position 2 of 5"
    );
}

/// The order after `how` on the reference model: `order` is back to front, `picked` the moved set.
fn model(order: &[usize], picked: &[usize], how: Reorder) -> Vec<usize> {
    let sel: Vec<usize> = order
        .iter()
        .copied()
        .filter(|i| picked.contains(i))
        .collect();
    let rest: Vec<usize> = order
        .iter()
        .copied()
        .filter(|i| !picked.contains(i))
        .collect();
    match how {
        Reorder::Front => rest.into_iter().chain(sel).collect(),
        Reorder::Back => sel.into_iter().chain(rest).collect(),
        Reorder::Forward => {
            let mut out = order.to_vec();
            for i in (0..out.len()).rev() {
                if picked.contains(&out[i]) && i + 1 < out.len() && !picked.contains(&out[i + 1]) {
                    out.swap(i, i + 1);
                }
            }
            out
        }
        Reorder::Backward => {
            let mut out = order.to_vec();
            for i in 0..out.len() {
                if picked.contains(&out[i]) && i > 0 && !picked.contains(&out[i - 1]) {
                    out.swap(i, i - 1);
                }
            }
            out
        }
        Reorder::Out | Reorder::In => order.to_vec(),
    }
}

#[test]
fn restacking_plans_agree_with_a_reference_model() {
    for seed in 1..=60_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let n = 2 + rng.below(7);
        for how in [
            Reorder::Front,
            Reorder::Back,
            Reorder::Forward,
            Reorder::Backward,
        ] {
            let d = Document::new(1);
            let ids: Vec<NodeId> = (0..n).map(|i| rect(&d, i as f64 * 12.0)).collect();
            let k = 1 + rng.below(n);
            let mut picked: Vec<usize> = Vec::new();
            while picked.len() < k {
                let c = rng.below(n);
                if !picked.contains(&c) {
                    picked.push(c);
                }
            }
            let nodes: Vec<NodeId> = picked.iter().map(|&i| ids[i]).collect();
            let plans = reorder_plans(&d, &nodes, how);
            for plan in &plans {
                d.move_nodes(plan).unwrap();
            }
            let got: Vec<usize> = d
                .children(None)
                .iter()
                .map(|id| ids.iter().position(|x| x == id).unwrap())
                .collect();
            let want = model(&(0..n).collect::<Vec<_>>(), &picked, how);
            let mut sorted_picked = picked.clone();
            sorted_picked.sort_unstable();
            let contiguous = sorted_picked.windows(2).all(|w| w[1] == w[0] + 1);
            let step = matches!(how, Reorder::Forward | Reorder::Backward);
            if !step || contiguous {
                assert_eq!(got, want, "seed {seed} {how:?} picked {picked:?}");
            }
            // Whatever the grouping, the relative order of the picked nodes and of the others
            // never changes, and nothing is lost.
            let rel = |set: &[usize], order: &[usize]| -> Vec<usize> {
                order.iter().copied().filter(|i| set.contains(i)).collect()
            };
            let all: Vec<usize> = (0..n).collect();
            let others: Vec<usize> = all
                .iter()
                .copied()
                .filter(|i| !picked.contains(i))
                .collect();
            assert_eq!(
                rel(&picked, &got),
                rel(&picked, &all),
                "seed {seed} {how:?}"
            );
            assert_eq!(rel(&others, &got), others, "seed {seed} {how:?}");
        }
    }
}

#[test]
fn restacking_nothing_or_a_stale_node_yields_no_plan_and_does_not_panic() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    for how in [
        Reorder::Front,
        Reorder::Back,
        Reorder::Forward,
        Reorder::Backward,
        Reorder::Out,
        Reorder::In,
    ] {
        assert!(
            reorder_plans(&d, &[], how).is_empty()
                || reorder_plans(&d, &[], how)
                    .iter()
                    .all(|p| p.nodes.is_empty())
        );
        let _ = reorder_plans(&d, &[a], how);
    }
}
