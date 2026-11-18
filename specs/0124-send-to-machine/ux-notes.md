# 0124 Send to machine: UX notes

ux-engineer, 2026-11-17. Covers criteria S1 to S18 of `specification.md` and its list "Needed before Ready". Checked against `specs/0029-laser-job-preview-and-output/ux-notes.md` 6.3 to 7 (output group, Export, export notice), `specs/0064-machine-connector/ux-notes.md` sections 3, 4, 8 and 9 (symbol buttons, link line texts, job block, unavailable states), the built `frontend/src/components/panel/OutputGroup.tsx` and `frontend/src/components/workbench/ConnectorGroup.tsx`, and `docs/design-system.md` (rows "Hidden, not disabled", "Tooltips and validation chips", "Action notice", "Connector group (`0064`)"; Interaction conventions "No notice after a success", "Nothing moves under the pointer"). Customer rules applied: no popups, no dialog, symbol buttons with tooltip and accessible name, labels that name a control or a state only, no operating tips. Choices marked (Proposal) are vetoable at the demo; section 11 lists them as U1 to U12.

## 1. Where it lives

- The Output group of a laser page's Operations tab (`0029` 6.3), last element: the **button row** (Export job file and Send to machine), then the **state line** and the **progress bar** under it. Nothing else in the group, the tab, the Machine block or the Connector row changes.
- The Send button exists only when the page's machine profile copy names a connector address (S1). Without one the button row is Export alone at full width, exactly as built. "Laser (off)" pages and other page kinds have no output group at all (`0029` 13).
- The state line and bar exist only for a profile with a connector address and a pairing, on the desktop (S13). The browser build never shows them (section 7).

## 2. Button row (S1)

| Part | Look |
|---|---|
| Row | one flex row, full content width, 4px gap, height of Export (32px; 36 coarse) |
| Export job file | as built (`0029` 6.6: Primary, `Download` 16px and "Export job file" 13px), now `flex: 1` (208px in the 244px content width; the words fit on one line) |
| Send to machine | square symbol button, 32 x 32 (36 x 36 coarse), right of Export, `rounded-[5px]` as Export |

