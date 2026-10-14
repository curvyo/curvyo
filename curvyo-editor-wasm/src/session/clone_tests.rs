//! Session-level tests of `specs/0049-linked-clone`: Linked clone with its notice and refusal,
//! Unlink clone, Select original, root original and clones, the panel data, the repair notice and
//! the orphan repair after an Open.

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, Point, RectBounds, Vec2};

use super::clones::{CloneKind, TOO_DEEP_NOTICE};
use super::{Session, Tool};

fn rect(session: &Session, x: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn kind_of(session: &Session, id: NodeId) -> &'static str {
    match session.document.object(id) {
        Some(ObjectSnapshot::Path(_)) => "path",
        Some(ObjectSnapshot::Clone(_)) => "clone",
        Some(ObjectSnapshot::Primitive(_)) => "primitive",
        None => "none",
    }
}

#[test]
fn ac1_2_5_linked_clone_converts_a_shape_selects_the_clones_and_says_so() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    let b = rect(&session, 20.0);
    session.selection.set(&[a, b]);
    assert!(session.linked_clone_shown());
    let result = session.linked_clone();
    assert_eq!(result.kind, CloneKind::Done);
    assert_eq!(result.notice, "Created 2 linked clones.");
    assert_eq!(kind_of(&session, a), "path", "converted silently");
    let ids = session.document.object_ids();
    assert_eq!(ids.len(), 4);
    assert_eq!(session.selection.ids(), &[ids[1], ids[3]]);
    assert!(
        session
            .selection
            .ids()
            .iter()
            .all(|&id| kind_of(&session, id) == "clone")
    );
}

#[test]
fn ac4_another_tool_hides_the_button() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    session.selection.set(&[a]);
    session.tool = Tool::Node;
    assert!(!session.linked_clone_shown());
    assert_eq!(session.linked_clone().kind, CloneKind::Ignored);
}

#[test]
fn ac52_a_ninth_level_is_refused_with_its_notice() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    session.selection.set(&[a]);
    for _ in 0..8 {
        assert_eq!(session.linked_clone().kind, CloneKind::Done);
    }
    let before = session.document.object_ids();
    let refused = session.linked_clone();
    assert_eq!(refused.kind, CloneKind::Refused);
    assert_eq!(refused.notice, TOO_DEEP_NOTICE);
    assert_eq!(session.document.object_ids(), before);
}

#[test]
fn ac25_56_select_original_root_and_clones_and_the_panel_block() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let a = session.selection.ids()[0];
    session.linked_clone();
    let b = session.selection.ids()[0];
    let panel = session.clone_panel();
    let block = panel.block.unwrap();
    assert_eq!((block.count, block.deepest), (1, 2));
    session.select_original();
    assert_eq!(session.selection.ids(), &[a]);
    session.selection.set(&[b]);
    session.select_root_original();
    assert_eq!(session.selection.ids(), &[o]);
    assert_eq!(session.clone_panel().followers, 1);
    session.select_clones();
    assert_eq!(session.selection.ids(), &[a]);
}

#[test]
fn ac28_31_unlink_and_the_repair_notice() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let c = session.selection.ids()[0];
    session
        .document
        .translate_objects(&[c], Vec2::new(30.0, 0.0))
        .unwrap();
    let unlinked = session.unlink_clones();
    assert_eq!(unlinked.notice, "Unlinked 1 clone.");
    assert_eq!(kind_of(&session, c), "path");
    assert_eq!(session.selection.ids(), &[c], "still selected");
    // Delete an original with two clones: the repair sentence.
    session.selection.set(&[o]);
    session.linked_clone();
    session.selection.set(&[o]);
    session.linked_clone();
    session.document.delete_objects(&[o]).unwrap();
    assert_eq!(session.take_clone_repair_notice(), "2 clones became paths.");
    assert_eq!(session.take_clone_repair_notice(), "");
}

