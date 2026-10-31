//! The session's Recognise text run (`specs/0088-trace-recognition` criteria 3 to 10): the job
//! over the selection, one line and tier per step so the host can paint progress and stop it,
//! the faces it asks for, and the write as one history step "Recognise text"; plus the session
//! record of guessed letters the Glyph table and the Text tool highlight.

use std::sync::Arc;

use curvyo_document_core::{
    FontRef, MovePlan, MoveTarget, NodeId, ObjectSnapshot, PieceFill, StyleEdit, TextChars,
};
use curvyo_text_core::Mm;
use curvyo_ui_core::{
    FaceState, FaceTier, JobStep, RecogniseJob, RecogniseSubjects, RecogniseWrite, Tiers,
    plan_recognise_write, recognise_available, recognise_subjects, recognise_tiers,
};

use super::{Session, Tool};

/// The label of the one step a run writes; its history name is "Recognise text".
pub(crate) const RECOGNISE_TEXT_LABEL: &str = "recognise_text";

/// A run in flight: the job, what it reads and the faces it asked for.
#[derive(Debug)]
pub(crate) struct RunningRecognise {
    job: RecogniseJob,
    subjects: RecogniseSubjects,
    /// The selection the run was started for; a change ends the run.
    selection: Vec<NodeId>,
    /// Listed faces asked for once: still not loaded at the next step, they are unavailable.
    requested: Vec<usize>,
    /// The document as the run read it; any change ends the run.
    version: curvyo_document_core::DocumentVersion,
}

/// One recognised line as the host words it.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognisedLine {
    /// The text node.
    pub text: NodeId,
    /// The string.
    pub string: String,
    /// The face's family and style, as listed.
    pub face: String,
    /// The tier the face is from.
    pub tier: FaceTier,
    /// Letters (spaces not counted).
    pub letters: usize,
    /// The guessed letters: character and its 1-based position in the string.
    pub guessed: Vec<(char, usize)>,
    /// The largest deviation of a sure letter, mm.
    pub deviation: f64,
}

/// What a step did.
#[derive(Debug, Clone, PartialEq)]
pub enum RecogniseOutcome {
    /// Nothing: no run, or the Select tool is not active.
    Ignored,
    /// The host loads the wanted font files and steps again.
    Loading,
    /// One line was searched in one tier.
    Running {
        /// Lines decided.
        done: usize,
        /// Lines found.
        total: usize,
    },
    /// The text was written in one step; the selection is what it produced.
    Applied {
        /// The lines, in order.
        lines: Vec<RecognisedLine>,
        /// Selected outlines left as paths.
        left: usize,
        /// Selected images, open paths and other objects left as they are.
        skipped: [usize; 3],
    },
    /// Nothing within the tolerance; the document is unchanged.
    Nothing {
        /// No line was found at all.
        no_lines: bool,
        /// The face that came nearest and its deviation.
        nearest: Option<(String, f64)>,
        /// The last tier searched.
        searched: Option<FaceTier>,
    },
}

impl RecogniseOutcome {
    /// The outcome as the host reads it: `{"kind": "ignored" | "loading" | "running" |
    /// "applied" | "nothing", ...}` with `done`/`total`, the lines (`string`, `face`, `tier`,
    /// `letters`, `guessed` as `[char, position]`, `deviation` in mm), `left` and `skipped`
    /// (`images`, `open_paths`, `others`), or `no_lines`, `nearest` (`face`, `deviation`) and
    /// `searched` (the last tier's code). The sentences are the host's (ADR 0001 §5).
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = match self {
            Self::Ignored => serde_json::json!({ "kind": "ignored" }),
            Self::Loading => serde_json::json!({ "kind": "loading" }),
            Self::Running { done, total } => {
                serde_json::json!({ "kind": "running", "done": done, "total": total })
            }
            Self::Applied {
                lines,
                left,
                skipped,
            } => serde_json::json!({
                "kind": "applied",
                "lines": lines.iter().map(|line| serde_json::json!({
                    "string": line.string,
                    "face": line.face,
                    "tier": line.tier.code(),
                    "letters": line.letters,
                    "guessed": line.guessed.iter().map(|(c, p)| serde_json::json!([c.to_string(), p])).collect::<Vec<_>>(),
                    "deviation": line.deviation,
                })).collect::<Vec<_>>(),
                "left": left,
                "skipped": {
                    "images": skipped[0],
                    "open_paths": skipped[1],
                    "others": skipped[2],
                },
            }),
            Self::Nothing {
                no_lines,
                nearest,
                searched,
            } => serde_json::json!({
                "kind": "nothing",
                "no_lines": no_lines,
                "nearest": nearest.as_ref().map(|(face, d)| serde_json::json!({ "face": face, "deviation": d })),
                "searched": searched.map(FaceTier::code),
            }),
        };
        value.to_string()
    }
}

