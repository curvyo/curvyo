//! Cut in the editor session (`specs/0036-cut-at-crossings` criteria 1, 2, 6, 7, 8, 9) and the
//! sheet of six inputs of `specs/0048-split-compound-path` (criterion 14): what the Path card
//! enables and what Split, Break apart and Cut do with each input.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Document, EllipseFrame, Length, ObjectSnapshot, Point, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{
    BreakApartOutcome, CombineOutcome, CutOutcome, Session, SplitOutcome, Tool,
};
use curvyo_ui_core::{BooleanOp, BreakApartRefusal, CutRefusal, SplitRefusal};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) {
    let _ = document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    });
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn select_all(session: &mut Session) {
    let from = pt(-200.0, -200.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(pt(400.0, 400.0), false, false);
    session.pointer_up(pt(400.0, 400.0), false, false);
}

fn doc(session: &Session) -> Document {
    unpack(3, &session.pack("0.1.0").unwrap()).unwrap()
}

fn objects(session: &Session) -> Vec<ObjectSnapshot> {
    let document = doc(session);
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn commit_labels(session: &Session) -> Vec<String> {
    let loro = loro::LoroDoc::new();
    loro.import(&doc(session).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |change| {
        changes.push((
            change.lamport,
            change
                .message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, label)| label).collect()
}

/// The inputs E1 to E6, each as a document and a step that makes the compound path or the union
/// of its selected objects.
#[derive(Clone, Copy, Debug)]
enum Input {
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
}

fn input(which: Input) -> Session {
    let document = Document::new(1);
    match which {
        Input::E1 | Input::E5 => {
            disc(&document, 50.0, 50.0, 20.0);
            disc(&document, 50.0, 50.0, 10.0);
            if matches!(which, Input::E5) {
                disc(&document, 50.0, 50.0, 5.0);
            }
        }
        Input::E2 | Input::E3 => {
            disc(&document, 20.0, 50.0, 10.0);
            disc(&document, 80.0, 50.0, 10.0);
        }
        Input::E4 | Input::E6 => {
            disc(&document, 40.0, 50.0, 10.0);
            disc(&document, 50.0, 50.0, 10.0);
        }
    }
    let mut session = session_of(&document);
    select_all(&mut session);
    match which {
        Input::E1 | Input::E2 | Input::E5 => {
            assert!(matches!(
                session.apply_combine(),
                CombineOutcome::Applied { .. }
            ));
        }
        Input::E6 => {
            assert!(matches!(
                session.apply_boolean(BooleanOp::Union),
                curvyo_editor_wasm::BooleanOutcome::Applied { .. }
            ));
        }
        Input::E3 | Input::E4 => {}
    }
    session
}

fn closed_paths(session: &Session) -> usize {
    objects(session)
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(p) if p.closed))
        .count()
}

/// E1: a ring. Split is refused, Break apart makes 2 closed paths, Cut is refused.
#[test]
fn e1_a_ring() {
    let state = input(Input::E1).path_availability();
    assert!(state.compound && state.cut);
    assert_eq!(
        input(Input::E1).apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece { compounds: 1 })
    );
    let mut broken = input(Input::E1);
    assert!(matches!(
        broken.apply_break_apart(),
        BreakApartOutcome::Applied { pieces: 2, .. }
    ));
    assert_eq!(closed_paths(&broken), 2);
    assert!(matches!(
        input(Input::E1).apply_cut(),
        CutOutcome::Refused(CutRefusal::Compound { .. })
    ));
}

/// E2: two circles combined. Split and Break apart give 2 closed paths, Cut is refused.
#[test]
fn e2_two_circles_in_one_compound_path() {
    let mut split = input(Input::E2);
    assert!(matches!(
        split.apply_split(),
        SplitOutcome::Applied { pieces: 2, .. }
    ));
    assert_eq!(closed_paths(&split), 2);
    let mut broken = input(Input::E2);
    assert!(matches!(
        broken.apply_break_apart(),
        BreakApartOutcome::Applied { pieces: 2, .. }
    ));
    assert!(matches!(
        input(Input::E2).apply_cut(),
        CutOutcome::Refused(CutRefusal::Compound { .. })
    ));
}

/// E3: the same circles as two objects. Split and Break apart are dimmed, Cut says they do not
/// cross.
#[test]
fn e3_two_separate_circles() {
    let mut session = input(Input::E3);
    let state = session.path_availability();
    assert!(!state.compound, "Split and Break apart are dimmed");
    assert!(state.cut);
    assert_eq!(
        session.apply_split(),
        SplitOutcome::Refused(SplitRefusal::NoCompound)
    );
    assert_eq!(
        session.apply_break_apart(),
        BreakApartOutcome::Refused(BreakApartRefusal::NoCompound)
    );
    assert_eq!(
        session.apply_cut(),
        CutOutcome::Refused(CutRefusal::NoCrossing)
    );
}

/// E4: two overlapping circles as two objects. Cut makes 4 open arcs that meet at the two
/// crossings, with Fill None; Split and Break apart are dimmed.
#[test]
fn e4_two_overlapping_circles() {
    let mut session = input(Input::E4);
    assert!(!session.path_availability().compound);
    let before = commit_labels(&session).len();
    assert_eq!(
        session.apply_cut(),
        CutOutcome::Applied {
            paths: 2,
            pieces: 4
        }
    );
    let labels = commit_labels(&session);
    assert_eq!(labels.len(), before + 1, "one commit");
    assert_eq!(labels.last().unwrap(), "cut_at_crossings");
    assert_eq!(session.selected_object_count(), 4);
    assert_eq!(session.tool(), Tool::Select);
    let crossings = [pt(45.0, 50.0 + 8.66), pt(45.0, 50.0 - 8.66)];
    let pieces = objects(&session);
    assert_eq!(pieces.len(), 4);
    for piece in pieces {
        let ObjectSnapshot::Path(path) = piece else {
            panic!("a path");
        };
        assert!(!path.closed);
        assert!(!path.style.fill.enabled, "Fill None");
        let (first, last) = (
            path.anchors[0].point,
            path.anchors[path.anchors.len() - 1].point,
        );
        for end in [first, last] {
            assert!(
                crossings
                    .iter()
                    .any(|c| (end.x - c.x).hypot(end.y - c.y) < 0.01),
                "{end:?}"
            );
        }
    }
}

/// E5: a ring with a disc in its hole. Split gives the ring and the disc, Break apart three
/// closed paths, Cut is refused.
#[test]
fn e5_a_ring_with_a_disc_in_its_hole() {
    let mut split = input(Input::E5);
    assert!(matches!(
        split.apply_split(),
        SplitOutcome::Applied {
            pieces: 2,
            with_holes: true,
            ..
        }
    ));
    let paths = objects(&split);
    assert!(matches!(&paths[0], ObjectSnapshot::Path(p) if p.is_compound()));
    assert!(matches!(&paths[1], ObjectSnapshot::Path(p) if !p.is_compound()));
    let mut broken = input(Input::E5);
    assert!(matches!(
        broken.apply_break_apart(),
        BreakApartOutcome::Applied { pieces: 3, .. }
    ));
    assert_eq!(closed_paths(&broken), 3);
    assert!(matches!(
        input(Input::E5).apply_cut(),
        CutOutcome::Refused(CutRefusal::Compound { .. })
    ));
}

/// E6: the union of E4's circles, one ordinary closed path. Split and Break apart are dimmed, Cut
/// says it does not cross itself.
#[test]
fn e6_the_union_of_two_circles() {
    let mut session = input(Input::E6);
    assert!(!session.path_availability().compound);
    assert_eq!(
        session.apply_cut(),
        CutOutcome::Refused(CutRefusal::NoCrossing)
    );
}

/// 8 and 9: a refusal changes nothing, outlines compound paths and not the "do not cross" case,
/// and repeats.
#[test]
fn refusals_change_nothing_and_outline_only_their_offenders() {
    let mut ring = input(Input::E1);
    let before = commit_labels(&ring);
    let plain = ring.draw_list().triangle_count();
    let first = ring.apply_cut();
    assert!(matches!(
        first,
        CutOutcome::Refused(CutRefusal::Compound { .. })
    ));
    assert_eq!(commit_labels(&ring), before);
    assert!(ring.draw_list().triangle_count() > plain, "outlined");
    assert_eq!(ring.apply_cut(), first, "a second press: the same");
    ring.clear_boolean_refusal();
    assert_eq!(ring.draw_list().triangle_count(), plain);

    let mut apart = input(Input::E3);
    let plain = apart.draw_list().triangle_count();
    assert_eq!(
        apart.apply_cut(),
        CutOutcome::Refused(CutRefusal::NoCrossing)
    );
    assert_eq!(apart.draw_list().triangle_count(), plain, "no outline");
}

/// 1: the command is ignored outside the Select tool and with nothing selected.
#[test]
fn cut_is_ignored_outside_the_select_tool() {
    let mut session = input(Input::E4);
    session.set_tool(Tool::Node);
    assert_eq!(session.apply_cut(), CutOutcome::Ignored);
    assert!(!session.path_availability().cut);
}

/// 6: the stored fill colour and opacity survive, so a solid fill restores them.
#[test]
fn the_pieces_keep_the_stored_fill_colour() {
    let document = Document::new(1);
    disc(&document, 40.0, 50.0, 10.0);
    disc(&document, 50.0, 50.0, 10.0);
    let first = document.object_ids()[0];
    document
        .edit_style(
            &[first],
            &StyleEdit::FillColor(curvyo_document_core::Color {
                r: 200,
                g: 10,
                b: 10,
            }),
        )
        .unwrap();
    let mut session = session_of(&document);
    select_all(&mut session);
    let _ = session.apply_cut();
    let reds = objects(&session)
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(p) if p.style.fill.color.r == 200 && !p.style.fill.enabled))
        .count();
    assert_eq!(reds, 2, "the two arcs of the red circle");
}