#[test]
fn ac35_an_open_removes_orphans_in_one_step_with_a_notice() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let peer = curvyo_document_core::unpack(
        2,
        &curvyo_document_core::pack(&session.document, "t").unwrap(),
    )
    .unwrap();
    let start = session.document.version();
    session.selection.set(&[o]);
    session.linked_clone();
    peer.delete_objects(&[o]).unwrap();
    session
        .document
        .import_updates(&peer.export_updates(&start).unwrap())
        .unwrap();
    let bytes = curvyo_document_core::pack(&session.document, "t").unwrap();
    let mut reopened = Session::open(3, &bytes).unwrap();
    assert_eq!(
        reopened.take_orphan_notice(),
        "Removed 1 clone whose original was not found."
    );
    assert_eq!(reopened.take_orphan_notice(), "");
    assert!(reopened.is_dirty(), "the project is marked changed");
    assert_eq!(
        reopened.document.read_objects().orphans,
        Vec::<NodeId>::new()
    );
}

/// A merged project in which one peer unlinked a clone while the other unlinked it and took that
/// back, in a timing that leaves the clone tag with the outline but no link (criterion 38 as
/// amended 2026-10-14).
fn project_with_a_linkless_clone() -> Vec<u8> {
    use curvyo_document_core::{AnchorId, CopySource, Document, pack, unpack};
    for pad in 0..144 {
        let a = Document::new(1);
        let o = a.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 0), Point::new(0.0, 0.0)),
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(9.0, 0.0)),
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(9.0, 9.0)),
            ],
            true,
        );
        let c = a.linked_clone(&[o], &[]).unwrap()[0];
        let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
        let start = a.version();
        for _ in 0..pad / 12 {
            a.translate_objects(&[o], Vec2::new(1.0, 0.0)).unwrap();
        }
        for _ in 0..pad % 12 {
            b.translate_objects(&[o], Vec2::new(0.0, 1.0)).unwrap();
        }
        for (d, peer) in [(&a, 7), (&b, 8)] {
            d.unlink_clones(&[CopySource {
                id: c,
                anchor_ids: (0..3).map(|k| AnchorId::new(peer, 100 + k)).collect(),
            }])
            .unwrap();
        }
        b.undo_step(b.own_steps().last().unwrap().step).unwrap();
        a.import_updates(&b.export_updates(&start).unwrap())
            .unwrap();
        if !a.read_objects().orphans.is_empty() {
            return pack(&a, "t").unwrap();
        }
    }
    panic!("no timing left the clone tag without a link");
}

#[test]
fn ac35_38_an_open_turns_a_linkless_clone_with_an_outline_into_a_path() {
    let mut reopened = Session::open(3, &project_with_a_linkless_clone()).unwrap();
    assert_eq!(reopened.take_orphan_notice(), "1 clone became a path.");
    assert!(reopened.is_dirty());
    let ids = reopened.document.object_ids();
    assert_eq!(ids.len(), 2);
    assert_eq!(kind_of(&reopened, ids[1]), "path");
}

#[test]
fn ac26_the_node_bar_names_a_clone() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    assert!(session.node_selection_is_clones());
    session.selection.set(&[o]);
    assert!(!session.node_selection_is_clones());
}

