//! The import half of the window link: the bundles that arrived, applied in order at the start
//! of a frame (ADR 0096 A5, A6): pictures first, updates once their pictures are here, then the
//! selection and the controls.

use curvyo_document_core::NodeId;
use curvyo_ui_core::EditTarget;
use curvyo_ui_core::window_bridge::{Bundle, Control, ProjectReason, Updates};

use super::{LinkRole, Session, TEXT_DELETED};

impl Session {
    /// Imports every bundle that arrived, in order; a refusal stops and reloads (A6).
    pub(super) fn link_import(&mut self) {
        let mut changed = false;
        loop {
            let Some(link) = self.link.as_mut() else {
                return;
            };
            // A project that waits for the host keeps what came after it for the next replica.
            if link.arrived.is_some() {
                break;
            }
            let Some(bundle) = link.inbox.pop_front() else {
                break;
            };
            let Ok(applied) = self.link_take_bundle(bundle) else {
                self.link_refuse();
                break;
            };
            changed |= applied;
        }
        if changed {
            self.link_after_import();
        }
    }

    /// One bundle: `Ok(true)` when the document changed, `Err` when it is refused.
    fn link_take_bundle(&mut self, bundle: Bundle) -> Result<bool, ()> {
        let Some(link) = self.link.as_mut() else {
            return Ok(false);
        };
        if bundle.generation != link.generation {
            let project = bundle
                .controls
                .iter()
                .any(|c| matches!(c, Control::Project(_)));
            match link.role {
                // A bundle of the old replica: only its requests count, not its view.
                LinkRole::Main => {
                    let requests = bundle
                        .controls
                        .into_iter()
                        .filter(|c| !matches!(c, Control::View(_)))
                        .collect();
                    self.link_controls(requests);
                    return Ok(false);
                }
                LinkRole::Pen if project && bundle.generation > link.generation => {
                    link.generation = bundle.generation;
                    link.seq_in = 0;
                }
                LinkRole::Pen => return Ok(false),
            }
        }
        let link = self.link.as_mut().ok_or(())?;
        if bundle.seq != link.seq_in {
            return Err(());
        }
        link.seq_in += 1;
        if link.role == LinkRole::Pen && link.out_of_sync {
            // Waiting for the project: everything else is about the replica being replaced.
            let projects = bundle
                .controls
                .into_iter()
                .filter(|c| matches!(c, Control::Project(_)))
                .collect();
            self.link_controls(projects);
            return Ok(false);
        }
        let mut changed = false;
        for (hash, picture) in bundle.assets {
            if self.document.insert_asset(&picture).is_ok() {
                changed = true;
            }
            let link = self.link.as_mut().ok_or(())?;
            link.asked.remove(&hash);
            link.assets.insert(hash);
        }
        if let Some(updates) = bundle.updates {
            self.link.as_mut().ok_or(())?.waiting.push_back(updates);
        }
        changed |= self.link_import_waiting()?;
        if let Some(ids) = bundle.selection {
            self.link_selection_in(ids);
        }
        self.link_controls(bundle.controls);
        Ok(changed)
    }

    /// Imports the waiting updates whose pictures are here, in order; the first one that still
    /// lacks a picture waits with all later ones and asks for it once (criterion 3).
    fn link_import_waiting(&mut self) -> Result<bool, ()> {
        let mut changed = false;
        loop {
            let Some(link) = self.link.as_mut() else {
                return Ok(changed);
            };
            let Some(front) = link.waiting.front() else {
                return Ok(changed);
            };
            let missing: Vec<_> = front
                .needs
                .iter()
                .flatten()
                .filter(|hash| self.document.asset(hash).is_none())
                .cloned()
                .collect();
            if !missing.is_empty() {
                let new: Vec<_> = missing
                    .into_iter()
                    .filter(|hash| link.asked.insert(hash.clone()))
                    .collect();
                if !new.is_empty() {
                    link.controls.push(Control::NeedAssets(new));
                }
                return Ok(changed);
            }
            let Some(Updates { bytes, .. }) = link.waiting.pop_front() else {
                return Ok(changed);
            };
            self.link_import_updates(&bytes)?;
            changed = true;
        }
    }

