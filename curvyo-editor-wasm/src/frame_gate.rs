//! Decides whether the next animation frame has anything new to draw (`specs/0126` phase 1).
//!
//! [`TrackedSession`] holds the [`Session`] of a `WasmSession` and counts every mutable access to
//! it as a change of the picture: the frame generation. Every host call that can change the
//! document, the view, the selection, a tool or gesture, a hover, an overlay, a preview, an import
//! or an animation (the caret's blink, the play bar) takes the session mutably through
//! [`DerefMut`] and so forces a frame, without a list of triggers that a new call could miss.
//! Shared access ([`Deref`]) never does. The few per-frame calls that need `&mut` but change
//! nothing on the canvas (taking a notice, the sync frame start without remote records) go
//! through [`TrackedSession::untracked_mut`]. A frame is skipped only when the generation it would
//! draw was drawn already on the current GPU target, twice: `WebKitGTK` shows a canvas one frame
//! late, so the last frame of a change is repeated once or the screen keeps the one before it
//! (seen under Xvfb, `specs/0126` phase 1). A replaced or lost target forces a frame.

use std::ops::{Deref, DerefMut};
use std::panic::Location;

use crate::session::Session;

/// [`TrackedSession::frame_state`]'s flag: the session has a sync link, whose frame start runs
/// every frame.
pub(crate) const FRAME_SYNC_LINK: u32 = 1;
/// [`TrackedSession::frame_state`]'s flag: a width-editing notice waits for the host.
pub(crate) const FRAME_WIDTH_NOTICE: u32 = 2;
/// [`TrackedSession::frame_state`]'s flag: the unreadable-image notice waits for the host.
pub(crate) const FRAME_BAD_ASSET_NOTICE: u32 = 4;

/// How many distinct change sites are kept between two probe reads.
const MAX_SITES: usize = 32;

/// The session of a `WasmSession` with its frame generation and the last frame drawn.
pub(crate) struct TrackedSession {
    session: Session,
    /// Bumped by every mutable access to `session`.
    generation: u64,
    /// The generation the current GPU target shows, `None` when it shows nothing known.
    drawn: Option<u64>,
    /// `drawn` was submitted twice in a row: the host shows it whatever its latency.
    settled: bool,
    /// How many frames were submitted to the GPU, for the performance probe.
    submitted: u64,
    /// The source lines of the mutable accesses since the probe last asked, with their counts.
    sites: Vec<(&'static Location<'static>, u32)>,
}

impl TrackedSession {
    /// `session`, not drawn yet.
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            generation: 0,
            drawn: None,
            settled: false,
            submitted: 0,
            sites: Vec::new(),
        }
    }

    /// The source lines (`file:line count`, one per line) of the changes since the last call:
    /// the performance probe names what keeps a document drawing.
    pub(crate) fn take_change_sites(&mut self) -> String {
        use std::fmt::Write as _;
        let mut text = String::new();
        for (at, count) in std::mem::take(&mut self.sites) {
            // invariant: writing to a String cannot fail
            let _ = writeln!(text, "{}:{} {count}", at.file(), at.line());
        }
        text
    }

    /// The session for a per-frame call that needs `&mut` but changes nothing the canvas draws:
    /// it does not count as a change.
    pub(crate) fn untracked_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// The frame generation: changes whenever the session may have changed.
    #[cfg(test)]
    pub(crate) fn frame_generation(&self) -> u64 {
        self.generation
    }

    /// Whether the current GPU target lacks the session's present picture.
    pub(crate) fn needs_frame(&self) -> bool {
        self.drawn != Some(self.generation) || !self.settled
    }

    /// Records that the present generation was submitted to the GPU target.
    pub(crate) fn frame_drawn(&mut self) {
        self.settled = self.drawn == Some(self.generation);
        self.drawn = Some(self.generation);
        self.submitted = self.submitted.wrapping_add(1);
    }

    /// Counts a change made through [`TrackedSession::untracked_mut`] after all.
    #[track_caller]
    pub(crate) fn mark_changed(&mut self) {
        self.changed_at(Location::caller());
    }

    /// Bumps the generation and counts `at` among the change sites.
    fn changed_at(&mut self, at: &'static Location<'static>) {
        self.generation = self.generation.wrapping_add(1);
        if let Some((_, count)) = self.sites.iter_mut().find(|(site, _)| *site == at) {
            *count = count.saturating_add(1);
        } else if self.sites.len() < MAX_SITES {
            // Bounded while no probe asks: the first lines are the telling ones.
            self.sites.push((at, 1));
        }
    }

    /// Forgets what the GPU target shows (a new canvas, a lost or reconfigured surface, a host
    /// request): the next frame is drawn even though the session did not change.
    pub(crate) fn force_frame(&mut self) {
        self.drawn = None;
    }

    /// How many frames were submitted to the GPU since the session was created.
    #[cfg(test)]
    pub(crate) fn frames_submitted(&self) -> u64 {
        self.submitted
    }

    /// The render loop's one call per frame: the generation, whether a frame is needed (1 or 0),
    /// the view's origin x and y and scale, the `FRAME_*` flags and the submitted frames. As
    /// `f64`, exact for counts below 2^53.
    #[allow(clippy::cast_precision_loss)] // counts stay far below 2^53
    pub(crate) fn frame_state(&self) -> [f64; 7] {
        let origin = self.session.screen_to_document(0.0, 0.0);
        let mut flags = 0;
        if self.session.has_sync_link() {
            flags |= FRAME_SYNC_LINK;
        }
        if self.session.width_notice_pending() {
            flags |= FRAME_WIDTH_NOTICE;
        }
        if self.session.bad_asset_notice_pending() {
            flags |= FRAME_BAD_ASSET_NOTICE;
        }
        [
            self.generation as f64,
            if self.needs_frame() { 1.0 } else { 0.0 },
            origin.x,
            origin.y,
            self.session.view().scale(),
            f64::from(flags),
            self.submitted as f64,
        ]
    }

    /// Imports the remote records of a shared document and exports the local changes before the
    /// frame is drawn: a change of the document only when records were imported or the document
    /// moved on, so an idle shared document draws no frame.
    pub(crate) fn sync_frame_start(&mut self) -> bool {
        let changed = self.document_moves(Session::sync_frame_start);
        if changed {
            self.changed_at(Location::caller());
        }
        changed
    }

    /// Runs `step` of the sync link (its clock, its transport), which changes the canvas only
    /// through the document: a change only when the document moved on. Presence and the link's
    /// state are drawn by the host's DOM, not on the canvas.
    pub(crate) fn document_moves<R>(&mut self, step: impl FnOnce(&mut Session) -> R) -> R {
        let before = self.session.document_version();
        let result = step(&mut self.session);
        if self.session.document_version() != before {
            self.changed_at(Location::caller());
        }
        result
    }
}

