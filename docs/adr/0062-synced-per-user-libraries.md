# ADR 0062: Synced per-user libraries: one library stream in the device group room

**Status:** Proposed 2026-11-13, `needs-customer` (persistence/sync: per-user library data leaves the
machine through the relay). Asked by `specs/0116-library-sync` (customer request of 2026-11-13).
Extends [ADR 0040](0040-own-devices-pairing-and-device-group.md) (the device group room gains a
second sealed stream); carries the record texts of [ADR 0038](0038-machine-profile-records.md)
(machines), [ADR 0055](0055-material-records.md)/[ADR 0058](0058-material-records-shared-across-lasers.md)
(materials) and the font library of `0075` unchanged; reuses ADR 0047's SHA-256 font identity. Keeps
[ADR 0044](0044-workspace-only-unit-of-sharing.md) unchanged: the workspace stays the only unit of
sharing, and a library never reaches another maker. Choices marked (Proposal) are the architect's,
vetoable at the demo.

## Context

The maker's three libraries are per-user data on each computer: installed Curvyo fonts (file bytes
in the app-data fonts folder on desktop, IndexedDB `curvyo-fonts` in the browser, metadata in
`font-library.toml` / `curvyo.font-library`), machine profiles (`machines/<uuid>.json`, ADR 0038)
and material records (`materials/<uuid>.json`, ADR 0055/0058). `0096` synced workspaces only and
deferred the libraries; today nothing syncs them except a folder-sync tool the maker may point at
the data directory, whose conflict copies the record readers simply ignore (ADR 0038 §1). The
maker's own devices already form one device group room (ADR 0040) whose keyring admits exactly the
devices that should see this data — and no one else.

### Options: how the libraries travel

**A. A library stream inside the existing device group room (chosen).** A second sealed stream
beside the devices list of the ADR 0040 room: records as texts and blob references; font file bytes
as content-addressed blobs sealed like workspace snapshot parts. No new room, no relay change;
removing a device rotates the room's epochs and covers the library with the same `NewEpoch`
(ADR 0040 decision 8).

**B. One separate ADR 0031 room per library.** Up to three rooms per maker with exactly the group's
membership: three keyrings to rotate on every removal, three room ids to hand over at pairing,
three more rooms for the relay to link (ADR 0040 decision 10) — for no gain, since membership is and
must be identical to the group's. Rejected.

**C. A hidden auto-synced workspace holding the libraries.** Rejected: it works around ADR 0044
instead of following it (the workspace is the unit of sharing with people; a hidden one the maker
cannot see, leave or unshare reuses that rule for something else), litters the workspace folder
with files no workspace command knows, and drags the libraries into snapshot and quota machinery
built for documents.

## Decision

1. **Transport.** The library stream is the second sealed stream of the device group room: a Loro
   map, module `library_list` in `curvyo-document-core` beside `devices_list` (no Loro type in its
   API), `format_version = 1`, local file `library.loro` in `devices/` beside `devices.loro`
   (temp-then-rename, ADR 0040 decision 2). Snapshots and pruning as ADR 0031 decision 10.
2. **Entries.** `machines.<uuid> = {sync, text}` and `materials.<uuid> = {sync, text}` hold the
   exact record texts the writers of ADR 0038/0055/0058 produce; the envelope gate
   (`curvyo-laser-core/src/record_envelope.rs`) and every `format_version` rule apply unchanged on
   every device. `fonts.<sha-256 of the file bytes> = {sync, file_name, tags, favourite, cuttable}` —
   the hash equals `FaceHash`/`AssetHash` of ADR 0047, so a document's embedded font and the
   library's copy are one identity; a family's metadata travels on each of its file entries.
3. **Blobs.** Font file bytes are blobs keyed by their SHA-256, sealed as parts of at most
   `PART_BYTES` (4 MiB, `curvyo-sync-core/src/workspace_stream.rs`) exactly like workspace snapshot
   parts and uploaded as ordinary sealed room records — no relay protocol change. A device that
   already holds a hash never receives it again; an unreferenced blob is pruned with the room's
   snapshots. Material photos of ADR 0058 travel the same path (Proposal).
4. **The per-item toggle lives in the stream** (`sync` on every entry), so it is one source of truth
   and every device shows the same state. Off keeps the entry as a marker and drops text and blob
   from the stream; every device keeps its local copy — turning sync off never deletes anything.
   Deleting an item removes the entry, and devices that received it through sync delete their local
   copy. New items sync on by default; a device's first sync writes all its local items as on
   (Proposal).
