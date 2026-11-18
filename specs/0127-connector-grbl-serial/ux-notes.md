# 0127 Connector speaks GRBL over serial: UX notes

ux-engineer, 2026-11-18. Covers criteria G1 to G27 of `specification.md` and its list "Needed before Ready" (ux-engineer item). Checked against `specs/0064-machine-connector/ux-notes.md` (Connector group: sections 2 to 4, 7 to 11), `specs/0125-send-to-machine/ux-notes.md` (state line, sections 4 and 5), `specs/0026-machine-profile/ux-notes.md` (Machine block, Connection sub-group, section 2 item 6), the built `frontend/src/components/workbench/ConnectorGroup.tsx` and `frontend/src/lib/connectorText.ts`, and `docs/design-system.md` (rows "Connector group (`0064`)", "Connector state line (`0125`)", "Schema-rendered workbench tab" (Notice look), "Hidden, not disabled"; Interaction conventions "No notice after a success", "Nothing moves under the pointer"). Customer rules applied: no popups, no dialog, no operating tips or hint labels (labels name a control or a state only), symbol buttons with tooltip and accessible name. Customer answers of 2026-11-18 are applied: Frage 238 A (flags only), 239 B (Unlock button in the app), 240 B (refusal at `$32=0`), 241 B (no terminal prompt); design principle: the connector is usually a systemd service, no new behaviour needs a person at its terminal. Choices marked (Proposal) are vetoable at the demo; section 12 lists them as V1 to V14.

## 1. What changes where

- **Connector row** (the Connector sub-group of the Machine block, `0064` section 2): the **Controller block** (section 2), new words in the job block's state line (sections 4 to 7) and one new button, the **Unlock** symbol button, shown only in `Alarm` (section 6). No change to the heading row, link line, trust block, Pair row or job toolbar.
- **Output group state line** (`0125` section 4): new words only (section 8). No new element.
- **Machine block, Connection fields** (`0026`): nothing changes in look or behaviour (section 3).
- **Connector terminal** and `--help`: new lines (section 10).
- Nothing else in the app changes. The fake controller (`--fake-controller`) shows exactly what it showed before, plus the one Controller row its `Welcome` already carries (section 2).

## 2. Controller block (G2, G4, G19)

