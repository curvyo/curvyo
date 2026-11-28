//! `Session`'s link to the other window of the same project (`specs/0158-pen-display` slice 2a,
//! ADR 0096 decisions 3 to 6, amendment A5, A6, A12): keeping this session's document and
//! selection equal to the other window's.
//!
//! Once per frame the session exports its commits since the last frame with the pictures the
//! other side has not got, imports the bundles that arrived in order, and marks its version
//! exported. The main window is the truth: a refused import, an undecodable bundle or a sequence
//! gap ends in a full project from the main window, opened by the pen window as a fresh replica.

mod incoming;
mod view;

use std::collections::{BTreeSet, VecDeque};

use curvyo_document_core::{AssetHash, DocumentVersion, NodeId};
use curvyo_ui_core::window_bridge::{
    Bundle, Control, Message, ProjectMessage, ProjectReason, Updates, decode, encode,
};

use super::{APP_VERSION, Session, Tool};

/// The notice after a reload from the main window (criterion 18).
pub(super) const RELOADED: &str = "Reloaded from the main window.";
/// The notice when the other window deleted the text under edit (criterion 15).
pub(super) const TEXT_DELETED: &str = "The text was deleted in the other window.";
/// The notice of a press on a sketch page in the pen window (criterion 22).
pub(super) const SKETCH_IN_MAIN: &str = "Sketch pages are edited in the main window.";
/// The History name of this device's pen window (criterion 6, ADR 0096 A7).
pub(super) const PEN_WINDOW_LABEL: &str = "Pen window";

/// Which window this session is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRole {
    /// The window that owns the file.
    Main,
    /// The pen window.
    Pen,
}

/// Where the link stands, for the page's states (UX 9.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// No link.
    None,
    /// The pen window waits for its first project.
    Waiting,
    /// A fate-deciding dialog of the main window is open (criterion 13).
    Held,
    /// An update was refused; a full project is on its way (criterion 18).
    OutOfSync,
    /// Both documents are kept equal.
    Live,
}

/// The link's state between frames.
// Independent facts of one link (held, out of sync, a project sent, a save asked, the field
// hold, the peer told); one enum would merge unrelated states.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct WindowLink {
    pub(super) role: LinkRole,
    /// The project generation; the main window raises it with every project it sends.
    generation: u32,
    seq_out: u64,
    seq_in: u64,
    /// The version whose changes went out or came in.
    exported: DocumentVersion,
    /// Pictures the other side holds or that went out.
    assets: BTreeSet<AssetHash>,
    /// The asset store's size when the pictures were last compared.
    asset_total: u64,
    /// The referenced pictures last named in `needs`.
    needs_sent: BTreeSet<AssetHash>,
    /// Pictures the other side asked for.
    asked_for: Vec<AssetHash>,
    /// Bundles that arrived, in order, for the next frame.
    inbox: VecDeque<Bundle>,
    /// Updates waiting for their pictures, in order (criterion 3).
    waiting: VecDeque<Updates>,
    /// Pictures this side asked for and has not received.
    asked: BTreeSet<AssetHash>,
    /// The selection both sides know.
    known_selection: Vec<NodeId>,
    /// An arrived selection that waits for a field to commit or for a text edit (criterion 4).
    pending_selection: Option<Vec<NodeId>>,
    /// A value field is typed in or dragged, or a chip is open (UX 9.4).
    field_busy: bool,
    /// Controls for the next bundle.
    controls: Vec<Control>,
    /// The main window holds the pen window (criterion 13).
    held: bool,
    /// A refused import waits for a full project (criterion 18).
    out_of_sync: bool,
    /// Main: a project went out in this generation, so updates may follow.
    projected: bool,
    /// Pen: the project that arrived, for the host to open.
    arrived: Option<ProjectMessage>,
    /// Pen: how the next session continues (from the project the host took).
    opening: Option<(ProjectReason, String, bool)>,
    /// Main: the pen window asked for a save (criterion 9).
    save: bool,
    /// Main: the blocked state last told the pen window (A12).
    shared_sent: Option<bool>,
    /// Pen: the main document's blocked state.
    pub(super) shared: bool,
    /// Pen: this replica's peer id went out in this generation.
    peer_told: bool,
    /// The other window's peer id, once told.
    pub(super) other_peer: Option<u64>,
    /// The document changed by an import since the host last asked.
    changed: bool,
    /// An import or an arrived selection changed the picture since the last frame.
    pub(super) redraw: bool,
    /// Notices for this window's notice slot.
    notices: VecDeque<String>,
    /// Follow view and the pen window's rectangle (2b).
    view: view::ViewLink,
}