5. **Merge.** Fonts: the bytes are immutable (hash = bytes), so no conflict is possible; metadata
   fields are registers, last writer wins, silent — a tag or a star is too small to guard. Machines
   and materials: **newest `modified` wins; the loser stays locally as a conflict copy with a
   notice** (naming in the feature-local decisions; the readers ignore such copies as ADR 0038 §1
   ignores a folder-sync tool's). Silent LWW was rejected: a test history would vanish without a
   trace, while `modified` exists in every record precisely for this merge (ADR 0038 §2, "kept for
   the merge of ADR 0004 §8"); the copy is the same honesty as today's folder sync and the
   "Record conflicts" debt ADR 0038 names. The union-by-id merge of ADR 0004 §8 stays future work.
   The winner is written back to the stream by every device, so all converge.
6. **Limits.** A font file over 16 MiB is refused with a notice (Proposal; regular and CJK fonts
   fit, hostile archives do not). The library's total blob budget is 512 MiB (Proposal, the project
   asset budget of ADR 0047 §3); an item that would pass it is refused with a notice and stays
   local. Record texts keep their existing limits (16 KiB machines, 256 KiB materials).
7. **Machine-local stays machine-local.** `machine-choice.json` (ADR 0038 decision 6),
   `material-order.json` and `test-grid-choice.json` (ADR 0055 §5, §6) never sync; a new device
   orders by name until the maker reorders there. `Generic laser` (ADR 0038 decision 4) and the
   Included fonts (ADR 0020) are code, never synced.
8. **Browser.** Reception writes through the host layer as today: IndexedDB `curvyo-fonts` and
   `curvyo.font-library` (`frontend/src/lib/browserFonts.ts`), `curvyo.machines/<id>`,
   `curvyo.materials/<id>`; desktop writes the app-data files through `curvyo-storage-io`. The
   session in `curvyo-editor-wasm` applies stream entries and turns them into host writes; record
   text is decoded only in Rust (ADR 0055 §7).
9. **Remove, leave, rotation.** Removing a device rotates the group room (ADR 0040 decision 8);
   the library stream rides the same `NewEpoch`, and any remaining device re-seals blobs under the
   new epoch from its local copy when it next pushes, so the relay holds nothing readable for the
   removed device. A removed or leaving device keeps its local library files and forgets
   `library.loro` with `devices/`.
10. **Crates.** `curvyo-sync-core`: the stream, the blob parts, the merge; `curvyo-document-core`:
    `library_list`; `curvyo-storage-io`: `library.loro` beside the devices files;
    `curvyo-editor-wasm`: session glue; `frontend`: browser host writes; `curvyo-app`: Tauri
    commands that move text and decide nothing. No new crate, no new dependency, no relay change,
    no project `format_version` change.

## Consequences

- Fonts, machines and materials appear on every paired device — only among the maker's own devices
  (the group room's members), never in a workspace, never with another maker (ADR 0044).
- Goldens in `curvyo-sync-core/tests/fixtures/library/`: stream records v1 (a machine, a material,
  a font with blob, a sync-off marker, a deletion) and a sealed blob part.
- Merge tests: newest-wins, conflict copy with notice, equal `modified` (feature-local rule), font
  hash stability, blob dedup.
- CPU budget under `curvyo-editor-wasm/tests/cpu_time/`: applying 200 machines, 200 materials and
  100 font entries with two blobs within the load budgets of ADR 0038/0055 (thread CPU time, never
  wall time).
- Every touched crate stays pure (timestamps still passed in by callers) and builds for
  `wasm32-unknown-unknown` in CI as before.
- The "Record conflicts" debt narrows: the device-sync path now resolves instead of ignoring;
  folder-sync conflict copies keep today's behaviour.
- The relay learns nothing new: one more sealed stream's size and cadence in the room it already
  links (ADR 0040 decision 10); names, tags and record contents stay sealed.

## Needs the customer

The PO asks with their own question numbers: the first-sync default of decision 4 (all on), the
budgets of decision 6 (16 MiB per font file, 512 MiB total), and conflict copy with notice versus
silent last-writer-wins of decision 5. Recommendation: as written. Default if unanswered: as
written (the Proposal items are vetoable at the demo).
