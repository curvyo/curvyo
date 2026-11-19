# 0134 Send and start: UX notes

ux-engineer, 2026-11-19. Covers criteria SS1 to SS12 of `specification.md` and its "Needed before Ready" item for the ux-engineer: the play glyph on the Output group's button, the two names and tooltips (SS9), the "Not started" line (SS6), the waiting states, focus after the press, the `0125` UX note rows. Checked against `specs/0125-send-to-machine/ux-notes.md` (button row, disabled causes, busy, focus), `specs/0118-machine-control/ux-notes.md` (on `story/machine-control`, PR #399: the Machine controls footer, its toolbar with Start / Resume `Play` and Send job... `FileUp` (W5), the one state line by Frage 262 A, live regions), `specs/0064-machine-connector/ux-notes.md` sections 3, 7, 8, `specs/0133-fix-pause-while-homing/ux-notes.md`, the built `frontend/src/components/panel/SendToMachine.tsx` and `MachineControls.tsx` (PR #399), and `docs/design-system.md` (rows "Send to machine button (`0125`)", "Machine controls (Operations tab) (`0118`)" as proposed, "Hidden, not disabled"; Interaction conventions "No notice after a success", "Nothing moves under the pointer"). Customer rules applied: no popups, no dialog, one press (Frage 263 A default), symbol buttons with tooltip and accessible name, labels that name a control or a state only, no operating tips (2026-10-16), no notice after a success (2026-10-31). Choices marked (Proposal) are vetoable at the demo; section 10 lists them as P1 to P10. No question for the customer.

## 1. What changes where

- **Output group, button row** (`0125` section 2): the Send to machine button gets a play glyph and a second name. Place, size, frame, secondary look, disabled look and cause order stay as built.
- **Machine controls footer** (`0118`): its state line shows the results of the one action ("Sending <name>", "Running", the waiting lines, "Not started: <reason>"), the one line of the tab by Frage 262 A. Its toolbar is unchanged.
- Nothing else changes. No new token; one new glyph use (`CirclePlay`, P1).

## 2. Glyph: two play buttons in one tab, told apart (SS12)

The Operations tab now holds two buttons that start a job: the page's button in the Output group and Start / Resume in the footer's toolbar (`0064` `Play`). They do different things: the page's button **sends this page's job** and starts it only on a free machine; Start starts **whatever job the connector holds** (also one sent by Send job... or another app). `0118` W5 set the rule that two different actions in one tab do not share a glyph.

| Button | Glyph | Changes? |
|---|---|---|
| Machine controls, Start / Resume | Lucide `Play` | no: the media convention, `0064`, as built |
| Output group, Send to machine | Lucide **`CirclePlay`** (play in a circle), 16px, 1.5px absolute stroke, `currentColor` | yes: was `Upload` |

