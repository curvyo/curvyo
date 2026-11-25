# 0118 Machine control, first slice: UX notes

ux-engineer, 2026-11-19. Covers the first slice of `specification.md` (criteria 1 to 7) and its "Needed before Ready" item for the ux-engineer: where the group sits in the 244px column, keyboard, the Home glyph and its states. Checked against `specs/0064-machine-connector/ux-notes.md` (Connector group, sections 2 to 10), `specs/0125-send-to-machine/ux-notes.md` (Output group button row and state line, sections 2 to 5), `specs/0127-connector-grbl-serial/ux-notes.md` (Controller block, Unlock, controller texts, sections 2, 4 to 9), `specs/0101-laser-operations/ux-notes.md` (the Operations tab read top to bottom), `specs/0129-fix-home-first-locked/specification.md` (homing failure texts, as built in `curvyo-connector-io/src/grbl/texts.rs` and `tests/grbl_home_first.rs`), the built `frontend/src/components/workbench/ConnectorGroup.tsx`, `frontend/src/components/panel/OperationsSection.tsx`, `OutputGroup.tsx`, `SendToMachine.tsx` and `PropertiesPanel.tsx` (the Operations column scrolls as a whole, `0103` 3.2), and `docs/design-system.md` (rows "Connector group (`0064`)", "Controller block (`0127`)", "Send to machine button (`0125`)", "Connector state line (`0125`)", "Hidden, not disabled"; Interaction conventions "No notice after a success", "Nothing moves under the pointer"). Customer rules applied: no popups, no dialog of our own (the OS file dialog only), symbol buttons with tooltip and accessible name, labels that name a control or a state only, no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31). Choices marked (Proposal) are vetoable at the demo; section 12 lists them as W1 to W15. One question for the customer: Frage 262 (section 6).

## 1. What changes where

