# Plan for library sync, slice 1 (machines and materials)

Implementer, 2026-11-13, on `story/library-sync`. Built against `specification.md` (criteria 1 to
15 and 21 to 27 are slice 1; 16 to 20 fonts are slice 2 and stay out), `adrs.md`, ADR 0062
(accepted) and `ux-notes.md` (the toggle UI is a later dispatch; this plan's core tasks give it
everything it needs). This dispatch builds the core only (tasks 1 to 5); the session glue, app
commands, frontend rows and the journey (tasks 6 to 9) are the next dispatches on this branch.
One PR per slice at the end. **(Proposal)** marks my own choices, vetoable at the demo.

## Affected crates/modules

- **`curvyo-document-core`**: `library_list` (new, beside `devices_list`): a Loro map of
  `machines.<uuid> = {sync, text}` and `materials.<uuid> = {sync, text}` (ADR 0062 decision 2),
  `format_version = 1`, no Loro type in its API, goldens.
- **`curvyo-laser-core`**: `library_merge` (new): the record-text merge of machines and materials
  (ADR 0062 decision 5) beside the record readers it parses `modified` with, and the conflict
  copy id `<uuid>.conflict` (feature-local rule). See Proposal P1.
- **`curvyo-sync-core`**: `library_stream` (new): the framing of the group room's second sealed
  stream — its record marker beside the bare devices list updates, and the room's two-stream
  snapshot — plus goldens in `tests/fixtures/library/`.
- **`curvyo-storage-io`**: `devices`: `library.loro` beside the devices files, read and written
  with the same discipline (ADR 0062 decision 1, ADR 0040 decision 2).
- **`curvyo-editor-wasm`** (later dispatch): session glue — routing library records in the group
  room, applying entries with the merge, host writes, arrivals, the framed room snapshot,
  first-sync-all-on, offline conflict convergence, delete and switch-off apply.
- **`curvyo-app`** (later dispatch): Tauri commands that move `library.loro` bytes and decide
  nothing (ADR 0062 decision 10).
- **`curvyo-ui-core`, `frontend`** (later dispatch): toggle texts, arrival and conflict notices,
  the three row toggles per the UX notes.
- Docs: `docs/technical-debt.md` (the merge's crate home, P1; the duplicated guard helpers, P7),
  `docs/design-system.md` rows (with the UI dispatch).

## Proposals taken (this dispatch)

- **P1 (crate of the merge).** ADR 0062 decision 10 puts "the merge" in `curvyo-sync-core`, but
  ADR 0031 decision 1's invariant (comment in its `Cargo.toml`) lets `curvyo-sync-core` depend
  only on `curvyo-crypto-core`, and the merge must parse `modified` with the existing record
  readers of `curvyo-laser-core` (dispatch rule: never a second field). The two ADRs cannot both
  hold; the merge sits beside those readers in `curvyo-laser-core` (`library_merge`), the stream
  framing stays in `curvyo-sync-core`. No new dependency either way.
- **P2 (entry size caps).** ADR 0062 decision 6 says "record texts keep their existing limits
  (16 KiB machines, 256 KiB materials)" — those numbers are stale: the machine reader takes
  128 KiB since record format 3 (a maximal fixture set is over 64 KiB) and the material reader
  512 KiB (ADR 0058 §6). A 16 KiB stream cap would refuse valid profiles, breaking criterion 5.
  `library_list` caps entry texts at the readers' current limits (128 KiB machines, 512 KiB
  materials), duplicated as constants (laser-core cannot be a dependency of document-core); the
  envelope gate still rules unchanged on apply.
- **P3 (wire shape).** The devices list keeps its bare updates (old builds in a group keep
  reading them); library records carry the ADR 0037 inner header with the fixed stream id
  `LIBRARY_STREAM_ID` (`b"curvyo.library-1"`), so the session routes by decoding the header. The
  room's one snapshot becomes a framed snapshot stream of both lists (devices first, stream id
  `b"curvyo.devices-1"`), assembled by the existing `SnapshotAssembler`. An old build in the same
  group silently ignores the framed snapshot and keeps converging through update records until
  the relay prunes them — the release notes must say to upgrade a group's devices together.
- **P4 (off before first sync).** Switching an item off that never synced writes nothing to the
  stream (criterion 4: nothing about it reaches the server); an item that was on keeps its marker
  entry without text (ADR 0062 decision 4).