    /// Imports one update as a relay import is checked; the main window of a shared document
    /// refuses extended path history (A12).
    fn link_import_updates(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let main = !self.link_is_pen();
        let result = if main && self.extended_edits_blocked() {
            self.document.import_shared_updates(bytes)
        } else {
            self.document.import_updates(bytes)
        };
        result.map_err(|_| ())?;
        // The pen window's steps leave with the main window's next relay export (A5).
        if main && self.sync.is_some() {
            self.sync_export();
        }
        Ok(())
    }

    /// The other window's selection: applied, unknown ids dropped, unless a field or a text
    /// edit holds it (criteria 4, 15).
    fn link_selection_in(&mut self, ids: Vec<NodeId>) {
        let kept: Vec<NodeId> = ids
            .into_iter()
            .filter(|id| self.document.object(*id).is_some())
            .collect();
        let Some(link) = self.link.as_mut() else {
            return;
        };
        link.pending_selection = Some(kept);
        self.link_apply_pending_selection();
    }

    /// Applies a waiting selection once no field or text edit holds it.
    pub(super) fn link_apply_pending_selection(&mut self) {
        let editing = self.text.edit.is_some();
        let Some(link) = self.link.as_mut() else {
            return;
        };
        if link.field_busy || editing {
            return;
        }
        let Some(ids) = link.pending_selection.take() else {
            return;
        };
        let ids: Vec<NodeId> = ids
            .into_iter()
            .filter(|id| self.document.object(*id).is_some())
            .collect();
        link.known_selection.clone_from(&ids);
        link.redraw = true;
        self.selection.set(&ids);
    }

    fn link_controls(&mut self, controls: Vec<Control>) {
        for control in controls {
            self.link_control(control);
        }
    }

    fn link_control(&mut self, control: Control) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        match (link.role, control) {
            (LinkRole::Main, Control::NeedProject) => {
                let reason = if link.projected {
                    ProjectReason::Reload
                } else {
                    ProjectReason::Start
                };
                self.link_send_project(reason);
            }
            (LinkRole::Main, Control::Save) => link.save = true,
            (LinkRole::Main, Control::Peer(peer)) => {
                link.other_peer = Some(peer);
                if !self.pen_peers.contains(&peer) {
                    self.pen_peers.push(peer);
                }
            }
            (_, Control::NeedAssets(hashes)) => link.asked_for.extend(hashes),
            (_, Control::View(message)) => self.link_view_in(&message),
            (LinkRole::Pen, Control::Project(project)) => {
                link.held = false;
                link.out_of_sync = false;
                link.arrived = Some(project);
            }
            (LinkRole::Pen, Control::Hold) => {
                link.held = true;
                link.redraw = true;
                // A stroke or drag in progress is dropped as by Escape: nothing is written.
                self.pointer_cancelled();
                self.pencil_reset();
            }
            (LinkRole::Pen, Control::Release) => link.held = false,
            (LinkRole::Pen, Control::Shared(shared)) => {
                link.shared = shared;
                self.document.set_live_share(shared);
            }
            _ => {}
        }
    }

    /// After an import changed the document: fonts, the selection's gone ids, a text under
    /// edit that the other window deleted (criterion 15).
    fn link_after_import(&mut self) {
        self.refresh_document_fonts();
        self.drop_gone_from_selection();
        if let Some(edit) = self.text.edit.as_ref()
            && let EditTarget::Existing(id) = edit.target()
            && self.document.object(id).is_none()
        {
            self.text.edit = None;
            self.text.drag_anchor = None;
            if let Some(link) = self.link.as_mut() {
                link.notices.push_back(TEXT_DELETED.to_owned());
            }
        }
        if let Some(link) = self.link.as_mut() {
            link.changed = true;
            link.redraw = true;
        }
    }
}