/// Review F5: the anchor ids the repair gives unlinked clones, across a Delete, an undo whose
/// restore repairs and a second Delete, are all different, all in the reserved half and never one
/// the session's minter gave.
#[test]
fn the_repair_ids_are_unique_in_one_session() {
    use std::collections::HashSet;

    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    session.selection.set(&[o]);
    session.linked_clone();
    let clones: Vec<NodeId> = session
        .document
        .object_ids()
        .into_iter()
        .filter(|&id| kind_of(&session, id) == "clone")
        .collect();
    let minted: HashSet<u128> = session
        .document
        .object_ids()
        .iter()
        .filter_map(|&id| match session.document.object(id) {
            Some(ObjectSnapshot::Path(path)) => Some(path),
            _ => None,
        })
        .flat_map(|path| {
            path.all_anchors()
                .map(|a| a.id.as_u128())
                .collect::<Vec<_>>()
        })
        .collect();
    let ids_of = |session: &Session| -> Vec<u128> {
        clones
            .iter()
            .filter_map(|&id| match session.document.object(id) {
                Some(ObjectSnapshot::Path(path)) => Some(path),
                _ => None,
            })
            .flat_map(|path| {
                path.all_anchors()
                    .map(|a| a.id.as_u128())
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let mut seen: Vec<u128> = Vec::new();
    // A Delete repair.
    session.selection.set(&[o]);
    session.recording(|this| this.document.delete_objects(&[o]).unwrap());
    seen.extend(ids_of(&session));
    // Undo brings the original back and turns the paths into clones again; a redo removes it
    // again, and its restore repairs.
    session.undo();
    session.redo();
    seen.extend(ids_of(&session));
    // A second Delete repair after an undo.
    session.undo();
    session.recording(|this| this.document.delete_objects(&[o]).unwrap());
    seen.extend(ids_of(&session));
    assert!(
        seen.len() >= 12,
        "three repairs of two clones: {}",
        seen.len()
    );
    let unique: HashSet<u128> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "every repair id is new");
    assert!(
        seen.iter().all(|id| (id >> 63) & 1 == 1),
        "the reserved half"
    );
    assert!(
        seen.iter().all(|id| !minted.contains(id)),
        "never a minted id"
    );
}

/// Review S2: the repair sentence belongs to the command that repaired; a later command that
/// repairs nothing does not show the old count.
#[test]
fn the_repair_count_is_reset_by_the_next_command() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    let other = rect(&session, 50.0);
    session.selection.set(&[o]);
    session.linked_clone();
    session.recording(|this| this.document.delete_objects(&[o]).unwrap());
    session.recording(|this| this.document.delete_objects(&[other]).unwrap());
    assert_eq!(session.take_clone_repair_notice(), "");
}

#[test]
fn ac62_a_screen_reader_hears_the_link() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let text = session.selection_announcement();
    assert!(text.starts_with("Linked clone of Path "), "{text}");
    assert!(
        text.contains("selected, 10.0 by 10.0 mm. Its original is Path "),
        "{text}"
    );
    assert!(!text.contains("Root original"), "{text}");
    // A chain names its root too (UX review should-fix 15).
    session.linked_clone();
    let text = session.selection_announcement();
    assert!(text.contains(". Its original is Clone of Path "), "{text}");
    assert!(text.contains(". Root original: Path "), "{text}");
}

/// `0049` criterion 25 with `0050`: a hidden original cannot be selected; the selection stays and a
/// refusal says why.
#[test]
fn ac25_a_hidden_original_is_left_out_with_a_refusal() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let clone = session.selection.ids()[0];
    session.document.set_node_visible(&[o], false).unwrap();
    let result = session.select_original();
    assert_eq!(result.kind, CloneKind::Refused);
    assert_eq!(
        result.notice,
        "The original is hidden or locked. Show or unlock it in the Layers tab. Nothing was changed."
    );
    assert_eq!(session.selection.ids(), &[clone]);
}

/// `0049` criterion 17: a node edit of a root that would push a clone past the limit is refused
/// and the notice names the clone.
#[test]
fn ac17_a_refused_root_edit_names_the_clone() {
    use curvyo_document_core::{Affine, AnchorId, NewAnchor};

    let mut session = Session::new(1);
    let corner = |k: u64, x: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, 0.0));
    let o = session.document.create_path(
        &[
            corner(0, 0.0),
            corner(1, 1.0),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(0.0, 1.0)),
        ],
        true,
    );
    session.selection.set(&[o]);
    session.linked_clone();
    let c = session.selection.ids()[0];
    let Some(ObjectSnapshot::Clone(clone)) = session.document.object(c) else {
        panic!("a clone")
    };
    session
        .document
        .transform_objects(
            &[ObjectSnapshot::Clone(
                clone.mapped_by(Affine::scale(5e6, 5e6)),
            )],
            false,
        )
        .unwrap();
    let far = AnchorId::new(1, 1);
    let refused = session.recording(|this| {
        this.document
            .move_anchors(&[(o, far, Point::new(10.0, 0.0))])
    });
    assert!(refused.is_err());
    let notice = session.take_clone_limit_notice();
    assert!(
        notice.starts_with("Nothing was changed: Clone of Path "),
        "{notice}"
    );
    assert_eq!(session.take_clone_limit_notice(), "");
}

