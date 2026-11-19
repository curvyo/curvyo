# 0134 Send and start: UX notes

ux-engineer, 2026-11-19; updated 2026-11-20 for Frage 263 B (the armed press, SS13). Covers criteria SS1 to SS13 of `specification.md` and its "UX notes must change" list: the play glyph on the Output group's button, the names and tooltips per state (SS9, SS13), the armed press and its look, disarm rules and keys (SS13), the "Not started" line (SS6), the waiting states, focus after the press, the `0125` UX note rows. Checked against `specs/0125-send-to-machine/ux-notes.md` (button row, disabled causes, busy, focus), `specs/0118-machine-control/ux-notes.md` (on `story/machine-control`, PR #399: the Machine controls footer, its toolbar with Start / Resume `Play` and Send job... `FileUp` (W5), the one state line by Frage 262 A, live regions), `specs/0064-machine-connector/ux-notes.md` sections 3, 7, 8, `specs/0081-inline-delete-confirm/ux-notes.md`, `specs/0133-fix-pause-while-homing/ux-notes.md`, the built `frontend/src/components/panel/SendToMachine.tsx`, `OutputGroup.tsx`, `frontend/src/hooks/useArm.ts`, `useDeleteArm.ts`, `frontend/src/lib/deleteArm.ts`, `frontend/src/components/ui/tooltip.tsx`, and `docs/design-system.md` (rows "Send to machine button (`0125`)", "Machine controls (Operations tab) (`0118`)" as proposed, "Hidden, not disabled"; Interaction conventions "Inline delete confirm", "Armed action", "No notice after a success", "Nothing moves under the pointer"). PR #396 (`fix/machine-profile-buttons`, the `--armed-accent` token and the "Armed action" accent exception for Update profile) is **not merged** into `main` on 2026-11-20; section 4 and section 11 say what to do in either case. Customer rules applied: no popups, no dialog, the armed press of Frage 263 B, symbol buttons with tooltip and accessible name, labels that name a control or a state only, no operating tips (2026-10-16), no notice after a success (2026-10-31). Choices marked (Proposal) are vetoable at the demo; section 10 lists them as P1 to P16. No question for the customer.

## 1. What changes where