impl WindowLink {
    fn new(role: LinkRole, version: DocumentVersion) -> Self {
        let mut controls = Vec::new();
        if role == LinkRole::Pen {
            controls.push(Control::NeedProject);
        }
        Self {
            role,
            generation: 0,
            seq_out: 0,
            seq_in: 0,
            exported: version,
            assets: BTreeSet::new(),
            asset_total: 0,
            needs_sent: BTreeSet::new(),
            asked_for: Vec::new(),
            inbox: VecDeque::new(),
            waiting: VecDeque::new(),
            asked: BTreeSet::new(),
            known_selection: Vec::new(),
            pending_selection: None,
            field_busy: false,
            controls,
            held: false,
            out_of_sync: false,
            projected: false,
            arrived: None,
            opening: None,
            save: false,
            shared_sent: None,
            shared: false,
            peer_told: false,
            other_peer: None,
            changed: false,
            redraw: false,
            notices: VecDeque::new(),
            view: view::ViewLink::default(),
        }
    }

    /// Whether this replica may send its commits now.
    fn exports(&self) -> bool {
        match self.role {
            LinkRole::Main => self.projected,
            LinkRole::Pen => {
                self.generation > 0
                    && !self.out_of_sync
                    && self.arrived.is_none()
                    && self.opening.is_none()
            }
        }
    }

    fn state(&self) -> LinkState {
        if self.out_of_sync {
            LinkState::OutOfSync
        } else if self.role == LinkRole::Pen && self.generation == 0 {
            LinkState::Waiting
        } else if self.held {
            LinkState::Held
        } else {
            LinkState::Live
        }
    }

    /// The next bundle's header.
    fn next_bundle(&mut self) -> Bundle {
        let bundle = Bundle {
            generation: self.generation,
            seq: self.seq_out,
            ..Bundle::default()
        };
        self.seq_out += 1;
        bundle
    }
}

impl Session {
    /// Starts the link of this window (`main` when it opens the pen window, `pen` when its page
    /// starts): the pen window asks for the project in its first bundle.
    pub fn link_start(&mut self, role: LinkRole) {
        self.link = Some(Box::new(WindowLink::new(role, self.document.version())));
    }

    /// Whether this session has a link (the frame asks before doing link work).
    #[must_use]
    pub fn has_window_link(&self) -> bool {
        self.link.is_some()
    }

    /// Ends the link: the pen window is gone (the hub's `PEER_GONE`), so the main window does no
    /// link work any more (criterion 19).
    pub fn link_end(&mut self) {
        self.link = None;
    }

