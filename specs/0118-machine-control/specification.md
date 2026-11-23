# Machine control: Home, jog, frame and focus from the app, next to the job controls in the Operations tab

Status: First slice Done (PR #399); second slice (jog, frame, origin and start-from, focus point, a member's Home, run-from-line with `0132` slice 2) Draft, specified 2026-11-23, needs the customer's answers to Fragen 308 to 315, then `adrs.md` and `ux-notes.md` addenda before Ready | Slice 2a (jog, member Home waits) Done (PR #453); slice 2b (frame) Done (PR #460; Frage 334 MC-FRAME-TRAVEL A, customer 2026-11-23); slices 2c to 2e still to build
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

## Second slice: jog, frame, origin and start-from, focus point, a member's Home, run-from-line

Specified 2026-11-23 from the customer's wishes in Origin (collapsible jog buttons; rectangular and rubber-band Frame; origin and start-from; focus-point mode; Run with the job's own start pick), `0132` criteria 12 and 13 (Frage 258 C), ADR 0071 decision 7 (Fragen 244 A, 245 A) and `0140` (Fragen 282 C, 283 A, 284 A). Everything not quoted from a decided Frage or an accepted criterion is a **(Proposal)**, vetoable at the demo. The customer's decisions are Fragen 308 to 315 at the end of this section; the criteria are written with their defaults.

### What other tools do

LightBurn's Move window: a jog pad with a step distance and a speed, continuous jog while a button is held, "Go to origin", "Set Origin" (GRBL user origin through `G10 L20`/`G92`), Frame and "Rubber Band Outline" frame in the Laser window, Start From: Absolute Coords, User Origin, Current Position with a nine-point Job Origin, and a "Fire" button for the dot. LaserGRBL: a jog pad with a step slider, a frame button, "set zero" (`G92`). Universal G-code Sender: jog with step and feed, continuous jog with jog cancel (`0x85`). Pain points we answer: a continuous jog that keeps running when the app or the network drops (we stop on a missing keep-alive); a "Set Origin" that writes a hidden offset into the controller's EEPROM and shifts the next job of another program (we keep the origin on the page, Frage 312 A); a frame that does not match the job's start (we frame with the job's own start pick); jog and frame from a second person's phone while someone stands at the laser (owner rule, Frage 308).

### One-line meanings (CLAUDE.md §3: the customer confirms each before it is built)

- **Motion command:** any command that moves the head outside a job: Home, Jog, Frame, Go to point (focus point) and Run from line. The rules of criteria 8 to 12 hold for all of them.
- **Jog:** moves the head by one step in X or Y with the laser off; held, it moves until released (Frage 309).
- **Jog step:** the distance of one jog press: 0.1, 1, 10 or 50 mm (Proposal).
- **Frame:** the head traces the outline of the job, as the job will be placed by its own start pick, with the laser off (Frage 310), once.
- **Rectangular frame:** the outline is the job's bounding box.
- **Rubber-band frame:** the outline is the convex hull of the job's burn moves, the shape a rubber band around the job would take (Frage 311 A).
- **Machine origin:** the corner the machine homes to, the origin of the page's coordinates since `0029` criterion 26.
- **User origin:** one point on the bed, stored on the page in machine millimetres, that the job's anchor is placed on (Frage 312 A).
- **Set origin here:** stores the head's current machine position as the page's user origin.
- **Start pick:** the page's one pick "Start" (`0029` criterion 38) that says where the job starts: Machine origin, Head position, Home then machine origin, and now User origin.
- **Focus point:** a mode in which a press on the canvas moves the head, laser off, to that point of the job, so the maker sets the lens height there (Frage 313 A).
- **Run from line:** the connector starts an interrupted job at line n after it has proved the position, rebuilt the machine state of line n from the job text and the owner has confirmed (`0132` criterion 12).
- **Homed since connect:** the connector has seen a Home end in `Homed` since it last identified this controller, with no reset, alarm or loss after it.

### What changes in the demo (second slice, per sub-slice)

