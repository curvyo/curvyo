# 0064 Machine connector, second slice: UX notes

ux-engineer, 2026-11-17. Covers the second slice of `specification.md` (criteria Q1 to Q22, mainly Q5 to Q9, Q11 to Q17, Q19) and the list "UX notes needed" there. Checked against `specs/0026-machine-profile/ux-notes.md` (Machine block, Connection sub-group), `specs/0093-laser-workbench-shell/ux-notes.md`, `specs/0098-page-kinds/ux-notes.md` (the Machine block now sits in a laser page's Document tab), the built `frontend/src/components/workbench/MachineBlock.tsx` (`ConnectionGroup`), and `docs/design-system.md` (rows "Machine block (profile picker)", "Action notice", "Hidden, not disabled", "Tooltips and validation chips", "Header row with symbol buttons"; Interaction conventions "No notice after a success", "Inline delete confirm", "Armed action", "Segmented icon control"). Customer rules applied: no popups, no dialog of our own (the OS file dialog only), symbol buttons with tooltips and accessible names, labels that name a control or a state only, no operating tips. Frage 1 is built as its default A (file picker). Choices marked (Proposal) are vetoable at the demo.

## 1. Where it lives

- The **Connector** sub-group is the last part of the Machine block's "Connection" sub-group: after Connector address, Controller port and Baud rate, 12px gap, then the Connector group. The Fixtures block follows below it as today. Nothing else in the Document tab, the strip, the rail or the status bar changes (spec: "Nothing else in the app changes").
- Content width 244px, controls 28px high (36px coarse pointer), 4px gaps between controls, 8px between rows of different kinds, as in the rest of the Machine block.
- The group is shown only for a laser page's Machine block. It reads and writes no project data and no profile record: none of its actions is a History step, Ctrl+Z never touches it (Q10).
- The group is per profile id: picking another profile shows that profile's pairing and job; a duplicated profile (new id) starts "Not paired".

## 2. Layout, top to bottom

Every row below appears only while its state applies and leaves at once (no animation, no reserved height); the heading row is the anchor and never moves.

1. **Heading row (28px).** "Connector" at the left (14px `--toolbar-icon`, `h4` inside the Connection section), at the right the **Test connection** button (section 3). The Fixtures heading with its `Plus` is the precedent.
2. **Link line.** 12px text, wraps (at most 3 lines, `overflow-wrap: anywhere` for addresses), the result of the last test, pairing or status read (section 4). Absent when there is nothing to say.
3. **Trust block (paired only).** Two lines on a 2-column grid (label column 80px, 12px `--panel-muted-fg`; value column the rest):
   - "Paired with" / the connector's name (12px `--toolbar-icon`, one line, end ellipsis; the full name is in the block's accessible text), and at the right the **Forget pairing** button (section 3).
   - "Key" / the fingerprint (section 5).
   The accessible text of the block is the spec's sentence "Paired with <connector name>, key <fingerprint>" (Q5).
4. **Key changed block (Q8 only, instead of the trust block).** Two fingerprint rows on the same grid: "Stored key" and "Presented key". No Forget button here (the Pair row below is the way on).
5. **Pair row (not paired, no longer paired, key changed).** The **Code** field (212 x 28) and the **Pair** button (28 x 28) 4px after it (section 6).
6. **Job toolbar (paired, key not changed).** `role="toolbar"` named "Job", left-aligned, four 28 x 28 buttons 4px apart (124px): **Send job...**, **Start / Resume** (one slot), **Pause**, **Stop** (section 3, section 7).
7. **Job block (a job is known).** State line, progress bar, counts line (section 8).

Unavailable states replace rows 2 to 7 (section 9).

## 3. Symbol buttons

28 x 28 (36 coarse), `rounded-md`, 16px Lucide glyph at 1.5px absolute stroke, `currentColor` (`--toolbar-icon`), hover `--editor-accent-hover`, focus ring 2px `--editor-accent` inside (the `ICON_BUTTON` of `MachineBlock.tsx`). Tooltip (Radix, 400ms, `side="left"`) and accessible name are the same text, the name of the control; no shortcut text, no reason text.

