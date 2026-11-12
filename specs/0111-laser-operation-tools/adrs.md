# ADRs for laser operation tools

Status: Proposed (architect, 2026-11-12). Builds first of the three redesign stories (`0111`, then `0112`, then `0113`). File shape: [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md) (`needs-customer`, accepted with the story PR). Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md): group assignment with existing `member.` keys, clone-member keys read as legacy (Frage 212 A), kind-button reuse rule, `main.interval_mm` in `op_settings`, operation-only settings in a new `op_options` register, `motorised_z` in `ProfileValues`, the page copy and profile record format 4; `format_version` 44.
- [ADR 0046](../../docs/adr/0046-laser-operations-record.md): operation nodes, membership keys, nearest rule, delete and off-page sweeps; reused; §2 and §7 amended by ADR 0056 §1 and §2.
- [ADR 0054](../../docs/adr/0054-laser-settings-and-test-grids-in-the-project-file.md): `op_settings`, link, overrides, refresh; §1 and §2 amended by ADR 0056 §5 (Interval). The test grid part stays as built until `0112` (ADR 0057).
- [ADR 0038](../../docs/adr/0038-machine-profile-records.md) and [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md): record and page copy, gain `motorised_z` (ADR 0056 §7).
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3, §4, §7: opaque plain values in document-core, never repaired on read, the bump rule.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one commit per assignment and per settings edit is one undo step.

## Feature-local decisions

- 2026-11-12: **Format and build order.** `format_version` 44 (main is at 43) and profile record format 4; the numbers are taken at merge. `0112` takes 45, `0113` takes 46 and profile record format 5. If the order changes, the numbers follow the merge order, not the story numbers.
- 2026-11-12: **Operation-only settings are a separate register `op_options`, not a sub-map of `op_settings`** (ADR 0056 options): no material path can touch them, a toggle and a save-time refresh merge without loss, and "No settings" can carry options. Interval sits in `op_settings.main` because `0112` makes it a material number.
- 2026-11-12: **Interval link rule.** In this slice no material has an interval, so pick, refresh and Reset keep the stored `main.interval_mm` and a typed interval writes `main`, not an override (criterion 19). `0112` needs no shape change.
- 2026-11-12: **Group assignment** writes the group's key plus the key on every inner node that already holds an own assignment (ADR 0056 §1); unticking removes them for that operation. No member list, no new key form. Inner rows of placed pages are not reached (the tick reads mixed) **(Proposal)**.
- 2026-11-12: **Clone-member keys** of old files: Frage 212 default A, read as legacy keys, kept and saved back, ignored by every rule; `operations_v39.curvyo` and `operations_clone_member_v39.curvyo` must open with members running the clone's operations. Frage 213 default A: inner rows of placed pages stay.
- 2026-11-12: **Session-only state**: entered group, unfolded row, Operations view toggle; none in the file, none a history step.
- 2026-11-12: **Undo labels**: the assign and unassign labels of `0101` (names "Assign to <name>", "Remove from <name>") for kind buttons, group ticks and Shift+press; `edit_operation_settings` ("Edit settings") for Interval, every option and Z offset; profile copy field edit for `motorised_z` as the other machine values.

## Crate placement (no new crate)

- `curvyo-laser-core`: `operation_options.rs` ("the operation-only settings with their ranges and defaults": `ScanAngle` tenths `i16`, `Overscan` percent `u8`, `ZOffset` tenths `i16`); `operation_group.rs` (ADR 0056 §1); `Interval` newtype in `material.rs`; the reuse rule in `operation_naming.rs`; `operation_settings.rs` and its rule read `interval_mm`; `machine_profile.rs`, `machine_profile_file.rs` (format 4) and `machine_profile_copy.rs` gain `motorised_z`; `operation_nearest.rs` ignores legacy paths.
- `curvyo-document-core`: `op_options` constant, plain-value read and write (`operation.rs`, `operation_codec.rs`); the assign command refuses a path key; `CURRENT_FORMAT_VERSION` 44 with its doc block.
- `curvyo-editor-wasm`: `session/operations/` (kind buttons, group assign, unfold, options edit), entered-group state, the view toggle per page.

## Architecture review findings (2026-11-06) in this slice

- **Finding 1 (session hub):** the entered group, the unfolded row and the view toggle live in one feature-owned struct in `session/operations/` with methods (`enter_group`, `leave_group`, `toggle_view`); page activation calls one `leave_group` and touches no field of it. No new field on `Session` beyond that struct.
- **Finding 2 (frontend hook):** kind buttons, the crumb, the canvas toggle and the unfolded row get their own hooks or components (`useOperationTools`, `OperationRow`), not new code in `useEditorSession.ts`; the hook gains at most one refresh call.
- **Finding 3 (JSON contracts):** `operations_view` gains `options`, `interval`, `z_offset_shown`, the group tick state and `select_bar_kinds`; decoders in `frontend/src/lib/operationsView.ts` and `operationSettingsView.ts` check every field. Shared fixtures in `curvyo-editor-wasm/tests/fixtures/operations_view/`: `options_engrave.json`, `options_cut_z_hidden.json`, `options_damaged.json`, `group_tick_mixed.json`, `select_bar_kinds.json`. The Rust test builds each scenario and compares the JSON value; the TypeScript test decodes each and rejects a copy with a missing field and one with a wrong type.

## Goldens and budgets

- Goldens: ADR 0056 §9 (two project goldens, two profile goldens, `newer_format.json` to 5).
- Budgets confirmed (criterion 26, thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`): kind button assigning 1,000 items, assigning a group of 1,000 leaves, entering a group of 1,000 leaves, unfolding a row on a page with 50 operations: each within 16 ms. The group assignment uses the session's membership index (`0101` adrs), never a full drawing read.

## Spec changes for the product owner

- S1. Criterion 25: the golden names of ADR 0056 §9; `operation_options_damaged_v44.curvyo` added.
- S2. Criterion 19: a typed interval on a linked operation is stored on the operation, not as an override, until `0112`.
- S3. Criterion 22: the capability is also in the profile record (format 4); an older record reads it as off.

## Questions for the customer

None new. Frage 212 and 213 keep their defaults (A, A). The ADR is `needs-customer` and is accepted with the story PR.