- **2a Jog:** the Machine controls toolbar (Operations tab) gains a **Jog** toggle after Home; it opens a jog pad above the toolbar (X-, X+, Y-, Y+, the step pick 0.1 / 1 / 10 / 50 mm). A press moves the head one step with the laser off, holding moves it until release. The pad is closed by default and stays as the maker left it. A member's Home (`0128`) now waits for an owner's confirmation instead of being refused.
- **2b Frame:** a **Frame** button (one press, the head traces the job's bounding box with the laser off, then returns to where it started) and a pick beside it "Rectangle / Rubber band". While the frame runs, the canvas draws the head's position on the job.
- **2c Origin and start-from:** a **Set origin here** button in the jog pad; the Start pick gains **User origin**; the job, its frame and the Toolpath view follow the pick; the work-area check measures the placed job.
- **2d Focus point:** a **Focus point** toggle in the jog pad; while it is on, a press on the canvas moves the head (laser off) to that point.
- **2e Run from line** (with `0132` slice 2): an interrupted job in the job list offers **Restart from line <n>** to an owner, under the rules a to d of `0132` criterion 12.
- Try each on the real GRBL laser with `curvyo-connector-server --serial /dev/ttyUSB0`, and without a laser with `--fake-controller` or the pty simulator.

### Slice plan (each its own PR, each testable end to end on the real GRBL laser; small first)

1. **2a Jog** (criteria 8 to 21, 37): the motion rules, homed-since-connect, machine position and work offset in `Status`, the jog pad, a member's Home waiting. Needs nothing new from other stories.
2. **2b Frame** (criteria 22 to 28): needs 2a (motion rules, laser-off guarantee). Uses `JobKind::Frame` of the protocol.
3. **2c Origin and start-from** (criteria 29 to 33): needs 2a (Set origin here reads the machine position) and 2b (the frame follows the pick). Page format change.
4. **2d Focus point** (criteria 34 to 36): needs 2a and 2c (the canvas-to-machine mapping of the start pick).
5. **2e Run from line and `0132` slice 2** (criteria 38, 39 here, `0132` criteria 12 and 13): needs 2a (homed since connect, machine position and work offset stored at the interruption, the M5 rule) and the Machine controls group; nothing of 2b to 2d.

### Motion rules (all motion commands; built in 2a, extended by each later sub-slice)

8. Given the connector in `Idle`, `Ready` or `Done`, then a motion command is accepted; given `Receiving`, `Running`, `Paused`, `AwaitingLocalStart`, `Homing` (another motion), `NoController`, or GRBL reporting `Door`, `Check` or `Sleep`, then it is refused with `Error { WrongState }` and the cause in words ("A job is running", "Homing", "Controller state: Door", as criterion 5 and ux-notes section 4), no byte reaches the port, and in the app the control is greyed with that cause; given `Alarm`, Jog, Frame and Go to point are refused with "Alarm" (GRBL refuses motion while locked, `error:9`), Home stays allowed (criterion 3). A Ready job is never discarded by a motion command that succeeds (as criterion 3 for Home).
9. Given any motion command other than Run from line, then the laser is off for its whole motion: the connector writes `M5` (and `M9`) before its first motion byte unless `M5` is the last spindle word it wrote since the last job line, frame line or preamble; no line of a motion command carries `M3`, `M4` or an `S` above 0. Test: the simulator's byte log for jog, frame and go to point holds no `M3`, `M4` and no `S` other than `S0`.
10. Given Stop during any motion command, then it ends that motion at once and never asks (`0064` section 3): a jog with the jog cancel (`0x85`), position kept, no alarm; a frame or a go to point with the sequence of `0127` G12 (hold, reset, banner, `M5`, `M9`), the state returns to the one it came from (`Idle`, `Ready` with the job kept, or `Done`); a Home as criterion 3. Stop stays enabled during every motion command.
11. Given a motion command, then it is never sent by the app or the connector on its own: not on opening the tab, not on reconnect, not after a job, not from a key the maker did not press on a focused control (`0064` section 3: no global chord; no arrow-key jog in this slice).
12. Given configured mode (`0128`) (Frage 308 A): an **owner's** Home, Frame and Go to point run at once; a **member's** Home, Frame or Go to point goes to `AwaitingLocalStart` with `requested_by` and no job and runs when an owner confirms it (ADR 0071 decision 7; Fragen 244 A, 245 A); an owner's press of the same command, or `curvyoctl confirm-start`, confirms; Stop from anyone discards a waiting motion; a member's **Jog** is refused with `Error { OwnerRequired }` and the jog pad is absent in a member's app (a confirmed jog step is meaningless); Run from line is owner-only (`0132` criterion 12). Given foreground mode, every paired client acts as owner (`0132` adrs L4); given `--require-local-start`, Home, Frame and Go to point wait for Enter as a start does (ADR 0070 decision 2, `adrs.md` M5), Jog does not (Frage 308 A). Every motion command writes one `jobs.log` line "<Command>: <client name>" (as `Home`, `adrs.md` M8); a jog writes one line per pad session, not per step (Proposal).
13. Given the connector, then it tracks **homed since connect**: set by `Event::Homed`, cleared by a reset it did not ask for, a power loss, a banner, an alarm, a loss of the controller and a reconnect; it is in `Status` (additive, golden) and drives criteria 17, 29 and `0132` criterion 12 a.
14. Given a controller report, then `Status` carries, besides the work position of `0127` G4, the **machine position** and the **work coordinate offset** (GRBL `MPos`/`WPos` and `WCO`, additive fields, golden files); given a non-zero work offset, the settings warning of ux-notes W9 reads "Work offset on the controller: X <x>, Y <y>" (Notice look; Start stays enabled; Curvyo writes no `G10`, `G92` or `G54`, Frage 312 A).

### Jog (2a)

15. Given a paired owner's (or foreground) app and the Machine controls footer, then the toolbar has a **Jog** toggle (symbol only, name and tooltip "Jog", `aria-expanded`) in Home's group (ux-notes W3); pressed, it opens the jog pad above the toolbar (the toolbar does not move, ux-notes W2): four buttons X-, X+, Y-, Y+ in a cross, and a step pick 0.1 / 1 / 10 / 50 mm (default 1 mm). Closed by default; open or closed and the step are remembered per app (not in the project file, not synced; Proposal). The pad's look, the arrows' direction on screen for each origin corner and the footer height are UX's (ux-notes addendum).
16. Given the pad open and a press on X+ (pointer up within 300 ms, or Enter/Space on the focused button), then the connector writes one GRBL jog `$J=G91 G21 X<step> F<feed>` and the head moves one step; the jog feed is 3,000 mm/min for 10 and 50 mm, 1,000 mm/min for 1 mm and 300 mm/min for 0.1 mm, capped at the controller's `$110`/`$111` (Proposal, constants in core). The direction on the bed matches the arrow on screen for every origin corner of the profile (`0029` criterion 26). Test: simulator byte log per corner.
17. Given homed since connect and the travel `$130`/`$131` (`0131`), then the connector clamps each jog so the target stays within the travel box in machine coordinates (sign by `$23`, architect), a jog that would leave it moves to the edge, and a jog at the edge writes nothing; given not homed (Proposal: many diode lasers have no switches), then the jog is not clamped (no position is trusted), GRBL's own soft and hard limits apply, and the pad shows the state "Not homed" (a state label, no tip). GRBL's `error:15` ("Jog beyond the travel") is shown in the state line as built.
18. Given hold-to-move (Frage 309 A): a press held longer than 300 ms on an arrow moves the head continuously in that direction until release; the app sends a keep-alive at least every 100 ms while held, and the connector cancels the jog (`0x85`) when no keep-alive arrives for 300 ms (constant, injected time) or the app's connection closes; continuous jog uses the step's feed and is clamped as criterion 17. Release ends the motion within 100 ms of the release on the fake controller (thread CPU time, no sleeps). Given Frage 309 B, only steps exist and a held button repeats nothing.
19. Given a jog runs, then the state line reads "Jogging" (neutral), Home, Send job..., Start and Frame are greyed with the cause "Jogging", Stop is enabled (criterion 10); a new jog press during a step queues at most one step (GRBL's planner takes jogs one after another; more presses are dropped, Proposal).
20. Given a job runs, is paused, waits or is received, then the jog pad's buttons are greyed with the cause (criterion 8); the pad stays open; nothing is written.
21. Given the tests, then core (jog accepted and refused per state, clamp table per corner and travel, keep-alive timeout with injected time), `-io` with the simulator (`$J=` bytes, `0x85` on Stop, release and keep-alive loss, `error:15`, no `M3`/`M4`), the app (`connector_command` "jog"), and the frontend (`jobButtons` gains Jog and its causes) cover 2a without wall-clock sleeps.

### Frame (2b)

22. Given a paired app, a laser page with a job and the connector in `Idle`, `Ready` or `Done`, when the maker presses **Frame** (symbol button in its own Frame row above the toolbar, name and tooltip "Frame"; ux-notes W28), then the app builds a frame job (`JobKind::Frame`) from the page's job as it will run, with the page's own start pick (criterion 24), and sends it; the connector runs it once and returns to the state it came from; a Ready job stays held. A frame is never listed in the job list, never counts as a job result, never writes the job's `jobs.log` result line (`0132` adrs L2; this amends L2's "a frame with a current job stays `Busy`": a Ready current job does not block a frame, a running one does).
23. Given the frame pick beside Frame (two values, **Rectangle** default and **Rubber band**, Frage 311 A; remembered per page as session state, not saved, Proposal): Rectangle traces the bounding box of every burn move of the job; Rubber band traces their convex hull, starting at the hull point nearest the machine origin. Both ignore travel moves and include fixture jobs that run (`0103`).
24. Given the page's Start pick (`0029` criterion 38), then the frame matches where the job will burn: Machine origin and Home then machine origin: the outline in absolute coordinates (the frame does not home; Proposal); Head position: in relative moves from where the head stands, the head being the job's first point, and back to that point at the end, so a Start after the frame begins where the frame began; User origin: as criterion 31.
25. Given a frame, then it runs with the laser off (Frage 310 A): `M5`, then `G1 … S0` moves (never `G0`, which ignores the feed) at the frame feed 3,000 mm/min capped at `$110`/`$111` (Proposal); no line sets `S` above 0, no `M3`/`M4` (criterion 9). Given an outline that leaves the work area, then Frame is greyed with the cause "Outside the work area" (the gate of `0029` criterion 4 b; with Head position the bed is not checked, as `0029` criterion 38).
26. Given the frame runs, then the state line reads "Framing", the canvas of that page shows the head's position on the job (the ring of `0029` criterion 37, fed by `Status.position`, Proposal), Stop ends it (criterion 10); the Alt+Arrow nudge of `0140` criterion 5 is not part of this slice (it needs the `0140` loop).
27. Given a page without output (no item in a laser operation, an empty page or only items that are not assigned) or the browser build, then Frame is greyed with the cause "Nothing to frame" or absent ("Needs the desktop app"), as the other buttons (ux-notes section 8). Given an operation with items but no power or speed (Export blocked by `0029` 4 a), then Frame still traces its geometry, the outline it will have once its numbers are set (Frage 335 B); the frame feed and `S0` of criterion 25 do not depend on the operation's numbers.
28. Given the tests, then core (frame accepted with Ready, refused with Running; never listed), `curvyo-laser-core` (frame text per start pick and per corner, rectangle and hull, golden files `frame_rectangle.gcode`, `frame_rubber_band.gcode`, `frame_head_position.gcode` and the four corners `frame_corners_bl.gcode`, `frame_corners_br.gcode`, `frame_corners_tl.gcode`, `frame_corners_tr.gcode`), the simulator (no `S` above 0, Stop mid-frame returns to Ready with the job kept) cover 2b.

### Origin and start-from (2c)

29. Given the jog pad and homed since connect, when the maker presses **Set origin here** (symbol button in the pad, name and tooltip "Set origin here"), then the page stores the head's machine position (criterion 14) as its **user origin** (X, Y in mm, one undo step, in the page's laser data; `format_version` change by the architect), the Start pick switches to User origin, and nothing is written to the controller (Frage 312 A). Not homed: the button is greyed with the cause "Not homed" (machine position not trusted). The origin is also a typed X, Y pair in the Output group under the Start pick while User origin is chosen (Proposal).
30. Given the Start pick (`0029` criterion 38), then it has four values: Machine origin (default), Head position, Home then machine origin, **User origin** (shown only once a user origin is stored, Proposal). Absolute vs current: Machine origin and User origin run in absolute coordinates; Head position runs relative to where the head stands (`G91`, as built).
31. Given User origin, then the job's coordinates are the machine-origin coordinates of `0029` criterion 26 shifted so the job's anchor sits on the user origin; the anchor is the job box's corner at the profile's origin corner (bottom-left for a bottom-left machine; the nine-position anchor of `0140` criterion 11 replaces it when `0140` is built, Frage 283 A); the file writes no `G10`, `G92` or `G54` (as criterion 26); the Toolpath view draws the job at that place on the bed, the work-area check (`0029` criterion 4 b) measures the shifted job and blocks when it leaves the bed ("Outside the work area by 12 mm", `0140` criterion 12 wording), and Frame (criterion 24) traces the shifted outline. Test: golden `start_user_origin.gcode`, one per origin corner, coordinates equal machine-origin coordinates plus the offset.
32. Given the page's origin corner (`0029` criterion 26, the profile's origin corner): it stays the machine origin and the origin of the page's rulers (`0015`, `0102`); a user origin never changes the page's coordinates or the drawing, only where the job burns; the canvas shows the user origin as a small cross on the work area while User origin is chosen (Proposal).
33. Given `0140` built later, then the user origin is the first entry of the page's reference points (`0140` criterion 9, Frage 282 C) and "Set origin here" is its Store position for the page list; this slice stores exactly one point and writes nothing to the profile.

