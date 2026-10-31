//! Independent tester cases for `specs/0088-trace-recognition` Part A in the editor session
//! (criteria 1, 3, 4, 6 to 10): face tiers and the stop at the first matching tier, what is
//! replaced, the one undo step and the guessed-letter record. Written from the specification
//! before the implementation was read; only public names were looked up.
//!
//! Lines are made the way a maker makes them: a text typed with the Text tool in an included
//! face, then Object to path (a group of compound paths, one per glyph). Faces are listed in the
//! blocks the host lists them in; a `sys:` or `my:` host id is the same included file listed a
//! second time as a system or Curvyo-folder font, and every file the session asks for is logged.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Document, FontKind, NodeId, ObjectSnapshot, Point, TextSnapshot, unpack,
};
use curvyo_editor_wasm::{
    BooleanOutcome, EscapeStep, KeyInput, ObjectToPathOutcome, RecogniseOutcome, RecognisedLine,
    RowPress, Session, Tool,
};
use curvyo_text_core::Mm;
use curvyo_ui_core::FaceTier;
use curvyo_ui_core::{BooleanOp, FlipAxis, FontBlock, TextInput};
use serde_json::Value;

type Listed = (String, String, String, u32, FontKind);

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// The included faces as `index.json` lists them; `keep` filters by (family, style).
fn included(keep: &dyn Fn(&str, &str) -> bool) -> Vec<Listed> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| keep(f["family"].as_str().unwrap(), f["style"].as_str().unwrap()))
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

fn all_included() -> Vec<Listed> {
    included(&|_, _| true)
}

/// One included file listed again under `prefix:` with the given family and style.
fn relisted(prefix: &str, file: &str, family: &str, style: &str) -> Listed {
    (
        format!("{prefix}:{file}"),
        family.to_owned(),
        style.to_owned(),
        0,
        FontKind::Outline,
    )
}

/// A session and the host ids of every font file it asked for.
struct Host {
    s: Session,
    read: Vec<String>,
}

impl Host {
    fn new() -> Self {
        let mut s = Session::new(88);
        s.resize_viewport(1600.0, 1000.0);
        s.set_font_list_with_kinds(FontBlock::Included, all_included(), Vec::new());
        let mut host = Self {
            s,
            read: Vec::new(),
        };
        host.load();
        host
    }

    fn load(&mut self) {
        for id in self.s.wanted_font_files() {
            let file = id.split_once(':').map_or(id.as_str(), |(_, f)| f);
            let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
            assert!(self.s.add_font_file(&id, &bytes), "{id} loads");
            self.read.push(id);
        }
    }

    fn face_index(&self, family: &str, style: &str) -> usize {
        let tab: Value = serde_json::from_str(&self.s.fonts_tab_json()).unwrap();
        let index = tab["faces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == family && f["style"] == style)
            .unwrap_or_else(|| panic!("{family} {style} is listed"))["index"]
            .as_u64()
            .unwrap();
        usize::try_from(index).unwrap()
    }

    fn doc(&self) -> Document {
        unpack(77, &self.s.pack("0.1.0").unwrap()).expect("the session output reopens")
    }

    fn texts(&self) -> Vec<TextSnapshot> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .filter_map(|id| match d.object(id) {
                Some(ObjectSnapshot::Text(t)) => Some(*t),
                _ => None,
            })
            .collect()
    }

