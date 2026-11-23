# ADRs for machine control (first slice: Home and the Machine controls group)

Architect, 2026-11-19. Covers criteria 1 to 7 of `specification.md` (first slice). **No new ADR, no
new crate, no new dependency, no `format_version` change** (the project file is untouched). The
wire change is additive inside protocol 0 (ADR 0035 §6, not released). Everything below is a
**(Proposal)** unless it quotes an ADR or a criterion.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) §6, §7: `Home` and the state
  `Homing` are two more protocol-0 names (additive, golden files); any paired connection may read
  the state and send `Stop` at any time, also while homing.
- [ADR 0070](../../docs/adr/0070-connector-through-the-relay.md) decision 2: `0118`'s Home is a
  **start** (it moves the head): it waits for the local press when the connector runs with
  `--require-local-start`, and from a relay member it always waits (relay slices, not here).
- ADR 0071 (Accepted, not yet on main; branch
  `story/connector-api`, spec `0128`) decisions 2 and 7: in maintenance mode Home answers
  `Error { Maintenance }` and no byte reaches the port; in configured mode a member's motion waits
  for an owner (M10).
- [`specs/0064-machine-connector/adrs.md`](../0064-machine-connector/adrs.md) addendum C: the
  `Controller` trait with its two implementations (C2), the pure GRBL part in `-io/src/grbl/`
  (C3), streaming rules and the stop (C4, G12), core's confirmed controls (C5), additive wire
  fields (C6). Addendum D (fifth slice, on `story/connector-api`): the access table D4 and the
  start rule D5 take Home as described in M10.
- `specs/0127-connector-grbl-serial` G9 (stop during homing: soft reset without feed hold), G13
  (alarm texts, laser off), G14 (the connector never sends `$X` or `$H` on its own), G15 (loss
  rule), G33 (the pattern `Unlock` follows: one command, one write, one log line); `specs/0129`
  as built in commit `217ab6ec` (while `$H` is in flight the loss clock pauses; the simulator's
  quiet homing, alarm 8/9 and `error:5`).

## Feature-local decisions

**M1. Wire (`curvyo-connector-core/src/message.rs`, `codec.rs`).** `Message::Home` (unit variant,
`{"type":"Home"}`, app to connector; `KNOWN_TYPES` gains `"Home"`, 18 names) and `State::Homing`
(`{"name":"Homing"}`). No new `ErrorCode`, no new `RejectReason`: Home has no job id, so the
job's `Busy` does not fit; a refused Home answers `Error { WrongState }` with a sentence naming
the cause ("A job is running.", "No controller.", "Controller state: Door."), which the app shows
as its existing "Connector busy" (UX notes section 5). New goldens in
`curvyo-connector-core/tests/fixtures/messages/`: `command_home.json`, `status_homing.json`
(no job) and `status_homing_ready.json` (a kept Ready job, M3); every old golden decodes and
re-encodes byte for byte. Protocol stays 0. An older app that meets `Homing` fails to decode that
`Status` (`Malformed`), acceptable while protocol 0 is unreleased and app and connector ship
together; the first release freezes the set (ADR 0071 decision 4).
*Rejected:* Homing as `Status.detail` on `Idle`/`Ready` (the 0129 "Controller state: Home" word):
the job machine would then accept `SubmitJob` and `Start` during homing, and the app would parse a
sentence to grey its buttons; a real state is what every client and the machine agree on (UX notes
section 13 asks for exactly this).