### Focus point (2d)

34. Given the jog pad, then it has a **Focus point** toggle (symbol only, `aria-pressed`, name and tooltip "Focus point"); on, a press on the canvas of the laser page moves the head with the laser off (criterion 9) to that page point as the job's start pick places it (Machine origin and User origin; with Head position or without a homed machine the toggle is greyed with the cause "Start is Head position" or "Not homed"), at the 10 mm jog feed, clamped to the travel (criterion 17); the canvas marks the target with the ring of criterion 26. The press selects nothing and is not a History step; Escape or the toggle ends the mode; switching tab or page ends it (Proposal).
35. Given the head reached the point, then it stays there with the laser off (no pulse, no "Fire", Frage 313 A); the state line returns to the state it came from; a second press moves again.
36. Given the tests, then the mapping canvas point to machine point per origin corner and start pick (core, table), the go to point bytes and the clamp (simulator), the mode's enabling causes (frontend) cover 2d.

### A member's Home (2a; `0128` follow-up)

37. Given configured mode and a **member**, when it presses Home, then the connector no longer answers `OwnerRequired` (as built in `curvyo-connector-core/src/access.rs`, marked Proposal there) but goes to `AwaitingLocalStart` with `requested_by` and no job (`adrs.md` M10, ADR 0071 decision 7, Fragen 244 A, 245 A); the member's state line reads "Waiting for <owner> to confirm" and the owner's app shows **Confirm** in the Home slot (texts of `0128` ux-notes section 8.2, UX addendum names the Home variant); an owner's Home confirms, `curvyoctl confirm-start` confirms, Stop discards; `admin.log` records "start confirmed" with the command Home. Test: the `access.rs` table row "Home" changes from `OwnerRequired` to "waits"; core table: member Home, owner confirm, Stop discards, a second member Home while one waits is `WrongState`.

