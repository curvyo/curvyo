# ADRs for library sync

Architect, 2026-11-13, for the customer request of 2026-11-13: the maker's three per-user libraries
(installed Curvyo fonts with file bytes and metadata, machine profiles, material records) sync
across the maker's own paired devices, with a per-item sync toggle on every font, machine and
material row. Constants marked Proposal are vetoable at the demo.

- [ADR 0062](../../docs/adr/0062-synced-per-user-libraries.md) (new, accepted 2026-11-13):
  the whole feature — one library stream as the second sealed stream of the device group room,
  machine/material entries as the existing record texts keyed by uuid, font entries keyed by the
  file's SHA-256, blobs sealed like workspace snapshot parts, the per-item toggle in the stream,
  newest-`modified`-wins with a local conflict copy, budgets (16 MiB per font file, 512 MiB total,
  Proposal).
- [ADR 0040](../../docs/adr/0040-own-devices-pairing-and-device-group.md): provides the room,
  keyring, pairing, remove and leave; the library stream is its second sealed stream, rides the
  same `NewEpoch` on removal and is forgotten with `devices/` on leave (decisions 1, 2, 8).
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md): rooms, sealed
  streams, snapshots and pruning (decision 10) — the library stream is one more of these; no frame,
  envelope or protocol change.
- [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md): the sealed snapshot with
  4 MiB parts whose sealing the blobs reuse; workspace sharing itself is untouched by this feature.
- [ADR 0038](../../docs/adr/0038-machine-profile-records.md): the machine record texts and their
  envelope gate are carried unchanged; `machine-choice.json` stays machine-local (decision 6);
  `Generic laser` is code, never synced (decision 4).
- [ADR 0044](../../docs/adr/0044-workspace-only-unit-of-sharing.md): the workspace stays the only
  unit of sharing — libraries never enter a workspace and are never shared with other makers.
- [ADR 0047](../../docs/adr/0047-document-fonts.md): documents embed the used font bytes per file;
  this feature syncs the library instead. Both use the same SHA-256 (`FaceHash` = `AssetHash`), so
  the two agree and never duplicate work.
- [ADR 0055](../../docs/adr/0055-material-records.md): the material record model and envelope;
  `material-order.json` and `test-grid-choice.json` stay machine-local (§5, §6).
- [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md): record format 2 is
  what syncs; its photo files travel as blobs under 0062's budget (Proposal).

## Feature-local decisions

- 2026-11-13: equal `modified` on a machine or material is broken by the bytewise-greater record
  text, so every device picks the same winner without a clock; the loser still gets a conflict copy.
- 2026-11-13: conflict copies are named `<uuid>.conflict.json` in `machines/`/`materials/` (browser
  key `curvyo.machines/<id>.conflict`), ignored by the readers like a folder-sync copy (ADR 0038 §1)
  and listed in one notice; deleting them is the maker's.
- 2026-11-13: a family with several face files is several stream entries (one per file hash); the
  fonts toggle sits on the family row and sets all of its file entries.
- 2026-11-13: the licence and readme files a .zip Add font kept (`0083`) travel with the
  font as small blobs under 0062's budget (the spec's criterion 18; a font without its
  licence loses its embedding permission).
- 2026-11-13: no master switch; the toggle is per item only, and nothing syncs while the app is
  closed (ADR 0040 decision 7 pattern).
- 2026-11-13: the browser's copy of the stream lives with the browser group state of ADR 0040
  decision 9; that store follows whatever 0040's browser spec decides.
- 2026-11-19 (P1): the record merge lives in `curvyo-laser-core` (`library_merge`), not in
  `curvyo-sync-core` as ADR 0062 decision 10 says: ADR 0031's invariant lets `curvyo-sync-core`
  depend on `curvyo-crypto-core` only, and the merge must parse `modified` with the record
  readers of `curvyo-laser-core`. The stream framing stays in `curvyo-sync-core`. No new
  dependency.
- 2026-11-19 (P2): an entry's record text is capped at the readers' limits, 128 KiB for machines
  and 512 KiB for materials, not the 16 KiB / 256 KiB of decision 6 (those numbers were stale: the
  machine reader takes 128 KiB since record format 3, the material reader 512 KiB, ADR 0058 §6).
  The constants are duplicated in `curvyo-document-core` (it cannot depend on laser-core).
- 2026-11-19 (P3): the devices list keeps its bare updates and library records carry the ADR 0037
  inner header, so an old build in a mixed group keeps reading the devices list but **loses library
  sync** (it ignores the framed room snapshot and the library records). A group's devices are
  upgraded together; the release notes say so.