**Place (V1).** In the Connector group, **after the job block**, as the last part of the group (row 8 after `0064` section 2's rows 1 to 7). Reason: it appears and goes with the controller (identify, loss, reconnect), and a block that appears above the job toolbar would move Start, Pause and Stop under the pointer. It has no controls, so what moves when the job block appears above it is text only.

**Shown when** the profile is paired (key not changed), the last `Welcome` of this connector named a controller, and the last `Status` is not `NoController`. Absent otherwise: no reserved height, no animation. Unpaired, the link line of a passing test still names the controller as built ("Connected, controller answered: GRBL, protocol 0, connector 0.2.0").

**Layout.** The trust block's grid (`0064` section 2 item 3): label column 80px, 12px `--panel-muted-fg`; value column the rest (164px), 12px `--toolbar-icon`, tabular figures, `overflow-wrap: anywhere` (long `/dev/serial/by-id/...` paths wrap, no ellipsis), selectable text so a port path can be copied. 4px above it. One row per known value, top to bottom; a row whose value the controller did not report is absent.

| Row label | Value | Source |
|---|---|---|
| Controller | "GRBL 1.1h" (name, a space, firmware); fake: "Fake controller curvyo-fake 0" | `ControllerInfo` name and firmware |
| Port | "/dev/ttyUSB0, 115200 baud" | the port and baud the connector uses (G19); absent for the fake |
| State | GRBL's word: "Door", "Check", "Home", "Sleep", "Jog" | G4; only while GRBL reports a state outside Idle, Run, Hold and Alarm (V2) |
| Laser mode | "On" / "Off" | `$32` |
| Max power | "1,000" (thousands comma, rounded to a whole number) | `$30` (V3: the profile's word for S-max, so the two read alike) |
| Travel | "400 × 300 mm" (one decimal only when not whole: "406.4 × 304.8 mm") | `$130`, `$131` |
| Position | "X 120.0, Y 45.3 mm" (one decimal, tabular) | `Status.position`, work position (G4); absent while `null` (V4) |

- **Semantics.** A `dl` inside a `div` named "Controller" (`aria-label`); each row a `dt` / `dd` pair, so a screen reader reads "Laser mode, On". Not a live region: Position changes up to 5 times a second and is `aria-live="off"`.
- **During homing inside a job** (G9): the job block reads "Running" with line 0, the State row reads "Home". Nothing else.
- **Freshness.** The values are those of the identify the connector reports now: after a loss and a reconnect (G3) the block shows the new identify's values, never the ones from before. The architect decides how the app learns them (a fresh `Welcome` when a pushed status leaves `NoController`, or the info inside `Status`); the UX rule is "never stale".

**Settings warning (G19).** Under the grid, 4px gap, at most two lines, each a 12px `TriangleAlert` (12px, `--toolbar-icon`) and 12px `--toolbar-icon` text: the Notice rule of the design system, "permitted, not an error", so never `--destructive`. The comparison uses the values the Machine block shows (the page's profile copy, which the job is made from).

| When | Line (characters) |
|---|---|
| `$32=1`, the copy's Laser mode is off | "Laser mode differs: profile off, controller on" (46) |
| rounded `$30` differs from the copy's Max power | "Max power differs: profile 1,000, controller 255" (48) |

- Laser line first, then the power line. Start stays enabled; nothing is armed or confirmed.
- The two lines sit in one `role="status"` (polite) container that is always in the DOM while the block is, empty when nothing differs, so a warning that appears after a reconnect or a profile edit is announced once. A status read on opening the block is never an alert (`0064` section 4).
- No warning for travel versus the profile's work area, nor for port or baud (V5; G19 names S-max and laser mode only).

**Laser mode off is a refusal, not a warning (G34, Frage 240 B).** With `$32=0` the connector refuses `SubmitJob` (`RejectReason::LaserModeOff`). Text, 25 characters, a state: **"Laser mode is off ($32=0)"**.
- Output group state line (`0125` section 5, the row "Connector refuses"): shown as the other refusals are, "Laser mode is off ($32=0)" in the failure look (`CircleAlert`, `--destructive`, polite status); Send to machine is enabled again at once (as after "Connector busy").
- Connector row: the Controller block's Laser mode row reads "Off" and its value text is `--destructive`-free neutral (the refusal is the line in the Output group, not a second message here). There is no warning line for `$32=0` any more, no override, no confirm.
- It fits the 70-character line with room ("Laser mode is off ($32=0)" plus " · <name>" is cut first).
- **Laser mode unknown** (`$32` not read; `RejectReason::LaserModeUnknown`, architect review): the same refusal in the same places and look, text, 33 characters, a state: **"Laser mode unknown ($32 not read)"**. The Controller block's Laser mode row is absent, as for any value not reported.

## 3. Profile's Controller port and Baud rate fields (Frage 238 A)

- The Machine block's **Controller port** and **Baud rate** fields (`0026` section 2 item 6) stay exactly as built: same labels, same place, editable on saved profiles, written to the library record, ranges and notices unchanged. The app never sends them, never compares them with the connector, and shows no line, mark or tooltip about them (V5).
- What the connector really uses is the Controller block's **Port** row, a few rows below in the same Connection group, so the maker sees both values side by side. A port name on the connector's computer often differs from the one typed on the app's computer; a mismatch has no effect on the job, so it earns no warning.
- No new text in the Connection group. "Only saved profiles have a connection." stays the only line there.

## 4. Job block: controller states (G2, G3, G15, G12)

The job block's state line (`0064` section 8). New texts appear in the states listed; all other rows of `0064` section 8 are unchanged. "Failure" is the `0064` failure look: `CircleAlert` 12px at the left, `--destructive` text (4.8:1 on `--panel-bg`). Retries every 2 s (G3) change nothing on screen while the cause stays the same: the text is replaced only when it differs, and only a change is announced.

| Connector state and cause | State line | Look | Live |
|---|---|---|---|
| `NoController`, no controller flag | "No controller" (as built) | neutral | status |
| `NoController`, port not found | "Port /dev/ttyUSB0 not found" | failure | status |
| `NoController`, permission denied | "No permission for /dev/ttyUSB0" | failure | status |
| `NoController`, port in use | "Port /dev/ttyUSB0 is in use" | failure | status |
| `NoController`, any other open error | "Cannot open /dev/ttyUSB0" | failure | status |
| `NoController`, no banner in 5 s or not a GRBL banner (G2) | "No GRBL answer on /dev/ttyUSB0" | failure | status |
| `NoController`, a GRBL 0.9 banner | "GRBL 0.9 is not supported" | failure | status |
| Lost during a started job (G15 d) | "Lost the controller at line 120" | failure | **alert** |
| Lost while a job was Receiving, Ready or AwaitingLocalStart | "Lost the controller" | failure | **alert** |
| Stop without a banner within 3 s (G12) | "Could not confirm the machine stopped" | failure | **alert** |

- The four open-failure texts and their causes are new (V6): the connector knows the cause (G2 prints it on its terminal) and the maker may be at the other computer. They name a state, not what to do. The architect carries the text in the additive field that G15 adds for `NoController`.
- **Which text wins.** A loss or unconfirmed-stop text stands over the retry's open-failure text: the maker must learn that the job died before learning why the port is gone. It also stays **after the reconnect** (state `Idle`, Controller block back) until the next send or the next job state, the way a `Done` line stays (V7; spec G15 "a client that connects after the loss reads the same text"). Send job... is enabled in that `Idle` as always.
- **No bar** with a loss or unconfirmed-stop text: the job was dropped, the line number is in the words (V7).
- "Lost the controller" (connector lost the machine) and "Connection lost" (`0064`: the app lost the connector) are different words for different links; both can be true at once only through the app's own retry, and then "Connection lost" shows (the app cannot know more).
- No "Retrying", no counter, no spinner while the connector retries (V8): the state does not change, so the line does not.

## 5. Job block: failed lines (G5, G7)

Done, Failed (the `0064` row "Failed at line <n>") gains the cause after a colon. Look unchanged: failure, bar held with the `--destructive` fill. A polite status, not an alert (V9): a failed line is a job result like Stopped, and the stop of G12 has already switched the laser off.

| Cause | State line |
|---|---|
| GRBL `error:<n>` with a known number | "Failed at line 12: No feed rate (error 22)" |
| GRBL `error:<n>`, unknown number | "Failed at line 12: error 18" |
| G5, line longer than 79 after preparation | "Failed at line 12: Longer than 79 characters" |
| G5, a `$<n>=`, `$RST` or `$N` line | "Failed at line 4: Changes a machine setting" |
| G5, a `$X` line | "Failed at line 4: Unlocks the machine" |
| G5, `!`, `~` or `?` left after the comment removal | "Failed at line 9: Contains !, ~ or ?" |
| The fake controller (no cause) | "Failed at line 12" (as built) |

- The number after "line" is the job line GRBL rejected or the prepared line that failed the check (G7), with a thousands comma. The bar is held at the acknowledged count.
- The G5 texts follow the GRBL pattern "Failed at line <n>: <cause>" instead of the spec's "Line 12 is longer than 79 characters" (V10), so every failed job reads one way in the row, the Output group and the terminal.
- Every text is at most 69 characters with a 5-digit line number (checked against the 70-character line of `0125`), so nothing is cut.

**GRBL 1.1 error meanings** (the `-io` table of G7; a state, never advice; 18 and 19 are not defined in GRBL 1.1 and read "error 18"):

| n | Meaning | n | Meaning |
|---|---|---|---|
| 1 | Command letter missing | 20 | Unsupported command |
| 2 | Bad number | 21 | Two commands of one modal group |
| 3 | Unknown $ command | 22 | No feed rate |
| 4 | Negative value | 23 | Whole number expected |
| 5 | Homing is not enabled | 24 | Two commands need axis words |
| 6 | Step pulse under 3 microseconds | 25 | Word repeated in one line |
| 7 | Settings lost, defaults loaded | 26 | No axis words |
| 8 | $ command only when idle | 27 | Line number out of range |
| 9 | G-code locked by alarm or jog | 28 | Value missing |
| 10 | Soft limits need homing | 29 | Work offset not supported |
| 11 | Line too long | 30 | G53 only with G0 or G1 |
| 12 | Step rate over 30 kHz | 31 | Axis words not used |
| 13 | Safety door open | 32 | Arc without an axis in its plane |
| 14 | Text over 80 characters | 33 | Invalid motion target |
| 15 | Jog beyond the travel | 34 | Arc radius does not fit |
| 16 | Invalid jog command | 35 | Arc without an offset in its plane |
| 17 | Laser mode needs PWM | 36 | Value words not used |
|  |  | 37 | Tool offset on the wrong axis |
|  |  | 38 | Tool number too high |

ASCII only (the log and the terminal stay plain).

## 6. Job block: alarms (G13, G14, G17)

Look unchanged from `0064`: "Alarm: <text>", `TriangleAlert` 12px, `--destructive`, the line is `role="alert"`, the bar held with the `--destructive` fill if a job ran, every job button off (`0064` section 7). Only the text is new.

**Text form (V11).** `Status.alarm` carries "<meaning> (alarm <n>)", so the existing frame reads "Alarm: Hard limit hit, position may be lost (alarm 1)", in the order of the error lines ("... (error 22)"). The spec's "Alarm <n>: <meaning>" would read "Alarm: Alarm 1: ..." in the built frame. `jobs.log` reads `Alarm: Hard limit hit, position may be lost (alarm 1)`.

| n | Meaning (GRBL 1.1) |
|---|---|
| 1 | Hard limit hit, position may be lost |
| 2 | Target beyond the soft limits |
| 3 | Reset while moving, position may be lost |
| 4 | Probe already triggered |
| 5 | Probe did not touch |
| 6 | Reset during homing |
| 7 | Door opened during homing |
| 8 | Homing pull-off did not clear the switch |
| 9 | Homing did not find the switch |
| other | "Unknown alarm (alarm <n>)" |

- **Alarm at power-up or after a reset with homing on** (G14, G17; GRBL's `[MSG:'$H'|'$X' to unlock]` with state `Alarm` and no number): "Alarm: Locked, not homed". GRBL's own "to unlock" advice is not shown.
- **State `Alarm` with neither a number nor that message:** "Alarm: Locked".
- **The first alarm's text stands** until the alarm clears (`AlarmCleared` on GRBL `Idle`): the reset the connector sends after alarm 1 (G13) leaves GRBL locked, and the line keeps "Hard limit hit, position may be lost (alarm 1)" instead of turning into "Locked, not homed" (V11).
- After the alarm clears (the **Unlock** button, or a power cycle without homing), the line goes as in `0064` (Idle: no job block).

**Unlock button (G33, Frage 239 B).**
- A symbol button, `LockOpen` glyph, 28 x 28 (36 coarse) as the other Connector symbol buttons (`0064` section 3): tooltip "Unlock", `aria-label` "Unlock". No text label, no hint line.
- Place: in the alarm line's row, at the right end, on the same baseline; the alarm text wraps in the space left of it (never under it). It is **hidden, not disabled** outside `Alarm` (design system "Hidden, not disabled"), so no state ever shows a greyed Unlock; it is absent while the Connector row is unpaired or the connection is lost.
- Busy: from the press until the alarm clears or 5 s pass, `LoaderCircle` replaces the glyph, the button stays focused and ignores presses; on a lost answer it returns to normal with no text (the alarm line stays, nothing else to say).
- Not a start: pressing it never changes the job buttons, which stay off while the alarm stands; after the alarm clears the row shows Idle and the maker presses Send to machine as always.
- No confirmation, no popup, no notice after it works (design system "No notice after a success"); the alarm line disappearing is the answer.
- Keyboard: it is a Tab stop after the alarm line (Enter and Space), nothing else moves; the alert role stays on the alarm line only. When the alarm clears while Unlock has the focus, the focus moves to the job toolbar's current button (Send job... in Idle), never to the page (Proposal, UX review). A refusal shown in the row ends when the state turns to Alarm or NoController, so it never hides the alarm line or Unlock. Works from every paired app, nothing to do at the connector's computer.
- "Position may be lost" is the controller's state after alarms 1 and 3, not a tip; GRBL's "re-homing recommended" is left out.
- Send to machine in alarm reads "Machine in alarm: Locked, not homed" in the Output group (`0125` section 5, unchanged mechanism); the longest form, "Machine in alarm: Homing pull-off did not clear the switch (alarm 8)", is 68 characters.

## 7. Running, pause, resume, stop on a real machine (G8, G10 to G12)

The words of `0064` section 8 and the buttons of section 7 are unchanged. What the real machine adds:

- **Pause:** "Running" stays until GRBL reports `Hold:0`; the Pause button shows its busy state (`LoaderCircle`) from the press until "Paused" (`Hold:1` is still "Running"). Then Start / Resume reads "Resume".
- **Resume:** "Running" when GRBL reports `Run`; the line count goes on from the same line.
- **Stop:** the Stop button is busy from the press until "Stopped at line <n>" (after the banner, G12) or the failure "Could not confirm the machine stopped". Note for the architect: the connector's worst case is 2 s hold plus 3 s banner, which meets the app's 5 s "Timed out" (`0064` section 3); the answer to `Stop` must come before the sequence ends, or the app's wait for this command must exceed 5 s, so a working stop never reads "Timed out".
- **Completed only when the head is still** (G8): after the last `ok` the row reads "Running" with "412 / 412 lines" and a full bar until GRBL reports `Idle`, then "Completed, 412 lines".
- **Homing first** (G9): "Running", line 0, State row "Home", for as long as it takes; no timer text beyond the elapsed time that is already there.
- Stop never asks and is never armed (`0064`).

## 8. Output group state line (`0125`), consistency

The line reads the same status object as the Connector row (`0125` S14), so every new text in sections 4 to 6 reads the same there, word for word. Changes to the `0125` section 4 table:

| Connector state | State line | Bar |
|---|---|---|
| Done, Failed | "Failed at line 12: No feed rate (error 22)" (section 5) | held, `--destructive` |
| Alarm | "Alarm: Hard limit hit, position may be lost (alarm 1)" (section 6) | held, `--destructive`, if a job ran |
| Lost (G15) | "Lost the controller at line 120", `CircleAlert`, `--destructive`, alert | none |
| Stop not confirmed (G12) | "Could not confirm the machine stopped", same look, alert | none |
| `Idle` after a reconnect with a loss text (V7) | the loss text, as above, until the next send | none |
| `NoController` with an open-failure text, no job | line absent (as `0125`: the Connector row has it) | absent |
| Send refused at `$32=0` (G34) | "Laser mode is off ($32=0)", failure look, polite | none |
| Send refused, `$32` not read (G34) | "Laser mode unknown ($32 not read)", failure look, polite | none |

- " · <name>" for a job not sent from this page (`0125` U6) is cut first; at 70 characters the failure words always fit whole.
- The Controller block and the settings warning are **not** in the Output group (V12): Start is in the Connector row, and the warning sits where Start is pressed.
- The sr-only state-word status of `0125` gains "Lost the controller at line <n>", "Could not confirm the machine stopped" and the full failure and alarm texts, each once per change.

## 9. Accessibility and keyboard

- **Alerts** (`role="alert"`): the alarm line (as built), "Lost the controller ...", "Could not confirm the machine stopped". These are safety states the maker must hear even with focus elsewhere. A status read on opening the block replays none of them as an alert (it is a status then).
- **Polite** (`role="status"`): every `NoController` cause, the failed-line texts, the settings warning container, the state words. Announced on change only; the 2 s retry and the 5-a-second polls never re-announce.
- **Not live:** Position, the counts line, the bar, the elapsed time (`0064` rule).
- **Keyboard:** no new control, no new Tab stop, no shortcut. The Controller block is text (selectable, reachable by a screen reader's reading keys, not by Tab). The job toolbar's roving focus, Enter, Space and the "Escape never sends Stop or Pause" rule are unchanged; a button that a state change greys keeps the focus (`0064` section 10).
- **Not colour alone:** failures carry `CircleAlert`, alarms `TriangleAlert` in `--destructive`, the settings warning `TriangleAlert` in `--toolbar-icon`; every state is also in words.
- **Contrast:** existing tokens only: `--toolbar-icon` 8.2:1, `--panel-muted-fg` 5.0:1, `--destructive` 4.8:1 on `--panel-bg`.

## 10. Connector terminal and `--help` (demo, not UI)

Plain lines, no colour codes, one line per event, the app's words where the app has words. A cause that repeats on every 2 s retry prints once, and again only when it changes.

```text
Curvyo connector 0.2.0, protocol 0
Listening on 192.168.1.20:7447
Key fingerprint: 3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
Serial port /dev/ttyUSB0, 115200 baud
GRBL 1.1h on /dev/ttyUSB0: laser mode on, max power 1000, travel 400 x 300 mm
```

| Event | Terminal line |
|---|---|
| Open fails, not found | "Cannot open /dev/ttyUSB0: not found" |
| Open fails, permission (G27) | "Cannot open /dev/ttyUSB0: permission denied (group uucp, this user is not a member)" |
| Open fails, in use | "Cannot open /dev/ttyUSB0: in use by another program" |
| No banner in 5 s | "No GRBL answer on /dev/ttyUSB0" |
| Text that is not a GRBL banner | "No GRBL answer on /dev/ttyUSB0 (got: <first 40 printable characters>)" |
| GRBL 0.9 | "GRBL 0.9 is not supported (on /dev/ttyUSB0)" |
| Retrying (once, after the first failure) | "Trying /dev/ttyUSB0 again every 2 s" |
| Identify (also after a reconnect) | the "GRBL 1.1h on ..." line above |
| `$32=0` (once per identify; jobs are refused, G34) | "Laser mode is off ($32=0)" |
| `$32` not read (once per identify; jobs are refused, G34) | "Laser mode unknown ($32 not read)" |
| Other GRBL state | "Controller state: Door" |
| Alarm | "Alarm: Hard limit hit, position may be lost (alarm 1)" / "Alarm: Locked, not homed" |
| `Unlock` from a client in alarm (G33) | "Unlock sent by <client name>", then "Alarm cleared" when GRBL reports Idle |
| Send refused at `$32=0` | "Job refused: Laser mode is off ($32=0)" |
| Send refused, `$32` not read | "Job refused: Laser mode unknown ($32 not read)" |
| Job results (`0064` "Job <name>: <result>") | "Job cut_line.gcode: Completed" / "...: Stopped at line 120" / "...: Failed at line 12: No feed rate (error 22)" / "...: Alarm: Hard limit hit, position may be lost (alarm 1)" / "...: Lost the controller at line 120" |
| Stop without banner | "Could not confirm the machine stopped (no GRBL banner within 3 s)" |
| SIGINT or SIGTERM, job running (G16) | "Stopping cut_line.gcode", "Job cut_line.gcode: Stopped at line 120", "Exiting" |
| SIGINT or SIGTERM, no job | "Exiting" |
| Usage errors (G1, exit 2) | "--serial and --fake-controller cannot be used together" / "--baud needs --serial" / "--baud must be 1 to 4000000" |
| `grbl_pty` example (G22) | "GRBL simulator on /dev/pts/7" |

- `jobs.log`: `Unlock: <client name>` (G33), `Failed: line 12, No feed rate (error 22)`, `Failed: line 12, Longer than 79 characters`, `Alarm: Hard limit hit, position may be lost (alarm 1)`, `Failed: controller lost at line 120` (spec G15). The architect fixes the exact log grammar with the golden file; the words after the line number are those of the app.
- **No terminal input for unlock** (design principle, Frage 239 B, 241 B): in alarm the terminal prints the alarm line and nothing else; there is no `unlock` line, no prompt and no "Unknown input" line. The only terminal input stays the Enter of `--require-local-start` (Frage 242).
- **`--help`** (documentation, so it may say what to do; G1, G27): `--serial <port>` "Serial port of the GRBL controller, for example /dev/ttyUSB0"; `--baud <n>` "Baud rate, 1 to 4000000 (default 115200)"; a section "Terminal input": "Enter: start the job waiting for the local start (only with --require-local-start)."; a section "Serial ports on Linux": "The user must be in the group that owns the port (dialout on Debian and Ubuntu, uucp on Arch) or a udev rule must grant access. ModemManager may probe ttyACM and ttyUSB devices and disturb the controller." The architect adds what the connector does about ModemManager, if anything.

## 11. Customer check list for the first real-laser run (G24)

For the PR and the demo; it is documentation, not UI. Tick what you did; the PR states which steps were done on the laser and which only on the simulator.

1. **Port access:** `ls /dev/ttyUSB* /dev/ttyACM*` shows the laser's port; `groups` lists `uucp` (Arch) or `dialout` (Debian, Ubuntu). If not, add the group and log in again.
2. **Safe setup:** empty bed or a sheet of paper or card, lid shut, eye protection on, air assist as usual, the machine's own power switch or emergency stop within reach, someone at the machine all the time.
3. **Connect:** `curvyo-connector-server --serial /dev/ttyUSB0` (add `--baud` if your controller does not use 115200). The terminal prints "GRBL 1.1... on /dev/ttyUSB0: ...".
4. **Controller block:** in the app's Connector row, Controller, Port, Laser mode, Max power and Travel show your machine's values. Laser mode reads "On"; no warning line, or a power or laser-mode warning you understand (Max power differs: set the profile's Max power to your `$30`).
5. **Alarm at power-up** (only if homing is on): the row reads "Alarm: Locked, not homed" and Start is off; press the Unlock button (tooltip "Unlock") in the Connector row; the alarm line goes and the button disappears. Nothing is typed on the connector.
5b. **Laser mode:** with `$32=0` Send to machine reads "Laser mode is off ($32=0)" and nothing is sent.
6. **Small page:** a 20 mm square, one cut operation at low power (5 to 10 %) and a moderate speed.
7. **Send to machine** in the Output group; the line reads "Ready: ...".
8. **Start** in the Connector row; the head moves, "Running" with the line count.
9. **Pause:** the head stops and the beam is off (look through the lid), the row reads "Paused".
10. **Resume:** the job goes on from where it stopped.
11. **Stop:** beam off at once, head stops, "Stopped at line <n>".
12. **Run to the end:** Send to machine again, Start, wait: "Completed, <n> lines" appears only after the head has stopped.
13. **Optional, cable pull** (paper, lowest power): pull the USB cable while running; the row reads "Lost the controller at line <n>" and never "Running"; plug it back in; the Controller block returns and the job does not start again.
14. Tell the lead which steps you did and anything that looked or sounded wrong.

## 12. Decisions for veto (Proposal)

- **V1** Controller block as the last part of the Connector group, after the job block, so the job toolbar never moves.
- **V2** GRBL's other states as a "State" row in the Controller block ("State: Door"), not in the job line.
- **V3** Row label "Max power" (the profile's word) for `$30`, not "S max".
- **V4** A Position row (X and Y, work position, not live); Z left out for a laser.
- **V5** No warning for travel, port or baud; the profile's Controller port and Baud rate fields stay as built, never sent, never compared.
- **V6** The app names the open failure: "Port ... not found", "No permission for ...", "Port ... is in use", "Cannot open ...".
- **V7** A loss or unconfirmed-stop text stays after the reconnect until the next send; no bar with it.
- **V8** No "Retrying" text while the connector retries.
- **V9** A failed line is a polite status; only alarm, loss and unconfirmed stop are alerts.
- **V10** G5 failures read "Failed at line <n>: <cause>" like the GRBL errors.
- **V11** Alarm text "<meaning> (alarm <n>)" in the "Alarm: <text>" frame; the first alarm's text stands until cleared.
- **V12** The Controller block and the settings warning stay in the Connector row, not in the Output group.
- **V13** Unlock is a `LockOpen` symbol button at the right of the alarm line, hidden outside `Alarm`, busy until the alarm clears or 5 s, no confirmation.
- **V14** The `$32=0` refusal text "Laser mode is off ($32=0)" appears in the Output group's state line (as a refusal of Send) and in the Controller block's Laser mode row as plain "Off"; no override.

## 13. Mismatches for the product owner

- **G13:** "Alarm <n>: <meaning>" becomes "<meaning> (alarm <n>)" (V11). The alarm list in G13 does not match GRBL 1.1 for 6 to 9: GRBL's are reset during homing, door opened during homing, pull-off did not clear the switch, did not find the switch (table of section 6).
- **G5:** "Line 12 is longer than 79 characters" becomes "Failed at line 12: Longer than 79 characters" (V10). G5 does not say whether a line with `!`, `~` or `?` left after comment removal fails the job; these notes assume it does ("Contains !, ~ or ?").
- **G4:** "Controller state: Door" is the terminal's line; the app shows the State row "Door" (V2).
- **G7 and G15:** the log grammar `Failed: <text>` needs a text without its own "Failed" (section 10).
- **G15:** add the open-failure texts (V6) and the persistence after reconnect (V7).
- **Not in the spec:** a GRBL hold the maker did not ask for (a hold button on the machine, the safety door during a job). Proposal: shown as GRBL reports it ("Paused" at `Hold:0`; the State row "Door" for the door) with Resume enabled, so the row never claims "Running" while the head stands.

## 14. Design-system rows to add after acceptance

- **"Controller block (`0127`)"** after "Connector group (`0064`)": section 2 (place, grid, rows, settings warning in the Notice look, not live except the warning container).
- **"Connector group (`0064`)"** gains: the controller texts of sections 4 to 6 (open failures, loss, unconfirmed stop as alerts, "Failed at line <n>: <cause>", "Alarm: <meaning> (alarm <n>)").
- **"Connector state line (`0125`)"** gains the same words and the loss rows of section 8.
- No new token, no new glyph, no new control.

## Questions for the customer

Fragen 238 to 241 are answered (2026-11-18); see `specification.md`. Frage 242 (CONN-LOCALSTART-SERVICE, local start for a service) is open with default C and has no UI in this slice.

## Links

Spec: `specs/0127-connector-grbl-serial/specification.md`. Built notes: `specs/0064-machine-connector/ux-notes.md`, `specs/0125-send-to-machine/ux-notes.md`, `specs/0026-machine-profile/ux-notes.md`. Code: `frontend/src/components/workbench/ConnectorGroup.tsx`, `frontend/src/lib/connectorText.ts`, `frontend/src/components/panel/OutputGroup.tsx`.
