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

# Plan for Machine control, slice 2a (jog and a member's Home)

Criteria 8 to 21 and 37 of the second slice; Fragen 308 A, 309 A, 315 A. Design: architect addendum
J1 to J9 in `adrs.md`, UX notes W16 to W27. Branch `story/machine-jog`. No new crate, dependency or
`format_version`; protocol stays 0 (additive `Jog`, `JogKeep`, `JogStop`, `State::Jogging`, three
`Status` fields). Old golden files stay byte-identical (new fields are left out when empty).
Commits (conventional, scope = crate, at most five): core, `-io` (server and ctl), app, frontend, docs.

## Affected crates/modules
- `curvyo-connector-core`: new `jog.rs` (steps, feeds, travel clamp, dead-man constant),
  `machine/jog.rs` (how the machine takes the jog messages and events, `Clock` dead-man),
  `machine/event.rs` (`Event` moved out of `machine/mod.rs`, which would pass 500 lines),
  `message.rs`, `codec.rs` (42 known types), `access.rs` (`Kind::Motion`; a member's Home passes),
  `machine/confirm.rs` and `home.rs` (a member's Home waits, `Confirmation.what`). Goldens
  `command_jog*.json`, `command_jog_keep.json`, `command_jog_stop.json`, `status_jogging.json`,
  `status_homed_positions.json`, `status_awaiting_home.json`.
- `curvyo-connector-io`: new `grbl/jog.rs` (`$J=`, the laser-off guarantee, the hold, the cancel),
  `grbl/controller.rs`, `grbl/replies.rs`, `fake_controller.rs` (position moved by jogs),
  `hub/jog.rs` (`Hub::jog`, the clock tick, the `jobs.log` line per pad session), `session.rs`
  (a closing session ends its hold), `admin_log.rs`, `hub/journal.rs` (Home confirmed).
  Simulator (test support) gains `$J=`, `0x85`, the `Jog` word, `error:15`.
- `curvyo-connector-ctl`: `confirm-start` confirms a waiting Home.
- `curvyo-app`: `connector.rs` (`"jog"`), new `connector_jog.rs` (hold, keep, release).
- `frontend`: `lib/jog.ts`, `JogPad.tsx`, `MachineControls.tsx`, `useMachineControls`,
  `connectorText.ts`, `machineControls.ts` (causes), the member's "Confirm home".
- `docs`: ADR amendments (0070, 0071, 0035 section 6), `docs/design-system.md` rows, `docs/technical-debt.md`.

## Tasks
Phase 1: core, `-io`, server, goldens, simulator
- [x] 1. Wire: `Jog`, `JogKeep`, `JogStop`, `State::Jogging`, `Status.homed`, `machine_position`,
        `work_offset`, goldens, old goldens unchanged (15, 16, 18, 19, 21; J1)
- [x] 2. `jog.rs`: steps, feeds, `TravelBox`, `clamp` with a table per travel, `$23` and edge (16, 17, 21; J2)
- [x] 3. Access table: `Kind::Motion` owner-only, `JogStop` and a member's Home pass (12, 37; J7)
- [x] 4. Core rules for both machines: Jog per state, refusals in words, queue of one step, Stop and
        `JogStop` cancel, `JogEnded`, `JogFailed`, Alarm and loss end the jog, a Ready job is kept (8, 10, 12, 19, 20; J3)
- [x] 5. Dead-man by `Event::Clock`: 299 ms keeps, 300 ms cancels (18; J4)
- [x] 6. Homed since connect and the positions in `Status` (13, 14; J5)
- [x] 7. A member's Home waits, an owner or `curvyoctl confirm-start` confirms, Stop discards, a second
        Home is `WrongState`, `admin.log` names Home (12, 37; J7)
- [x] 8. GRBL: `M5` before the first motion, `$J=` per sign, feed cap, clamp, hold increments, `0x85`
        repeated until Idle, the stop of G12 after 2 s, `error:15`, the `Jog` word is no foreign state (9, 10, 16, 17, 18; J6)
- [x] 9. Fake controller position moved by jogs in injected time (18, 21; J9)
- [x] 10. Hub: clock tick, `Hub::jog`, `jobs.log` line per pad session, session close ends a hold (11, 12, 18; J4, J7)
Phase 2: app and frontend
- [x] 11. App `connector_command` "jog", `connector_jog` hold, keep, release (15, 18, 21; J8)
- [x] 12. Frontend: `jog.ts` corner table and press reducer, `JogPad`, causes, "Jogging", "Not homed",
        work-offset notice, member without pad, "Confirm home" (15 to 20, 37; W16 to W27)
Phase 3: docs
- [x] 13. ADR amendments, design-system rows, technical debt (21)

## Validation
- Core: `tests/jog_0118.rs` (J3 table, Clock, clamp, J5, J7 rows, goldens), `access.rs` table.
- `-io`: `tests/grbl_jog.rs` (simulator bytes per sign, `M5` first, `0x85`, release, keep loss and
  the late `$J=`, no `M3`/`M4`/`S` above 0), `tests/jog_fake.rs` (running connector, fake controller).
- App: `connector_command` unit tests. Frontend: `tests/jog_0118.test.ts`.
- Full gate of `CLAUDE.md` §7, `cargo deny`, `python3 -I scripts/check_docs.py`. The browser check is the tester's.

# Plan for Machine control, slice 2b (frame)

Criteria 22 to 28 of the second slice; Fragen 308 A, 310 C, 311 A, 334 A. Design: architect addendum
F1 to F7 in `adrs.md`, UX notes W28 to W38. Branch `story/machine-frame`. No new crate, dependency or
`format_version`; protocol stays 0 (additive `State::Framing`, `RejectReason::NotAFrame`,
`Status.frame_id`). Old golden files stay byte-identical, except that the nine `curvyo-editor-wasm`
`output_view` view goldens gain an additive `frame` block (view JSON, not a file format; **(Proposal)**). Commits (conventional, scope = crate, at
most five): laser-core, connector (core, `-io`, server, ctl), app and frontend, docs.

## Affected crates/modules
- `curvyo-laser-core`: new `job_frame.rs` (outline: box and hull), `job_write_frame.rs` (the bytes; named like the other writers for the 0093 file-name rule),
  `job_write::write_frame`; goldens `frame_rectangle`, `frame_rubber_band`, `frame_head_position`,
  `frame_corners_{bl,br,tl,tr}.gcode`.
- `curvyo-connector-core`: `message.rs` (`State::Framing`, `RejectReason::NotAFrame`,
  `Status.frame_id`), new `frame_text.rs` (the lines a frame may hold, `FrameMoves::fits`),
  `machine/frame.rs` (the rules), `machine/confirm.rs` (`Confirms::Frame`, a member's frame waits),
  `machine/event.rs` (`ControllerCommand::Frame`), `access.rs` unchanged (a frame is a `Job`).
  Goldens `status_framing.json`, `status_awaiting_frame.json`, `submit_frame.json`,
  `reject_not_a_frame.json`.
- `curvyo-connector-io`: new `grbl/frame.rs` (travel check, then the stream), `fake_controller.rs`
  (runs the lines in injected time), `hub` (`frame_job()` passed, `jobs.log` `Frame: <client>`),
  `admin_log.rs` (Frame named), simulator support.
- `curvyo-connector-ctl`: `confirm-start` confirms a waiting frame.
- `curvyo-editor-wasm`, `curvyo-app`: `frame_bytes`, new `connector_frame.rs`, `connector.rs` `"frame"`.
- `frontend`: Frame row, frame pick, `connectorText.ts`, causes, head ring, "Confirm frame".
- `docs`: ADR amendments, design-system rows (done), technical debt.

## Tasks
Phase 1: laser-core, core, `-io`, simulator
- [x] 1. `job_frame.rs`: box and convex hull over burn and sweep ends, `None` without a burn (22, 23, 27; F1)
- [x] 2. `job_frame_grbl.rs` and `write_frame`: `M5`, `M9`, `G1 … S0` at the capped feed, Head position
        relative and back to the start; goldens per pick, shape and corner (24, 25, 28; F1)
- [x] 3. Wire: `State::Framing`, `Status.frame_id`, `RejectReason::NotAFrame`, goldens (22, 26; F2)
- [x] 4. `frame_text.rs`: every word a frame may hold, `fits` per corner (25, 28; F3, Frage 334)
- [x] 5. Rules: Frame accepted in Idle, Ready, Done, a Ready job kept, never listed, `Busy` elsewhere,
        refused words are `NotAFrame`, Stop, Jog, Home, Start in Framing, Alarm (22, 26, 28; F4)
- [x] 6. A member's frame waits for an owner, `Confirms::Frame`, `LocalPress` waits (37 analogue, 308 A; F5)
- [x] 7. GRBL: travel check then stream, `M5` first, no `M3`/`M4`/`S` above 0, Stop mid-frame, outside the
        travel writes nothing, `$32=0` does not refuse (25, 28; F6, Frage 334)
- [x] 8. Fake controller runs a frame in injected time; hub passes the frame, `jobs.log` line (22; F6)
Phase 2: app and frontend
- [x] 9. `frame_bytes` (wasm), `connector_frame.rs`, `"frame"` command (22, 24, 27; F2)
- [x] 9a. An operation without power or speed frames its geometry: `build_frame_job` with stand-in numbers,
        the wasm cause ignores 4 a and measures the bed on that job (27; F1 amended, Frage 335 B)
- [x] 10. Frontend: Frame row, pick, causes, "Framing", "Confirm frame", head ring (22 to 27; W28 to W38, F7)
Phase 3: docs
- [x] 11. ADR amendments, technical debt (28)

## Validation
- laser-core: `tests/frame_goldens_0118.rs` (goldens, `M5` first, no `G0`, no `S` above 0, box and hull tables).
- Core: `tests/frame_0118.rs` (F3 per word, `fits` per corner, F4 per state in both machines, F5 rows, goldens).
- `-io`: `tests/grbl_frame.rs` (simulator), `tests/frame_fake.rs` (running connector, fake controller).
- App: `connector_frame` unit tests. Frontend: `tests/frame_0118.test.ts`.
- Full gate of `CLAUDE.md` §7, `cargo deny`, `python3 -I scripts/check_docs.py`. The browser check is the tester's.