    /// Every object in z-order with its whole snapshot, for exact comparison.
    fn fingerprint(&self) -> Vec<String> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .map(|id| format!("{id:?} {:?}", d.object(id)))
            .collect()
    }

    fn selected(&self) -> Vec<NodeId> {
        self.s.style_scope().ids
    }

    fn select(&mut self, ids: &[NodeId]) {
        self.s.set_tool(Tool::Select);
        for (i, &id) in ids.iter().enumerate() {
            let press = if i == 0 {
                RowPress::Plain
            } else {
                RowPress::Toggle
            };
            self.s
                .press_layer_row(&serde_json::to_string(&id).unwrap(), press);
        }
        assert_eq!(self.selected(), ids, "selected");
    }

    /// A text typed at `at` in `family` `style` at cap height `size`.
    fn add_text(
        &mut self,
        family: &str,
        style: &str,
        size: f64,
        at: Point,
        string: &str,
    ) -> NodeId {
        self.s.set_tool(Tool::Select);
        self.s.escape();
        self.s.escape();
        let index = self.face_index(family, style);
        self.s.pick_font(index);
        self.load();
        self.s.set_text_size(size);
        let before: Vec<NodeId> = self.texts().iter().map(|t| t.id).collect();
        self.s.set_tool(Tool::Text);
        self.s.pointer_hover(at, false, false);
        self.s.pointer_down(at, false);
        self.s.pointer_up(at, false, false);
        assert!(self.s.text_editing(), "a click starts edit mode");
        self.s.text_input(TextInput::Insert(string.to_owned()));
        self.load();
        assert_eq!(self.s.escape(), EscapeStep::ClearedState);
        self.load();
        let text = self
            .texts()
            .into_iter()
            .find(|t| !before.contains(&t.id))
            .unwrap();
        assert_eq!(
            (text.font.family.as_str(), text.font.style.as_str()),
            (family, style)
        );
        assert!(
            (text.size.as_mm() - size).abs() < 1e-9,
            "size {:?}",
            text.size
        );
        text.id
    }

    /// `string` typed in `family` `style` and turned into outlines with Object to path; the
    /// selected result.
    fn outlines_of(&mut self, family: &str, style: &str, size: f64, string: &str) -> Vec<NodeId> {
        let id = self.add_text(family, style, size, Point::new(40.0, 120.0), string);
        self.select(&[id]);
        assert_eq!(self.s.object_to_path(), ObjectToPathOutcome::Converted);
        let selected = self.selected();
        assert_ne!(
            selected,
            [] as [NodeId; 0],
            "Object to path selects its result"
        );
        assert!(
            self.texts().iter().all(|t| t.id != id),
            "the text became paths"
        );
        selected
    }

    /// Runs Recognise text on the selection to its end, loading faces as the host does.
    fn recognise(&mut self, tol: f64, favourites: &[&str]) -> RecogniseOutcome {
        let favourites: Vec<String> = favourites.iter().map(|f| (*f).to_owned()).collect();
        self.s.set_tool(Tool::Select);
        assert!(
            self.s.recognise_available(),
            "the Select bar offers Recognise text"
        );
        assert!(self.s.recognise_start(Mm(tol), &favourites), "a run starts");
        for _ in 0..10_000 {
            match self.s.recognise_step() {
                RecogniseOutcome::Loading => self.load(),
                RecogniseOutcome::Running { .. } => {}
                done => return done,
            }
        }
        panic!("the run did not end");
    }
}

/// The readout's count of face sources (the wasm surface hands the same count to the host).
fn sources(h: &Host, favourites: &[&str]) -> usize {
    let favourites: Vec<String> = favourites.iter().map(|f| (*f).to_owned()).collect();
    h.s.recognise_tiers(&favourites).sources()
}

