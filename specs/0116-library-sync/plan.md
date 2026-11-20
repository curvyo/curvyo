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

- **P17 (never-synced items, review finding 3).** An item without an entry in the group's list
  (the first sync declined, or made before the pairing) reads as the plain cloud and its toggle
  turns it on (the first sync of that item). A local edit publishes such an item only once the
  first sync was confirmed (the glue's `library_new_items`, set by `library_first_sync` and
  restored by the page from the answer it keeps per group room); a joiner's own earlier items
  need their toggles pressed. The first-sync confirm shows only in a group without entries
  until answered; its decline is "Keep local" (the pick block has the workspaces' one "Not now").
- **P18 (arrivals, review finding 6).** Only an item new to the device is an arrival with a
  notice. An edit of a held item is applied silently and, when the local record still equals the
  entry's earlier text and the change saw everything this replica held, without a conflict copy
  (before, every plain edit of another device left a "Both copies are kept" copy). The page
  flushes the queued host writes and re-reads its rows on `take_library_applied`, never
  depending on a notice.
- **P19 (glyph, review finding 7).** The toggle's `RefreshCw`/`CloudOff` is the group's library
  stream state; a refused item has no glyph of its own (the view is not per item). Noted in
  `ux-notes.md` section 1 and the design-system row (F5).
- **P20 (state-only room snapshot, review finding 1).** The library part of the room snapshot is
  state-only, so a switched-off or deleted item's text leaves the server and a fresh member's
  oplog; a member with history starts over from it. See `adrs.md` (feature-local decisions of
  2026-11-19) for the trade-off.

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
     toggles, `S` key on machine and material rows (none in the fonts tree), greyed states per the UX notes, the confirm of criterion 3 calling
     `library_first_sync` (criteria 1 to 4, 13). Built as `curvyo-ui-core::library_text`
     (every notice and the tooltip's state lines), the `library_view_json` state fields
     (`online`, `acked`, `state`) in the glue, `LibrarySyncToggle` in the machine and material
     rows (greyed with the reason line, glyph states from the view), the first-sync
     confirm in the pairing block (its own group, "Sync libraries" / "Not now"), the pump
     refreshing the rows and flushing the arrival's host writes, and the docs rows
     (`docs/design-system.md` "Library sync toggle", "Materials tab rows, sync placement", the
     Machine block note; `docs/technical-debt.md` P1 and P7). See P12 to P15.

8. [~] Tester acceptance run from the spec (acceptance_0116_tester in three crates, green; a switch-off or delete now uploads the room snapshot after its ack, criteria 23 and 27); the three-device journey (A, B, browser R) with the
   relay data scan; UX and architect reviews; docs (design-system rows, technical debt).
   Done: the reviews' findings of 2026-11-19 (P17 to P20, conflict copies on the desktop, Ctrl+S
   in the materials list, the dead `library_delete` export, the confirm sentence, the duplicate
   design-system row). Left: the three-device journey with the relay data scan, the demo.
9. [ ] Slice 2, fonts (criteria 16 to 20): see "Slice 2" below (tasks F1 to F8).

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

# Slice 2: fonts (criteria 16 to 20)

Implementer, 2026-11-20, on `story/library-fonts` (from origin/main after slice 1, PR #369).
Built on the slice-1 library stream; ADR 0062 decisions 2, 3, 4, 6, 8, `ux-notes.md` (font family
row, notices). **(Proposal)** marks my own choices, vetoable at the demo; numbers continue after P20.

## Affected crates/modules

- **`curvyo-document-core`**: `library_fonts` (new, `impl LibraryList`): the `fonts` root map.
- **`curvyo-sync-core`**: `library_blob` (new): blob parts and the blob stream entry of the room's
  snapshot; goldens `library_record_font_entry.bin`, `font_blob_part_v1.bin` in `tests/fixtures/library/`
  (a reader golden of a version 2 list: `curvyo-document-core/tests/fixtures/library_list_v2_fonts.loro`).
- **`curvyo-ui-core`**: `library_text` words (fonts notice, refusal, confirm); `font_library_sync`
  (new, `impl FontLibrary`): the metadata of a family as it travels, and applying it.
- **`curvyo-editor-wasm`**: `device_group_fonts.rs` (new, beside `device_group_library.rs`): font
  entries, the blob store, budgets, arrivals; `wasm_library_fonts.rs` (new): its bindings;
  `font_library_glue.rs` gets the two sync calls; `tests/cpu_time/` + `library_fonts_cpu_budget.rs`.
- **`curvyo-storage-io`**, **`curvyo-app`**: `write_synced_font` and the command `add_synced_fonts`
  (desktop reception into the fonts folder, bytes in, same naming rules as Add font).
- **`frontend`**: `lib/fontSync.ts`, `hooks/useFontSync.ts`, `browserFonts.ts` reception,
  `FontTreeRows` toggle (no key: FONT-SKEY C), the pair line.
- Docs: `docs/design-system.md` rows, `docs/technical-debt.md` (blob store in memory).

## Proposals taken (slice 2)

- **P21 (entry shape).** `fonts` is a flat root map of primitive registers keyed
  `<sha-256 hex>.<field>` (ADR 0062 decision 2 names the entry, not the Loro layout): `sync`,
  `file_name`, `folder`, `size`, `tags` (own tags of the file's first family), `favourite`,
  `cuttable` (`auto`/`yes`/`no`, the maker's override of the Cuttable tag; the other built-in
  overrides (Script, Sans, Serif, Initials) travel inside `tags` as `add:<id>`/`del:<id>`, own tags
  as `own:<name>`), `documents`
  (`<sha-256> <size> <name>` strings). One register per field makes "last writer wins per field"
  literal, needs no nested container, and the bytes stay deterministic for goldens. A tag set is
  one register (two offline tag edits: the newer set wins, with the notice naming "tags").
  `size`, `folder` and `documents` are additive to ADR 0062's list (needed by the budgets, the
  zip subfolder of `0083` and criterion 18).
- **P22 (format_version becomes 2 with the first font entry).** ADR 0040 decision 11 refuses a
  snapshot or update that writes a newer `format_version` as a whole. A slice-1 build in the group
  would otherwise keep uploading group snapshots without the font blob entries; the relay then
  prunes the blob records they cover and a later device finds entries without bytes. Writing
  version 2 when the first font entry is set makes a slice-1 build raise its `newer_library` flag,
  refuse the list and upload no snapshot (the rule doing its job), at the price that old builds
  stop syncing the libraries once a font is on (P3 already says to upgrade a group together). A
  library without fonts stays version 1, so it stays readable by slice-1 builds. The
  slice-1 tests that used 2 as "newer" use 3 now (`library_list_v3.loro`).
- **P23 (blob wire shape).** A blob is split into parts of at most `PART_BYTES` (4 MiB) plaintext,
  each `hash[32] | index u32 | count u32 | total u64 | bytes`, pushed as ordinary sealed
  `Asset` records (no relay change). The room's snapshot repeats every blob of an on entry as an
  entry of the framed snapshot stream (`BLOB_STREAM_ID`, bytes `hash[32] | file bytes`), because
  the relay prunes records up to a snapshot's `covers_seq` (a later device could not get a pruned
  blob); a switched-off, deleted or unreferenced blob is therefore gone from the next snapshot
  (criteria 4 and 27). Receivers check `SHA-256(bytes) == hash` before anything is written; a
  mismatch is dropped with the damaged notice.
- **P24 (the glue holds the blobs).** The page supplies a font's bytes when its toggle goes on, a
  font is installed, or the glue asks (`library_font_wants`) for the bytes of an on entry it does
  not hold (the snapshot needs them); the glue keeps them in memory while the group is open. Cost:
  up to the 512 MiB budget resident. Recorded as technical debt (a streaming snapshot is the
  later fix); typical libraries are tens of MiB.
- **P25 (budgets).** 16 MiB per file and 512 MiB for the on entries of all fonts (a hash counted
  once, licence files counted with their bytes) are checked in `library_font_set` before anything
  is written, so a refused font writes no entry and no blob (criterion 19). The toggle stays off;
  a canvas notice gives the reason and numbers (`curvyo-ui-core`).
- **P26 (metadata of a collection).** An entry belongs to one file; a `.ttc` has several families.
  Its metadata is read from the file's first family and applied to every family of the file.
- **P27 (reception).** Arrived blobs are handed to the host as `{hash, file_name, folder,
  metadata, documents}` plus the bytes; the host writes them through its Add-font rules (desktop
  `add_synced_fonts` into the fonts folder, browser into `curvyo-fonts`), re-hashes what it
  stored (the same SHA-256, criterion 16) and applies the metadata by family. A hash the host
  already holds is never written again (dedup, ADR 0062 decision 3).
- **P28 (deleting a synced font).** Criterion 11 and ADR 0062 decision 4 decide it, so it is no
  Proposal: deleting an on font removes its entry and the next snapshot drops its bytes; a device
  that holds that hash moves the file to the OS trash on the desktop (existing `trash` crate of
  `curvyo-storage-io`, `trash_font_files`) and deletes it in the browser (no trash there), with
  the deleted notice.
- **P29 (metadata conflict notice).** The glue remembers the fields it changed locally until an
  import proves the room knew every local change; an incoming change to one of them that is
  concurrent with a local edit (the import did not see this replica's history) shows "<Font>: newer <field> from your devices kept." (at most 70 characters, name cut to 24).

## Questions

None: data leaving the machine for fonts is covered by Fragen 214 to 217; P22 is reversible in one
PR (bumping to 2 would need the architect and a release note).

## Tasks (slice 2)

- [x] F1 `curvyo-document-core::library_fonts`: entries, set/update/switch-off/remove, reads that
  skip bad shapes, budgets as pure functions (criteria 16, 17, 19).
- [x] F2 `curvyo-sync-core::library_blob` + goldens (criteria 16, 18, 20).
- [x] F3 `curvyo-ui-core`: `library_text` words, `font_library_sync` metadata (criteria 17, 19).
- [x] F4 `curvyo-editor-wasm` glue: blob store, push/receive/snapshot, arrivals, budgets, notices;
  session-level tests with two devices (criteria 16 to 19).
- [x] F5 CPU budget test `tests/library_fonts_cpu_budget.rs` (criterion 20).
- [x] F6 wasm bindings, `curvyo-storage-io`/`curvyo-app` reception (criteria 16, 18).
- [x] F7 `frontend`: toggle (click only, no `S`: FONT-SKEY C), greyed states, pair line, host writes (criteria 16, 17, 19).
- [x] F8 docs rows, technical debt; full gate.

## Built and not built (slice 2)

Built: F1 to F8 (see the commits). Snapshot uploads are held back while the bytes of a switched-on
font are still arriving as parts (`DeviceGroupGlue::fonts_complete`), not while they are missing. Not built: PDF documents of a font folder do not
travel on the desktop (text only, technical debt); the room has no size cap (checked in the relay
and server: only the 64 MiB message limit, which a 4 MiB part never reaches); "last writer" of a
metadata field is Loro's order, not a wall-clock time (no timestamp travels with a font); the
fonts arrival notice is its own notice, not merged into the machine and material batch notice.

## Review round (2026-11-20)

Findings of the architect, UX and tester reviews, fixed on this branch:

- A font whose bytes never arrive no longer blocks snapshots; only a blob still arriving as parts
  does, and one quiet for 10 minutes is given up (Proposal). A blob push that failed stays owed.
- Receiving side: held plus arriving bytes count against the 512 MiB budget (blobs no entry names
  make room first), a blob is refused with a notice when it would pass it or is not the size its
  entry says; entries over 16 MiB (font) or 1 MiB (licence file) or with more than 16 licence
  files are not read at all.
- A room snapshot never reads as the deletion of a font (it can replace the whole list); the
  browser and the desktop trash a font only when it was written here through sync (a set of
  hashes in `localStorage`, `curvyo.fontsViaSync`).
- The last part of a blob is padded to 64 KiB (the relay cannot recognise a known font by its
  exact size); `BlobAssembler::start` takes part 0 into the assembly.
- Reception names go through `sanitised_name` (no reserved Windows names, `:` or trailing dots);
  the browser checks licence names with the document rule and never overwrites a file.
- FONT-SKEY C (customer): no `S` key in the fonts tree. A press on a partly synced family
  switches all of it off; the first-sync confirm ignores groups that hold font entries; a refusal
  after an automatic switch-on is told once per font, not at every start.
