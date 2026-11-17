//! The staged finish of Find centre lines (`specs/0117-centre-lines` criterion 15b, ADR 0063
//! amendment 2026-11-18, W): a slider reruns only its own stage and those after it, never the
//! skeleton, and gives the same result as a fresh finish.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::{CentreOptions, CentreSkeleton, CentreStages, Outline, StageRuns};

const WHALE: &str = include_str!("fixtures/centre_lines/whale_traced.txt");
const DOLPHIN: &str = include_str!("fixtures/centre_lines/dolphin_traced.txt");

type Triple = (Point, Vec2, Vec2);

fn whale() -> Vec<(Vec<Triple>, bool)> {
    traced(WHALE)
}

/// The outlines of a traced fixture text, all subjects together.
fn traced(text: &str) -> Vec<(Vec<Triple>, bool)> {
    let mut outlines: Vec<(Vec<Triple>, bool)> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") | None => {}
            Some(&"outline") => outlines.push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                outlines.last_mut().unwrap().0.push((
                    Point::new(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
        }
    }
    outlines
}

fn skeleton(outlines: &[(Vec<Triple>, bool)]) -> CentreSkeleton {
    let outlines: Vec<Outline<'_>> = outlines.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    CentreSkeleton::build(&outlines).unwrap()
}

/// What each stage ran between two calls.
fn delta(after: StageRuns, before: StageRuns) -> [usize; 4] {
    [
        after.cut - before.cut,
        after.through - before.through,
        after.branches - before.branches,
        after.inner - before.inner,
    ]
}

#[test]
fn each_slider_reruns_only_its_stages_and_matches_a_fresh_finish() {
    let outlines = whale();
    let skeleton = skeleton(&outlines);
    let mut stages = CentreStages::new(skeleton.clone());
    let base = CentreOptions {
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    let steps: [(&str, CentreOptions); 7] = [
        ("first", base),
        ("same", base),
        (
            "overlap",
            CentreOptions {
                overlap: -0.1,
                ..base
            },
        ),
        (
            "offset",
            CentreOptions {
                overlap: -0.1,
                inner_offset: Some(Length::from_mm(-0.5)),
                ..base
            },
        ),
        (
            "optimise",
            CentreOptions {
                overlap: -0.1,
                inner_offset: Some(Length::from_mm(-0.5)),
                optimise: 0.4,
                ..base
            },
        ),
        (
            "thick",
            CentreOptions {
                overlap: -0.1,
                inner_offset: Some(Length::from_mm(-0.5)),
                optimise: 0.4,
                thick_parts: Some(Length::from_mm(9.0)),
                join_tips: true,
            },
        ),
        (
            "no limit",
            CentreOptions {
                thick_parts: None,
                ..base
            },
        ),
    ];
    // Which stages ran at all (each runs once per region that reaches it).
    let regions = |runs: [usize; 4]| runs.map(|n| usize::from(n > 0));
    let mut wanted: Vec<[usize; 4]> = Vec::new();
    let mut got: Vec<[usize; 4]> = Vec::new();
    for (name, options) in steps {
        let before = stages.runs();
        let memoised = stages.finish(&options, &[]);
        let fresh = skeleton.finish(&options, &[]);
        assert_eq!(
            format!("{memoised:?}"),
            format!("{fresh:?}"),
            "{name}: staged differs"
        );
        got.push(regions(delta(stages.runs(), before)));
        wanted.push(match name {
            "same" => [0, 0, 0, 0],
            "overlap" => [0, 0, 1, 0],
            "offset" => [0, 0, 0, 1],
            "optimise" => [0, 1, 1, 0],
            _ => [1, 1, 1, 1],
        });
    }
    assert_eq!(got, wanted);
}

/// Join tips (criterion 9b, CENTRE-TIPS): switching it makes the other chains once, on first
/// use, reruns the stages after them and matches a fresh finish; switching back and forth makes
/// them no more.
#[test]
fn join_tips_makes_its_chains_once_and_matches_a_fresh_finish() {
    let outlines = traced(DOLPHIN);
    let skeleton = skeleton(&outlines);
    let mut stages = CentreStages::new(skeleton.clone());
    let on = CentreOptions::default();
    let off = CentreOptions {
        join_tips: false,
        ..on
    };
    let mut made = Vec::new();
    for options in [on, off, on, off] {
        let before = stages.runs();
        let memoised = stages.finish(&options, &[]);
        let fresh = skeleton.finish(&options, &[]);
        assert_eq!(format!("{memoised:?}"), format!("{fresh:?}"));
        let after = stages.runs();
        made.push(after.tips - before.tips);
        assert!(
            after.branches > before.branches,
            "the paths follow the switch"
        );
    }
    assert_eq!(made[0], 0);
    assert!(made[1] > 0, "the chains with Join tips off are made");
    assert_eq!(&made[2..], &[0, 0]);
}