### Run from line (2e, the primitive `0132` criterion 12 needs)

38. Given an interrupted job in the job list (`0132` criterion 6) and an owner, when the owner confirms **Restart from line <n>** (`0132` criterion 12 b), then the connector checks rules a to d of `0132` criterion 12 in this order and refuses with the first cause that fails ("Home first", "Laser mode is off ($32=0)", "Cannot rebuild the state before line <n>: <word>"); accepted, it writes: `M5`, `M9`; the rebuilt modal state of line n (units, plane, distance mode, feed, coolant, motion mode; `S` held, `M3`/`M4` held back); a travel to the start point of line n in `G90` at the 10 mm jog feed with the laser off; then `M3`/`M4` with the stored `S`; then line n and the rest, streamed as a job (`0127` G5 to G12), the list entry becoming the current job at line n. The rebuild is a pure function in `curvyo-connector-core` over the job text (no clock, no I/O; table test of `0132` criterion 12). Only for interrupted jobs (Frage 314 A). For a job written with Head position (`G91` at the top), the restart is refused ("Starts at the head position") because its position is not reconstructible (Proposal).
39. Given the stored interruption (`0132` adrs L6), then it carries the line, the machine position and the work offset at the interruption (criterion 14) and the homed-since-connect flag (criterion 13); rule a is "homed since connect after the interruption" or "the same work offset and the same machine position within 0.01 mm"; the line is the last line GRBL acknowledged (`ok`), not the last one written; the approximation of L6 (at most once a second) is replaced for the stored line by "the last acknowledged line at the moment of the interruption" when the connector still runs (Stop, alarm, loss), and stays approximate after a crash (the restart then names that line and the owner decides; Proposal, architect).

