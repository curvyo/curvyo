//! `Session`'s glue for the Text bar and the Fonts tab (`specs/0074-text-objects` criteria 11 to
//! 19): what they show, a font pick, a size, and the font lists and files the host hands in.

use curvyo_document_core::{FontRef, Length, MAX_TEXT_SIZE, MIN_TEXT_SIZE, NodeId, TextSnapshot};
use curvyo_ui_core::FaceMatch;

use super::text::{NEW_TEXT_ID, edit_id};
use super::{Session, Tool};

/// What the Text bar (Text tool) or the Select bar's Text group shows.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBarView {
    /// Whether it is shown: the Text tool, or a selection that holds a text.
    pub shown: bool,
    /// The face name, "Source Sans 3 Regular"; empty when several faces differ.
    pub font: String,
    /// The face is missing on this computer (criterion 29).
    pub missing: bool,
    /// The size in mm; `None` when several sizes differ ("Mixed").
    pub size_mm: Option<f64>,
}

/// What a typed or stepped size did (criterion 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSizeOutcome {
    /// Set as typed.
    Set,
    /// Set to the largest size: "Size set to 2000 mm, the largest size."
    ClampedToMax,
    /// Set to the smallest size: "Size set to 0.1 mm, the smallest size."
    ClampedToMin,
    /// Not a number.
    Invalid,
}

impl Session {
    /// Replaces the faces the host listed for `block` (the Curvyo folder with its unreadable
    /// files, the included fonts, the system fonts): `(host id, family, style, index)`, all
    /// outline faces.
    pub fn set_font_list(
        &mut self,
        block: curvyo_ui_core::FontBlock,
        faces: Vec<(String, String, String, u32)>,
        skipped: Vec<String>,
    ) {
        let faces = faces
            .into_iter()
            .map(|(id, family, style, index)| {
                (
                    id,
                    family,
                    style,
                    index,
                    curvyo_document_core::FontKind::Outline,
                )
            })
            .collect();
        self.fonts.set_block(block, faces, skipped);
    }

    /// [`Session::set_font_list`] with each face's kind (`specs/0076-single-line-fonts`
    /// criterion 1): `(host id, family, style, index, kind)`.
    pub fn set_font_list_with_kinds(
        &mut self,
        block: curvyo_ui_core::FontBlock,
        faces: Vec<(String, String, String, u32, curvyo_document_core::FontKind)>,
        skipped: Vec<String>,
    ) {
        self.fonts.set_block(block, faces, skipped);
    }

    /// The host ids of the font files the session needs and has not got: the host reads each
    /// and hands it to [`Session::add_font_file`] (once per face per session, criterion 33).
    #[must_use]
    pub fn wanted_font_files(&self) -> Vec<String> {
        // A read lays every text out, which notes the faces it misses.
        let _ = self.render_objects();
        let _ = self.next_text_font();
        let _ = self.laid_out_edit();
        self.fonts.wanted_files()
    }

    /// What the Fonts tab lists and marks (criteria 15 to 19, 29), as JSON: every listed face
    /// with its index, block, family and style; the unreadable folder files; whether the system
    /// list has arrived; and the current face (of the edited or selected texts, or of the next
    /// text), whether it is missing, mixed, or the next text's.
    #[must_use]
    pub fn fonts_tab_json(&self) -> String {
        let texts = self.bar_texts();
        let next = texts.is_empty();
        let fonts: Vec<FontRef> = if next {
            self.next_text_font().into_iter().collect()
        } else {
            texts.into_iter().map(|t| t.font).collect()
        };
        let mixed = fonts
            .windows(2)
            .any(|w| w[0].family != w[1].family || w[0].style != w[1].style);
        let current = fonts.first().filter(|_| !mixed).map(|font| {
            serde_json::json!({
                "family": font.family,
                "style": font.style,
                "missing": self.fonts.is_missing(font),
                "index": self.current_face_index(font),
                // The face a missing one is drawn in (`0076` criterion 22, UX notes §6).
                "substitute": match font.kind {
                    curvyo_document_core::FontKind::Outline => super::text_fonts::FALLBACK_FAMILY,
                    curvyo_document_core::FontKind::SingleLine => {
                        super::text_fonts::SINGLE_LINE_FALLBACK_FAMILY
                    }
                },
            })
        });
        let block = |block: curvyo_ui_core::FontBlock| match block {
            curvyo_ui_core::FontBlock::Curvyo => "curvyo",
            curvyo_ui_core::FontBlock::Included => "included",
            curvyo_ui_core::FontBlock::System => "system",
        };
        let faces: Vec<serde_json::Value> = self
            .fonts
            .faces()
            .iter()
            .enumerate()
            .map(|(index, face)| {
                serde_json::json!({
                    "index": index,
                    "block": block(face.block),
                    "family": face.family,
                    "style": face.style,
                    "id": face.host_id,
                    "loaded": face.key.is_some(),
                    "faceIndex": face.index,
                    "singleLine": face.kind == curvyo_document_core::FontKind::SingleLine,
                })
            })
            .collect();
        serde_json::json!({
            "faces": faces,
            "skipped": self.fonts.skipped(),
            "systemRead": self.fonts.system_read(),
            "current": current,
            "mixed": mixed,
            "next": next,
        })
        .to_string()
    }

    /// How many texts of the open project use a face of the family `family` from the Curvyo
    /// fonts block: the Remove prompt's "2 texts here use it" (`specs/0075-font-library`
    /// criterion 25).
    #[must_use]
    pub fn texts_using_family(&self, family: &str) -> usize {
        let key = curvyo_ui_core::family_key(family);
        self.document
            .object_ids()
            .into_iter()
            .filter_map(|id| self.document.text(id))
            .filter(|text| {
                self.current_face_index(&text.font)
                    .and_then(|index| self.fonts.faces().get(index))
                    .is_some_and(|face| {
                        face.block == curvyo_ui_core::FontBlock::Curvyo
                            && curvyo_ui_core::family_key(&face.family) == key
                    })
            })
            .count()
    }