/// The session record of guessed letters (`adrs.md` decision 6): per text, the character
/// positions; not in the document, dropped on that text's first edit, a selection change by the
/// maker, or undo.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GuessedRecord {
    /// Per text: its id, the guessed positions, and the string and the size's bits the run wrote
    /// (a change of either is an edit of that text).
    pub(crate) texts: Vec<(NodeId, Vec<usize>, String, u64)>,
    /// The selection the record was made with.
    pub(crate) selection: Vec<NodeId>,
}

/// The run and the record.
#[derive(Debug, Default)]
pub(crate) struct RecogniseState {
    pub(crate) running: Option<RunningRecognise>,
    pub(crate) guessed: GuessedRecord,
}

impl Session {
    fn recognise_skips(&self) -> impl Fn(NodeId) -> bool + use<> {
        let read = self.document_read();
        let (inert, guides) = (Arc::clone(&read.inert), Arc::clone(&read.guides));
        move |id| inert.contains(&id) || guides.contains(&id)
    }

    /// Whether the Select bar shows Recognise text: the Select tool is active and a selected leaf
    /// is a path with a closed outline, filled or not (UX notes §1).
    #[must_use]
    pub fn recognise_available(&self) -> bool {
        if self.tool != Tool::Select || self.selection.is_empty() {
            return false;
        }
        let leaves = Arc::clone(&self.document_read().leaves);
        recognise_available(&leaves, self.selection.ids(), &self.recognise_skips())
    }

    /// The fonts of the document's texts (tier 1).
    fn document_fonts(&self) -> Vec<FontRef> {
        let mut fonts: Vec<FontRef> = Vec::new();
        for object in self.document_read().leaves.iter() {
            if let ObjectSnapshot::Text(text) = object
                && !fonts.contains(&text.font)
            {
                fonts.push(text.font.clone());
            }
        }
        fonts
    }

    /// The tiers for the maker's favourite families (criterion 8).
    #[must_use]
    pub fn recognise_tiers(&self, favourites: &[String]) -> Tiers {
        recognise_tiers(self.fonts.faces(), &self.document_fonts(), favourites)
    }

    /// Starts a run over the selection with `tolerance` (criterion 6); false when nothing can be
    /// read. A run in flight is replaced.
    pub fn recognise_start(&mut self, tolerance: Mm, favourites: &[String]) -> bool {
        self.recognise.running = None;
        if !self.rail_command_ready() || !self.recognise_available() {
            return false;
        }
        let leaves = Arc::clone(&self.document_read().leaves);
        let subjects = recognise_subjects(&leaves, self.selection.ids(), &self.recognise_skips());
        let tiers = self.recognise_tiers(favourites);
        self.recognise.running = Some(RunningRecognise {
            job: RecogniseJob::new(&subjects.outlines, tolerance, tiers),
            subjects,
            selection: self.selection.ids().to_vec(),
            requested: Vec::new(),
            version: self.document.version(),
        });
        true
    }

    /// Stops the run in flight; the document was not touched (UX notes §3).
    pub fn recognise_stop(&mut self) {
        self.recognise.running = None;
    }

    /// Whether a run is in flight.
    #[must_use]
    pub fn recognise_running(&self) -> bool {
        self.recognise.running.is_some()
    }

    fn face_state(&self, index: usize, requested: &[usize]) -> FaceState {
        let Some(face) = self.fonts.faces().get(index) else {
            return FaceState::Unavailable;
        };
        match face
            .key
            .and_then(|key| self.fonts.store().get(&key).cloned())
        {
            Some(found) => FaceState::Ready(found),
            None if requested.contains(&index) => FaceState::Unavailable,
            None => FaceState::Loading,
        }
    }

