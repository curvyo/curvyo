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
