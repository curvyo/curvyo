//! The faces a code's human-readable text is set with (ADR 0092 §3): the glyph tables of the
//! session, the registry the generation funnel reads, the "Loading font..." precondition and the
//! Symbology switch.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{FontRef, NodeId};
use curvyo_extension_core::{GlyphTable, ParamKind, ParamValue, StoredParams};
use curvyo_text_core::FaceKey;
use curvyo_ui_core::code_glyphs::{GlyphRegistry, glyph_table, set_active_glyphs};
use curvyo_ui_core::instance;

use crate::session::{Session, Tool};

impl Session {
    /// The face a `Font` value is set in: the named face, or the included sans for `None` and for
    /// a face missing on this computer; `None` while that face is still loading (and asked for).
    fn code_face(&self, font: Option<&FontRef>) -> Option<FaceKey> {
        use curvyo_ui_core::FaceMatch;
        match font.map(|font| self.fonts.resolve(font)) {
            Some(FaceMatch::Face(key)) => Some(key),
            Some(FaceMatch::NeedsLoad(_)) => None,
            Some(FaceMatch::Missing) | None => self
                .fonts
                .fallback(curvyo_document_core::FontKind::Outline)
                .map(|(key, _)| key),
        }
    }

    /// The glyph table of `key`, built once per face.
    fn code_table(&self, key: FaceKey) -> Option<Arc<GlyphTable>> {
        let at = (key.hash.to_hex(), key.index);
        if let Some(table) = self.generators.glyph_tables.borrow().get(&at) {
            return Some(Arc::clone(table));
        }
        let table = Arc::new(glyph_table(self.fonts.face(&key)?));
        self.generators
            .glyph_tables
            .borrow_mut()
            .insert(at, Arc::clone(&table));
        Some(table)
    }

    /// The stored `Font` value of a code: `None` for an object without a font row, `Some(None)` for
    /// the included sans.
    #[allow(clippy::option_option)] // outer: has a font row; inner: the included sans when `None`
    fn code_font_of(&self, id: NodeId) -> Option<Option<FontRef>> {
        // The kind and the parameters only: the outline of a large code is never decoded here.
        let stored = self.document.generated_params(id)?;
        let found = instance::find_kind(&self.manufacturing.extensions, &stored.kind)?;
        let param = found
            .kind
            .params()
            .find(|param| matches!(param.kind, ParamKind::Font))?;
        match param.read(&stored.params) {
            ParamValue::Font(font) => Some(font),
            _ => None,
        }
    }

    /// Makes the tables of the selected codes' faces and of the included sans the ones the
    /// generation funnel reads; with `force` also when no code is selected (a placing, a switch).
    pub(in crate::session) fn publish_code_glyphs(&self, force: bool) {
        let mut fonts: Vec<Option<FontRef>> = self
            .selected_generated()
            .into_iter()
            .filter_map(|id| self.code_font_of(id))
            .collect();
        fonts.extend(self.generators.extra_font.clone().map(Some));
        if !force && fonts.is_empty() && self.tool != Tool::Generators {
            return;
        }
        let default = self.code_face(None).and_then(|key| self.code_table(key));
        let mut faces = HashMap::new();
        for font in fonts.into_iter().flatten() {
            if let Some(table) = self
                .code_face(Some(&font))
                .and_then(|key| self.code_table(key))
            {
                faces.insert((font.face_hash, font.face_index), table);
            }
        }
        set_active_glyphs(GlyphRegistry { default, faces });
    }

    /// Whether the face the code `id` is set in is not loaded yet: its rows, handles and Symbology
    /// group wait ("Loading font...", ADR 0092 §3.4).
    pub(in crate::session) fn code_loading(&self, id: NodeId) -> bool {
        self.code_font_of(id)
            .is_some_and(|font| self.code_face(font.as_ref()).is_none())
    }

