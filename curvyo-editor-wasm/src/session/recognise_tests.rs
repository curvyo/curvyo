//! Recognise text in the session (`specs/0088-trace-recognition` criteria 1, 4, 6, 7 and 10):
//! the run step by step with the faces loaded as the host loads them, the one history step, undo,
//! and the guessed letters the Text tool selects.

use curvyo_document_core::{Direction, NewAnchor, NodeId, Point, StyleEdit, Vec2};
use curvyo_text_core::{LayoutOptions, Mm, layout};
use kurbo::{Affine, BezPath, PathEl};

use super::recognise::RecogniseOutcome;
use super::text_tests::{labels, load_wanted, session};
use super::{Session, Tool};

/// The anchors of one closed subpath, handles kept.
fn anchors_of(outline: &BezPath, session: &mut Session) -> Vec<NewAnchor> {
    let mut anchors: Vec<NewAnchor> = Vec::new();
    let mut start = kurbo::Point::ORIGIN;
    for element in outline.elements() {
        match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                if anchors.is_empty() {
                    start = p;
                }
                anchors.push(NewAnchor::corner(
                    session.minter.mint(),
                    Point::new(p.x, p.y),
                ));
            }
            PathEl::CurveTo(c1, c2, p) => {
                let previous = anchors.last_mut().unwrap();
                previous.handle_out = Vec2::new(c1.x - previous.point.x, c1.y - previous.point.y);
                let mut anchor = NewAnchor::corner(session.minter.mint(), Point::new(p.x, p.y));
                anchor.handle_in = Vec2::new(c2.x - p.x, c2.y - p.y);
                anchors.push(anchor);
            }
            PathEl::QuadTo(..) | PathEl::ClosePath => {}
        }
    }
    let last = anchors[anchors.len() - 1];
    if (last.point.x - start.x).abs() < 1e-9 && (last.point.y - start.y).abs() < 1e-9 {
        anchors[0].handle_in = last.handle_in;
        anchors.pop();
    }
    anchors
}

/// One filled compound path of `text` laid out in the included face `family` `style` (as Trace
/// writes its result: one path, many outlines), selected; and its id.
fn traced_text(session: &mut Session, text: &str, size: f64) -> NodeId {
    let key = session
        .fonts
        .faces()
        .iter()
        .find(|f| f.family == "Source Sans 3" && f.style == "Bold")
        .and_then(|f| f.key);
    let key = key.unwrap_or_else(|| {
        let index = session
            .fonts
            .faces()
            .iter()
            .position(|f| f.family == "Source Sans 3" && f.style == "Bold")
            .unwrap();
        session.fonts.request(index);
        load_wanted(session);
        session.fonts.faces()[index].key.unwrap()
    });
    let face = session.fonts.store().get(&key).unwrap().clone();
    let laid = layout(&face, text, Mm(size), &LayoutOptions::default()).unwrap();
    let mut outlines: Vec<BezPath> = Vec::new();
    for glyph in &laid.lines[0].glyphs {
        let local = Affine::translate((20.0 + glyph.origin.x, 40.0 + glyph.origin.y))
            * Affine::scale(laid.mm_per_em);
        for element in (local * face.outline(glyph.glyph).path.clone()).elements() {
            if let PathEl::MoveTo(_) = element {
                outlines.push(BezPath::new());
            }
            outlines.last_mut().unwrap().push(*element);
        }
    }
    let subpaths: Vec<(Vec<NewAnchor>, bool)> = outlines
        .iter()
        .map(|o| (anchors_of(o, session), true))
        .collect();
    let seed = session.document.create_path(
        &[
            NewAnchor::corner(session.minter.mint(), Point::new(900.0, 900.0)),
            NewAnchor::corner(session.minter.mint(), Point::new(901.0, 900.0)),
        ],
        false,
    );
    let id = session
        .document
        .replace_with_path(&[seed], seed, &subpaths, "boolean_union")
        .unwrap();
    session
        .document
        .edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    session.selection.set(&[id]);
    id
}

/// Steps the run to its end, loading the faces it asks for as the host does.
fn finish(session: &mut Session) -> RecogniseOutcome {
    for _ in 0..1000 {
        match session.recognise_step() {
            RecogniseOutcome::Loading => load_wanted(session),
            RecogniseOutcome::Running { .. } => {}
            done => return done,
        }
    }
    panic!("the run did not end");
}

#[test]
fn criteria_1_and_6_a_traced_line_becomes_a_text_in_one_undoable_step() {
    let mut session = session();
    let path = traced_text(&mut session, "Laser 42", 10.0);
    assert!(session.recognise_available());
    let steps_before = labels(&session).len();
    assert!(session.recognise_start(Mm(0.3), &[]));
    let RecogniseOutcome::Applied { lines, left, .. } = finish(&mut session) else {
        panic!("nothing recognised");
    };
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].string, "Laser 42");
    assert_eq!(lines[0].face, "Source Sans 3 Bold");
    assert_eq!(left, 0);
    assert_eq!(lines[0].guessed, Vec::new());
    let new_labels = &labels(&session)[steps_before..];
    assert_eq!(new_labels, ["recognise_text"]);
    assert_eq!(
        session.history_target_name(Direction::Undo).as_deref(),
        Some("Recognise text")
    );
    let text = session.document.text(lines[0].text).unwrap();
    assert_eq!(text.string, "Laser 42");
    assert!(
        session.document.path(path).is_none(),
        "the path is replaced"
    );
    assert_eq!(session.selection.ids(), [lines[0].text]);
    // One undo brings the path back exactly.
    session.undo();
    assert!(session.document.path(path).is_some());
    assert!(session.document.text(lines[0].text).is_none());
}