- **Why `CirclePlay` (P1).** It is the play symbol the customer asked for, and it is not the toolbar's bare triangle: one glyph family for "run", the circle marking the page's one-shot "send and run". It stays the right partner of Export's `Download` in the row (both are glyphs with an outline shape, both in the 32px outline frame).
- **Other differences that already hold:** place (the Output group in the column vs the fixed footer), frame (32 x 32 outline button with a 1px `--field-border` next to Export vs a borderless 28px toolbar button), names ("Send to machine and start" / "Send to machine" vs "Start" / "Resume").
- **Veto default:** `Play` on both (the spec's wording); then place, frame and name tell them apart.
- **The glyph does not change with the state (SS8).** It stays `CirclePlay` whether the next press will start or only send, and while disabled; while sending it is `LoaderCircle` as built (`0125` section 3). A glyph that flips between play and upload as jobs come and go would change the button under the pointer each time another app sends; the name carries the difference (section 3).

## 3. Names and tooltips (SS9)

| The app's last status says | Name, tooltip line 1, `aria-label` |
|---|---|
| Owner (every paired app until `0128`) or member (P3), connector `Idle` or `Done`, controller Idle (no Door, Check, Sleep), no job held or waiting, list empty (`0132`) | **"Send to machine and start"** (25 characters) |
| Anything else: a job held or waiting, Alarm, Door, Check, Sleep, NoController, no status read yet, Connection lost | **"Send to machine"** |

- **One function, one frame.** The name comes from the core's "will it start" function (SS11) over the last status; tooltip, `aria-label` and the disabled description's first line are that one string, set in the same render, so they never disagree.
- **Without surprising (P2).** The name changes only when a connector status changes it (a job ends, another app sends, the role arrives); never on hover, focus or a timer. The button does not move or change size (25 characters fit a tooltip line; the button has no label). An open tooltip updates its text in place: no close and reopen, no animation, no flash. A name change is not announced (it is not a live region); the state line's own live region already announces the state change that caused it.
- **`--require-local-start` and member:** the app sends Start there too (SS3), so if the status tells the app that it is a member or that the connector wants the local start, the name still reads "Send to machine and start" (the press does start, the connector holds it for the confirmation) (P3). Until the status carries the flag, the app cannot know; the name follows criterion 1 only.
- **Disabled** (SS8, `0125` S2 to S5, S8): the two-line tooltip of `0125` section 3, line 1 the name of the table above, line 2 the cause in the same order as built ("<check text>", "Needs the desktop app", "The connector does not speak <format>", "Not paired", "No longer paired", "This connector's key changed", "Sending"). Without a pairing there is no status, so the name is "Send to machine". The glyph stays `CirclePlay`; the disabled look is Export's (`--field-disabled-bg`, `--field-disabled-fg`), as built.

## 4. The press: when does it start (SS1, SS2, SS9)

The press re-reads the status (SS9). **(P4, a safety rule, for the PO)** The press starts only when the fresh status **and** the name the maker saw both say it will start: a fresh status can turn "and start" into "only send", never "only send" into "and start". A maker who read "Send to machine" never gets a running laser from that press, also when a job ended in the same instant. "Never from the label" (SS9) stays true in the other direction: a label saying "and start" never starts a machine the fresh status shows busy.

## 5. Busy and the state line during the action (SS1, SS3, SS6)

The action has two steps; the button is busy through both, so one press reads as one action (SS10).

| Step | Page button | Footer state line (visible) |
|---|---|---|
| Sending, until `JobAccepted` (`0125` S6 to S8) | busy, as built: `LoaderCircle`, `aria-busy`, `aria-disabled`, tooltip line 2 "Sending" | "Sending <name>" |
| `JobAccepted` with `Ready`, `Start` sent, until the connector's answer (the 5 s answer limit as built) | **still busy**, tooltip line 2 **"Starting"** (P5) | what the connector reports: "Ready: <name>, <n> lines, <size>" for the moment it holds the job |
| Started | not busy; name "Send to machine" (the machine now holds a job) | "Running" (counts line with the name, bar, elapsed) |
| `--require-local-start` | not busy | "Waiting for the start at the machine" (as built, bar at 0) |
| Member (`0128`) | not busy | "Waiting for <owner> to confirm" (`0128` text; the owner's name cut at its end with an ellipsis so the line stays within 70 characters) |
| Start refused or failed (SS6) | not busy | "Ready: <name>, <n> lines, <size>" and under it "Not started: <reason>" (section 6) |
| Press only sent (a held or waiting job) | as `0125` | `0125` result as built ("Connector busy" until `0132`; then the list shows the added job) |

- **Nothing optimistic.** The line shows only what the connector reports; "Running" comes with the connector's Running.
- **The footer's toolbar during the Start step (P6):** Start / Resume is greyed (its own Start is the one in flight), the other buttons keep their row: Stop is enabled in Ready (it discards, as built) and in Running. If Stop discards the job before the Start answer, the Start is refused, the job is gone, the line follows the connector (no "Not started" without a Ready job).
- **The Lasered commit** (SS7) happens on `Ready` as built and shows nothing new (no notice after a success; the fixture-off notice of `0125` section 6 when N > 0, unchanged).
- **Homing first (SS4):** the job's own `$H` reads "Running" at line 0 as built (`0127` section 7); a Pause there follows `0133` ("Pause after homing").

## 6. "Not started: <reason>" (SS5, SS6)

- **Where (P7).** In the footer's job block, a second 12px line directly under the state line "Ready: <name>, <n> lines, <size>", in the failure look (`CircleAlert` 12px, `--destructive`, 4.8:1 on `--panel-bg`); wraps, at most two lines. The footer grows upward as all its parts do, so the toolbar never moves (`0118` W2). The Output group shows nothing (it has no state line, Frage 262 A).
- **Words.** "Not started: " and the connector's or the app's own text, as built elsewhere, capital after the colon as "Alarm: <text>": "Not started: Connector busy", "Not started: Controller state: Door", "Not started: Timed out", "Not started: Lost the controller", "Not started: Connection lost". "Laser mode is off ($32=0)" refuses the send itself (SS5) and reads as the `0125` refusal, not as "Not started". A state, never an instruction: no "Press Start", no "Try again".
- **When.** (a) `Start` refused or failed after `JobAccepted` while the job stays `Ready` (SS6). (b) The press only sent on a machine that holds no job, because a controller state blocks the start (Door, Check, Sleep; SS5): "Not started: Controller state: Door", so the maker learns why the laser did not move. (c) The connection dropped after `JobAccepted`: after the reconnect the fresh status decides; Running means it started (no text), Ready means "Not started: Connection lost". Not when the press only sent because a job was held: that result is the `0125` refusal or the `0132` list.
- **How long.** Until the next state change of the connector, the next press of the page button, or a press of the footer's Start. It leaves without a trace; it is never a notice and never a History step.
- **Live.** A hidden `role="alert"` (it answers the maker's own press), once; the Ready line before it is not announced separately (section 7).
- **The footer's Start is enabled** (SS6) by the Ready row as built; nothing points to it (no tip).

## 7. Live regions (SS1, SS3, SS6)

The two hidden regions of `0118` section 11 carry the footer line; the visible line has no live role.

- During the one action the polite region **does not announce the in-between "Ready: ..."** when the Start step follows (P8); it announces "Sending <name>" at the start and then the result once: "Running", "Waiting for the start at the machine", "Waiting for <owner> to confirm", or, as an alert, "Not started: <reason>" (the Ready line is part of that announcement: "Ready: <name>, <n> lines, <size>. Not started: <reason>").
- A press that only sends announces as `0125` (the Ready line, or the refusal as an alert).
- Refusals of the send itself (`0125` section 5: "Connector busy", "Machine in alarm: <text>", "Nothing answers at <host:port>", "Timed out", "Not received") are alerts, as built.

## 8. Focus and keyboard (SS10)

- **After the press, focus stays where `0125` section 5 put it:** a keyboard press keeps focus on the page button (it is `aria-disabled` while busy, so it keeps focus and its Tab stop), a pointer press returns focus as Export does (`onReturnFocus`). **Focus never jumps to the footer or to Stop** (P9): a moving focus would surprise and Stop is always in view in the fixed footer. From the page button, the next Tab stop is the footer (Unlock when shown, then the toolbar; End in the toolbar reaches Stop).
- Enter or Space presses. **No shortcut and no global chord** (`0125` U12, now more so: the press can start a laser). Escape does nothing to the action; Stop is the way to end a started job.
- No confirmation, no armed press (SS10, Frage 263 A default). If Frage 263 is answered B, these notes gain the Armed action of `0081` on this button; nothing else changes.
- Hit target 32px (36 coarse), as built.

## 9. Checks for the review

- **States to try** (`curvyo-connector-server --fake-controller`, `--require-local-start`, `--fake-alarm-at-line`; the pty simulator with Door): free machine, one press → "Sending", "Running", one announcement; a Ready job held → name "Send to machine", press → "Connector busy"; Done → name "Send to machine and start"; `--require-local-start` → "Waiting for the start at the machine"; Door on an idle machine → name "Send to machine", press → Ready and "Not started: Controller state: Door"; Start refused (state change between) → Ready and "Not started: ..."; connection dropped after `JobAccepted`; Home first job → "Running" at line 0, Pause → "Pause after homing" (`0133`); every disabled cause of `0125` with the new glyph; the browser build ("Needs the desktop app", glyph `CirclePlay`).
- **Name and status agree** in every row of section 3; an open tooltip updates in place when another app sends.
- **Nothing moves:** the button row keeps its place and size; the footer grows upward.
- **No popups, no hint texts, no notice after a success.** Contrast: existing tokens only (`--toolbar-icon` 8.2:1, `--destructive` 4.8:1 on `--panel-bg`; disabled look exempt).

## 10. Decisions for veto (Proposal)

- **P1** The page's button takes Lucide `CirclePlay`; the footer's Start / Resume keeps `Play`. Veto default: `Play` on both.
- **P2** The name changes only with a connector status; an open tooltip updates in place; the change is not announced.
- **P3** Under `--require-local-start` and as a member the name still reads "Send to machine and start" (the press starts, the connector holds it).
- **P4** A press starts only when both the name the maker saw and the fresh status say it will start.
- **P5** The button stays busy through the Start step, tooltip line 2 "Starting".
- **P6** The footer's Start / Resume is greyed during the Start step; Stop keeps its row.
- **P7** "Not started: <reason>" is a second failure-look line under "Ready: ..." in the footer's job block, also when a controller state blocks the start of an otherwise free machine.
- **P8** The in-between "Ready: ..." is not announced during the one action; the result is announced once.
- **P9** Focus stays on the page button (keyboard) or returns as Export's (pointer); it never jumps to the footer.
- **P10** No shortcut for the button.

## 11. For the product owner, the architect and the lead

- **PO, "What changes in the demo" and SS12:** "Lucide `Play`" becomes `CirclePlay` (P1). SS6 "the Output state line reads 'Ready: ...'" becomes the footer's state line (Frage 262 A). SS9 gains P4 (the start needs both the shown name and the fresh status). SS5 gains "Not started: Controller state: <word>" (P7). SS3 and P3: the name under `--require-local-start` and for a member.
- **Architect:** the "will it start" function takes the last status (state, controller state, role, `--require-local-start` if the status carries it, list) and returns the name's case; the press calls it twice, with the status the name came from and with the fresh one (P4). "Starting" and "Not started: <reason>" need the Start refusal's reason in words from the existing refusal texts.
- **Amends `0125` UX notes** (lines added there, this branch): section 2 glyph and name, section 11 U1, section 10 design-system rows.
- **Amends `0118` UX notes** (on `story/machine-control`; the lead adds the line there): section 5 table gains "Not started: <reason>" under the Ready line and the member line "Waiting for <owner> to confirm" (amended by 0134).
- **Design-system rows**, to land with the build: "Send to machine button (`0125`)": "Lucide `CirclePlay` (was `Upload`, `0134`); name, tooltip and `aria-label` 'Send to machine and start' when the core's will-it-start function says so for the last status, otherwise 'Send to machine'; the glyph never changes with the state; busy through 'Sending' and 'Starting'; a press starts only when the shown name and a fresh status both say so; focus stays or returns as before." "Machine controls (Operations tab) (`0118`)": "the job block shows 'Not started: <reason>' in the failure look under 'Ready: ...' after a send and start that did not start (`0134`); 'Waiting for <owner> to confirm' (`0128`)." "Hidden, not disabled": unchanged. No new token.

## Links

Spec: `specs/0134-send-and-start/specification.md`. Notes: `specs/0125-send-to-machine/ux-notes.md`, `specs/0118-machine-control/ux-notes.md` (branch `story/machine-control`), `specs/0064-machine-connector/ux-notes.md`, `specs/0133-fix-pause-while-homing/ux-notes.md`. Code: `frontend/src/components/panel/SendToMachine.tsx`, `frontend/src/components/panel/MachineControls.tsx`, `frontend/src/lib/connectorText.ts`.