/// UX review must-fix 1 (criteria 21, 22): with only clones selected the Style tab is reachable
/// (its body is the clone block) and its subject line names the clone.
#[test]
fn ac21_22_the_style_tab_opens_for_clones_only_with_the_clone_subject() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let clone = session.selection.ids()[0];
    assert!(
        session.press_panel_tab("style"),
        "the Style tab is not dimmed"
    );
    assert_eq!(session.panel_content(), curvyo_ui_core::PanelContent::Style);
    let name = curvyo_ui_core::object_name(&session.object_read(), clone);
    assert!(name.starts_with("Clone of Path "), "{name}");
    assert_eq!(session.style_panel_view().subject, name);
    let o2 = rect(&session, 40.0);
    session.selection.set(&[o2]);
    session.linked_clone();
    let second = session.selection.ids()[0];
    session.selection.set(&[clone, second]);
    assert_eq!(session.style_panel_view().subject, "2 clones");
}

/// UX review must-fix 10 (criterion 31): the Delete key's notice says what was deleted and how
/// the clones were repaired, both sentences.
#[test]
fn ac31_the_delete_notice_has_both_sentences() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    session.selection.set(&[o]);
    session.delete_selected();
    assert_eq!(
        session.take_delete_notice(),
        "Deleted 1 object. 1 clone became a path."
    );
    assert_eq!(session.take_delete_notice(), "");
    // Without a repair there is no notice, as before.
    let p = rect(&session, 50.0);
    session.selection.set(&[p]);
    session.delete_selected();
    assert_eq!(session.take_delete_notice(), "");
}

/// UX review must-fix 3: the hover box of Select original sits over the marks and does not
/// hide them.
#[test]
fn ac25_57_the_hover_box_keeps_the_marks() {
    let mut session = Session::new(1);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let clone = session.selection.ids()[0];
    session
        .document
        .translate_objects(&[clone], Vec2::new(40.0, 0.0))
        .unwrap();
    let before = session.clone_marks();
    session.set_clone_hover("original");
    let during = session.clone_marks();
    assert_eq!(during.traces, before.traces);
    assert_eq!(during.tags, before.tags);
    session.set_clone_hover("");
}

/// Design call on UX must-fix 3: the violet traces are drawn on top of the hover box of Select
/// original, so a box that coincides with an outline never hides the trace.
#[test]
fn ac25_57_the_traces_are_drawn_over_the_hover_box() {
    let mut session = Session::new(1);
    session.resize_viewport(1600.0, 1000.0);
    let o = rect(&session, 0.0);
    session.selection.set(&[o]);
    session.linked_clone();
    let clone = session.selection.ids()[0];
    session
        .document
        .translate_objects(&[clone], Vec2::new(40.0, 0.0))
        .unwrap();
    session.set_clone_hover("original");
    let list = session.draw_list();
    let colours: Vec<_> = list.triangles.iter().map(|v| v.color).collect();
    let is_trace = |c: &curvyo_render_core::RgbaColor| (c.r, c.g, c.b) == (0x7B, 0x3F, 0xD6);
    let is_hover =
        |c: &curvyo_render_core::RgbaColor| (c.r, c.g, c.b, c.a) == (0x2F, 0x6F, 0xEE, 166);
    let first_trace = colours.iter().position(is_trace).expect("a trace");
    let last_hover = colours.iter().rposition(is_hover).expect("a hover box");
    assert!(
        first_trace > last_hover,
        "traces {first_trace} over the hover box {last_hover}"
    );
    session.set_clone_hover("");
}

/// UX re-check: the red refusal outline of a depth-8 refusal is drawn over the violet marks.
#[test]
fn ac52_the_refusal_outline_is_drawn_over_the_clone_marks() {
    let mut session = Session::new(1);
    session.resize_viewport(1600.0, 1000.0);
    let a = rect(&session, 0.0);
    session.selection.set(&[a]);
    for _ in 0..8 {
        session.linked_clone();
    }
    assert_eq!(session.linked_clone().kind, CloneKind::Refused);
    let list = session.draw_list();
    let colours: Vec<_> = list.triangles.iter().map(|v| v.color).collect();
    let is_trace = |c: &curvyo_render_core::RgbaColor| (c.r, c.g, c.b) == (0x7B, 0x3F, 0xD6);
    let is_refusal = |c: &curvyo_render_core::RgbaColor| (c.r, c.g, c.b) == (0xB3, 0x26, 0x1E);
    let last_trace = colours.iter().rposition(is_trace).expect("a trace");
    let first_refusal = colours
        .iter()
        .position(is_refusal)
        .expect("a refusal outline");
    assert!(
        first_refusal > last_trace,
        "refusal {first_refusal} over the traces {last_trace}"
    );
}