    /// Whether a kind with a `Font` row would be placed or switched to while the included sans is
    /// not loaded.
    pub(in crate::session) fn included_sans_loading(&self, kind: &str) -> bool {
        let has_font =
            instance::find_kind(&self.manufacturing.extensions, kind).is_some_and(|found| {
                found
                    .kind
                    .params()
                    .any(|p| matches!(p.kind, ParamKind::Font))
            });
        has_font && self.code_face(None).is_none()
    }

    /// Whether a selected code waits for its face.
    pub(in crate::session) fn selected_code_loading(&self) -> bool {
        self.selected_generated()
            .into_iter()
            .any(|id| self.code_loading(id))
    }

    /// Whether the face the code `id` names is missing on this computer (its Font row shows the
    /// missing look and an edit sets the text in the included sans).
    pub(in crate::session) fn code_font_missing(&self, id: NodeId) -> bool {
        self.code_font_of(id)
            .flatten()
            .is_some_and(|font| self.fonts.is_missing(&font))
    }

    /// The Symbology group's press: switches the selected code to the kind `kind`
    /// (`<extension>/<kind>` of its family) in one step "Change QR code to EAN-13" and remembers it
    /// as the one the Code segment places. `false` when nothing was switched.
    pub fn switch_generated_kind(&mut self, kind: &str) -> bool {
        let selected = self.selected_generated();
        let [id] = selected[..] else {
            return false;
        };
        let Some(stored) = self.document.generated_params(id) else {
            return false;
        };
        if self.selection.ids().len() != 1
            || self.code_loading(id)
            || self.included_sans_loading(kind)
        {
            return false;
        }
        let (Some(from), Some(to)) = (self.kind_label(&stored.kind), self.kind_label(kind)) else {
            return false;
        };
        self.publish_code_glyphs(true);
        let name = format!("Change {} to {}", instance::noun(from), instance::noun(to));
        let outcome = self.recording_named_guarded(Some(name), |this| {
            instance::switch_kind(&this.document, &this.manufacturing.extensions, id, kind).ok()
        });
        let Some(switched) = outcome else {
            return false;
        };
        self.remember_family_kind(&switched.kind);
        if let Some(found) = self.document.generated_params(id) {
            let params: StoredParams = found.params;
            self.generators
                .refusals
                .borrow_mut()
                .insert(id, (params, switched.refused));
        }
        true
    }

    /// A pick in the Fonts tab for the code selected alone: writes its `Font` row in one step
    /// "EAN-13: Font". A single-line face is refused (`false` and nothing changes, the Fonts tab
    /// words it); `false` also when no code with a font row is selected.
    pub(in crate::session) fn set_code_font(&mut self, font: &FontRef) -> bool {
        let selected = self.selected_generated();
        let [id] = selected[..] else {
            return false;
        };
        if self.selection.ids().len() != 1 || self.code_font_of(id).is_none() {
            return false;
        }
        if font.kind == curvyo_document_core::FontKind::SingleLine {
            return true;
        }
        let Some(label) = self
            .document
            .generated_params(id)
            .and_then(|stored| self.kind_label(&stored.kind))
        else {
            return false;
        };
        let stored = ParamValue::Font(Some(font.clone()));
        let name = format!("{label}: Font");
        self.generators.extra_font = Some(font.clone());
        self.publish_code_glyphs(false);
        let _ = self.recording_named_guarded(Some(name), |this| {
            instance::change_value(
                &this.document,
                &this.manufacturing.extensions,
                id,
                "font",
                &stored,
            )
            .ok()
        });
        self.generators.extra_font = None;
        true
    }

    /// Remembers `kind` as the last of its family, for the Code segment.
    pub(in crate::session) fn remember_family_kind(&mut self, kind: &str) {
        let family = instance::find_kind(&self.manufacturing.extensions, kind)
            .and_then(|found| found.kind.family);
        let Some(family) = family else {
            return;
        };
        kind.clone_into(&mut self.generators.family_kind);
        let current_family =
            instance::find_kind(&self.manufacturing.extensions, &self.generators.kind)
                .and_then(|found| found.kind.family);
        if current_family == Some(family) {
            kind.clone_into(&mut self.generators.kind);
        }
    }
}