`0132` slice 2 is `0132` criteria 12 and 13 on top of criteria 38 and 39 here; it needs from `0118`: homed since connect (13), machine position and work offset in `Status` and in the stored interruption (14, 39), the laser-off rule (9), the run-from-line primitive (38) and the Machine controls footer and job list (`0132` ux-notes). The armed confirm "Restart <name> from line <n>?" of `0132` criterion 12 b is UX's (`0132` ux-notes addendum).

### Changes accepted or built behaviour (second slice)

- A member's Home: built as `OwnerRequired` (`access.rs`, Proposal) becomes "waits for an owner" (criterion 37). Reason: ADR 0071 decision 7, Fragen 244 A, 245 A, `adrs.md` M10.
- `0132` adrs L2: "`JobKind::Frame` with a current job stays `Busy`" becomes "with a running, paused or receiving job" (a Ready job does not block a frame, criterion 22). Reason: frame before start is the point of a frame.
- `0029` criterion 38: the Start pick gains User origin (criterion 30); `0029` criterion 26 is unchanged for Machine origin. Reason: customer wish "origin and start-from".
- `0140` criterion 5's "rubber-band frame of `0118`" now names criterion 23; `0140` "Out of scope: convex hull" is taken by this slice (Rubber band, Frage 311 A).