#[test]
fn criterion_4_nothing_within_the_tolerance_changes_nothing() {
    let mut session = session();
    let path = traced_text(&mut session, "Laser", 10.0);
    // Stretch the path across: no face draws such letters.
    let stretched: Vec<_> = session
        .document
        .path(path)
        .unwrap()
        .subpaths()
        .map(|s| {
            let anchors = s
                .anchors
                .iter()
                .map(|a| {
                    let mut a = *a;
                    a.point.x *= 1.4;
                    a.handle_in.x *= 1.4;
                    a.handle_out.x *= 1.4;
                    a.id = session.minter.mint();
                    a
                })
                .collect::<Vec<_>>();
            (anchors, true)
        })
        .collect();
    let stretched = session
        .document
        .replace_with_path(&[path], path, &stretched, "boolean_union")
        .unwrap();
    session.selection.set(&[stretched]);
    let steps_before = labels(&session).len();
    assert!(session.recognise_start(Mm(0.3), &[]));
    let RecogniseOutcome::Nothing {
        no_lines, nearest, ..
    } = finish(&mut session)
    else {
        panic!("something was recognised");
    };
    assert!(!no_lines);
    let (_, deviation) = nearest.unwrap();
    assert!(deviation > 0.3);
    assert_eq!(labels(&session).len(), steps_before);
    assert!(session.document.path(stretched).is_some());
}

#[test]
fn a_run_ends_when_the_selection_changes() {
    let mut session = session();
    traced_text(&mut session, "Laser", 10.0);
    assert!(session.recognise_start(Mm(0.3), &[]));
    session.selection.clear();
    assert_eq!(session.recognise_step(), RecogniseOutcome::Ignored);
    assert!(!session.recognise_running());
}

#[test]
fn criterion_7_the_text_tool_selects_the_first_guessed_letter() {
    let mut session = session();
    let path = traced_text(&mut session, "Laser", 10.0);
    // Grow the outline of "L" (the first letter) by 15 %: it is guessed.
    let read = session.document.path(path).unwrap();
    let subpaths: Vec<_> = read.subpaths().map(|s| s.anchors.to_vec()).collect();
    let box_of = |anchors: &[NewAnchor]| {
        let xs = anchors.iter().map(|a| a.point.x);
        (
            xs.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let mut order: Vec<usize> = (0..subpaths.len()).collect();
    order.sort_by(|&a, &b| box_of(&subpaths[a]).0.total_cmp(&box_of(&subpaths[b]).0));
    let s_index = order[0];
    let (x0, x1) = box_of(&subpaths[s_index]);
    let ys: Vec<f64> = subpaths[s_index].iter().map(|a| a.point.y).collect();
    let (y0, y1) = (
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let centre = Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1));
    let grown: Vec<_> = subpaths
        .iter()
        .enumerate()
        .map(|(k, anchors)| {
            let anchors = anchors
                .iter()
                .map(|a| {
                    let mut a = *a;
                    if k == s_index {
                        a.point = Point::new(
                            centre.x + (a.point.x - centre.x) * 1.15,
                            centre.y + (a.point.y - centre.y) * 1.15,
                        );
                        a.handle_in = Vec2::new(a.handle_in.x * 1.15, a.handle_in.y * 1.15);
                        a.handle_out = Vec2::new(a.handle_out.x * 1.15, a.handle_out.y * 1.15);
                    }
                    a.id = session.minter.mint();
                    a
                })
                .collect::<Vec<_>>();
            (anchors, true)
        })
        .collect();
    let grown = session
        .document
        .replace_with_path(&[path], path, &grown, "boolean_union")
        .unwrap();
    session.selection.set(&[grown]);
    assert!(session.recognise_start(Mm(0.3), &[]));
    let RecogniseOutcome::Applied { lines, .. } = finish(&mut session) else {
        panic!("nothing recognised");
    };
    assert_eq!(lines[0].string, "Laser");
    assert_eq!(lines[0].guessed, vec![('L', 1)], "{lines:?}");
    let text = lines[0].text;
    assert_eq!(session.recognise_guessed(text), Some(vec![0]));
    assert_eq!(session.recognise_guessed_chars(), vec!['L']);
    // Enter on the text opens it with the guessed letter selected.
    session.edit_text_at(text, None);
    assert_eq!(session.tool, Tool::Text);
    assert_eq!(session.text.edit.as_ref().unwrap().selection(), 0..1);
    // The first edit ends the record.
    session.text_input(curvyo_ui_core::TextInput::Insert("x".to_owned()));
    session.finish_text_edit();
    assert_eq!(session.recognise_guessed(text), None);
}

#[test]
fn a_text_turned_into_unfilled_paths_is_read_back() {
    let mut session = super::text_tests::with_text("Laser 42");
    let id = super::text_tests::texts(&session)[0].id;
    session.set_tool(Tool::Select);
    session.selection.set(&[id]);
    let _ = session.object_to_path();
    // The glyph paths are stroked, not filled: a closed outline is read as filled.
    assert!(session.recognise_available());
    assert!(session.recognise_start(Mm(0.3), &[]));
    let RecogniseOutcome::Applied { lines, left, .. } = finish(&mut session) else {
        panic!("nothing recognised");
    };
    assert_eq!(lines[0].string, "Laser 42");
    assert_eq!(left, 0);
}
