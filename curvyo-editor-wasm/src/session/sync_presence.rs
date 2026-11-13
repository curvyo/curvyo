//! `Session`'s presence channel (ADR 0031 decision 7, `specs/0092` criteria 7, 14 to 16): what
//! this device tells the room about its pointer, selection, tool and drag, and the other
//! participants' marks as the canvas overlay draws them.
//!
//! Presence is drawn, never written: nothing here touches the document or the history.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot, Vec2, ViewTransform};
use curvyo_sync_core::Millis;
use curvyo_ui_core::object_bounds;
use curvyo_ui_core::presence::{PRESENCE_VERSION, PresencePayload};
use curvyo_ui_core::share_text::{editing_marker, participant_colour};
use serde_json::{Value, json};

use super::{Session, Tool};

/// The tool's name in a presence payload (the host's names).
const fn tool_word(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "select",
        Tool::Pen => "pen",
        Tool::Node => "node",
        Tool::Rectangle => "rectangle",
        Tool::Ellipse => "ellipse",
        Tool::PolygonStar => "polygon-star",
        Tool::Offset => "offset",
        Tool::Measure => "measure",
        Tool::Text => "text",
        Tool::Trace => "trace",
        Tool::TestGrid => "test-grid",
    }
}

/// One other participant as the overlay draws it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PeerMark {
    pub(super) label: String,
    pub(super) colour: u8,
    pub(super) tool: String,
    /// The pointer in canvas CSS pixels, `None` while it is off the canvas.
    pub(super) pointer: Option<(f64, f64)>,
    /// The selection's union box in canvas CSS pixels.
    pub(super) selection: Option<[f64; 4]>,
    /// The local pointer is on, or the local selection holds, an object of this selection
    /// (criterion 15).
    pub(super) contested: bool,
}

impl Session {
    /// This device's presence now.
    fn own_presence(&self) -> PresencePayload {
        let pointer = self.pointer_position.map(|p| [p.x, p.y]);
        let drag = (self.tool == Tool::Select)
            .then(|| {
                let cursor = self.pointer_position?;
                let live = self.select.live_move_snapped(
                    cursor,
                    self.held.shift,
                    self.held.ctrl && !self.held.alt,
                    self.snap_result().map_or(Vec2::ZERO, |r| r.offset),
                )?;
                // A copy leaves the originals where they are.
                (!live.copy).then_some([live.offset.x, live.offset.y])
            })
            .flatten();
        PresencePayload {
            v: PRESENCE_VERSION,
            label: self
                .sync
                .as_ref()
                .map(|s| s.label.clone())
                .unwrap_or_default(),
            peer: self.peer.to_string(),
            pointer,
            selection: self.selection.ids().to_vec(),
            tool: tool_word(self.tool).to_owned(),
            drag,
            document: self.presence_document(),
            list_peer: self
                .workspace_channel()
                .map(|c| c.list_peer.to_string())
                .unwrap_or_default(),
        }
    }

    /// The open document's id as hex inside a shared workspace, empty otherwise: marks are
    /// drawn only between sessions of one document (`specs/0095-workspace`).
    fn presence_document(&self) -> String {
        self.workspace_channel()
            .and_then(|c| c.stream)
            .map(|stream| super::sync::hex(&stream))
            .unwrap_or_default()
    }

    /// Hands the session this device's presence when it changed; the session throttles it.
    pub(super) fn send_presence(&mut self, now: Millis) {
        let Some(link) = self.sync.as_ref() else {
            return;
        };
        if link.ended.is_some() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec(&self.own_presence()) else {
            return;
        };
        if let Some(link) = self.sync.as_mut()
            && link.last_presence != bytes
        {
            link.last_presence.clone_from(&bytes);
            link.client.set_presence(bytes, now);
        }
    }

    /// The live label of the participant whose session has Loro peer id `peer`, for the History
    /// tab (adrs.md decision 4).
    pub(super) fn live_peer_label(&self, peer: u64) -> Option<String> {
        let link = self.sync.as_ref()?;
        link.peers.values().find_map(|p| {
            let presence = p.presence.as_ref()?;
            (presence.peer_id() == Some(peer)).then(|| presence.label.clone())
        })
    }

    /// The objects with every other participant's move drag in flight applied (criterion 7):
    /// the dragged objects are drawn where their drag has them.
    pub(super) fn with_remote_drags(
        &self,
        objects: Arc<[ObjectSnapshot]>,
    ) -> Arc<[ObjectSnapshot]> {
        let Some(link) = self.sync.as_ref() else {
            return objects;
        };
        let mut offsets: HashMap<NodeId, Vec2> = HashMap::new();
        let document = self.presence_document();
        for presence in link
            .peers
            .values()
            .filter_map(|p| p.presence.as_ref())
            .filter(|p| p.document == document)
        {
            if let Some([dx, dy]) = presence.drag {
                for id in &presence.selection {
                    offsets.insert(*id, Vec2::new(dx, dy));
                }
            }
        }
        if offsets.is_empty() {
            return objects;
        }
        objects
            .iter()
            .map(|o| {
                offsets
                    .get(&o.id())
                    .map_or_else(|| o.clone(), |v| o.translated(*v))
            })
            .collect()
    }

