//! The single open project this slice's app state holds: just the path it
//! was last saved to or opened from, if any
//! (`specs/0001-project-file-foundation/specification.md` — no multi-project,
//! multi-window support is in scope here). No document lives here any
//! more (`specs/0002-path-node-editing/adrs.md`'s PR review: "the host does
//! byte I/O only") — the real document is the `WasmSession` the frontend
//! owns; this crate reads and writes `.curvyo` bytes on its behalf and
//! otherwise never looks inside them. Also buffers at most one pending
//! open payload for the frontend to pick up once it has mounted (see
//! `take_pending_open`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// The app's one open project: nothing but where it lives on disk.
#[derive(Default)]
pub struct ProjectState {
    /// Where it was last saved to or opened from, or `None` for a
    /// never-saved new project.
    pub path: Option<PathBuf>,
}

impl ProjectState {
    /// A fresh, never-saved project (File → New, and the app's initial
    /// launch state).
    pub fn new_unsaved() -> Self {
        Self { path: None }
    }

    /// A project just read back from `path` — recorded only once the
    /// frontend confirms the bytes actually parsed (`main.rs`'s
    /// `confirm_project_opened`); a refusal never reaches this
    /// constructor, so an already-open project is never touched by one
    /// (acceptance criterion 7).
    pub fn opened(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// The file name (with extension) the title bar should show, if this
    /// project has ever been saved or opened.
    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
    }
}

/// One pending "open" result, buffered for a not-yet-mounted frontend to
/// pick up (see [`AppState::take_pending_open`]'s own doc comment for
/// why this race exists at all).
pub enum PendingOpen {
    /// The file at `path` was read successfully; here are its raw bytes
    /// for the frontend to hand to `WasmSession::open` itself — this host
    /// never parses them (`specs/0002-path-node-editing/adrs.md`'s PR review).
    Bytes {
        /// The path these bytes were read from.
        path: PathBuf,
        /// The file's raw contents.
        bytes: Vec<u8>,
    },
    /// The path could not even be read (missing, permissions, not a
    /// file) — a host-level I/O failure, distinct from a `.curvyo` whose
    /// *content* is invalid (that refusal is the frontend's own, once it
    /// tries `WasmSession::open` on the bytes above).
    IoError(String),
}

/// Tauri-managed application state: the one open project, behind a mutex
/// since menu events and commands can both reach it (`tauri::State`), plus
/// at most one pending open payload.
///
/// The pending buffer exists for one reason: when the app is launched by
/// double-clicking a `.curvyo` (the OS file-association path), the open
/// attempt runs synchronously in Tauri's `.setup()`, before the
/// frontend's event listeners have attached. A live "open-bytes"/"open-error"
/// event does not survive that race, so a file opened this way could be
/// silently dropped — exactly what acceptance criterion 7 forbids. The
/// frontend polls `take_pending_open` once on mount.
///
/// Two flags decide the window close without a round trip to the page
/// (`specs/0051-sidebar-and-no-menu/` criterion 41): `dirty`, which the page reports on every
/// change, and `quitting`, which the quit command sets so the exit it triggers is not held.
#[derive(Default)]
pub struct AppState {
    project: Mutex<ProjectState>,
    pending_open: Mutex<Option<PendingOpen>>,
    dirty: AtomicBool,
    quitting: AtomicBool,
    /// The file Export SVG… chose, until the page hands over the text (`specs/0024` criterion 4).
    svg_export_path: Mutex<Option<std::path::PathBuf>>,
}

impl AppState {
    /// Keeps the file the export's save dialog chose.
    pub fn set_svg_export_path(&self, path: Option<std::path::PathBuf>) {
        if let Ok(mut slot) = self.svg_export_path.lock() {
            *slot = path;
        }
    }

    /// Takes the file the export's save dialog chose.
    pub fn take_svg_export_path(&self) -> Option<std::path::PathBuf> {
        self.svg_export_path
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    /// Records what the page reported: the project has changes that no saved file holds.
    pub fn set_dirty(&self, dirty: bool) {
        self.dirty.store(dirty, Ordering::SeqCst);
    }

    /// Whether the page last reported unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::SeqCst)
    }

