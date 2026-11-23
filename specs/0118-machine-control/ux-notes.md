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
