# Plan for the machine profile

Implementer, 2026-11-03. Branch `story/0026-machine-profile`. ADRs 0038, 0039 (written for
Frage 122 A and Frage 123 A, built as A/A), `adrs.md` note 4 (host-drawn `Block::MachineProfile`).

## Affected crates/modules

- **`curvyo-laser-core`** (ADR 0038 §7, ADR 0039 §3): new modules `machine_profile` (record,
  `OriginCorner`, newtypes `FeedRate`, `PowerValue`, `BaudRate`, clamps with notice text, the
  built-in, `new_profile_id`), `machine_profile_file` (record and `machine-choice.json` codec),
  `machine_profiles` (the library list: load with skip counts, add, duplicate, rename, update,
  delete, connection edit; each edit returns the host write), `machine_profile_copy` (the
  `mfg_profile` map: per-field read with defaults, writer, compare with a record). `serde` joins.
  The manifest's `Params { "Machine" }` block becomes `Block::MachineProfile`. Goldens in
  `tests/fixtures/machine_profiles/`.
- **`curvyo-extension-core`**: `Block::MachineProfile { label }`; `HostKey`, `ParamKind::Display`,
  `ParamValue::Text`, `WorkbenchContribution::{machine_name, work_width_mm, work_height_mm}`
  removed (no user left; the work area of a new manufacturing is the profile's); `CHANGELOG.md`.
- **`curvyo-document-core`**: `mfg_profile` register (atomic map), `ManufacturingSnapshot::profile`
  plus the format-35 `legacy_machine_name`, `NewManufacturing::profile`,
  `set_manufacturing_profile(id, map, work_area, label)` (one commit, also drops
  `mfg_machine_name`), open check (map of plain values), integers kept as Loro `I64`,
  `format_version` 36, goldens `machine_profile_v36.curvyo`, `machine_profile_damaged_v36.curvyo`,
  `mfg_profile_not_a_map_v36.curvyo`; export fixtures follow the version.
- **`curvyo-ui-core`**: history names of the new commit labels.
- **`curvyo-render-core`**: `build_origin_marker` (constant screen size, cased).
- **`curvyo-editor-wasm`**: the session holds the `MachineProfiles` and the last pick; machine
  commands (pick, field edits, reset, save as, update, duplicate, rename, delete, connection);
  an outbox of host writes; the `machine` part of the manufacturing view JSON (replaces the
  `host.*` values); origin marker in the Laser frame; wasm bindings; CPU-time test.
- **`curvyo-storage-io`**: `read_dir_texts(dir, suffix, max_files, max_bytes)`, `remove_file`.
- **`curvyo-app`**: `machine_profiles.rs` with five commands that move text.
- **`frontend`**: `platform/host.ts` localStorage commands, `lib/machineProfilesFile.ts` (load once,
  apply to each session, perform the outbox), `MachineBlock.tsx` (+ small helpers), ManifestTab
  draws the new block kind, registry types; vitest for pure helpers.
- **docs**: design-system rows (UX notes "Design-system rows"), technical-debt entry
  (record envelope to `curvyo-library-core` later), spec status line.

## Tasks

- [x] 1. laser-core `machine_profile`: record type, defaults, ranges, clamps and notice texts,
  name cut, unique "copy" naming, UUID v4 from 16 bytes; unit tests (AC 1, 2, 3, 9).
- [x] 2. laser-core `machine_profile_file`: canonical writer, reader with every damage case of
  ADR 0038 §3, choice file; golden fixtures (AC 4, 10, 13).
- [x] 3. laser-core `machine_profiles`: load (skip counts, 1,000 cap, non-uuid names ignored),
  list order, add/save-as, duplicate, rename, update, delete, connection edit, writes returned;
  built-in protected (AC 2, 3, 4, 9, 13).
- [x] 4. laser-core `machine_profile_copy`: read with per-field defaults and damaged-name flag,
  format-35 reading, writer without connection, differs compare (tolerance 0.01 mm) (AC 6, 7, 8,
  12, 15).
- [x] 5. extension-core `Block::MachineProfile`, removals, changelog; laser manifest uses it
  (note 4).
- [x] 6. document-core `mfg_profile`, snapshot/new/set command, validator, format 36, migration
  reading of 35, goldens and tests (AC 5, 6, 8, 12).
- [x] 7. render-core origin marker; session frame draws it from the copy (AC 11).
- [x] 8. editor-wasm session: library state, outbox, commands, view JSON, last pick for Add
  manufacturing, history names; session tests incl. Differs with a fixture record; 0093 tests
  moved off `host.*` (AC 1 to 12).
- [x] 9. editor-wasm CPU-time test: 200 records load within 200 ms, list view within 100 ms (AC 14).
- [x] 10. storage-io helper and app commands (AC 4, 13, 15).
- [x] 11. frontend host.ts, machine profiles file, MachineBlock, ManifestTab, notices (AC 1 to 11).
- [x] 12. Browser check end to end, screenshots; docs; full gate (all).

## Validation

- Unit tests in laser-core per module (codec goldens under `tests/fixtures/machine_profiles/`).
- document-core goldens for criterion 12 (`CURVYO_WRITE_FIXTURES=1` rewrites them).
- Session tests in editor-wasm for pick/edit/reset/save/update/missing/damaged name, one undo step
  each, outbox content, and Differs with a fixture record.
- CPU-time budget test under `curvyo-editor-wasm/tests/cpu_time/` (criterion 14).
- Browser run of the demo build: create, pick, edit, Differs, Update, Reset, delete, reload.
- Full gate of `CLAUDE.md` §7 once at the end.
