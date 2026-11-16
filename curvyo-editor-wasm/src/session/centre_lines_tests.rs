//! Find centre lines in the session (`specs/0117-centre-lines` criteria 1 to 4, 12 and 18): the
//! enabled rule, one step that undo and redo take, the selection after, the result's style and
//! profile, refusals without a history entry.

use curvyo_document_core::{
    Length, LineCap, NewAnchor, ObjectSnapshot, Point, RectBounds, StyleEdit,
};
use curvyo_geometry_core::CentreOptions;
use curvyo_ui_core::CentreLinesRefusal;

use super::CentreLinesOutcome;
use super::text_tests::labels;
use super::{Session, Tool};

fn select(session: &mut Session, ids: &[curvyo_document_core::NodeId]) {
    session.set_tool(Tool::Select);
    session.selection.clear();
    for &id in ids {
        session.selection.add(id);
    }
}

/// A filled closed path: a 40 x 2 mm bar.
fn bar(session: &Session) -> curvyo_document_core::NodeId {
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let anchors = [(0.0, 0.0), (40.0, 0.0), (40.0, 2.0), (0.0, 2.0)]
        .map(|(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)));
    let id = session.document.create_path(&anchors, true);
    session
        .document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    id
}

#[test]
fn ac01_ac04_enabled_by_a_filled_shape_refused_otherwise() {
    let mut session = Session::new(1);
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let open = session.document.create_path(
        &[
            NewAnchor::corner(minter.mint(), Point::new(0.0, 0.0)),
            NewAnchor::corner(minter.mint(), Point::new(10.0, 0.0)),
        ],
        false,
    );
    select(&mut session, &[open]);
    assert!(!session.centre_lines_available());
    let steps = labels(&session).len();
    assert_eq!(
        session.apply_centre_lines(CentreOptions::default()),
        CentreLinesOutcome::Refused(CentreLinesRefusal::NotFilled)
    );
    assert_eq!(labels(&session).len(), steps);
    let filled = bar(&session);
    select(&mut session, &[filled, open]);
    assert!(session.centre_lines_available());
}

#[test]
fn ac02_ac03_one_step_a_profiled_centre_path_undo_and_redo() {
    let mut session = Session::new(1);
    let a = bar(&session);
    select(&mut session, &[a]);
    let steps = labels(&session).len();
    let outcome = session.apply_centre_lines(CentreOptions::default());
    assert_eq!(
        outcome,
        CentreLinesOutcome::Applied {
            converted: 1,
            kept: 0,
            kept_regions: 0,
            failed: 0,
            skipped: 0,
            unlinked: 0
        }
    );
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "centre_lines");
    let objects = session.objects();
    assert_eq!(objects.len(), 1);
    let ObjectSnapshot::Path(path) = &objects[0] else {
        panic!("a path");
    };
    let style = objects[0].style();
    assert!(style.stroke.enabled && !style.fill.enabled);
    assert_eq!(style.stroke.cap, LineCap::Round);
    assert!((style.stroke.width.as_mm() - 2.0).abs() <= 0.06);
    assert_ne!(session.document.width_points(path.id).unwrap().len(), 0);
    assert_eq!(session.selection.ids(), [objects[0].id()]);
    session.undo();
    assert_eq!(session.objects().len(), 1);
    assert_eq!(session.objects()[0].id(), a);
    session.redo();
    assert_ne!(session.objects()[0].id(), a);
}

#[test]
fn ac12_a_square_stays_filled_and_is_counted() {
    let mut session = Session::new(1);
    let disc = session.document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    session
        .document
        .edit_style(&[disc], &StyleEdit::FillEnabled(true))
        .unwrap();
    select(&mut session, &[disc]);
    let steps = labels(&session).len();
    let outcome = session.apply_centre_lines(CentreOptions::default());
    assert_eq!(
        outcome,
        CentreLinesOutcome::Applied {
            converted: 0,
            kept: 1,
            kept_regions: 1,
            failed: 0,
            skipped: 0,
            unlinked: 0
        }
    );
    assert_eq!(labels(&session).len(), steps);
}

