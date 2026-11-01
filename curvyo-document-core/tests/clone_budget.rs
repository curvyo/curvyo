//! The cost of the clone read and the size of clones in a file (`specs/0049-linked-clone`
//! criteria 43, 46 and 63): 10,000 clones of one path of 100 nodes, and 1,250 chains of depth 8
//! on it, read with every clone resolved within 300 ms (a second read of the same version is the
//! session's cached read, pinned by `object_cache.rs`); the bytes a clone adds to the file are
//! printed for the PR; the switches of `specs/0057-clone-overrides` for 10,000 clones within
//! 250 ms each. Budgets are asserted in release builds only; a debug build runs a tenth of
//! the size and prints the numbers. Every cost is the thread CPU time of the work; the tests of
//! this file still share caches and memory bandwidth when they run in parallel (the switch took
//! up to twice as long on CI), so CI runs them one at a time, like the session's clone budget.
//! Run: `cargo test --release -p curvyo-document-core --test clone_budget -- --nocapture
//! --test-threads=1`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::Duration;

use curvyo_document_core::{AnchorId, Document, NewAnchor, NodeId, Point, Vec2, pack};

const CLONES: usize = if cfg!(debug_assertions) {
    1_000
} else {
    10_000
};
const READ_BUDGET: Duration = Duration::from_millis(300);
/// The busy threshold of `0016` (review L4 of `specs/0057-clone-overrides`): a switch of 10,000
/// clones stays below it.
const SWITCH_BUDGET: Duration = Duration::from_millis(250);

/// The result of `work` and the CPU time it cost this thread (`CLOCK_THREAD_CPUTIME_ID`, as in
/// the rotate test of `acceptance_0016_pr2_tester.rs`); wall time off Unix.
fn cpu_time_of<T>(work: impl FnOnce() -> T) -> (T, Duration) {
    #[cfg(unix)]
    let now = || {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Duration::new(
            u64::try_from(t.tv_sec).unwrap(),
            u32::try_from(t.tv_nsec).unwrap(),
        )
    };
    #[cfg(not(unix))]
    let now = {
        let origin = std::time::Instant::now();
        move || origin.elapsed()
    };
    let start = now();
    let result = work();
    (result, now().saturating_sub(start))
}

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

/// A closed path of 100 nodes on a circle.
fn root(document: &Document) -> NodeId {
    let anchors: Vec<NewAnchor> = (0..100_u64)
        .map(|k| {
            let angle = k as f64 / 100.0 * std::f64::consts::TAU;
            NewAnchor::corner(
                AnchorId::new(1, k),
                Point::new(20.0 + 10.0 * angle.cos(), 20.0 + 10.0 * angle.sin()),
            )
        })
        .collect();
    document.create_path(&anchors, true)
}

fn direct_clones() -> Document {
    let document = Document::new(1);
    let original = root(&document);
    for k in 0..CLONES {
        let clone = document.linked_clone(&[original], &[]).unwrap()[0];
        document
            .translate_objects(
                &[clone],
                Vec2::new((k % 100) as f64 * 25.0, (k / 100) as f64 * 25.0),
            )
            .unwrap();
    }
    document
}

fn chains() -> Document {
    let document = Document::new(1);
    let original = root(&document);
    for _ in 0..CLONES / 8 {
        let mut last = original;
        for _ in 0..8 {
            last = document.linked_clone(&[last], &[]).unwrap()[0];
        }
    }
    document
}

#[test]
fn ac43_63_reading_ten_thousand_clones_direct_or_in_chains() {
    for (label, document) in [
        ("direct clones", direct_clones()),
        ("chains of 8", chains()),
    ] {
        let (read, took) = cpu_time_of(|| document.read_objects());
        check(&format!("read of {CLONES} {label}"), took, READ_BUDGET);
        assert_eq!(read.objects.len(), CLONES + 1);
        assert_eq!(read.orphans, Vec::<NodeId>::new());
    }
}

#[test]
fn ac46_the_bytes_a_clone_adds_to_the_file() {
    let with = pack(&direct_clones(), "0.1.0").unwrap().len();
    let without = {
        let document = Document::new(1);
        root(&document);
        pack(&document, "0.1.0").unwrap().len()
    };
    let per_clone = (with - without) as f64 / CLONES as f64;
    println!(
        "{CLONES} clones add {} bytes: {per_clone:.0} bytes per clone (each with one move)",
        with - without
    );
    assert!(
        per_clone < 1_000.0,
        "a few hundred bytes per clone, not {per_clone:.0}"
    );
}

/// `specs/0057-clone-overrides` criterion 41 and review L4: switching 10,000 clones to own style
/// and own placement (each one commit), and reading them all with own styles within 300 ms.
#[test]
fn ac41_switching_and_reading_ten_thousand_clones_with_overrides() {
    let document = direct_clones();
    let clones: Vec<NodeId> = document
        .read_objects()
        .objects
        .iter()
        .skip(1)
        .map(curvyo_document_core::ObjectSnapshot::id)
        .collect();
    let (switched, took) = cpu_time_of(|| document.set_own_style(&clones, true).unwrap());
    assert_eq!(switched, CLONES);
    check(
        &format!("own style for {CLONES} clones"),
        took,
        SWITCH_BUDGET,
    );
    let (switched, took) = cpu_time_of(|| document.set_own_placement(&clones, true).unwrap());
    assert_eq!(switched, CLONES);
    check(
        &format!("own placement for {CLONES} clones"),
        took,
        SWITCH_BUDGET,
    );
    let (read, took) = cpu_time_of(|| document.read_objects());
    check(
        &format!("read of {CLONES} clones with own style"),
        took,
        READ_BUDGET,
    );
    assert_eq!(read.objects.len(), CLONES + 1);
    let ((), took) = cpu_time_of(|| {
        document
            .translate_objects(&[read.objects[0].id()], Vec2::new(1.0, 0.0))
            .unwrap();
    });
    // Not gated before the first measurement (criterion 41): printed for the PR.
    println!("move of the original holding back {CLONES} clones: {took:?} (target 3 s)");
    assert_eq!(document.take_clone_hold().held, CLONES);
}