    /// Takes one message of the other window; it is imported at the start of the next frame.
    pub fn link_receive(&mut self, bytes: &[u8]) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        match decode(bytes) {
            Ok(Message::PeerGone) => self.link_end(),
            Ok(Message::Bundle(bundle)) => link.inbox.push_back(bundle),
            Err(_) => self.link_refuse(),
        }
    }

    /// One frame of the link: exports this replica's commits, imports what arrived and marks
    /// the version exported; the bundle to send, `None` when there is nothing or no link.
    pub fn link_frame(&mut self) -> Option<Vec<u8>> {
        self.link.as_ref()?;
        let (assets, updates) = self.link_export();
        let generation = self.link.as_ref()?.generation;
        self.link_import();
        let version = self.document.version();
        // A project that went out in this frame takes the selection in the next bundle of its
        // own generation, so the pen window shows it as soon as it opens.
        let same_generation = self.link.as_ref()?.generation == generation;
        let selection = if same_generation {
            self.link_selection_out()
        } else {
            None
        };
        self.link_shared_out();
        let view = self.link_view_out();
        let link = self.link.as_mut()?;
        link.exported = version;
        let mut bundle = link.next_bundle();
        if link.generation == generation {
            bundle.assets = assets;
            bundle.updates = updates;
            bundle.selection = selection;
        } else {
            // A project went out meanwhile: what was exported belongs to the old replica.
            bundle.generation = link.generation;
        }
        if link.role == LinkRole::Pen && link.exports() && !link.peer_told {
            link.peer_told = true;
            link.controls.push(Control::Peer(self.peer));
        }
        bundle.controls = std::mem::take(&mut link.controls);
        bundle.controls.extend(view);
        if bundle.is_empty() {
            link.seq_out -= 1;
            return None;
        }
        Some(encode(&bundle))
    }

    /// The commits since the last frame with the pictures the other side lacks (A5).
    fn link_export(&mut self) -> (Vec<(AssetHash, Vec<u8>)>, Option<Updates>) {
        let version = self.document.version();
        let Some(link) = self.link.as_ref().filter(|link| link.exports()) else {
            return (Vec::new(), None);
        };
        let asked: Vec<AssetHash> = link.asked_for.clone();
        let changed = version != link.exported;
        if !changed && asked.is_empty() {
            return (Vec::new(), None);
        }
        let updates = changed
            .then(|| self.document.export_updates(&link.exported).ok())
            .flatten();
        // Pictures are compared only when the store grew, as `sync_export` does.
        let asset_total = self.document.asset_bytes();
        let referenced = (asset_total != link.asset_total).then(|| self.referenced_assets());
        let mut send: Vec<AssetHash> = asked;
        if let Some(referenced) = &referenced {
            send.extend(
                referenced
                    .iter()
                    .filter(|h| !link.assets.contains(h))
                    .cloned(),
            );
        }
        let pictures: Vec<(AssetHash, Vec<u8>)> = send
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|hash| Some((hash.clone(), self.document.asset(&hash)?.to_vec())))
            .collect();
        // invariant: `link_export` returned early above when there is no link.
        #[allow(clippy::expect_used)]
        let link = self.link.as_mut().expect("the link checked above");
        link.asked_for.clear();
        link.asset_total = asset_total;
        link.assets
            .extend(pictures.iter().map(|(hash, _)| hash.clone()));
        let needs = referenced
            .filter(|referenced| *referenced != link.needs_sent)
            .map(|referenced| {
                link.needs_sent.clone_from(&referenced);
                referenced.into_iter().collect()
            });
        let updates = updates.map(|bytes| Updates { bytes, needs });
        (pictures, updates)
    }

    /// The local selection when it differs from the one both sides know.
    fn link_selection_out(&mut self) -> Option<Vec<NodeId>> {
        let ids = self.selection.ids().to_vec();
        let link = self.link.as_mut()?;
        if !link.exports() || link.pending_selection.is_some() || link.known_selection == ids {
            return None;
        }
        link.known_selection.clone_from(&ids);
        Some(ids)
    }

    /// Main: tells the pen window when the blocked state of extended path edits changed (A12).
    fn link_shared_out(&mut self) {
        let shared = self.extended_edits_blocked();
        let Some(link) = self.link.as_mut() else {
            return;
        };
        if link.role == LinkRole::Main && link.projected && link.shared_sent != Some(shared) {
            link.shared_sent = Some(shared);
            link.controls.push(Control::Shared(shared));
        }
    }

    /// Whether editable curves and width profiles are refused here (ADR 0053).
    pub(super) fn extended_edits_blocked(&self) -> bool {
        self.document.is_shared() || self.document.workspace_share().is_some()
    }

    /// Main: a full project goes out in a new generation; the pen window opens it as a fresh
    /// replica (A6). Nothing is marked saved: `pack` is not a save (criteria 9, 10).
    fn link_send_project(&mut self, reason: ProjectReason) {
        let Ok(bytes) = self.pack(APP_VERSION) else {
            return;
        };
        let shared = self.extended_edits_blocked();
        let page = self.active_page().id;
        let version = self.document.version();
        let referenced = self.referenced_assets();
        let asset_total = self.document.asset_bytes();
        let selection = self.selection.ids().to_vec();
        let Some(link) = self.link.as_mut() else {
            return;
        };
        link.generation += 1;
        link.seq_out = 0;
        link.seq_in = 0;
        link.exported = version;
        link.assets.clone_from(&referenced);
        link.needs_sent = referenced;
        link.asset_total = asset_total;
        link.asked_for.clear();
        link.inbox.clear();
        link.waiting.clear();
        link.asked.clear();
        link.pending_selection = None;
        // The pen window opens with an empty selection after a reload or another project; on
        // its first open it takes the main window's (criteria 4, 12, 18).
        link.known_selection = if reason == ProjectReason::Start {
            Vec::new()
        } else {
            selection
        };
        link.projected = true;
        link.shared_sent = Some(shared);
        link.view = link.view.restarted(LinkRole::Main);
        link.controls
            .retain(|c| matches!(c, Control::Hold | Control::Release));
        link.controls.push(Control::Project(ProjectMessage {
            reason,
            shared,
            page,
            bytes,
        }));
    }

    /// A refused import, an undecodable bundle or a gap (A6): the main window sends its
    /// project, the pen window asks for it and waits.
    fn link_refuse(&mut self) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        match link.role {
            LinkRole::Main => self.link_send_project(ProjectReason::Reload),
            LinkRole::Pen => {
                if !link.out_of_sync {
                    link.out_of_sync = true;
                    link.controls.push(Control::NeedProject);
                }
                link.inbox.clear();
                link.waiting.clear();
            }
        }
    }

    /// Main: a bundle with only `Hold` or `Release`, sent at once around a fate-deciding dialog
    /// (criterion 13); `None` without a pen window.
    pub fn link_hold(&mut self, on: bool) -> Option<Vec<u8>> {
        let link = self.link.as_mut().filter(|l| l.role == LinkRole::Main)?;
        let mut bundle = link.next_bundle();
        bundle.controls = vec![if on { Control::Hold } else { Control::Release }];
        Some(encode(&bundle))
    }

    /// Pen: Ctrl+S asks the main window to save (criterion 9).
    pub fn link_request_save(&mut self) {
        if let Some(link) = self.link.as_mut().filter(|l| l.role == LinkRole::Pen) {
            link.controls.push(Control::Save);
        }
    }

    /// Main: whether the pen window asked for a save since the last call.
    pub fn link_take_save(&mut self) -> bool {
        self.link
            .as_mut()
            .is_some_and(|link| std::mem::take(&mut link.save))
    }

    /// Pen: the project that arrived, for the host to open with a fresh peer id and then
    /// [`Session::link_continue`] on.
    pub fn link_take_project(&mut self) -> Option<Vec<u8>> {
        let link = self.link.as_mut()?;
        let project = link.arrived.take()?;
        link.opening = Some((project.reason, project.page, project.shared));
        Some(project.bytes)
    }

    /// A value field is typed in or dragged, or a chip is open: an arriving selection waits
    /// (criterion 4).
    pub fn link_set_field_busy(&mut self, busy: bool) {
        if let Some(link) = self.link.as_mut() {
            link.field_busy = busy;
        }
        if !busy {
            self.link_apply_pending_selection();
        }
    }

    /// The link's state for the page.
    #[must_use]
    pub fn link_state(&self) -> LinkState {
        self.link.as_ref().map_or(LinkState::None, |l| l.state())
    }

    /// Whether this session is the pen window.
    #[must_use]
    pub fn link_is_pen(&self) -> bool {
        self.link.as_ref().is_some_and(|l| l.role == LinkRole::Pen)
    }

    /// Whether an import or an arrived selection changed the picture since the last call.
    pub fn link_take_redraw(&mut self) -> bool {
        self.link
            .as_mut()
            .is_some_and(|link| std::mem::take(&mut link.redraw))
    }

    /// Whether imports changed the document since the last call: the host refreshes its panels.
    pub fn link_take_changed(&mut self) -> bool {
        self.link
            .as_mut()
            .is_some_and(|link| std::mem::take(&mut link.changed))
    }

    /// The next notice of the link, once.
    pub fn link_take_notice(&mut self) -> Option<String> {
        self.link.as_mut()?.notices.pop_front()
    }

    /// Whether the pen window ignores input now: no project yet, held, or out of sync
    /// (criteria 13, 18).
    pub(super) fn link_blocks_input(&self) -> bool {
        self.link.as_ref().is_some_and(|l| {
            l.role == LinkRole::Pen
                && (l.state() != LinkState::Live || l.arrived.is_some() || l.opening.is_some())
        })
    }

    /// A press on a sketch page in the pen window: refused with its notice (criterion 22).
    pub(super) fn link_refuses_sketch_press(&mut self) -> bool {
        if !self.link_is_pen() || self.workbench() != super::Workbench::Sketch {
            return false;
        }
        if let Some(link) = self.link.as_mut() {
            link.notices.push_back(SKETCH_IN_MAIN.to_owned());
        }
        true
    }

    /// The link of `old` continues in this session, which the host opened in its place (A6,
    /// criterion 12): the main window's New or Open sends the new project; the pen window's new
    /// replica keeps the tool (the Pencil on the first open) and the view after a reload, fits
    /// the page otherwise, and starts with an empty selection and undo.
    pub fn link_continue(&mut self, old: &mut Session) {
        let Some(mut link) = old.link.take() else {
            return;
        };
        link.changed = true;
        self.pen_peers.clone_from(&old.pen_peers);
        match link.role {
            LinkRole::Main => {
                self.link = Some(link);
                self.link_send_project(ProjectReason::Replaced);
            }
            LinkRole::Pen => {
                let (reason, page, shared) =
                    link.opening
                        .take()
                        .unwrap_or((ProjectReason::Reload, String::new(), false));
                self.adopt_pen_link(link, reason, shared);
                self.continue_window_state(old, reason, &page);
            }
        }
    }

    fn adopt_pen_link(&mut self, mut link: Box<WindowLink>, reason: ProjectReason, shared: bool) {
        link.generation = link.generation.max(1);
        link.seq_out = 0;
        link.exported = self.document.version();
        link.assets = self.referenced_assets();
        link.needs_sent.clone_from(&link.assets);
        link.asset_total = self.document.asset_bytes();
        link.known_selection = Vec::new();
        link.pending_selection = None;
        link.out_of_sync = false;
        link.peer_told = false;
        link.shared = shared;
        link.view = link.view.restarted(LinkRole::Pen);
        if reason == ProjectReason::Reload {
            link.notices.push_back(RELOADED.to_owned());
        }
        self.document.set_live_share(shared);
        self.link = Some(link);
    }

    /// The pen window's own state across a new replica: tool, Pencil settings and canvas size;
    /// the view kept on a reload, fitted to `page` otherwise (criteria 12, 18, 22).
    fn continue_window_state(&mut self, old: &Session, reason: ProjectReason, page: &str) {
        let tool = if reason == ProjectReason::Start {
            Tool::Pencil
        } else {
            old.tool
        };
        self.set_tool(tool);
        self.pencil.settings = old.pencil.settings;
        self.pencil.eraser_mode = reason != ProjectReason::Start && old.pencil.eraser_mode;
        self.device_pixel_ratio = old.device_pixel_ratio;
        self.covered_right = old.covered_right;
        let (_, height) = old.viewport.canvas_size();
        self.resize_viewport(old.canvas_css_width, height);
        if reason == ProjectReason::Reload {
            self.restore_view_text(&old.saved_view_text());
        } else {
            if !page.is_empty() {
                self.activate_page_in_place(page);
            }
            let (left, top) = old.pen_fit_insets;
            self.pen_fit_insets = (left, top);
            self.view_fit_page(left, top);
        }
    }
}
