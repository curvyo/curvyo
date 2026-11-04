# ADR 0038: Machine profile records in the per-user data directory

**Status:** Accepted 2026-11-04 (customer accepted PR #258, built with Frage 122 A and Frage 123 A). Realises the
machine part of [ADR 0004](0004-persistence-and-cross-machine-sync.md) §6 for the laser family;
places the record model in `curvyo-laser-core` instead of the `curvyo-library-core` slot of
ADR 0004 §8 and [ADR 0011](0011-workspace-and-crate-layout.md) §2 until a second user of it
exists. Spec: `specs/0026-machine-profile` criteria 2, 3, 4, 9, 10, 13, 14, 15.

## Context

A machine profile describes one physical laser (bed, origin corner, limits, how to reach it) and
is reused by every laser manufacturing in every project (R-MFG-001). It is the maker's data, not
the project's: it lives on this computer per user, follows the data directory when the maker
syncs that folder (ADR 0004 §7), and never leaves the machine on its own (R-SYS-002, R-SYS-007).
The project keeps a copy of the used values ([ADR 0039](0039-machine-profile-copy-in-the-project-file.md)).

ADR 0004 §6 (accepted) already shaped this data: "the machine/material database ... one small
file per record, `machines/<uuid>.json`, ... a UUID, a `format_version` and a `modified`
timestamp". The two per-user files written since (`document-formats.toml`, `0045`;
`font-library.toml`, `0075`) chose one TOML file each and named ADR 0004 §6 as rejected for
them. The question is which pattern the machine profiles follow.

### Options considered

- **A. One JSON file per profile, `machines/<uuid>.json`, as ADR 0004 §6 says (chosen).**
  Two computers that edit different profiles offline sync without a conflict; one damaged
  record is skipped while the others load (criterion 13) with no extra code; a record with a
  newer `format_version` is left alone without making the whole library read-only. The test
  results of `0028` are already designed as logs per machine/material pair keyed by this UUID,
  so the record shape and its id are the ones the material database will join on.
- **B. One `machine-profiles.toml` next to the formats and font library files.** One read, one
  write, same code shape as `0045`/`0075`, "last picked" fits in. Lost because it contradicts an
  accepted ADR for exactly this data, turns any two-computer edit into a whole-file conflict
  copy, and would need a format migration the day the material database (per record by ADR 0004
  §6) arrives. Fonts and formats are settings; machines are records other records point at.