### Out of scope (second slice)

- Z jog, auto-focus, a focus test pattern, a low-power pulse or "Fire" (Frage 313), rotary, A axis.
- Keyboard jog (arrow keys, numpad), a gamepad or pendant.
- The frame loop, fine nudging and reference-point lists (`0140`), a frame at low power (Frage 310 B/C, with `0140` criterion 2's frame power).
- Writing `G10`, `G92`, `G54` or any `$` setting to the controller (`0127` G5).
- Restart from a line of a job that was not interrupted (Frage 314), resume after a crash without the checks of `0132` criterion 12.

### Questions for the customer (second slice)

Frage 308 (MC-MOTION-ROLES): Who may move the head outside a job, and does it wait for a confirmation?
- **A** An owner jogs, frames and goes to a point at once; a member's Home, Frame and Focus point wait for an owner's confirmation (as a member's Start, ADR 0071 decision 7); a member cannot jog at all (the jog pad is absent for members, because confirming every jog step makes no sense); under `--require-local-start` Home, Frame and Focus point wait for Enter, Jog does not.
- **B** As A, but a member may also jog, each jog session waiting for one owner confirmation.
- **C** Every motion command is owner-only; members only send jobs (changes the meaning of Frage 245 A for Home).
Recommendation **A**. Default if unanswered: **A**.

Frage 309 (MC-JOG-HOLD): How does a jog button move the head?
- **A** One press moves one step; holding the button moves continuously until release, with a dead-man rule (the connector stops the head when the app's keep-alive is missing for 300 ms or the connection drops).
- **B** Steps only; holding does nothing more.
- **C** Continuous only (move while held), no steps.
Recommendation **A** (fast positioning and exact steps, as LightBurn and UGS). Default if unanswered: **B** (the safest; hold-to-move added later by one answer).

Frage 310 (MC-FRAME-LASER): Is the laser on during Frame?
- **A** Off: the head traces the outline; you watch the head or a red-dot pointer if the machine has one.
- **B** On at a low frame power from the machine profile (`0140` criterion 2, default 0 %), set by you per machine.
- **C** Off now; B comes with `0140`'s frame loop.
Recommendation **C**. Default if unanswered: **A** (laser off; B is never the default).

Frage 311 (MC-FRAME-SHAPE): What is the "rubber-band" frame?
- **A** The convex hull around the job's burn moves (LightBurn's "Rubber Band Outline"), beside the rectangle.
- **B** A rectangle you drag on the canvas, which the head then traces.
- **C** Both: hull, and a dragged rectangle.
Recommendation **A**. Default if unanswered: **A**.

Frage 312 (MC-USER-ORIGIN): Where is a user origin kept?
- **A** On the page, in machine millimetres; the job file is shifted by it; nothing is written to the controller (no `G10`, `G92`, `G54`), so another program and the next power-up see an unchanged machine.
- **B** On the controller with `G10 L20 P1` (G54, kept in GRBL's EEPROM across power cycles, as LightBurn's "Set Origin"); the job runs in G54.
- **C** On the controller with `G92` (temporary, lost at a reset or alarm).
Recommendation **A**. Default if unanswered: **A**.

Frage 313 (MC-FOCUS): What is "focus-point mode"?
- **A** A press on the canvas moves the head, laser off, to that point, so you set the lens height there.
- **B** As A, then a short low-power pulse so you see the dot on the material.
- **C** A focus test pattern (lines at stepped heights or powers) to find the best focus.
Recommendation **A**. Default if unanswered: **A** (no laser output).

Frage 314 (MC-RUN-FROM-LINE): From which jobs can a run start at a line?
- **A** Only an interrupted job in the job list (`0132` criterion 12).
- **B** Also the current Ready job, from the position of the Toolpath view's play slider (`0029` criterion 37).
- **C** Also any job from a typed line number.
Recommendation **A** (smallest, every case guarded by rules a to d). Default if unanswered: **A**.

Frage 315 (MC-JOG-STEPS): Which jog steps does the pad offer?
- **A** 0.1, 1, 10, 50 mm, default 1 mm, fixed feeds per step (criterion 16).
- **B** As A, plus a typed step and a typed jog speed.
- **C** 1 and 10 mm only.
Recommendation **A**. Default if unanswered: **A**.

### Needed before Ready (second slice)

- Customer: Fragen 308 to 315 (defaults stand), and the one-line meanings above.
- `architect`: addendum to `adrs.md` for 2a first: the `Jog` command and jog cancel, the keep-alive, `Status` fields (machine position, work offset, homed since connect), the clamp in machine coordinates (`$23`, force-origin builds), the M5 rule, `access.rs` for the member's Home and Jog; then per sub-slice: frame text in `curvyo-laser-core` and `JobKind::Frame` with a Ready job (2b), the page's user origin and `format_version` (2c), go to point (2d), the modal rebuild and the run-from-line preamble (2e, with `0132` adrs).
- `ux-engineer`: addendum to `ux-notes.md`: the Jog toggle and pad in the footer (244px, the footer's height at 800 x 600, arrows per origin corner, hold-to-move feedback, keyboard), Frame and its pick, Set origin here and the User origin row, the Focus point mode on the canvas, the member's Confirm for Home, the "Not homed" and "Jogging"/"Framing" states.

## Needed by other stories

`0132` criterion 12 (Frage 258 C) needs the **run-from-line** of criteria 38 and 39 (sub-slice 2e); `0132` slice 2 is built in the same PR.

A member's Home waiting for an owner's confirmation (ADR 0071 decision 7; Fragen 244 A, 245 A) is criterion 37 (sub-slice 2a).

`0140` (frame loop, reference points) builds on 2b (Frame) and 2c (user origin, criterion 33).

## Out of scope

- Homing inside a job (`0029` criterion 38 stays; its bug is `0129`), changing GRBL settings, other controllers.

## Needed before Ready

Done: `ux-notes.md` (UX, W1 to W14) and the architect addendum in `adrs.md` (`Command::Home`, the move of the group, no new crate).

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003, R-MFG-011
Depends on: `specs/0127-connector-grbl-serial/`, `specs/0125-send-to-machine/` (Done, PR #379)
Related: `specs/0101-laser-operations/` (Operations tab), `specs/0129-fix-home-first-locked/`
PR: -

## Answers to the second-slice questions (customer, 2026-11-23)

- Frage 308 (MC-MOTION-ROLES): **A**. Frage 309 (MC-JOG-HOLD): **A**. Frage 310 (MC-FRAME-LASER): **C** (laser off now; a low frame power later with `0140`). Frage 311 (MC-FRAME-SHAPE): **A**. Frage 312 (MC-USER-ORIGIN): **A**. Frage 315 (MC-JOG-STEPS): **A**.
- Frage 313 (MC-FOCUS): **B**, with a condition: the low-power pulse must be switchable off, and its power belongs in the machine settings (machine profile), not in the Machine controls group. Criteria 34 to 36 take this on when slice 2d is built.
- Frage 314 (MC-RUN-FROM-LINE): not answered; default **A** (interrupted jobs only) applies.
- Frage 334 (MC-FRAME-TRAVEL, asked as 326 before `main` took that number): **A**, a homed frame that would leave the travel is refused before the head moves.
- Frage 335 (MC-FRAME-NO-POWER): **B**, frame the outline anyway: an operation with no power and/or speed set still frames from its geometry (criterion 27); only a page with nothing to trace reads "Nothing to frame".