/// ADR 0063 §12: a grouped result (a T: main path and branch) writes every leaf's profile in
/// the one batched write: each leaf holds its width points, multipliers in (0, 1].
#[test]
fn ac03_each_leaf_of_a_grouped_result_holds_its_profile() {
    let mut session = Session::new(1);
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let tee = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 2.0),
        (21.0, 2.0),
        (21.0, 22.0),
        (19.0, 22.0),
        (19.0, 2.0),
        (0.0, 2.0),
    ]
    .map(|(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)));
    let id = session.document.create_path(&tee, true);
    session
        .document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    select(&mut session, &[id]);
    let outcome = session.apply_centre_lines(CentreOptions::default());
    assert!(
        matches!(outcome, CentreLinesOutcome::Applied { converted: 1, .. }),
        "{outcome:?}"
    );
    let group = session.selection.ids()[0];
    let leaves = session.document.leaves_of(group);
    assert_eq!(leaves.len(), 2, "main path and branch");
    for leaf in leaves {
        let points = session.document.width_points(leaf).unwrap();
        assert!(
            points.len() >= 2,
            "{leaf:?} has {} width points",
            points.len()
        );
        assert!(
            points
                .iter()
                .all(|p| p.multiplier > 0.0 - 1e-12 && p.multiplier <= 1.0 + 1e-12)
        );
    }
}

/// The overlap option reaches the result: the branch of a T starts further inside the bar the
/// larger the overlap is (the frontend sends percent / 100 through `apply_centre_lines`).
#[test]
fn ac10_the_overlap_moves_the_start_of_the_branch() {
    let starts: Vec<f64> = [-0.2, 0.0, 0.2]
        .into_iter()
        .map(|overlap| {
            let mut session = Session::new(1);
            let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
            let tee = [
                (0.0, 0.0),
                (40.0, 0.0),
                (40.0, 2.0),
                (21.0, 2.0),
                (21.0, 22.0),
                (19.0, 22.0),
                (19.0, 2.0),
                (0.0, 2.0),
            ]
            .map(|(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)));
            let id = session.document.create_path(&tee, true);
            session
                .document
                .edit_style(&[id], &StyleEdit::FillEnabled(true))
                .unwrap();
            select(&mut session, &[id]);
            let options = CentreOptions {
                overlap,
                ..CentreOptions::default()
            };
            session.apply_centre_lines(options);
            // The branch is the leaf whose anchors reach y = 22 (the bar's path stays at y = 1).
            let group = session.selection.ids()[0];
            let mut start = f64::NAN;
            for leaf in session.document.leaves_of(group) {
                let ys: Vec<f64> = session
                    .objects()
                    .iter()
                    .filter(|o| o.id() == leaf)
                    .flat_map(|o| match o {
                        ObjectSnapshot::Path(p) => p.anchors.iter().map(|a| a.point.y).collect(),
                        _ => Vec::new(),
                    })
                    .collect();
                if ys.iter().any(|y| *y > 15.0) {
                    start = ys.iter().copied().fold(f64::INFINITY, f64::min);
                }
            }
            start
        })
        .collect();
    assert!(
        starts[0] > starts[1] + 0.2 && starts[1] > starts[2] + 0.2,
        "{starts:?}"
    );
}

/// The customer saw an Apply do nothing after many steps: Apply and Undo forty times, each Apply
/// converts and each Undo restores.
#[test]
fn ac02_apply_and_undo_many_times() {
    let mut session = Session::new(1);
    let id = bar(&session);
    for round in 0..40 {
        select(&mut session, &[id]);
        let outcome = session.apply_centre_lines(CentreOptions::default());
        assert!(
            matches!(outcome, CentreLinesOutcome::Applied { converted: 1, .. }),
            "round {round}: {outcome:?}"
        );
        session.undo();
        assert!(
            session.document.object(id).is_some(),
            "round {round}: undo restores the subject"
        );
    }
    // An Apply with nothing selected says why instead of doing nothing.
    session.selection.clear();
    assert_eq!(
        session.apply_centre_lines(CentreOptions::default()),
        CentreLinesOutcome::Refused(CentreLinesRefusal::NotFilled)
    );
}