    /// The listed face `font` resolves to, for the current mark.
    fn current_face_index(&self, font: &FontRef) -> Option<usize> {
        match curvyo_ui_core::match_face(font, self.fonts.faces()) {
            FaceMatch::Face(key) => self.fonts.faces().iter().position(|f| f.key == Some(key)),
            FaceMatch::NeedsLoad(index) => Some(index),
            FaceMatch::Missing => None,
        }
    }

    /// The texts the Text bar or the Select bar's Text group act on: the edited one, else the
    /// selected texts.
    fn bar_texts(&self) -> Vec<TextSnapshot> {
        if let Some(edit) = self.text.edit.as_ref() {
            return vec![edit.snapshot(edit_id(edit).unwrap_or(NEW_TEXT_ID))];
        }
        self.selection
            .ids()
            .iter()
            .filter_map(|id| self.document.text(*id))
            .collect()
    }

    /// What the Text bar shows (criterion 11).
    #[must_use]
    pub fn text_bar(&self) -> TextBarView {
        let texts = self.bar_texts();
        let shown = self.tool == Tool::Text || !texts.is_empty();
        let (font, size) = match texts.split_first() {
            Some((first, rest)) => (
                rest.iter()
                    .all(|t| t.font.family == first.font.family && t.font.style == first.font.style)
                    .then(|| first.font.clone()),
                rest.iter()
                    .all(|t| t.size == first.size)
                    .then_some(first.size),
            ),
            None => (self.next_text_font(), Some(self.next_text_size())),
        };
        let size = self.text.size_preview.or(size);
        TextBarView {
            shown,
            font: font
                .as_ref()
                .map(|f| format!("{} {}", f.family, f.style))
                .unwrap_or_default(),
            missing: font.as_ref().is_some_and(|f| self.fonts.is_missing(f)),
            size_mm: size.map(Length::as_mm),
        }
    }

    /// A pick in the Fonts tab (criteria 13, 18): the listed face `index` becomes the face of
    /// the edited or selected texts (one step "Change font") and of the next text. A face that is
    /// not loaded yet is applied when its bytes arrive.
    pub fn pick_font(&mut self, index: usize) {
        if let Some(face) = self.fonts.faces().get(index) {
            let (host_id, face_index) = (face.host_id.clone(), face.index);
            self.pick_font_face(&host_id, face_index);
        }
    }

    /// [`Session::pick_font`] by the face's file and its index in it, which a fresh listing does
    /// not move.
    pub fn pick_font_face(&mut self, host_id: &str, face_index: u32) {
        self.recording(|this| {
            let Some(index) = this.fonts.position(host_id, face_index) else {
                return;
            };
            let Some(font) = this.fonts.font_of(index) else {
                this.fonts.request(index);
                this.text.pending_pick = Some((host_id.to_owned(), face_index));
                return;
            };
            this.text.pending_pick = None;
            this.text.next_font = Some(font.clone());
            if let Some(edit) = this.text.edit.as_mut() {
                edit.font = font.clone();
            }
            let ids = this.bar_text_ids();
            if !ids.is_empty() {
                let _ = this.document.set_text_font(&ids, &font);
            }
        });
    }

    /// The existing texts the bar writes to.
    fn bar_text_ids(&self) -> Vec<NodeId> {
        match self.text.edit.as_ref() {
            Some(edit) => edit_id(edit).into_iter().collect(),
            None => self
                .selection
                .ids()
                .iter()
                .copied()
                .filter(|id| self.document.text(*id).is_some())
                .collect(),
        }
    }

    /// The Size field's typed or stepped value in mm (criteria 12, 13): held to 0.1 to 2,000 mm,
    /// one step "Change text size" for the edited or selected texts, and the size of the next
    /// text.
    pub fn set_text_size(&mut self, mm: f64) -> TextSizeOutcome {
        if !mm.is_finite() {
            return TextSizeOutcome::Invalid;
        }
        self.recording(|this| {
            this.text.size_preview = None;
            let size = curvyo_document_core::clamp_text_size(Length::from_mm(mm));
            let outcome = if mm > MAX_TEXT_SIZE.as_mm() {
                TextSizeOutcome::ClampedToMax
            } else if mm < MIN_TEXT_SIZE.as_mm() {
                TextSizeOutcome::ClampedToMin
            } else {
                TextSizeOutcome::Set
            };
            this.text.next_size = Some(size);
            if let Some(edit) = this.text.edit.as_mut() {
                edit.size = size;
            }
            let ids = this.bar_text_ids();
            if !ids.is_empty() {
                let _ = this.document.set_text_size(&ids, size);
            }
            outcome
        })
    }

    /// The Size field's live value while an arrow key is held: drawn, not written; `None` ends
    /// it (Escape restores).
    pub fn preview_text_size(&mut self, mm: Option<f64>) {
        self.text.size_preview = mm
            .filter(|mm| mm.is_finite())
            .map(|mm| curvyo_document_core::clamp_text_size(Length::from_mm(mm)));
    }

    /// Hands in the bytes of a listed font file; a pick waiting for it is applied.
    pub fn add_font_file(&mut self, host_id: &str, bytes: &[u8]) -> bool {
        let added = self.fonts.add_file(host_id, bytes);
        if let Some((pending, face_index)) = self.text.pending_pick.clone()
            && pending == host_id
        {
            self.pick_font_face(&pending, face_index);
        }
        added
    }
}
