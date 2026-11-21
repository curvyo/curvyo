# Plan for the bed size from the controller

## Affected crates/modules

- `curvyo-connector-io`: `grbl/settings.rs::travel()` drops a non-finite value (criterion 3).
  Tests: unit in `settings.rs`; `tests/grbl_travel_0131.rs` on the simulator (`SimConfig` gains
  `travel`, tests only).
- `curvyo-laser-core`: `machine_profile.rs` gains `machine_size_offer` (criteria 1, 3).
- `curvyo-document-core`: `ProfileChange::UseMachineSize`, commit label `use_machine_size`,
  registered in the history row lists (criterion 2). No format change.
- `curvyo-ui-core`: history name "Use machine size", step scope entry.
- `curvyo-editor-wasm`: `machine_size_offer` and `use_machine_size` on the session and on
  `WasmSession` (criteria 1, 2, 4).
- `frontend`: `useWorkbenches` calls, offer row in `MachineBlock.tsx` (criteria 1, 2, 5).

## Tasks

- [x] 1. `-io`: failing tests, then `travel()` finite only; `$13=1` leaves travel as is (3).
- [x] 2. `-io`: simulator `travel`, `Welcome.controller.settings.travel = [400, 850]` (1, 6).
- [x] 3. `laser-core`: table test, then `machine_size_offer` (1, 3, 6).
- [x] 4. `document-core` and `ui-core`: `UseMachineSize` and its label (2).
- [x] 5. `editor-wasm`: session tests (one undo, library write for a user profile, none for the
  built-in, offer returns after a hand edit), then the calls (1, 2, 4).
- [x] 6. Frontend: offer row, kept place, focus, screen-reader line, tests (1, 2, 5).
- [x] 7. Docs: design-system row, spec link row; gate; browser build shows no row (5).

## Validation

Table and session tests in Rust, connector test on the simulator, frontend component test, one
check of the browser build. No timing tests.
