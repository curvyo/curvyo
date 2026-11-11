//! The background writer of a shared workspace (`specs/0095-workspace` criteria 31 and 32, ADR
//! 0037 decision 8): records of documents that are not open wait per document and are written
//! into the document's file at most every 5 s **(Proposal)**, by unpacking the file, importing
//! the records and packing it again. It runs in a Web Worker with its own wasm instance, so the
//! open document's frames never wait for it; the page reads and writes the files.

use std::collections::BTreeMap;

use curvyo_document_core::{Document, DocumentVersion, WorkspaceShareRecord, pack, unpack};

use crate::session::{APP_VERSION, BackgroundKind};

/// A document is written at most this often (ms).
pub const WRITE_INTERVAL_MS: u64 = 5_000;

/// Why a file was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// Width profile data cannot yet be sent to older collaboration peers.
    UnsupportedDocument,
    /// A queued record could not be imported without losing or changing data.
    RejectedRecord,
    /// The file is no container this build opens.
    Unreadable,
    /// The document could not be packed again.
    Pack,
}

impl core::fmt::Display for WriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::UnsupportedDocument => {
                "documents with editable curves or width profiles cannot be shared yet"
            }
            Self::RejectedRecord => "a workspace update could not be read; it remains queued",
            Self::Unreadable => "the document file cannot be read",
            Self::Pack => "the document could not be written",
        })
    }
}

impl std::error::Error for WriteError {}

/// A document's file with its waiting records written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The new file bytes.
    pub bytes: Vec<u8>,
    /// The highest sequence they hold.
    pub through: u64,
    /// The file's own changes the room has not had (edits made here while the document was not
    /// open on a link, or a copy connected by Connect this folder): the page pushes them as
    /// the document's records ([`Session::workspace_send_local`](crate::Session::workspace_send_local)).
    /// While they are not sent, the file's `share.json` says they are not.
    pub outgoing: Option<Vec<u8>>,
}

/// One record waiting for its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    /// The room sequence.
    pub seq: u64,
    /// What it carries.
    pub kind: BackgroundKind,
    /// The payload.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
struct Queue {
    records: Vec<Waiting>,
    /// When its file was last written (ms), `None` before the first write.
    written_at: Option<u64>,
    /// Asked for the file and not answered yet.
    asked: bool,
}

/// The records waiting per document, and when each document is due.
#[derive(Debug)]
pub struct BackgroundWriter {
    peer: u64,
    queues: BTreeMap<[u8; 16], Queue>,
}

impl BackgroundWriter {
    /// A writer whose unpacked documents use the Loro peer `peer` (they never commit).
    #[must_use]
    pub fn new(peer: u64) -> Self {
        Self {
            peer,
            queues: BTreeMap::new(),
        }
    }

    /// A record of `document` arrived.
    pub fn add(&mut self, document: [u8; 16], record: Waiting) {
        self.queues
            .entry(document)
            .or_default()
            .records
            .push(record);
    }

    /// The documents whose file to ask for now: records wait and the last write lies at least
    /// [`WRITE_INTERVAL_MS`] back. Each is asked once until its answer.
    pub fn due(&mut self, now: u64) -> Vec<[u8; 16]> {
        let mut due = Vec::new();
        for (document, queue) in &mut self.queues {
            let rested = queue
                .written_at
                .is_none_or(|at| now.saturating_sub(at) >= WRITE_INTERVAL_MS);
            if !queue.records.is_empty() && rested && !queue.asked {
                queue.asked = true;
                due.push(*document);
            }
        }
        due
    }

    /// Drops what waits for `document` (it is open now, or its file is gone); returns the
    /// highest sequence dropped.
    pub fn forget(&mut self, document: [u8; 16]) -> Option<u64> {
        let queue = self.queues.remove(&document)?;
        queue.records.iter().map(|r| r.seq).max()
    }

    /// Writes every waiting record of `document` into its file's `bytes` at `now`: the new
    /// bytes and the highest sequence they hold ([`write_for`](Self::write_for) with the
    /// workspace the file's own record names, if any). The records stay waiting when it fails.
    ///
    /// # Errors
    /// [`WriteError::Unreadable`] for bytes that do not open, [`WriteError::Pack`].
    pub fn write(
        &mut self,
        document: [u8; 16],
        bytes: &[u8],
        now: u64,
    ) -> Result<(Vec<u8>, u64), WriteError> {
        let workspace = unpack(self.peer, bytes)
            .ok()
            .and_then(|file| file.workspace_share())
            .filter(|record| record.document == document)
            .map(|record| record.workspace);
        self.write_into(document, workspace, bytes, now)
            .map(|written| (written.bytes, written.through))
    }

