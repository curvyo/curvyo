# Machine control: Home, jog, frame and focus from the app, next to the job controls in the Operations tab

Status: First slice Done (PR #399); jog, frame and focus still to specify
Priority: Should
Origin: Customer request of 2026-11-17 (Run, Pause, Stop, Home, collapsible jog buttons, rectangular and rubber-band Frame, origin and start-from, focus-point mode), extended by feedback of the first laser test, 2026-11-18. Translated: "I need a homing button. And the buttons must move to Operations, they make more sense there."

## User value

As a maker standing at my laser I want Home and the job buttons in the Operations tab, where I read the job, so that I prepare, send, home and run in one place without going back to the Machine block (`R-MFG-LASER-002`, `R-MFG-LASER-003`, `R-MFG-011`).

## What changes in the demo (first slice)

- A **Machine controls** group sits in the Operations tab of a laser page whose profile names a connector, below the operation list and its summary: the state line and progress bar, the job buttons **Send job...**, **Start/Resume**, **Pause**, **Stop**, **Unlock** (`0127` G33) and the new **Home** symbol button.
- **Home** sends `$H` through the connector. It is enabled when the connector holds no running or paused job (Idle, Alarm, or Ready), disabled with its cause otherwise; during homing the state line reads "Homing" and Stop works as in `0127` G9. After Home the state reads Idle and a job that was Ready can be started.
- The Machine block keeps what describes the machine: profile, work area, origin, limits, laser mode, Connector address, Controller port, Baud rate, **Test connection**, **Pair**, **Forget pairing**, the trust block and the Controller line. The job buttons, state line and progress bar leave it.
- Try it: `curvyo-connector-server --fake-controller` (Home answers with a short fake homing), or the pty simulator.

## Changes accepted behaviour

- Where the connector's job buttons, state line and progress bar live moves from the Connector row of the Machine block (`0064` Q14 to Q17, ux-notes sections 2 to 11; `0125` S15 "Start, Pause, Resume and Stop live in the Connector row only"; `0127` G33 Unlock "in the Connector row") to the Operations tab. Reason: customer feedback of the first laser test, 2026-11-18: they make more sense next to the operations. **Frage 262 (MC-STATELINE), A decided by the customer 2026-11-19:** the Operations tab shows one state line, the footer's; the Output group's state line and bar of `0125` S13 to S15 go. Every result of a Send to machine press ("Sending <name>", "Ready: ...", "Connector busy", "Nothing answers at ...", "Timed out", "Not received", "Laser mode is off ($32=0)") is shown in the footer's line; Send to machine keeps its disabled causes in its tooltip. This amends `0125` criteria S13 (state line in the Output group), S14 (one status object, now read by one line) and S15 (job buttons in the Connector row only; now the Machine controls group). S15's rule "one place for a command that moves a machine" now names the Machine controls group of the Operations tab. Design-system row "Connector group" (`0064`) is split in two. The words, states, enabling rules and tests of the buttons are unchanged; the moves in `0064`/`0125`/`0127` criteria are rewritten by this slice, not before.

## Acceptance criteria (first slice)

1. Given a laser page whose profile names a connector, then the Operations tab shows the Machine controls group with the buttons of the Connector row, with identical names, glyphs, enabling rules, busy and failure texts (`0064` ux-notes tables); the one exception is the glyph of **Send job...**, which is `FileUp` (ux-notes W5), because Send to machine in the same tab uses `Upload`; given no connector address, the group is absent; given the browser build, it shows "Needs the desktop app".
2. Given the Machine block, then it holds no job button, state line or progress bar; Test connection, Pair, Forget pairing and the trust block are unchanged there.
3. Given a paired connector in Idle, Ready, Done or Alarm, when the maker presses **Home**, then the app sends one new command `Home` (additive, protocol stays 0, golden `command_home.json`), the connector writes `$H` to GRBL once and reports the new state `Homing` to every client at once, until GRBL answers `ok` (`adrs.md` M1 to M4); Home is never sent by the connector on its own. Given `--require-local-start`, then Home waits for the press at the connector like a Start (ADR 0070 decision 2, `adrs.md` M5). Given a Ready job, then a Home that succeeds does not discard it. Given Stop during Home, then the homing stops and GRBL's alarm 6 ("Alarm: Reset during homing (alarm 6)") follows: on the fake controller a held Ready job is kept; on GRBL the alarm drops it, as after every alarm (architect addendum, M3; ux-notes W8). *(Amended by `0142`: "until GRBL answers `ok`" gains "or the homing cap ends it as a lost controller"; the Home button's failure texts gain "Lost the controller: homing took longer than <cap> s", shown in the state line and not written to `jobs.log`. Enabling, Stop and `--require-local-start` are unchanged.)*
4. Given Home fails (`ALARM:8`/`9`, `$22=0`), then the state line shows GRBL's cause in words, with the built texts of `curvyo-connector-io/src/grbl/texts.rs` (as `0129` criterion 2): "Homing did not find the switch (alarm 9)", "Homing pull-off did not clear the switch (alarm 8)" (shown as "Alarm: ...") and, for `$22=0`, "Homing is not enabled (error 5)" and the machine is left safe; after an alarm a Ready job is dropped as after every alarm, after `error:5` it is kept (`adrs.md` M3).
5. Given a running or paused job, then Home is disabled with the cause "A job is running"; Stop stays enabled and never asks.
6. Given the fake controller and the simulator, then Home, Stop during Home, Home while a Stop is pending and Home in Alarm are covered by integration tests without wall-clock sleeps (`CLAUDE.md` §5); Home failure by the simulator and core tests (the fake gets no failure mode, `adrs.md` M6).
7. Given the gate, `docs/design-system.md` has the row "Machine controls (Operations tab)" and the old Connector-row text is amended.

## Open (to be specified before Ready, no question yet)

Jog buttons (collapsible), Frame (rectangle and rubber band), origin and start-from, focus mode, Run with the job's own start pick. Order: Home and the move first, because it is the customer's next real-laser need.

## Needed by other stories

`0132` criterion 12 (Frage 258 C) needs a **run-from-line** the connector can do after a trusted position check (homed, laser off, modal state reconstructed from the job text, explicit confirmation naming the line). It is specified with `0132`; the Machine controls group shows its button there. Not part of the first slice.

After slice `0128`, a member's Home waits for an owner's confirmation (ADR 0071 decision 7; customer decisions 244 A, 245 A), as a member's Start does. Not part of the first slice; `0128` or its follow-up builds it on the `Home` command of this slice.

## Out of scope

- Homing inside a job (`0029` criterion 38 stays; its bug is `0129`), changing GRBL settings, other controllers.

## Needed before Ready

Done: `ux-notes.md` (UX, W1 to W14) and the architect addendum in `adrs.md` (`Command::Home`, the move of the group, no new crate).

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003, R-MFG-011
Depends on: `specs/0127-connector-grbl-serial/`, `specs/0125-send-to-machine/` (Done, PR #379)
Related: `specs/0101-laser-operations/` (Operations tab), `specs/0129-fix-home-first-locked/`
PR: -
