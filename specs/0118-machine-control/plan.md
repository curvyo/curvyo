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

# Plan for Machine control, slice 2c (origin and start-from)

Criteria 29 to 33; Fragen 312 A, 341 MC-ORIGIN-WPOS A, 345 MC-ORIGIN-MACHINE C (defaults). Design:
architect addendum O1 to O10 in `adrs.md`, UX notes W39 to W52. Branch `story/machine-origin`. No new
crate, dependency or wire change; `format_version` 61 (`main` is at 60: 59 is `0150` (#468), 60 is `0145` slice 3 (#475)). Connector crates and `curvyo-app` stay untouched.
Commits (conventional, scope = crate, at most five): docs, document-core, laser-core, ui-core and
editor-wasm, frontend.

## Affected crates/modules
- `curvyo-document-core`: `laser_output_commands.rs` (`user_origin` spelling, `set_user_origin`,
  `NoUserOrigin`, `BadUserOrigin`), `manufacturing.rs` (`write_profile`: a Pick of another record
  resets Start User origin), `history/rows.rs`, `document.rs` (format 61); goldens
  `user_origin_v61.curvyo`, `user_origin_damaged_v61.curvyo`.
- `curvyo-laser-core`: `output_settings.rs` (`Start::UserOrigin`, `user_origin`), new `job_anchor.rs`,
  `job_moves.rs` (`Job.shift`, `Job::machine_point`, run-out clamp at the placed position),
  `job_build.rs` (the shift), writers `job_write_grbl.rs`, `job_write_frame.rs`, `job_frame.rs`,
  `job_gate.rs` and `job_gate_text.rs` (`OutsideBed.over_um`); goldens
  `output/grbl/start_user_origin_{bl,br,tl,tr}.gcode`, `grbl_frame/frame_user_origin.gcode`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (labels).
- `curvyo-editor-wasm`: `session/output/commands.rs` (`set_user_origin`), `input.rs` (the point into
  `JobInput`), `view_json.rs` (pick lists User origin only when stored, the X, Y row), `toolpath.rs`
  and `raster_view.rs` (drawn at the shift, items at 35 %), the cross and the dashed box overlay,
  `print_and_cut.rs` (the warn).
- `frontend`: `JogPad.tsx` (Set origin here in the centre cell), `machineControls.ts` (its causes),
  `OutputGroup.tsx` and `outputView.ts` (the pick's fourth value, the User origin row).

## Tasks
Phase 1: document-core
- [x] 1. `set_user_origin` writes the point and Start in one step, undo restores both, range
        refused, `set_start("user_origin")` without a point refused (29, 30; O1, O2)
- [x] 2. Format 61, goldens `user_origin_v61`, `user_origin_damaged_v61` (29; O3)
- [x] 3. A profile Pick of another record resets Start User origin to Machine origin in the same
        step, the point stays; the same record and every edit keep it (Frage 345 C; O9)
Phase 2: laser-core
- [x] 4. `Start::UserOrigin` and the point read, damaged shapes counted, User origin without a point
        is `start_damaged` (30; O1, O4)
- [x] 5. `job_anchor.rs`: burn box and the anchor per corner, shared with the Head position bed
        offset (31; O5)
- [x] 6. `Job.shift`, `Job::machine_point`, writers and frame outline; per corner the burn lines are
        Machine origin's plus `U - A`; goldens per corner and the frame (31; O5, O7)
- [x] 7. Bed check measures the placed job, `over_um` and "… bed by 12 mm." (31; O7, W50)
Phase 3: ui-core, editor-wasm
- [x] 8. Labels "Set origin here", "Start: User origin" (29, 30; O2)
- [x] 9. `set_user_origin(x_mm, y_mm)`, the pick lists User origin only when stored, the X, Y row,
        print and cut warns for User origin (29, 30; O4, O6, W42, W43)
- [x] 10. Toolpath at the shift, items at 35 %, the cross and the dashed box in the view (31, 32;
        O7, W46 to W48)
Phase 4: frontend
- [x] 11. Set origin here (W39 to W41, W51), its causes; the pick and the User origin row (W42, W43)

## Validation
- document-core: `tests/user_origin_format.rs` (goldens, undo, refusals, profile pick rule).
- laser-core: `tests/user_origin_0118.rs` (per corner: burn lines equal Machine origin's plus the
  offset, goldens, frame, bed check), unit tests in `job_anchor.rs`, `output_settings.rs`.
- editor-wasm: `session/output/tests/user_origin_tests.rs`. Frontend: `tests/set_origin_0118.test.ts`.
- Full gate of `CLAUDE.md` §7, `cargo deny`, `python3 -I scripts/check_docs.py`. The browser check is the
  tester's.

# Plan for Machine control, slice 2d (focus point)

Criteria 10, 12, 34 to 36 with 35 a to e; Fragen 369 B (pulse off in a new profile), 370 A, 371 A,
380 A (defaults). Design: architect addendum P1 to P12 in `adrs.md`, UX notes W53 to W64. Branch
`story/machine-focus`, stacked on `story/machine-origin` (#477). No new crate or dependency; protocol
0 stays (additive `GoToPoint`, `Focusing`, `Pulsing`, `Status.target`); profile record format 8 (7 is
`0150`'s, #468) and `format_version` 62 (61 is slice 2c, #477).
Commits (conventional, scope = crate, at most five with the docs commit): docs, connector-core and
-io, laser-core and document-core, editor-wasm and app, frontend.

## Affected crates/modules
- `curvyo-connector-core`: new `focus.rs` (`PulsePower`, `PulseLength`, `Pulse`, `clamp_target`,
  the target bound), `message.rs` (`GoToPoint`, `Focusing`, `Pulsing`, `Status.target`),
  `codec.rs` (43 types), `access.rs` (`GoToPoint` is `Motion`), new `machine/focus.rs` (P3, P6),
  `machine/event.rs` (`GoTo`, `Pulse`, `GoToTarget`, `PulseEnded`, `PulseFailed`, `PulseSkipped`),
  the state lists of `home.rs`, `jog.rs`, `arrival.rs`, `controls.rs`, `confirm.rs`. Goldens
  `command_go_to_point*.json`, `status_focusing.json`, `status_pulsing.json`,
  `status_awaiting_point.json`.
- `curvyo-connector-io`: new `grbl/focus.rs` (the `$J=G90` move, the pulse block, `Owed::Pulse`),
  `grbl/controller.rs`, `grbl/replies.rs` (answers, Hold or Door mid-pulse, the stop), `fake_jog.rs`
  and `fake_controller.rs` (move and dwell in injected time), `hub/motion.rs` (`go_to_point`, the two
  `jobs.log` lines), `hub/link.rs` (the clock while pulsing), `hub.rs` (shutdown Stop),
  `session.rs`, `job_log.rs`; simulator `$J=G90`, `G1`, `M3` at rest, `G4 P` seconds. Golden
  `tests/fixtures/grbl/focus_pulse.gcode`.
- `curvyo-laser-core`: `machine_profile.rs` (`FocusPulse`, `PulsePermille`, `PulseMs`),
  `machine_profile_file.rs` (record format 8), `machine_profile_copy.rs` (three keys, damage flag).
- `curvyo-document-core`: `manufacturing.rs` (`ProfileChange` variants), `history/rows.rs`,
  `document.rs` (format 62); goldens `focus_pulse_v62.curvyo`, `focus_pulse_damaged_v62.curvyo`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs`.
- `curvyo-editor-wasm`: `session/output/focus.rs` (`focus_target`, `focus_ring`), profile setters,
  `machine_view.rs` (three fields), the range test against connector-core.
- `curvyo-app`: `connector.rs` (`"go_to_point"` with `{ x, y, pulse? }`).
- `frontend`: `JogPad.tsx` (Focus point toggle), `lib/focusPoint.ts` (causes, mode reducer, the
  pulse from the copy), the canvas press and the two rings, `MachineBlock.tsx` (three fields).

## Tasks
Phase 1: connector-core
- [x] 1. `focus.rs`: bounded `PulsePower`, `PulseLength` refuse 0, 51, 49, 2,001 on decode; the
        target within 10,000 mm; `clamp_target` per `$23` and box, no box unclamped (35 c; P2, P4)
- [x] 2. `GoToPoint`, `Focusing`, `Pulsing`, `Status.target`, goldens, access row (12, 34; P2, P11)
- [x] 3. Machine: P3 per state in both machines (Not homed, Alarm, busy words, Ready kept), arrival
        at 0.049 / 0.051 mm, no pulse with laser off or unknown, a pending stop or a busy word,
        `PulseEnded`, the deadline by `Clock`, Stop in each state, `Waiting::Point` with Enter and
        Stop (10, 12, 34, 35, 35 c; P3, P6)
Phase 2: connector-io
- [x] 4. GRBL: `M5`/`M9` first, `$J=G90 G21`, the block in one write, `S` per `$30` (1000, 255, the
        `s = 0` skip), `$32=0` no block, Stop mid-move (`0x85`, no block) and mid-pulse (reset, `M5`,
        `M9`, no alarm, homed kept), Hold/Door mid-pulse, `error:15`, golden (9, 10, 35 b, 35 c; P5, P6)
- [x] 5. Fake controller, hub `go_to_point`, both `jobs.log` lines, the clock while pulsing, the
        shutdown Stop, the session (12, 35 c; P9, P10)
Phase 3: laser-core, document-core, ui-core
- [x] 6. `FocusPulse` fields, record format 8, copy keys, damaged values read the default and count
        (35 a; P7)
- [x] 7. Format 62, `ProfileChange` for the three fields, labels, goldens (35 a; P8)
Phase 4: editor-wasm, app
- [x] 8. `focus_target` per corner and start pick (table), Head position and Nothing to place
        causes, `focus_ring` its inverse, the profile setters and view fields, the range test
        (34, 35 a, 36; P1, P7)
- [x] 9. `"go_to_point"` command (34; P10)
Phase 5: frontend
- [x] 10. Toggle, causes, mode reducer, canvas press, target and head rings, the three fields
        (34, 35 a, 35 d; W53 to W64)
Phase 6: docs
- [x] 11. ADR amendments (P2 to P11 onto ADR 0035, 0038, 0039, 0071), `specs/README.md` row
Phase 7: tester findings
- [x] 12. The pulse deadline counts from the pulse's start; Stop mid-pulse holds before the reset (a
        Hold or Door report starts at the reset); the target is clamped in core before `Focusing`
        (`Event::Travel`); the late pulse's sentence shows while the stop waits; the focus lines of
        `jobs.log` in their own golden `jobs_focus.log`, `jobs_v2.log` back to its bytes (10, 34, 35 c)

## Validation
- Core: `tests/focus_0118.rs` (P12 core rows, goldens); unit tests in `focus.rs`.
- `-io`: `tests/grbl_focus.rs` (simulator), `tests/focus_hub.rs` (fake, jobs.log, a second client).
- laser-core: unit tests in `machine_profile_file.rs`, `machine_profile_copy.rs`; record goldens.
- document-core: `tests/focus_pulse_format.rs`. editor-wasm: `session/output/tests/focus_tests.rs`.
- Frontend: `tests/focus_point_0118.test.ts`.
- Full gate of `CLAUDE.md` §7, `cargo deny`, `python3 -I scripts/check_docs.py`. The browser check is
  the tester's.
