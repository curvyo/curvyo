//! The font entries of the device group's library stream as the glue drives them (ADR 0062
//! decisions 2 to 6, `specs/0116-library-sync` criteria 16 to 19): the entries the page sets,
//! the blobs it supplies, and what arrives for the page to write to its fonts folder.
//!
//! The font bytes stay with the host; the glue holds them in memory only for the fonts whose
//! sync is on (plan P24), because the room's snapshot has to carry them (plan P23). A snapshot
//! is not uploaded while the bytes of a switched-on font are still arriving as parts (the relay
//! would prune the part records that complete them), but a font whose bytes never come does not
//! hold it back. What arrives is counted against the library budget (ADR 0062 decision 6).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use curvyo_document_core::{
    AssetHash, CuttableChoice, FontBudgetError, FontDetails, FontDocument, FontEntry,
    MAX_SYNCED_FONT_BYTES, MAX_SYNCED_FONTS_BYTES,
};
use curvyo_sync_core::{BlobAssembler, Millis, RecordKind, encode_blob_parts};
use curvyo_ui_core::library_text as texts;

use super::DeviceGroupGlue;

/// How long a half-arrived blob may stay quiet, or the page may take to supply a hash, before the
/// glue stops waiting for it (Proposal).
const ASSEMBLY_PATIENCE_MS: u64 = 10 * 60 * 1000;

/// The longest a snapshot waits for the bytes of one hash, however often the sender starts the
/// blob again (Proposal).
const TOTAL_WAIT_MS: u64 = 30 * 60 * 1000;

/// A blob arriving as parts.
#[derive(Debug)]
pub(super) struct Assembly {
    pub(super) assembler: BlobAssembler,
    /// The time of its latest part.
    pub(super) last_at: Millis,
    /// The time of its first part (kept when the sender starts the blob again).
    pub(super) first_at: Millis,
}

/// What the page gives the glue about a font it switches on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontInput {
    /// The file name, no folder.
    pub file_name: String,
    /// The `.zip` subfolder of the fonts folder.
    pub folder: Option<String>,
    /// The travelling tags of the file's first family.
    pub tags: Vec<String>,
    /// Whether the family is starred.
    pub favourite: bool,
    /// The Cuttable override.
    pub cuttable: CuttableChoice,
    /// The kept licence and readme files with their bytes.
    pub documents: Vec<(String, Vec<u8>)>,
}

/// One change for the page's fonts folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontChange {
    /// A font new to this host: write its file and documents, then apply the metadata.
    Add(FontDetails),
    /// A font this host holds: apply the metadata.
    Meta(FontDetails),
    /// The font was deleted elsewhere: delete the local copy (criterion 11).
    Removed,
}

/// A change of one font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontArrival {
    /// The file's SHA-256.
    pub hash: AssetHash,
    /// What to do.
    pub change: FontChange,
}

/// The state the glue keeps for the font stream.
#[derive(Debug)]
pub(super) struct FontSync {
    /// The bytes of switched-on fonts and their documents, by hash.
    pub(super) blobs: BTreeMap<AssetHash, Vec<u8>>,
    /// Blobs arriving as parts, by hash.
    pub(super) assembling: BTreeMap<[u8; 32], Assembly>,
    /// The most bytes held and arriving together (the library budget; a field so tests can
    /// lower it).
    pub(super) limit: u64,
    /// Hashes whose blob records still have to go out (while offline).
    pub(super) to_push: Vec<AssetHash>,
    /// Client ids of blob records waiting for the relay's ack.
    pub(super) pending_acks: BTreeSet<u64>,
    /// The hashes this host holds as font files, as the page last said.
    pub(super) host_has: BTreeSet<AssetHash>,
    /// Entries already handed to the page since they were switched on.
    pub(super) delivered: BTreeSet<AssetHash>,
    /// Metadata and removals queued by imports.
    pub(super) queued: VecDeque<FontArrival>,
    /// Licence files the page staged for the next switch-on.
    pub(super) staged: Vec<(String, Vec<u8>)>,
    /// Metadata fields changed here since an import last proved the room knew every local
    /// change (a concurrent remote change to one of them is a lost edit, criterion 17).
    pub(super) dirty: BTreeSet<(AssetHash, &'static str)>,
    /// A snapshot was due while the bytes of a font were still arriving.
    pub(super) snapshot_deferred: bool,
    /// Since when the page has been asked to supply each hash it holds and the glue lacks.
    pub(super) want_since: BTreeMap<AssetHash, Millis>,
}

impl Default for FontSync {
    fn default() -> Self {
        Self {
            blobs: BTreeMap::new(),
            assembling: BTreeMap::new(),
            limit: MAX_SYNCED_FONTS_BYTES,
            to_push: Vec::new(),
            pending_acks: BTreeSet::new(),
            host_has: BTreeSet::new(),
            delivered: BTreeSet::new(),
            queued: VecDeque::new(),
            staged: Vec::new(),
            dirty: BTreeSet::new(),
            snapshot_deferred: false,
            want_since: BTreeMap::new(),
        }
    }
}

pub(super) type FontState = BTreeMap<AssetHash, FontEntry>;

pub(super) fn state_of(entries: Vec<FontEntry>) -> FontState {
    entries.into_iter().map(|e| (e.hash.clone(), e)).collect()
}

/// The name a font arrives under in a notice: its file name without the extension.
pub(super) fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

impl FontSync {
    pub(super) fn ack(&mut self, client_id: u64) {
        self.pending_acks.remove(&client_id);
    }