- 2026-11-19: the room snapshot is **state-only** for the library (`ExportMode::StateOnly`), so
  the text of a switched-off or deleted item is not in it or in a fresh member's oplog
  (decision 4, criteria 23 and 27); `library.loro` on a device keeps the full history. The
  devices list keeps its full snapshot. Trade-off: Loro cannot merge a state-only snapshot onto
  a replica that holds history (the import stays pending), so a member behind the snapshot
  starts over from it (`LibraryList::import_snapshot`) and its library changes not yet pushed
  are lost; the record files stay on disk and the maker can switch the item on again. A peer
  whose history ends before the snapshot cannot merge the updates of a peer that started from it
  either. Imported changes are never pushed back: after an import the device's exported version
  moves past them, so it sends its own changes only.
- 2026-11-19: a record that arrives is a conflict only when it competes with an edit of this
  device. A local record that still equals the entry's earlier text, when the incoming change
  was made knowing everything this replica held (its dependencies include the replica's
  version), is superseded without a conflict copy or a notice; an edit of an item the device
  holds is no arrival. An item that never synced (no entry) is published only by its toggle
  or, once the first sync was confirmed, as a new item (criterion 4).

### Slice 2 (fonts), amendments of 2026-11-20

Notes from the architect review of `story/library-fonts`; the numbers are the plan's.

- 2026-11-20 (P21): an entry is a flat root map of registers `<sha-256>.<field>` with `sync`,
  `file_name`, `folder`, `size`, `tags`, `favourite`, `cuttable`, `documents`. `folder`, `size`
  and `documents` are additive to decision 2's list (the `.zip` subfolder of `0083`, the budgets,
  criterion 18). A family's metadata travels on each of its file entries (decision 2), read from
  the file's first family. Entries whose registers have a bad shape, a font over 16 MiB or a
  licence file over 1 MiB are not read (decision 6, receiving side).
- 2026-11-20 (P22): the first font entry writes `library.loro` as **format 2** (decision 1 says
  `format_version = 1`). ADR 0040 decision 11 makes a build that reads only format 1 refuse the
  update and upload no snapshot, so it cannot let the relay prune blob records it never held.
  The price: a build without slice 2 in the group stops syncing machines and materials too once
  a font is on; the release notes say so (as for P3). A list without fonts stays format 1.
- 2026-11-20 (P23): blobs travel as parts (`hash | index | count | total | bytes`, the last part
  padded to 64 KiB so the relay cannot recognise a known font by its size) in ordinary sealed
  `Asset` records **and** live in the room's snapshot as entries of their own stream, because the
  relay prunes the records a snapshot covers. A switched-off, deleted or unreferenced blob is gone
  from the next snapshot (criteria 4 and 27).
- 2026-11-20 (P24): the glue keeps the bytes of switched-on fonts in memory for the snapshot.
  Peak is about three times the library (held bytes, the encoded snapshot, parts, sealed copies):
  recorded in `docs/technical-debt.md`. On the receiving side, held plus arriving bytes are
  counted against the 512 MiB budget, a blob that would pass it or is not the size its entry says
  is refused with a notice, and blobs no entry names make room first. A blob still arriving as
  parts holds the next snapshot back; one quiet for 10 minutes is given up (Proposal), and the
  wait for one hash is capped at 30 minutes from its first part (10 minutes for the page's
  supply, counted from the first ask), however often a sender starts over. A font whose bytes
  nobody sends does not hold it back, so a stuck font cannot stop the relay's pruning. The
  snapshot path has the same budget: a snapshot entry over 16 MiB plus its hash (blob stream) or
  32 MiB (a list) or a blob that cannot fit the budget drops the assembly with a notice, so an
  endless snapshot never grows in memory. Trade-off: a snapshot uploaded without a blob's bytes
  lets the relay prune its parts; the next snapshot of a device that holds them restores it.
  The transfer state (`assembling`, `to_push`, pending acks) is in memory only: after a restart a
  half-received blob is lost and a font switched on offline never pushes its parts, yet its row
  reads as transferred. Only the next snapshot of a device that holds the bytes repairs it
  (recorded in `docs/technical-debt.md`).
- 2026-11-20 (P28): deleting a synced font removes its entry. Another device moves its copy to
  the trash (desktop) or deletes it (browser) **only if the copy was written there through
  sync** (a set of the host ids of the files written through sync, dropped when the file goes),
  never a font the maker installed there himself or a copy of the same bytes in another folder.
  A deletion whose local copy stays is remembered (a persisted set of hashes), so the font is not
  switched on and published again. A room snapshot can replace a whole list (a state-only
  snapshot onto a replica with history); only then is an entry missing after it not read as a
  deletion and nothing is trashed. A snapshot that merges reports a deletion like an update. A
  replace also reverts a font switched on while offline (its entry is gone, the file stays and
  its toggle reads off until it is pressed again).
- 2026-11-20 (correction of decision 3): "a device that already holds a hash never receives it
  again" holds for what the page writes (a hash the host has is not written again), not for the
  wire: the relay still sends every part record to every device, and every snapshot uploads all
  blobs of the on entries again.