| Control | Glyph | Name and tooltip | Enabled when |
|---|---|---|---|
| Test connection | `PlugZap` | "Test connection" | a connector address is set and no test or command is running |
| Forget pairing | `Unlink` | "Forget pairing"; armed: "Forget pairing. A new code is needed." | always while shown |
| Pair | `KeyRound` | "Pair" | the Code field holds 8 characters of the code alphabet and no pairing is running |
| Send job... | `Upload` | "Send job..." | section 7 table |
| Start / Resume | `Play` | "Start" (Ready, and while disabled) / "Resume" (Paused) | section 7 table |
| Pause | `Pause` | "Pause" | section 7 table |
| Stop | `Square` | "Stop" | section 7 table |

- **Not possible = greyed, in place (Proposal, a new recorded exception to "Hidden, not disabled").** The job toolbar's buttons, Test connection and Pair are `aria-disabled` at 40% when they do not apply, keep their place, Tab stop and tooltip; a press does nothing and sends nothing. Reason: a row of machine controls whose buttons come and go would move Stop under the pointer, and Q15 asks for "enabled only in the states where it applies" (the flip buttons and the Share tab's "Armed action" are the precedents).
- **Busy.** From the press until the answer (or 5 s, then "Timed out"): the pressed button is `aria-disabled`, `aria-busy="true"`, its glyph is `LoaderCircle` turning once per second (still under `prefers-reduced-motion`); the other buttons of the group keep their state. The app never changes a state before the connector's `Status` says so (no optimistic flip of Start to Pause).
- **Forget pairing** removes the pinned key and the token of this profile from the credential store. It has no undo and a new code is needed, so it is the **Armed action** (`useArm`: first press fills the button `--destructive` with a white glyph, second press after 400ms acts, disarms after 4s, on Escape, on another press, when focus leaves). After it the trust block and the job rows go, "Not paired" and the Pair row appear, focus moves to the Code field.
- **Stop never asks** and is never armed: it is the safety control. Stop in Ready discards the held job (P17); same glyph and name.
- **Start / Resume share a slot (Proposal):** they never apply in the same state, both mean "run from here", and the media-player convention puts them in one place. The name changes with the state; the glyph does not.
- Escape never sends Stop or Pause (Escape keeps its panel meanings). No global chord for any job command in this slice (Proposal; a laser must not start from a stray key).

## 4. Link line texts

Look: 12px. Neutral texts `--toolbar-icon`; muted state words `--panel-muted-fg`; failures `--destructive` (4.8:1 on `--panel-bg`) with a 12px `CircleAlert` glyph at the left (shape, not colour alone). Live region: `role="status"` (polite) for everything; a failure that answers the maker's own press is also written to a hidden `role="alert"` (the Trace precedent). A status read on opening the block (Q16) is never an alert.