    pub(super) fn idle(&self) -> bool {
        self.to_push.is_empty() && self.pending_acks.is_empty()
    }
}

impl DeviceGroupGlue {
    /// Hands the glue the licence file `name` of the font the next [`font_set`] switches on.
    ///
    /// [`font_set`]: DeviceGroupGlue::font_set
    pub fn font_stage_document(&mut self, name: &str, bytes: Vec<u8>) {
        self.fonts.staged.push((name.to_owned(), bytes));
    }

    /// Switches the font with `bytes` on (criteria 3, 16, 18, 19): the entry, then the blob. A
    /// font over a budget writes nothing. Returns the hash, or the refusal's notice text.
    ///
    /// # Errors
    /// The notice text of a refusal (criterion 19), or of a name the list refuses.
    pub fn font_set(&mut self, input: FontInput, bytes: Vec<u8>) -> Result<AssetHash, String> {
        let mut input = input;
        input
            .documents
            .extend(std::mem::take(&mut self.fonts.staged));
        let hash = AssetHash::of(&bytes);
        let details = FontDetails {
            file_name: input.file_name.clone(),
            folder: input.folder.clone(),
            size: bytes.len() as u64,
            tags: input.tags.clone(),
            favourite: input.favourite,
            cuttable: input.cuttable,
            documents: input
                .documents
                .iter()
                .map(|(name, doc)| FontDocument {
                    hash: AssetHash::of(doc),
                    size: doc.len() as u64,
                    name: name.clone(),
                })
                .collect(),
        };
        let shown = stem(&details.file_name).to_owned();
        self.library
            .font_check_budget(&hash, &details)
            .map_err(|refusal| match refusal {
                FontBudgetError::TooLarge { size, limit } => {
                    texts::font_too_large_notice(&shown, size, limit)
                }
                FontBudgetError::OverBudget { total, limit } => {
                    texts::font_over_budget_notice(&shown, total, limit)
                }
            })?;
        self.library
            .font_set(&hash, &details)
            .map_err(|_| texts::FONT_SKIPPED_NOTICE.to_owned())?;
        self.fonts.delivered.insert(hash.clone());
        self.fonts.blobs.insert(hash.clone(), bytes);
        let mut hashes = vec![hash.clone()];
        for ((_, doc), meta) in input.documents.into_iter().zip(&details.documents) {
            self.fonts.blobs.insert(meta.hash.clone(), doc);
            hashes.push(meta.hash.clone());
        }
        self.fonts.to_push.extend(hashes);
        self.library_changed();
        Ok(hash)
    }

    /// The metadata of the switched-on font `hash` changed here (a star, a tag, Cuttable):
    /// only the registers that differ are written (criterion 17).
    pub fn font_meta(
        &mut self,
        hash: &AssetHash,
        tags: Vec<String>,
        favourite: bool,
        cuttable: CuttableChoice,
    ) {
        let Some(mut details) = self
            .library
            .font_entry(hash)
            .and_then(|entry| entry.details)
        else {
            return;
        };
        for (field, differs) in [
            ("tags", details.tags != tags),
            ("star", details.favourite != favourite),
            ("Cuttable", details.cuttable != cuttable),
        ] {
            if differs {
                self.fonts.dirty.insert((hash.clone(), field));
            }
        }
        details.tags = tags;
        details.favourite = favourite;
        details.cuttable = cuttable;
        if self.library.font_set(hash, &details).is_ok() {
            self.library_changed();
        }
    }

