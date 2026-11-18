# Plan for Connector speaks GRBL over USB serial (0127, slice 4 of 0064)

Criteria G1 to G34 (G33 and G34 came with the customer's answers to Fragen 239 B and 240 B). Design: architect addendum C1 to C13 in `specs/0064-machine-connector/adrs.md`;
UI: `ux-notes.md`. Branch `story/connector-serial` from main. Commits by layer (Conventional
Commits, scope = crate): core, `-io` GRBL, server, app and frontend, integration test and docs.
No new crate, no `format_version` change, protocol stays 0.

## Affected crates/modules
- `curvyo-connector-core`: `message.rs` (`Status.detail`, `ControllerInfo.port/baud/settings`,
  `ControllerSettings`), `machine/` split into `mod.rs` and `controls.rs` (confirm-before-state
  Pause, Resume, Stop; `Held`, `Resumed`, `Stopped`, `JobFailedWith`, `StopUnconfirmed`,
  `NoControllerDetail`, `ControllerWord`, `UnlockPressed`; `ControllerCommand::Unlock`), goldens.
- `curvyo-connector-io`: `controller.rs` (trait, `Advance`), `fake_controller.rs` (implements it),
  `grbl/` (`prepare`, `flow`, `reply`, `texts`, `settings`, `identify`, `stop`, `stream`,
  `controller`), `serial_link.rs` (port I/O thread), `hub.rs` (trait object, `Welcome` on change,
  unlock, loss), `server.rs` (`ControllerChoice`), `session.rs`/`watch.rs` (forward `Welcome`),
  `job_log.rs` (`Failed: <text>`), `examples/grbl_pty.rs`, `tests/common/grbl_sim/`.
- `curvyo-connector-server`: `args.rs` (`--serial`, `--baud`), `signals.rs` (SIGINT/SIGTERM by
  `sigwait`), `serial_notes.rs` (ModemManager line), `--help`.
- `curvyo-ui-core`: `controller_warnings.rs` (G19 mismatch lines); `curvyo-editor-wasm`: one call.
- `curvyo-app`: `TestAnswer` carries the controller info, the watcher forwards `Welcome`.
- `frontend`: Controller block, new state-line words, Stop and Pause busy until the end state.
- `deny.toml` (MPL-2.0 for `serialport` only), `docs/design-system.md`, `docs/guides/connector-serial-linux.md`.

## Tasks
- [x] 1. Core wire fields `Status.detail`, `ControllerInfo` port/baud/settings; old goldens unchanged, new goldens `status_failed`, `status_lost`, `status_controller_state`, `welcome_grbl` (G7, G15, G19, G21)
- [x] 2. Core confirm-before-state machine (`Machine::confirming`): Pause/Resume/Stop wait for `Held`/`Resumed`/`Stopped`, "Stopping" detail, failure and loss texts, sticky loss text after reconnect, first alarm text stands, `Unlock` only in Alarm, `LaserModeOff` refusal (G7, G10 to G12, G14, G15, G29, G30, G33, G34)
- [x] 3. `-io` pure GRBL parts with table tests and fixtures: `prepare` (G5), `flow` (G6), `reply` and `settings` (G4, G19), `texts` (G7, G13) (G21)
- [x] 4. `Controller` trait, `FakeController` and `GrblController` (identify, polling, streaming, pause, resume, stop, alarm, loss, unlock) (G2 to G4, G6 to G15, G17, G20, G28, G29)
- [x] 5. GRBL simulator in `tests/common/grbl_sim/` and in-memory behaviour tests, one per behaviour; golden jobs of `0029` and a 20,000-line job (G6, G22)
- [x] 6. Hub with the trait object, `ControllerChoice`, serial thread, `Welcome` on change, log texts (G3, G15, G18, G19, G30)
- [x] 7. Server: `--serial`, `--baud`, usage errors, signals, terminal lines, ModemManager line, `--help` (G1, G14, G16, G27, G31)
- [x] 8. `deny.toml` and `Cargo.lock` (G25, G32)
- [x] 9. `curvyo-ui-core` warning lines and wasm call; app bridge; frontend Controller block, words, busy rules; Unlock button; frontend tests (G2, G4, G5, G7, G12, G13, G15, G19, G30, G33, G34)
- [x] 10. pty integration test with `RunningServer` and the simulator; `grbl_pty` example (G22, G23)
- [x] 11. Design-system rows, Linux guide, full gate and `cargo deny` (G26, G27)

## Changes against the first plan (customer answers of 2026-11-18)
- Frage 239 B: no terminal `unlock`; the app's Unlock button sends the new protocol message
  `Unlock` (additive, `command_unlock.json`), answered with the status, logged `Unlock: <client>` (G33).
- Frage 240 B: `$32=0` refuses `SubmitJob` with `RejectReason::LaserModeOff` (additive,
  `reject_laser_mode_off.json`); the GRBL controller also refuses `Start` then, as a second guard (G34).
- Frage 241 B: the terminal prints the alarm only. No new behaviour needs the connector's terminal.
- `Machine::new` keeps slice 2's immediate Pause/Resume/Stop (fake controller, the `0064` P tests);
  `Machine::confirming` waits for the controller (GRBL). (Proposal)

## Changes after the architect's safety review (2026-11-18)
- [x] 12. Laser in a pause without relying on `DISABLE_LASER_DURING_HOLD`: `$32` unknown refuses
  every job (`RejectReason::LaserModeUnknown`, `reject_laser_mode_unknown.json`; the controller
  refuses `Start` too); in a hold the `A:` field decides `0x9E` (once). Simulator modes: laser on
  in a hold, `Bf:`, door, silent at the Stop (G10, G34)
- [x] 13. The 3 s rule while the stop's reset waits (`StopUnconfirmed`); a banner during the hold
  step finishes the stop; `Start` ignored while a stop is pending (G12, G15)
- [x] 14. `Bf:` cross-check for a foreign `ok` with lines in flight (G28); `reply` parses `Bf`
- [x] 15. Safety bytes alone at a loss; `prepare` refuses characters outside `0x20`-`0x7E`;
  `GetStatus` judges a stale controller; `Door`/`Check`/`Sleep` are `Busy` (G4, G5, G15)
- [x] 16. Exit joins the serial thread and the notice printer instead of sleeping 100 ms. A
  second SIGINT or SIGTERM during the stop at exit is ignored: the signals stay blocked and only
  the first is awaited (G16). `hub.rs` split: pairing moved to `hub/pairing.rs`

## Validation
Pure parts by table tests and golden files in `curvyo-connector-io/tests/fixtures/grbl/`. Behaviour by
the simulator over an in-memory byte channel with stepped time (no wall-clock sleeps), asserting the
simulator's byte log (no overflow, no real-time byte inside a line, laser off after every stop,
alarm and loss) and the state at every status push. One end-to-end test over a real pty (Linux).
The real laser (G24) is the customer's check; the PR says which steps were done there.