    /// Writes every waiting record of `document`, a document of `workspace`, into its file's
    /// `bytes` at `now`. The file keeps (or, for a copy from before the workspace was shared,
    /// gets) its `share.json` v2; changes of the file the room lacks come back as
    /// [`Written::outgoing`]. The records stay waiting when it fails.
    ///
    /// # Errors
    /// [`WriteError::Unreadable`] for bytes that do not open, [`WriteError::Pack`].
    pub fn write_for(
        &mut self,
        document: [u8; 16],
        workspace: [u8; 16],
        bytes: &[u8],
        now: u64,
    ) -> Result<Written, WriteError> {
        self.write_into(document, Some(workspace), bytes, now)
    }

    fn write_into(
        &mut self,
        document: [u8; 16],
        workspace: Option<[u8; 16]>,
        bytes: &[u8],
        now: u64,
    ) -> Result<Written, WriteError> {
        let queue = self.queues.entry(document).or_default();
        queue.asked = false;
        let mut records = std::mem::take(&mut queue.records);
        records.sort_by_key(|record| record.seq);
        let Some(through) = records.iter().map(|r| r.seq).max() else {
            return Ok(Written {
                bytes: bytes.to_vec(),
                through: 0,
                outgoing: None,
            });
        };
        match apply(self.peer, (workspace, document), bytes, &records) {
            Ok((written, outgoing)) => {
                queue.written_at = Some(now);
                Ok(Written {
                    bytes: written,
                    through,
                    outgoing,
                })
            }
            Err(error) => {
                // Tried again after the interval, not at once.
                queue.written_at = Some(now);
                queue.records = records;
                Err(error)
            }
        }
    }

    /// Writes the first file of `document`, a document of `workspace` this computer holds no
    /// file of yet (another member created it, `specs/0095-workspace` criterion 35, or Download
    /// again): built from its records, a snapshot's container or the updates of its stream from
    /// its creation, with `share.json` v2 naming every imported change as synced. The records
    /// stay waiting when they do not make a document yet.
    ///
    /// # Errors
    /// [`WriteError::Unreadable`] while no record starts the document, [`WriteError::Pack`].
    pub fn create(
        &mut self,
        document: [u8; 16],
        workspace: [u8; 16],
        now: u64,
    ) -> Result<(Vec<u8>, u64), WriteError> {
        let queue = self.queues.entry(document).or_default();
        queue.asked = false;
        queue.written_at = Some(now);
        let mut records = queue.records.clone();
        records.sort_by_key(|r| r.seq);
        let through = records.iter().map(|r| r.seq).max().unwrap_or(0);
        let built = start_document(self.peer, &records)?.ok_or(WriteError::Unreadable)?;
        import_records(self.peer, &built, &records)?;
        if built.has_extended_path_history() {
            return Err(WriteError::UnsupportedDocument);
        }
        built.set_workspace_share(Some(WorkspaceShareRecord {
            workspace,
            document,
            synced_version: built.version(),
        }));
        let bytes = pack(&built, APP_VERSION).map_err(|_| WriteError::Pack)?;
        if let Some(queue) = self.queues.get_mut(&document) {
            queue.records.clear();
        }
        Ok((bytes, through))
    }

    /// The file of `document` is not there yet: it is asked for again after the interval.
    pub fn postpone(&mut self, document: [u8; 16], now: u64) {
        if let Some(queue) = self.queues.get_mut(&document) {
            queue.asked = false;
            queue.written_at = Some(now);
        }
    }

    /// Whether anything waits.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.queues.values().all(|q| q.records.is_empty())
    }
}

