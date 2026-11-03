# 0064 Machine connector: UX notes

Status: Draft for Ready (slice 1). Owner: ux-engineer, 2026-11-02. Authority: `specification.md` (criteria
11 to 33 and 38 name the visible behaviour), `adrs.md`, ADR 0032. References: `docs/design-system.md`
("Properties panel: tabs", "File tab", "Inline confirm", "Canvas notice slot", "Interaction conventions",
"Keyboard shortcuts established so far"), `PanelStrip.tsx`, `CanvasNotice.tsx`, `ValueField.tsx`,
`FormatList.tsx`, `FormatForm.tsx`, `PanelRadioGroup.tsx`. **(Proposal)** marks reversible choices.
Customer goal: nice, consistent, above all working, no new toolbars. Customer rule 2026-10-16: no
operating tips or hint labels; labels that name a control or a state stay, and so do reasons that explain
why a control does nothing. Every text below that carries a number or a state is formatted in Rust
(`curvyo-ui-core`), as for the other panels.

## 1. Where machines live

Two surfaces, one source of truth (the session's machine view).

- **Machine tab** in the Properties panel strip, label "Machine", Lucide `Cpu` 16px, appended after the
  last tab that exists when this ships (the order never changes). Tooltip "Machine (Shift+Ctrl+M)" on the
  desktop build, "Machine" in the browser build (Chrome keeps Shift+Ctrl+M, the same reason as the Fonts
  tab). It holds everything: records, pairing, state, home, jog, frame, file, send, progress. It never
  activates by itself (like History). Present in every build; where the connector cannot be reached by the
  page (section 10) the body says why instead of hiding the tab.
- **Machine pill**, the only always-visible piece, floating at the canvas's bottom right (section 3). It
  answers "which machine is the target, in what state" and holds Pause/Resume and Stop. It exists only while
  a machine record is selected, so a drawing session without machines has no extra chrome.

No toolbar, no rail card, no top bar, no popup, no dialog of our own. Results and refusals are inline lines
in the tab or in the pill; the canvas notice slot is used only for the key press result of section 12.

## 2. Machine tab layout (244px content, 12px body padding, 8px rows)

Header row: "Machine" at the left, subject line right-aligned: the selected record's name and state, "Ortur LM2,
Idle" (12px `--panel-muted-fg`, ellipsis, full text in a tooltip). Blocks, top to bottom; a block that has
nothing to show is not in the tree (hidden, not disabled), except where stated.

| # | Block | Contents |
|---|---|---|
| 1 | Machines | The record list, a `FormatList` look: one row per record (28px, 36 coarse): state glyph, name, work area "400 × 400 mm" muted; selecting is a radio (`role="radiogroup"` "Machine", Up/Down, Enter or Space selects). Row end: Pencil (edit). Last row "Add machine" (Plus). No record: the sentence "No machine yet." and one button "Add machine" |
| 2 | Banners | At most one, in this precedence: changed key (3.4), alarm (6), controller lost (8), not reachable (8). `role="alert"` when it appears, then static |
| 3 | State | Connection line, state chip, work position (section 5) |
| 4 | Move | Home, Unlock (Alarm only), jog pad, step choice, reason line (section 6) |
| 5 | Job | File button, the statement (section 7), Frame, Send; while a job exists the Progress block replaces Frame and Send (section 8); then the Last job line |

Scroll: the body scrolls under the fixed header row, position reset on tab change (existing rule). At
800 x 600 blocks 1 to 4 fit above the fold; Job scrolls into view when a file is chosen.

## 3. Pairing and the record form

### 3.1 The form

"Add machine" and the Pencil replace the list in place by the inline form (the `FormatForm` pattern: draft
kept across tab changes, Escape or Cancel drops it, New and Open drop it). Fields, one per 28px row with a
12px label above, `EntryField`/`ValueField` behaviour (Enter or Tab commits, invalid gets `--field-invalid`
border, a message line, `aria-invalid`):

1. **Name** (text, required, unique).
2. **Work area** width and height in mm (two `ValueField`s, W and H).
3. **Jog speed** in mm/min (criterion 14; default 3000, **Proposal**).
4. **Frame power** in percent, 0 to 100, default 0 (criterion 16).
5. **Connector address** (text, `host:port`, e.g. `192.168.1.40:8765`; `ws://` is added by us).
6. **Pairing code** (see 3.2), only while the record has no pinned key.
7. Buttons: Cancel, and **Pair** (new record or re-pair) or **Save** (existing paired record), 76 x 28 /
   118 x 28, Apply look for the primary.

A record cannot be saved unpaired: Pair is the save. Editing a paired record's address, name, work area,
jog speed or frame power uses Save and keeps the pinned key.

### 3.2 Pairing step

The Pairing code field is a text field showing digits in two groups of four ("4821 7365"), accepting spaces
and a hyphen on paste, `inputmode="numeric"`, `autocomplete="off"`, `maxlength` 9, mono digits
(`tabular-nums`). Label "Pairing code", placeholder "8 digits". No instruction text: the connector prints
the code and the address in the same grouping (section 11), so the two places read alike. Enter in the code
field presses Pair.

While pairing: the button reads "Pairing..." and ignores presses, the fields are `aria-busy`, Cancel stays
live. Pairing never takes more than one handshake; there is no wizard and no second screen.

Result lines (message line under the code field, 12px, `--field-invalid` with `aria-invalid` on a refusal;
`role="alert"` for refusals, `role="status"` for success):

| Case | Text |
|---|---|
| Success | The form closes, the record is selected, the list row shows the key line (3.3) |
| Wrong code | "Wrong code. 3 tries left." (count from the connector; at 0: "Too many tries. Start pairing again on the connector.") |
| Expired or spent | "That code has expired. Start pairing again on the connector." |
| Connector shows no code | "This connector is not offering a code." |
| Unreachable | "Cannot reach 192.168.1.40:8765." |
| Wrong protocol or not a connector | "That address is not a Curvyo connector." |
| Already paired with this app but key differs | the changed-key banner (3.4), no form message |

The code is never stored, never logged, and the field is cleared after any outcome except Unreachable.

### 3.3 Fingerprint display

The connector's key fingerprint is shown once the record is paired: in the form (read-only row "Connector
key") and in the record row's tooltip. Format: 16 base32 characters in four groups of four, mono 12px,
`--panel-muted-fg`, selectable ("7K2Q 9XMD 4HTR 2WVC"). Same helper and same grouping as the participant
fingerprints of 0092 and the connector's own `devices` output, so one string can be compared across all
three. It is information, not a step: pairing does not ask the maker to compare it (SPAKE2 already binds the
code to the key).

### 3.4 Changed key

When the connector presents another key under a paired record (criterion 9) the Banners block shows, and
the app does not connect:

- Glyph `ShieldAlert` 16px in `--destructive`, then the sentence, 12px `--toolbar-icon`: "The connector's key
  changed. Nothing is sent to it until you pair again."
- Two fingerprint rows in the 3.3 format: "Paired" and "Now", labels 12px muted, so the maker sees they
  differ.
- One button, **Pair again**, 244 x 28, `--destructive` look (border and label, fills on press). It opens
  the form of 3.1 for that record with the Pairing code field focused. No "Trust anyway".
- The pill (section 4) shows the state "Key changed" with `ShieldAlert`; Send, Frame, Jog, Home are
  `aria-disabled` with the reason line "Key changed." Stop stays enabled and tries (it also fails to
  connect, which is shown, section 8).

### 3.5 Delete

Delete is in the form (below the buttons, outline, label `--destructive`): the **Inline confirm** (two
buttons): "Delete Ortur LM2? Its pairing is forgotten here. Pair again on the connector to use it." This one
asks because Ctrl+Z does not take it back (the rule: a reversible action never asks). The connector's own
list of paired devices is not touched (`curvyo-connector unpair` does that).

## 4. Machine pill

A floating card, the look of the contextual bars: `--toolbar-bg`, `--panel-elevation-shadow`, `rounded-md`,
12px text. Part of a **status dock** (also used by 0092's collaboration pill): bottom right of the canvas,
inset 12px from the right and bottom edges, a column with 8px gaps, collaboration pill above, machine pill
below, `z-40` (above the notice slot's `z-30`, below any dialog). The pill is `role="group"`
`aria-label="Machine"`, below the canvas in tab order and above the status bar, so Tab reaches it after the
canvas and Shift+Tab leaves it again.

Row 1 (height 36, padding 4 8): state glyph 16px, the record name (14px, ellipsis, max 120px), the state
text (12px muted), then at the right the buttons: **Pause** or **Resume** (28px icon button, only while a
job runs or is held) and **Stop** (below). Click on the name area opens the Machine tab (shows the body if it
is hidden), focus to the first control; it is a `button` named "Machine, Ortur LM2, Idle".

Row 2 only while a job or a frame exists: the progress bar (6px) and "412 of 1,200 lines" (12px
`tabular-nums`, left) and elapsed "0:42" (right). Under 640px of canvas width the name and row 2's texts are
dropped; the glyph, the Pause/Resume and Stop remain (the name is in the accessible name).

**Stop** (Proposal for the look): 56 x 28px, glyph `Square` 14px filled plus the word "Stop", ground
`--stop-bg` (= `--destructive`), label `--stop-fg` (white), 6.5:1 on the ground. It is the only filled red
control in the app and stays that way. Focus ring: 2px `--editor-accent` with 2px `--toolbar-bg` inside, like
the strip. Pressing it acts on the press, with no confirmation, no delay (the Inline confirm's 400ms rule
does not apply here, criterion 25) and no dimming: enabled whenever the selected record is paired, in every
connection state. Pointer: acts on `pointerdown`. Keyboard: Enter, Space, and the chord of section 12.
Pressing Stop moves no focus.

Pill states (glyph, text; colour is never the only cue):

| State | Glyph | Text | Token |
|---|---|---|---|
| Not reachable / reconnecting | `Unplug` | "Not reachable" / "Reconnecting..." | `--panel-muted-fg` |
| Key changed | `ShieldAlert` | "Key changed" | `--destructive` |
| Idle | `Circle` | "Idle" | `--panel-muted-fg` |
| Run | `Play` | "Running" / "Framing" / "Sending" | `--editor-accent` |
| Hold | `Pause` | "Paused" | `--state-hold` |
| Jog | `Move` | "Jogging" | `--editor-accent` |
| Alarm | `TriangleAlert` | "Alarm 2" | `--destructive` |
| Home | `House` | "Homing" | `--editor-accent` |
| Door, Check | `DoorOpen`, `ClipboardCheck` | "Door open", "Check mode" | `--state-hold` |
| Waiting for local start | `Hourglass` | "Waiting at machine" | `--state-hold` |
| Controller lost | `Unplug` | "Controller lost" | `--destructive` |

Glyph colour is the token, text is always `--toolbar-icon` (readable on `--toolbar-bg`). A change of state
is announced once by a polite live region in the pill ("Ortur LM2: Paused"); the position and the progress
count are not live (no per-second chatter). Alarm, Controller lost, Key changed and a failed Stop are
announced `assertive`.

## 5. State block

Three lines, 12px, no card:

1. Connection: "Connected, GRBL 1.1h" / "Controller not answering" / "Connector not reachable". A paired record
   never connects before the pairing handshake of the app's start; "Connecting..." until the first answer.
2. State chip: the glyph and text of the table in section 4 at 14px, plus muted "not homed" next to Idle
   while the app does not know the machine is homed (after a connector start, criterion 33, and after
   controller lost).
3. Work position: "X 123.40   Y 56.00 mm", mono, `tabular-nums`, 14px, two decimals, not live-announced.
   While the position is unknown (not homed after power-up where the controller reports no valid position,
   controller lost) the digits are replaced by "X --   Y -- mm". Within 1 second of a change (criterion 11).

## 6. Move block: home, unlock, jog

Left column (92px): the **jog pad**, a 3 x 3 grid of 28px cells with 4px gaps, the four arrow buttons
(Lucide `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, 16px) at the plus positions; the centre cell is
**Home** (`House`), so the pad is the whole of "move". Corners empty. `role="group"` "Jog". Right column
(rest, 144px): the step choice, a `role="radiogroup"` "Step" of four items **0.1, 1, 10, 100** (the
`PanelRadioGroup` segmented look, 4 x 34px, unit "mm" written once in the group label's visible caption
"Step, mm"), 1 mm default, remembered per user. Below the pad, full width, **Unlock** (`LockOpen` + label
"Unlock", outline, 118 x 28) in the tree only in state Alarm; **Home** also has its own place in the pad
(always in the tree).

- Arrow directions are the machine's: Up = +Y (away from the operator), Right = +X, matching a front-left
  home as on typical diode lasers. **(Proposal, question JOG-DIR.)**
- Pointer or key on an arrow sends exactly one `$J=` with the chosen step and the record's jog speed
  (criterion 14). Arrow keys on a focused pad button do the same (and do not nudge the selection: focus is in
  the panel). Enter and Space on a pad button jog; repeat events are ignored.
- **Tap or hold:** a press shorter than 400 ms lets the step finish; a press held for 400 ms or more cancels
  the jog (0x85) on release, so the head stops where the key was let go (criterion 15). **(Proposal, question
  JOG-RELEASE.)**
- **Refusals** (visible reason, criterion 14): the pad keeps its Tab stops, buttons are `aria-disabled`
  (40% glyph) and a single reason line under the pad (12px `--panel-muted-fg`, `aria-describedby` of the
  group, `role="status"`) says it: "No jog while a job runs." / "No jog in Alarm. Unlock or home first." /
  "No jog while homing." / "Not connected." / "Key changed." A jog that would leave the work area is refused
  before anything is sent and the line shows "Beyond the work area on the left." (right, top, bottom); the
  pad is otherwise live. The line clears on the next press, state change or after 8 seconds.
- Home: a press while Idle sends `$H`; in other states `aria-disabled` with the same reason line. During
  Home the state chip reads Homing and a progress is not shown (duration unknown).
- **Alarm** banner (Banners block): `TriangleAlert` in `--destructive`, then "Alarm 2: Soft limit. The move
  would leave the work area." (code and plain text from the fixtures, criterion 12), nothing else. Home and
  Unlock are both visible in the Move block while it shows; Send, Frame and Jog read "Alarm." as the reason.

Pad keyboard order: Up, Left, Home, Right, Down (screen order, reading left to right, top to bottom), then
the Step group (one Tab stop, arrows move inside), then Unlock.

## 7. Job block, frame and the send statement

**File.** First control in the block: a text button "G-code file..." (118 x 28 outline, Lucide `FileCode`
16px before the label) until a file is chosen, then the file name (14px, ellipsis, tooltip with the path
on desktop) with a Change button ("Change...", 76 x 28). Desktop: the OS open dialog with filter "G-code
(*.gcode, *.nc, *.ngc, *.gc)" then "All files"; browser: the same button opens the file picker. Reading
and measuring the file (line count, bounding box, criterion 16) runs off the main thread; for a file over
1 MiB the line "Reading logo.gcode..." replaces the statement and the cursor is `wait` on the button only. A
file with no moves: the line "No moves found in logo.gcode." under the button (`role="alert"`, the file is
not loaded). The chosen file is session state; it is not stored in the project.

**Statement (R-SYS-008, criterion 18).** A `dl` of 12px rows, label column 64px `--panel-muted-fg`, value
`--toolbar-icon`, ellipsis with tooltip, always in this order, with no Send before it is shown:

| Label | Value |
|---|---|
| Machine | "Ortur LM2, 400 × 400 mm" |
| File | "logo.gcode" |
| Lines | "1,200" |
| Size | "120.0 × 80.0 mm at X 10.0, Y 12.0" (bounding box, X/Y of its lower left) |
| Material | "None selected" |

Material is always the last row; until the material library exists it is the literal "None selected" in
`--panel-muted-fg` (it is a statement, not a field). Without a file only the Machine row shows. The block is
the confirmation: there is no extra dialog before Send. It is the Send button's `aria-describedby`.

**Frame and Send.** Two buttons in one row, 118 x 28 each, 8px apart (36 coarse): **Frame** (outline,
`Frame` glyph) and **Send** (Apply look: `--toolbar-icon-active-bg` ground, white label, `Send` glyph).
Frame: sends the small frame job (criterion 16), pill reads "Framing", it never runs while a job runs, no
confirm. The head returning to the start point is the end of the frame; the line "Framed." (status, 8s)
shows in place of the reason line. Frame has no frame-power control here: it is in the record (3.1).

**Send** is `aria-disabled` (40%, keeps its Tab stop and tooltip, the exception recorded in the design
system) with one reason line under the row (12px, `--panel-muted-fg`, `aria-describedby`, `role="status"`).
Precedence, the first that applies: "Select a machine." / "Pair the machine first." / "Not connected." /
"Key changed." / "Alarm: unlock or home first." / "Home the machine first." / "Machine busy." /
"Choose a G-code file." / the overshoot "Outside the work area by 12.0 mm on the right." (several sides:
"Outside the work area: left 3.2 mm, bottom 10.0 mm", values in mm, only non-zero sides; Frame still
works, criterion 20). A refusal from the connector (second client, no machine record named, "machine busy",
criterion 28) shows the same line in `--field-invalid` with `role="alert"`.

Nothing is dimmed except by these reasons; no other disabled controls in the block.

## 8. Progress, pause, stop, failure

When Send is accepted the Frame/Send row is replaced in place by the Progress block, reserved at its
height so nothing below moves:

- Phase "Sending to the connector" (upload, ADR 0032): label with a percentage, then "Running". Both phases
  use the same bar.
- **Bar:** 6px, track `--toolbar-icon` at 15% with a 1px `--toolbar-icon` at 60% edge, fill
  `--editor-accent`. `role="progressbar"`, `aria-valuenow` = acknowledged lines, `aria-valuemax` = total,
  `aria-label` "Job progress". Text row: "412 of 1,200 lines" left, "0:42" right, 12px `tabular-nums`. Updates at
  least once per second (criterion 22); no animation (the bar jumps; none under `prefers-reduced-motion`).
  No time estimate in this slice.
- Buttons in the block: **Pause** (`Pause`, 118 x 28 outline) which becomes **Resume** (`Play`) in place,
  and **Stop** (the pill's red button, same look) beside it. Both duplicate the pill's controls and are the
  same state; focus is not stolen when the pill acts.
- State "Waiting for start at the machine": text "Waiting for start at the machine. 9:41 left." (counts
  down per second, polite only at 5-minute marks), Stop discards the job, Frame and Jog stay live
  (criterion 29). After the 10 minutes: Last job "Not started".
- **Link lost during a job** (criterion 30): the pill shows "Reconnecting..." with the last known progress
  at 60% and "last seen 14:03:41" in the tooltip; the Banners block "Connection lost. The machine keeps
  running the job." (with `on_link_lost = hold`: "...The machine paused the job."; the text comes from the
  connector's setting, no UI for the setting). On return the state is read and shown; no notice, nothing to
  click. Stop works from the new connection at once.
- **Stop failed** (criterion 25): the pill row is replaced by "Stop failed: no connection." in
  `--field-invalid`, `role="alert"`, and stays until the link returns, when the
  real state replaces it. It is never an 8-second notice.
- **Controller lost** (criterion 32): Banners block "Controller lost. The job failed and the position is
  unknown. Home the machine again." (`--destructive`, alert), the position reads "--", chip shows "not
  homed" after reconnect.
- **Machine busy by another device:** the progress block shows the job read from the connector ("Job from
  another device"); Stop works; Send reads "Machine busy."

**Last job line** under the block, replaced by the next job, cleared on selecting another record or on
quit: "logo.gcode: Done, 1,200 lines, 3:21" / "Stopped at line 412" / "Not started" / "Interrupted" (after a
connector restart, criterion 33). A failed job (criterion 27): "logo.gcode: Failed at line 88." and a second
line in mono 12px, ellipsis, tooltip: "G1 X999 Y10 / error 20: Unsupported command"; `role="alert"` once.

Close and quit never ask about a running job: the job lives on the connector. **(Proposal, question
QUIT-JOB.)**

## 9. Browser and desktop differences

| Aspect | Desktop (Tauri) | Browser |
|---|---|---|
| Machine tab key | Shift+Ctrl+M | none |
| Stop chord | Ctrl+. / Cmd+. | same |
| G-code file | OS dialog | file picker |
| Reaching the connector | `ws://` from the webview (spike item b decides if it holds on all three OSs) | only from `http://` or `localhost` pages (CONN-BROWSER default A) |
| Hosted `https` page | n/a | Machine tab body: the sentence "This page is served over https, so the browser blocks the connection to a machine on your network." The records, form and Pair are not shown; nothing else changes |
| Records and keys | local settings of the OS user | page storage; if unreadable, records and keys do not persist and the tab says "Machines cannot be stored in this browser." |

Export of the G-code file is out of this slice's UI (nothing produces one yet); it stays reachable "whatever
the connector's state" (criterion 38) because it is in no way tied to this tab.

## 10. Connector terminal output (headless, section "CONN-UI A")

Plain text, ASCII only, 80 columns, no colour unless the terminal asks for it and `NO_COLOR` unset (and then
only for the word `Alarm`/`error`), one fact per line, the same grouping and words as the app:

```
curvyo-connector 0.1.0
Machine    GRBL 1.1h on /dev/ttyUSB0 at 115200
Listening  192.168.1.40:8765
Key        7K2Q 9XMD 4HTR 2WVC

Pairing code  4821 7365   valid 5 minutes, one use
```

- With a paired device and no `pair`, the last two lines are absent (criterion 4). `curvyo-connector pair`
  prints the Pairing code line and nothing else new.
- `devices` prints a table: Name, Fingerprint, Paired (date), Last seen.
- A rejected attempt prints one line: "Pairing refused: wrong code (3 tries left)." The code itself and the
  G-code are never printed (criterion 10).
- Local start (criterion 29): `Job 'logo.gcode', 1,200 lines, from 'Marc laptop' is waiting. Press Enter to
  start or q to discard.` (the keys are part of the prompt, not a hint); after confirmation or the timeout:
  one line "Started." / "Discarded." / "Not started: no confirmation in 10 minutes."
- Job end: "Job 'logo.gcode' done: 1,200 lines in 3:21." (also the log line, criterion 3).

## 11. Tokens and components

Reused unchanged: `--toolbar-bg`, `--toolbar-icon`, `--toolbar-icon-active-bg/-fg`, `--editor-accent`,
`--editor-accent-hover`, `--panel-muted-fg`, `--destructive`, `--field-invalid`, `--panel-elevation-shadow`.
Components reused: `PanelStrip` tab, `FormatList`/`FormatForm` pattern, `ValueField`, `EntryField`,
`PanelRadioGroup`, Inline confirm, the outline text button, `Tooltip`, the canvas notice slot.

New (to be entered into `docs/design-system.md` when this ships; contrast to be measured in review):

| Token | Value | Used for |
|---|---|---|
| `--stop-bg` | `--destructive` (`#B3261E`), alias | the Stop button ground |
| `--stop-fg` | `#FFFFFF` | its label and glyph, 6.5:1 |
| `--state-hold` | `#7A4F00` | glyph colour of Paused, Door, Check, Waiting; about 5:1 on `--toolbar-bg` |
| `--progress-track` | `--toolbar-icon` at 15%, 1px `--toolbar-icon` at 60% edge | the progress bar track |

New components: **status dock** (shared with 0092), **Machine pill**, **jog pad**, **progress bar**,
**statement list**. Nothing else is new; no new tab chrome, no popup.

## 12. Keyboard map

| Action | Key | Notes |
|---|---|---|
| Stop | `Ctrl+.` (`Cmd+.` on macOS, the system's own "cancel") | **(Proposal, STOP-KEY; to be confirmed in the R-SYS-009 review.)** A page chord on every OS, fires on keydown in every focus including text fields, chips and the Pen, during a drag and while the panel is hidden, once per press (repeat ignored), never gated like the letters. Not reserved in Chrome or Firefox. Escape never stops. With no machine record selected: nothing. After the press the canvas notice slot shows "Stop sent." (status, 3s) for the keyboard route only |
| Show Machine tab | `Shift+Ctrl+M` (`Cmd`), desktop only | shows the body, first control focused |
| Jog | Arrows on a focused pad button; Enter, Space | section 6 |
| Pause / Resume, Frame, Send, Home | none | buttons only (YAGNI); Tab reaches the pill and the tab |
| Select a record | Up/Down, Enter or Space in the list | |

No Escape behaviour is added: in the tab, Escape returns focus to the canvas (existing); in the form it
cancels the draft.

## 13. Accessibility

- Every control has an accessible name; icon buttons have a text tooltip (400ms) and the same text as
  `aria-label` ("Jog left", "Home", "Unlock", "Pause", "Stop", "Frame", "Send").
- Targets 28px (36 coarse); the Stop button 56 x 28 (56 x 36 coarse). Focus rings as the strip.
- State is never colour alone: glyph plus text (section 4).
- Live regions: one polite for state changes, one assertive for Alarm, Controller lost, Key changed, Stop
  failed, a refused Send. Position and progress are not live. The progress bar is a real `progressbar`.
- Reasons for disabled controls are text in the tree and referenced with `aria-describedby`, not
  tooltip-only; `aria-disabled` keeps the Tab stop so the reason is reachable by keyboard.
- Reduced motion: no transitions on the pill, the bar or the banner; nothing animates in this feature.
- The fingerprint and code are plain selectable text (a screen reader reads them as groups of four).
- No timing the maker must beat: the pairing code's 5 minutes belong to the connector and the field shows
  the refusal text, nothing counts down in the app.

## 14. Empty and error states (checklist for the review)

No record; record unpaired (cannot occur after save; a record whose pinned key is lost shows "Pair again");
https browser; connector unreachable at start; unreachable during a job; key changed; wrong, expired, spent
code; controller lost; Alarm; no file; file unreadable or without moves; file outside the work area;
machine busy; connector refuses a job without a record; local start waiting and timed out; stop failed;
1,000,000-line file (reading is off the main thread, the Machine tab stays responsive, Stop works).

## 15. Open UX questions (each with a default; the lead puts them to the customer only if wanted)

- **MACHINE-PLACE:** Machine tab plus Machine pill (this note) / only the tab, with Stop in the status bar /
  a top bar. Default: tab plus pill; a status-bar Stop is too small and a top bar is a new toolbar.
- **STOP-KEY:** `Ctrl+.` (`Cmd+.`) / `Ctrl+Alt+S` / `F-key`. Default: `Ctrl+.`; layout independent on
  QWERTZ and QWERTY, no browser reservation known, matches the macOS cancel.
- **STOP-ALWAYS:** show Stop whenever a paired record is selected (this note) / only while a job or a
  connection exists. Default: always, so the control is never missing when it matters.
- **JOG-RELEASE:** tap completes the step, hold 400 ms or more stops on release / every release stops.
  Default: the first; the second makes a click on 100 mm useless.
- **JOG-DIR:** Up = +Y as the machine moves / Up = down on the screen's document. Default: machine
  directions, with a front-left home.
- **QUIT-JOB:** quitting or closing while a job runs does not ask (this note) / asks once. Default: does not
  ask; the job is on the connector.
- **DEVICE-NAME:** the connector's list of paired devices needs a name for this app. Default: the "Name shown
  to others" setting of 0092's Share tab; before that exists, the OS host name.
- **PAIR-TEXT:** one sentence under the code field telling where to find the code. Default: none (customer
  rule of 2026-10-16); ask only if testers cannot pair without it.
- **PLACEMENT-PREVIEW:** a small work-area diagram with the file's bounding box in the Job block. Default:
  no in this slice; the numbers are in the statement and the real preview comes with 0029.
- **MULTI-RECORD:** several machine records (this note, because the list is cheap and 0026 needs it) / one.
  Default: several.
