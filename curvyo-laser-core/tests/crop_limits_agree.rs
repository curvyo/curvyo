//! Architect review A2 (`specs/0135` slice 2, ADR 0072 §3): document-core's visible frame and
//! laser-core's Crop fields agree on which crops read: a crop the job runs is exactly the frame
//! the document draws, and one the job does not run leaves the whole picture drawn.

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{AdjustEntry, AdjustStep, UnitRect, visible_rect};
use curvyo_laser_core::adjust::ManualStep;

fn entry(pct: [i64; 4]) -> AdjustEntry {
    let keys = ["left_pct", "top_pct", "width_pct", "height_pct"];
    AdjustEntry::Step(AdjustStep {
        kind: "crop".to_owned(),
        enabled: true,
        fields: keys
            .iter()
            .zip(pct)
            .map(|(k, v)| ((*k).to_owned(), v))
            .collect(),
    })
}

#[test]
fn the_documents_frame_and_the_jobs_crop_agree_on_every_edge_case() {
    let values = [-1, 0, 1, 49, 50, 51, 98, 99, 100, 101];
    for l in values {
        for t in [0, 50, 99, 100] {
            for w in values {
                for h in [0, 1, 50, 100, 101] {
                    let pct = [l, t, w, h];
                    let read = ManualStep::read("crop", |key| {
                        ["left_pct", "top_pct", "width_pct", "height_pct"]
                            .iter()
                            .position(|k| *k == key)
                            .map(|i| pct[i])
                    });
                    let drawn = visible_rect(&[entry(pct)]);
                    match read {
                        Ok(ManualStep::Crop { pct: kept }) => {
                            let [l, t, w, h] = kept.map(|v| f64::from(v) / 100.0);
                            let expected = UnitRect {
                                left: l,
                                top: t,
                                width: w,
                                height: h,
                            };
                            assert!(
                                (drawn.left - expected.left).abs() < 1e-12
                                    && (drawn.width - expected.width).abs() < 1e-12
                                    && (drawn.top - expected.top).abs() < 1e-12
                                    && (drawn.height - expected.height).abs() < 1e-12,
                                "{pct:?}: {drawn:?}"
                            );
                        }
                        Ok(other) => panic!("{other:?}"),
                        Err(_) => assert_eq!(drawn, UnitRect::WHOLE, "{pct:?} is not run"),
                    }
                }
            }
        }
    }
}