/// The file `bytes` of the document `(workspace, document)` with `records` imported, and the
/// file's own changes the room lacks. An unreadable record leaves the whole queue pending.
/// Without a `workspace` (a file of no workspace
/// record) the file is only merged.
///
/// The file's record says what the room has: its `synced_version`, or, for a file without a
/// record of this workspace (a copy from before it was shared, criterion 14), the state the
/// records alone build. A file with nothing of its own is marked synced up to the merge; one
/// with changes of its own keeps that mark until the page has sent them.
fn apply(
    peer: u64,
    (workspace, document): (Option<[u8; 16]>, [u8; 16]),
    bytes: &[u8],
    records: &[Waiting],
) -> Result<(Vec<u8>, Option<Vec<u8>>), WriteError> {
    let file = unpack(peer, bytes).map_err(|_| WriteError::Unreadable)?;
    if workspace.is_some() && file.has_extended_path_history() {
        return Err(WriteError::UnsupportedDocument);
    }
    let Some(workspace) = workspace else {
        import_records(peer, &file, records)?;
        return Ok((
            pack(&file, APP_VERSION).map_err(|_| WriteError::Pack)?,
            None,
        ));
    };
    let record = file
        .workspace_share()
        .filter(|r| r.workspace == workspace && r.document == document);
    let own = match &record {
        Some(record) if file.version() != record.synced_version => {
            let updates = file
                .export_updates(&record.synced_version)
                .map_err(|_| WriteError::Pack)?;
            Some((record.synced_version.clone(), updates))
        }
        _ => None,
    };
    import_records(peer, &file, records)?;
    if file.has_extended_path_history() {
        return Err(WriteError::UnsupportedDocument);
    }
    let (synced, outgoing) = match (record, own) {
        (Some(_), Some((synced, updates))) => (synced, Some(updates)),
        (Some(_), None) => (file.version(), None),
        (None, _) => {
            let room = room_version(peer, records)?;
            if file.version() == room {
                (room, None)
            } else {
                let updates = file.export_updates(&room).map_err(|_| WriteError::Pack)?;
                (room, Some(updates))
            }
        }
    };
    file.set_workspace_share(Some(WorkspaceShareRecord {
        workspace,
        document,
        synced_version: synced,
    }));
    let bytes = pack(&file, APP_VERSION).map_err(|_| WriteError::Pack)?;
    Ok((bytes, outgoing))
}

/// The version the records alone build: what the room holds of the document, as far as these
/// records show it ([`DocumentVersion::never`] when they start none).
fn room_version(peer: u64, records: &[Waiting]) -> Result<DocumentVersion, WriteError> {
    let mut sorted = records.to_vec();
    sorted.sort_by_key(|r| r.seq);
    let Some(room) = start_document(peer.wrapping_add(2), &sorted)? else {
        return Ok(DocumentVersion::never());
    };
    import_records(peer.wrapping_add(2), &room, &sorted)?;
    Ok(room.version())
}

/// The document the first container among `records`, else the first update, starts.
fn start_document(peer: u64, records: &[Waiting]) -> Result<Option<Document>, WriteError> {
    if let Some(container) = records.iter().find(|r| r.kind == BackgroundKind::Container) {
        return unpack(peer, &container.bytes)
            .map(Some)
            .map_err(|_| WriteError::RejectedRecord);
    }
    let Some(first) = records.iter().find(|r| r.kind == BackgroundKind::Update) else {
        return Ok(None);
    };
    match Document::from_updates(peer, &first.bytes) {
        Ok(document) => Ok(Some(document)),
        Err(curvyo_document_core::SyncError::Pending) => Ok(None),
        Err(_) => Err(WriteError::RejectedRecord),
    }
}

/// Imports every record, refusing the batch if any record cannot be imported.
fn import_records(peer: u64, document: &Document, records: &[Waiting]) -> Result<(), WriteError> {
    for record in records {
        match record.kind {
            BackgroundKind::Update => {
                document
                    .import_updates(&record.bytes)
                    .map_err(|_| WriteError::RejectedRecord)?;
            }
            BackgroundKind::Asset => {
                document
                    .insert_asset(&record.bytes)
                    .map_err(|_| WriteError::RejectedRecord)?;
            }
            BackgroundKind::Container => {
                let received = unpack(peer.wrapping_add(1), &record.bytes)
                    .map_err(|_| WriteError::RejectedRecord)?;
                for hash in image_assets(&received) {
                    if let Some(picture) = received.asset(&hash) {
                        document
                            .insert_asset(&picture)
                            .map_err(|_| WriteError::RejectedRecord)?;
                    }
                }
                let all = received
                    .export_updates(&DocumentVersion::never())
                    .map_err(|_| WriteError::RejectedRecord)?;
                document
                    .import_updates(&all)
                    .map_err(|_| WriteError::RejectedRecord)?;
            }
        }
    }
    Ok(())
}