- **P5 (storage shape).** `library.loro` gets its own `read_library_list`/`write_library_list`
  beside the group files instead of a field of `DeviceGroupFiles` (which `curvyo-app` constructs;
  this dispatch does not touch that crate). The later glue writes it before
  `write_device_group`, so `group.json` stays the commit point.
- **P6 (merge against a gated local).** A damaged local text loses to a valid incoming one and
  its bytes are kept as the conflict copy; a local text from a newer build is never rewritten
  (ADR 0004 §9) and the incoming text is kept as the conflict copy. A material of format 1 loses
  like a damaged one (its conversion is still unbuilt).
- **P7 (duplicated guards).** `library_list` repeats the private format-guard helpers of
  `devices_list` (bound doc, newer-format scan, hex) instead of refactoring that module open
  mid-story; noted for the refactorer.
- **P8 (arrival notice wording).** The arrival notice cannot name the device the record came
  from: a room record carries no sender identity the glue could map to a device label (the
  Loro peer of the inner header is not a member id). The notice says "… arrived from your
  devices."; the UX dispatch (task 7) owns the final sentence.
- **P9 (a damaged `library.loro`).** A damaged or newer-format `library.loro` fails the group's
  `resume` like a damaged `devices.loro` does (the group shows as unreadable); a missing file
  (pre-0116 group) starts a fresh list, which re-converges through the room's records.
- **P10 (where the conflict copy lands).** ADR 0062 decision 5 keeps the loser "locally as a
  conflict copy": the losing device always writes it; when the register order already favours
  the winner, the winner's device never sees the loser's text and writes no copy. Criterion 9's
  "both devices end with the same two records" holds whenever the loser's entry update reaches
  the other device (the normal offline case, where the write-back carries it); the demo decides
  whether that reading stands.
- **P11 (notices before the UX texts).** The arrival, conflict, switch-off, skip and delete
  notices are module constants in `session/library_sync.rs` (as the machine block's notices are
  today); task 7 moves them into `curvyo-ui-core` with the final wording.
