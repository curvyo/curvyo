# Plan for Laser operation tools (0111), Parts A to E

Branch `story/laser-operation-tools`, on `main` after spec PR #336 (format 43, target 44; profile
record format 3, target 4). Built on `specification.md` (criteria 1 to 27), `adrs.md`, ADR 0056 and
`ux-notes.md` (T1 to T12, M1 to M8). Frage 212 = A (clone-member keys kept, ignored, written back
unchanged), Frage 213 = A (inner rows of placed pages stay). Curvyo is not deployed: no legacy path
beyond what the spec and ADR 0056 name (format 43 opens with defaults; `operations_v39` and
`operations_clone_member_v39` still open). One PR for the slice; Parts A to D are commits on this
branch, each ending with the gate of its crates; Part E runs the full gate of `CLAUDE.md` §7.

## Affected crates/modules

- `curvyo-laser-core` (pure, wasm32): new `operation_options.rs` (the operation-only settings with
  their ranges and defaults), new `operation_group.rs` (the keys a group assignment writes and the
  group's tick state); `material.rs` (`Interval`); `operation_settings.rs` and
  `operation_settings_rule.rs` (`main.interval_mm`, link rule of ADR 0056 §5); `operation_naming.rs`
  (last operation of a kind); `operation_nearest.rs` (path keys are legacy); `machine_profile.rs`,
  `machine_profile_file.rs` (record format 4), `machine_profile_copy.rs` (`motorised_z`).
- `curvyo-document-core`: `operation.rs` (`op_options` register constant), `operation_codec.rs`
  (plain-value read and write of `op_options`), `operation_members.rs` (assign refuses a path key;
  group write and unwrite in one commit), `document.rs` (`CURRENT_FORMAT_VERSION` 44 and its doc
  block). Goldens in `tests/fixtures/`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (`motorised_z_on`, `motorised_z_off`; the
  assign, remove and `edit_operation_settings` labels are reused).
- `curvyo-editor-wasm`: `session/operations/` new `tools.rs` (one feature-owned struct
  `OperationTools`: entered group, unfolded row, view toggle per page; `enter_group`, `leave_group`,
  `toggle_view`, `unfold`), new `kind_press.rs` (reuse rule, Shift create, skip notice), new
  `options.rs` (options and Interval edits); `assign.rs`, `items.rs`, `index.rs` (group leaves,
  legacy keys ignored, clone members gone), `view_json.rs`, `settings_view.rs`, `look.rs` (look off
  when the view is unpressed); `session/context.rs` (laser-page group entry read-only);
  `session/manufacturing/machine.rs` (copy field `motorised_z`); `wasm_operations.rs`. Fixtures
  `tests/fixtures/operations_view/`; budget `tests/operation_tools_budget.rs` on `tests/cpu_time/`.
- Frontend: `lib/operationsView.ts`, `lib/operationSettingsView.ts` (new fields), new
  `hooks/useOperationTools.ts`, `components/SelectToolbar.tsx` (Operation group), new
  `components/OperationKindGroup.tsx`, `components/OpenGroups.tsx` (page crumb, Read-only),
  `components/PageFrames.tsx` (Operations view toggle), `components/panel/OperationRow.tsx`
  (unfold), `OperationSettingsBlock.tsx` (moves into the row; Interval), new
  `components/panel/OperationOptionsPart.tsx` (lower part), `OperationsSection.tsx` (removals
  below), the Machine block (Motorised Z axis switch), `hooks/useScrollWindow.ts` (one tall item).

## Removed from the Operations tab

- The add row of the five kind buttons and every route that creates an empty operation (criterion 8;
  the session's empty `add_operation` entry point and its wasm binding go too).
- The "View: Operations / Drawn" radiogroup (criterion 16); the state moves to the page label.
- The settings block below the list for one selected row (criterion 17); it becomes the unfolded
  row's body. The multi-row material pick under the list stays (`0028` criterion 27).
- The "Clone members" group (`CloneMembers` in `OperationsSection.tsx`) and `cloneMembers` in the
  view JSON and its decoder (criterion 14, Frage 212 A).
- Stays: Cuts last, the list, the summary, the Test grid button (until `0112`), ticks, drag onto a
  row, digits 1 to 9.

## Part A: core rules and format 44 (Opus, medium)

Tests first in every task.

- [x] A1. laser-core `operation_options.rs`: `ScanAngle` (tenths `i16`, -900 to 900), `Overscan`
  (percent `u8`, 0 to 50), `ZOffset` (tenths `i16`, -200 to 200), `OperationOptions` with defaults
  (crosshatch false, bidirectional true, 0.0, 2, 0.0); read from plain values with damage (not a map,
  unknown key, wrong type, out of range, too many decimals, hatch key on a non-engrave kind); write
  the whole map; validation texts of UX section 10 (AC 20, 24, 25).
- [x] A2. laser-core `material.rs` `Interval` (hundredths `u16`, 1 to 200, default 10);
  `operation_settings.rs` reads `main.interval_mm` and `overrides.main.interval_mm` on `engrave` and
  the hatch of `engrave_outline`, damage on `outline` or another kind, absent reads 0.10;
  `operation_settings_rule.rs`: pick, refresh, Reset and Use own numbers keep the stored interval;
  `edit_number` for Interval writes `main`, never an override (AC 19, 23, 25).
- [x] A3. laser-core `operation_naming.rs` `last_of_kind(ops, kind)`: lowest in the list, not
  Unknown, not Damaged, not `op_grid`; `next_name` unchanged (AC 3 to 6).
- [x] A4. laser-core `operation_group.rs`: `group_assign_keys(group, index, op)` (group key plus
  every inner node, any depth, placements included, with an own key on any operation; placed-page
  inner rows not reached), `group_unassign_keys` (group key plus O's inner keys), `group_tick(group,
  op)` by the nearest rule over the leaves: Checked, Mixed, Unchecked (AC 9, 10).
- [x] A5. laser-core `operation_nearest.rs`: a key whose path holds more than one id is legacy:
  ignored by the chain, ticks and counts; a leaf inside a clone ends its chain at the clone (AC 14).
- [x] A6. laser-core `machine_profile.rs` `ProfileValues.motorised_z` (default false, Generic
  laser false); `machine_profile_file.rs` record format 4 (`motorised_z` after `laser_mode`, always
  written; formats 1 to 3 read false and are not rewritten; the key in format 1 to 3 damages; above
  4 skipped); `machine_profile_copy.rs` `mfg_profile.motorised_z` (absent false, non-boolean damaged,
  Differs compares it). Goldens `tests/fixtures/machine_profiles/format4_motorised_z.json` (byte for
  byte), `format3_motorised_z_key.json` (damaged); `newer_format.json` to 5 (AC 22).
- [x] A7. document-core: `op_options` constant and plain-value read and write in `operation.rs`
  and `operation_codec.rs` (interprets nothing, ADR 0033 §3; a fixture job never reads it); the
  assign command refuses a path key; the group write and unwrite as one commit with the labels
  "Assign to <name>" / "Remove from <name>" (AC 9, 10, 14, 20).
- [x] A8. document-core format 44: `CURRENT_FORMAT_VERSION` 44 with its doc block (migration none);
  a format 45 file refused; format 43 files open with defaults and are not rewritten on open.
  Goldens: `operation_tools_v44.curvyo` (Engrave with interval and every option, Engrave + Outline
  linked to a material with its own interval, a Cut with Z offset under a copy with `motorised_z`
  false, a second manufacturing with it true, a group assigned by A4 with one member in another
  operation) reads and writes back equal; `operation_options_damaged_v44.curvyo` (options not a map,
  unknown key, scan angle 90.1, overscan 51, crosshatch on a Cut, interval on a Cut, interval 0.005)
  reads each as Damaged settings and writes back unchanged; `operations_v39` and
  `operations_clone_member_v39` open, member keys run nothing and save back (AC 14, 25).
- [x] A9. ui-core labels `motorised_z_on` ("Motorised Z axis on"), `motorised_z_off` in
  `history_names.rs` and one step-scope list (AC 22).
- Gate A: fmt, clippy, nextest for laser-core, document-core, ui-core and the editor-wasm tests that
  pin the format; wasm32 build of the core crates.
- Part A done (2026-11-12). Deviations:
  - A2: a typed Interval goes through a new `edit_interval` (not `edit_number`/`TypedNumber`, so
    the session's exhaustive matches stay); `use_material` takes the previous `SettingsRead` to
    keep `main.interval_mm` (its one caller updated); `RunSettings.interval` is `Some` for Engrave
    kinds only; an `overrides.main.interval_mm` from a file is read, kept and runs, but is no
    `Field` (no "Differs" line in this slice); new damage `SettingsDamage::BadInterval`.
  - A1: damage is its own `OptionsDamage`; `OperationOptions::running_z_offset(motorised_z)` is
    the run rule of criterion 21. A3: `last_of_kind` also skips damaged `op_options`.
  - A4: the pure rule takes the group's inner node ids and its leaves' chains (the session gives
    them from its index in B2), and returns only the keys that change.
  - A6/A9: `ProfileChange::MotorisedZ { on }` in document-core with the labels
    `motorised_z_on`/`motorised_z_off` (history rows list, ui-core names and step scope).
  - A7: the group write and unwrite are the existing `set_operation_members` (one commit, assign
    or remove label); `Document::write_operation_options` writes the whole map ("Edit settings").
    **Moved to B2:** the assign command's refusal of a path key, together with the removal of the
    session's clone-member route that still writes one (refusing now breaks that route's tests).
    The laser rules already ignore path keys (A5).
  - A8: the clone-member generator of `operations_format.rs` is gone (the v39 golden on disk is the
    record); the path-key sweep tests plant the legacy key in a saved file.
  - Superseded pins updated: format 43 to 44 (document-core tests, `acceptance_0106_tester`,
    `acceptance_0024_slice2_tester`, the SVG export `*.document.json` goldens), profile record
    format 3 to 4 (`acceptance_0026/0102/0103` testers of laser-core and editor-wasm,
    `machine_profiles.rs`), the copy key lists (`motorised_z`), `acceptance_0023_tester` skips
    `operation_tools_*` (holds a group), and two clone-member session tests now expect the legacy
    key to run nothing (`operations_look_session`, `operations/tests.rs`).

## Part B: session, view JSON, wasm bridge, budgets (Opus, medium)

- [x] B1. `session/operations/kind_press.rs`: `press_kind(kind, shift)` on the selection's targets:
  reuse by A3 or create "<Kind> <N>" at its section end (Show on, Output on), assign in one step;
  no step when nothing changes or only guides, dimensions, fixtures; skip notice of `0101`
  criterion 10 (at most 70 characters); polite status text "Cut 3 added." / "Assigned to Engrave
  1."; a group target goes through A4 (AC 3 to 7, 9). Session tests in `operations/tests.rs`.
- [x] B2. `assign.rs`, `items.rs`, `index.rs`: tick per row from A4 for a selected group (mixed
  with "Group 3 in part" / "2 groups in part"), mixed press adds the rest, untick removes by A4;
  the membership index serves group leaves without a drawing read; clone member items and their
  assignment route removed; legacy path keys ignored (AC 10, 12, 14).
- [x] B3. `session/context.rs` with `tools.rs`: on a laser page the double-click (and Enter with one
  group selected) enters read-only, one level per double-click; inside, selection and marquee of
  members only; drag, nudge, Delete, Ctrl+D, Cut, Paste, transform and edit keys and node edit
  refused silently; Escape leaves in one press with the group selected; a press outside leaves and
  selects what it hits; page activation calls `leave_group` once; not a history step. Drawing pages
  as built (AC 11 to 13). Session tests beside `grouping_tests.rs`.
- [x] B4. `tools.rs` view toggle per laser page (default pressed, session only, no step); `look.rs`
  draws the drawing's own style when unpressed, no chips, no Not assigned look (AC 15).
- [x] B5. `options.rs`: edit one option, toggle, Interval and Z offset, each one step "Edit
  settings" with the whole `op_options` map; out-of-range values refused with the texts of A1;
  material commands never touch `op_options` or the interval (AC 19, 20, 23, 24).
  `session/manufacturing/machine.rs`: `motorised_z` field edit on the page copy, one step (AC 22).
- [x] B6. `view_json.rs` / `settings_view.rs`: `select_bar_kinds` (shown or absent by criterion 1,
  description "Assigns to <name>" / "Creates <Kind> <N>"), `entered_group` (crumb words), per-row
  `unfolded`, `group_tick` and `group_in_part`, `options`, `interval`, `z_offset_shown`,
  `operations_view` per page; `cloneMembers` gone. Shared fixtures in
  `tests/fixtures/operations_view/`: `options_engrave.json`, `options_cut_z_hidden.json`,
  `options_damaged.json`, `group_tick_mixed.json`, `select_bar_kinds.json`;
  `unknown_and_clone_members.json` and `placed_and_clone_rows.json` lose the member rows. The Rust
  contract test `tests/operations_view_fixtures.rs` builds each scenario and compares the JSON value
  (AC 1, 2, 10, 15, 17 to 21).
- [x] B7. `wasm_operations.rs`: `press_operation_kind(kind, shift)`, `unfold_operation_row`,
  `toggle_operations_view`, `edit_operation_option`, `set_motorised_z`; remove the empty add
  operation binding (AC 8).
- [x] B8. Budget `tests/operation_tools_budget.rs` (thread CPU, `tests/cpu_time/`, release, in the
  `performance-budgets` job): kind press on 1,000 items, group of 1,000 leaves assigned, entering a
  group of 1,000 leaves, unfolding a row on a page with 50 operations; each within 16 ms. A failed
  budget is recorded in `adrs.md`, never silently changed (AC 26).
- Gate B: fmt, clippy, nextest for editor-wasm and ui-core; wasm32 build.
- Part B done (2026-11-12). Gate B green: fmt; clippy `-D warnings` (host and wasm32) for
  editor-wasm, ui-core, document-core; nextest for those three (editor-wasm in five chunks, the
  whole suite, Part A's last edits included); wasm32 build of editor-wasm; rustdoc. Budgets
  (release, thread CPU): kind press on 1,000 items 5.5 ms, group of 1,000 leaves 3.1 ms, entering
  it 0.4 ms, unfold among 50 operations with the tab's JSON 2.4 ms; `operation_tools_budget` is in
  the `performance-budgets` job. The frontend still calls the removed bindings until Part C.
  Deviations:
  - B1: the press returns `None` when nothing changes (no step); the polite status is read with
    `take_operations_status`. The skip notice stays as built (guides named, dimensions silent);
    the dimension and mixed texts of UX section 10 are not built.
  - B2: the row keeps one `tick` (`none`, `some`, `all`) computed by the group rule; the line-2
    words are `group_in_part` ("Group <short id>" or the group's own name, as the Layers tab names
    it; "2 groups" for several). `items` no longer counts legacy path keys. Moved from A7: the
    document's `add_operation` and `set_operation_members` refuse a path key (`NoSuchTarget`);
    `set_clone_member_in_operation` and the `clone_members` rows are gone.
  - B3: the entered group is the existing `0023` context stack; read-only is derived (any entered
    group while a laser page is active), so `OperationTools` holds the unfolded row and the view
    toggles only, and `Session` gains no field. Refused silently: the key table except Escape,
    undo/redo, Select all, Enter and the Select letter; every guarded command (`recording`);
    Delete; `clipboard_allowed` is false; a move or handle drag is cancelled at the press (a
    marquee goes on; new `SelectTool::area_drag_in_flight` in ui-core); a double-click only
    enters a nested group. The host's own Ctrl+D and paste routes are checked in Part C.
  - B4: unpressed pages are a set per manufacturing node; `pages_json` gains `operationsView`
    (`null` off laser pages); the tab's JSON has `operations_view` for the active page.
  - B5: one entry `edit_operation_option(id, option, value)` for Interval, the toggles and the
    fields; damaged `op_options` make the operation read as Damaged settings, so no material
    command writes it. `set_machine_motorised_z` on the page copy; the machine view JSON gains
    `motorised_z`.
  - B6: `select_bar_kinds` is a list of `{kind, description}` or `null`; `entered_group` is
    `{page, groups}` or `null`; the settings block gains `interval_mm`, `options` (`{hatch | null,
    z_offset_mm}`, `null` when damaged) and `z_offset_shown`. The `settings_*` fixtures of
    `material_test_library_view_contract` gained these fields.
  - B7: also `take_operations_status`; `toggle_operations_view` takes the page id;
    `set_operations_drawn` is gone.
  - D1 partly done here (the suites must stay green): `tests/operation_shims/` stands in for the
    old empty add and the view switch in the older tester files; `tests/legacy_key/` plants a
    path key as an old file holds it; in-crate tests use `Session::add_test_operation`. Rewritten
    to the new rules: `acceptance_0101_tester` c2 (two) and c18, `test_grid` criterion 6 (two),
    the clone tests of `operations/tests.rs`; `fixture_jobs_gaps_0103` ac16 and the 0101 key test
    got an item on the page.

## Part C: frontend (Sonnet, medium; after the finished UX notes)

- [x] C1. Decoders `operationsView.ts`, `operationSettingsView.ts` check every new field;
  `tests/operationsView.test.ts` decodes each B6 fixture and rejects a copy with a missing field and
  one with a wrong type (AC 1, 10, 17 to 21).
- [x] C2. `useOperationTools.ts` (the only caller of the new bindings; `useEditorSession.ts` gains at
  most one refresh call); `OperationKindGroup.tsx` in `SelectToolbar.tsx` after the settings group:
  five 28 px glyph buttons, tooltip and name the kind word, description, Shift press, Tab order,
  hidden off laser pages and for out-of-context selections; inside an entered group the bar holds
  only this group (AC 1 to 5, 11; UX 3, T1, T12).
- [x] C3. `OpenGroups.tsx`: page crumb, group, Lock "Read-only", "Esc: leave"; dimming 35 % and the
  accent outline on the canvas (AC 11, 13; UX 4.2, M3).
- [x] C4. `PageFrames.tsx`: `ListTree` toggle after the Laser chip, 24 px hit area, `aria-pressed`
  (AC 15; UX 5).
- [x] C5. `OperationRow.tsx` unfolds (one at a time, press folds, Enter/Space, `aria-expanded`,
  focus into the body and Escape twice back); `OperationSettingsBlock.tsx` as the upper part with
  Interval in Engrave and the Hatch group; `OperationOptionsPart.tsx` lower part (toggles, Scan
  angle, Overscanning, Z offset only when shown); validation chips; windowing with one tall item
  (AC 17 to 21, 24; UX 6).
- [x] C6. `OperationsSection.tsx` removals (section above), mixed tick and line 2 "· Group 3 in
  part"; the Machine block "Motorised Z axis" `PanelSwitch` (AC 8, 10, 16, 22; UX 4.1, 6.3).
- Gate C: `npm run build:wasm && npx tsc -b --noEmit && npm test && npx oxlint`.
- Part C done (2026-11-12). Gate C: build:wasm 0, `tsc -b --noEmit` 0, `npm test` 0 (1108 pass,
  215 skipped as before), `oxlint` 0 (warnings as before; the only new one in a touched file is the
  existing grid effect). No browser run (Part E).
  Deviations:
  - C1: the clone-member and view-mode decoders are gone with their fields; `tests/operationsView`
    decodes every B6 fixture and rejects a missing field and a wrong type per new field.
  - C2: `OperationKindGroup.tsx` is its own file (not inside `SelectToolbar.tsx`); the bar takes one
    `operationKinds` prop. Inside an entered group the bar is only that group, or absent.
    `useOperations` lost `press`, `setCloneMember`, `setDrawn`; the polite status text of a press
    is in the tab's sr-only region. A press selects its row (no unfold) and scrolls it in view.
  - C3: crumb, Lock "Read-only" and "Esc: leave" in `OpenGroups` (`crumbRow` takes the names). The
    35 % dimming and the group outline are drawn by the wasm canvas (`context_dim`, not touched
    here); Part E must look at the outline.
  - C4: `pages_json.operationsView` read in `PageFrames`; its root lost `aria-hidden` (the toggle
    is a Tab stop), the name and chip keep it. `Canvas` takes `onToggleOperationsView`.
  - C5: the body is the built Settings block (Power and Speed row, Passes row) with Interval beside
    Passes, not the UX 6.2 layout (UX 6.2 itself says "all as built"). Several selected rows keep
    the multi pick under the list. One Escape in a body field reverts it and puts the focus on the
    row (the field's own handler), not two presses. The body height is measured (ResizeObserver);
    `UNFOLDED_BODY_ESTIMATE` stands in for the first frame; the list grows to at most 520 px for
    the open row. The second click of a double-click never folds.
  - C6: the Test grid button stays alone in a toolbar named "Test grid". `set_motorised_z` is called
    from `useWorkbenches().machine` next to the other machine setters, not from the tools hook
    (the Machine block has no tools prop); `MachineBlock` gets `onReturnFocus`.
  - Host Ctrl+D and paste in a read-only entered group (open point of B3), checked: Ctrl+D is only
    the session's key table (answers `consumed`, so the host still `preventDefault`s the
    browser bookmark); the host's paste listener asks `clipboard_allowed`, which is false in an
    entered laser group. No host-side change needed. Dropping a file onto the canvas there is not
    guarded; noted for the lead. Resolved in Part D: `import_finish` refuses in a read-only entered group
    (notice "Read-only: press Esc to leave the group first.", alert, no step); covers drop, picker, paste.


## Part D: mechanical follow-ups (Haiku)

- [x] D1. Superseded tester tests (rows of "Changes to accepted specs"): `acceptance_0101_*`
  (empty add, every press creates, clone member assignment, View switch), `acceptance_0106_tester`
  (laser group entry editable, clone rows expand), `acceptance_0028_*` (block below the list, Test
  grid beside kind buttons), format 43 pins to 44, profile format 3 pins to 4. Each change listed
  in the PR.
- [x] D2. Docs at acceptance: ADR 0056 status and notes on ADR 0046 §2 §7, 0054 §1 §2, 0038 §2,
  0039 §1 §5; `docs/adr/index.md`; `docs/design-system.md` rows of UX M3 and section 14; spec
  `Status: Done (PR #n)` (lead in the PR).

## Part E: verify (Sonnet, medium)

- [x] E1. Chromium run of the demo path below (one agent drives the browser); screenshots to the
  scratchpad `0111/`; UX review by the ux-engineer; architect review (format, profile record).
- [x] E2. Full gate of `CLAUDE.md` §7, steps separately; report to the lead.

- Part D/E done (2026-11-12). Chromium run of the demo path, screenshots in the scratchpad `0111/`.
  Found and fixed: the entered read-only group drew handles and no outline (`build_context_outline`,
  no transform decoration), and the 35 % dimming did not reach the operation looks, hatch and chips
  of items outside (`look.rs`). Open: Escape does not leave while focus sits on a kind button (focus
  stays there by UX notes); undo label reads "Remove from operation (1 path)" not "Remove from
  <name>" (as the 0101 names); Interval is absent in the "No settings" state.
- UX review fixes (2026-11-12): Interval shows with "No settings" for Engrave and Engrave + Outline
  (0.10, or the typed value) and is kept in the session's settings draft beside power, speed and
  passes; it is written into `main.interval_mm` with them (typed power and speed, or a material
  pick), so "No settings" never stores a partial `op_settings` (Proposal; AC 19). The "Recognise
  text" bar seen in the 800 px review run was opened by the run's own clicks landing on the Select
  bar's buttons (the bar covers that canvas area at 800 x 640); it stays open across selection
  changes as `0088` defines. No change.

## Validation

- Tests first for A1 to A8 and B1 to B5; goldens read, write back equal, damage per case; shared
  Rust/TS fixtures pin producer and decoder; budgets in thread CPU time only.
- The tester writes `acceptance_0111_*` from the spec before reading this code.
- AC 27: no network, device I/O, G-code, output or telemetry code is added; the tester checks the
  diff. `0029` reads `op_options`, `interval_mm` and `motorised_z`.

## Demo path (desktop and browser)

1. New project, laser page with Generic laser; the Operations tab has no kind buttons and no View
   switch; the page label has the pressed `ListTree` toggle.
2. Draw rectangles A, B; select A: five symbols in the Select bar. Press Engrave: "Engrave 1"
   created and A assigned. Select B, press Engrave: B joins Engrave 1, still one row. Shift+Engrave
   on B: "Engrave 2". Select A, press Engrave: A goes to Engrave 2 (lowest).
3. Select a rectangle on a drawing page: no symbols.
4. Group two letters G, put letter B into Engrave 1; select G, press Cut: Cut 1 holds G and B; B
   still in Engrave 1. With G selected the Cut 1 tick is checked, the Engrave 1 tick mixed with
   "· Group 1 in part"; press the mixed tick: A joins Engrave 1 (one step); untick Cut 1: G and B
   leave Cut 1 (one step "Remove from Cut 1").
5. Double-click G: crumb "<page> › Group 1 · Read-only · Esc: leave", outside dimmed, no handles;
   drag and arrows do nothing; select A inside, press Score: A runs in Score 1. Escape: G selected.
6. Press the toggle in the page label: the drawing's own style; press again: operation looks.
7. Press Engrave 1: it unfolds under itself; Interval 0.10, Crosshatch off, Bidirectional on, Scan
   angle 0, Overscanning 2; no Z offset. Type Scan angle 90.1: chip "Scan angle is -90.0 to
   90.0°.", nothing written; 15: one step "Edit settings". Pick a material: options and Interval
   stay. Press Cut 1: Engrave folds, Cut unfolds with no lower part.
8. Machine block: Motorised Z axis on (one step): Cut 1 shows Z offset; type 1.5; switch off: the
   field goes; on again: 1.5 is back.
9. Save, reopen: options, interval, Z offset, capability, group keys as before; a clone of the group
   has no member rows anywhere.