/// The assets the images of `document` show.
pub(crate) fn image_assets(document: &Document) -> Vec<curvyo_document_core::AssetHash> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(image) => Some(image.asset.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Length, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) {
        let _ = document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(5.0),
            height: Length::from_mm(5.0),
        });
    }

    /// A file and an update that adds one rectangle to it.
    fn file_and_update() -> (Vec<u8>, Vec<u8>) {
        let document = Document::new(3);
        rect(&document, 0.0);
        document.set_workspace_share(Some(WorkspaceShareRecord {
            workspace: [1; 16],
            document: [2; 16],
            synced_version: document.version(),
        }));
        let file = pack(&document, "test").unwrap();
        let before = document.version();
        rect(&document, 10.0);
        (file, document.export_updates(&before).unwrap())
    }

    #[test]
    fn a_profiled_file_cannot_emit_background_updates() {
        use curvyo_document_core::{AnchorId, NewAnchor};
        let document = Document::new(3);
        let path = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(3, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(3, 2), Point::new(10.0, 0.0)),
            ],
            false,
        );
        document
            .set_width_point(path, AnchorId::new(3, 3), 0.5, 2.0)
            .unwrap();
        let bytes = pack(&document, "test").unwrap();
        assert!(matches!(
            apply(9, (Some([1; 16]), [2; 16]), &bytes, &[]),
            Err(WriteError::UnsupportedDocument)
        ));
    }

    #[test]
    fn incoming_extended_history_cannot_be_reexported_by_background_writer() {
        use curvyo_document_core::{AnchorId, NewAnchor};
        let document = Document::new(3);
        let path = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(3, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(3, 2), Point::new(10.0, 0.0)),
            ],
            false,
        );
        let file = pack(&document, "test").unwrap();
        let before = document.version();
        document
            .set_width_point(path, AnchorId::new(3, 3), 0.5, 2.0)
            .unwrap();
        let records = [Waiting {
            seq: 1,
            kind: BackgroundKind::Update,
            bytes: document.export_updates(&before).unwrap(),
        }];
        assert!(matches!(
            apply(9, (Some([1; 16]), [2; 16]), &file, &records),
            Err(WriteError::UnsupportedDocument)
        ));
    }

    #[test]
    fn background_creation_rejects_extended_container_and_retains_record() {
        use curvyo_document_core::{AnchorId, CurveKind, NewAnchor};
        let document = Document::new(3);
        document
            .create_editable_curve(
                &[
                    NewAnchor::corner(AnchorId::new(3, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(3, 2), Point::new(10.0, 0.0)),
                ],
                false,
                CurveKind::BSpline,
            )
            .unwrap();
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 1,
                kind: BackgroundKind::Container,
                bytes: pack(&document, "test").unwrap(),
            },
        );
        assert!(matches!(
            writer.create([2; 16], [1; 16], 0),
            Err(WriteError::UnsupportedDocument)
        ));
        assert!(!writer.is_idle());
    }

    #[test]
    fn future_format_record_preserves_the_whole_background_queue() {
        use loro::{ExportMode, LoroDoc};
        let (file, normal_update) = file_and_update();
        let base = unpack(12, &file).unwrap();
        let all = base.export_updates(&DocumentVersion::never()).unwrap();
        let raw = LoroDoc::new();
        raw.import(&all[4..]).unwrap();
        let before = raw.oplog_vv();
        raw.set_peer_id(99).unwrap();
        raw.get_map("root")
            .insert(
                "format_version",
                i64::from(curvyo_document_core::CURRENT_FORMAT_VERSION) + 1,
            )
            .unwrap();
        raw.commit();
        let mut update = vec![0, 0, 0, 0];
        update.extend(raw.export(ExportMode::updates(&before)).unwrap());
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 1,
                kind: BackgroundKind::Update,
                bytes: normal_update,
            },
        );
        writer.add(
            [2; 16],
            Waiting {
                seq: 2,
                kind: BackgroundKind::Update,
                bytes: update.clone(),
            },
        );
        assert!(writer.write_for([2; 16], [1; 16], &file, 0).is_err());
        assert_eq!(writer.queues.get(&[2; 16]).unwrap().records.len(), 2);
        assert_eq!(writer.forget([2; 16]), Some(2));
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 1,
                kind: BackgroundKind::Container,
                bytes: file,
            },
        );
        writer.add(
            [2; 16],
            Waiting {
                seq: 2,
                kind: BackgroundKind::Update,
                bytes: update,
            },
        );
        assert_eq!(
            writer.create([2; 16], [1; 16], 0),
            Err(WriteError::RejectedRecord)
        );
        assert_eq!(writer.queues.get(&[2; 16]).unwrap().records.len(), 2);
    }

    #[test]
    fn unreadable_background_snapshot_is_reported_and_retained() {
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 7,
                kind: BackgroundKind::Container,
                bytes: b"invalid".to_vec(),
            },
        );
        assert_eq!(
            writer.create([2; 16], [1; 16], 0),
            Err(WriteError::RejectedRecord)
        );
        assert_eq!(writer.forget([2; 16]), Some(7));
    }

    #[test]
    fn reordered_updates_are_applied_in_sequence() {
        let document = Document::new(45);
        let file = pack(&document, "test").unwrap();
        let before = document.version();
        rect(&document, 0.0);
        let first = document.export_updates(&before).unwrap();
        let after_first = document.version();
        rect(&document, 10.0);
        let second = document.export_updates(&after_first).unwrap();
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 2,
                kind: BackgroundKind::Update,
                bytes: second,
            },
        );
        assert_eq!(
            writer.write_for([2; 16], [1; 16], &file, 0),
            Err(WriteError::RejectedRecord)
        );
        writer.add(
            [2; 16],
            Waiting {
                seq: 1,
                kind: BackgroundKind::Update,
                bytes: first,
            },
        );
        let written = writer.write_for([2; 16], [1; 16], &file, 5_000).unwrap();
        assert_eq!(written.through, 2);
        assert_eq!(
            unpack(50, &written.bytes)
                .unwrap()
                .read_objects()
                .objects
                .len(),
            2
        );
    }

    /// Criterion 32: a document's records are written into its file, which keeps its
    /// `share.json` v2; the next write waits 5 s.
    #[test]
    fn records_go_into_the_file_at_most_every_five_seconds() {
        let (file, update) = file_and_update();
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 7,
                kind: BackgroundKind::Update,
                bytes: update.clone(),
            },
        );
        assert_eq!(writer.due(1_000), vec![[2; 16]]);
        assert!(writer.due(1_000).is_empty(), "asked once");
        let Written {
            bytes: written,
            through,
            outgoing,
        } = writer.write_for([2; 16], [1; 16], &file, 1_000).unwrap();
        assert_eq!(outgoing, None, "nothing of its own");
        assert_eq!(through, 7);
        let opened = unpack(5, &written).unwrap();
        assert_eq!(opened.read_objects().objects.len(), 2);
        assert_eq!(opened.workspace_share().unwrap().document, [2; 16]);
        assert!(writer.is_idle());
        writer.add(
            [2; 16],
            Waiting {
                seq: 8,
                kind: BackgroundKind::Update,
                bytes: update,
            },
        );
        assert!(writer.due(3_000).is_empty(), "within 5 s of the last write");
        assert_eq!(writer.due(6_000), vec![[2; 16]]);
    }

    /// Criterion 35: a document another member created is written from its records, the updates
    /// of its stream from its creation; records that do not start a document keep waiting.
    #[test]
    fn a_new_document_is_built_from_its_records() {
        let created = Document::new(3);
        rect(&created, 0.0);
        let first = created.export_updates(&DocumentVersion::never()).unwrap();
        let before = created.version();
        rect(&created, 10.0);
        let second = created.export_updates(&before).unwrap();
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 5,
                kind: BackgroundKind::Update,
                bytes: second.clone(),
            },
        );
        assert_eq!(
            writer.create([2; 16], [1; 16], 0),
            Err(WriteError::Unreadable)
        );
        assert!(!writer.is_idle(), "the records keep waiting");
        writer.postpone([2; 16], 0);
        assert!(
            writer.due(1_000).is_empty(),
            "asked again after the interval"
        );
        writer.add(
            [2; 16],
            Waiting {
                seq: 4,
                kind: BackgroundKind::Update,
                bytes: first,
            },
        );
        let (bytes, through) = writer.create([2; 16], [1; 16], 6_000).unwrap();
        assert_eq!(through, 5);
        let opened = unpack(5, &bytes).unwrap();
        assert_eq!(opened.read_objects().objects.len(), 2);
        let record = opened.workspace_share().unwrap();
        assert_eq!((record.workspace, record.document), ([1; 16], [2; 16]));
        assert_eq!(record.synced_version, created.version());
        assert!(writer.is_idle());
    }

    /// A container from a snapshot merges into the file; a file that does not open keeps the
    /// records waiting; forgetting a document drops them.
    #[test]
    fn containers_merge_and_failures_keep_the_records() {
        let (file, _) = file_and_update();
        let other = Document::new(4);
        rect(&other, 20.0);
        let container = pack(&other, "test").unwrap();
        let mut writer = BackgroundWriter::new(9);
        writer.add(
            [2; 16],
            Waiting {
                seq: 3,
                kind: BackgroundKind::Container,
                bytes: container,
            },
        );
        assert_eq!(
            writer.write([2; 16], b"junk", 0),
            Err(WriteError::Unreadable)
        );
        assert!(!writer.is_idle());
        let (written, _) = writer.write([2; 16], &file, 0).unwrap();
        assert_eq!(unpack(5, &written).unwrap().read_objects().objects.len(), 2);
        writer.add(
            [3; 16],
            Waiting {
                seq: 9,
                kind: BackgroundKind::Asset,
                bytes: vec![1],
            },
        );
        assert_eq!(writer.forget([3; 16]), Some(9));
        assert!(writer.is_idle());
    }
}