- **P12 (task 7: the machine block's pair line).** The UX notes place "Pair a device to sync
  machine profiles." above the profile list; it renders inside `MachineProfileList`, above the
  listbox, so it shows exactly while the list is open (a closed Machine block shows no list and
  no line), and the SSR tests reach it without opening the block.
- **P13 (task 7: the first-sync confirm).** Its own group inside the `0096` workspace pick block
  (under the list, where that flow's confirm sentence sits): the spec's sentence, "Sync
  libraries" (Apply) and "Not now" (outline). It asks on every completed pairing (the opener's
  block appears then), not only the group's first: `library_first_sync` writes every local item
  on, which is idempotent, and a later pairing's new items are on by Frage 214 anyway.
- **P14 (task 7: notice wordings).** The switch-off notice keeps the closing period the other
  notices carry ("Sync off, this copy stays here." — criterion 10 quotes it without); the
  arrival counts "machine profiles" as the UX notes' batch sentence does ("1 machine profile
  and 1 material arrived from your devices."), while a batch of exactly one item of one kind
  names it ("Machine profile K40 arrived from your devices.", the name cut to 24 — UX notes
  section 5, UX review 2026-11-16); the conflict notice names the copy ("Both copies are
  kept: <name> (from your devices).", criterion 9 with P8's "your devices").
- **P15 (task 7: the toggle's transfer state).** The library stream is one Loro list, so
  "this item transfers" is the stream's state: `library_view_json` carries `online`, `acked`
  (no push waiting, the exported version current) and the formatted `state` line; every
  pressed toggle swaps its glyph with it, off and greyed toggles always keep the cloud.
- **P16 (the failures model).** UX notes section 5 promised an anchored refusal line under
  the row for a press the group could not carry out; the built model (UX review 2026-11-16,
  lead-approved) flips the local switch first and retries through the group — P15's glyph
  swap carries the state and the canvas notices tell skips and refusals — so no anchored
  line exists and section 5 was amended to say so (built 2026-11-13, lead-approved).

## Tasks

### This dispatch (core, test-first)

1. [x] `curvyo-document-core`: `library_list` — root map `library` with `format_version`, the
   maps `machines`/`materials` of `{sync, text}` registers keyed by record id; set (on, with
   text, validated id and size), switch off (marker without text, P4), remove (deletion);
   entries/entry reads that skip bad shapes; snapshot/update bytes with the newer-format
   refusal of ADR 0040 decision 11; goldens `library_list_v1.loro` and damaged variants
   (criteria 4, 7, 10, 11, 12, 23, 24, 26, 27).
2. [x] `curvyo-laser-core`: `library_merge` — `merge_machine_record`/`merge_material_record`
   parse both texts with the existing readers, newest `modified` wins, equal `modified` by the
   bytewise-greater text (the same winner on every device), the loser becomes the conflict copy
   `<uuid>.conflict` (readers ignore it), the envelope gate skips damaged/newer/older incoming
   texts with their reason, a local newer text is never rewritten (P6); the winner write-back to
   the stream is the glue's (criteria 7, 8, 9, 15, 21 to 25).
3. [x] `curvyo-sync-core`: `library_stream` — `LIBRARY_STREAM_ID`, `DEVICES_STREAM_ID`, the
   library record codec over the ADR 0037 inner header, the group room's two-stream snapshot
   entries; goldens in `tests/fixtures/library/` (`machine_record_v1.bin`, `material_record_v1.bin`,
   `machine_off_marker_v1.bin`, `machine_deletion_v1.bin`) and the snapshot round-trip through
   `split_snapshot`/`SnapshotAssembler` (criteria 5, 6, 13, 21, 23; ADR 0062 consequences).
4. [x] `curvyo-storage-io`: `library.loro` beside the devices files — `read_library_list`
   (`None` while absent: a pre-0116 group), `write_library_list` (temp-then-rename),
   `remove_device_group` already forgets it with `devices/`; tests (criteria 2, 21, 22).
   (Completed in the session-glue dispatch: an interrupted run had pre-checked this task.)
5. [x] Gate of the four crates (`fmt`, `clippy`, `nextest` per crate, wasm32 check of the core
   crates), commits after each green task.

### Later dispatches on this branch (slice 1, unchecked)

6. [x] `curvyo-editor-wasm` session glue: route library records in the group room (bare =
      devices list, inner header = library), export/push library updates, apply entries against the
   local records through `library_merge` with conflict-copy host writes and notices, first sync
   writes all local items on, off keeps local copies with the notice, deletion removes local
   copies, arrivals, the framed room snapshot upload (parts when large), `library.loro` in the
   host files (criteria 3 to 15, 21 to 27). Built as `device_group_library.rs` beside the group
   glue and `session/library_sync.rs`; the wasm commands (`library_first_sync`, `library_switch`,
   `library_sync_pump`, `library_view_json`, `library_delete`) and the app's `library.loro` byte
   commands (they ride `read_device_group`/`write_device_group`, P5's write order kept) are in;
   the page's slow poll runs the pump. Left for task 7: the first-sync confirm's call, the row
   toggles and greyed states that read `library_view_json`.
7. [x] `curvyo-ui-core` texts (arrival, conflict, switch-off notices, P11); `frontend` row
     toggles, `S` key, greyed states per the UX notes, the confirm of criterion 3 calling
     `library_first_sync` (criteria 1 to 4, 13). Built as `curvyo-ui-core::library_text`
     (every notice and the tooltip's state lines), the `library_view_json` state fields
     (`online`, `acked`, `state`) in the glue, `LibrarySyncToggle` in the machine and material
     rows (`S`, greyed with the reason line, glyph states from the view), the first-sync
     confirm in the pairing block (its own group, "Sync libraries" / "Not now"), the pump
     refreshing the rows and flushing the arrival's host writes, and the docs rows
     (`docs/design-system.md` "Library sync toggle", "Materials tab rows, sync placement", the
     Machine block note; `docs/technical-debt.md` P1 and P7). See P12 to P15.

8. [ ] Tester acceptance run from the spec; the three-device journey (A, B, browser R) with the
   relay data scan; UX and architect reviews; docs (design-system rows, technical debt).
9. [ ] Slice-2 placeholders (fonts, criteria 16 to 20): `fonts.<sha-256> = {sync, file_name,
   tags, favourite, cuttable}` entries, blobs sealed as 4 MiB parts keyed by the face hash, the
   16 MiB per-file and 512 MiB total budgets, IndexedDB reception, licence files as blobs, the
   fonts goldens (a font with blob, a sealed blob part) and the CPU budget test.

## Validation

- Unit tests per crate (entry model, merge, framing, file discipline); golden files for every
  new format (`library_list_v1.loro` + damaged variants, the four library stream records, the
  two-stream snapshot), regenerated by env var as the existing goldens are.
- The merge's convergence property is tested directly: for any two texts the same winner comes
  out whichever side is incoming.
- No new dependency, no new crate; `cargo check --target wasm32-unknown-unknown` stays clean for
  the touched core crates.
- The end-to-end criteria (5 to 15, 21 to 27) are the later journey's and tester's, on top of
  this core.