fn applied(outcome: RecogniseOutcome) -> (Vec<RecognisedLine>, usize) {
    match outcome {
        RecogniseOutcome::Applied { lines, left, .. } => (lines, left),
        other => panic!("expected a written text, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------

/// Criteria 1 and 6: "Curvyo 2026" at 24 mm in Source Serif 4 Bold, turned into outlines with
/// Object to path, becomes one text in that face and size; one undo step brings back exactly
/// the outlines (same ids, z-order, geometry, style) with their selection; redo the text.
#[test]
fn c1_c6_object_to_path_line_becomes_one_text_and_one_undo_restores_it_exactly() {
    let mut h = Host::new();
    let paths = h.outlines_of("Source Serif 4", "Bold", 24.0, "Curvyo 2026");
    let before = h.fingerprint();
    let undo_before = h.s.undo_len();

    let (lines, left) = applied(h.recognise(0.3, &[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].string, "Curvyo 2026");
    assert_eq!(lines[0].face, "Source Serif 4 Bold");
    assert_eq!(lines[0].tier, FaceTier::Included);
    assert!(lines[0].guessed.is_empty(), "{lines:?}");
    assert!(lines[0].deviation <= 0.3);
    assert_eq!(left, 0);
    let texts = h.texts();
    assert_eq!(texts.len(), 1);
    let t = &texts[0];
    assert_eq!(t.string, "Curvyo 2026");
    assert_eq!(
        (t.font.family.as_str(), t.font.style.as_str()),
        ("Source Serif 4", "Bold")
    );
    assert!((t.size.as_mm() - 24.0).abs() <= 0.1, "size {:?}", t.size);
    assert_eq!(h.s.undo_len(), undo_before + 1, "one history step");
    assert_eq!(h.selected(), vec![t.id], "the new text is selected");

    h.s.undo();
    assert_eq!(
        h.fingerprint(),
        before,
        "one undo restores the outlines exactly"
    );
    assert_eq!(h.selected(), paths, "with their selection");
    assert_eq!(h.s.undo_len(), undo_before);
    h.s.redo();
    assert_eq!(h.texts().len(), 1, "redo writes the text again");
}

/// Criterion 4 (lead's mirrored case): mirrored outlines of a text stay paths, the run says
/// "nothing within the tolerance" with a nearest face, and nothing in the document changes.
#[test]
fn c4_mirrored_text_changes_nothing_and_names_a_nearest_face() {
    let mut h = Host::new();
    h.outlines_of("Source Sans 3", "Bold", 24.0, "Curvyo 2026");
    h.s.flip_selection(FlipAxis::Horizontal);
    let before = h.fingerprint();
    let undo_before = h.s.undo_len();
    match h.recognise(0.3, &[]) {
        RecogniseOutcome::Nothing {
            no_lines, nearest, ..
        } => {
            assert!(!no_lines, "a line was found");
            let (face, d) = nearest.expect("a nearest face");
            assert!(!face.is_empty() && d > 0.3, "{face} {d}");
        }
        other => panic!("mirrored text must stay paths, got {other:?}"),
    }
    assert_eq!(h.fingerprint(), before, "nothing changes");
    assert_eq!(h.s.undo_len(), undo_before, "no history step");
}

/// Criterion 4: with the line's face no longer listed, nothing is within 0.3 mm and the nearest
/// face and deviation are named; that deviation is the one the tolerance judges: a tolerance
/// just above it lets the line through.
#[test]
fn c4_the_named_deviation_is_the_one_the_tolerance_judges() {
    let mut h = Host::new();
    h.outlines_of("Source Serif 4", "Regular", 10.0, "Curvyo 2026");
    // The serif family leaves the library (it was uninstalled); only the sans faces remain.
    h.s.set_font_list_with_kinds(
        FontBlock::Included,
        included(&|family, _| family != "Source Serif 4"),
        Vec::new(),
    );
    let before = h.fingerprint();
    let (face, d) = match h.recognise(0.3, &[]) {
        RecogniseOutcome::Nothing { nearest, .. } => nearest.expect("a nearest face is named"),
        other => panic!("no face is within 0.3 mm, got {other:?}"),
    };
    assert!(face.starts_with("Source Sans 3"), "{face}");
    assert!(d > 0.3, "{d}");
    assert_eq!(h.fingerprint(), before, "nothing changes");
    let (lines, _) = applied(h.recognise(d + 0.02, &[]));
    assert!(
        lines[0].deviation <= d + 0.02,
        "{} within {:.2}",
        lines[0].deviation,
        d + 0.02
    );
}

/// Criterion 8: a face used by a text of the document matches in tier 1; the search stops there
/// and no system font file is read, though the system block lists matching faces.
#[test]
fn c8_a_document_face_stops_the_search_and_reads_no_system_file() {
    let mut h = Host::new();
    h.s.set_font_list_with_kinds(
        FontBlock::System,
        vec![
            relisted(
                "sys",
                "source-sans-3/SourceSans3-Bold.otf",
                "Sys Sans",
                "Bold",
            ),
            relisted(
                "sys",
                "source-serif-4/SourceSerif4-Regular.ttf",
                "Sys Serif",
                "Regular",
            ),
        ],
        Vec::new(),
    );
    h.add_text(
        "Source Sans 3",
        "Bold",
        12.0,
        Point::new(40.0, 40.0),
        "Kept",
    );
    h.outlines_of("Source Sans 3", "Bold", 24.0, "Curvyo 2026");
    h.read.clear();
    let (lines, _) = applied(h.recognise(0.3, &[]));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].string, "Curvyo 2026");
    assert_eq!(lines[0].tier, FaceTier::Document);
    assert_eq!(lines[0].face, "Source Sans 3 Bold");
    assert!(
        h.read.iter().all(|f| !f.starts_with("sys:")),
        "no system font file is read: {:?}",
        h.read
    );
}

/// Criterion 8: a face only among the system fonts is found in tier 5 and named as such; the
/// outlines have no fill (closed paths are read filled or not, lead 2026-10-31).
#[test]
fn c8_a_face_only_in_the_system_fonts_is_found_in_tier_5_unfilled() {
    let mut h = Host::new();
    let paths = h.outlines_of("Source Serif 4", "Italic", 24.0, "Curvyo 2026");
    h.s.set_tool(Tool::Select);
    h.select(&paths);
    h.s.set_fill_paint(false);
    let d = h.doc();
    let mut unfilled = 0;
    for id in d.object_ids() {
        if let Some(ObjectSnapshot::Path(p)) = d.object(id) {
            assert!(!p.style.fill.enabled, "fill off");
            assert!(p.subpaths().all(|o| o.closed), "closed outlines");
            unfilled += 1;
        }
    }
    assert!(unfilled > 0);
    // Source Serif 4 Italic now exists only as a system font.
    h.s.set_font_list_with_kinds(
        FontBlock::Included,
        included(&|family, style| !(family == "Source Serif 4" && style == "Italic")),
        Vec::new(),
    );
    h.s.set_font_list_with_kinds(
        FontBlock::System,
        vec![relisted(
            "sys",
            "source-serif-4/SourceSerif4-It.ttf",
            "Source Serif 4",
            "Italic",
        )],
        Vec::new(),
    );
    h.read.clear();
    let (lines, _) = applied(h.recognise(0.3, &[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].string, "Curvyo 2026");
    assert_eq!(lines[0].tier, FaceTier::System);
    assert_eq!(lines[0].face, "Source Serif 4 Italic");
    assert!(h.read.iter().any(|f| f.starts_with("sys:")), "{:?}", h.read);
}

/// Criterion 8: a face listed in two tiers counts in its earliest (included before system), so
/// no system file is read for it.
#[test]
fn c8_a_face_in_two_tiers_counts_in_its_earliest() {
    let mut h = Host::new();
    h.s.set_font_list_with_kinds(
        FontBlock::System,
        vec![relisted(
            "sys",
            "source-serif-4/SourceSerif4-It.ttf",
            "Source Serif 4",
            "Italic",
        )],
        Vec::new(),
    );
    h.outlines_of("Source Serif 4", "Italic", 24.0, "Curvyo 2026");
    h.read.clear();
    let (lines, _) = applied(h.recognise(0.3, &[]));
    assert_eq!(lines[0].tier, FaceTier::Included, "{lines:?}");
    assert!(
        h.read.iter().all(|f| !f.starts_with("sys:")),
        "{:?}",
        h.read
    );
}

/// Criterion 9: when an earlier tier (the Curvyo fonts folder, holding the bold cut) reaches
/// 80 % sure letters, it wins over the line's own face in a later tier (included), although
/// that one deviates less.
#[test]
fn c9_an_earlier_tier_at_80_percent_wins_over_a_closer_later_face() {
    let mut h = Host::new();
    h.outlines_of("Source Sans 3", "Regular", 5.0, "Curvyo 2026");
    h.s.set_font_list_with_kinds(
        FontBlock::Curvyo,
        vec![relisted(
            "my",
            "source-sans-3/SourceSans3-Bold.otf",
            "My Sans",
            "Bold",
        )],
        Vec::new(),
    );
    // At 5 mm and 0.7 mm the bold cut has all 10 letters of a regular line within the tolerance
    // (its largest deviation is about 0.57 mm); the regular cut itself is exact.
    let (lines, _) = applied(h.recognise(0.7, &[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        (lines[0].tier, lines[0].face.as_str()),
        (FaceTier::Installed, "My Sans Bold"),
        "the earlier tier wins: {lines:?}"
    );
    assert!(
        lines[0].letters * 4 <= (lines[0].letters - lines[0].guessed.len()) * 5,
        "80 % sure"
    );
}

/// Criterion 10: the readout counts the face sources that hold a face (5 when all have one);
/// and the Select bar offers the operation for closed paths, not for texts alone.
#[test]
fn c10_the_readout_counts_the_face_sources() {
    let mut h = Host::new();
    assert_eq!(sources(&h, &[]), 1, "included only");
    let id = h.add_text(
        "Source Sans 3",
        "Bold",
        12.0,
        Point::new(40.0, 40.0),
        "Kept",
    );
    assert_eq!(sources(&h, &[]), 2, "+ document");
    assert_eq!(sources(&h, &["Source Serif 4"]), 3, "+ favourites");
    h.s.set_font_list_with_kinds(
        FontBlock::Curvyo,
        vec![relisted(
            "my",
            "source-sans-3/SourceSans3-Light.otf",
            "My Sans",
            "Light",
        )],
        Vec::new(),
    );
    h.s.set_font_list_with_kinds(
        FontBlock::System,
        vec![relisted(
            "sys",
            "source-serif-4/SourceSerif4-It.ttf",
            "Sys Serif",
            "Italic",
        )],
        Vec::new(),
    );
    assert_eq!(sources(&h, &["Source Serif 4"]), 5, "all five");
    h.select(&[id]);
    assert!(
        !h.s.recognise_available(),
        "a text alone does not offer Recognise text"
    );
}

/// The default face at 36 mm, "Hamburg 1999", recognised: in the engine this line reads with H
/// and a guessed and the m left as a path. The new text alone (a part of the run's selection) is
/// the Glyph table's subject; the text's id.
fn guessed_line(h: &mut Host) -> NodeId {
    // Implementer, 2026-10-31 (fixture only, the assertions on the record are the tester's): the
    // line used to give guessed letters only through the large-size defect fixed with this
    // change, so 'H' and 'a' are now moved 1 mm up (one nudge): their fits still find them, the
    // laid-out text puts them 1 mm off, so they are guessed. Nothing stays a path any more.
    let glyphs = h.outlines_of("Source Sans 3", "Regular", 36.0, "Hamburg 1999");
    h.s.ungroup_selection();
    let d = h.doc();
    let mut leaves: Vec<(f64, NodeId)> = h
        .selected()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Path(p)) => Some((
                p.all_anchors()
                    .map(|a| a.point.x)
                    .fold(f64::INFINITY, f64::min),
                id,
            )),
            _ => None,
        })
        .collect();
    leaves.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(leaves.len() >= glyphs.len().min(2));
    h.select(&[leaves[0].1, leaves[1].1]);
    let nudged = h.s.key_down(KeyInput {
        key: "ArrowUp",
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
    assert_ne!(nudged, curvyo_editor_wasm::KeyOutcome::Ignored);
    let all: Vec<NodeId> = leaves.iter().map(|l| l.1).collect();
    h.select(&all);
    let (lines, _) = applied(h.recognise(0.3, &[]));
    let line = &lines[0];
    assert_eq!(line.guessed, vec![('H', 1), ('a', 2)], "{line:?}");
    assert!(
        line.deviation <= 0.3,
        "sure letters only: {}",
        line.deviation
    );
    let text = h.texts()[0].id;
    assert_eq!(h.s.recognise_guessed(text), Some(vec![0, 1]));
    h.select(&[text]);
    assert_eq!(h.s.recognise_guessed_chars(), ['H', 'a']);
    text
}

/// Criterion 7: guessed letters are reported with their 1-based positions and held for the new
/// text; undo ends the record.
#[test]
fn c7_guessed_letters_are_held_for_the_new_text_until_undo() {
    let mut h = Host::new();
    let text = guessed_line(&mut h);
    h.s.undo();
    assert_eq!(h.s.recognise_guessed(text), None, "undo ends the record");
    assert_eq!(h.s.recognise_guessed_chars(), [] as [char; 0]);
}

/// Criterion 7: "until the first edit of that text": a size change is an edit of the text.
#[test]
fn c7_the_first_edit_of_the_text_ends_the_record() {
    let mut h = Host::new();
    let text = guessed_line(&mut h);
    h.s.set_text_size(30.0);
    assert_eq!(h.texts()[0].size.as_mm(), 30.0, "an edit of that text");
    assert_eq!(
        h.s.recognise_guessed(text),
        None,
        "the first edit ends the record"
    );
}

/// Joined script (lead): a Great Vibes word whose letters are welded into one outline (as a
/// trace returns it) stays paths: touching letters are not split.
#[test]
fn joined_script_welded_into_one_outline_stays_paths() {
    let mut h = Host::new();
    h.outlines_of("Great Vibes", "Regular", 24.0, "minimum");
    h.s.ungroup_selection();
    let welded = h.s.apply_boolean(BooleanOp::Union);
    assert!(
        matches!(welded, BooleanOutcome::Applied { .. }),
        "the glyphs weld: {welded:?}"
    );
    let before = h.fingerprint();
    match h.recognise(0.3, &[]) {
        RecogniseOutcome::Nothing { .. } => {}
        RecogniseOutcome::Applied { lines, .. } => {
            panic!("welded script became text: {lines:?}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(h.fingerprint(), before);
}

/// The clean line the guessed-letter cases start from reads whole: every letter sure, nothing
/// left as a path (so the nudge in `guessed_line` is what makes H and a uncertain).
#[test]
fn the_clean_hamburg_line_reads_whole() {
    let mut h = Host::new();
    h.outlines_of("Source Sans 3", "Regular", 36.0, "Hamburg 1999");
    let (lines, left) = applied(h.recognise(0.3, &[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].string, "Hamburg 1999");
    assert_eq!(lines[0].guessed, vec![], "{lines:?}");
    assert_eq!(left, 0);
}

/// Wrong sure letters in the session: a Source Serif 4 line at 10 mm, the serif family no longer
/// listed, read against the sans faces at tolerances from 0.05 to 2.00 mm every `step` x 0.05 mm.
/// Returns the texts written and the wrong sure letters found.
fn session_cross_family(step: usize) -> (usize, Vec<String>) {
    let truth = "Curvyo 2026";
    let mut written = 0;
    let mut wrong = Vec::new();
    for k in (1..=40).step_by(step) {
        let tol = f64::from(u32::try_from(k).unwrap()) * 0.05;
        let mut h = Host::new();
        h.outlines_of("Source Serif 4", "Regular", 10.0, truth);
        h.s.set_font_list_with_kinds(
            FontBlock::Included,
            included(&|family, _| family != "Source Serif 4"),
            Vec::new(),
        );
        if let RecogniseOutcome::Applied { lines, .. } = h.recognise(tol, &[]) {
            for line in lines {
                written += 1;
                let truth: Vec<char> = truth.chars().collect();
                let got: Vec<char> = line.string.chars().collect();
                for (i, c) in got.iter().enumerate() {
                    let guessed = line.guessed.iter().any(|g| g.1 == i + 1);
                    if !guessed && !c.is_whitespace() && truth.get(i) != Some(c) {
                        wrong.push(format!("{tol:.2} mm: {:?} {:?}", line.string, line.guessed));
                        break;
                    }
                }
            }
        }
    }
    (written, wrong)
}

/// Criterion 9 with a wide tolerance, in the session (every 0.5 mm; the 0.05 mm sweep is the
/// ignored case below).
#[test]
fn session_cross_family_takes_no_wrong_letter_as_sure() {
    let (written, wrong) = session_cross_family(10);
    println!("session cross-family subset: {written} texts");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
#[ignore = "release sweep, prints numbers"]
fn session_cross_family_every_0_05_mm() {
    let (written, wrong) = session_cross_family(1);
    println!("session cross-family, every 0.05 mm: {written} texts");
    assert!(wrong.is_empty(), "{wrong:#?}");
}