    /// Switches the font `hash` off: only its marker stays, and its bytes leave the snapshot.
    pub fn font_switch_off(&mut self, hash: &AssetHash) {
        if self.library.font_entry(hash).is_some() && self.library.font_switch_off(hash).is_ok() {
            self.snapshot_after_ack = true;
            self.fonts.delivered.remove(hash);
            self.forget_blobs();
            self.library_changed();
        }
    }

    /// The font `hash` was deleted here: its entry goes if it was on (criterion 11).
    pub fn font_remove(&mut self, hash: &AssetHash) {
        let on = self
            .library
            .font_entry(hash)
            .is_some_and(|entry| entry.sync);
        if on && self.library.font_remove(hash).is_ok() {
            self.snapshot_after_ack = true;
            self.fonts.delivered.remove(hash);
            self.forget_blobs();
            self.library_changed();
        }
    }

    /// Tells the glue which font files this host holds, so it never asks for a blob twice
    /// (ADR 0062 decision 3).
    pub fn font_host_hashes(&mut self, hashes: BTreeSet<AssetHash>) {
        self.fonts.host_has = hashes;
    }

    /// The hashes of switched-on fonts (and their documents) whose bytes this host holds but
    /// the glue does not: the snapshot needs them, the page supplies them with
    /// [`font_supply_blob`](DeviceGroupGlue::font_supply_blob).
    #[must_use]
    pub fn font_wants(&self) -> Vec<AssetHash> {
        self.wanted()
            .into_iter()
            .filter(|hash| self.fonts.host_has.contains(hash))
            .collect()
    }

    /// The bytes the page read for a hash [`font_wants`](DeviceGroupGlue::font_wants) named; a
    /// blob that does not hash to what it claims is dropped.
    pub fn font_supply_blob(&mut self, hash: &AssetHash, bytes: Vec<u8>) {
        if &AssetHash::of(&bytes) != hash || !self.wanted().contains(hash) {
            return;
        }
        self.fonts.blobs.insert(hash.clone(), bytes);
        self.retry_deferred_snapshot();
    }

    /// The held bytes of `hash`, empty when none.
    #[must_use]
    pub fn font_blob(&self, hash: &AssetHash) -> &[u8] {
        self.fonts.blobs.get(hash).map_or(&[], Vec::as_slice)
    }

    /// The changes for the page's fonts folder, once: switched-on fonts new to this host whose
    /// bytes are complete (the notice counts them), metadata changes, and deletions.
    pub fn take_font_arrivals(&mut self) -> Vec<FontArrival> {
        let mut out: Vec<FontArrival> = self.fonts.queued.drain(..).collect();
        let mut added = Vec::new();
        for entry in self.library.font_entries() {
            let Some(details) = entry.details.filter(|_| entry.sync) else {
                continue;
            };
            if self.fonts.delivered.contains(&entry.hash) {
                continue;
            }
            if self.fonts.host_has.contains(&entry.hash) {
                self.fonts.delivered.insert(entry.hash.clone());
                out.push(FontArrival {
                    hash: entry.hash,
                    change: FontChange::Meta(details),
                });
            } else if self.complete(&entry.hash, &details) {
                self.fonts.delivered.insert(entry.hash.clone());
                added.push(stem(&details.file_name).to_owned());
                out.push(FontArrival {
                    hash: entry.hash,
                    change: FontChange::Add(details),
                });
            }
        }
        match added.as_slice() {
            [] => {}
            [one] => self.notices.push_back(texts::font_arrived_notice(one)),
            many => self
                .notices
                .push_back(texts::arrived_notice(many.len(), 0, 0)),
        }
        out
    }

    /// Sends the blob records still owed, once online (the entry went out with the list). A
    /// push that fails stays owed for the next try.
    pub(super) fn flush_font_blobs(&mut self) {
        if !self.online {
            return;
        }
        let mut todo = std::mem::take(&mut self.fonts.to_push).into_iter();
        while let Some(hash) = todo.next() {
            let Some(bytes) = self.fonts.blobs.get(&hash) else {
                continue;
            };
            debug_assert!(bytes.len() as u64 <= MAX_SYNCED_FONT_BYTES.max(1 << 20));
            let mut sent = true;
            for part in encode_blob_parts(&hash.to_bytes(), bytes) {
                if let Ok(id) = self.client.push(RecordKind::Asset, part) {
                    self.fonts.pending_acks.insert(id);
                } else {
                    sent = false;
                    break;
                }
            }
            if !sent {
                self.fonts.to_push.push(hash);
                self.fonts.to_push.extend(todo);
                return;
            }
        }
    }

