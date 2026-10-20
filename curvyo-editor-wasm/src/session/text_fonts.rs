//! Resolves a text's font reference to a face of the session (`specs/0074-text-objects` criteria
//! 16, 29, 30, 33; ADR 0001 §5): the listed faces with the engine's store of those handed in, the
//! ones still wanted, and the layout in the resolved face.

use std::cell::RefCell;

use curvyo_document_core::{FontRef, TextOutline, TextSnapshot};
use curvyo_text_core::{FaceKey, FaceStore};
use curvyo_ui_core::{FaceMatch, FontBlock, ListedFace, lay_out, match_face};

/// The face a text whose face is missing is drawn in (criterion 29), and the first face of a new
/// text (criterion 14): the included sans, Regular.
pub(crate) const FALLBACK_FAMILY: &str = "Source Sans 3";
/// See [`FALLBACK_FAMILY`].
pub(crate) const FALLBACK_STYLE: &str = "Regular";

/// The fonts of the session.
#[derive(Debug, Default)]
pub(crate) struct TextFonts {
    store: FaceStore,
    faces: Vec<ListedFace>,
    /// Folder files that are not readable fonts, for the block header (criterion 19).
    skipped: Vec<String>,
    /// Whether the host has delivered the system list (the header shows "Reading…" until then).
    system_read: bool,
    /// Bumped whenever the faces or the store change: the object cache lays out again.
    generation: u64,
    /// The listed faces a layout asked for that are not loaded, by their index in `faces`.
    wanted: RefCell<Vec<usize>>,
}

impl TextFonts {
    /// What changes the layout of a text: compared by the object cache.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// The listed faces.
    pub(crate) fn faces(&self) -> &[ListedFace] {
        &self.faces
    }

    pub(crate) fn skipped(&self) -> &[String] {
        &self.skipped
    }

    pub(crate) const fn system_read(&self) -> bool {
        self.system_read
    }

    /// Replaces the faces of `block` (a fresh listing of the folder or the system), keeping the
    /// keys of faces already loaded.
    pub(crate) fn set_block(
        &mut self,
        block: FontBlock,
        listed: Vec<(String, String, String, u32)>,
        skipped: Vec<String>,
    ) {
        let old: Vec<ListedFace> = self
            .faces
            .iter()
            .filter(|f| f.block == block)
            .cloned()
            .collect();
        let same = old.len() == listed.len()
            && old
                .iter()
                .zip(&listed)
                .all(|(face, (host_id, family, style, index))| {
                    face.host_id == *host_id
                        && face.family == *family
                        && face.style == *style
                        && face.index == *index
                });
        // A listing that changes nothing changes no index and lays nothing out again.
        if same && (block != FontBlock::Curvyo || self.skipped == skipped) {
            if block == FontBlock::System {
                self.system_read = true;
            }
            return;
        }
        self.faces.retain(|face| face.block != block);
        for (host_id, family, style, index) in listed {
            let key = old
                .iter()
                .find(|f| f.host_id == host_id && f.index == index)
                .and_then(|f| f.key);
            self.faces.push(ListedFace {
                block,
                family,
                style,
                host_id,
                index,
                key,
            });
        }
        // The blocks stay in their order (Curvyo, Included, This computer), so a fresh listing of
        // one block moves no face of another.
        self.faces.sort_by_key(|face| face.block);
        if block == FontBlock::Curvyo {
            self.skipped = skipped;
        }
        if block == FontBlock::System {
            self.system_read = true;
        }
        self.wanted.borrow_mut().clear();
        self.generation += 1;
    }

    /// Hands in the bytes of a listed file: every listed face of that file is parsed and keyed.
    /// Returns false when no face of it could be read (it then stays unloaded; a hostile file
    /// never panics, criterion 32).
    pub(crate) fn add_file(&mut self, host_id: &str, bytes: &[u8]) -> bool {
        let mut any = false;
        for face in self.faces.iter_mut().filter(|f| f.host_id == host_id) {
            if let Ok(key) = self.store.add(bytes.to_vec(), face.index) {
                face.key = Some(key);
                any = true;
            }
        }
        self.wanted.borrow_mut().clear();
        self.generation += 1;
        any
    }

    /// The host ids of the files a layout asked for and that are not loaded, each once.
    pub(crate) fn wanted_files(&self) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        for &index in self.wanted.borrow().iter() {
            if let Some(face) = self.faces.get(index)
                && face.key.is_none()
                && !files.contains(&face.host_id)
            {
                files.push(face.host_id.clone());
            }
        }
        files
    }

    fn want(&self, index: usize) {
        let mut wanted = self.wanted.borrow_mut();
        if !wanted.contains(&index) {
            wanted.push(index);
        }
    }

    /// How `font` resolves now (criteria 29, 30); asks for the face's bytes when it is listed but
    /// not loaded.
    pub(crate) fn resolve(&self, font: &FontRef) -> FaceMatch {
        let found = match_face(font, &self.faces);
        if let FaceMatch::NeedsLoad(index) = found {
            self.want(index);
        }
        found
    }

    /// The included sans Regular, as a font reference once it is loaded.
    pub(crate) fn fallback(&self) -> Option<(FaceKey, FontRef)> {
        let index = self.faces.iter().position(|face| {
            face.block == FontBlock::Included
                && face.family == FALLBACK_FAMILY
                && face.style == FALLBACK_STYLE
        })?;
        let font = self.font_of(index);
        if font.is_none() {
            self.want(index);
        }
        Some((self.faces[index].key?, font?))
    }

    /// The listed face of file `host_id` at `face_index` in it: a pick names the face, not a
    /// position that a fresh listing can move.
    pub(crate) fn position(&self, host_id: &str, face_index: u32) -> Option<usize> {
        self.faces
            .iter()
            .position(|face| face.host_id == host_id && face.index == face_index)
    }

    /// The font reference of the listed face `index`, once loaded (a pick in the Fonts tab).
    pub(crate) fn font_of(&self, index: usize) -> Option<FontRef> {
        let face = self.faces.get(index)?;
        let key = face.key?;
        Some(FontRef {
            family: face.family.clone(),
            style: face.style.clone(),
            face_hash: key.hash.to_hex(),
            face_index: key.index,
        })
    }

    /// Asks for the bytes of the listed face `index` (a pick of a face not loaded yet).
    pub(crate) fn request(&self, index: usize) {
        if self.faces.get(index).is_some_and(|face| face.key.is_none()) {
            self.want(index);
        }
    }

    /// The face's engine data for `key`.
    pub(crate) fn face(&self, key: &FaceKey) -> Option<&curvyo_text_core::Face> {
        self.store.get(key).map(AsRef::as_ref)
    }

    /// `text` laid out: in its face, or, when that is missing, in the included sans and marked as
    /// a substitute; `None` while a face it needs is loading.
    pub(crate) fn lay_out(&self, text: &TextSnapshot) -> Option<TextOutline> {
        match self.resolve(&text.font) {
            FaceMatch::Face(key) => lay_out(text, self.face(&key)?, false),
            FaceMatch::NeedsLoad(_) => None,
            FaceMatch::Missing => {
                let (key, _) = self.fallback()?;
                lay_out(text, self.face(&key)?, true)
            }
        }
    }

    /// Whether `font` resolves to no listed face (the text is drawn with the substitute and
    /// marked, criterion 29).
    pub(crate) fn is_missing(&self, font: &FontRef) -> bool {
        match_face(font, &self.faces) == FaceMatch::Missing
    }
}
