//! The Text tool's `wasm-bindgen` surface (`specs/0074-text-objects`): the input sink's messages,
//! the Text bar, the Fonts tab, the fonts the host lists and hands in, and Object to path. Every
//! method is a pass-through to `Session`; strings and scalars only (ADR 0001 §5).

use curvyo_ui_core::{CaretMove, FontBlock, TextInput, TextRefusal};
use wasm_bindgen::prelude::*;

use crate::session::{ObjectToPathOutcome, TextSizeOutcome};
use crate::wasm_api::WasmSession;

/// The code of a refusal for texts that do not convert (criteria 24, 29): `"fonts_loading"`
/// when a face's bytes have not arrived, `"single_line_texts"` for single-line texts refused by
/// a command on areas (`0076` criterion 19), else `"missing_fonts"`.
pub(crate) const fn refusal_kind(refusal: &TextRefusal) -> &'static str {
    if refusal.no_area {
        "single_line_texts"
    } else if refusal.loading {
        "fonts_loading"
    } else {
        "missing_fonts"
    }
}

/// What the Text bar shows.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct TextBarStateView {
    /// Shown: the Text tool, or a selection with a text.
    pub shown: bool,
    /// "Source Sans 3 Regular"; empty for "Mixed".
    #[wasm_bindgen(getter_with_clone)]
    pub font: String,
    /// The face is missing on this computer.
    pub missing: bool,
    /// The size in mm; NaN for "Mixed".
    pub size_mm: f64,
    /// The one edited or selected text the font preview follows; empty for none.
    #[wasm_bindgen(getter_with_clone)]
    pub sample_node: String,
    /// That text's first line, at most 200 characters.
    #[wasm_bindgen(getter_with_clone)]
    pub sample: String,
}

/// What Object to path did: `kind` is `"ignored"`, `"converted"`, `"missing_fonts"` (`count`
/// texts; `face` names the face when there is one) or `"fonts_loading"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ObjectToPathView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The texts to blame.
    pub count: u32,
    /// The one missing face, "Source Serif 4 Bold"; empty for several.
    #[wasm_bindgen(getter_with_clone)]
    pub face: String,
}

fn caret_move(code: &str) -> Option<CaretMove> {
    Some(match code {
        "left" => CaretMove::Left,
        "right" => CaretMove::Right,
        "up" => CaretMove::Up,
        "down" => CaretMove::Down,
        "home" => CaretMove::LineStart,
        "end" => CaretMove::LineEnd,
        "word-left" => CaretMove::WordLeft,
        "word-right" => CaretMove::WordRight,
        _ => return None,
    })
}