- **Output group, button row** (`0125` section 2): the Send to machine button gets a play glyph, a second name and, when its next press will start, the armed press. Place, size, frame, secondary look, disabled look and cause order stay as built.
- **Machine controls footer** (`0118`): its state line shows the results of the one action ("Sending <name>", "Running", the waiting lines, "Not started: <reason>"), the one line of the tab by Frage 262 A. Its toolbar is unchanged; its Start is never armed.
- Nothing else changes. No new token (the armed fill is `--armed-accent` of PR #396, an alias of `--editor-accent`); one new glyph use (`CirclePlay`, P1).

## 2. Glyph: two play buttons in one tab, told apart (SS12)

The Operations tab now holds two buttons that start a job: the page's button in the Output group and Start / Resume in the footer's toolbar (`0064` `Play`). They do different things: the page's button **sends this page's job** and starts it only on a free machine, after an armed press; Start starts **whatever job the connector holds** (also one sent by Send job... or another app), with one press. `0118` W5 set the rule that two different actions in one tab do not share a glyph.

| Button | Glyph | Changes? |
|---|---|---|
| Machine controls, Start / Resume | Lucide `Play` | no: the media convention, `0064`, as built; one press, never armed (`0064` Q14) |
| Output group, Send to machine | Lucide **`CirclePlay`** (play in a circle), 16px, 1.5px absolute stroke, `currentColor` | yes: was `Upload` (accepted by the customer 2026-11-20) |

- **Why `CirclePlay` (P1).** It is the play symbol the customer asked for, and it is not the toolbar's bare triangle: one glyph family for "run", the circle marking the page's one-shot "send and run". It stays the right partner of Export's `Download` in the row.
- **Other differences that already hold:** place (the Output group in the column vs the fixed footer), frame (32 x 32 outline button with a 1px `--field-border` next to Export vs a borderless 28px toolbar button), names ("Send to machine and start" / "Send to machine" vs "Start" / "Resume"), and now the press (armed vs one press).
- **Why the footer's Start is not armed and the page's button is.** The footer's Start acts on a job the connector already holds, which the maker sent on purpose; the page's button turns one press into "send and run a laser" from the place where the maker also exports. The customer decided both (`0064` Q14 stays; Frage 263 B); these notes do not change the footer.
- **The glyph does not change with the state (SS8).** It stays `CirclePlay` whether the next press will start or only send, armed or not, and while disabled; while sending and starting it is `LoaderCircle` as built (`0125` section 3). The name and the armed look carry the difference (sections 3 and 4).

## 3. Names and tooltips per state (SS9, SS13)

The name comes from the core's "will it start" function (SS11) over the last status. Tooltip line 1, `aria-label` and the disabled description's first line are one string, set in the same render, so they never disagree.

| State | Look (section 4) | Name, tooltip line 1, `aria-label` | Tooltip line 2 / `aria-describedby` | A press |
|---|---|---|---|---|
| **Idle, free machine**: owner (every paired app until `0128`) or member (P3), connector `Idle` or `Done`, controller Idle (no Door, Check, Sleep), no job held or waiting, list empty (`0132`) | secondary outline, as built | **"Send to machine and start"** (25 characters) | none | arms (sends nothing) |
| **Idle, send only**: a job held or waiting (also this app's own `Ready` job, Frage 264 A), Alarm, Door, Check, Sleep, NoController, no status read yet, Connection lost | secondary outline | **"Send to machine"** | none | sends only, as `0125`; never arms |
| **Armed** (from the arming press until the confirm, a disarm or 4 s) | filled `--armed-accent`, white glyph | **"Press again to send to machine and start"** (40 characters) | none | after the guard: sends and starts (section 4) |
| **Guard** (the first 400 ms of Armed) | as Armed, no extra look | as Armed | none | ignored, no feedback (a double-click never starts) |
| **Sending** (after the confirm, until `JobAccepted`) | busy, as built: `LoaderCircle`, Export's disabled ground | "Send to machine and start" | **"Sending"** | nothing |
| **Starting** (`Start` sent, until the connector's answer) | busy, as Sending | "Send to machine and start" | **"Starting"** (P5) | nothing |
| **Disabled** (SS8, `0125` S2 to S5, S8) | Export's disabled look, as built | "Send to machine and start" or "Send to machine" by the rows above; without a pairing there is no status, so "Send to machine" | the cause of `0125` section 3, in its order | nothing |

- **"Held"** in this table is a job the connector holds or waits for (row "Idle, send only"); it never arms (SS2). A busy button (Sending, Starting) is `aria-busy` and `aria-disabled` and keeps its Tab stop, as built.
- **The armed name is a state, not a tip.** "Press again to send to machine and start" names what the next press does, as the armed trash's "Delete profile. Projects keep their copy." does (`0081`); it is the spec's wording (SS9, SS13). Shorter "Press again to start" was weighed and dropped: it hides that the press also sends.
- **Without surprising (P2).** The name changes only with a connector status, the page's checks, or the arm itself; never on hover, focus or a timer other than the 4 s disarm. The button does not move or change size (the name is never a visible label; the 40-character armed name takes at most two lines of the tooltip's `max-w-64` at 12px). An open tooltip updates its text in place: no close and reopen, no animation, no flash. A name change by status is not announced (it is not a live region); the state line's own live region already announces the state change that caused it. Only arming is announced (section 7).
- **`--require-local-start` and member:** the app sends Start there too (SS3), so the name reads "Send to machine and start" and the press arms; the connector holds the start for the confirmation (P3). Until the status carries the flag, the app cannot know; the name follows criterion 1 only.

## 4. The press: armed, and when it starts (SS1, SS2, SS9, SS13)

### 4.1 The armed look (P11)

- **Fill `--armed-accent`** (alias of `--editor-accent`, `#2F6FEE`), border the same colour (the 1px `--field-border` frame turns accent, so the button stays exactly 32 x 32, 36 coarse), glyph `CirclePlay` white (`--toolbar-icon-active-fg`). Hover keeps the armed fill unchanged (no `--editor-accent-hover`, no brightness, as the armed trash and Update since `24cebdb3`), so the look never flickers under the pointer. Focus ring as built: 2px `--editor-accent` with the 1px white offset, which separates the ring from the fill. No transition over 120 ms, none under `prefers-reduced-motion`; no countdown, no pulse.
- **Contrast** (measured 2026-11-20, WCAG 2.x formula): white glyph on `#2F6FEE` **4.54:1** (meets 4.5:1 text and 3:1 graphics); the armed fill against `--panel-bg` `#DCDCE0` **3.32:1** (meets 3:1 for a UI component, 1.4.11); the focus ring against its white offset 4.54:1. For comparison: white on `--destructive` 6.54:1, `--destructive` on `--panel-bg` 4.78:1; idle glyph `--toolbar-icon` on `--panel-bg` 8.27:1. The disabled look is exempt and unchanged.
- **Confirmed: accent, not red (the PO's proposal).** Reasons: (1) `--destructive` means "the next press loses something" in every armed use (Inline delete confirm, the Share tab, Forget pairing); a start loses nothing, it makes something. (2) Stop in the footer is the red, final action; a red play would read as stop or as danger, and two reds in one tab would blur which one ends a job. (3) The house already has the accent armed look for an armed press whose second press keeps or makes rather than loses (Update profile, PR #396, customer Frage 254 A); this is its second user, not a third armed look. (4) The cue is the change from outline to filled, which reads as "the press took effect, press again", and the name and the polite status carry it for those who do not see colour (1.4.1).
- **Next to Export.** For at most 4 s the row holds two filled accent buttons: Export (wide, label "Export job file", `Download`) and the armed Send (square, no label, `CirclePlay`), 4px apart. They stay told apart by shape, label and glyph, and only the armed one is new since the press; Export's own hover brightens, the armed Send does not. Not a second Primary: Primary is a resting role, armed a 4 s state. No extra gap, no outline ring around the armed button (rejected: a ring would read as focus).
- **PR #396 is merged** (2026-11-20 rebase): the build uses `--armed-accent` directly; the token row and the "Accent exception" paragraph of `docs/design-system.md` already name Send to machine as its second user.
- **Dark theme:** none exists yet; `--armed-accent` swaps with `--editor-accent` when it does (#396's token row).

### 4.2 Arming and confirming

- **Arming press.** A press (pointer, `Enter` or `Space`) on the button in "Idle, free machine" re-reads the status (SS9). It arms only if the name the maker saw ("Send to machine and start") **and** the fresh status both say it will start (P4). It sends nothing, the look turns armed, the name becomes the armed name, the polite status says it (section 7). One armed action in the app at a time: the arming press is a press elsewhere for any other armed control, which disarms (`useDeleteArm`'s window listeners, as built).
- **An arming press that finds the start gone arms nothing and sends nothing (P12).** The fresh status says "only send" (another app sent, an alarm, Door): the press does nothing but bring the name to "Send to machine"; the footer's state line already shows the cause. A press never does more than its shown name says: a maker who expected the first press to only arm never gets a send from it.
- **Guard.** A press within 400 ms of arming is ignored (`ARM_GUARD_MS`), so a double-click never starts. Nothing shows the guard.
- **Confirming press.** A press after the guard and within 4 s of arming re-reads the status. If the fresh status still says it will start (P4 at the second press), the button disarms, turns busy and runs the one action: send, then on `JobAccepted` with `Ready` and the sent job id, `Start` (SS1; section 5). If the fresh status no longer says start (the race between the last status and the fresh read), the button disarms and the press **sends only** by SS2/SS5 (P13): "Connector busy" until `0132`, then the list; or, on a free machine whose controller blocks the start, Ready and "Not started: Controller state: <word>" (section 6). Nothing starts.
- **Key repeat never arms or confirms (P14).** A held `Enter` repeats its click after about 500 ms on most systems, which is past the 400 ms guard: a repeated `keydown` (`event.repeat`) on the button is ignored (default prevented), armed or not. `Space` clicks on key-up and does not repeat.
- **P4, unchanged in its direction.** A fresh status can turn "and start" into "only send" (or into nothing, P12), never "only send" into "and start". A maker who read "Send to machine" never arms and never gets a running laser from that press, also when a job ended in the same instant: the press sends only; the next press, with the new name read, arms.

### 4.3 Disarm (SS13)

It disarms **silently** (the polite status is emptied, no announcement, no notice) and returns to the "Idle" row its current status gives, on any of:

| Cause | Note |
|---|---|
| 4 s after the arming press | `ARM_TIMEOUT_MS`, no countdown |
| `Escape` anywhere | consumed: the first Escape only disarms (as `useDeleteArm`); a second Escape goes on as usual. Not armed, Escape does nothing to the button |
| Any other pointer press, anywhere in the app, also the right button | the arm's control attribute sits on this button only |
| Focus leaving the button | the arm's zone is the button itself, not the Output group: Tab to the footer disarms |
| Connector status or the page's checks change so that the name is no longer "Send to machine and start" | another app sends, a job ends elsewhere, Alarm, Door, the pairing goes, a check starts to block (then disabled) |
| **The job the press would send changes** (P15) | an undo or redo step, a cooperative edit by a peer, a change of the Start pick or an output setting: the maker armed for the job he looked at. A status change that keeps the name "start" (Idle to Done) does not disarm |
| The tab, page or workbench changes, the panel collapses | as the Inline delete confirm's "the list changes" |

### 4.4 Tooltip and arming (P16)

- The house tooltip closes with a press (pointer up, and Radix's own close on click, also for `Enter`/`Space`). The arming press does **not** reopen it: an armed state that throws up a floating text would be a popup in all but name. The armed look and, for screen readers, the polite status carry the change.
- If the pointer leaves and rests on the button again (400 ms) or keyboard focus returns to it while armed, the tooltip shows the armed name. A tooltip that is open while the arm ends (4 s, a status change) changes its text in place to the Idle name, as P2 says; it does not close, flash or move.
- A coarse pointer has no tooltip; the armed look and the name carry the state.

## 5. Busy and the state line during the action (SS1, SS3, SS6)

The action after the confirming press has two steps; the button is busy through both, so the confirmed press reads as one action (SS10). While armed, nothing changes in the footer.

| Step | Page button | Footer state line (visible) |
|---|---|---|
| Armed (section 4) | armed look, armed name | unchanged ("Idle" / the Done line as before) |
| Sending, until `JobAccepted` (`0125` S6 to S8) | busy, as built: `LoaderCircle`, `aria-busy`, `aria-disabled`, tooltip line 2 "Sending" | "Sending <name>" |
| `JobAccepted` with `Ready`, `Start` sent, until the connector's answer (the 5 s answer limit as built) | **still busy**, tooltip line 2 **"Starting"** (P5) | what the connector reports: "Ready: <name>, <n> lines, <size>" for the moment it holds the job |
| Started | not busy; name "Send to machine" (the machine now holds a job); never armed again until it is free | "Running" (counts line with the name, bar, elapsed) |
| `--require-local-start` | not busy | "Waiting for the start at the machine" (as built, bar at 0) |
| Member (`0128`) | not busy | "Waiting for <owner> to confirm" (`0128` text; the owner's name cut at its end with an ellipsis so the line stays within 70 characters) |
| Start refused or failed (SS6) | not busy | "Ready: <name>, <n> lines, <size>" and under it "Not started: <reason>" (section 6) |
| Press only sent (a held or waiting job; or the confirm found the start gone, P13) | as `0125` | `0125` result as built ("Connector busy" until `0132`; then the list shows the added job); while the connector holds a job its line and bar stay and the refusal reads under it (P17) |

- **Nothing optimistic.** The line shows only what the connector reports; "Running" comes with the connector's Running.
- **The footer's toolbar during the Start step (P6):** Start / Resume is greyed (its own Start is the one in flight), the other buttons keep their row: Stop is enabled in Ready (it discards, as built) and in Running, and never asks. If Stop discards the job before the Start answer, the Start is refused, the job is gone, the line follows the connector (no "Not started" without a Ready job).
- **The footer's Start while the page button is armed:** on a free machine it is greyed already (no job held); a press on it is a press elsewhere and disarms.
- **The Lasered commit** (SS7) happens on `Ready` as built and shows nothing new (no notice after a success; the fixture-off notice of `0125` section 6 when N > 0, unchanged).
- **Homing first (SS4):** the job's own `$H` reads "Running" at line 0 as built (`0127` section 7); a Pause there follows `0133` ("Pause after homing").

## 6. "Not started: <reason>" (SS5, SS6)

- **Where (P7).** In the footer's job block, a second 12px line directly under the state line "Ready: <name>, <n> lines, <size>", in the failure look (`CircleAlert` 12px, `--destructive`, 4.8:1 on `--panel-bg`); wraps, at most two lines. The footer grows upward as all its parts do, so the toolbar never moves (`0118` W2). The Output group shows nothing (it has no state line, Frage 262 A).
- **Words.** "Not started: " and the connector's or the app's own text, as built elsewhere, capital after the colon as "Alarm: <text>": "Not started: Connector busy", "Not started: Controller state: Door", "Not started: Timed out", "Not started: Lost the controller", "Not started: Connection lost". "Laser mode is off ($32=0)" refuses the send itself (SS5) and reads as the `0125` refusal, not as "Not started". A state, never an instruction: no "Press Start", no "Try again".
- **When.** (a) `Start` refused or failed after `JobAccepted` while the job stays `Ready` (SS6). (b) The press only sent on a machine that holds no job, because a controller state blocks the start (Door, Check, Sleep; SS5), with the unarmed "Send to machine" or after a confirm that found the start gone (P13): "Not started: Controller state: Door", so the maker learns why the laser did not move. (c) The connection dropped after `JobAccepted`: after the reconnect the fresh status decides; Running means it started (no text), Ready means "Not started: Connection lost". Not when the press only sent because a job was held: that result is the `0125` refusal or the `0132` list. Never after a disarm: a disarm sent nothing.
- **A refused send under a held job (P17, UX review 2026-11-20).** A press that only sends while the connector holds or waits for a job (Receiving, Ready, AwaitingLocalStart, Running, Paused) and is refused ("Connector busy", also a "Timed out" or "Not received") reads in the same second line under that job's line; the line ("Waiting for the start at the machine", "Running" with elapsed time) and its bar stay, so the refusal never hides why the machine is busy, for as long as that state lasts. Said once as an alert (the refusal only; the job's line was said before). With nothing held, or with the connection lost, the refusal is the line itself, as `0125`.
- **How long.** Until the next state change of the connector, the next press of the page button (also an arming press), or a press of the footer's Start. It leaves without a trace; it is never a notice and never a History step.
- **Live.** A hidden `role="alert"` (it answers the maker's own press), once; the Ready line before it is not announced separately (section 7).
- **The footer's Start is enabled** (SS6) by the Ready row as built; nothing points to it (no tip).

## 7. Live regions (SS1, SS3, SS6, SS13)

- **The arm's own region.** One sr-only `role="status"` (polite) next to the button, as the Inline delete confirm's: it holds "Press again to send to machine and start" while armed and is empty otherwise. Arming announces once; a disarm empties it without an announcement; the confirming press empties it before the footer's region speaks, so the two never talk over each other.
- **The footer's regions** (the two hidden regions of `0118` section 11; the visible line has no live role). During the one action the polite region **does not announce the in-between "Ready: ..."** when the Start step follows (P8), also when the start's answer comes before the connector's Running; it announces "Sending <name>" at the start and then the result once: "Running", "Waiting for the start at the machine", "Waiting for <owner> to confirm", or, as an alert, "Not started: <reason>" (the Ready line is part of that announcement: "Ready: <name>, <n> lines, <size>. Not started: <reason>").
- A press that only sends announces as `0125` (the Ready line, or the refusal as an alert). An arming press that found the start gone (P12) announces nothing of its own; the footer has already announced the state change.
- Refusals of the send itself (`0125` section 5: "Connector busy", "Machine in alarm: <text>", "Nothing answers at <host:port>", "Timed out", "Not received") are alerts, as built.

## 8. Focus and keyboard (SS10, SS13)

- **Keys.** `Enter` or `Space` on the focused button arms, and again (after the guard) confirms; focus stays on the button through arming, the guard, Sending and Starting (it is `aria-disabled` while busy, so it keeps focus and its Tab stop). A repeated `keydown` is ignored (P14). `Escape` disarms when armed (consumed) and otherwise does nothing to the action; Stop is the way to end a started job. **No shortcut and no global chord** (P10; `0125` U12, now more so: the press can start a laser).
- **Pointer: the arming press keeps focus where the press put it (P9).** Today a pointer press returns focus to the canvas (`onReturnFocus`, as Export); on the arming press that would be "focus leaving the button" and disarm at once, so the arming press does not call it. The confirming press, and a press that only sends, return focus as Export does. In a WebKit build a pointer press does not focus a button; focus then stays where it was and only a focus change that lands outside the button disarms, as `useDeleteArm` already handles.
- **Focus never jumps to the footer or to Stop**: a moving focus would surprise and Stop is always in view in the fixed footer. From the page button, the next Tab stop is the footer (Unlock when shown, then the toolbar; End in the toolbar reaches Stop); Tab out of an armed button disarms.
- Hit target 32px (36 coarse), as built; the armed look keeps it.

## 9. Checks for the review

- **States to try** (`curvyo-connector-server --fake-controller`, `--require-local-start`, `--fake-alarm-at-line`; the pty simulator with Door): free machine, first press → armed look, name "Press again to send to machine and start", one polite announcement, nothing sent (connector log); second press after 0.5 s → "Sending", "Running", one announcement; double-click → armed only; wait 4 s → disarmed, silent; Escape, a canvas press, Tab out, a right press → disarmed; Ctrl+Z with focus on the armed button → disarmed; a second app sends while armed → disarmed, name "Send to machine"; hold `Enter` for 2 s on the focused button → armed only, never started; a Ready job held → name "Send to machine", one press → "Connector busy", never armed; Done → name "Send to machine and start"; `--require-local-start` → armed, then "Waiting for the start at the machine"; Door on an idle machine → name "Send to machine", press → Ready and "Not started: Controller state: Door"; Start refused (state change between) → Ready and "Not started: ..."; connection dropped after `JobAccepted`; Home first job → "Running" at line 0, Pause → "Pause after homing" (`0133`); every disabled cause of `0125` with the new glyph; the browser build ("Needs the desktop app", glyph `CirclePlay`, never armed).
- **Name, look and status agree** in every row of section 3; an open tooltip updates in place when another app sends or the arm times out; the arming press opens no tooltip.
- **Nothing moves:** the button row keeps its place and size in every state, armed included (border turns accent, no width change); the footer grows upward.
- **No popups, no hint texts, no notice after a success.** Contrast: section 4.1 numbers; existing tokens only (`--armed-accent` / `--editor-accent`, `--toolbar-icon` 8.27:1, `--destructive` 4.78:1 on `--panel-bg`; disabled look exempt).
- **One armed action:** arm an Inline delete confirm trash elsewhere, then arm Send → the trash disarms; and the other way round.

## 10. Decisions for veto (Proposal)

- **P1** The page's button takes Lucide `CirclePlay`; the footer's Start / Resume keeps `Play` (accepted 2026-11-20).
- **P2** The name changes only with a connector status, the checks or the arm; an open tooltip updates in place; a name change by status is not announced.
- **P3** Under `--require-local-start` and as a member the name still reads "Send to machine and start" and the press arms (the press starts, the connector holds it).
- **P4** Arming and confirming each need both the shown name and a fresh status to say it will start.
- **P5** The button stays busy through the Start step, tooltip line 2 "Starting".
- **P6** The footer's Start / Resume is greyed during the Start step; Stop keeps its row.
- **P7** "Not started: <reason>" is a second failure-look line under "Ready: ..." in the footer's job block, also when a controller state blocks the start of an otherwise free machine.
- **P8** The in-between "Ready: ..." is not announced during the one action; the result is announced once.
- **P9** Focus stays on the button through arming (pointer and keyboard); after the confirm it stays (keyboard) or returns as Export's (pointer); it never jumps to the footer.
- **P10** No shortcut for the button.
- **P11** Armed look: filled `--armed-accent` (`--editor-accent`), white `CirclePlay`, accent border, hover unchanged; not `--destructive`.
- **P12** An arming press that finds the start gone arms nothing and sends nothing; only the name updates.
- **P13** A confirming press that finds the start gone disarms and sends only (SS2/SS5); nothing starts.
- **P14** A repeated `keydown` never arms or confirms (a held `Enter` cannot start a laser).
- **P15** A change to the job the press would send (undo, redo, a peer's edit, the Start pick, an output setting) disarms.
- **P16** The arming press does not reopen the tooltip; a later rest or keyboard focus shows the armed name.
- **P17** A refused send while the connector holds a job reads under that job's line, in the "Not started" look; the line and bar stay (UX review 2026-11-20, tester note on the Busy refusal under "Waiting for the start at the machine").

## 11. For the product owner, the architect and the lead

- **PO:** SS13 gains P12 (an arming press that finds the start gone does nothing), P13 (the confirm then sends only, matching "only the send rule of SS2/SS5 applies"), P14 (key repeat) and P15 (a changed job disarms) as disarm and press rules; SS9's armed name stays "Press again to send to machine and start". SS12's "armed look" is P11.
- **Architect:** the "will it start" function is called at the arming press and at the confirming press (P4); the arm itself is the existing `useArm` / `useDeleteArm` (one more user, no new hook); the disarm on a job change (P15) needs a cheap signal that the page's job inputs changed (the document revision and the output settings, not a job build).
- **Amends `0125` UX notes** (lines added there, this branch): section 2 glyph and name, the armed press and look; section 5 focus; section 8 keys; section 11 U1, U2; section 10 design-system rows.
- **Amends `0118` UX notes** (on `story/machine-control`; the lead adds the line there): section 5 table gains "Not started: <reason>" under the Ready line and the member line "Waiting for <owner> to confirm" (amended by 0134).
- **Design-system rows**, to land with the build:
  - "Send to machine button (`0125`)": "Lucide `CirclePlay` (was `Upload`, `0134`); name, tooltip and `aria-label` 'Send to machine and start' when the core's will-it-start function says so for the last status, otherwise 'Send to machine'; the glyph never changes with the state. When the name is 'start' it is an **armed press** (row 'Armed action', accent case): the first press arms and sends nothing (fill and border `--armed-accent`, white glyph, hover unchanged, name and tooltip 'Press again to send to machine and start', a polite status repeats it, no tooltip opens on the press), a press within 400 ms is ignored, the second press after a fresh status check sends and starts; it disarms silently after 4 s, on Escape, a press elsewhere, focus leaving the button, a status or checks change that ends the 'start' name, or a change to the job; a repeated keydown is ignored; the arming press keeps focus. A press never does more than its name: an arming press that finds the start gone does nothing, a confirm that finds it gone sends only. Busy through 'Sending' and 'Starting'; focus stays or returns as before."
  - "Armed action", accent case: done in `docs/design-system.md` (Accent exception names Update profile and Send to machine; "Used by" gains Send to machine (`0134`)).
  - `--armed-accent` token row (from #396): done; "Used by" names the armed Send to machine (`0134`); white glyph 4.54:1, fill 3.32:1 on `--panel-bg`.
  - "Machine controls (Operations tab) (`0118`)": "the job block shows 'Not started: <reason>' in the failure look under 'Ready: ...' after a send and start that did not start (`0134`); 'Waiting for <owner> to confirm' (`0128`)." "Hidden, not disabled": unchanged. No new token.

## Links

Spec: `specs/0134-send-and-start/specification.md`. Notes: `specs/0125-send-to-machine/ux-notes.md`, `specs/0118-machine-control/ux-notes.md` (branch `story/machine-control`), `specs/0064-machine-connector/ux-notes.md`, `specs/0081-inline-delete-confirm/ux-notes.md`, `specs/0133-fix-pause-while-homing/ux-notes.md`, `specs/0026-machine-profile/ux-notes.md` (Update profile, amended by `0130`). Code: `frontend/src/components/panel/SendToMachine.tsx`, `frontend/src/components/panel/OutputGroup.tsx`, `frontend/src/components/panel/MachineControls.tsx`, `frontend/src/hooks/useArm.ts`, `frontend/src/hooks/useDeleteArm.ts`, `frontend/src/lib/deleteArm.ts`, `frontend/src/components/ui/tooltip.tsx`, `frontend/src/lib/connectorText.ts`.
