# Plan for Machine control, first slice (0118): Home and the Machine controls group

Criteria 1 to 7 of the first slice. Design: architect addendum M1 to M12 in `adrs.md`; UI:
`ux-notes.md` sections 1 to 14 (W1 to W14), Frage 262 A (decided by the customer 2026-11-19). Branch `story/machine-control`.
Commits (conventional, scope = crate, by layer): core, `-io`, app, frontend, docs. No new crate,
no new dependency, no `format_version` change; protocol stays 0 (additive `Home`, `Homing`).

## Affected crates/modules
- `curvyo-connector-core`: `message.rs` (`Message::Home`, `State::Homing`), `codec.rs`
  (18 known types), `machine/mod.rs` (`Event::Homed`, `Event::HomingFailed`,
  `ControllerCommand::Home`, the waiting kind), new `machine/home.rs` (M3, M5),
  `machine/controls.rs` (Stop and the local press in `Homing`), `send_watch.rs` (`Homing` holds
  progress). Goldens `command_home.json`, `status_homing.json`, `status_homing_ready.json`.
- `curvyo-connector-io`: new `grbl/home.rs` (`$H` write, `Owed`, answer), `grbl/controller.rs`
  (dispatch, loss-clock pause, stop without a stream), `grbl/replies.rs` (owed answers, homing
  alarm supersedes the lock), `fake_controller.rs` (2 s homing), `hub.rs` (`home`, log, local
  start words), `session.rs`, `job_log.rs` (`Home: <client>`), `watch.rs` (retry while homing);
  simulator `$H` refused while not idle (M7).
- `curvyo-app`: `connector.rs` (`"home"` command).
- `frontend`: `lib/connectorLink.ts` (shared `busy`, read state), `lib/connectorText.ts`
  (`Homing`, refused homing), new `lib/machineControls.ts` (Home's cause, the one state line, the
  footer notice, settle rules), `lib/sendLine.ts` (only the length rule left),
  `hooks/useConnectorLink.ts` (was `useConnector`, link only), new `hooks/useMachineControls.ts`,
  new `components/panel/MachineControls.tsx` (moved job toolbar and block), new
  `components/workbench/SymbolButton.tsx`, `ConnectorGroup.tsx` slimmed, `OutputGroup.tsx` without
  its line and bar, `PropertiesPanel.tsx` (the fixed footer), `MachineBlock.tsx`.
- `docs/design-system.md` (section 14 rows); `0064` Q14 to Q17, `0125` S13 to S15, `0127` G33
  marked as amended.

## Tasks
- [x] 1. Wire: `Home`, `Homing`, three goldens, old goldens unchanged (criterion 3; M1)
- [x] 2. Core rules: allowed and refused states with causes, Door/Check/Sleep, a kept Ready job,
        `Homed`, `HomingFailed`, alarm drops the job, Stop in both machines, loss, local start (3, 4, 5; M2, M3, M5)
- [x] 3. GRBL: `$H` once without `$X`, owed answers typed, error 5 and alarms 8/9 in words,
        loss-clock pause, Stop by reset without hold, alarm 6 (3, 4, 6; M4)
- [x] 4. Fake controller 2 s homing, Stop cancels, Home in its alarm (6; M6); simulator error 8 (M7)
- [x] 5. Hub and session: `Home: <client>` log line, terminal line, a second app sees Homing and may Stop (3; M8)
- [x] 6. App `connector_command` takes `"home"` (3; M9)
- [x] 7. Frontend store: one busy press across both places, read state (1, 5; M11)
- [x] 8. Frontend texts and rules: Homing, Home's causes, the one state line (Frage 262 A), footer notices (1, 3, 4, 5)
- [x] 9. Machine controls footer in the Operations tab; Connector group slimmed; Output group line gone (1, 2; W1 to W14)
- [x] 10. Design-system rows and amended criteria (7)

## Validation
- Core: `tests/home_0118.rs` (the M3 table, M5, goldens), `golden_messages.rs` (18 types).
- `-io`: `tests/grbl_home.rs` (simulator, stepped time), `tests/home_fake.rs` (running connector,
  fake controller, waits by status), fake controller unit tests, `jobs.log` golden.
- App: `home_maps_to_the_home_message_in_any_state_and_unknown_words_to_none`.
- Frontend: `tests/machine_control_0118.test.ts`; the `0125` and `0127` line tests now read the
  footer's line.
- Full gate of `CLAUDE.md` §7 and `cargo deny`. The browser check is the tester's.
