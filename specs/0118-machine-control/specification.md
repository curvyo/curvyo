# Machine control: Home, jog, frame and focus from the app, next to the job controls in the Operations tab

Status: Draft (only the parts decided by the customer on 2026-11-18 are specified; the rest of the row is still to be written before Ready)
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

- Where the connector's job buttons, state line and progress bar live moves from the Connector row of the Machine block (`0064` Q14 to Q17, ux-notes sections 2 to 11; `0125` S15 "Start, Pause, Resume and Stop live in the Connector row only"; `0127` G33 Unlock "in the Connector row") to the Operations tab. Reason: customer feedback of the first laser test, 2026-11-18: they make more sense next to the operations. The Output group's state line of `0125` S13 stays, and S15's rule "one place for a command that moves a machine" now names the Machine controls group of the Operations tab. Design-system row "Connector group" (`0064`) is split in two. The words, states, enabling rules and tests of the buttons are unchanged; the moves in `0064`/`0125`/`0127` criteria are rewritten by this slice, not before.

## Acceptance criteria (first slice)

1. Given a laser page whose profile names a connector, then the Operations tab shows the Machine controls group with the buttons of the Connector row, with identical names, glyphs, enabling rules, busy and failure texts (`0064` ux-notes tables); given no connector address, the group is absent; given the browser build, it shows "Needs the desktop app".
2. Given the Machine block, then it holds no job button, state line or progress bar; Test connection, Pair, Forget pairing and the trust block are unchanged there.
3. Given a paired connector in Idle or Alarm, when the maker presses **Home**, then the app sends one new command `Home` (additive, protocol stays 0, golden `command_home.json`), the connector writes `$H` to GRBL once and answers `Status` "Homing" until GRBL answers `ok`; Home is never sent by the connector on its own. Given a Ready job, then Home does not discard it.
4. Given Home fails (`ALARM:8`/`9`, `$22=0`), then the state line shows GRBL's cause in words (as `0129` criterion 2) and the machine is left safe.
5. Given a running or paused job, then Home is disabled with the cause "A job is running"; Stop stays enabled and never asks.
6. Given the fake controller and the simulator, then Home, Home during Stop, Home in Alarm and Home failure are covered by integration tests without wall-clock sleeps (`CLAUDE.md` §5).
7. Given the gate, `docs/design-system.md` has the row "Machine controls (Operations tab)" and the old Connector-row text is amended.

## Open (to be specified before Ready, no question yet)

Jog buttons (collapsible), Frame (rectangle and rubber band), origin and start-from, focus mode, Run with the job's own start pick. Order: Home and the move first, because it is the customer's next real-laser need.

## Needed by other stories

`0132` criterion 12 (Frage 258 C) needs a **run-from-line** the connector can do after a trusted position check (homed, laser off, modal state reconstructed from the job text, explicit confirmation naming the line). It is specified with `0132`; the Machine controls group shows its button there. Not part of the first slice.

## Out of scope

- Homing inside a job (`0029` criterion 38 stays; its bug is `0129`), changing GRBL settings, other controllers.

## Needed before Ready

`ux-engineer`: ux-notes for the group (placement in the 244 px column, keyboard, Home glyph and states). `architect`: addendum for `Command::Home` and the move of the group (no new crate expected).

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003, R-MFG-011
Depends on: `specs/0127-connector-grbl-serial/`, `specs/0125-send-to-machine/` (Done, PR #379)
Related: `specs/0101-laser-operations/` (Operations tab), `specs/0129-fix-home-first-locked/`
PR: -