    /// The other participants' marks for this frame, in canvas CSS pixels.
    pub(super) fn peer_marks(&self) -> Vec<PeerMark> {
        let Some(link) = self.sync.as_ref() else {
            return Vec::new();
        };
        if link.ended.is_some() || !link.online {
            return Vec::new();
        }
        let view = self.view();
        let objects = self.objects();
        let by_id: HashMap<NodeId, &ObjectSnapshot> = objects.iter().map(|o| (o.id(), o)).collect();
        let mine: HashSet<NodeId> = self.selection.ids().iter().copied().collect();
        let pressed_or_hovered = self.hovered_object;
        let mut marks = Vec::new();
        let document = self.presence_document();
        for peer in link.peers.values() {
            let Some(presence) = peer.presence.as_ref().filter(|p| p.document == document) else {
                continue;
            };
            let fingerprint = peer
                .member
                .and_then(|m| self.participant_fingerprint(&m))
                .unwrap_or_default();
            let selection = selection_box(presence, &by_id, &view);
            let contested = presence
                .selection
                .iter()
                .any(|id| mine.contains(id) || pressed_or_hovered == Some(*id));
            marks.push(PeerMark {
                label: presence.label.clone(),
                colour: participant_colour(&fingerprint),
                tool: presence.tool.clone(),
                pointer: presence
                    .pointer
                    .map(|[x, y]| view.document_to_screen(curvyo_document_core::Point::new(x, y))),
                selection,
                contested,
            });
        }
        marks
    }

    /// The other participants' marks (pointer, selection box, editing marker) as the overlay reads
    /// them: one JSON array.
    #[must_use]
    pub fn presence_marks_json(&self) -> String {
        let marks: Vec<Value> = self
            .peer_marks()
            .into_iter()
            .map(|m| {
                json!({
                    "label": m.label,
                    "colour": m.colour,
                    "tool": m.tool,
                    "pointer": m.pointer.map(|(x, y)| [x, y]),
                    "selection": m.selection,
                    "marker": m.contested.then(|| editing_marker(Some(&m.label))),
                })
            })
            .collect();
        Value::Array(marks).to_string()
    }

    /// The keyboard route of criterion 15: the notice "Ben is editing this." once when the local
    /// selection gains an object another participant has selected.
    pub fn take_contested_notice(&mut self) -> Option<String> {
        let mine: Vec<NodeId> = self.selection.ids().to_vec();
        let link = self.sync.as_mut()?;
        let label = link
            .online
            .then(|| {
                link.peers
                    .values()
                    .filter_map(|p| p.presence.as_ref())
                    .find(|p| p.selection.iter().any(|id| mine.contains(id)))
                    .map(|p| p.label.clone())
            })
            .flatten();
        match label {
            None => {
                link.contested_for.clear();
                None
            }
            Some(_) if link.contested_for == mine => None,
            Some(label) => {
                link.contested_for = mine;
                Some(format!("{}.", editing_marker(Some(&label))))
            }
        }
    }
}

impl Session {
    /// For an undo in a shared document that wrote nothing because every object it touched was
    /// deleted later: the notice naming who deleted them ("Someone" when unknown). `None`
    /// outside that case, so an unshared document keeps its notices (criterion 23).
    pub(super) fn deleted_by_peer_notice(
        &mut self,
        report: &curvyo_document_core::RestoreReport,
    ) -> Option<String> {
        use curvyo_document_core::{Author, ObjectResult, Role, SkipReason};
        self.sync.as_ref()?;
        let all_deleted = !report.wrote
            && !report.objects.is_empty()
            && report
                .objects
                .iter()
                .all(|o| o.result == ObjectResult::Skipped(SkipReason::DeletedLater));
        if !all_deleted {
            return None;
        }
        let ids: HashSet<NodeId> = report.objects.iter().map(|o| o.id).collect();
        // The newest step of another session that deleted one of them names the participant.
        let peer = self
            .document
            .history_page(0, 200)
            .into_iter()
            .find(|row| {
                row.author != Author::You
                    && row
                        .objects
                        .iter()
                        .any(|o| o.role == Role::Deleted && ids.contains(&o.id))
            })
            .map(|row| row.peer);
        let label = peer.and_then(|p| self.live_peer_label(p));
        Some(curvyo_ui_core::share_text::skipped_deleted_notice(
            label.as_deref(),
        ))
    }

    /// "Made from version a1b2c3d." for an export of a shared document (criterion 29): the
    /// version is taken in the one call that writes the export, so no remote edit reaches it
    /// (criterion 30). Empty when the document is not shared.
    #[must_use]
    pub fn export_version_note(&mut self) -> String {
        if !self.is_shared() {
            return String::new();
        }
        let id = self.history_now_id();
        if id.is_empty() {
            String::new()
        } else {
            format!("Made from version {id}.")
        }
    }
}

/// The screen box around the objects `presence` has selected, moved by its drag, as
/// `[x, y, width, height]` in canvas CSS pixels; `None` when none of them is in the document.
fn selection_box(
    presence: &PresencePayload,
    by_id: &HashMap<NodeId, &ObjectSnapshot>,
    view: &ViewTransform,
) -> Option<[f64; 4]> {
    let offset = presence.drag.map_or(Vec2::ZERO, |[x, y]| Vec2::new(x, y));
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for object in presence.selection.iter().filter_map(|id| by_id.get(id)) {
        let (low, high) = object_bounds(object);
        let low = low.translated(offset);
        let high = high.translated(offset);
        bounds = Some(bounds.map_or((low.x, low.y, high.x, high.y), |b| {
            (
                b.0.min(low.x),
                b.1.min(low.y),
                b.2.max(high.x),
                b.3.max(high.y),
            )
        }));
    }
    let (x0, y0, x1, y1) = bounds?;
    let (sx0, sy0) = view.document_to_screen(curvyo_document_core::Point::new(x0, y0));
    let (sx1, sy1) = view.document_to_screen(curvyo_document_core::Point::new(x1, y1));
    Some([
        sx0.min(sx1),
        sy0.min(sy1),
        (sx1 - sx0).abs(),
        (sy1 - sy0).abs(),
    ])
}