| When | Text | Look |
|---|---|---|
| Test or status read running | "Testing" | muted |
| Pairing running | "Pairing" | muted |
| Test succeeded (Q11) | "Connected, controller answered: <controller name>, protocol 0, connector <version>" | neutral |
| Typed address is not an address (Q11) | "Not a valid address" | failure |
| Nothing listens (Q11) | "Nothing answers at <host:port>" (the address as dialled, port 7447 when none was typed) | failure |
| No answer in 5 s | "Timed out" | failure |
| No credentials for this profile | "Not paired" | neutral |
| Token revoked (Q9) | "No longer paired" (see Open point 1) | neutral |
| Key changed (Q8) | "This connector's key changed" | failure, always an alert, also on open |
| Protocol differs | "Protocol mismatch: app 0, connector <n>" | failure |
| Pairing: wrong code (Q6) | "Wrong code, <n> attempts left" ("1 attempt left") | failure |
| Pairing: third wrong code | "Code spent" (see Open point 1) | failure |
| Pairing: expired | "Code expired" | failure |
| Pairing: code already used (Q5) | "Code not valid" | failure |
| Pairing: man in the middle (Q7) | same as a wrong code: "Wrong code, <n> attempts left" (SPAKE2 cannot tell a different key from a wrong code, ADR 0035 §4) | failure |
| A job command names a job the connector does not hold (Q15) | "The connector holds another job" | failure |
| Connection lost while a job was active (Proposal, new text) | "Connection lost" | failure |
| Forget pairing fails: the credential store would not remove the record (Q10a; Proposal, new text) | "Could not forget the pairing" (the pairing stays shown; also an alert, it answers the maker's press) | failure |
| This computer has no credential store (Proposal, new text) | "No credential store on this computer" | failure |
| Any other error the app has no words for (Proposal, new text) | "Connector error" (the detail goes to the log only, never to the row) | failure |

- The link line's live region is always in the DOM, empty when there is nothing to say (never `display: none`), so a screen reader announces the text when it appears.
- A successful pairing writes no line (no notice after a success): the trust block appearing is the feedback.
- The line stays while the block is shown for that profile; it is not stored (Q12). A profile pick, a page change or a new test replaces or clears it.
- **Opening the block (Q16):** with a paired profile and an address, the app reads `Status` once ("Testing" while it runs). While a job is in Receiving, Ready, AwaitingLocalStart, Running or Paused and the block is shown, the row follows the connector's pushes. When that connection drops, the line reads "Connection lost", the job block keeps its last values, and the app tries again every 5 s while the block is shown (Proposal); the job buttons keep the last known state, and a press opens a fresh connection first, so Stop works from a new connection (Q16, P19).

## 5. Key fingerprint

- 8 groups of 4 upper-case hex characters, the same order as the connector's terminal prints them (Q1), shown as **two lines of 4 groups** (19 characters each; one line of 39 does not fit 164px), 12px monospace with tabular figures, `--toolbar-icon`, groups separated by one space.
- Selectable text (`user-select: text`) so the maker can copy and compare it; no copy button (YAGNI).
- Screen readers read the two lines in order as one value; the row label ("Key", "Stored key", "Presented key") is its name.
- Shown in: the trust block (paired), the key changed block (both fingerprints, Q8). Nowhere else in the app.

## 6. Pair row

- **Code field**: 212 x 28, white ground, label "Code" inside the left edge (12px `--panel-muted-fg`), value 14px monospace, shown in upper case as typed (`autocapitalize="characters"`, `autocomplete="off"`, `spellcheck="false"`). Spaces and hyphens are ignored, lower case is accepted; over 9 characters is cut as typed. The value is never stored.
- **Pair** is greyed until the field holds 8 characters of the code's alphabet, so a typing slip never spends one of the three attempts.
- Enter in the field presses Pair; Escape clears the field.
- During pairing the field is read-only and Pair shows the busy state. On success the Pair row goes, the trust block and the job toolbar appear, focus moves to Send job.... On failure the typed code stays, selected, focus stays in the field.

## 7. Job buttons per state

The state is the last `Status` the app read. "No status" means no status has been read yet or the last read failed.

| State | Send job... | Start / Resume | Pause | Stop |
|---|---|---|---|---|
| No status | on | off | off | off |
| Idle | on | off | off | off |
| Receiving (sending) | off | off | off | off |
| Ready | off | on, "Start" | off | on (discards) |
| AwaitingLocalStart | off | off | off | on (discards) |
| Running | off | off | on | on |
| Paused | off | on, "Resume" | off | on |
| Done (Completed, Stopped, Failed) | on | off | off | off |
| Alarm | off | off | off | off |
| NoController | off | off | off | off |

With a last known Running, Paused or AwaitingLocalStart and "Connection lost", the buttons keep that row (section 4).

**Send job... and the file picker (Frage 1 A).**
- Opens the **OS file dialog** only (no own dialog), title "Send job", filters "G-code and text (*.gcode, *.nc, *.txt)" first, then "All files". It starts in the folder last used for Send job, remembered per OS user (Proposal), else the OS default.
- Cancel: nothing changes, focus returns to Send job....
- A pick sends at once (Send does not run anything: Frage 2 A, Start is the second press). The job's name is the file name.
- A file over 64 MiB or with a byte outside printable ASCII and newline is refused with the same texts whether the app or the connector finds it (section 8).
- Desktop only (section 9).

## 8. Job block

Three parts, top to bottom, 4px apart.

1. **State line** (12px, wraps, `role="status"`): the text of the table below at the left; the **elapsed time** at the right in Running, Paused and Done (12px tabular, `--panel-muted-fg`, accessible name "Elapsed <text>").
2. **Progress bar** (only in states with a line count): 244 x 6px, `rounded-full`, white track with a 1px `--field-border` border, fill `--editor-accent` from the left to `line / lines` (4.5:1 to the white track, 3.3:1 to the panel); fill `--destructive` in Failed and Alarm (the state words carry it as well). `role="progressbar"` named "Job progress", `aria-valuemin` 0, `aria-valuemax` lines, `aria-valuenow` line, `aria-valuetext` "1,234 of 5,000 lines". The width changes without a transition.
3. **Counts line** (12px): the job name at the left (`--panel-muted-fg`, cut in the middle with an ellipsis so the extension stays, to fit), and, in states with a line count, "<line> / <lines> lines" at the right (tabular, `--toolbar-icon`).

| State | State line | Bar and count |
|---|---|---|
| Sending | "Sending <name>" | no |
| Ready (Q13) | "Ready: <name>, <n> lines, <size>" | no |
| AwaitingLocalStart (Q17) | "Waiting for the start at the machine" | yes, at 0 |
| Running (Q14) | "Running" | yes |
| Paused (Q15) | "Paused" | yes, held |
| Done, Completed | "Completed, <n> lines" | yes, full |
| Done, Stopped | "Stopped at line <n>" | yes, held |
| Done, Failed | "Failed at line <n>" (Proposal, new text) | yes, held |
| Alarm (Q19) | "Alarm: <text>", `TriangleAlert` 12px, `--destructive`, also `role="alert"` | yes, held, if a job ran |
| Refused on send (Q13) | "Not plain ASCII text" / "Larger than 64 MiB" / "Hash mismatch" / "Connector busy" / "No controller"; "Could not read the file" when the chosen file cannot be read (nothing was sent); "Machine in alarm: <text>" when the connector answered Busy while its status has an alarm (Q19); `CircleAlert`, `--destructive` | no; the counts line shows the name |
| Idle, NoController | block absent; NoController writes "No controller" in the state line with no bar | no |

- A refusal stays until the next Send, test or state change; nothing is held (Q13).
- **Numbers:** lines with a thousands comma ("5,000", as `0026`); size in binary units to match the 64 MiB limit: "812 B", "4.2 KiB", "120 KiB", "3.5 MiB" (one decimal under 10, Proposal).
- **Elapsed:** from the status's `elapsed_ms`, "m:ss" under an hour ("0:07", "12:34"), "h:mm:ss" from an hour ("1:02:03"); it changes only when a status arrives (at most 5 a second, Q14).
- **Live region load:** only the state words are announced (Running, Paused, Completed, ...); the line count, bar and elapsed time are not live (`aria-live="off"` on the counts line), so a 20-line-a-second job does not flood a screen reader.
- The block shows whatever the connector reports, also a job another computer sent.

## 9. Unavailable and disabled states

| Case | What the Connector group shows |
|---|---|
| Browser build | Heading "Connector" (no Test button) and one 12px `--panel-muted-fg` line **"Needs the desktop app"**. Nothing else: no control can apply in this build, so they are hidden, not greyed. G-code export is unaffected. |
| Built-in profile or "Not in your profiles" (Q11) | The Connection group's existing line "Only saved profiles have a connection." already says why. The heading row shows Test connection greyed with `aria-describedby` on that line; rows 2 to 7 are absent. |
| Saved profile, empty Connector address | Heading with Test connection greyed; link line "No connector address" (muted, a state). No Pair row, no job rows. |
| Address set, not paired | Test connection on, "Not paired", Pair row. |
| Paired | Test on, trust block, job toolbar, job block when a job is known. |
| Key changed (Q8) | Failure line, key changed block, Pair row; no job rows (nothing is sent to that key, no token). |
| No credential store (Proposal) | Failure line "No credential store on this computer"; no Pair row (pairing cannot work without a store: hidden, not greyed); Test connection stays. |
| No longer paired (Q9) | "No longer paired", Pair row; the stale trust block and job rows go. |

## 10. Keyboard and focus

- **Tab order in the Connection group:** Connector address, Controller port, Baud rate, Test connection, Forget pairing (when shown), Code, Pair (when shown), the Job toolbar (one Tab stop), then the Fixtures block. Greyed buttons keep their Tab stop.
- **Job toolbar:** roving focus, Left and Right move, Home and End jump, no wrap; greyed buttons are reached too (their tooltip names them). Enter or Space presses.
- Focus after a button unmounts never drops to the body: after pairing, Send job...; after Forget pairing, Code; after a state change that greys the focused job button, the focus stays on it.
- Enter and Escape in the Code field as in section 6; Escape on an armed Forget pairing disarms it.
- Hit targets 28px, 36px coarse; tooltips appear on keyboard focus as on hover.

## 11. Connector terminal wording (demo, not UI)

Plain lines, no colour codes required, no hint lines beyond what the spec fixes (Proposal):

```text
Curvyo connector 0.1.0, protocol 0
Fake controller: no machine is attached, nothing moves.
Listening on 192.168.1.20:7447
Key fingerprint: 3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
Pairing code: K7QM-4XTR, valid for 10 minutes
```

- The code is printed as two groups of 4 with a hyphen (the Code field ignores it).
- One line per job outcome, matching the log result words (Q18): "Job layer-test.gcode: Completed".
- With `--require-local-start`: "Press Enter to start <name>" (Q17), and after Enter "Started <name>".
- A damaged key file (Q3): one line naming the file and the refusal to start; no stack trace.

## 12. Checks for the review

- **Contrast:** only existing tokens. 12px `--toolbar-icon` 8.2:1, `--panel-muted-fg` 5.0:1, `--destructive` 4.8:1 on `--panel-bg`; bar fill 3.3:1 to the panel and 4.5:1 to its white track; greyed buttons at 40% are exempt (inactive) and carry their name.
- **States to try:** every row of sections 4, 7, 8 and 9, in the demo setup with `--fake-lines-per-second`, `--fake-alarm-at-line`, `--require-local-start`, `--revoke`, a replaced state directory (key changed) and the connector stopped (nothing answers, connection lost).
- **No popups, no notices after success:** pairing, sending, start and a passing test add no toast; the only floating layers are tooltips and the OS file dialog.
- **Screen reader:** state words announced once per change, line counts not live; failures of a press as alerts.
- **Undo:** no connector action is a History step.
- **Large documents:** nothing here reads the drawing.

## Open points for the lead

1. **Hint texts in the spec (customer question).** Q6 fixes "Code spent, start the connector with --pair" and Q9 "No longer paired, pair again"; the second halves are operating tips, which the customer rule of 2026-10-16 forbids. These notes use "Code spent" and "No longer paired" (the Pair row is the visible way on). Proposed question: Frage (CONN-HINT): A (default, recommended) the short state texts; B keep the spec's sentences as a recorded exception. The PO aligns Q6 and Q9 with the answer.
2. **New texts (Proposal, PO to add to the spec):** "Testing", "Pairing", "No connector address", "Connection lost" with the 5 s retry, "Failed at line <n>".
3. **Architect:** the credential record per profile id also keeps the connector's name (the trust block shows it offline); deleting a user profile removes its record too (Proposal; the profile's `0081` delete arm already guards it).

## Design-system rows to add on acceptance

None now (this file only). After the build: one row "Connector group (`0064`)" after "Machine block (profile picker)" (sections 2 to 9), and in "Hidden, not disabled" a sixth recorded exception for the job toolbar, Test connection and Pair (section 3).