fn block(code: &str) -> Option<FontBlock> {
    Some(match code {
        "curvyo" => FontBlock::Curvyo,
        "included" => FontBlock::Included,
        "system" => FontBlock::System,
        _ => return None,
    })
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether a text is in edit mode: the host keeps focus in the hidden input sink.
    #[must_use]
    pub fn text_editing(&self) -> bool {
        self.session.text_editing()
    }

    /// Typed, pasted or composed characters at the caret. Returns whether the character limit
    /// cut them (the notice of criterion 31).
    pub fn text_insert(&mut self, text: &str) -> bool {
        self.session
            .text_input(TextInput::Insert(text.to_owned()))
            .limited
    }

    /// Backspace (`backward`) or Delete.
    pub fn text_delete(&mut self, backward: bool) {
        self.session.text_input(if backward {
            TextInput::DeleteBackward
        } else {
            TextInput::DeleteForward
        });
    }

    /// A caret move: `"left"`, `"right"`, `"up"`, `"down"`, `"home"`, `"end"`, `"word-left"`,
    /// `"word-right"`; `extend` with Shift.
    pub fn text_move(&mut self, code: &str, extend: bool) {
        if let Some(to) = caret_move(code) {
            self.session.text_input(TextInput::Move { to, extend });
        }
    }

    /// Ctrl+A in edit mode.
    pub fn text_select_all(&mut self) {
        self.session.text_input(TextInput::SelectAll);
    }

    /// The IME composition so far and its caret (UTF-16 offset in it); empty ends it.
    pub fn text_preedit(&mut self, text: &str, caret_utf16: u32) {
        let mut units = 0_u32;
        let caret = text
            .char_indices()
            .find(|(_, c)| {
                let before = units;
                units += u32::try_from(c.len_utf16()).unwrap_or(1);
                before >= caret_utf16
            })
            .map_or(text.len(), |(i, _)| i);
        self.session.text_input(TextInput::Preedit {
            text: text.to_owned(),
            caret,
        });
    }

    /// Ctrl+Z (`redo` false) or Ctrl+Shift+Z inside edit mode.
    pub fn text_undo(&mut self, redo: bool) {
        self.session.text_input(if redo {
            TextInput::Redo
        } else {
            TextInput::Undo
        });
    }

    /// The selected characters (Copy and Cut; Cut then deletes them).
    #[must_use]
    pub fn text_selected(&self) -> String {
        self.session.text_selected()
    }

    /// Takes the notice that edit mode ended without its text being written.
    pub fn take_text_write_failed(&mut self) -> bool {
        self.session.take_text_write_failed()
    }

    /// Takes the notice "A text holds at most 10000 characters."
    pub fn take_text_limit_notice(&mut self) -> bool {
        self.session.take_text_limit_notice()
    }

    /// Ends edit mode as its one step; returns whether it was on.
    pub fn finish_text_edit(&mut self) -> bool {
        self.session.finish_text_edit()
    }

    /// Writes the session as its step and edit mode goes on (Save in edit mode).
    pub fn checkpoint_text_edit(&mut self) {
        self.session.checkpoint_text_edit();
    }

    /// The caret's blink phase.
    pub fn set_text_caret_on(&mut self, on: bool) {
        self.session.set_text_caret_on(on);
    }

    /// The caret's canvas point in CSS px, `[x, y]`, or empty outside edit mode: where the sink
    /// sits for the IME window.
    #[must_use]
    pub fn text_caret_point(&self) -> Vec<f64> {
        self.session
            .text_caret_screen_point()
            .map(|(x, y)| vec![x, y])
            .unwrap_or_default()
    }

    /// The edited string and its selection (scalar positions) as JSON `{"text", "start",
    /// "end"}`, or `"null"`: what the sink mirrors.
    #[must_use]
    pub fn text_sink_json(&self) -> String {
        self.session.text_sink_state().map_or_else(
            || "null".to_owned(),
            |(text, start, end)| {
                serde_json::json!({ "text": text, "start": start, "end": end }).to_string()
            },
        )
    }

    /// What the Text bar shows.
    #[must_use]
    pub fn text_bar(&self) -> TextBarStateView {
        let bar = self.session.text_bar();
        TextBarStateView {
            shown: bar.shown,
            font: bar.font,
            missing: bar.missing,
            size_mm: bar.size_mm.unwrap_or(f64::NAN),
            sample_node: bar.sample_node,
            sample: bar.sample,
        }
    }

    /// The Size field's value: `"set"`, `"max"`, `"min"` (the notices of criterion 12) or
    /// `"invalid"`.
    pub fn set_text_size(&mut self, mm: f64) -> String {
        match self.session.set_text_size(mm) {
            TextSizeOutcome::Set => "set",
            TextSizeOutcome::ClampedToMax => "max",
            TextSizeOutcome::ClampedToMin => "min",
            TextSizeOutcome::Invalid => "invalid",
        }
        .to_owned()
    }

    /// The Size field's live value while an arrow key is held; NaN ends it.
    pub fn preview_text_size(&mut self, mm: f64) {
        self.session.preview_text_size(mm.is_finite().then_some(mm));
    }

    /// A pick in the Fonts tab: the face `face_index` of the listed file `id`.
    pub fn pick_font(&mut self, id: &str, face_index: u32) {
        self.session.pick_font_face(id, face_index);
    }

    /// The faces the host listed for `block` (`"curvyo"`, `"included"`, `"system"`), as JSON:
    /// `{"faces": [{"id", "family", "style", "index", "kind"?}], "skipped": ["name"]}`; `kind` is
    /// `"single_line"` for a single-line face, else absent.
    pub fn set_font_list(&mut self, block_code: &str, json: &str) {
        let Some(block) = block(block_code) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
            return;
        };
        let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_owned();
        let faces = value["faces"]
            .as_array()
            .map(|faces| {
                faces
                    .iter()
                    .map(|face| {
                        (
                            text(&face["id"]),
                            text(&face["family"]),
                            text(&face["style"]),
                            face["index"]
                                .as_u64()
                                .and_then(|i| u32::try_from(i).ok())
                                .unwrap_or(0),
                            if face["kind"].as_str() == Some("single_line") {
                                curvyo_document_core::FontKind::SingleLine
                            } else {
                                curvyo_document_core::FontKind::Outline
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let skipped = value["skipped"]
            .as_array()
            .map(|names| names.iter().map(text).collect())
            .unwrap_or_default();
        self.session.set_font_list_with_kinds(block, faces, skipped);
    }

    /// The host ids of the font files to read and hand in with [`WasmSession::add_font_file`].
    #[must_use]
    pub fn wanted_font_files(&self) -> Vec<String> {
        self.session.wanted_font_files()
    }

    /// The bytes of the listed file `id`, once per file per session (criterion 33). Returns
    /// whether a face of it could be read.
    pub fn add_font_file(&mut self, id: &str, bytes: &[u8]) -> bool {
        self.session.add_font_file(id, bytes)
    }

    /// What the Fonts tab lists and marks, as JSON (see `Session::fonts_tab_json`).
    #[must_use]
    pub fn fonts_tab_json(&self) -> String {
        self.session.fonts_tab_json()
    }

    /// Object to path of the selection (criterion 22), with its refusal (criterion 29).
    pub fn object_to_path(&mut self) -> ObjectToPathView {
        let view = |kind: &str, count: usize, face: String| ObjectToPathView {
            kind: kind.to_owned(),
            count: u32::try_from(count).unwrap_or(u32::MAX),
            face,
        };
        match self.session.object_to_path() {
            ObjectToPathOutcome::Ignored => view("ignored", 0, String::new()),
            ObjectToPathOutcome::Converted => view("converted", 0, String::new()),
            ObjectToPathOutcome::Refused(refusal) => {
                let face = match refusal.missing_faces.as_slice() {
                    [one] => one.clone(),
                    _ => String::new(),
                };
                view(refusal_kind(&refusal), refusal.offenders.len(), face)
            }
        }
    }

    /// The face named in a path command's refusal of texts, "Source Serif 4 Bold", or empty
    /// for several: the selection is still the refused one.
    #[must_use]
    pub fn text_refusal_face(&self) -> String {
        self.session.text_refusal_face()
    }

    /// The canvas points of the "Font missing" tags, flat `[x, y, x, y, ...]` in CSS px.
    #[must_use]
    pub fn missing_font_marks(&self) -> Vec<f64> {
        self.session
            .missing_font_marks()
            .into_iter()
            .flat_map(|(x, y)| [x, y])
            .collect()
    }
}
