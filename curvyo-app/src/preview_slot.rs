//! Which highlighted file the import dialog's preview decodes next (`specs/0087-image-clipboard-and-drop`
//! criteria 26 and 28, architect review): at most one decode at a time, the newest request waiting.
//!
//! A decode may hold some 400 MB, so holding an arrow key in a folder of large photos must not run
//! one per file: requests that arrive while a decode runs replace each other in one slot, and only
//! the newest is started when the running decode ends.

use std::path::PathBuf;

/// The state of the preview's single decode.
#[derive(Debug, Default)]
pub(crate) struct PreviewSlot {
    running: bool,
    /// Whether a request came after the running decode began, so its result is not shown.
    stale: bool,
    waiting: Option<PathBuf>,
}

/// What follows the end of a decode.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AfterDecode {
    /// Whether the finished result is still the newest request and is shown.
    pub(crate) show: bool,
    /// The decode to start now, if a file is waiting.
    pub(crate) next: Option<PathBuf>,
}

impl PreviewSlot {
    /// A new highlight: `path` to decode, `None` for nothing (a folder, no file, preview off).
    /// Gives the path to decode now when no decode runs; otherwise it waits in the slot,
    /// replacing any earlier waiting one.
    pub(crate) fn request(&mut self, path: Option<PathBuf>) -> Option<PathBuf> {
        if self.running {
            self.stale = true;
            self.waiting = path;
            return None;
        }
        self.running = path.is_some();
        path
    }

    /// The running decode ended.
    pub(crate) fn finished(&mut self) -> AfterDecode {
        let show = !self.stale;
        self.stale = false;
        let next = self.waiting.take();
        self.running = next.is_some();
        AfterDecode { show, next }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unnecessary_wraps)] // every request and answer is an Option
    fn path(name: &str) -> Option<PathBuf> {
        Some(PathBuf::from(name))
    }

    #[test]
    fn a_request_on_an_idle_slot_starts_at_once_and_is_shown() {
        let mut slot = PreviewSlot::default();
        assert_eq!(slot.request(path("a.png")), path("a.png"));
        assert_eq!(
            slot.finished(),
            AfterDecode {
                show: true,
                next: None
            }
        );
    }

    #[test]
    fn two_quick_requests_lead_to_one_decode() {
        let mut slot = PreviewSlot::default();
        assert_eq!(slot.request(path("a.png")), path("a.png"));
        assert_eq!(slot.request(path("b.png")), None);
        assert_eq!(slot.request(path("c.png")), None, "never a second decode");
        // a ends: its result is stale, only the newest (c) starts, b is skipped.
        assert_eq!(
            slot.finished(),
            AfterDecode {
                show: false,
                next: path("c.png")
            }
        );
        assert_eq!(slot.request(path("d.png")), None, "c still runs");
        assert_eq!(slot.finished().next, path("d.png"));
        assert_eq!(
            slot.finished(),
            AfterDecode {
                show: true,
                next: None
            }
        );
        assert_eq!(slot.request(path("e.png")), path("e.png"), "idle again");
    }

    #[test]
    fn a_folder_highlighted_during_a_decode_drops_its_result() {
        let mut slot = PreviewSlot::default();
        slot.request(path("a.png"));
        assert_eq!(slot.request(None), None);
        assert_eq!(
            slot.finished(),
            AfterDecode {
                show: false,
                next: None
            }
        );
        assert_eq!(slot.request(None), None, "nothing to decode");
        assert_eq!(slot.request(path("b.png")), path("b.png"));
    }
}
