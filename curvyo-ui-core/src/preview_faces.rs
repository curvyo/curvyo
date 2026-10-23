//! Keeps the faces the font preview draws within a byte budget, the least recently drawn dropped
//! first (`specs/0077-font-preview` criteria 17 and 23, `adrs.md` decision 2).

use std::collections::BTreeMap;
use std::sync::Arc;

use curvyo_text_core::{Face, FaceStore, TextError};

/// The face bytes the preview holds at most (`adrs.md` decision 2): scrolling 2,000 system
/// families never keeps every file in wasm memory.
pub const PREVIEW_FACE_BUDGET: usize = 256 * 1024 * 1024;

#[derive(Debug)]
struct Held {
    face: Arc<Face>,
    bytes: usize,
    /// The use counter's value at the last draw: a counter, not a clock (`CLAUDE.md` §6).
    used: u64,
}

/// The preview's faces by the frontend's id (host file id and face index).
#[derive(Debug)]
pub struct PreviewFaces {
    budget: usize,
    held: usize,
    uses: u64,
    faces: BTreeMap<String, Held>,
}

impl Default for PreviewFaces {
    fn default() -> Self {
        Self::with_budget(PREVIEW_FACE_BUDGET)
    }
}

impl PreviewFaces {
    /// An empty store with [`PREVIEW_FACE_BUDGET`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty store holding at most `budget` bytes of faces (more only while a single face is
    /// larger).
    #[must_use]
    pub const fn with_budget(budget: usize) -> Self {
        Self {
            budget,
            held: 0,
            uses: 0,
            faces: BTreeMap::new(),
        }
    }

    /// Parses face `index` of `bytes` and keeps it under `id`, marked as drawn just now; the
    /// least recently drawn others are dropped while the bytes held exceed the budget.
    ///
    /// # Errors
    /// [`TextError`] for bytes that hold no usable face; nothing is kept then.
    pub fn add(&mut self, id: &str, bytes: Vec<u8>, index: u32) -> Result<(), TextError> {
        let size = bytes.len();
        let mut store = FaceStore::new();
        let key = store.add(bytes, index)?;
        let face = store
            .get(&key)
            .cloned()
            .ok_or(TextError::Unreadable(index))?;
        self.uses += 1;
        if let Some(old) = self.faces.insert(
            id.to_owned(),
            Held {
                face,
                bytes: size,
                used: self.uses,
            },
        ) {
            self.held -= old.bytes;
        }
        self.held += size;
        self.evict(id);
        Ok(())
    }

    fn evict(&mut self, keep: &str) {
        while self.held > self.budget {
            let oldest = self
                .faces
                .iter()
                .filter(|(id, _)| id.as_str() != keep)
                .min_by_key(|(_, held)| held.used)
                .map(|(id, _)| id.clone());
            let Some(oldest) = oldest else {
                return;
            };
            if let Some(dropped) = self.faces.remove(&oldest) {
                self.held -= dropped.bytes;
            }
        }
    }

    /// Whether the face `id` is held.
    #[must_use]
    pub fn has(&self, id: &str) -> bool {
        self.faces.contains_key(id)
    }

    /// The face `id`, marked as drawn just now.
    pub fn get(&mut self, id: &str) -> Option<Arc<Face>> {
        self.uses += 1;
        let uses = self.uses;
        self.faces.get_mut(id).map(|held| {
            held.used = uses;
            held.face.clone()
        })
    }

    /// The bytes of the faces held.
    #[must_use]
    pub const fn held_bytes(&self) -> usize {
        self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font(path: &str) -> Vec<u8> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(dir.join("../frontend/public/fonts").join(path)).unwrap()
    }

    const SANS: &str = "source-sans-3/SourceSans3-Regular.otf";
    const SERIF: &str = "source-serif-4/SourceSerif4-Regular.ttf";
    const SCRIPT: &str = "hershey-script-1/HersheyScript1.svg";

    #[test]
    fn a_face_is_kept_and_found() {
        let mut faces = PreviewFaces::new();
        assert!(!faces.has("sans#0"));
        faces.add("sans#0", font(SANS), 0).unwrap();
        assert!(faces.has("sans#0"));
        assert!(faces.get("sans#0").is_some());
        assert!(faces.get("serif#0").is_none());
        assert_eq!(faces.held_bytes(), font(SANS).len());
    }

    #[test]
    fn bytes_that_are_no_font_are_refused_and_not_kept() {
        let mut faces = PreviewFaces::new();
        assert!(faces.add("x", b"not a font".to_vec(), 0).is_err());
        assert!(!faces.has("x"));
        assert_eq!(faces.held_bytes(), 0);
    }

    #[test]
    fn the_least_recently_drawn_face_goes_first_above_the_budget() {
        let budget = font(SANS).len() + font(SERIF).len() + 10;
        let mut faces = PreviewFaces::with_budget(budget);
        faces.add("sans", font(SANS), 0).unwrap();
        faces.add("serif", font(SERIF), 0).unwrap();
        // Drawing Sans again makes Serif the oldest.
        let _ = faces.get("sans");
        faces.add("script", font(SCRIPT), 0).unwrap();
        assert!(faces.has("sans"));
        assert!(!faces.has("serif"));
        assert!(faces.has("script"));
        assert!(faces.held_bytes() <= budget);
    }

    #[test]
    fn a_face_larger_than_the_budget_is_still_held_alone() {
        let mut faces = PreviewFaces::with_budget(10);
        faces.add("serif", font(SERIF), 0).unwrap();
        faces.add("sans", font(SANS), 0).unwrap();
        assert!(faces.has("sans"));
        assert!(!faces.has("serif"));
    }

    #[test]
    fn adding_the_same_id_again_replaces_it() {
        let mut faces = PreviewFaces::new();
        faces.add("a", font(SANS), 0).unwrap();
        faces.add("a", font(SANS), 0).unwrap();
        assert_eq!(faces.held_bytes(), font(SANS).len());
    }
}
