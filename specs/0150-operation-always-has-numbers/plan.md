# Plan for operation always has numbers

Implementer, 2026-11-24. Built with the defaults of Fragen 337 to 340 (all A) and `adrs.md`
decisions 1 to 8. `format_version` 59 is the next free number on `origin/main` (58; #464 took 57, #470 took 58).

## Affected crates/modules

- `curvyo-laser-core`: `machine_profile.rs` (`DefaultNumbers`, `ProfileValues.defaults`, the
  two clamps, the max-feed limit), `machine_profile_copy.rs` (two keys, read per field),
  `machine_profile_file.rs` (record format 7), `operation_settings.rs` (`read_settings` with
  defaults, `SettingsRead::None` gone), `operation_settings_rule.rs` (`SettingsState::NoSettings`
  gone), `job_numbers.rs`, `job_gate.rs`, `job_gate_cause.rs`, `job_gate_text.rs`
  (`NumbersRead::NoSettings`, `Cause::NoSettings` gone), `operation_naming.rs`, `test_grid_ops.rs`.
- `curvyo-document-core`: `operation.rs` (`NewOperation.settings`), `manufacturing.rs`
  (`write_profile` fills, `EditDefaultPower`, `EditDefaultSpeed`), `history/rows.rs`,
  `document.rs` (`CURRENT_FORMAT_VERSION` 59), golden `operation_defaults_v59.curvyo`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs`.
- `curvyo-editor-wasm`: `manufacturing/machine.rs` (two setters, Max feed lowers the speed, fills
  before a pair change), `operations/kind_press.rs` (numbers in the add step),
  `operations/settings.rs`, `options.rs`, `settings_view.rs`, `mod.rs` (draft removed),
  `materials/refresh.rs` and `project.rs` (save fill), `output/input.rs`, `frame.rs`,
  `raster_view.rs`, `tab_facts.rs`, `manufacturing/machine_view.rs`, `wasm_machine_profiles.rs`.
- `frontend`: `MachineBlock.tsx` and `useWorkbenches.ts` (two fields), the removals of decision 6
  in `operationSettingsView.ts`, `OperationSettingsBlock.tsx`, `OperationRow.tsx`,
  `operationSettingsRows.ts`, `operationsView.ts`.

## Tasks

- [x] 1. laser-core: `DefaultNumbers` (10 %, 1,000 mm/min), `ProfileValues.defaults`,
  `clamp_default_power`, `clamp_default_speed`, Max feed lowering the Default speed; tests first
  (AC 1, 2).
- [x] 2. laser-core: copy keys `default_power_pct`, `default_speed_mm_min`, read per field with
  the max-feed limit, never rewritten on read; record format 7 with its damage cases and the
  profile goldens of `adrs.md`; tests first (AC 3).
- [x] 3. laser-core: `read_settings(value, kind, defaults)`, an absent register reads own numbers;
  `SettingsRead::None`, `SettingsState::NoSettings`, `NumbersRead::NoSettings`,
  `Cause::NoSettings` and its text removed; ranks of the others kept; tests first (AC 4 to 6,
  13, 15, 17 to 19, 23 to 27).
- [x] 4. document-core: `NewOperation.settings` written in the add step; `write_profile` takes
  the fills; labels `edit_default_power`, `edit_default_speed`; `format_version` 59 with the
  golden `operation_defaults_v59.curvyo`; `laser_settings_v43.curvyo` criterion-16 test; tests
  first (AC 4, 8, 15, 16, 20 to 22, 28).
- [x] 5. ui-core: History names and step scope of the two labels (AC 22).
- [x] 6. editor-wasm: Machine block view and setters, kind press with numbers, the three fill
  points, draft removed, frame and raster arms; session tests (AC 1, 2, 4, 8 to 11, 14 to 16,
  20 to 22, 25, 26).
- [x] 7. frontend: Default power and Default speed fields after Max power; "No settings" lines
  and the `no_settings` state removed (AC 1, 23, 27; W1 to W12).
- [x] 8. Docs: spec status, `specs/README.md` row in "In progress, Ready", design-system row
  already amended; full gate, `cargo deny`, `scripts/check_docs.py` (AC 28, 29).

## Validation

- Golden project file `curvyo-document-core/tests/fixtures/operation_defaults_v59.curvyo` (the
  cases of `adrs.md`), opened, read and saved back unchanged.
- Old file `laser_settings_v43.curvyo`: open reads own 10 % / 1,000 mm/min; a save without a
  change commits nothing and leaves `document.loro` byte-equal; a save after a change stores the
  numbers.
- Profile record goldens in `curvyo-laser-core/tests/fixtures/machine_profiles/`
  (`format7_defaults.json` byte round trip, four damage cases, format 5 and 6 read the built-in
  pair).
- G-code of an operation on derived numbers equals one with the same typed numbers (AC 26).
- AC 29: no network, device I/O or telemetry is added (no new dependency, core crates only
  take values).