impl Deref for TrackedSession {
    type Target = Session;

    fn deref(&self) -> &Session {
        &self.session
    }
}

impl DerefMut for TrackedSession {
    /// A mutable access may change the picture: the next frame is drawn.
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Session {
        self.changed_at(Location::caller());
        &mut self.session
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;

    use super::*;
    use crate::session::Tool;

    /// A session drawn once, as after the first frame.
    fn drawn() -> TrackedSession {
        let mut session = Session::new(1);
        session.show_default_view();
        session.resize_viewport(800.0, 600.0);
        let mut tracked = TrackedSession::new(session);
        assert!(tracked.needs_frame(), "a new session draws its first frame");
        settle(&mut tracked);
        tracked
    }

    /// Draws frames until none is needed, as the render loop does: two per change.
    fn settle(tracked: &mut TrackedSession) {
        let before = tracked.frames_submitted();
        while tracked.needs_frame() {
            tracked.frame_drawn();
            assert!(
                tracked.frames_submitted() - before <= 2,
                "a change settles in two frames"
            );
        }
    }

    /// A rectangle drawn with the Rectangle tool, left selected.
    fn rectangle(tracked: &mut TrackedSession) {
        rectangle_in(tracked);
    }

    /// What the render loop and the overlays read every frame of an idle document.
    fn idle_frame(tracked: &mut TrackedSession) {
        let _ = tracked.frame_state();
        let _ = tracked.sync_frame_start();
        let _ = tracked.view();
        let _ = tracked.pages_json();
        let _ = tracked.measure_view_json();
        let _ = tracked.presence_marks_json();
        let _ = tracked.is_dirty();
        let _ = tracked.frame_draw_list();
        let _ = tracked.untracked_mut().take_width_edit_notice();
        let _ = tracked.take_unsupported_width_notice();
        let _ = tracked.untracked_mut().take_bad_asset_notice();
        let _ = tracked.untracked_mut().take_history_notice();
        let _ = tracked.untracked_mut().take_key_update();
        let _ = tracked.untracked_mut().take_sync_notice();
        tracked.document_moves(|s| s.sync_tick(curvyo_sync_core::Millis(1_000)));
    }

    #[test]
    fn an_unchanged_session_renders_no_second_frame() {
        let mut tracked = drawn();
        for _ in 0..3 {
            idle_frame(&mut tracked);
            assert!(!tracked.needs_frame(), "an idle frame draws nothing");
        }
        assert_eq!(
            tracked.frames_submitted(),
            2,
            "the first frame and its repeat"
        );
        assert_eq!(tracked.frame_state()[1], 0.0, "the loop is told so");
    }

    #[test]
    fn a_change_is_drawn_twice_for_a_host_that_shows_the_canvas_one_frame_late() {
        let mut tracked = drawn();
        tracked.set_tool(Tool::Pen);
        tracked.frame_drawn();
        assert!(tracked.needs_frame(), "the repeat");
        tracked.set_tool(Tool::Select);
        tracked.frame_drawn();
        assert!(
            tracked.needs_frame(),
            "a change during the repeat draws again"
        );
        tracked.frame_drawn();
        assert!(!tracked.needs_frame());
    }

    #[test]
    fn a_drawn_document_stays_idle_too() {
        let mut tracked = drawn();
        rectangle(&mut tracked);
        settle(&mut tracked);
        idle_frame(&mut tracked);
        assert!(!tracked.needs_frame());
    }

    /// Each trigger of `specs/0126` phase 1 forces exactly the next frame.
    #[test]
    fn every_kind_of_change_forces_a_frame() {
        type Change = fn(&mut TrackedSession);
        let changes: [(&str, Change); 13] = [
            ("document edit", |t| rectangle(t)),
            ("undo", |t| {
                rectangle(t);
                settle(t);
                let _ = t.undo();
            }),
            ("wheel zoom", |t| {
                t.wheel(0.0, -120.0, 50.0, 50.0, false, true);
            }),
            ("pan", |t| t.wheel(30.0, 0.0, 50.0, 50.0, false, false)),
            ("resize", |t| t.resize_viewport(640.0, 480.0)),
            ("device pixel ratio", |t| t.set_device_pixel_ratio(2.0)),
            ("tool", |t| t.set_tool(Tool::Pen)),
            ("selection", |t| {
                rectangle(t);
                settle(t);
                t.set_tool(Tool::Select);
                settle(t);
                t.pointer_down(Point::new(10.0, 10.0), false);
            }),
            ("hover", |t| {
                t.pointer_hover(Point::new(20.0, 20.0), false, false);
            }),
            ("gesture", |t| {
                t.set_tool(Tool::Rectangle);
                settle(t);
                t.pointer_down(Point::new(5.0, 5.0), false);
            }),
            ("caret blink", |t| t.set_text_caret_on(false)),
            ("trace view rest", |t| t.set_trace_view_moving(true)),
            ("play bar tick", |t| {
                let _ = t.play_tick(16.0);
            }),
        ];
        for (name, change) in changes {
            let mut tracked = drawn();
            change(&mut tracked);
            assert!(tracked.needs_frame(), "{name} forces a frame");
            settle(&mut tracked);
            assert!(!tracked.needs_frame(), "{name}: one frame, then idle");
        }
    }

    #[test]
    fn a_step_that_moves_the_document_forces_a_frame() {
        let mut tracked = drawn();
        tracked.document_moves(rectangle_in);
        assert!(tracked.needs_frame(), "the document moved on");
        settle(&mut tracked);
        tracked.document_moves(|s| s.set_tool(Tool::Pen));
        assert!(
            !tracked.needs_frame(),
            "a step that leaves the document draws nothing"
        );
    }

    fn rectangle_in(session: &mut Session) {
        session.set_tool(Tool::Rectangle);
        session.pointer_down(Point::new(100.0, 100.0), false);
        session.pointer_up(Point::new(200.0, 160.0), false, false);
    }

    #[test]
    fn the_probe_names_the_lines_that_changed_the_session() {
        let mut tracked = drawn();
        tracked.set_tool(Tool::Pen);
        tracked.set_tool(Tool::Select);
        let sites = tracked.take_change_sites();
        assert_eq!(sites.lines().count(), 2, "{sites}");
        assert!(sites.contains("frame_gate.rs"), "{sites}");
        assert!(tracked.take_change_sites().is_empty(), "taken once");
    }

    #[test]
    fn moved_dom_chips_force_a_frame_and_the_same_chips_do_not() {
        // As `WasmSession::set_clone_tag_obstacles`: the clone marks keep clear of the chips.
        let mut tracked = drawn();
        let report = |t: &mut TrackedSession, flat: &[f64]| {
            if t.untracked_mut().set_clone_tag_obstacles(flat) {
                t.mark_changed();
            }
        };
        report(&mut tracked, &[10.0, 10.0, 40.0, 20.0]);
        assert!(tracked.needs_frame(), "a chip appeared");
        settle(&mut tracked);
        report(&mut tracked, &[10.0, 10.0, 40.0, 20.0]);
        assert!(!tracked.needs_frame(), "the same chips draw nothing");
    }

    #[test]
    fn a_lost_or_replaced_gpu_target_forces_a_frame_without_a_change() {
        let mut tracked = drawn();
        let generation = tracked.frame_generation();
        tracked.force_frame();
        assert!(tracked.needs_frame());
        assert_eq!(
            tracked.frame_generation(),
            generation,
            "the session did not change"
        );
        settle(&mut tracked);
        assert!(!tracked.needs_frame());
    }

    #[test]
    fn the_frame_state_carries_the_view_and_the_submitted_frames() {
        let mut tracked = drawn();
        let state = tracked.frame_state();
        let origin = tracked.screen_to_document(0.0, 0.0);
        assert_eq!(state[0], 0.0, "no change since the session was made");
        assert_eq!((state[2], state[3]), (origin.x, origin.y));
        assert_eq!(state[4], tracked.view().scale());
        assert_eq!(state[5], 0.0, "no sync link, no notice");
        assert_eq!(state[6], 2.0, "the first frame and its repeat");
        tracked.wheel(0.0, -120.0, 50.0, 50.0, false, true);
        let moved = tracked.frame_state();
        assert_ne!(moved[0], state[0]);
        assert_eq!(moved[1], 1.0);
        assert_ne!(moved[4], state[4], "the zoom is in the state");
    }
}