    /// One step of the run: asks for faces, searches one line in one tier, or writes the result.
    pub fn recognise_step(&mut self) -> RecogniseOutcome {
        let Some(mut running) = self.recognise.running.take() else {
            return RecogniseOutcome::Ignored;
        };
        if !self.rail_command_ready()
            || running.selection != self.selection.ids()
            || running.version != self.document.version()
        {
            return RecogniseOutcome::Ignored;
        }
        let requested = running.requested.clone();
        let step = running
            .job
            .step(&|index| self.face_state(index, &requested));
        match step {
            JobStep::NeedFaces(faces) => {
                for &index in &faces {
                    self.fonts.request(index);
                }
                running.requested.extend(faces);
                self.recognise.running = Some(running);
                RecogniseOutcome::Loading
            }
            JobStep::Running { done, total } => {
                self.recognise.running = Some(running);
                RecogniseOutcome::Running { done, total }
            }
            JobStep::Finished => self.finish_recognise(&running),
        }
    }

    fn listed_name(&self, index: usize) -> String {
        self.fonts
            .faces()
            .get(index)
            .map(|f| format!("{} {}", f.family, f.style))
            .unwrap_or_default()
    }

    fn font_of_listed(&self, index: usize) -> Option<FontRef> {
        self.fonts.font_of(index)
    }

    fn finish_recognise(&mut self, running: &RunningRecognise) -> RecogniseOutcome {
        let job = &running.job;
        let subjects = &running.subjects;
        let skipped = [subjects.images, subjects.open_paths, subjects.others];
        let runs: Vec<_> = job
            .accepted()
            .iter()
            .filter_map(|line| {
                Some((
                    (line.run.clone(), self.font_of_listed(line.face)?),
                    (line.face, line.tier),
                ))
            })
            .collect();
        let leaves = Arc::clone(&self.document_read().leaves);
        let (plain, sources): (Vec<_>, Vec<_>) = runs.into_iter().unzip();
        let write = plan_recognise_write(&leaves, subjects, &plain);
        if write.texts.is_empty() {
            return RecogniseOutcome::Nothing {
                no_lines: job.lines() == 0,
                nearest: job.nearest().map(|(face, d)| (self.listed_name(face), d)),
                searched: job.last_tier(),
            };
        }
        let Some(ids) = self.write_recognised(&write) else {
            return RecogniseOutcome::Ignored;
        };
        let mut lines = Vec::new();
        let mut record = Vec::new();
        for (text, id) in write.texts.iter().zip(&ids) {
            let (face, tier) = sources[text.run];
            let run = &plain[text.run].0;
            let chars: Vec<char> = text.string.chars().collect();
            lines.push(RecognisedLine {
                text: *id,
                string: text.string.clone(),
                face: self.listed_name(face),
                tier,
                letters: chars.iter().filter(|c| **c != ' ').count(),
                guessed: text
                    .guessed
                    .iter()
                    .filter_map(|&p| chars.get(p).map(|&c| (c, p + 1)))
                    .collect(),
                deviation: run.max_deviation.get(),
            });
            if !text.guessed.is_empty() {
                record.push((
                    *id,
                    text.guessed.clone(),
                    text.string.clone(),
                    text.size.as_mm().to_bits(),
                ));
            }
        }
        self.recognise.guessed = GuessedRecord {
            texts: record,
            selection: self.selection.ids().to_vec(),
        };
        RecogniseOutcome::Applied {
            lines,
            left: write.left,
            skipped,
        }
    }

