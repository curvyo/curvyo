//! The view half of the window link (`specs/0158-pen-display` slice 2b criteria 5 and 6, ADR 0096
//! decision 4, A5 `View`): Follow view between the two windows and the pen window's visible
//! rectangle for the frame in the main window.
//!
//! View state is ephemeral: nothing here is in the document or the history. A view that this
//! window applied from the other one becomes the known view, so it is never sent back (no echo);
//! two changes that cross settle on the main window's.

use curvyo_ui_core::SavedView;
use curvyo_ui_core::window_bridge::{Control, ViewMessage};

use super::{LinkRole, Session};

/// Two zoom factors closer than this share of the zoom are the same view.
const ZOOM_TOLERANCE: f64 = 1e-9;
/// Two document points or rectangle edges closer than this (mm) are the same.
const POINT_TOLERANCE_MM: f64 = 1e-6;

/// One link's view exchange.
#[derive(Debug, Default)]
pub(super) struct ViewLink {
    /// Pen: Follow view is on. Main: the pen window's Follow as last told.
    follow: bool,
    /// Pen: the Follow flag the main window was last told, `None` before the first view.
    follow_told: Option<bool>,
    /// The view at the end of the last frame, or the one just applied: a later difference is a
    /// pan or zoom of this window's own.
    known: Option<SavedView>,
    /// Pen: the visible rectangle last sent.
    visible_sent: Option<[f64; 4]>,
    /// Views this side has sent in this generation.
    count: u32,
    /// The other side's last `count` received.
    seen: u32,
    /// Main: the pen window's visible rectangle in document mm, for the frame.
    pen_visible: Option<[f64; 4]>,
    /// Main: Follow was just turned on, so the main window's view goes out unasked.
    answer: bool,
}

impl ViewLink {
    /// The exchange of a new generation (New, Open, a reload): the pen window keeps its switch,
    /// everything else starts over.
    pub(super) fn restarted(&self, role: LinkRole) -> Self {
        Self {
            follow: role == LinkRole::Pen && self.follow,
            ..Self::default()
        }
    }
}

fn same_view(a: SavedView, b: SavedView) -> bool {
    (a.zoom - b.zoom).abs() <= ZOOM_TOLERANCE * a.zoom.abs().max(1.0)
        && (a.centre.x - b.centre.x).abs() <= POINT_TOLERANCE_MM
        && (a.centre.y - b.centre.y).abs() <= POINT_TOLERANCE_MM
}

fn same_rect(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y).abs() <= POINT_TOLERANCE_MM)
}

impl Session {
    /// Pen: turns Follow view on or off. On, the pen window takes the main window's view as soon
    /// as the main window answers; off, both keep the views they have (criterion 5).
    pub fn link_set_follow(&mut self, on: bool) {
        if let Some(link) = self.link.as_mut().filter(|l| l.role == LinkRole::Pen) {
            link.view.follow = on;
        }
    }

    /// Whether Follow view is on (pen: its switch; main: as the pen window last told).
    #[must_use]
    pub fn link_follow(&self) -> bool {
        self.link.as_ref().is_some_and(|l| l.view.follow)
    }

    /// The document rectangle this canvas shows: left, top, right, bottom in mm.
    pub(in crate::session) fn view_rect_mm(&self) -> [f64; 4] {
        let (width, height) = self.viewport.canvas_size();
        let top_left = self.viewport.screen_to_document(0.0, 0.0);
        let bottom_right = self.viewport.screen_to_document(width, height);
        [top_left.x, top_left.y, bottom_right.x, bottom_right.y]
    }

    /// Main: the pen window's visible rectangle in document mm, `None` without a pen window or
    /// before its first view (criterion 6).
    #[must_use]
    pub fn link_pen_visible(&self) -> Option<[f64; 4]> {
        self.link
            .as_ref()
            .filter(|l| l.role == LinkRole::Main)
            .and_then(|l| l.view.pen_visible)
    }

    /// Main: the pen window's visible rectangle on this canvas in CSS px (left, top, right,
    /// bottom), for the frame overlay; `None` when there is none to draw.
    #[must_use]
    pub fn link_pen_frame(&self) -> Option<[f64; 4]> {
        let [left, top, right, bottom] = self.link_pen_visible()?;
        let view = self.viewport.view();
        let (x0, y0) = view.document_to_screen(curvyo_document_core::Point::new(left, top));
        let (x1, y1) = view.document_to_screen(curvyo_document_core::Point::new(right, bottom));
        Some([x0, y0, x1, y1])
    }

    /// This frame's view for the other window, if it has one to send: the pen window sends its
    /// rectangle and switch when they change and its own moves while following; the main window
    /// sends its own moves while the pen window follows, and its view once Follow is turned on.
    pub(super) fn link_view_out(&mut self) -> Option<Control> {
        let saved = self.viewport.saved_view();
        let visible = self.view_rect_mm();
        let link = self.link.as_mut().filter(|l| l.exports())?;
        let role = link.role;
        let view = &mut link.view;
        let own_move = view.known.is_some_and(|known| !same_view(known, saved));
        view.known = Some(saved);
        let (moved, send) = match role {
            LinkRole::Pen => {
                let moved = view.follow && view.follow_told == Some(true) && own_move;
                let told = view.follow_told != Some(view.follow);
                let rect = view
                    .visible_sent
                    .is_none_or(|sent| !same_rect(sent, visible));
                view.follow_told = Some(view.follow);
                view.visible_sent = Some(visible);
                (moved, moved || told || rect)
            }
            LinkRole::Main => {
                let moved = (view.follow && own_move) || std::mem::take(&mut view.answer);
                (moved, moved)
            }
        };
        if !send {
            return None;
        }
        view.count += 1;
        Some(Control::View(ViewMessage {
            count: view.count,
            seen: view.seen,
            follow: view.follow,
            moved,
            zoom: saved.zoom,
            centre: (saved.centre.x, saved.centre.y),
            visible,
        }))
    }

    /// The other window's view: the main window keeps the pen's rectangle and answers a Follow
    /// just turned on; either window follows a move of the other's own while Follow is on,
    /// except the main window drops a pen move that crossed one of its own.
    pub(super) fn link_view_in(&mut self, message: &ViewMessage) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        let view = &mut link.view;
        view.seen = view.seen.max(message.count);
        let apply = match link.role {
            LinkRole::Main => {
                let was = view.follow;
                view.follow = message.follow;
                view.pen_visible = Some(message.visible);
                view.answer |= message.follow && !was;
                link.redraw = true;
                was && message.follow && message.moved && message.seen >= view.count
            }
            LinkRole::Pen => view.follow && message.moved,
        };
        if !apply {
            return;
        }
        self.viewport.restore_view(SavedView {
            zoom: message.zoom,
            centre: curvyo_document_core::Point::new(message.centre.0, message.centre.1),
        });
        let applied = self.viewport.saved_view();
        if let Some(link) = self.link.as_mut() {
            link.view.known = Some(applied);
            link.redraw = true;
        }
    }
}