    /// The hashes the room's snapshot needs bytes for: every switched-on font and document
    /// that the glue does not hold yet.
    fn wanted(&self) -> Vec<AssetHash> {
        let mut missing = Vec::new();
        for entry in self.library.font_entries() {
            let Some(details) = entry.details.filter(|_| entry.sync) else {
                continue;
            };
            let all =
                std::iter::once(entry.hash).chain(details.documents.into_iter().map(|d| d.hash));
            missing.extend(all.filter(|hash| !self.fonts.blobs.contains_key(hash)));
        }
        missing
    }

    fn complete(&self, hash: &AssetHash, details: &FontDetails) -> bool {
        self.fonts.blobs.contains_key(hash)
            && details
                .documents
                .iter()
                .all(|doc| self.fonts.blobs.contains_key(&doc.hash))
    }

    /// Whether the next snapshot has to wait: a switched-on font's bytes are still arriving as
    /// parts (the relay prunes the part records that complete them), or this host holds bytes
    /// the page has not supplied yet. Neither waits longer than its patience, counted from the
    /// first part or the first ask of that hash: not for bytes that nobody is sending, and not
    /// for a sender that keeps starting over, or one stuck font would stop the relay's pruning.
    fn snapshot_blocked(&self) -> bool {
        let arriving = self.wanted().iter().any(|hash| {
            self.fonts
                .assembling
                .get(&hash.to_bytes())
                .is_some_and(|a| self.now.since(a.first_at) <= TOTAL_WAIT_MS)
        });
        let supplying = self.font_wants().iter().any(|hash| {
            self.fonts
                .want_since
                .get(hash)
                .is_none_or(|at| self.now.since(*at) <= ASSEMBLY_PATIENCE_MS)
        });
        arriving || supplying
    }

    /// Notes since when each hash the page has to supply has been asked for, and forgets the
    /// ones no longer asked.
    fn track_wants(&mut self) {
        let wants = self.font_wants();
        self.fonts.want_since.retain(|hash, _| wants.contains(hash));
        for hash in wants {
            self.fonts.want_since.entry(hash).or_insert(self.now);
        }
    }

    /// Whether a snapshot may go up now.
    pub(super) fn fonts_complete(&mut self) -> bool {
        self.track_wants();
        let blocked = self.snapshot_blocked();
        if blocked {
            self.fonts.snapshot_deferred = true;
        }
        !blocked
    }

    pub(super) fn retry_deferred_snapshot(&mut self) {
        self.track_wants();
        if self.fonts.snapshot_deferred && !self.snapshot_blocked() {
            self.fonts.snapshot_deferred = false;
            if !self.newer_list && !self.newer_library {
                let covers_seq = self.client.cursor();
                self.upload_group_snapshot(covers_seq);
            }
        }
    }

    /// Time passed: a half blob quiet for [`ASSEMBLY_PATIENCE_MS`] is given up, the snapshot a
    /// wait held back may go, and blob records that failed to go out are tried again.
    pub(super) fn font_tick(&mut self) {
        let now = self.now;
        self.fonts
            .assembling
            .retain(|_, a| now.since(a.last_at) <= ASSEMBLY_PATIENCE_MS);
        self.retry_deferred_snapshot();
        self.flush_font_blobs();
    }

    /// Drops the blobs of `hash` and its documents unless another switched-on font names them.
    pub(super) fn forget_blobs(&mut self) {
        let keep = self.referenced();
        self.fonts.blobs.retain(|held, _| keep.contains(held));
    }

    /// The hashes the switched-on fonts name, files and documents, with the size each says.
    pub(super) fn referenced_sizes(&self) -> BTreeMap<AssetHash, u64> {
        let mut named = BTreeMap::new();
        for entry in self.library.font_entries() {
            if let Some(details) = entry.details.filter(|_| entry.sync) {
                named.insert(entry.hash, details.size);
                named.extend(details.documents.into_iter().map(|d| (d.hash, d.size)));
            }
        }
        named
    }

    pub(super) fn referenced(&self) -> BTreeSet<AssetHash> {
        self.referenced_sizes().into_keys().collect()
    }

    /// The blobs of every switched-on font and document, for the room's snapshot.
    pub(super) fn font_snapshot_blobs(&self) -> Vec<([u8; 32], &[u8])> {
        let keep = self.referenced();
        self.fonts
            .blobs
            .iter()
            .filter(|(hash, _)| keep.contains(hash))
            .map(|(hash, bytes)| (hash.to_bytes(), bytes.as_slice()))
            .collect()
    }

    /// A local change to the font entries: push and export.
    fn library_changed(&mut self) {
        self.files_changed = true;
        self.bump();
        self.export_library();
        self.flush_font_blobs();
        self.pump();
    }
}
