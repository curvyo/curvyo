# 0133 Pause while homing: UX notes

ux-engineer, 2026-11-19. Covers criteria PH1 to PH11 of `specification.md` and its "Needed before Ready" item for the ux-engineer: the pending Pause state, the words "Pause after homing" in the state line, the exact look of criterion 8. Checked against `specs/0118-machine-control/ux-notes.md` (on `story/machine-control`, PR #399: footer, toolbar, Home causes, state line texts, live regions, Frage 262 A), `specs/0064-machine-connector/ux-notes.md` sections 3, 7, 8 (symbol buttons, buttons per state, job block), `specs/0127-connector-grbl-serial/ux-notes.md` section 7 (Pause busy until "Paused"; homing first reads "Running", line 0), `specs/0125-send-to-machine/ux-notes.md`, the built `frontend/src/components/panel/MachineControls.tsx` and `frontend/src/lib/connectorText.ts` (PR #399), and `docs/design-system.md` (rows "Machine controls (Operations tab) (`0118`)" as proposed, "Connector group (`0064`)", "Hidden, not disabled"; Interaction conventions "No notice after a success", "Nothing moves under the pointer"). Customer rules applied: no popups, no dialog, labels that name a control or a state only, no operating tips (2026-10-16), no notice after a success (2026-10-31). Choices marked (Proposal) are vetoable at the demo; section 8 lists them as H1 to H8. No question for the customer.

## 1. Where it shows

Only in the **Machine controls** footer of the Operations tab (`0118`): the Pause button of the toolbar, the Start / Resume slot next to it, and the job block's state line. By Frage 262 A (customer, 2026-11-19) the Output group has no state line any more, so "the state line" of PH8 and of the spec's "Needed before Ready" (`0125` S13) is the footer's one line. Nothing changes in the Output group, the Machine block or the canvas.

## 2. The state line (PH1, PH2, PH8)

The job block of `0118` section 5 (`0064` section 8 form: state words at the left, elapsed time at the right, bar, counts line with the name and "<line> / <lines> lines").

| When | State line | Elapsed, bar, counts line |
|---|---|---|
| A job's `$H` runs, no pause pressed | "Running" (as built, `0127` section 7: line 0) | as built: elapsed runs, bar at 0, "0 / 412 lines" |
| The connector reports `pause_pending: true` | **"Pause after homing"** | unchanged from the row above: elapsed runs, bar and count as reported |
| GRBL ends homing, the connector holds | "Paused" (the footer's built word) | elapsed held, bar held, count the line after `$H` |
| Resume while pending | back to "Running" | unchanged |
| Stop while pending | the Stop result as built: "Stopped at line <n>", or on GRBL "Alarm: Reset during homing (alarm 6)" | as built |
| Homing fails while pending (PH6) | the built failure: "Alarm: Homing did not find the switch (alarm 9)", "Alarm: Homing pull-off did not clear the switch (alarm 8)", "Failed at line <n>: Homing is not enabled (error 5)", "Lost the controller at line <n>" | as built |
| Connection lost while pending | "Connection lost" (as built, last values kept) | last values |

- **Words (H1).** "Pause after homing", 18 characters, well within the 70-character line; neutral look (`--toolbar-icon`, no glyph), it is a state, not a failure. It names what will happen and when; no "Please wait", no "Homing cannot be paused", no instruction. It replaces "Running" (or "Homing", if the architect has the connector report a job's `$H` as `Homing`) for as long as the status carries `pause_pending`.
- **Read from the status only (PH8).** The line, the Pause look and the Resume slot follow `pause_pending` in the last `Status`; the app keeps no memory of its own press. A second app watching the same connector shows the same line and look. A restarted connector or a lost controller (PH9) reports no pending pause, so the line follows without a text of its own.
- **A dropped pending pause has no text of its own (H2).** When Stop, a homing failure or a lost controller ends the job, the result line says what happened; "Pause cancelled" would only repeat that the job ended. Resume cancels it by the maker's own press, and the line returning to "Running" is the answer.
- **Mismatch with PH8:** the spec quotes "Paused at line <n>"; that is the `0125` Output line, which left by Frage 262 A. The footer reads "Paused" with the line in the counts line (as built). The PO aligns PH8.

## 3. The Pause button (PH1, PH4, PH8)

**The pending pause looks like a Pause that has not been confirmed yet, because that is what it is (H3).** `0127` section 7 already keeps Pause busy from the press until "Paused" (`LoaderCircle`, until GRBL reports `Hold:0`). A pause during homing is the same wait, only longer:

| Phase | Pause | Start / Resume slot |
|---|---|---|
| Press, until the connector's answer (at most 5 s, then "Timed out", as built) | busy: `LoaderCircle` turning once a second, `aria-busy="true"`, `aria-disabled` | "Start", greyed (as built while a button waits) |
| Status says `pause_pending: true` | **stays busy**, now driven by the status, not by the press: same `LoaderCircle`, `aria-busy`, `aria-disabled`; **no time limit** while homing (Home's busy rule in `0118` section 4, W12) | **"Resume", enabled**, glyph `Play` (PH8) |
| Status says Paused | greyed as in the Paused row (`0064` section 7) | "Resume", enabled (as built) |

- **No `aria-pressed` (H4).** Pause is not a toggle: a second press does not release it (PH4: a second Pause is idempotent) and Resume, a different button, cancels it. `aria-pressed` would tell a screen reader that pressing again un-pauses. No toggle-on ground (`--toolbar-icon-active-bg`) either: that look means "on, press to turn off" everywhere else in Curvyo (Guide, Link corners).
- **Tooltip and description while pending.** Two lines, the Home pattern (`0118` section 4): line 1 "Pause" (13px), line 2 "Pause after homing" (12px `--panel-muted-fg`); the accessible name stays "Pause", `aria-describedby` points to an sr-only span with "Pause after homing". This is the one other cause line in the toolbar besides Home's (amends `0118` W6); it is a state, the same string as the line.
- **Not "the others greyed while one waits".** The built rule greys the other buttons while one button waits for its answer. The status-driven pending phase is not such a wait: **Stop is enabled** (PH5, never held back) and **Resume is enabled** (PH8). Only the short press phase (until the answer) greys them as built. The implementer keeps the press wait and the pending state apart.
- **Pause enabled during a job's homing.** `0118` section 3 greys Pause "while Homing". That rule is for Homing from the Home button without a job (PH3). While a job runs its own `$H` (the connector reports Running at line 0, or Homing with the job, as the architect fixes it), Pause is enabled, because the job can be paused after homing. Home stays off with "A job is running" while a job homes (it was not the Home button's homing).

## 4. Resume, Stop, failure (PH4 to PH6)

- **Resume while pending** sends `Resume` as built: busy until the status drops `pause_pending`; then the slot reads "Start" greyed (the Running row), Pause is enabled again with its glyph, the line reads "Running". The focus stays on the slot (a button greyed by a state change keeps the focus, `0064` section 10). A new Pause press makes it pending again.
- **Stop while pending** is Stop as built: enabled, never asks, never armed, busy until the machine confirms; the pending look leaves with the status that follows. No extra text (H2).
- **Homing fails while pending:** the alarm or failure line of section 2 as built (alarm `role="alert"`, Unlock shown); Pause leaves the busy look with the status, the slot reads "Start" greyed. The line never reads "Paused" (PH6).

## 5. "No job to pause" (PH3)

- While the connector reports Homing from the Home button (with or without a Ready job), Pause is greyed in place as built (`0118` section 3), with its name-only tooltip "Pause". The app never sends Pause there.
- The refusal still reaches the app when the state changes between the last status and the press (another app pressed Home), or from `curvyo-connector-ctl`. Then the footer's state line reads **"No job to pause"** (15 characters) in the failure look (`CircleAlert` 12px, `--destructive`), as a hidden `role="alert"` (it answers the maker's own press), until the next command or state change, the rule of "Homing is not enabled (error 5)" in `0118` section 5. Homing goes on; nothing is remembered.
- Words: a state, not an instruction; no "Wait for homing to end", no "Use Stop".

## 6. Live regions and screen reader (PH8)

The two hidden regions of `0118` section 11 (`role="status"` and `role="alert"`, sr-only, repeating the state line) carry it; the visible line has no live role.

- "Pause after homing": **polite**, once, when the status first carries `pause_pending` (it is the answer to the press but not a failure). The press itself announces nothing else (the busy `aria-busy` is not announced by itself).
- "Paused": polite, once, when GRBL has homed and the connector holds.
- "Running": polite, once, when Resume cancels the pending pause.
- Stop, alarm and failure lines: as built (alarms and own-press failures as alerts). "No job to pause": alert.
- The elapsed time, bar and counts are not live (as built).
- Not colour alone: the pending state is in words (line, description) and in the turning glyph.

## 7. Checks for the review

- **States to try** (pty simulator with silent homing, `0127` G22; fake controller): Pause during a job's `$H` (line, Pause busy, Resume enabled, Stop enabled); then `ok` ("Paused", no motion); Resume while pending; second Pause (nothing new, Pause stays busy); Stop while pending; `ALARM:9` while pending; controller lost while pending; connector restarted while pending; a second app watching (same line, same look); Home button homing and a Pause from `curvyo-connector-ctl` ("No job to pause"); Pause in a normal run (unchanged: busy until "Paused", the 2 s drop as built).
- **Nothing moves:** the line changes words in place; the toolbar never moves (the footer grows upward, `0118` W2).
- **No popups, no hint texts, no notice.** Contrast: existing tokens only (`--toolbar-icon` 8.2:1, `--panel-muted-fg` 5.0:1, `--destructive` 4.8:1 on `--panel-bg`).

## 8. Decisions for veto (Proposal)

- **H1** The state line reads "Pause after homing" while the status carries `pause_pending` (the spec's words, kept).
- **H2** A pending pause that Stop, a homing failure or a lost controller ends has no text of its own; the result line says what happened.
- **H3** The pending Pause keeps Pause's busy look (`LoaderCircle`, `aria-busy`, `aria-disabled`), driven by the status, without a time limit while homing, as Home during homing.
- **H4** No `aria-pressed` and no toggle-on ground: Pause is not a toggle, Resume cancels.
- **H5** While pending, Pause's tooltip and description have a second line "Pause after homing" (the toolbar's second cause line after Home's).
- **H6** While pending, Stop and Resume are enabled; only the press wait greys the other buttons, as built.
- **H7** Pause is enabled while a job runs its own `$H`; it stays greyed while the Home button homes.
- **H8** "No job to pause" reads in the footer's state line in the failure look, as an alert, until the next command or state change.

## 9. For the product owner, the architect and the lead

- **PO, PH8:** "Paused at line <n>" becomes "Paused" (footer form, line in the counts line; Frage 262 A). "Needed before Ready" names `0125` S13's Output line, which is gone; the footer's line is meant.
- **Architect:** the status must tell "a job runs its own `$H`" from "Homing from the Home button" (H7: Pause enabled in the first, greyed in the second), whichever state name the job's `$H` gets; `pause_pending` is read on every `Status` push, so a second app follows (PH8).
- **Amends `0118` UX notes** (on `story/machine-control`, not on this branch; the lead adds the lines there or in its successor): section 3 "Pause off while Homing" gains "except while a job runs its own `$H` (amended by 0133)"; section 3 "While one button waits, the others are greyed" gains "not during a pending pause (amended by 0133)"; W6 gains Pause's pending line (amended by 0133).
- **Design-system row "Machine controls (Operations tab) (`0118`)"**, to land with the build, gains: "Pause during a job's homing: the connector keeps it pending (`pause_pending`); the state line reads 'Pause after homing', Pause keeps its busy look driven by the status without a time limit, a two-line tooltip 'Pause' / 'Pause after homing', no `aria-pressed`; Resume is enabled and cancels it, Stop is enabled. 'No job to pause' in the failure look when the Home button's homing refuses a Pause (`0133`)." No new token, no new glyph.

## Links

Spec: `specs/0133-fix-pause-while-homing/specification.md`. Notes: `specs/0118-machine-control/ux-notes.md` (branch `story/machine-control`), `specs/0064-machine-connector/ux-notes.md`, `specs/0127-connector-grbl-serial/ux-notes.md`. Code: `frontend/src/components/panel/MachineControls.tsx`, `frontend/src/lib/connectorText.ts`.
