//! The cost of keeping dimensions current in the commit funnel (`specs/0070-measure-and-dimension`
//! criterion 11, ADR 0027 §4 and Consequences): a move of 200 measured paths, with their 200
//! dimensions refreshed in the same commit, and a commit in a document of 5,000 paths without
//! dimensions (the scan for dimensions). Budgets are asserted in release builds only; a debug
//! build prints the numbers. Every cost is the thread CPU time of the work.
//! Run: `cargo test --release -p curvyo-document-core --test dimension_budget -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use curvyo_document_core::{
    AnchorId, DimensionKind, DimensionPlace, DimensionSlots, Document, Length, NewAnchor, NodeId,
    ObjectSnapshot, Point, PointSlot, Vec2,
};

/// A 200-object move commit in a document with 200 dimensions.
const MOVE_BUDGET: Duration = Duration::from_millis(60);
/// One commit in a document of 5,000 paths and no dimension.
const SCAN_BUDGET: Duration = Duration::from_millis(15);

/// The result of `work` and the CPU time it cost this thread; wall time off Unix.
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

fn line(document: &Document, k: u64) -> NodeId {
    let y = k as f64 * 2.0;
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(5, 2 * k), Point::new(0.0, y)),
            NewAnchor::corner(AnchorId::new(5, 2 * k + 1), Point::new(25.0, y)),
        ],
        false,
    )
}

#[test]
fn a_move_of_200_measured_paths_refreshes_200_dimensions_within_budget() {
    let document = Document::new(1);
    let paths: Vec<NodeId> = (0..200).map(|k| line(&document, k)).collect();
    for (k, &path) in paths.iter().enumerate() {
        let k = k as u64;
        document
            .add_dimension(
                DimensionKind::Aligned,
                DimensionSlots::Points(vec![
                    PointSlot::Anchor {
                        node: path,
                        anchor: AnchorId::new(5, 2 * k),
                    },
                    PointSlot::Anchor {
                        node: path,
                        anchor: AnchorId::new(5, 2 * k + 1),
                    },
                ]),
                DimensionPlace::at_offset(Length::from_mm(1.0)),
            )
            .unwrap();
    }
    let ((), took) = cpu_time_of(|| {
        document
            .translate_objects(&paths, Vec2::new(3.0, 4.0))
            .unwrap();
    });
    check("move of 200 measured paths", took, MOVE_BUDGET);
    let moved = document
        .read_objects()
        .objects
        .iter()
        .filter(|object| {
            matches!(object, ObjectSnapshot::Dimension(d)
                if d.linked && (d.kept.points[0].x - 3.0).abs() < 1e-9)
        })
        .count();
    assert_eq!(
        moved, 200,
        "every kept value refreshed in the move's commit"
    );
}

#[test]
fn a_commit_without_dimensions_pays_only_the_scan() {
    let document = Document::new(1);
    let paths: Vec<NodeId> = (0..5_000).map(|k| line(&document, k)).collect();
    let ((), took) = cpu_time_of(|| {
        document
            .translate_objects(&paths[..1], Vec2::new(1.0, 0.0))
            .unwrap();
    });
    check("one commit among 5,000 paths", took, SCAN_BUDGET);
}