    /// The page asked to leave the application: the exit that follows is not held.
    pub fn begin_quit(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    /// The page holds an empty, never-saved, clean project: forgets the path and the dirty flag.
    /// A file handed over at launch stays buffered for the page to take.
    pub fn reset_project(&self) {
        *self.lock_project() = ProjectState::new_unsaved();
        self.set_dirty(false);
    }

    /// The page opened the file at `path`: it is the project's path now, the project is clean,
    /// and a buffered copy of the file is dropped so that a later page start (a reload) does not
    /// open it again.
    pub fn confirm_opened(&self, path: PathBuf) {
        *self.lock_project() = ProjectState::opened(path);
        self.set_dirty(false);
        let _ = self.take_pending_open();
    }

    /// Whether a window close or an exit must wait for the page's answer: there are unsaved
    /// changes and the page has not asked to quit. A clean project is never held, so a hung page
    /// cannot block closing it.
    pub fn must_hold_close(&self) -> bool {
        self.is_dirty() && !self.quitting.load(Ordering::SeqCst)
    }

    /// Locks the project state.
    ///
    /// # Panics
    /// Propagates a poisoned-lock panic if a previous holder panicked
    /// while holding the lock. Nothing in this crate panics while holding
    /// it, so this should not occur in practice.
    pub fn lock_project(&self) -> MutexGuard<'_, ProjectState> {
        #[allow(clippy::unwrap_used)]
        // invariant: nothing in this crate panics while holding the lock.
        self.project.lock().unwrap()
    }

    /// Records a pending open payload so a not-yet-mounted frontend can
    /// still pick it up via [`Self::take_pending_open`]. Overwrites any
    /// previous pending value — only the most recent open attempt
    /// matters.
    ///
    /// # Panics
    /// See [`Self::lock_project`]; the same invariant applies.
    pub fn set_pending_open(&self, pending: PendingOpen) {
        #[allow(clippy::unwrap_used)]
        // invariant: nothing in this crate panics while holding the lock.
        {
            *self.pending_open.lock().unwrap() = Some(pending);
        }
    }

    /// Returns and clears the pending open payload, if any. Meant to be
    /// called exactly once, at frontend mount time — a later call (after
    /// the live "open-bytes"/"open-error" event path has taken over)
    /// finds nothing new and returns `None`.
    ///
    /// # Panics
    /// See [`Self::lock_project`]; the same invariant applies.
    pub fn take_pending_open(&self) -> Option<PendingOpen> {
        #[allow(clippy::unwrap_used)]
        // invariant: nothing in this crate panics while holding the lock.
        self.pending_open.lock().unwrap().take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 41: only a dirty project that is not being quit is held.
    #[test]
    fn only_a_dirty_project_that_is_not_quitting_holds_the_close() {
        let state = AppState::default();
        assert!(!state.must_hold_close(), "a clean project closes at once");
        state.set_dirty(true);
        assert!(state.must_hold_close());
        state.begin_quit();
        assert!(
            !state.must_hold_close(),
            "the exit the page asked for goes through"
        );
        state.set_dirty(false);
        assert!(!state.must_hold_close());
    }

    /// A webview reload starts an empty page: the host must not keep the old path (Ctrl+S would
    /// write the new document over the old file) or a dirty flag the new page never reports.
    #[test]
    fn a_page_start_resets_the_path_and_the_dirty_flag_but_keeps_a_launch_file() {
        let state = AppState::default();
        state.confirm_opened(PathBuf::from("/work/old.curvyo"));
        state.set_dirty(true);
        state.set_pending_open(PendingOpen::IoError("launch".to_string()));
        state.reset_project();
        assert_eq!(state.lock_project().path, None);
        assert!(!state.is_dirty());
        assert!(!state.must_hold_close());
        assert!(matches!(
            state.take_pending_open(),
            Some(PendingOpen::IoError(_))
        ));
    }

    /// An opened file is the project's path, clean, and no longer pending.
    #[test]
    fn opening_a_file_sets_the_path_and_drops_the_buffered_copy() {
        let state = AppState::default();
        state.set_dirty(true);
        state.set_pending_open(PendingOpen::IoError("handed over".to_string()));
        state.confirm_opened(PathBuf::from("/work/new.curvyo"));
        assert_eq!(
            state.lock_project().file_name().as_deref(),
            Some("new.curvyo")
        );
        assert!(!state.is_dirty());
        assert!(state.take_pending_open().is_none());
    }
}