- **Operations tab of a laser page** whose profile copy names a connector address: a new **Machine controls** group, a footer of the tab (section 2). It holds the job toolbar of the Connector row with **Home** added, the state line, the progress bar and the counts line, and **Unlock**. Same names, glyphs (one exception, W5), enabling rules, busy and failure texts as `0064` sections 3, 7, 8 and `0127` sections 4 to 7 (criterion 1).
- **Machine block, Connector group** (Document tab): loses the job toolbar and the job block (state line, bar, counts line, Unlock). Keeps the heading row with **Test connection**, the link line, the trust block with **Forget pairing**, the key-changed block, the Pair row and the Controller block's rows (section 9). The Connection fields (Connector address, Controller port, Baud rate) are untouched (criterion 2).
- **Output group** (`0125`): the button row (Export job file, Send to machine) is unchanged; its state line and bar leave by Frage 262 A, decided by the customer 2026-11-19 (the footer's line is the one line).
- Nothing else in the app changes. No new token, one new glyph use (`House`, and `FileUp` by W5).

## 2. Place: the footer of the Operations tab (criterion 1, the spec's "placement in the 244 px column")

Today the tab reads, top to bottom: Cuts last, the operation list (its own 300px scroll area), the Settings block of several selected rows, the Fixture jobs section, the Output group (machine line, Start pick, checks, Total, Export and Send). The whole column scrolls (`0103` 3.2). With a long list and fixture jobs, the end of the column is below the fold at 800 x 600.

**Machine controls is a fixed footer of the tab, outside the column's scroll (W1).**

- The column above keeps scrolling as a whole, exactly as today; the footer sits under it, at the bottom of the panel, always in view while the Operations tab is shown. Reason: Stop is the safety control; it must never be scrolled away while a job runs, and the maker reads the operations and the job state at once. Precedents: the History and Layers tabs keep a fixed footer under their list.
- The footer starts with the group's divider (the 1px Output-group `Divider`, 25% `--toolbar-icon`), 8px gap, then its parts. Ground `--panel-bg`; it shares the column's 12px side padding and the panel's 12px bottom padding.
- Order of the tab now reads the job's way: what runs (list), what else runs (fixture jobs), is it fine and send it (Output), move and run the machine (Machine controls). "Below the operation list and its summary" (spec) is met: the footer is the last part of the tab.
- Absent without a connector address (criterion 1): no footer, no reserved height; the column takes the whole tab as today. Other page kinds and "Laser (off)" pages have no Operations tab content for it.

**Layout of the footer, top to bottom (W2: the toolbar is the bottom row).**

1. **Heading** "Machine controls": `h3`, 11px semibold `--panel-muted-fg`, the Output group's heading look; 16px line.
2. **Settings warning** (only when one applies, section 7): at most two Notice-look lines.
3. **Job block** (only while a job or a state line is known): the state line with Unlock or the elapsed time at its right, the 6px progress bar, the counts line. `0064` section 8 look, unchanged, width 244px.
4. **Toolbar** (always while paired): one 28px row (36 coarse), the last row of the tab.

Everything that comes and goes (warning lines, job block, Unlock, longer wrapped texts) sits **above** the toolbar, and the footer is anchored at the panel's bottom: the footer grows upward, the column above gets shorter, and **the toolbar never moves**. This is the `0064` rule "nothing moves Stop under the pointer", kept now that the toolbar is no longer the top of its group. The heading moves up and down with the footer; it is not a control.

Heights (fine pointer): idle and paired 61px (divider, heading, toolbar); with a running job about 110px; worst case (two warning lines, a three-line alarm text, bar, counts) about 170px. At 800 x 600 the column above keeps at least 360px.

## 3. The toolbar (criteria 1, 3, 5)

`role="toolbar"` named "Machine", left-aligned, 28 x 28 buttons (36 coarse), 4px apart, an 8px gap after Home (W3): **Home** | **Send job...**, **Start / Resume**, **Pause**, **Stop**. 160px wide (200px coarse), so it fits the 244px column with room for the later jog and frame buttons, which join Home's group.

Every button is the `0064` symbol button: `rounded-md`, 16px Lucide glyph at 1.5px absolute stroke, `currentColor` (`--toolbar-icon`), hover `--editor-accent-hover`, focus ring 2px `--editor-accent` inside, tooltip (Radix, 400 ms) and accessible name the same text. Tooltip side `top` (the footer is at the panel's bottom; `left` would cover the canvas edge only, `top` covers the job block, which the button's own press changes; W4: `top`, as Send to machine).

| Control | Glyph | Name and tooltip | Enabled when |
|---|---|---|---|
| Home | `House` | "Home"; disabled: line 2 the cause (section 4) | section 4 |
| Send job... | `FileUp` (W5; was `Upload`) | "Send job..." | `0064` section 7 table, plus off while Homing |
| Start / Resume | `Play` | "Start" / "Resume" | `0064` section 7 table, plus off while Homing |
| Pause | `Pause` | "Pause" | `0064` section 7 table (off while Homing, except while a job runs its own `$H`; amended by 0133) |
| Stop | `Square` | "Stop" | `0064` section 7 table, **plus on while Homing** (criterion 5, `0127` G9) |
| Unlock | `LockOpen` | "Unlock" | `0127` section 6: only in Alarm, at the right of the alarm line in the job block, hidden otherwise |

- **Not possible = greyed in place**, `aria-disabled`, 40% opacity, Tab stop (within the roving toolbar) and tooltip kept, a press does nothing and sends nothing: the recorded `0064` exception to "Hidden, not disabled", now for the Machine controls toolbar.
- **Busy** as `0064` section 3: `aria-busy="true"`, `LoaderCircle` turning once a second (also under `prefers-reduced-motion`), from the press until the connector's answer or 5 s ("Timed out"); Pause, Resume, Stop and Unlock stay busy until the machine confirms (`0127` section 7). While one button waits, the others are greyed as built (not during a pending pause; amended by 0133).
- **Stop never asks, is never armed, and is never disabled while anything can move**: Running, Paused, AwaitingLocalStart, Ready (discards the held job), Homing. Escape never sends Stop or Pause; no global chord for any machine command (`0064` section 3; a laser must not move from a stray key).
- **Send job...** opens the OS file dialog as built (`0064` section 7): title "Send job", same filters, same refusals. Its press failures read in the group's state line.

## 4. Home (criteria 3, 4, 5)

**Glyph `House`** (Lucide, 16px). The house is the homing symbol makers know from LightBurn and machine pendants; it is not used anywhere else in Curvyo. Name and tooltip "Home", no "...", no shortcut text, no label.

**Enabled** in Idle, Ready, Done (Completed, Stopped, Failed: the controller is idle) and Alarm (GRBL's `$H` is its own way out of "Locked, not homed"). A Ready job stays held (criterion 3).

**Disabled, with its cause.** Home is the only toolbar button with a cause (criterion 5 names one); the others keep their name-only tooltips as built (W6). Tooltip two lines, the Send-to-machine pattern: line 1 "Home" (13px), line 2 the cause (12px `--panel-muted-fg`); the accessible name stays "Home", `aria-describedby` points to an sr-only span with the cause. The first cause that applies wins:

| # | When | Home | Cause (line 2 and description) |
|---|---|---|---|
| 1 | The connector reports Homing (this app's press or another app's) | busy | "Homing" |
| 2 | Running or Paused | off | "A job is running" (criterion 5) |
| 3 | AwaitingLocalStart | off | "Waiting for the start at the machine" |
| 4 | Receiving | off | "Sending" |
| 5 | NoController | off | the state line's text: "No controller", "Port /dev/ttyUSB0 not found", "No GRBL answer on /dev/ttyUSB0", ... (`0127` section 4) |
| 6 | Connection lost | off | "Connection lost" |
| 7 | Idle, Ready or Done while GRBL reports Door, Check or Sleep (the connector refuses Home there, `0127` G4; not in Alarm) | off | "Controller state: <word>", e.g. "Controller state: Door" (W15) |
| 8 | No status read yet | off | the read's own text: "Testing" while it runs, else its failure ("Nothing answers at <host:port>", "Timed out", "Connector error") |
| 9 | Another button of the group waits for its answer | off | none (as built for the job buttons) |

Each cause is a state, never an instruction. A cause change never moves the button or the focus.

**Press.** One press sends one `Home` command. Home is never sent by the app on its own, never on opening the tab, never on reconnect (criterion 3). Busy from the press until the connector answers (5 s, then "Timed out"); then, while the connector's status reads Homing, Home keeps the busy look (`LoaderCircle`, `aria-busy`, `aria-disabled`, cause "Homing"), driven by the status, not by the press, so a second app watching the same connector shows the same. There is **no time limit on homing** (`0127` G9 waits for `ok` without one): no "Timed out" while the head travels; Stop is the way to end it.

**During homing.** State line "Homing" (section 5). Send job..., Start / Resume and Pause are greyed; **Stop is enabled** and never asks (criterion 5). The Controller block's Position row (Machine block) keeps updating, not live.

**After homing.** The connector reports Idle: the "Homing" line leaves (no job) or returns to "Ready: <name>, <n> lines, <size>" (a held job, criterion 3), and Start is enabled again. No notice, no "Homed" line (no notice after a success); the line's change is the answer. The sr-only status announces the state word that follows ("Idle" or the Ready line) once (W7), so a screen reader user hears that homing ended.

**Stop during homing.** GRBL resets and alarms: "Alarm: Reset during homing (alarm 6)" (`0127` alarm table), alert, Unlock shown, Home enabled again. On the fake controller a held Ready job is kept; on GRBL the alarm 6 drops it, as every alarm does (architect addendum, M3; W8). Stop in Ready still discards.

## 5. State line texts (criteria 3, 4)

The job block's state line (`0064` section 8 frame, 12px, wraps, `role="status"`; alarm lines `role="alert"`). New rows for Home; all `0064`, `0125` and `0127` texts are unchanged and keep their looks.

| When | State line | Look | Live |
|---|---|---|---|
| Homing, no held job | "Homing"; the counts line names no job | neutral `--toolbar-icon`, no glyph | polite |
| Homing, a Ready job held | "Homing"; the counts line keeps the job's name | neutral | polite |
| Homing fails, limit switch not found (`ALARM:9`) | "Alarm: Homing did not find the switch (alarm 9)" | alarm (`TriangleAlert`, `--destructive`), Unlock | alert |
| Homing fails, pull-off (`ALARM:8`) | "Alarm: Homing pull-off did not clear the switch (alarm 8)" | alarm, Unlock | alert |
| Door opened while homing (`ALARM:7`) | "Alarm: Door opened during homing (alarm 7)" | alarm, Unlock | alert |
| Stop while homing (`ALARM:6`) | "Alarm: Reset during homing (alarm 6)" | alarm, Unlock | alert |
| Homing not enabled, `$22=0` (GRBL `error:5` on `$H`) | "Homing is not enabled (error 5)" | failure (`CircleAlert`, `--destructive`) | alert (answers the maker's press) |
| The connector refuses Home (a job started meanwhile) | "Connector busy" (existing text) | failure | alert |
| No answer to the press in 5 s | "Timed out" (existing text) | failure | alert |
| The controller is lost while homing | "Lost the controller" (existing `0127` text) | failure | alert |
| Amended by `0134`: a send and start from the page that did not start, while the job is still `Ready` | "Ready: <name>, <n> lines, <size>" and under it "Not started: <reason>" | the second line in the failure look (`CircleAlert`, `--destructive`) | alert, once, with the Ready line |
| Amended by `0134`: a member's start (`0128`) | "Waiting for <owner> to confirm" | neutral | polite |

- **The counts line names a job only while the connector holds it** (the status carries it), or next to the refusal of a chosen file (`0064` section 8). After an alarm dropped the job, and while homing without a job, it names none (UX review 2026-11-19, tester finding F2).
- **Texts reused word for word** from the connector's GRBL table as built for `0129` (`texts.rs`, `grbl_home_first.rs`), so a failed homing reads the same whether it came from the Home button or from a job's own `$H` (criterion 4). The only difference: outside a job there is no line number, so `error:5` reads "Homing is not enabled (error 5)" instead of "Failed at line 4: Homing is not enabled (error 5)". The alarm's first text stands until the alarm clears (`0127` V11).
- The `$22=0` failure stays until the next command or state change; GRBL stays Idle, so Home is enabled again at once (a second press gives the same text; nothing else to say).
- "The machine is left safe" (criterion 4) is the connector's job (`0127` G13); the line names the state only. No "re-home", no "check the switches".
- All texts are at most 60 characters, so they fit the 70-character line; with Unlock at the right the line wraps in the 212px left of it (never under it).

## 6. One state line, not two (Frage 262)

Both lines now sit in one tab, 40px to 100px apart: the Output group's line under Export and Send (`0125` S13) and the job block's line in the footer. They read one status object (S14), so they would always show the same words twice and a screen reader would hear each change twice (`0125` section 4 assumed the two sat in different tabs). The spec says the Output line "stays"; dropping it changes accepted behaviour, so it is the customer's call.

Frage 262 (MC-STATELINE): One state line in the Operations tab, or two?
- **A** One line: the Output group's state line and bar leave; the footer's job block is the one place for the job's state, including the results of a Send to machine press ("Sending <name>", "Ready: ...", "Connector busy", "Nothing answers at ...", "Timed out", "Not received", "Laser mode is off ($32=0)"). Send to machine keeps its disabled causes in its tooltip. Nothing is lost: every word moves 40px down, into the footer that is always in view.
- **B** Two lines with a split: the Output line shows only a Send to machine press's own result (Sending, refusals, "Not received", "Timed out") until the job is Ready; the footer shows the connector's state. Two places, but never the same words twice.
- **C** Two full lines as the spec says now: the same words in both places; the Output line's live region is silenced so it is heard once.

Recommendation: **A** (one source, one place, nothing read twice; the footer is in view whenever the Output group is). Default if unanswered: **A**. **A decided by the customer 2026-11-19.** With B or C the footer stays as these notes describe; only the Output group differs.

With A, the job block takes the `0064` form (state words, elapsed time, bar, counts line with the name) rather than the `0125` one-line form ("Running 120 / 412", " · <name>"): the counts line already names every job, also one sent by another app or by Send job....

## 7. Settings warning moves with Start (W9)

The two Notice-look lines of the Controller block ("Laser mode differs: profile off, controller on", "Max power differs: profile 1,000, controller 255"; `0127` section 2) sit where Start is pressed (`0127` V12). Start moves, so the lines move: they are part 2 of the footer, above the job block, in their own polite `role="status"` container (always in the DOM while the footer is, empty when nothing differs). They leave the Machine block; the Controller block's rows stay there (section 9). Same look: 12px `TriangleAlert` and text in `--toolbar-icon`, never `--destructive`, Start stays enabled.

## 8. Unpaired and unavailable states of the footer (criterion 1)

| Case | Footer |
|---|---|
| No connector address | absent |
| Browser build | heading and one 12px `--panel-muted-fg` line **"Needs the desktop app"**; no toolbar (nothing can apply in this build: hidden, not greyed) |
| Desktop, not paired | heading and the muted line "Not paired"; no toolbar |
| No longer paired | heading and "No longer paired" |
| Key changed | heading and "This connector's key changed" in the failure look (polite here; the alert is the Machine block's) |
| No credential store | heading and "No credential store on this computer" in the failure look |
| Paired | heading, toolbar, job block when a job or a state line is known |

- Each line names a state; there is no "pair in the Document tab", no link, no button to the Machine block (no operating tips). The Pair row stays in the Machine block, the one place for pairing.
- The footer and the Machine block sit in different tabs; only the mounted one reads and follows the status (the `0064` Q16 rule: read once when shown, follow the pushes while shown, retry every 5 s after "Connection lost" while shown).

## 9. The Machine block afterwards (criterion 2)

The Connector group keeps, in this order: heading row "Connector" with Test connection; the link line; the trust block (Paired with, Forget pairing, Key) or the key-changed block; the Pair row; the Controller block's rows (Controller, Port, State, Laser mode, Max power, Travel, Position). Nothing job-related remains: no toolbar, no state line, no bar, no counts line, no Unlock, no warning lines.

- **Link line**: test, pairing and status-read texts as built, plus "Connection lost". The failures that answer a job press ("The connector holds another job", a job command's "Timed out" or "Connector error") move to the footer's state line, the place of the press.
- **Controller block without a controller (W10).** Today the `NoController` causes ("No controller", "Port /dev/ttyUSB0 not found", "No permission for /dev/ttyUSB0", ...) are only in the job block's line, which leaves. So that the maker who types the port in this block also sees why it does not open, the Controller block shows, while the state is `NoController`, one row "Controller" with that text as value (the failure look for the four open failures, neutral for "No controller"), polite. The same text is the footer's state line and Home's cause.
- **Empty state.** With a paired connector, a controller and no job, the group is heading, trust block and Controller block: no placeholder text, no "No job" line. The words that remain all name a state or a value.
- **Focus after pairing** (`0064` section 6 sent it to Send job..., which is gone): Test connection (W11).
- Heights shrink by the toolbar and job block; the Fixtures block below moves up. Nothing above the Connector heading moves.

## 10. Keyboard and focus

- **Tab order of the tab:** the column (Cuts last, the list, the Settings block, Fixture jobs, the Output group's Start pick, check buttons, Export job file, Send to machine), then the footer: Unlock (when shown, it sits above the toolbar), then the toolbar (one Tab stop). Shift+Tab walks back the same way. The visual order and the Tab order agree.
- **Toolbar:** roving focus, Left and Right move, Home and End jump to the first and last button, no wrap; greyed buttons are reached too (their tooltip names them, Home's with its cause). Enter or Space presses. The first stop on entry is Home, after that the last focused button (as built).
- A button that a state change greys keeps the focus (`0064` section 10); Home keeps the focus through its whole busy time. When the alarm clears while Unlock has the focus, the focus moves to the toolbar's current button (as built, `0127` section 6).
- Tooltips appear on keyboard focus as on hover. No new shortcut, no global chord; Escape keeps its panel meanings.
- Hit targets 28px, 36px with a coarse pointer (the toolbar is then 200px wide).

## 11. Accessibility

- The footer is a `section` labelled by its heading "Machine controls"; the toolbar is named "Machine" (so the two names are not read twice).
- **Two hidden live regions carry the line** (UX review 2026-11-19): a `role="status"` and a `role="alert"` paragraph, sr-only, in the tree while paired, repeat the state line (polite or alert by the rules below) and the W7 word; the visible line has no live role. So a line that appears with the job block ("Homing" after Idle) is read, a polite line that becomes an alert is read once, and a refusal of the maker's own press is not read twice. The settings warning's region stays in the accessibility tree while empty (no `display: none`).
- **Live regions as built:** the state line polite; alarm lines, "Lost the controller ...", "Could not confirm the machine stopped" and failures of the maker's own press are alerts; the counts line, bar, elapsed time and Position are not live. "Homing" is announced once (polite); the state word after homing once (W7).
- **Home's state** is in its name and description: "Home", `aria-disabled`, described by the cause; during homing `aria-busy="true"` and the cause "Homing".
- **Not colour alone:** failures `CircleAlert`, alarms `TriangleAlert`, every state also in words; greyed buttons at 40% carry their names.
- **Contrast:** existing tokens only: `--toolbar-icon` 8.2:1, `--panel-muted-fg` 5.0:1, `--destructive` 4.8:1 on `--panel-bg`; bar fill 3.3:1 to the panel, 4.5:1 to its white track.
- **No popups, no dialog** (the OS file dialog of Send job... only), no notice after a success, no hint text, no connector action is a History step.
- **Panel collapsed or another tab chosen:** the footer is not shown, as the Connector row was not outside the Document tab. One place for a command that moves the machine (`0125` S15) stays one place.

## 12. Decisions for veto (Proposal)

- **W1** Machine controls is a fixed footer of the Operations tab, outside the column's scroll, so Stop is always in view while the tab is shown.
- **W2** The toolbar is the footer's bottom row; warnings and the job block grow upward above it, so the toolbar never moves.
- **W3** Toolbar order Home | Send job..., Start / Resume, Pause, Stop, with an 8px gap after Home for the later jog and frame buttons.
- **W4** Toolbar tooltips open on top (the footer is at the panel's bottom).
- **W5** Send job... takes `FileUp` instead of `Upload`: Send to machine (`Upload`, this page's job) and Send job... (a file) now sit in one tab and must not look alike.
- **W6** Only Home has a cause line; the other toolbar buttons keep their name-only tooltips (amended by 0133: Pause's pending line "Pause after homing").
- **W7** After homing, the sr-only status says the next state word ("Idle", or the Ready line) once; no visible text.
- **W8** Stop during homing stops the homing only. A held Ready job is kept on the fake controller only; on GRBL the reset raises alarm 6 and the alarm drops the job (architect addendum).
- **W9** The Controller block's two settings warnings move to the footer, above the job block.
- **W10** The Machine block's Controller block shows a "Controller" row with the `NoController` cause.
- **W11** Focus after pairing goes to Test connection.
- **W12** No time limit and no "Timed out" while homing; only the command's answer has the 5 s limit.
- **W13** Home is enabled in Done and in Alarm.
- **W14** The footer's unpaired states show one state line and no toolbar.
- **W15** Home is greyed with the cause "Controller state: <word>" while GRBL reports Door, Check or Sleep outside Alarm, because the connector refuses Home there (UX review 2026-11-19, tester finding F1).

## 13. For the product owner and the architect

- **Criterion 1 "identical glyphs"** conflicts with W5 (`FileUp` for Send job...). If W5 stands, criterion 1 names the one exception.
- **Criterion 4 and `0129` criterion 2** (aligned by the PO, 2026-11-19) quoted "Homing failed: limit switch not found" and "Homing is not enabled ($22=0)"; the built texts are "Homing did not find the switch (alarm 9)" and "Homing is not enabled (error 5)" (section 5). These notes use the built ones; the PO aligns both specs.
- **Criterion 5** names one cause; section 4 adds "Homing", "Waiting for the start at the machine", "Sending", the `NoController` texts, "Connection lost" and the status-read texts (new uses of existing words).
- **Criterion 3** "Idle, Alarm, or Ready": W13 adds Done. AwaitingLocalStart is off.
- **Architect:** Homing must be a state the connector reports to every client (the busy look follows the status, not the press, section 4); the answer to `Home` comes at once, not after `ok` (the 5 s answer limit, W12); W8 (Stop in Homing keeps a Ready job on the fake only; on GRBL alarm 6 drops it, settled in the addendum); GRBL 1.1 accepts `$H` in Alarm (W13; if a GRBL needs `$X` first, Home is off in Alarm with the cause "Alarm" and the PO is told, as `0129` criterion 3 says).
- **`0125` S13 and S15, `0127` V12** are rewritten by this slice per Frage 262 and W9.

## 14. Design-system rows (criterion 7; to land with the build)

- **New row "Machine controls (Operations tab) (`0118`)"**, after "Send to machine button (`0125`)": "The fixed footer of a laser page's Operations tab while the profile names a connector address, outside the column's scroll, anchored to the panel's bottom; divider, `h3` 'Machine controls', the settings warning (Notice look, polite), the job block (`0064` state line with Unlock or elapsed time, 6px bar, counts line), and as its bottom row the toolbar `role="toolbar"` named 'Machine': Home `House` | Send job... `FileUp`, Start/Resume `Play`, Pause, Stop `Square`, 28px (36 coarse), 4px apart, 8px after Home. Everything that comes and goes sits above the toolbar, so the toolbar never moves. Home: enabled in Idle, Ready, Done, Alarm; otherwise `aria-disabled` at 40% with a two-line tooltip (name, cause: 'Homing', 'A job is running', 'Waiting for the start at the machine', 'Sending', the NoController text, 'Connection lost', the status-read text); busy with `LoaderCircle` while the connector reports Homing, no time limit; state line 'Homing'; failures 'Alarm: Homing did not find the switch (alarm 9)', '... pull-off did not clear the switch (alarm 8)', '... Reset during homing (alarm 6)', 'Homing is not enabled (error 5)'. Stop is enabled while homing and never asks. Unpaired: heading and one state line ('Not paired', 'No longer paired', 'This connector's key changed', 'No credential store on this computer'); browser: 'Needs the desktop app'. No shortcut, no notice after a success, no History step." Reference: `specs/0118-machine-control/ux-notes.md`.
- **"Connector group (`0064`)"** amended: "Since `0118` the job toolbar, the job block and Unlock live in the row 'Machine controls (Operations tab)'; the Connector group keeps the heading with Test connection, the link line, the trust or key-changed block, the Pair row and the Controller block; focus after pairing goes to Test connection." The `0127` words stay listed, with "(shown in the Machine controls footer)".
- **"Controller block (`0127`)"** amended: the warning lines move to the footer (W9); the block gains the "Controller" row with the `NoController` cause (W10).
- **"Connector state line (`0125`)"**: with Frage 262 A, replaced by a pointer to the Machine controls row; with B or C, amended as answered.
- **"Hidden, not disabled"**: the `0064` exception now reads "the Test connection and Pair buttons of the Connector group, and the toolbar of the Machine controls footer, Home with its cause in the tooltip's second line (`0118`)".
- **"Send to machine button (`0125`)"**: "the glyph of the Connector row's Send job..." becomes "`Upload`; Send job... in the Machine controls footer takes `FileUp`".
- No new token.

## Links

Spec: `specs/0118-machine-control/specification.md`. Built notes: `specs/0064-machine-connector/ux-notes.md`, `specs/0125-send-to-machine/ux-notes.md`, `specs/0127-connector-grbl-serial/ux-notes.md`, `specs/0101-laser-operations/ux-notes.md`. Code: `frontend/src/components/workbench/ConnectorGroup.tsx`, `frontend/src/components/panel/OperationsSection.tsx`, `frontend/src/components/panel/OutputGroup.tsx`, `frontend/src/components/panel/PropertiesPanel.tsx`, `frontend/src/lib/connectorText.ts`, `curvyo-connector-io/src/grbl/texts.rs`.

## Second slice 2a (jog)

ux-engineer, 2026-11-23. Covers criteria 8 to 21 and 37 of the second slice with the answers Frage 308 A, 309 A, 315 A. Checked against sections 2 to 12 above (W1 to W15), `0128` ux-notes 8.2 and 8.3 (Confirm slot, "Waiting for <owner> to confirm"), `0132` ux-notes (job list, rights drawn, not greyed), the built `MachineControls.tsx`, `CornerControl.tsx`, `texts.rs` and the design-system rows "Machine controls", "Move entry chip" (segment pill), "Origin corner control". No hint labels, no popups, no notice after a success. New choices W16 to W27, (Proposal).

**Jog toggle (criterion 15).** Toolbar, in Home's group: **Home**, **Jog** (4px) | 8px | Send job..., Start / Resume, Pause, Stop. Glyph `Move`, name and tooltip "Jog", `aria-expanded`, `aria-controls` the pad; open = `--toolbar-icon-active-bg` / `-fg` (the Link-corners toggle look). Opening moves nothing, so the toggle is never greyed while paired, also during a job (criterion 20). Toolbar width 192px (240px coarse), within 244px. Open or closed and the step are an app setting on this computer (not in the project, not synced); default closed, 1 mm (W16).

**Pad (W17).** The row directly above the toolbar, below the job list, 8px above it; the toolbar never moves (W2), the column above gets shorter. `role="group"` named "Jog". Left: a 3 x 3 grid of 28px cells (36 coarse), 4px gaps, 92 x 92px (116 coarse): up, left, right, down arrow buttons, the centre and corners empty. Right, 12px on: label "Step (mm)" (12px `--panel-muted-fg`, names the control), 4px, then the step pick: a four-segment pill "0.1 | 1 | 10 | 50" (Move-entry-chip look, 28px high, 36 coarse, active segment `--toolbar-icon-active-bg`), `role="radiogroup"` "Step", radios named "0.1 mm", "1 mm", "10 mm", "50 mm"; under it, only while not homed since connect, the state label "Not homed" (12px muted, not live, W18). With a coarse pointer the arrows (116px), the gap and the pill (146px) need 274px, more than the column: the step pick wraps under the arrows, 12px below them (UX review 2026-11-23). Heights: idle with pad 161px (fine), running job about 210px, worst case about 270px; at 800 x 600 the column above keeps about 260px (W19). Pad closed: not in the DOM.

**Arrows (criterion 16).** Glyphs `ChevronUp`, `ChevronLeft`, `ChevronRight`, `ChevronDown` (not `Arrow*`, which Job order uses 40px above), the 0064 symbol button. A glyph is the direction **on screen**, so the head moves the way the canvas shows the bed; its name and tooltip is the machine axis it drives, per the profile's origin corner (W20):

| Origin corner | up | down | left | right |
|---|---|---|---|---|
| Bottom left | "Y+" | "Y-" | "X-" | "X+" |
| Top left | "Y-" | "Y+" | "X-" | "X+" |
| Bottom right | "Y+" | "Y-" | "X+" | "X-" |
| Top right | "Y-" | "Y+" | "X+" | "X-" |

**Press and hold (criteria 16, 18, 19; W21).** Pointer down: the button takes the pressed look (`--toolbar-icon-active-bg` / `-fg`) at once, its tooltip closes and stays closed while held, `setPointerCapture`. Up within 300 ms: one step of the chosen size. Held past 300 ms: continuous jog; the pressed look stays the whole hold (no spinner, no animation; the arrow glyph never changes). The hold ends at the first of: pointer up anywhere, the pointer leaving the button's box, `pointercancel`, `lostpointercapture`, window blur, the document hidden, a tab, page or panel change, the pad closing, the connection lost; ending sends the release at once, the keep-alive timeout is the connector's fallback (criterion 18). Only the first pointer counts; other pointers on the pad are ignored while one holds. A press during a step queues one step (criterion 19), more are dropped without a word. A press at the travel edge writes nothing; the button is not greyed for it in 2a (W22).

**Visual feedback from the connector.** While the connector reports a jog the state line reads "Jogging" (neutral, no glyph, no bar, counts line empty), in every app on that connector; Home, Send job..., Start / Resume (and Frame in 2b) are greyed with the cause "Jogging"; Stop stays enabled and never asks (criterion 10). "Jogging" is not put into the live regions (each step would be read; the press is the maker's own, W23). `error:15` reads "Jog beyond the travel (error 15)" (failure look, alert: it answers the press). After the jog the line returns to the state it came from, no notice.

**Keyboard (criterion 11; W24).** Tab order: ... job list, the pad's arrows (one Tab stop, roving), the step pick (one Tab stop), the toolbar. Inside the arrows the arrow keys move the **focus** to the button in that direction on screen and never move the head (no arrow-key jog, Out of scope); Home/End are not used. Enter or Space on a focused arrow: one step on key down; key repeat (`event.repeat`) is ignored. Space held past 300 ms: continuous, ended by key up, Escape, Tab, blur or any end of the list above; Enter never holds. In the step pick Left/Right change the value (radiogroup). Opening the pad keeps the focus on Jog; closing it with the focus inside sends the focus to Jog. No shortcut, no global chord.

**Touch.** `touch-action: none` and no text selection on the pad, the long-press context menu is suppressed on the arrows, 36px targets with a coarse pointer; no haptics, no sound.

**Disabled arrows (criteria 8, 20).** Greyed in place (the W-section-3 exception, now also the pad's arrows), `aria-disabled`, 40%, two-line tooltip (axis name; cause) and sr-only description; the pad stays open, nothing is written. First cause wins: "Connection lost" and the status-read text (the last status is stale then); "Homing"; "Jogging" (another app's jog); "A job is running" (Running, Paused); the waiting texts as the state line ("Waiting for the start at the machine", "Waiting for Marc to confirm", "Home requested by Phone"); "Sending"; "Alarm"; the NoController text; "Controller state: <word>". Laser mode and "Not homed" are no causes (the laser is off for a jog; an unhomed jog runs unclamped, criterion 17). The step pick is never greyed.

**Member (criteria 12, 37; Frage 308 A; W25).** A member's app has **no Jog toggle and no pad** (rights drawn, not greyed, as the `0132` job list); its toolbar is Home | Send job..., Start / Resume, Pause, Stop as in slice 1. An owner's jog shows "Jogging" in the member's line and greys its Home with that cause.

| Moment | Member app (requester) | Other member | Owner app |
|---|---|---|---|
| Member presses Home | line "Waiting for Marc to confirm" (polite); Home greyed, cause the same | "Phone waits for Marc to confirm"; Home greyed | line "Home requested by Phone" (polite); the Home slot reads **"Confirm home"** (glyph `House`, enabled, `aria-describedby` the state line); Start greyed "Start" |
| Owner presses Confirm home, or `curvyoctl confirm-start` | "Homing", as slice 1 | "Homing" | "Homing", Confirm home turns to the busy Home |
| Stop from anyone | own Stop: no line; else "Not started: Home" (muted) until the next state change | no line | no line |

No owner label: "an owner" ("Waiting for an owner to confirm"). Foreground `--require-local-start`: "Waiting for the start at the machine" for Home in every app (as built), Jog runs at once. The owner's arrows during a waiting Home are greyed with "Home requested by Phone" (W26); the Jog toggle is not, as it moves nothing (criterion 15). Confirm home is not armed (`0128` V15) and keeps the focus through its busy time.

**Accessibility.** Pad `role="group"` "Jog"; arrows named by axis, with `aria-disabled` and the cause as description; no `aria-pressed` on arrows (they are not toggles); the hold is not announced; the step pick a radiogroup with `aria-checked`. Contrast: existing tokens only (pressed `--toolbar-icon-active-fg` on `-bg` as the Link corners toggle). Focus ring as the toolbar. Not colour alone: the held arrow also has the "Jogging" line.

**Decisions for veto (Proposal).** **W16** pad open/closed and step are an app setting, default closed and 1 mm. **W17** the pad is the row above the toolbar; the toolbar never moves. **W18** "Not homed" shows in the pad only while not homed; homed shows nothing. **W19** about 260px left for the column at 800 x 600 in the worst case. **W20** arrows by screen direction, names by machine axis per origin corner. **W21** the hold ends when the pointer leaves the button, not only on release. **W22** no greying at the travel edge in 2a. **W23** "Jogging" is not announced. **W24** Space can hold, Enter cannot; arrow keys move focus only. **W25** a member has no Jog toggle (drawn, not greyed). **W26** an owner cannot jog while a member's Home waits. **W27** the owner's Home slot reads "Confirm home" while a member's Home waits.

**For 2b.** Frame in Home's group makes the coarse toolbar 280px (over 244px); 2b's addendum places Frame and its pick (e.g. in the pad, or a second toolbar row) before it is built.

## Second slice 2b (frame)

ux-engineer, 2026-11-23. Covers criteria 22 to 28 with the answers Frage 308 A (a member's Frame waits for an owner), 310 C (laser off), 311 A (Rubber band = convex hull, Rectangle default). Checked against sections 2 to 11, the 2a addendum (W16 to W27), `0029` ux-notes 5 (the head of the play bar), `0128` ux-notes 8.2 and 8.3, the built `MachineControls.tsx` and `JogPad.tsx`, the design-system rows "Machine controls", "Jog pad", "Move entry chip". No hint labels, no popups, no notice after a success. New choices W28 to W38, (Proposal).

**Place: a Frame row above the toolbar (W28).** Frame in Home's group (criterion 22's wording) makes the coarse toolbar 280px, and the pick adds about 170px more. So Frame and its pick get a row of their own, **directly above the toolbar**; the toolbar stays the bottom row, unchanged (Home, Jog | Send job..., Start / Resume, Pause, Stop; 192px, 240 coarse), so Home, Jog and Stop never move. Footer order, top to bottom: heading, settings warning, job block, job list, jog pad (when open), **Frame row**, toolbar, each 8px apart. The pad now opens above the Frame row, which does not move when the pad opens or closes (amends W17's "directly above the toolbar"). The row is in the footer whenever the toolbar is (paired, owner and member apps alike); the browser build has no toolbar and no row (section 8: heading and "Needs the desktop app", criterion 27). Heights (fine pointer): idle 97px; pad open 197px; running job about 246px; worst case about 306px; at 800 x 600 the column above keeps about 224px (amends W19's 260px).

**Frame row (W29).** `role="group"` named "Frame", left-aligned: **Frame** (the `0064` symbol button, 28px, 36 coarse, glyph Lucide `Scan`, not used elsewhere in Curvyo; `Square` is Stop, `SquareDashed` is Guide), 8px, then the **frame pick**: a two-segment pill "Rectangle | Rubber band" (Move-entry-chip look, as the step pick: 28px high, 36 coarse, 12px text, 8px side padding, 10 coarse, active segment `--toolbar-icon-active-bg` / `-fg`), `role="radiogroup"` named "Frame shape", radios "Rectangle" and "Rubber band". About 197px wide (213 coarse), within 244px. No label: the segment words name the values. Frame's name and tooltip "Frame" (tooltip `top`, 400 ms, no shortcut text).

**Pick (criterion 23; W30).** Session state per page: default Rectangle; each laser page keeps its own value while the app runs; not saved in the project, not synced, not an app setting, not a History step, no notice; lost on reload. Never greyed (it moves nothing); a change during a frame applies to the next one. Read at the press, with the page's Start pick and the job as it is then.

**Enabled (criteria 8, 22, 25, 27).** Frame is enabled in Idle, Ready (the job stays held) and Done only. Greyed in place (`aria-disabled`, 40%, Tab stop kept, two-line tooltip "Frame" / cause, sr-only description), first cause wins (W31): "Connection lost" and the status-read text; "Homing"; "Jogging"; "A job is running" (Running, Paused); the waiting texts as the state line ("Waiting for the start at the machine", "Waiting for Marc to confirm", "Phone waits for Marc to confirm", "Home requested by Phone"); "Sending"; "Alarm"; the NoController text; "Controller state: <word>"; then the page's causes "Nothing to frame" (no item in a laser operation; an operation without power or speed still frames, Frage 335 B) and "Outside the work area" (the `0029` 4 b gate; not with Head position); a button of the group waiting for its answer: no cause, as built. Laser mode and "Not homed" are no causes (the laser is off; an unhomed frame traces where the job would burn). Connector causes come first because they are shared by every button and the page's causes need an edit to change.

**Press (criteria 22, 24).** One press sends one frame; nothing frames on its own (criterion 11). Busy (`LoaderCircle`, `aria-busy`) from the press until the answer (5 s, then "Timed out"); then, while the connector reports Framing, Frame keeps the busy look driven by the status, in every app on that connector, no time limit (as Home, section 4). A second press while busy does nothing.

**State line "Framing" (criterion 26; W32).** Neutral `--toolbar-icon`, no glyph, no bar; the counts line keeps a held Ready job's name, else names no job (a frame is never a job, criterion 22). Announced once (polite), and the state after it once (W7), because one press starts it and it lasts seconds (unlike "Jogging", W23). During Framing: Home, the pad's arrows, Send job..., Start / Resume greyed with the cause "Framing"; Pause greyed as built; the Jog toggle and both picks stay enabled; **Stop enabled, never asks**. After the frame the line returns to the one it showed before ("Ready: ...", a Done line, or none); no "Framed", no notice.

**Stop and failures (criterion 10).** Stop mid-frame: the line returns to the state it came from (Ready with the job kept); no line for the one who pressed Stop or for the other apps. Texts as built, word for word: "Connector busy" (a job started meanwhile), "Timed out", "Lost the controller", the alarm table (door, hard limit; Unlock shown; on GRBL the alarm drops a held job, M3), GRBL `error:<n>` answers without a line number (as Home, section 5). All failures answer the maker's press: alert.

**Head ring on the canvas (criterion 26; W33, W34).** In the app that sent the frame, on the page it framed: the `0029` section 5 head (16px ink ring, 2px, 4px white casing, 2px ink dot), at `Status.position` mapped to the page by the origin corner and the Start pick, drawn over the page whether the Toolpath view is pressed or not. It jumps to each status push (no tween, no trail, no drawn outline); it appears with the first Framing status and leaves at once when the state leaves Framing (end, Stop, alarm, loss). Screen-constant, `aria-hidden`, `pointer-events: none`; the view never pans or zooms to it. While it shows, the play bar's own head is not drawn and Play pauses, so one ring means one head (W34). Other apps, including an owner who confirms a member's frame, draw no ring (they do not hold the requester's page placement).

**Member (criterion 12; Frage 308 A; W35).** A member's app has the Frame row (Frame waits, it is not refused); the frame is built from the requester's page and pick.

| Moment | Member app (requester) | Other member | Owner app |
|---|---|---|---|
| Member presses Frame | line "Waiting for Marc to confirm" (polite); Frame and Home greyed, cause the same | "Phone waits for Marc to confirm"; Frame and Home greyed | line "Frame requested by Phone" (polite); the Frame slot reads **"Confirm frame"** (`Scan`, enabled, `aria-describedby` the state line); Home, the arrows and Start greyed "Frame requested by Phone" |
| Owner presses Confirm frame, or `curvyoctl confirm-start` | "Framing", the ring on its page | "Framing" | "Framing", Confirm frame turns into the busy Frame |
| Stop from anyone | own Stop: no line; else "Not started: Frame" (muted) until the next state change | no line | no line |

No owner label: "an owner". Foreground `--require-local-start`: "Waiting for the start at the machine" for Frame in every app (Enter at the machine starts it). While a member's Home waits, the owner's Frame is greyed "Home requested by Phone" (the Home slot reads "Confirm home", W27). Confirm frame is not armed (`0128` V15), confirms the member's frame and not the owner's page, and keeps the focus through its busy time (W36).

**Keyboard (criterion 11; W37).** Tab order: ... job list, the pad (arrows, step pick), **Frame**, **the frame pick** (one Tab stop), the toolbar. Enter or Space on Frame: one press on key down, key repeat ignored. In the frame pick Left/Right move and pick (radiogroup, selection follows focus, no wrap). No shortcut, no global chord, Escape keeps its panel meanings. A button greyed by a state change keeps the focus (section 10).

**Accessibility.** Row `role="group"` "Frame"; Frame named "Frame", `aria-disabled` with the cause as description, `aria-busy` while busy; the owner's slot named "Confirm frame"; the pick a radiogroup with `aria-checked`. The ring is not announced; "Framing" is (once). Contrast: existing tokens only. Not colour alone: the active segment also has `aria-checked` and the state line names Framing.

**Decisions for veto (Proposal).** **W28** Frame and its pick sit in a row of their own above the toolbar, not in Home's group (criterion 22's "in Home's group" to be amended by the PO). **W29** glyph `Scan`; the pick is a word pill without a label. **W30** the pick is per-page session state, lost on reload. **W31** connector causes before the page's causes. **W32** "Framing" is announced once. **W33** the ring only in the sending app, only while Framing, no drawn outline. **W34** the play bar's head hides and Play pauses while the ring shows. **W35** a member's app keeps the Frame row. **W36** the owner's Frame slot reads "Confirm frame" while a member's frame waits. **W37** Frame takes its own Tab stop between the pad and the toolbar. **W38** no line after Stop mid-frame.

## Second slice 2c (origin and start-from)

ux-engineer, 2026-11-24. Covers criteria 29 to 33 with the answer Frage 312 A (the user origin is kept on the page in machine millimetres; nothing is written to the controller). Checked against sections 2 to 11, the 2a addendum (W16 to W27), the 2b addendum (W28 to W38), `0029` ux-notes 2 to 6 and 10 (Toolpath look, play bar head, Start pick, checks, texts), `0026` ux-notes (origin corner marker), `0015` (rulers measure from the page's top-left, unchanged), `0140` criteria 11 to 13 (anchor, "Outside the work area by 12 mm", reference crosses), `0121` (Take head position, `LocateFixed`), the built `JogPad.tsx`, `FrameRow.tsx`, `frameControls.ts`, `OutputGroup.tsx`, and the design-system rows "Jog pad", "Frame row", "Origin corner control and marker", "Take head position", "Laser canvas marks". No hint labels, no popups, no notice after a success. New choices W39 to W52, (Proposal); one question, Frage 345.

**Set origin here: place (criterion 29; W39).** The **centre cell** of the jog cross, between the four chevrons (the cell is empty today, so the pad keeps 92 x 92px, 116 coarse, and no height or width changes anywhere: the footer heights of 2b and the 224px left for the column at 800 x 600 stand). "Here" is where the head stands, and the centre of the arrows is where a pendant puts it. The `0064` symbol button, 28px (36 coarse), glyph Lucide **`LocateFixed`**, the glyph of Print and cut's Take head position (`0121`), which also stores the head's position; name and tooltip "Set origin here" (tooltip `top`, 400 ms, no shortcut text). The step pick, "Not homed" and the Frame row do not move. A member's app has no pad (W25), so no Set origin here (W52).

**Enabled (criterion 29; W40).** The press only reads the last status and edits the page: no byte goes to the connector or the controller (Frage 312 A). So it follows Take head position (a read sends nothing), not the motion rules of criterion 8: enabled in Idle, Ready, Done, Receiving, Paused and while a member's command waits, whenever the status carries a machine position and homed since connect. Otherwise greyed in place (`aria-disabled`, 40%, Tab stop kept, two-line tooltip "Set origin here" / cause, sr-only description); first cause wins: "Connection lost" and the status-read text (the status is stale); the NoController text; "No position from the machine" (a status without a machine position); "Machine is moving" (Running, Homing, Jogging, Framing; the `0121` word); "Alarm"; "Controller state: <word>" (Door, Check with its simulated position, Sleep; as Take head position); "Not homed" (criterion 29: the machine position is not trusted). Laser mode, the Start pick and the page's checks are no causes. Never busy.

**Press and what confirms it (criteria 29, 30; W41).** One press stores the head's machine position as the page's user origin and sets the Start pick to User origin, **one undo step "Set origin here"** (the one control of the footer that is a History step: it edits the page, not the machine). No notice, no line in the state line. What shows it, in the same frame: the user-origin cross appears on the canvas under the head's place (below), the Start pick in the Output group reads "User origin", and the X and Y fields under it show the numbers. A screen reader hears it once from the footer's sr-only status region: "User origin X 120.00 mm, Y 85.50 mm" (polite, as the W7 word; nothing visible). A press at the stored point (within 0.01 mm) with User origin already picked changes nothing and writes no step. Focus stays on the button. The press does not touch a job the connector holds (Ready): that job runs as it was sent; the next Send to machine or Frame uses the new origin.

**Start pick (criterion 30; W42).** Four values in this order: Machine origin, Head position, Home, then machine origin, **User origin**. "User origin" is the last option and is **absent until a point is stored** (nothing to pick yet; it appears at the end, so no option above it moves). Plain words, no greying, the `0029` 6.3 inline list; a pick writes one undo step "Start: User origin". Picking another value keeps the stored point, so User origin can be picked again with the same point. While User origin is picked, the **User origin row** sits under the Start pick, 4px below it, inside the Output group's scroll (W43): `role="group"` "User origin", two number fields "X" and "Y" (label inside left, display unit inside right, two decimals, 13px tabular; 118px each, 8px apart, 244px in all), the `0121` Mark row field rules (Enter commits, Escape reverts, Up and Down 0.01 mm, Shift 0.1, -10,000 to 10,000 mm), each commit one undo step "User origin". The numbers are **bed coordinates from the machine origin into the bed**, the job file's coordinates of that point (`0029` criterion 26), so a point inside the bed never reads negative, whatever sign GRBL's `MPos` has (architect, below). No row with the other picks.

**Clearing (W44).** There is no Clear button and no "remove origin" (one point per page, kept until replaced; YAGNI). The point goes only with an undo of the Set origin here that stored the first one; that same undo step also returns the pick to what it was, so User origin is never picked without a point. A page whose stored Start is User origin but whose point is missing or unreadable reads as `0029` criterion 4 e: the pick's value "Damaged", the block "Start is damaged. Pick one above.", never guessed into a file.

**The page changes machine (Frage 345, default C; W45).** When the page's machine profile is replaced by another one (the Machine block's profile pick), the point stays stored, the pick returns to **Machine origin** in the same undo step as the profile pick, and User origin stays in the list to pick again. A point taken from one machine's head is no place on another, so it is never used silently; the maker who switches between two equal lasers picks it again in one press. Edits of the same profile (work area, origin corner, a library update `0116`) keep the point and the pick; the work-area check measures again.

**User-origin cross on the canvas (criterion 32; W46).** While User origin is picked, the laser canvas marks the point: a 4px filled square on the point with four 6px arms (16px across), 2px `--toolbar-icon` over a 1px `--selection-casing` casing, screen-constant, at the point mapped to the page by the origin corner (the inverse of `0029` criterion 26). The square echoes the origin corner marker (an origin), the arms tell it apart from it; it is ink and screen-sized where a registration mark is faint green and true-sized (`--reg-mark`), and it has no ring (the head ring and the measured marks have one). No label (one point; the Start pick names it). View only: `aria-hidden`, `pointer-events: none`, not hit, not selectable, not exported, drawn above the drawing and the toolpath, and above the head ring (an HTML overlay over the canvas; UX review 2026-11-24: the ring with its casing stays readable around the 16px cross); hidden while the page's shorter side is under 48 screen px (as the origin corner marker); drawn also outside the page on the pasteboard when a typed point lies off the bed. Absent with the other picks. The origin corner marker, the rulers and their zero, the grid and the page's coordinates do not change (criterion 32).

**Where the job burns, drawn page (W47).** The drawing stays where it is drawn (criterion 32), so without the Toolpath view the canvas also shows where the job will burn: the job box at its shifted place, its anchor corner on the cross, as a hollow 1px `--toolbar-icon` rectangle dashed 4 on 4, no casing, view only as the cross; absent while the shift is under 0.01 mm. One line, no fill, no label.

**Toolpath view (criterion 31; W48).** The moves, rings, arrowheads and the play bar's head are drawn at the shifted place (where the job burns), the dashed box is not (the moves show the place). The items themselves are drawn at their own place in their own look at 35 % opacity while the shift is 0.01 mm or more, so what the pointer selects and moves is still seen (hit testing stays on the items, `0029` section 3); the selection box and handles stay on the items. Travel moves follow the file: from and back to the point the file starts and ends at (architect, below). A change of the drawing, the point or the pick redraws in the same frame as the checks.

**Frame ring (criterion 31; W49).** Nothing new in the Frame row: Frame traces the shifted outline (criterion 24), "Outside the work area" is measured on it, the pick and the causes are those of 2b. The head ring follows `Status.position` on the bed as built (absolute, as Machine origin), so it runs around the place the job burns, next to the cross and on the dashed box, not around the drawing.

**Work-area check (criteria 4 b, 31; W50).** The block line names the distance with User origin: "3 items lie outside the 400 × 300 mm bed by 12 mm." (at most 70 characters; the count stays the text button that selects the items, `0029` 6.4; the distance is the largest overshoot of the shifted burn box over the bed's four edges, whole millimetres rounded up, one decimal under 1 mm: "by 0.4 mm"; tolerance 0.01 mm). The Machine origin text "3 items lie outside the 400 × 300 mm bed." is unchanged; Head position stays unchecked (as built). Export, Send to machine and Frame are greyed by it as by every block.

**Member (criterion 12; W52).** A member's app has no pad and thus no Set origin here (drawn, not greyed, as W25). The Start pick, User origin and the User origin row are page edits, not machine commands, so they are the same in every app that edits the page, member or owner, and in the browser build (no footer there; a stored point is picked and typed, Export uses it). A member's Frame with User origin waits as 2b and is built from the requester's page (W35); the owner who confirms draws no ring and no cross for it.

**Keyboard (W51).** Set origin here is **its own Tab stop** after the arrows (DOM order after the four arrows, placed in the centre cell by the grid): Tab order ... job list, the arrows (one stop, roving), **Set origin here**, the step pick, Frame, the frame pick, the toolbar. The arrow keys in the cross keep moving the focus between the four arrows only (the centre is not in the roving set, so a roving Enter never stores an origin). Enter or Space presses once on key down, key repeat ignored; Space never holds. In the Output group: the Start pick as built, then the X and Y fields (two Tab stops) while User origin is picked. No shortcut, no global chord, Escape keeps its panel meanings. A cause change keeps the focus on the button.

**Accessibility.** Button named "Set origin here", `aria-disabled` with the cause as description, no `aria-pressed` (not a toggle); fields named "User origin X" and "User origin Y"; the cross and the dashed box are not announced; the stored point is announced once (W41). Contrast: existing tokens only (`--toolbar-icon` 8.2:1 on `--panel-bg`; the cross with its casing reads on white and on `--pasteboard-bg` as the origin corner marker). Not colour alone: shape (square and arms), the pick's word and the numbers.

**244 px and 800 x 600.** The footer does not change size (the button takes the empty centre cell). The Output group gains the 28px User origin row (36 coarse) and its 4px gap only while User origin is picked; it is in the scrolling column, so the footer and the column's 224px minimum stay. The longest new text, the check line, wraps in 244px as the other checks.

**Frage 345 (MC-ORIGIN-MACHINE): What happens to the user origin when a page gets another machine profile?**
- **A** The point is deleted and the Start pick returns to Machine origin (one undo step with the profile pick).
- **B** The point and the User origin pick stay; the work-area check measures it on the new bed.
- **C** The point stays stored, but the Start pick returns to Machine origin; User origin can be picked again with one press.
Recommendation **C** (never burns at another machine's point unasked, loses nothing). Default if unanswered: **C**.

**For the product owner and the architect.** (1) `0140` criterion 12 makes a point outside the work area a **warning** ("the job still runs or exports"), `0118` criterion 31 a **block**; these notes follow 31 (as `0029` 4 b) and the PO aligns `0140` before it is built. (2) Criterion 32's "the origin of the page's rulers": the rulers measure from the page's top-left (`0015`), not from the origin corner; read as "the rulers do not change". (3) Architect: the stored point and the X, Y fields are bed coordinates from the machine origin (the file's coordinates, criterion 26), converted from `MPos` by `$23` or the force-origin build as the jog clamp (criterion 17); with a non-zero work offset (criterion 14) the job runs in work coordinates, so either the point is stored in work coordinates or the shift subtracts the offset (the W9 warning line stays). (4) Architect: where a User origin file starts and ends its travel (back to the user origin, Proposal, so a second run lands on the same place, or to the machine origin as Machine origin); the Toolpath view draws what the file does. (5) Frage 345 C needs the page to know that its profile was replaced (not edited), e.g. the profile id the point was taken on.

**Decisions for veto (Proposal).** **W39** Set origin here in the centre cell of the cross, `LocateFixed`. **W40** enabled as Take head position (a read), greyed only while the position is stale, missing, moving, alarmed, simulated or not homed. **W41** no notice; the cross, the pick and the fields are the answer, a screen reader hears the point once; a press on the same point writes no step. **W42** User origin is the last option, absent until a point is stored; another pick keeps the point. **W43** the User origin row (X, Y) under the Start pick while User origin is picked, in bed coordinates. **W44** no Clear; the first point goes only by undo, with its pick. **W45** another profile returns the pick to Machine origin and keeps the point (Frage 345 default C). **W46** the cross: 4px square and four 6px arms, ink with casing, no label, only while User origin is picked. **W47** the dashed burn box on the drawn page while the job is shifted. **W48** in the Toolpath view the items stay visible at 35 % at their own place while the job is shifted. **W49** the frame ring follows the head on the bed, around the burn place. **W50** the bed check names the distance with User origin. **W51** Set origin here is its own Tab stop, outside the arrows' roving. **W52** a member has no Set origin here; the pick and the fields are the same for every role.

## Second slice 2e (run from line)

ux-engineer, 2026-11-25. Covers criteria 38 and 39 with `0132` criteria 12 and 13 (Frage 258 C; Frage 314 default A: only an interrupted job in the job list restarts from a line). Checked against sections 2 to 11, the 2a addendum (W16 to W27, "Not homed", the motion causes), the 2b addendum (W28 to W38, the cause order W31, Stop mid-frame W38), the 2c addendum (W39 to W52), `specs/0132-connector-job-queue/ux-notes.md` sections 1 to 8 (the interrupted row, the grid, rights drawn, not greyed), `0128` ux-notes 8.2 and 8.3, the built `JobList.tsx`, `jobList.ts`, `MachineControls.tsx`, `connectorText.ts` (`jobStateLine`), and the design-system rows "Job list (`0132`)", "Machine controls", conventions "Inline confirm", "Inline delete confirm", "Armed action". No hint labels, no popups, no notice after a success. W53 to W64 are left to the 2d addendum; new choices W65 to W78, (Proposal); two questions, Fragen 390 and 391.

**The interrupted row (`0132` criterion 6; W65).** First row of the list, outside the order, as built; it grows to **56px (60 coarse), three 12px lines**, the participant-row precedent, because "line n of total" and the sender do not fit one 160px line: (1) the name (`--toolbar-icon`, cut in the middle, full name in `title`); (2) `CircleAlert` and **"Interrupted at line 1,234"** in `--destructive` (the built text without the sender); (3) **"4,000 lines, 4.2 KiB, Phone"** (`--panel-muted-fg`, the waiting rows' counts line, "this app" for its own), so line 2 and line 3 read "line 1,234 of 4,000". The line is the one the restart starts at, the same number in the row, the button, the confirm and the state line. After a crash the stored line is approximate (criterion 39): line 2 reads **"Interrupted near line 1,234"** (W66; needs the flag from the architect, below). No position, no cause word, no bar: the line is what the owner decides on. Owner row, right to left: **Remove** (as built), 4px, **Restart from line** (below), in the column where waiting rows have Run next automatically; the grip slot stays empty (16px). Name column 160px (144 coarse), unchanged.

**Restart from line: place and look (`0132` criterion 12; W67).** The `0064` symbol button, 28px (36 coarse), glyph Lucide **`ListRestart`** (not used elsewhere in Curvyo; `SkipForward` is Run next automatically, `RotateCcw` is reset in value fields, `Play` is Start). Tooltip **"Restart from line 1,234"** (`top`, 400 ms, no shortcut text); accessible name "Restart sign.gcode from line 1,234" (the row-action pattern of "Remove <name>"). Owner only: a member's app has no Restart on the row (rights drawn, not greyed, `0132` section 3, W25); the member's row stays as built, no actions. Start never restarts it (as built); Remove stays and removes it.

**Enabled, and its causes (criteria 8, 38; `0132` 12 a; W68).** Greyed in place (`aria-disabled`, 40%, reachable by Left/Right in the row, two-line tooltip "Restart from line 1,234" / cause, sr-only description). First cause wins; causes the owner cannot clear from this app come before those one press clears:
1. "Connection lost" and the status-read text (the status is stale);
2. the NoController text;
3. from the job text, never changing: "Starts at the head position" (a `G91` job, criterion 38), "Cannot rebuild the state before line 1,234: <word>" (rule d);
4. the machine is busy (the motion causes of W31): "Homing", "Jogging", "Framing", "A job is running" (Running, Paused), the waiting texts ("Waiting for the start at the machine", "Waiting for Marc to confirm", "Phone waits for Marc to confirm", "Home requested by Phone", "Frame requested by Phone"), "Sending" (Receiving), **"Another job is ready"** (a held Ready job: the restart takes the current-job place);
5. "Alarm"; "Controller state: <word>" (Door, Check, Sleep);
6. "Laser mode is off ($32=0)" (the `0125` text);
7. **"No position from the machine"** (W40's words: the controller reports no machine position or work offset now, or the interruption stored none, as a `list.json` of `0132` slice 1; architect R4, fixup 2026-11-25);
8. **"Work offset changed"** (the controller's work offset differs from the stored one; Curvyo writes no `G10`/`G92`, so Home does not clear it);
9. **"Not homed"** (rule a fails: not homed since the interruption and the position is not the stored one within 0.01 mm; the 2a state word, not "Home first", which is an instruction).
Enabled in Idle and Done when none applies. A member: no button (above), not a cause. The lead's order put "Not homed" first; W68 puts busy before it because a busy machine cannot be homed either, so "Not homed" during a job would point at a press that is greyed (Proposal).

**The confirm (`0132` criterion 12 b; W69, W70).** A press on an enabled Restart opens the **Inline confirm** in the row's place: the interrupted row leaves the grid and the block stands directly under the header, above the waiting rows (it grows the footer upward; the toolbar does not move). `role="group"` named by its sentence, not a dialog:
- **Sentence** (12px `--toolbar-icon`, up to three lines, Frage 391 C, customer 2026-11-25): **"Restart sign.gcode from line 1,234 of 4,000?"**, the question alone, no sentence about the material. The name is cut at 40 characters as "Start <name>".
- **Buttons**, 28px (36 coarse), 8px apart, 244px together: **Cancel** 76px left, **"Restart from line 1,234"** 160px right (the confirm button carries the number, criterion 12 b; wider than Cancel so a seven-digit line fits; the one departure from equal widths). The action uses the **accent look** of the Inline confirm's start variant (1px `--armed-accent` border and label on white, pressed fills `--armed-accent` with a white label; `0134`'s accent exception: the press starts a job, it removes nothing).
- **No default button** (criterion 12 b): the focus goes to **Cancel**, so Enter or Space at once cancels; the action ignores presses for the first 400 ms and a repeated keydown (the `sendArm.ts` rules); no time-out (the sentence is read).
- **Closes without sending** on Cancel, Escape, a tab, page or panel change, a press outside the block, and when the facts change: a push that gives Restart a cause (W68), removes the row or changes its line. It then shows the row again, the button with its cause; the focus goes back to Restart. The confirm press re-reads the last status: if a cause applies by then, nothing is sent (the connector checks rules a to d again).
- One confirm per app; another owner app has its own; when one owner restarts, the row leaves and the other's block closes.

**After the confirm (criterion 38; W71).** The block closes, the row comes back with **Restart busy** (`LoaderCircle`, `aria-busy`) until the connector's answer (5 s, then "Timed out"). Accepted: the row leaves the list with the next push and the job block shows the job as current; the focus goes to **Stop** in the toolbar (the head moves next; Stop is the control that matters then; W72). Refused: the row stays, the job block's state line reads the refusal in the failure look, alert, once (the Home refusal place): "Not homed", "Work offset changed", "Laser mode is off ($32=0)", "Cannot rebuild the state before line 1,234: <word>", "Starts at the head position", "Connector busy", "Timed out"; the focus stays on Restart.

**State line during the restart (W73).** The job block as for a job (`0064` section 8 frame), counts line "sign.gcode, Phone" and at its right "1,234 / 4,000 lines", the bar at 1,234 of 4,000 (the line numbers are the job's own):
| Phase | State line | Look | Live |
|---|---|---|---|
| `--require-local-start` set | "Waiting for the start at the machine" (built text) | neutral | polite |
| `M5`, `M9`, the rebuilt state, the travel to the start of line n, `M3`/`M4` | **"Moving to line 1,234"** | neutral, no glyph | polite, once |
| line n and the rest | **"Running from line 1,234"** (Paused reads "Paused", as built) | neutral | polite, once |
| ends well | **"Completed, lines 1,234 to 4,000"** | neutral | polite |
| Stopped, Failed, Alarm, lost | the built texts ("Stopped at line 2,001", "Failed at line ...", the alarm table, "Lost the controller") | as built | as built |
During both phases: Home, the pad's arrows, Set origin here, Frame, Send job... and Start / Resume follow the Running rules as built (Send job... stays on: a send is added to the list); Pause is enabled (a feed hold during the travel is harmless, Resume continues it); **Stop enabled, never asks**. No canvas mark: the job may come from any app or file, so the page does not know where line n lies (the head ring of 2b stays a frame-only mark).

**Stop during it (criterion 10; W74).** Before line n: a start waiting at the machine is discarded; the entry move ends at once (the frame sequence of criterion 10: hold, reset, banner, `M5`, `M9`). Nothing has burned, and the job **returns to the list as the interrupted row at the same line** (the restart did not begin; the stored interruption is unchanged); the asked reset keeps homed since connect, so Restart is enabled again at once. No line for the app that pressed Stop; other apps "Not started: Restart from line 1,234" (muted) until the next state change (the `0118` "Not started: Home" pattern). From line n on: Stop ends the job as `0127` G12 ("Stopped at line 2,001") and, by Frage 390 C (customer 2026-11-25), it becomes the interrupted row again, its line backed off from the last acknowledged one, never before n.

**Member (`0128`; criterion 12; W75).** A member sees the interrupted row as built plus line 3, no Restart, no Remove; Restart is owner-only and never waits for an owner (it is not a member's command). During the restart the member's footer shows the same job block and state lines; the member's Stop works (Stop from anyone). Foreground mode: every app acts as owner (`0132` adrs L4) and has Restart.

**Keyboard (W76).** The row as built (`role="grid"`, one Tab stop): Left/Right move between the row and its buttons (Restart, Remove); Enter or Space on Restart opens the confirm (key down, repeat ignored); `Delete` still arms Remove; no key restarts from the row itself, no shortcut, no global chord. In the block: Tab order Cancel, Restart from line 1,234 (two stops, outside the grid); Escape cancels and returns the focus to Restart in the row; Escape keeps its panel meanings otherwise.

**Accessibility.** Row text "sign.gcode, 4,000 lines, 4.2 KiB, from Phone, interrupted at line 1,234 of 4,000" ("near line" after a crash). Restart named "Restart sign.gcode from line 1,234", `aria-disabled` with the cause as description, `aria-busy` while busy. The block a `role="group"` named by its sentence; its opening is not announced beyond the focus landing on Cancel inside the named group. Live: "Moving to line 1,234" and "Running from line 1,234" once each (polite), the refusals as alerts; other apps' restarts are not announced beyond their state lines. Not colour alone: line 2 has `CircleAlert` and words. Contrast: existing tokens only (`--armed-accent` label 4.5:1 on white, measured for `0130`; `--destructive` 4.8:1 on `--panel-bg`).

**244 px and 800 x 600 (W77).** Row: 4px inset, 16px grip slot, 4px, 160px name column, Restart 28px, 4px, Remove 28px = 244px (coarse: 144px column, 36px buttons). Interrupted row 56px instead of 40px (+16px). Confirm block: sentence three lines (48px) + 8px + 28px buttons = 84px (92 coarse); while it is open the grid shows at most **2 waiting rows** (80px), then scrolls, so the list with header stays at 200px. Heights (fine): Idle, pad closed, interrupted row and 3 waiting: 97 + 8 + 28 (header) + 4 + 164 (the grid's built limit, then it scrolls) = about 301px, the column keeps about 229px; with the confirm open 97 + 8 + 28 + 4 + 84 + 4 + 80 = about 305px, column about 225px (the 2b floor of 224px holds); worst case (two warning lines, a Done block, pad open, confirm open) about 500px, the column above shrinks to a few rows while the confirm is open and returns at its first press (accepted, Proposal). The longest new text, the sentence, wraps in 244px.

**For the product owner and the architect.** (1) PO: criterion 38's "Home first" reads "Not homed" in the app (a state, not an instruction); the connector may keep "Home first" in `admin.log`. (2) Architect: the interrupted `ListEntry` carries the connector's own restart cause (`restart_cause: Option<String>`, recomputed on every status change with the rules a to d and the job-text checks), so the app greys with the same rule the connector refuses with (one rule, one place, as `access.rs`); plus `line_approximate: bool` for W66. (3) Architect: a work offset that differs from the stored one refuses even after Home (the job's coordinates are work coordinates; criterion 39's "or" reads as "homed and the same work offset, or the same offset and position"). (4) Architect: `Status` names the restart's phase (entry move vs. from line n) and the start line, for W73; a Stop before line n returns the entry to the list unchanged (W74), as a frame returns to the state it came from. (5) PO: a restarted job that ends Done continues the chain of `0132` criterion 11 like any current job (Proposal); nothing starts by itself while the confirm is open or the restart waits for the start at the machine.

**Decisions for veto (Proposal).** **W65** the interrupted row has three lines (name, "Interrupted at line n", the counts line), 56px. **W66** "near line n" after a crash. **W67** Restart is a `ListRestart` symbol button in the row, owner only, left of Remove. **W68** cause order: stale, no controller, job-text causes, busy, alarm and controller word, laser mode, no position, work offset, not homed. **W69** the confirm is the Inline confirm in the row's place, sentence with name, line and total, Cancel focused. **W70** the confirm button carries the number, 160px, in the accent look. **W71** Restart is busy until the answer; refusals read in the job block's line. **W72** focus to Stop after an accepted restart. **W73** state lines "Moving to line n", "Running from line n", "Completed, lines n to total". **W74** Stop before line n returns the row unchanged. **W75** a member sees the row, never Restart. **W76** no key restarts from the row; Cancel and the action are two Tab stops. **W77** two waiting rows visible while the confirm is open. **W78** no canvas mark for the restart's start point.

**Frage 390 (RFL-WHICH-END): Which ended jobs become an "Interrupted" row that an owner can restart from its line?**
- **A** Only a job that was running when the connector restarted or crashed (`0132` criterion 6 as built); a job that ends in Stop, Alarm or a lost controller ends as today ("Stopped at line n", the alarm, "Lost the controller") and is gone from the list.
- **B** As A, plus a job that ended in an Alarm (door, limit switch) or with a lost controller (USB, the laser's power) while the connector ran; Stop stays a final end.
- **C** As B, plus a job ended with Stop (also a restarted one): every Stop leaves an interrupted row the owner removes or restarts.
Recommendation **B** (the restart is for the ends the owner did not choose; Stop stays final, so the list does not fill with every stopped job; criterion 39 already stores the line for Stop, alarm and loss). Default if unanswered: **A** (the smallest, as built; B or C later by one answer). **Answered C by the customer (2026-11-25).**

**Frage 391 (RFL-CONFIRM-WORDS): How does the confirm read before a restart from a line (`0132` criterion 12 b)?**
- **A** "Restart sign.gcode from line 1,234 of 4,000? The rest burns where the material lies now." with Cancel and "Restart from line 1,234".
- **B** "Restart sign.gcode from line 1,234 of 4,000? Has the material stayed where it was?" (the press answers yes), same buttons.
- **C** Only "Restart sign.gcode from line 1,234 of 4,000?", same buttons.
Recommendation **A** (names the consequence without telling what to do; B reads as a check question, C leaves the material out). Default if unanswered: **A**. **Answered C by the customer (2026-11-25).**