**Send to machine button.**
- Glyph: Lucide **`Upload`**, 16px, 1.5px absolute stroke, `currentColor` (U1). It is the glyph of the Connector row's "Send job..." (`0064` 3): the same verb to the same target gets the same symbol, and it reads as the counterpart of Export's `Download` (to this computer / to the machine).
- Look: a secondary (outline) button, not a second Primary: `--panel-bg` ground, 1px `--field-border`, glyph `--toolbar-icon`, hover `--editor-accent-hover`, focus ring 2px `--editor-accent` with 1px offset (Export's ring) (U2). One Primary per group stays the export.
- Name, tooltip and accessible name "Send to machine" (S1). Tooltip: Radix, 400ms, `side="top"` like Export's, so it never covers Export.
- No label text, no shortcut text, no "..." (it opens nothing).

## 3. Disabled and busy states (S2 to S5, S8)

**Disabled = in place, focusable (S2).** `aria-disabled="true"`, keeps its Tab stop and tooltip, a press does nothing and sends nothing. Look: Export's disabled look (`--field-disabled-bg` ground, `--field-disabled-fg` glyph, no border), so the two buttons read as disabled together when the checks block (U3). Not 40% opacity: Export never was.

**Tooltip while disabled:** two lines, line 1 the name "Send to machine" (13px), line 2 the cause (12px `--panel-muted-fg`); the precedent is the Arc kind buttons' second muted line. The accessible name stays "Send to machine"; `aria-describedby` points to an sr-only span with the cause, as Export does. A cause is a state, not an instruction: no "pair it first", no "use the desktop app to send" (customer rule of 2026-10-16).

**Causes, first one that applies wins (U4):**

| # | When | Cause (line 2 and description) | Export |
|---|---|---|---|
| 1 | A check blocks (S2; `0029` 4 a to d) | the first block's text, the same string Export shows | disabled, same cause |
| 2 | Browser build (S5) | "Needs the desktop app" | enabled (download) |
| 3 | The output format is one the connector does not speak (S4) | "The connector does not speak <format name>" | enabled |
| 4 | No pairing for this profile (S3; also "No credential store on this computer") | "Not paired" | enabled |
| 5 | The token was revoked (S3, `0064` Q9) | "No longer paired" | enabled |
| 6 | The pinned key does not match (S3, `0064` Q8) | "This connector's key changed" | enabled |
| 7 | A send of this page is running (S8) | "Sending" (busy look, below) | enabled |

- Order reason (architect addendum B2, the code is the reference): what Export shares comes first (one cause on both buttons, S2), then the platform, then what holds for this format whatever the pairing, then the pairing, whose way on is the Pair row of the Connector row; a send under way last.
- Reachability is never guessed (S3): a paired profile has Send enabled; "Nothing answers at <address>" comes only after a press (section 5).
- The cause updates in the same frame as the checks; a cause change never moves the button or the focus.

**Sending (busy, S8).** From the press until `JobAccepted`, `JobRejected`, a failure or, for a send whose answer was lost with the connection, 30 s (the line then reads "Timed out", Send is enabled again and a late `Ready` of that job still commits once; Proposal): Send is `aria-disabled`, `aria-busy="true"`, its glyph is `LoaderCircle` turning once per second (the `0064` busy rule; it turns under `prefers-reduced-motion` too, it is the only sign of work); tooltip line 2 "Sending". Export stays enabled. The state line reads "Sending <name>" (section 4). Nothing optimistic: the line changes to Ready only when the connector says so.

## 4. State line and progress bar (S13 to S15)

**Place.** Under the button row, 6px gap; the bar 4px under the line. Both come after the buttons, the last things in the tab, so appearing and leaving moves nothing above them: the buttons never shift (S13). No reserved row while absent (no empty height, no animation).

**Line.** 12px, `--toolbar-icon`, wraps (`overflow-wrap: anywhere`), no glyph in the neutral states. Texts are those of S13 and the Connector row (`0064` 8), one line of at most 70 characters, the job name cut at its end with an ellipsis to fit (U5); the live region (below) carries the full name, the line has no tooltip.

| Connector state | State line | Bar |
|---|---|---|
| Receiving (this page's send) | "Sending <name>" | none |
| Ready (S9) | "Ready: <name>, <n> lines, <size>" | none |
| AwaitingLocalStart | "Waiting for the start at the machine" | at 0 |
| Running | "Running <line> / <lines>" | fill to line / lines |
| Paused | "Paused at line <n>" | held |
| Done, Completed | "Completed, <n> lines" | full |
| Done, Stopped | "Stopped at line <n>" | held |
| Done, Failed | "Failed at line <n>" | held, `--destructive` fill |
| Alarm | "Alarm: <text>", `TriangleAlert` 12px, `--destructive` | held, `--destructive` fill, if a job ran |
| Connection lost (S14) | "Connection lost", `CircleAlert` 12px, `--destructive` (the `0064` link line look) | the last values, held |
| Idle, NoController, no status read yet | line absent | absent |

- **A job not sent from this page (S15).** When the held job's id is not the last one this page sent in this session, the line adds " · <name>" after the state words in the active and done states ("Running 120 / 412 · layer-test.gcode"), so the maker sees it is another job; Sending and Ready already name it (U6).
- **Numbers and sizes** as the Connector row: thousands comma ("5,000"), sizes in binary units ("812 B", "9.2 KiB", "3.5 MiB", one decimal under 10) (U7).
- **Bar.** The Connector row's bar (`0064` 8.2): full content width x 6px, `rounded-full`, white track with 1px `--field-border`, fill `--editor-accent`, `--destructive` in Failed and Alarm; `role="progressbar"` named "Job progress", `aria-valuemin` 0, `aria-valuemax` lines, `aria-valuenow` line, `aria-valuetext` "120 of 412 lines". Width changes without a transition. No elapsed time and no counts line here: the Connector row has them (U8).
- **No buttons (S15).** The line and bar are text and a meter: no Start, Pause, Resume or Stop, no link to the Connector row, no Tab stop.
- **One source (S14).** The line and the Connector row read the same status object, so they always show the same values; the 5 s retry after "Connection lost" is the Connector row's one retry.

**Live region (U9).**
- The visible line and the bar are not live (`aria-live="off"`): a running job changes the count up to 5 times a second.
- One sr-only `role="status"` (polite) next to the line is in the DOM whenever the button row has a Send button, empty while there is nothing to say, so an appearing text is announced. It carries only the **state words** and changes only on a state change: "Sending <name>", "Ready: <name>, <n> lines, <size>", "Waiting for the start at the machine", "Running", "Paused at line <n>", "Completed, <n> lines", "Stopped at line <n>", "Failed at line <n>", "Connection lost".
- A failure that answers the maker's own press (section 5) and "Alarm: <text>" are also written to a hidden `role="alert"` (the `0064` rule). A status read when the group is first shown is never an alert.
- The Connector row and the Output group sit in different tabs, so only one of them is mounted and announces a change.

## 5. After a press: texts in the state line (S7, S9, S11)

All failures use the failure look (`CircleAlert` 12px, `--destructive`, 4.8:1 on `--panel-bg`), stay until the next send or the next connector state change, and are also alerts (section 4). Nothing is committed (S11). No popup, no dialog, no notice.

| When | State line |
|---|---|
| Connector refuses (`0064` Q13) | "Connector busy" / "No controller" / "Not plain ASCII text" / "Larger than 64 MiB" / "Hash mismatch" |
| Busy while the connector is in alarm (`0064` Q19) | "Machine in alarm: <text>" |
| Cannot be reached | "Nothing answers at <host:port>" (the address as dialled) |
| No answer in 5 s (`0064` text) | "Timed out" (U10) |
| Lost pairing on the press | "No longer paired" / "This connector's key changed"; Send then shows that cause (section 3) and the Connector row its Pair row |
| Connection dropped after the last chunk, the re-read shows another job or Idle (S11) | "Not received" |
| Accepted | "Ready: <name>, <n> lines, <size>" (the result, no extra text) |

- Focus after a send: a keyboard press keeps focus on Send (it is `aria-disabled` while sending, so it keeps focus and its Tab stop); a pointer press returns focus as Export does (`onReturnFocus`). Focus never jumps to the Connector row.
- A page that has become blocked at the moment of the press (S6) sends nothing and shows nothing new: the button is already disabled with its cause.

## 6. Notice after an accepted send (S10)

- **(U11, differs from S10's text)** No "Sent <name> to <connector name>." line: the state line turning to "Ready: …" in the group the maker pressed in is the visible result, and the customer rule "No notice after a success" (2026-10-31) leaves only the export's notice, which exists because nothing else shows after a save dialog.
- When fixture jobs were switched off (N > 0), one canvas notice (the `0029` 7 notice component, `role="status"`): "Output turned off on N fixture jobs." ("1 fixture job"), 3 s. It reports a change to the project that undo can take back, which the maker has to know.
- History step label "Send job to machine" (S10); none when nothing changed (S12).

## 7. Browser build (S5)

- Send is shown (when the profile names a connector address) in the disabled look with the tooltip "Send to machine" / "Needs the desktop app", focusable, a press does nothing. Export downloads as built. No state line, no bar, no live region text. The Connector row shows its own "Needs the desktop app" as built (`0064` 9).
- This is a recorded exception to "Hidden, not disabled": the button is greyed, not hidden, because the profile does name a machine and the maker should see that sending exists and why it does not work here (spec S5).

## 8. Keyboard and focus

- **Tab order in the Output group:** Start pick, each check's button, Export job file, Send to machine. Two separate Tab stops, no toolbar (Export is a word button); disabled ones keep their stop.
- Enter or Space presses Send. No shortcut and no global chord (U12; a job reaching a machine from a stray key is worse than a Tab). Escape does nothing to a running send (it cannot be taken back halfway; the connector discards an incomplete job).
- Tooltips appear on keyboard focus as on hover, with the cause line when disabled.
- Hit target 32px (36 coarse).

## 9. Checks for the review

- **Contrast:** existing tokens only. Glyph `--toolbar-icon` 8.2:1 on `--panel-bg`; `--field-border` around Send as the fields; failure text `--destructive` 4.8:1; disabled look exempt (inactive) and carries its name and cause.
- **States to try** (fake connector, `--fake-lines-per-second`, `--fake-alarm-at-line`, `--require-local-start`, `--revoke`, a replaced state directory, the connector stopped): every row of sections 3, 4 and 5; a check that blocks both; a second send of an unchanged page; a job sent by the Connector row's "Send job..." (line with " · <name>"); the browser build.
- **Nothing moves:** the button row keeps its place and size in every state; the state line only adds height below it.
- **No popups, no hint texts:** the only floating layers are the tooltips and the fixture-off notice.
- **Screen reader:** state words once per change; counts not live; failures of a press as alerts.
- **Large documents:** the job is built only on the press (as Export); nothing here reads the drawing on hover or focus.

## 10. Design-system rows to add after acceptance

- **"Send to machine button (`0124`)"**, after "Output group (`0029`)" (or after "Connector group (`0064`)" while that row is not yet in): sections 2, 3 and 8 (glyph `Upload`, secondary look 32 x 32 next to a `flex: 1` Export, disabled look of Export, two-line tooltip with the cause, the cause order, busy `LoaderCircle`).
- **"Connector state line (`0124`)"**, after it: section 4 and 5 (one line, 70 characters, the bar of the Connector group, " · <name>" for another job, sr-only state-word live region, failures as alerts).
- In "Hidden, not disabled", the Output tab exceptions gain: "Send to machine is `aria-disabled` with its cause in the tooltip's second line, also in the browser build (`0124`)".
- No new token, no new glyph.

## 11. Decisions for veto (Proposal)

- **U1** Glyph `Upload`, the Connector row's "Send job..." glyph.
- **U2** Send is a secondary outline symbol button; Export stays the one Primary.
- **U3** Disabled Send uses Export's disabled look, not 40% opacity.
- **U4** One cause shown, in the order of section 3; tooltip line 2, sr-only description.
- **U5** Job name cut at its end with an ellipsis so the line stays within 70 characters.
- **U6** A job not sent from this page adds " · <name>" to the line.
- **U7** Sizes in binary units as the Connector row ("9.2 KiB", not "9 KB").
- **U8** No elapsed time and no counts line in the Output group.
- **U9** Visible line not live; an sr-only status with the state words only.
- **U10** "Timed out" after 5 s without an answer, as the Connector row.
- **U11** No "Sent …" notice; only "Output turned off on N fixture jobs." when N > 0.
- **U12** No keyboard shortcut for Send.

## 12. Mismatches for the product owner

- **S10 notice:** U11 replaces "Sent <name> to <connector name>." (and its second line) by the fixture-off line alone; S10's text and test follow the answer. Safe default if vetoed: the spec's two-line notice, 3 s / 5 s as `0029` P19.
- **"What changes in the demo" and S13:** "9 KB" reads "9.2 KiB" (U7); add "Timed out" (U10) and "Not received" to S7's list of texts; S15 gains the " · <name>" rule (U6).
- **Words:** "Paused at line <n>" and "Running <line> / <lines>" (S13) differ from the Connector row's "Paused" and "Running" with a counts line; both stay, each place has its own layout, and the values are equal (S14).

## Links

Spec: `specs/0124-send-to-machine/specification.md`. Built notes: `specs/0029-laser-job-preview-and-output/ux-notes.md`, `specs/0064-machine-connector/ux-notes.md`. Code: `frontend/src/components/panel/OutputGroup.tsx`, `frontend/src/components/workbench/ConnectorGroup.tsx`.
