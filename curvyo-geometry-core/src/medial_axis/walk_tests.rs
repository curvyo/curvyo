//! Guards for the graph walks of the axis clean-up: on a graph with a small closed loop every
//! walk ends (`specs/0117-centre-lines`, the stress hang in `settle_corner`).

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use kurbo::Point;

use super::AxisSample;
use super::boundary::Boundary;
use super::graph::{Graph, NodeKind};
use super::{extract, spurs, trim};

/// A graph of `points` (position and clearance) joined by `links`.
fn graph(points: &[(f64, f64, f64)], links: &[(usize, usize)]) -> Graph {
    let mut graph = Graph {
        samples: points
            .iter()
            .map(|&(x, y, radius)| AxisSample {
                point: Point::new(x, y),
                radius,
            })
            .collect(),
        kinds: vec![NodeKind::Chord; points.len()],
        links: vec![Vec::new(); points.len()],
        alive: vec![true; points.len()],
    };
    for &(a, b) in links {
        graph.links[a].push(b);
        graph.links[b].push(a);
    }
    graph
}

/// A 20 mm square around the origin.
fn boundary() -> Boundary {
    Boundary::new(&[vec![
        Point::new(-10.0, -10.0),
        Point::new(10.0, -10.0),
        Point::new(10.0, 10.0),
        Point::new(-10.0, 10.0),
    ]])
}

/// Runs `f` in a thread and fails when it takes longer than 10 s.
fn finishes(name: &str, f: impl FnOnce() + Send + 'static) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    assert!(
        rx.recv_timeout(Duration::from_secs(10)).is_ok(),
        "{name} did not finish"
    );
}

/// A junction (node 0) with a short spur to a leaf (node 1), a long arm (nodes 4 to 7) and a
/// small loop through nodes 2 and 3 that stays within a radius of it.
fn lollipop() -> Graph {
    graph(
        &[
            (0.0, 0.0, 2.0),
            (0.5, 0.0, 0.5),
            (0.0, 0.3, 1.9),
            (0.3, 0.3, 1.9),
            (-2.0, 0.0, 2.0),
            (-4.0, 0.0, 2.0),
            (-6.0, 0.0, 2.0),
            (-8.0, 0.0, 2.0),
        ],
        &[
            (0, 1),
            (0, 2),
            (2, 3),
            (3, 0),
            (0, 4),
            (4, 5),
            (5, 6),
            (6, 7),
        ],
    )
}

#[test]
fn the_spur_prune_ends_on_a_small_loop_at_its_junction() {
    finishes("prune_spurs", || {
        for subtrees in [false, true] {
            let mut g = lollipop();
            spurs::prune_spurs(&mut g, &boundary(), subtrees);
        }
    });
}

#[test]
fn a_corner_settled_on_a_small_loop_ends() {
    // Once the spur is pruned the junction has degree two and both of its sides are the same
    // small loop: the corner walk comes back to the junction (the stress hang).
    finishes("prune_spurs on a loop", || {
        let mut g = graph(
            &[
                (0.0, 0.0, 2.0),
                (0.5, 0.0, 0.5),
                (0.0, 0.3, 1.9),
                (0.3, 0.3, 1.9),
            ],
            &[(0, 1), (0, 2), (2, 3), (3, 0)],
        );
        spurs::prune_spurs(&mut g, &boundary(), true);
    });
}

#[test]
fn the_end_trims_and_the_extract_end_on_a_small_loop() {
    finishes("trims and extract", || {
        let mut g = lollipop();
        trim::trim_ends(&mut g, &boundary());
        trim::trim_steep_ends(&mut g, &boundary());
        let _ = extract::extract(&g);
        // A bare ring, and a ring with a duplicated link.
        let mut ring = graph(
            &[
                (0.0, 0.0, 1.0),
                (1.0, 0.0, 1.0),
                (1.0, 1.0, 1.0),
                (0.0, 1.0, 1.0),
            ],
            &[(0, 1), (1, 2), (2, 3), (3, 0), (3, 0)],
        );
        trim::trim_ends(&mut ring, &boundary());
        trim::trim_steep_ends(&mut ring, &boundary());
        spurs::prune_spurs(&mut ring, &boundary(), true);
        let _ = extract::extract(&ring);
    });
}