    /// Writes the texts and the leftover pieces in one step; selects what it produced. `None`
    /// when the write could not be made (nothing is changed then).
    fn write_recognised(&mut self, write: &RecogniseWrite) -> Option<Vec<NodeId>> {
        self.recording(|this| {
            let written = this.document.batch(RECOGNISE_TEXT_LABEL, |document| {
                let mut texts = Vec::new();
                for text in &write.texts {
                    let id = document
                        .create_text_with_chars(
                            &text.string,
                            &text.font,
                            curvyo_document_core::Length::from_mm(text.size.as_mm()),
                            text.placement,
                            &text.layout,
                            &TextChars::default(),
                        )
                        .ok()?;
                    for edit in style_edits(&text.style) {
                        document.edit_style(&[id], &edit).ok()?;
                    }
                    document
                        .move_nodes(&MovePlan {
                            nodes: vec![id],
                            target: MoveTarget::Above(text.source),
                        })
                        .ok()?;
                    texts.push(id);
                }
                let pieces = document
                    .replace_with_pieces(
                        &write.removed,
                        &write.pieces,
                        PieceFill::Keep,
                        RECOGNISE_TEXT_LABEL,
                    )
                    .ok()?;
                Some((texts, pieces))
            });
            let Some((texts, pieces)) = written else {
                // invariant: the plan was made from the objects just read; a failed write is
                // reverted by the history as a whole, so nothing half done stays.
                debug_assert!(false, "a planned Recognise text could not be written");
                return None;
            };
            this.command_refusal = None;
            this.node.clear_selection();
            let mut selected = texts.clone();
            selected.extend(pieces);
            this.selection.set(&selected);
            this.hovered_object = None;
            Some(texts)
        })
    }

    /// The guessed letters of text `id` while the session record holds them (criterion 7), as
    /// character positions: the text exists with the string the run wrote and the selection is
    /// still the run's or a part of it (a selection change by the maker, the first edit of its
    /// string or size, and undo end the record; lead decision 2026-10-31 for the size).
    #[must_use]
    pub fn recognise_guessed(&self, id: NodeId) -> Option<Vec<usize>> {
        let record = &self.recognise.guessed;
        let (_, positions, string, size) =
            record.texts.iter().find(|(text, _, _, _)| *text == id)?;
        let selected = self.selection.ids();
        let within = !selected.is_empty() && selected.iter().all(|s| record.selection.contains(s));
        let unchanged = self
            .document
            .text(id)
            .is_some_and(|t| t.string == *string && t.size.as_mm().to_bits() == *size);
        (within && unchanged).then(|| positions.clone())
    }

    /// The guessed characters of the Glyph table's subject (one selected text or the edited
    /// one), each once, in text order (criterion 7).
    #[must_use]
    pub fn recognise_guessed_chars(&self) -> Vec<char> {
        let id = match self
            .text
            .edit
            .as_ref()
            .map(curvyo_ui_core::TextEdit::target)
        {
            Some(curvyo_ui_core::EditTarget::Existing(id)) => id,
            _ => match self.selection.ids() {
                [id] => *id,
                _ => return Vec::new(),
            },
        };
        let Some(positions) = self.recognise_guessed(id) else {
            return Vec::new();
        };
        let Some(text) = self.document.text(id) else {
            return Vec::new();
        };
        let chars: Vec<char> = text.string.chars().collect();
        let mut out: Vec<char> = Vec::new();
        for p in positions {
            if let Some(&c) = chars.get(p)
                && !out.contains(&c)
            {
                out.push(c);
            }
        }
        out
    }

    /// The byte range of the first guessed letter of text `id`, selected when the Text tool
    /// opens it from the keyboard (criterion 7: the ordinary selection highlight).
    pub(crate) fn first_guessed_range(&self, id: NodeId) -> Option<std::ops::Range<usize>> {
        let first = *self.recognise_guessed(id)?.first()?;
        let text = self.document.text(id)?;
        let (start, c) = text.string.char_indices().nth(first)?;
        Some(start..start + c.len_utf8())
    }
}

/// The edits that give a new text `style` whole, stroke switch last (a stroke edit switches it
/// on).
fn style_edits(style: &curvyo_document_core::Style) -> Vec<StyleEdit> {
    vec![
        StyleEdit::FillRgba(style.fill.color, style.fill.opacity),
        StyleEdit::FillEnabled(style.fill.enabled),
        StyleEdit::StrokeRgba(style.stroke.color, style.stroke.opacity),
        StyleEdit::StrokeWidth(style.stroke.width),
        StyleEdit::StrokeJoin(style.stroke.join),
        StyleEdit::StrokeCap(style.stroke.cap),
        StyleEdit::StrokeDash(style.stroke.dash.clone()),
        StyleEdit::StrokeEnabled(style.stroke.enabled),
    ]
}
