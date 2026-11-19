# Plan for the fix "a Pause pressed while the machine homes is forgotten" (0133)

Criteria PH1 to PH11. Design: architect addendum P1 to P8 in `adrs.md`; UI: `ux-notes.md` H1 to
H8. Branch `fix/pause-while-homing` from `main` (PR #399 merged). No new crate, no new dependency,
no `format_version` change; protocol stays 0 (additive `Status.pause_pending`).

## Cause (PH10, confirmed with the simulator before the fix)

The connector wrote GRBL's feed hold `!` while the job's `$H` ran; GRBL ignores `!` while it homes,
so no `Hold:0` came and `0127` G29 dropped the pause after 2 s, and the job streamed on when
homing ended. Reproduced on `main` with the silent homing of `0129`: Pause one second into a 9 s
`$H`, the job ran to `Done { Completed }` with every line after `$H` sent.

## Affected crates/modules
- `curvyo-connector-core`: `message.rs` (`Status.pause_pending`), `machine/mod.rs`
  (`Event::PauseDeferred`, the flag, cleared on leaving `Running`), `machine/controls.rs` (Pause
  idempotent, Resume cancels, Stop clears, `Held`), `machine/home.rs` ("No job to pause"); every
  `Status` literal gains `pause_pending: false`; golden `status_pause_pending.json`.
- `curvyo-connector-io`: `grbl/stream.rs` (`Hold::Deferred`, the hand-over after `$H`'s answer,
  `Requested` without limit), `grbl/controller.rs` (pause, resume, drop rule), `grbl/replies.rs`
  (`!` on `$H`'s `ok` before `fill`).
- `frontend`: `lib/connectorLink.ts` (`pause_pending?`), `lib/connectorText.ts` and
  `lib/machineControls.ts` (state line, Pause pending look, Resume enabled),
  `components/panel/MachineControls.tsx` (two-line tooltip).
- `docs/design-system.md` row "Machine controls (Operations tab)".

## Tasks
- [x] Reproduce with the simulator (PH10).
- [x] Core: field, golden, `PauseDeferred`, clear rules, "No job to pause" (PH1, PH3 to PH6, PH9).
- [x] `-io`: deferred hold, `!` on `$H`'s `ok`, no drop limit; tests `grbl_pause_homing.rs`
      (PH1, PH2, PH4 to PH7, PH9), `stream.rs` unit tests.
- [x] Frontend: state line, Pause busy without time limit, Resume and Stop enabled (PH8).
- [x] Docs: design-system row, spec status (PH11 gate).

## Validation
Core unit tests in `tests/confirming_machine.rs`, golden files, simulator tests (stepped time),
frontend table tests, full gate, the new connector tests 20 times under CPU load.