- **Crate: C. A new `curvyo-library-core` now (ADR 0011's slot).** Lost: it would hold one
  laser-only record, no merge (ADR 0004 §8 is not built yet), and no second family (`CLAUDE.md`
  §5, §6). **D. `curvyo-ui-core`, where the font library lives.** Lost: the fields are laser
  fields, and ADR 0034 makes laser data the laser extension's (R-EXT-003).

## Decision

1. **Location.** Desktop: one file per user profile in `machines/` under the data directory
   (Tauri `app_data_dir()` until the configurable data directory exists), named `<id>.json`.
   Browser: one `localStorage` entry per profile, key `curvyo.machines/<id>`, holding the same
   text. A file or key whose name is not `<uuid>.json` / `curvyo.machines/<uuid>` is ignored and
   never touched (a sync tool's conflict copy stays where it is; detection is the open
   "Record conflicts" entry in `docs/technical-debt.md`).
2. **Record format** (`format_version = 1`, canonical writer: this key order, two-space indent,
   trailing newline, so read, write, read is the identity):
   ```json
   {
     "format_version": 1,
     "id": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
     "kind": "laser",
     "modified": 1762171200000,
     "name": "K40 in the garage",
     "work_area_mm": { "width": 400.0, "height": 300.0 },
     "origin": "bottom_left",
     "max_feed_mm_per_min": 6000,
     "max_power": 1000,
     "laser_mode": true,
     "connection": { "connector_address": "", "controller_port": "", "baud_rate": 115200 }
   }
   ```
   `id` a lowercase UUID v4, equal to the file name; `kind` is `"laser"` (other families add
   their kind with a new `format_version`); `modified` Unix milliseconds, passed in by the caller
   (core has no clock), kept for the merge of ADR 0004 §8; `origin` one of `top_left`,
   `top_right`, `bottom_left`, `bottom_right`; feed and power are integers, baud rate an integer.
   Ranges as spec criteria 1 and 9. Unknown keys are damage (`deny_unknown_fields`). No secret
   of any kind is a field (the pairing token stays in the OS credential store on the desktop and
   in IndexedDB in the browser, ADR 0035 §4).
3. **Reading.** Per record: text over 16 KiB, a JSON or type error, a value out of its range, a
   kind other than `laser` or an id that differs from the file name is damage: the record is
   skipped, the load returns one notice with the count, and the file is never rewritten, renamed
   or deleted by the reader. A `format_version` above 1 is skipped with its own notice and never
   written (ADR 0004 §9: never partly read). At most 1,000 records are read, in file-name order.
   Every other record loads.
4. **The built-in.** `Generic laser`, id `generic-laser` (not a UUID, so no file can collide),
   exists in code only, with the defaults of criterion 1; it is never written to a file.
5. **Ids.** A new user profile's id is a UUID v4 made in core from 16 random bytes the caller
   passes in (the page's `crypto.getRandomValues`, as `editorSession.ts` does). Random ids are
   required: a file from another computer carries its id (ADR 0039), and a counter or a name
   slug would match an unrelated profile of this library and show a false "Differs".
6. **Last picked** (criterion 10) is this computer's preference, not a record: `machine-choice.json`
   (`{ "format_version": 1, "last_picked": "<id>" }`) in the config directory (`app_config_dir()`,
   machine-local as `0045` decision 1 defines it), browser key `curvyo.machine-choice`. An id the
   library no longer has falls back to Generic laser. Only a local pick writes it; a pick that
   arrives from a peer (ADR 0031) does not.
7. **Crates and modules.**
   - `curvyo-laser-core` (pure, wasm32, `#![forbid(unsafe_code)]`), three modules:
     `machine_profile` ("the laser machine profile record with its limits and the built-in"):
     `MachineProfile`, `OriginCorner`, newtypes `FeedRate` (mm/min, `u32`), `PowerValue` (`u16`),
     `BaudRate` (`u32`), the work area as `DocumentSize`, `clamp_*` per field with the notice
     text, `GENERIC_LASER`, `new_profile_id(random: [u8; 16])`.
     `machine_profile_file` ("reads and writes one machine profile record"): `read_record(name,
     text) -> Result<MachineProfile, RecordError { Damaged, Newer }>`, `write_record(&profile)
     -> String`, and the same for `machine-choice.json`.
     `machine_profiles` ("the maker's machine profiles as one list"): load from `(name, text)`
     pairs with the notices, add, duplicate, rename, update, delete, find by id; each edit
     returns the write the host must do (`Write { id, text }` or `Remove { id }`).
     `serde` joins the existing `serde_json` (workspace dependency, already in `Cargo.lock`).
   - `curvyo-editor-wasm`: the session holds the `MachineProfiles` (library edits are outside
     document history, criterion 4) and turns its writes into host calls; strings and scalars
     across the boundary (ADR 0001 §5).
   - `curvyo-storage-io`: one generic helper, `read_dir_texts(dir, max)`; the existing
     `write_atomic`, `read_optional` and a `remove_file` do the rest.
   - `curvyo-app`: `machine_profiles.rs`, four Tauri commands that move text and decide nothing:
     `list_machine_profiles`, `write_machine_profile`, `remove_machine_profile`, and read/write of
     `machine-choice.json`. Browser: the same four in `frontend/src/platform/host.ts` over
     `localStorage`.

## Consequences

- One more file shape we write, with golden fixtures in
  `curvyo-laser-core/tests/fixtures/machine_profiles/` (a full record, the built-in duplicated,
  each damage case of decision 3, a `format_version` 2 record, a non-UUID file name).
- Two per-user persistence patterns now coexist on purpose: settings as one TOML file (formats,
  fonts, extensions), records as one JSON file each (machines now, materials later).
- The record model sits in the laser extension. When `0028` brings material records or a second
  machine family arrives, a `curvyo-library-core` ADR extracts the shared envelope (`id`, `kind`,
  `format_version`, `modified`) and the merge; the file format does not change. Tracked in
  `docs/technical-debt.md`.
- The connection fields sync with the data directory if the maker syncs it; that is their own
  folder. They never enter a project file (ADR 0039 §2).
- Budgets of criterion 14 (200 records: list in 100 ms, load in 200 ms of thread CPU time) are
  confirmed; a test under `curvyo-editor-wasm/tests/cpu_time/` holds them.

### If the customer answers B

- **Frage 122 B** (library only, the file stores name and bed): this ADR is unchanged. ADR 0039
  shrinks to `{ id, name }` (see there).
- **Frage 122 C** (no library): this ADR is withdrawn; nothing of decisions 1 to 7 is built.
- **Frage 123 B** (connection travels with the file): this ADR is unchanged; the record keeps the
  connection, and ADR 0039 §2 adds it to the copy.