/// The bar's path in the Node tool (criteria 1, 2, 15, 15b; ADR 0063 amendment 2026-11-18, A):
/// open sends the subjects, the worker answers a preview, Apply commits the worker's result in
/// slices and one step, without the session running the kernel.
fn open_in_node_tool(session: &mut Session, id: curvyo_document_core::NodeId) -> Vec<f64> {
    session.set_tool(Tool::Node);
    session.selection.set(&[id]);
    session.centre_lines_open().expect("the bar opens")
}

#[test]
fn ac15b_the_preview_and_apply_run_on_the_worker_result_in_the_node_tool() {
    use crate::centre_lines_glue::CentreLinesWorker;
    let mut session = Session::new(1);
    let a = bar(&session);
    let subjects = open_in_node_tool(&mut session, a);
    assert!(session.centre_lines_is_open());
    let mut worker = CentreLinesWorker::new();
    worker.set_subjects(&subjects, &mut |_| {}).unwrap();
    let steps = labels(&session).len();
    let preview = worker.finish(&CentreOptions::default(), true);
    let counts = session
        .centre_lines_show(&preview)
        .expect("the preview fits");
    assert_eq!((counts.paths, counts.kept), (1, 0));
    // A preview is never a commit.
    assert_eq!(labels(&session).len(), steps);
    assert_eq!(session.objects().len(), 1);
    // The canvas draws the preview and no node marks while the bar is open.
    assert!(session.centre_lines_hides_marks());
    let result = worker.result();
    let slices = session.centre_lines_apply_begin(&result).unwrap();
    assert!(slices >= 2);
    let mut outcome = None;
    for _ in 0..slices {
        outcome = session.centre_lines_apply_step();
        if outcome.is_some() {
            break;
        }
    }
    assert!(matches!(
        outcome,
        Some(CentreLinesOutcome::Applied { converted: 1, .. })
    ));
    assert_eq!(labels(&session).len(), steps + 1);
    assert!(!session.centre_lines_is_open());
    assert_eq!(session.tool, Tool::Node);
    // The same as the one-call Apply in the Select tool.
    let mut direct = Session::new(1);
    let b = bar(&direct);
    select(&mut direct, &[b]);
    direct.apply_centre_lines(CentreOptions::default());
    let shape = |s: &Session| {
        s.objects()
            .iter()
            .map(|o| match o {
                ObjectSnapshot::Path(p) => format!(
                    "{:?} {:?} {:?}",
                    p.anchors
                        .iter()
                        .map(|a| (a.point, a.handle_in, a.handle_out))
                        .collect::<Vec<_>>(),
                    p.style.stroke.width,
                    p.width_points
                        .iter()
                        .map(|w| (w.position, w.multiplier))
                        .collect::<Vec<_>>()
                ),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&session), shape(&direct));
}

#[test]
fn ac15b_cancel_and_a_changed_selection_write_nothing() {
    use crate::centre_lines_glue::CentreLinesWorker;
    let mut session = Session::new(1);
    let a = bar(&session);
    let subjects = open_in_node_tool(&mut session, a);
    let mut worker = CentreLinesWorker::new();
    worker.set_subjects(&subjects, &mut |_| {}).unwrap();
    let _ = worker.finish(&CentreOptions::default(), true);
    // A changed selection makes the result stale: nothing is written.
    let other = bar(&session);
    let steps = labels(&session).len();
    session.selection.set(&[a, other]);
    assert!(!session.centre_lines_current());
    assert_eq!(
        session.centre_lines_apply_begin(&worker.result()),
        Err(CentreLinesOutcome::Stale)
    );
    // Escape during the slices drops them.
    session.selection.set(&[a]);
    let subjects = session.centre_lines_open().unwrap();
    worker.set_subjects(&subjects, &mut |_| {}).unwrap();
    let _ = worker.finish(&CentreOptions::default(), false);
    session.centre_lines_apply_begin(&worker.result()).unwrap();
    session.centre_lines_close();
    assert_eq!(
        session.centre_lines_apply_step(),
        Some(CentreLinesOutcome::Ignored)
    );
    assert_eq!(labels(&session).len(), steps);
    // A press on empty canvas deselects and closes the bar; a press on the subject does nothing.
    session.centre_lines_open().unwrap();
    session.pointer_down(Point::new(20.0, 1.0), false);
    session.pointer_up(Point::new(20.0, 1.0), false, false);
    assert!(session.centre_lines_is_open());
    assert_eq!(session.selection.ids(), [a]);
    session.pointer_down(Point::new(200.0, 200.0), false);
    session.pointer_up(Point::new(200.0, 200.0), false, false);
    assert!(!session.centre_lines_is_open());
    assert!(session.selection.is_empty());
    assert_eq!(labels(&session).len(), steps);
}

/// An edit between the slices of Apply (a peer's, or a key the canvas let through) makes the
/// result stale before the write: nothing is written, the subjects stay, no line lands beside
/// them, and the bar stays open to send the subjects anew (architect review, blocker 1).
#[test]
fn ac02_an_edit_between_the_slices_writes_nothing() {
    use crate::centre_lines_glue::CentreLinesWorker;
    let mut session = Session::new(1);
    let a = bar(&session);
    let subjects = open_in_node_tool(&mut session, a);
    let mut worker = CentreLinesWorker::new();
    worker.set_subjects(&subjects, &mut |_| {}).unwrap();
    let _ = worker.finish(&CentreOptions::default(), false);
    let slices = session.centre_lines_apply_begin(&worker.result()).unwrap();
    assert!(slices >= 2);
    assert_eq!(session.centre_lines_apply_step(), None);
    // The document changes between two slices.
    let other = bar(&session);
    let steps = labels(&session).len();
    let before: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    let mut outcome = None;
    for _ in 0..slices {
        outcome = session.centre_lines_apply_step();
        if outcome.is_some() {
            break;
        }
    }
    assert_eq!(outcome, Some(CentreLinesOutcome::Stale));
    let after: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    assert_eq!(
        after, before,
        "no partial commit, no lines beside the subjects"
    );
    assert!(after.contains(&a) && after.contains(&other));
    assert_eq!(labels(&session).len(), steps, "no Find centre lines step");
    assert!(session.centre_lines_is_open());
    assert!(!session.centre_lines_current());
    // The slices are dropped: a further step writes nothing either.
    assert_eq!(
        session.centre_lines_apply_step(),
        Some(CentreLinesOutcome::Ignored)
    );
}

#[test]
fn the_result_wire_gives_the_drawing_of_the_kernel_result() {
    use crate::centre_lines_wire::{SubjectLines, decode_results, encode_results};
    use curvyo_geometry_core::{CentreSkeleton, Outline};
    let session = Session::new(1);
    let a = bar(&session);
    let path = session.document.path(a).unwrap();
    let anchors: Vec<_> = path
        .anchors
        .iter()
        .map(|x| (x.point, x.handle_in, x.handle_out))
        .collect();
    let lines = CentreSkeleton::build(&[Outline::new(&anchors, true)])
        .unwrap()
        .finish(&CentreOptions::default(), &[]);
    let wire = encode_results(&[SubjectLines {
        lines: Some(lines.clone()),
        painted: Vec::new(),
    }]);
    let back = decode_results(&wire).unwrap();
    let subject = curvyo_ui_core::CentreSubject {
        id: a,
        outlines: vec![(anchors, true)],
        style: path.style.clone(),
        clone: false,
    };
    assert_eq!(
        curvyo_ui_core::centre_lines_drawing(&subject, back[0].lines.as_ref().unwrap(), None),
        curvyo_ui_core::centre_lines_drawing(&subject, &lines, None)
    );
    assert!(decode_results(&wire[..wire.len() - 1]).is_err());
}