**M2. Core events and commands (internal, not on the wire).** `ControllerCommand::Home`;
`Event::Homed` (the controller's `ok` to `$H`); `Event::HomingFailed { detail: String }` (an
`error:<n>` answer to `$H`, GRBL stays where it was). An alarm during homing is the existing
`Event::Alarm`. The home logic is its own module `machine/home.rs` ("how the job machine takes
Home and the controller's homing events"), so `machine/mod.rs` stays under 500 lines.

**M3. Core rules (`Machine`, both `new` and `confirming`).**

| State when `Home` arrives | Result |
|---|---|
| `Idle`, `Ready`, `Done { .. }`, `Alarm` | `ControllerCommand::Home`, state `Homing` at once (the answer `Status` comes before GRBL's `ok`, UX W12) |
| the same, with `require_local_start` | `AwaitingLocalStart` (M5) |
| `Idle`/`Ready`/`Done` while the controller word is `Door`, `Check` or `Sleep` | `WrongState` "Controller state: <word>." (G4's busy words; homing into an open door is not started) |
| `Running`, `Paused` (also with a stop pending) | `WrongState` "A job is running." |
| `Receiving`, `AwaitingLocalStart`, `NoController` | `WrongState` with its cause |
| `Homing` | nothing written, `Status` unchanged (as `Unlock` outside `Alarm`, G33) |

- The machine remembers the state it homed from (`Idle`, `Ready` or `Alarm`; `Done` counts as
  `Idle`). `Homed` gives `Ready` when a Ready job was kept (same `job_id`, criterion 3), else
  `Idle`; `alarm` and the alarm text are cleared (GRBL leaves the alarm by homing).
  `HomingFailed { detail }` returns to that state with `detail` ("Homing is not enabled
  (error 5)", built by `texts::error_text`, UX notes section 5); from `Alarm` the alarm text
  stays. `Event::Alarm` in `Homing` gives `Alarm` with the new text and **drops a kept Ready job,
  as every alarm does in core today** (position is not trusted after a failed homing).
- `Status` in `Homing` carries the kept Ready job's `Progress` (line 0), as `Ready` does
  (`0064` slice-review note), so every app keeps its name; `alarm` is `null`; `detail` is
  `"Stopping"` while a stop is pending, else `null` (the controller word `Home` that 0129 pushes is
  not shown in this state: the state already says it).
- In `Homing`: `SubmitJob` is `Busy` (as built for every non-idle state), `Start`, `Pause`,
  `Resume` are `WrongState`, `Unlock` writes nothing, `ClientDisconnected` changes nothing.
  `ControllerLost` gives `NoController` with "Lost the controller" (UX notes section 5).
- **Stop in `Homing`** sends `ControllerCommand::Stop`. `Machine::new` (fake): back to the state
  it homed from at once. `Machine::confirming` (GRBL): stop pending (detail "Stopping") until
  `Stopped`, then back to that state; GRBL's alarm 6 follows and gives `Alarm`, which drops a kept
  Ready job (above). **Conflict with UX W8** ("the held Ready job is kept"): it holds for the fake
  controller only; on GRBL the alarm drops it. Never `Done`: there was no job, so no job result
  and no `jobs.log` result line.
- **Home during Stop** (a stop of a running job still pending): the state is still `Running` or
  `Paused`, so `WrongState`; once the stop ends (`Done { Stopped }`) Home is allowed.

**M4. GRBL (`curvyo-connector-io/src/grbl/`).** New module `grbl/home.rs` ("the `$H` the Home
command writes, from the write to its answer"); `controller.rs` (465 lines) only dispatches.
- On `ControllerCommand::Home` (never otherwise, G14): write `$H\n` once, in one write, alone, and
  record it in the owed queue as a typed entry (`owed: VecDeque<usize>` becomes
  `VecDeque<Owed>`, `Owed::Line(bytes) | Owed::Home`), so its answer is told apart from the
  answers of `M5`, `M9`, `$X` that may still be owed. No `$X` first: GRBL 1.1 accepts `$H` in
  `Idle` and `Alarm` (its `$H` check is "Idle or Alarm"), and a successful homing leaves the alarm
  lock; this answers `0129` criterion 3 for Home (UX W13).
- The answer: `ok` gives `Event::Homed` and clears `alarmed`, `alarm_code`, `locked`; `error:<n>`
  gives `Event::HomingFailed { detail: error_text(n) }` and leaves the alarm flags as they were.
  `ALARM:<n>` while `$H` is in flight takes the new code even when `alarmed` was already set (the
  homing failure supersedes the power-up lock; the "first alarm stands" rule of G13 applies from
  there), and the `ok` GRBL 1.1f sends after that alarm is consumed without `Homed`.
- **The 0129 loss-clock pause applies**: `during_homing` holds while `stream.homing()` **or** a
  `Owed::Home` is in flight; polling runs at the fast period meanwhile; port errors still end the
  link. No time limit on homing (G9).
- **Stop during Home**: `begin_stop` also starts when no stream exists but `$H` is in flight, with
  `homing = true` (reset without feed hold, G9), banner limit as G12, then `M5`, `M9`;
  `finish_stop` emits `Stopped` and the deferred alarm 6. The banner clears the owed queue
  (as built). `buffer_disagrees` needs no change (it only runs with a stream).
- A job's own first-line `$H` (`0029` criterion 38) is unchanged: it stays `Running` with the word
  `Home`.

**M5. `--require-local-start` (foreground mode).** Home is a start (ADR 0070 decision 2): it goes
to `AwaitingLocalStart` with **no `job`** in `Status` (a waiting job start always carries one; doc
comment on `State::AwaitingLocalStart` says so). The machine remembers internally that a Home
waits (an enum `Waiting::Start | Home`, which `0128` D5 extends with `Resume`). `LocalStartPressed`
then gives `ControllerCommand::Home` and `Homing`; `Stop` cancels a waiting Home and returns to
`Ready` (job kept) or `Idle`. The terminal prints the same line it prints for a waiting start.
Not in the criteria before; criterion 3 now says so (conflict 2).

**M6. Fake controller (`fake_controller.rs`).** `Home` starts a fake homing of
`FAKE_HOMING = 2 s` (named constant, injected time, no sleep); `run_until` emits `Homed` when it
ends; `Stop` cancels it (the non-confirming machine needs no `Stopped`); a Home in the fake's
alarm (`--fake-alarm-at-line`) ends in `Homed` and so clears it, as GRBL's does. **No failure
mode and no new flag** (`CLAUDE.md` §5): Home failures are covered by the simulator and by core
tests with injected events (conflict 3).

**M7. Simulator (`tests/common/grbl_sim/`).** Already models `$H` (homing steps, `$22`, quiet
homing, `homing_fails` 8/9 with `ok` and the locked restart, `error:5`, alarm 6 on a reset during
homing). Added only: `$H` while `Hold`, `Run` or `Jog` answers `error:8` (GRBL's "not idle"),
so the connector's refusal (M3) is the only guard the tests prove, not the simulator's leniency.

**M8. Who may Home, and the log.** Today every paired app may (as `Unlock` and `Stop`, ADR 0035
§7); Home is never sent by the app or the connector on its own (criterion 3, G14). The hub takes
Home like `Unlock` (the session passes the client name): one `jobs.log` line `Home: <client name>`
(`LogResult::Home(String)`, the golden `jobs.log` gains it, old lines stay) and one terminal line
"Home sent by <client name>". Results of the homing are not logged (no job); an alarm is, as built.

**M9. App and webview boundary.** No new Tauri command: `connector_command` accepts `"home"` and
sends `Message::Home` over a fresh paired connection; the answer is the existing `CommandAnswer`
(`WrongState` for a refusal). The webview never sees the token (ADR 0035 §1). No `curvyo-editor-wasm`
change: the Machine controls group takes `profileId` and `address` from the Output view's `send`
(present only for a saved profile with a connector address; cause `browser` in the browser build),
which already exists. `curvyo-connector-core` stays pure and builds for wasm32 (two more enum
variants). `jobIsActive` in `connectorText.ts` counts `Homing` as active (a lost connection while
homing shows "Connection lost").

**M10. Rule for `0128` (configured mode, owners and members).** Whichever of `0118` and `0128`
merges second carries this; until then Home follows M8. ADR 0071 decision 7 (Frage 244 A,
245 A) lists "`0118`'s motion" among the starts a member's request waits for, so: in D4's table
`Home` sits in the row "job and control messages" (`Maintenance` in maintenance mode); under
D5's `OwnerConfirms` a **member's** Home goes to `AwaitingLocalStart` with `requested_by` and no
job (M5's `Waiting::Home`), an **owner's** `Home` runs at once and also confirms a waiting Home
(no new message, as `Start { job_id }` confirms a start), and `Stop` from anyone discards a
waiting Home. Making Home owner-only instead (a member's Home refused with `OwnerRequired`) would
change that accepted rule and needs a customer question; not proposed.

**M11. Frontend: one store, one component per place.**
- `src/lib/connectorLink.ts` stays the one source per profile (`0125` S14). It gains **`busy`**
  (the one press that waits for its answer, today local state of `useConnector`), so "another
  button of the group waits for its answer" (UX section 4 row 8) holds across the Machine block
  and the Operations tab: `claim(profileId, kind)` / `release(profileId)`, unit-tested without
  React. Home's `busy` ends with the connector's answer (the `Homing` status); its spinner during
  homing comes from the status (UX section 4), never from `busy`, so **Stop stays pressable
  while homing** (with `busy` held through homing, `guard` would swallow Stop).
- `useConnector.ts` is split by place: `useConnectorLink` (pairing read, Test connection, Pair,
  Forget pairing, the link line and its alert) for the Machine block, and `useMachineControls`
  (Send job..., the commands with `"home"`, the refusal line, the failure line of its own presses
  and its alert, the settle rules) for the Operations tab. Both read the entry with
  `useLinkEntry`; the Machine controls reader bootstraps (`bootstrapping = true`, as the Output
  group), because the Machine block need not be mounted. Link-level failures still go through
  `linkStore.linkFailure`, so a key change or lost pairing shows in both places.
- `src/components/panel/MachineControls.tsx` (new): the toolbar (Home first, UX section 3) and the
  job block (state line, bar, counts line, Unlock), **moved** from `ConnectorGroup.tsx`, not
  copied; rendered once, in the Operations tab only, where the UX notes put it. `SymbolButton`
  and the glyph constants move to `src/components/workbench/SymbolButton.tsx`, shared by both.
  `ConnectorGroup.tsx` keeps Test connection, the link line, the key-changed block, the trust
  block, the Pair row and the Controller block (criterion 2; its warning lines go where UX W9
  puts them).
- `connectorText.ts`: `JobState` gains `"Homing"`; `jobButtons` gains `home` and its cause (the
  UX section 4 table as one pure function); `jobStateLine` gains the Homing rows (UX section 5).
- If the Output group's state line stays (Frage 262), it reads the same entry: equal values by
  construction.

**M12. Tests** (no wall-clock sleeps; stepped time as the 0127 bench, `CLAUDE.md` §5).
- Core (`tests/state_machine.rs`, `confirming_machine.rs`, `golden_messages.rs`): the M3 table
  state by state; `Homed` back to `Ready` with the same id and to `Idle`; `HomingFailed` from
  `Idle`, `Ready`, `Alarm` (text kept); alarm in `Homing` drops the Ready job; Stop in `Homing`
  in both machines; `ControllerLost` in `Homing`; `SubmitJob`/`Start` refused while homing; M5 with
  `require_local_start` (press, Stop keeps the Ready job); the three goldens and `KNOWN_TYPES`.
- `-io` with the simulator (new `tests/grbl_home.rs`): Home from Idle (byte log has `$H` exactly
  once); 9 s quiet homing without a loss; Home from the power-up lock without `$X`; `$22=0` gives
  "Homing is not enabled (error 5)" and the state it came from; `ALARM:8`/`9` texts, laser and
  air off, no `Homed` from the trailing `ok`; Stop during Home: reset with no `!`, alarm 6, `M5`
  and `M9` after the banner; Home in `Running` writes nothing; connect, alarm and reconnect never
  write `$H` (G14). Fake controller unit tests: Home, Stop during Home, Home in its alarm.
  Hub/server: the `jobs.log` golden line; a second paired client sees `Homing` and may Stop.
- App: `connector_command` maps `"home"` (beside `start_takes_the_job_id_from_a_ready_status_only`).
- Frontend: `jobButtons` and `jobStateLine` tables, the shared `busy` in the store tests, and a
  component test that the Machine block renders no job button, state line or bar while the
  Operations tab renders the group once (absent without an address, "Needs the desktop app" in
  the browser build).

## Spec conflicts fixed in `specification.md` (2026-11-19)

1. Criterion 3 named "Idle or Alarm" while "What changes" says "Idle, Alarm, or Ready" and the
   rule is "no running or paused job": now Idle, Ready, Done or Alarm (UX W13 agrees); "answers
   `Status` Homing" now names the state `Homing`, reported at once to every client; "does not
   discard it" now says "a Home that succeeds".
2. Criterion 3 said the connector writes `$H` at once; under `--require-local-start` Home waits
   for the press (ADR 0070 decision 2, M5). Added.
3. Criterion 6 asked the fake controller to cover a Home failure; that needs a new fake flag
   (`CLAUDE.md` §5). Failures are now covered by the simulator and core tests (M6); "Home during
   Stop" is spelled out as both cases, Stop during Home and Home while a Stop is pending (M3).
4. Criterion 4 did not say what happens to a Ready job: an alarm drops it as every alarm does,
   `error:5` keeps it (M3). Added. UX W8 (Stop keeps the job) holds for the fake only.

No other conflict. Not changed: criteria 1, 2, 5, 7 (UX notes section 13 lists its own word-level
points for the product owner).

## Second slice 2a (jog)

Architect, 2026-11-23, criteria 8 to 21 and 37 (Fragen 308 A, 309 A, 315 A). **No new ADR, crate,
dependency or `format_version`**; wire additive in protocol 0 (ADR 0035 §6). All is **(Proposal)**.

**J1. Wire.** App to connector: `Jog { axis: "X"|"Y", direction: "Plus"|"Minus", step, hold }`
(`hold` left out when false), `JogKeep`, `JogStop` (`KNOWN_TYPES` 39 to 42). `step` is the closed set
of Frage 315 A (`"0.1"`, `"1"`, `"10"`, `"50"`), never a number. No feed on the wire: feeds are core
constants (criterion 16; a typed feed is Frage 315 B, not chosen). `JogStop` is not `Stop`: a release
arriving after the jog ended must not discard a Ready job; outside `Jogging` it does nothing. New
`State::Jogging`; `Status` gains `homed`, `machine_position`, `work_offset` (13, 14). Axes are
machine axes (the arrow map per origin corner is the app's, J8). Goldens: one per new message,
`status_jogging.json`, `status_homed_positions.json`, `status_awaiting_home.json`; old ones unchanged.

**J2. Core modules.** `src/jog.rs` ("the jog steps, their feeds and the travel clamp"): `JogStep`,
`Feed` (mm/min newtype), `feed(step)` (3,000 / 3,000 / 1,000 / 300), `TravelBox`, `clamp(from, axis,
delta, box) -> Option<Millimetres>` with tolerance `EDGE = 0.01 mm` (`None`: at the edge, write
nothing). `machine/jog.rs` ("how the job machine takes jog messages and jog events"); `Event`
moves to `machine/event.rs` (refactor commit) if `machine/mod.rs` passes 500 lines.

**J3. Rules (both machines).** `Jog` in `Idle`, `Ready`, `Done` gives `ControllerCommand::Jog { axis,
sign, distance, feed, homed }` (or `JogHold`) and `Jogging`; the state it came from is kept as for
Home (`homed_from` becomes `motion_from`). Refused, `WrongState` with criterion 8's words, in
`Receiving`, `Running`, `Paused`, `AwaitingLocalStart`, `Homing`, `NoController`, `Alarm` and with
`BUSY_WORDS`. In `Jogging`: one more step is taken once, later ones dropped with the answer `Status`
(criterion 19); `Home`, `Start` refused ("Jogging"), `SubmitJob` `Busy`, `Pause` "No job to pause";
`Stop` and `JogStop` give `JogCancel` (fake: back at once; GRBL: "Stopping" until `JogEnded`). New
internal events `JogEnded`, `JogFailed { detail }` (`error:15`, built text), `ControllerReset`.
`Alarm` drops a Ready job (M3); `--require-local-start` does not hold a jog (Frage 308 A).

**J4. Dead-man; the clock stays outside core.** Core takes `Event::Clock(Duration)` from the hub
on every `TICK` (50 ms, `hub/link.rs`) and before each jog message; a hold or `JogKeep` sets
`deadline = now + JOG_DEAD_MAN` (300 ms), a `Clock` past it gives `JogCancel`. A session that started
a hold hands the hub a `JogStop` when it closes.

**J5. Homed since connect.** Core's flag: set by `Event::Homed` in any state (also the `ok` to a
job's own `$H`); cleared by `Alarm`, `ControllerLost*`, `ControllerConnected`, `StopUnconfirmed` and
`ControllerReset`, which `on_banner` emits for an unasked banner outside a stream (emits nothing
today); a stop's own reset and the jog cancel keep it. `Event::Position` also carries `MPos`, `WCO`.

**J6. GRBL (`-io/src/grbl/jog.rs`, "the `$J=` lines and the jog cancel, from the press to Idle").**
- Laser off (criterion 9): `spindle_off` is true after the connector's own `M5`, false once a job
  line is written; the first motion line follows `M5\nM9\n` unless true. `$J=` carries no M or S word.
- Step: `$J=G91 G21 X<d> F<f>`, `f` capped at `$110`/`$111` (`Settings::max_rate`), owed as
  `Owed::Jog`. Clamp when homed and `$130`/`$131` known: box `[-travel, 0]`, or `[0, travel]` for an
  axis with its `$23` bit set whose position after homing is 0 within `EDGE` (force-origin build).
- Hold: increments of `feed x 0.1 s`, at most about 0.25 s of motion queued (`HOLD_AHEAD`), each
  clamped, so a dead connector stops the head within that (*rejected:* one `$J=` to the edge, UGS).
- Cancel: `0x85`, repeated at each report while a `$J=` is owed or the word is `Jog` (GRBL ignores
  `0x85` outside Jog, so a `$J=` still in its buffer would run); `JogEnded` at `Idle` with nothing
  owed; not `Idle` within 2 s: the stop of G12. Fast polling while jogging.

**J7. A member's Home (criterion 37), `access.rs`, `confirm.rs`.** `Kind::Home` joins `Kind::Job`
(member "yes"); new `Kind::Motion` (`Jog`, `JogKeep`): member `OwnerRequired`, else as `Job`;
`JogStop` is `Kind::Job`. A member's Home in `Idle`/`Ready`/`Done`/`Alarm` goes to `request(
Waiting::Home)`; an owner's `Home` there confirms; a second `Home` waiting is `WrongState`; `Stop`
from anyone discards it. `shown_requester` drops its `Waiting::Home` exclusion; no `job` (M5).
`Confirmation` gets `what: Start | Resume | Home` (for `resume`) and `job_id: Option<Uuid>`;
`admin.log` "start confirmed" names Home; `curvyoctl confirm-start` sends `Home` when `requested_by`
is set and `job` absent. `jobs.log`: `Jog: <client>` once per run of one client's jogs, ended by any
other `jobs.log` line or client (criterion 12's pad session without a clock; golden).

**J8. App and frontend.** `connector_command` takes `"jog"` with `{ axis, direction, step }` (fresh
connection, as Home). Hold: new `curvyo-app/src/connector_jog.rs` (`connector.rs` is 825 lines):
`connector_jog_hold` keeps a paired connection per profile, `connector_jog_keep` forwards one
`JogKeep` per webview call, `connector_jog_release` sends `JogStop` and closes; Rust sends no
keep-alive of its own, so a frozen webview stops the head. Frontend: `src/lib/jog.ts` (pure: arrow
to axis per origin corner; press reducer: release before 300 ms is a step, else hold, keep every
100 ms), `JogPad.tsx` in `MachineControls.tsx`, jog calls in `useMachineControls`, `connectorText.ts`
gains `Jogging`, `jobButtons.jog`, "Not homed", the work-offset Notice (beyond 0.001 mm); no pad
for a member (`Authenticated`); open/closed and step are per-app view state.

**J9. Tests, no sleeps.** Core: J3 per state in both machines, J4 by `Clock` (299 ms keeps, 300 ms
cancels), `clamp` per travel/`$23`/edge, J5, J7 rows, goldens. Simulator gains `$J=`, `Jog`, `0x85`,
`error:15`; `tests/grbl_jog.rs`: bytes per sign, `M5` first, `0x85` on Stop, release, keep loss and
the late `$J=`, no `M3`/`M4`/`S>0`. Fake: a position moved by jogs in injected time; 18 by CPU time.

**Spec points, no customer question.** Criterion 16's "byte log per corner" is `jog.ts`'s corner
table plus simulator bytes per sign; 21's "clamp per corner" is per `$23` and travel; 18's 300 ms is
checked per hub tick (cancel by 350 ms).
