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

## Second slice 2b (frame)

Architect, 2026-11-23, criteria 22 to 28 (Fragen 308 A, 310 C, 311 A), ux-notes W28 to W38. **No new
ADR, crate, dependency or `format_version`**; wire additive in protocol 0. All is **(Proposal)**.

**F1. Frame text, `curvyo-laser-core` (pure, wasm32).** `job_frame.rs` ("the outline a job's frame
traces"): `FrameShape { Rectangle, RubberBand }`, `outline(&Job, shape) -> Option<Vec<PointUm>>` over
both ends of every `Burn` and `Sweep` move (fixture blocks included; `Travel`, `RunOut`, `Z` not).
Rectangle from the box corner nearest the machine origin, X first; Rubber band the convex hull
(monotone chain, exact `i128` cross products on µm), counter-clockwise in machine coordinates, from the
point nearest the machine origin. No burn move: `None` ("Nothing to frame"). *Amended (Frage 335 B):* the outline is taken from `job_build::build_frame_job`, which plans a block with items and no power or speed (4 a) at stand-in numbers (lowest power, the machine's maximum feed, one pass; never written, the frame is `S0` at its own feed); the wasm frame cause ignores the 4 a blocks and measures 4 b on that job (`outside_the_bed`). `job_frame_grbl.rs` ("the
GRBL bytes of a frame") behind `job_write::write_frame`: comments, `G21`, `G90` (`G91` for Head
position), `G17`, `M5`, `M9`, then `G1 X Y F<f> S0` to each point and back to the first; `f =
min(3,000 mm/min, profile max_feed)`, GRBL's planner caps each axis at `$110`/`$111` (*rejected:* the
connector rewriting `F` after the hash check). No `G0` (ignores `F`), no `$H` (Home then machine origin
frames as Machine origin); Head position runs relative from `Job::home` and back to it. Goldens
`frame_rectangle`, `frame_rubber_band`, `frame_head_position`, `frame_corners_{bl,br,tl,tr}.gcode`.

**F2. Send path and wire.** Wasm `frame_bytes(page, shape)` from the cached job, with the causes "Nothing to
frame" and "Outside the work area" (4 b). App: new `connector_frame.rs` (`connector_send.rs` is 493
lines): raw body, fresh paired connection, `SubmitJob { kind: Frame }` and one chunk, answer as
`CommandAnswer`; no watch, start or list; `job_send.rs` takes the kind as a parameter. **Wire:**
`JobKind::Frame` exists (`0064` P2). New `State::Framing`, `RejectReason::NotAFrame`,
`Status.frame_id: Option<Uuid>` (omitted when `None`; set in `Framing` and while a frame waits, since
`journal.rs` reads "no job" as a waiting Home). One chunk at most (`TooLarge`). Goldens
`status_framing.json`, `status_awaiting_frame.json`, `submit_frame.json`, `reject_not_a_frame.json`.

**F3. Core check, `src/frame_text.rs`** ("the lines a frame may hold"): comments, `G21`, `G90`,
`G91`, `G17`, `M5`, `M9`, `G1` with `X`, `Y`, `F` > 0, `S0`, `M5` before the first `G1`; any other
word (`M3`, `M4`, `M8`, `S` > 0, `Z`, `$`, `G0`, `G10`, `G92`, `G54`) is `NotAFrame`.
`FrameMoves::fits(box, mpos, wco)` tests the targets in machine coordinates against `jog::TravelBox`.
Since the check proves laser off, `$32=0` or unknown does not refuse a frame (UX W31); jobs keep G34.

**F4. Rules, `machine/frame.rs`** ("how the job machine runs a frame job"),
both machines. `SubmitJob { Frame }`, before the list, in `Idle`, `Ready`, `Done` without a busy word
assembles in its own slot `frame: Option<FrameRun>` beside `held`, state unchanged (the Ready job stays
held); `Busy` in every other state or with a frame arriving (`0132` L2 amended). Last chunk: hash,
ASCII, F3 (else `JobRejected`), the state checked again, `JobAccepted`, `motion_from` = that state
(`Done` kept), then the start rule as Home: `LocalPress` or a member under `OwnerConfirms` wait
(`Waiting::Frame`), else `ControllerCommand::Frame { job_id, homed }` and `Framing`. In `Framing`:
`Status` keeps the Ready job's `Progress` (`Progress` events never touch `held.line`); `SubmitJob`
`Busy`; `Start`, `Home`, `Jog`, `Pause`, `Resume` `WrongState` "Framing."; `JobFinished` returns to
`motion_from` (no `Done`, result, `RunNext` or `jobs.log` result); `JobFailed*` return with the
detail; `Alarm` drops a Ready job (M3); a closing client does not end it. **Stop**: fake at once, GRBL
"Stopping" until `Stopped` (G12 holds before the reset: no alarm), then `motion_from` with the job and
`homed` kept; Stop from anyone discards a waiting frame.

**F5. A member's frame (308 A).** As J7's Home, waiting with `requested_by`, `frame_id`, no job. The
owner's Confirm frame (W36), Enter and `curvyoctl confirm-start` send `Start { job_id: frame_id }`; a
member's is `WrongState`; `Confirms::Frame` with the frame's id, `admin.log` names Frame. *Rejected:*
the owner's own Frame press confirming (it carries the owner's page).

**F6. `-io`.** The hub passes `machine.frame_job()` with `Frame`; `jobs.log` `Frame: <client>`
(golden). `grbl/frame.rs` ("the frame's start: the travel check, then the stream"): homed with
`$130`/`$131` and WCO known, a miss of `fits` is `JobFailedWith` "Outside the travel by <n> mm" and no
byte (else GRBL's limits, as jog); then `prepare` and `Stream` as a job (flow, G12, loss clock). Fake:
runs the lines in injected time, moving its position.

**F7. Frontend and tests (no sleeps).** `connectorText.ts`: `Framing`, `jobButtons.frame` (W31
order), W32 rows; Framing's busy look comes from the status (Stop stays pressable, M11). Ring (W33):
`Frame::page_point` (laser-core, inverse of `machine_point`) via wasm; Head position offsets from the
first Framing position. Tests: laser-core goldens, box/hull tables; core F3 per word, `fits` per
corner, F4 per state in both machines (the next Start runs the held job, never listed), F5 rows;
simulator `tests/grbl_frame.rs` (`M5` first, no `M3`/`M4`/`S`>0, Stop mid-frame back to `Ready` with
the same id, outside the travel writes nothing); app `connector_frame`; frontend `jobButtons`. Spec
points for the PO: 22's "in Home's group" is W28's row; 25's "G0/G1" is `G1 … S0`; 28 adds corners.

Frage 334 (MC-FRAME-TRAVEL): What if a frame would leave the travel of a homed machine?
- **A** Refused before the head moves: "Outside the travel by <n> mm".
- **B** Clamped to the travel edge, as a jog.
- **C** No check; GRBL's soft or hard limits stop it with an alarm.
Recommendation **A** (a clamped frame shows a wrong outline). Default if unanswered: **A**.

## Second slice 2c (origin and start-from)

Architect, 2026-11-24, criteria 29 to 33 (Frage 312 A MC-USER-ORIGIN, 283 A, 282 C of `0140`). **No new
ADR, crate or dependency; no wire change; one `format_version` bump.** Amends ADR 0065 §1, §2 (a fourth
Start value, one page register) and ADR 0064 §7, §8 (the job's placement); the notes go onto both ADRs
with the story PR, after #468 (which edits ADR 0064) has merged. All is **(Proposal)** unless quoted.

**O1. Stored on the page (ADR 0065 §1 amended).** One process register on the laser page's
manufacturing node, beside `.start`: `x.curvyo.laser.user_origin_um` = `[x, y]`, two integers, the
point in **µm of the job file's coordinates** (O6), each within ±10,000,000 (10 m); absent = no user
origin. `.start` gains the spelling `"user_origin"`. *Rejected:* two registers `x`/`y` (a sync merge
of two devices could pair one's X with the other's Y; one list value is atomic in Loro); mm as
floats (rounding noise and `-0.0`; µm is the job's grid and GRBL's three decimals exactly); page
coordinates (a changed origin corner or bed size in the profile copy would move the physical point);
the machine profile (synced to every page, Frage 312 A says page; profile points are `0140`'s). Read in
`output_settings.rs`: `OutputSettings.user_origin: Option<PointUm>`; a value not of that shape is
damaged (counts one, never rewritten on read). Start `user_origin` with no readable origin reads as
`start_damaged` (blocks Export and Send with `StartDamaged`, ADR 0065 §1: never a guessed start).
Choosing another Start keeps the register (switching back shows it again); nothing deletes it in 2c.

**O2. Writes and steps (`curvyo-document-core/src/laser_output_commands.rs`).** `STARTS` gains
`("user_origin", "set_start_user_origin")` ("Start: User origin"); `set_start("user_origin")` on a page
without the register is `OperationError::NoUserOrigin`. New `Document::set_user_origin(manufacturing,
[i64; 2])` writes the register **and** `start = "user_origin"` in **one** commit `set_user_origin`
("Set origin"), so one undo takes back both (criterion 29); the typed X, Y pair of the Output group
calls the same command (the start is already User origin, so only the point changes); out of range is
`OperationError::BadUserOrigin`. Labels in `history_names.rs`, `step_scope.rs`, `history/rows.rs`.
The document-core list and `Start::ALL` stay equal by the session test of ADR 0065 §1.

**O3. Format (ADR 0065 §2: "a fourth value is a format change").** `CURRENT_FORMAT_VERSION` goes up by
one, to **61**: `main` is at 60 (59 is `0150`, #468; 60 is `0145` slice 3, #475). A build at 60 would read `"user_origin"` as a damaged
Start ("damaged file", the wrong message, as v49's reasoning); by the bump it refuses the file. No
migration: a file of 60 or lower opens unchanged. Goldens in `curvyo-document-core/tests/fixtures/`:
`user_origin_v61.curvyo` (page 1 `start` `user_origin` with `[123456, 78900]`, page 2 an origin with
`machine_origin`, page 3 none) and `user_origin_damaged_v61.curvyo` (`[1.5, 2]`, `[1, 2, 3]`, `"12,3"`,
and `user_origin` with no register: each reads damaged, counts one, saves back unchanged).

**O4. Start (`output_settings.rs`, ADR 0064 §7 amended).** `Start::UserOrigin`, fourth in `ALL`
(the pick's order, criterion 30); `relative()` and `homes()` false: absolute `G90`, no `$H` (no "Home,
then user origin" value; Proposal). The Output group's view lists User origin only while a readable
origin is stored, or while it is the stored start (then the pick shows it checked, or none when
damaged). `print_and_cut` gives User origin the warning it gives Head position (the measured marks
are bed positions; a moved job leaves them), its frame stays the unshifted bed map.

**O5. The shift and the anchor (`curvyo-laser-core`, pure, wasm32).** New `job_anchor.rs` ("the page
vector that puts a job's anchor on the user origin"): `burn_box(points)` and `anchor(box, corner)` (the
box corner at the profile's origin corner: bottom-left box corner for a bottom-left machine; in machine
coordinates always the box's min corner), shared with the gate's `bed_offset` (Head position, one rule)
and `job_frame`. `Job` gains `shift: PointUm` (page µm, zero for every other start) and
`Job::machine_point(p) = frame.machine_point(p + shift)`, `Job::page_point(m) = frame.page_point(m) - shift`;
`Frame` stays the pure bed mirror. `build_with` plans exactly as for Machine origin (entry from the
machine origin; ADR 0064 §7: "the plan never depends on the start mode", plans and cache are shared),
then takes the box of every plan path point and sweep end (the set the frame's `burn_points` and the
gate's box read), `A` its anchor, `U` the stored origin, `shift = frame.page_point(U) - A`. Run-outs
clamp to the bed at their placed position. So in machine coordinates **every burn line is Machine
origin's plus `U - A_machine`**. **Travel (lead's point 3):** `home = A`, so `Job::machine_point(home)
= U`: the file assumes the head stands on the user origin (where Set origin here left it), its first
line after the preamble is the `G0` to the first burn start, its last the `G0` back to `U`, so a second
run lands on the same place (as Machine origin starts and ends at the machine origin); the Toolpath
view draws exactly these travels. Writers (`job_write_grbl.rs` 59, 141; `job_write_frame.rs` 22) and
`job_frame::burn_points` switch from `job.frame.machine_point` to `job.machine_point`; the wasm
`head_place` keeps the bed map (W49). No `G10`, `G92`, `G54`.
*Rejected:* translating the moves and sweeps (clones every raster `Levels` vector); the shift inside
`Frame` (print and cut and the bed cross need the plain mirror; every bed reader would subtract it).

**O6. Which position "Set origin here" stores (lead's points 1, 2).** The head's position **in the
job file's coordinates**: GRBL's work position `Status.position` (`WPos`, or `MPos - WCO`), stored
verbatim, rounded by `head_position_um`, as `0121`'s Take head position (`print_and_cut.rs`). A `G90`
line `X u` goes to work position `u`, so the anchor burns exactly where the head stood, whatever the
controller's work offset (G54 etc.); **the shift subtracts nothing** and Curvyo writes no offset. With
no work offset (the case Curvyo leaves) this is the machine position of criterion 14. If another
program later changes the offset, the job moves with it, as every Machine origin job does; the W9
Notice stays the warning. **No sign conversion** (*rejected:* the jog clamp's `$23`/force-origin sign
rule of J6): that rule maps `MPos` onto the travel box for limits, while GRBL itself maps work
coordinates for the file; flipping a negative `WPos` would send the head to the mirror point. A machine
whose work coordinates run negative into the bed cannot run any Curvyo job (criterion 26 writes
positive coordinates), and the 4 b check then shows the placed job outside the bed, so the mismatch is
seen, not hidden. The X, Y fields (W43) show the stored numbers. Nothing is sent (Frage 312 A): **no
wire message, no connector change**; the frontend reads the last `Status` of `connectorLink` and calls
the wasm `set_user_origin(x_mm, y_mm)`. Enabled and greyed **as UX W40** (a read, like Take head
position): `homed` (2a J5) and a reported position required; greyed while stale, without position,
moving (Running, Homing, Jogging, Framing), in Alarm or a busy word; the motion rules of criterion 8 do
not apply. A member has no jog pad (W52). *Rejected:* a `SetOrigin` message or a position request (the
pushed `Status` already carries `homed` and both positions); `G10 L20`/`G92` (312 B, C). Frage 341.

**O7. What follows the pick.** The **work-area check** (4 b): `bed_offset` returns `job.shift` for
User origin, so `outside_bed` measures the placed job and **blocks, as criterion 31 says and as 4 b
does for Machine origin (lead's point 5: criterion 31 holds for 2c)**; `0140` criterion 12's "warning
only" is the PO's to align before `0140` is built. `Cause::OutsideBed` gains `over_um: Option<i64>`
(set only with User origin: the largest overshoot of the placed burn box over the four bed edges), so
the text reads "… bed by 12 mm." (W50 rounding, in `job_gate_text.rs`); Machine origin's text keeps
its bytes. The **frame** (2b): `build_frame_job` goes through the same `build_with`, so
its job carries the same shift; `outline`, `write_frame` (`G90`, back to the first point), the
"Outside the work area" cause and the connector's travel check F6 follow with no change; the head ring
stays on the bed map (`frame.page_point`, W49), around the place the job burns. The **Toolpath view** (`toolpath.rs`,
`raster_view.rs`, `play.rs`): the look stays built in page µm and is drawn translated by `shift` (one
translation in the draw step; `DrawKey` gains it), the ring with it. The **canvas cross** at the user
origin (criterion 32): the view JSON gives `frame.page_point(U)` in page mm while User origin is chosen;
the dashed burn box (W47) is the burn box plus `shift`, in the same view JSON; looks are UX's
(W46 to W48). The page's coordinates, rulers and drawing never change (criterion 32). 2d's focus point
maps a canvas point with `Job::machine_point`.

**O8. Relation to `0140` (criterion 33).** The user origin is the page list's first reference point
(Frage 282 C): `0140` reads entry 1 from `user_origin_um` and keeps further page points in a register
of its own, so no 2c file is rewritten; "Set origin here" becomes the page list's Store position;
`0140`'s Absolute / point / Current position map to Machine origin / User origin / Head position;
its nine-position anchor (Frage 283 A) is a register whose absent value is O5's corner, so a 2c file
burns the same under `0140`. 2c writes nothing to the profile. `0140` criterion 12 ("permit over
refuse") conflicts with criterion 31 here; 2c follows 31 (O7).

**O9. Another machine on the page (Frage 345 C; lead's point 4).** The page's profile copy already
names its record (`mfg_profile` id, ADR 0039); a **replace** is `write_profile` with `ProfileChange::Pick`
to a different id; `Reset`, `SaveAs` (same machine, new record) and every `Edit*` (origin corner, work
area, a library update `0116`) are **edits**. In `write_profile`, a Pick to another id on a page whose
`start` is `"user_origin"` writes `start = "machine_origin"` in the same commit (one undo restores
both); the point stays. No "taken on" id is stored (§5: the copy's id already tells it). Accepted edge:
a sync merge of one device's profile pick with another's concurrent Set origin can leave User origin
picked on the new machine; the bed check still measures it. Test: Pick another profile resets the
start, Pick the same id and each `Edit*` keep it.

**O10. Crates and tests (no sleeps).** `curvyo-document-core` (O2, O3, O9; set, undo restores start and
origin, `NoUserOrigin`, the profile-pick rule, goldens); `curvyo-laser-core` (O1 reading table; `anchor` per corner;
`tests/job_builder.rs`: per corner the User-origin file's burn lines equal Machine origin's plus the
offset; goldens `tests/fixtures/output/grbl/start_user_origin_{bl,br,tl,tr}.gcode` and
`grbl_frame/frame_user_origin.gcode`; gate: a user origin that pushes the box past the bed blocks 4 b,
one inside passes; every old golden keeps its bytes); `curvyo-ui-core` (labels); `curvyo-editor-wasm`
(the pick lists User origin only when stored, `set_user_origin` rounding and range, the Toolpath
shift, the cross and dashed box, the `over_um` text); frontend (Set origin causes table in `connectorText.ts`, the pick,
the typed pair). Untouched: `curvyo-connector-core`, `-io`, `-server`, `-ctl`, `curvyo-app`.

**Spec points for the PO (no question).** 29 "machine position" reads "the head's position in the job
file's coordinates (GRBL's work position, the machine position without a work offset)" (O6, Frage 341
A), and its causes are W40's; 30 has no "Home, then user origin"; 31's wording is W50's ("… bed by
12 mm."), and `0140` criterion 12 is aligned to the block before `0140` is built; 32's ruler wording
is corrected in `specification.md` ("the rulers do not change", UX point 2); print and cut warns for
User origin (O4).

Frage 341 (MC-ORIGIN-WPOS): Which head position does "Set origin here" store?
- **A** The position in the job file's coordinates (GRBL's work position), as Print and cut's Take head position: the job burns where the head stood, also when another program left a work offset on the controller.
- **B** The machine position: with a work offset on the controller the job burns off by that offset (the Notice of criterion 14 shows it).
- **C** As B, and Set origin here is greyed while the controller holds a work offset ("Work offset on the controller").
Recommendation **A** (equal to B without an offset, which Curvyo never writes; right also with one). Default if unanswered: **A**.

## Second slice 2d (focus point)

Architect, 2026-11-25, criteria 34 to 36 with 35 a to e (Frage 313 B MC-FOCUS; defaults of Fragen 369 B,
370 A, 371 A; ux-notes W53 to W64 and Frage 380 MC-FOCUS-MEMBER default A). **No new ADR, crate or
dependency; wire additive in protocol 0; one `format_version` bump and profile record format 8.**
Amends ADR 0035 §6, §7, ADR 0038 §2, ADR 0039 §1, §5, ADR 0071 decision 7 (P11); the notes go onto those
ADRs with the story PR. All is **(Proposal)** unless quoted.

**P1. Canvas to machine (`curvyo-editor-wasm`, pure laser-core underneath).** A press gives a page point
`p` (µm). Machine origin and Home, then machine origin: `job.frame.machine_point(p)` (the bed mirror per
origin corner, `0029` 26; no job needed, W55). User origin: `Job::machine_point(p)` of 2c O5 (`frame.
machine_point(p + shift)`), the job being `job_build::build_frame_job`'s (2b, Frage 335 B: it exists for
every page with items, also without power or speed), so the press lands where that drawing point burns;
User origin with nothing to build greys the toggle ("Nothing to place", text UX's). Head position is
greyed (criterion 34). The result is in **job file coordinates** (what a `G90` line writes, 2c O6), rounded
to µm and sent in mm. New wasm `focus_target(page, x_mm, y_mm) -> {x, y} | cause` and `focus_ring(page, x,
y)` (the inverse, `Job::page_point` / `frame.page_point`, for W59's ring). No clamp in the app (P4).

**P2. Wire (`message.rs`, `codec.rs`).** App to connector: `GoToPoint { x, y, pulse }` (`KNOWN_TYPES` 43),
`x`, `y` `Millimetres` in job file coordinates, each within ±10,000 mm (else `Malformed`); `pulse` left
out when none, else `{ power_permille, length_ms }`: `PulsePower` (u16, 1 to 50 = 0.1 to 5 %) and
`PulseLength` (u16 ms, 50 to 2,000) are bounded newtypes in `src/focus.rs` whose serde `try_from` refuses
anything outside (`Malformed`, nothing moves): **the connector owns both caps, whatever the app sends**
(35 c). New states `State::Focusing` (the move) and `State::Pulsing` (the pulse), two names so every app
shows "Focus point" and "Pulse" from the state, not from a sentence (M1's reasoning; lead's point 1).
`Status.target: Option<[Millimetres; 2]>` (omitted when `None`): the **clamped** target in job file
coordinates, set in `Focusing`, `Pulsing` and while a go to point waits for Enter. Goldens in
`tests/fixtures/messages/`: `command_go_to_point.json`, `command_go_to_point_pulse.json`,
`status_focusing.json`, `status_pulsing.json`, `status_awaiting_point.json`; old goldens keep their bytes.
*Rejected:* a frame job of one point (`JobKind::Frame`): F3 forbids `S` > 0 and must stay the proof that a
frame is laser off; a submit plus chunk per click is also three messages for one press. *Rejected:* reusing
`Jogging` (W58 and lead's point 1 need the own words; `JogStop`/dead-man semantics do not apply).

**P3. Core rules (`machine/focus.rs`, "how the job machine takes a go to point and its pulse").** Both
machines. `GoToPoint` in `Idle`, `Ready`, `Done` without a busy word and with `homed` (J5) gives
`ControllerCommand::GoTo { target, feed }` (the pulse stays in core's `FocusRun` and goes out as its own
`ControllerCommand::Pulse` after arrival; architect 2026-11-25, as built) and `Focusing`, `motion_from` as Home (a Ready job stays
held, its `Progress` in `Status`); refused with criterion 8's `WrongState` words in every other state,
"Alarm" in `Alarm`, **"Not homed"** without `homed` (the app greys it, W55; this is the backstop). In
`Focusing` and `Pulsing` a second `GoToPoint` is `WrongState` ("Focus point", "Pulse"; W58: the app sends
none), `SubmitJob` `Busy`, `Start`, `Home`, `Jog`, `Frame`, `Pause`, `Resume` `WrongState` with that word.
`feed` is `JogStep::Ten.feed()` (3,000 mm/min; -io caps it at `$110`/`$111` as J6). On `JogEnded` in
`Focusing` (P5): the last `Event::Position` within `ARRIVED = 0.05 mm` of `target` per axis, a pulse asked,
no stop pending, no busy word, `laser_off` and `laser_unknown` false (G34's flags) gives
`ControllerCommand::Pulse { power, length }` and `Pulsing`; otherwise back to `motion_from` with no pulse
(missed by more: detail "Did not reach the point", text UX's; laser mode off or unknown: no detail, W9's
controller warning already names it; lead's point 3). `PulseEnded` returns to `motion_from`; `PulseFailed {
detail }` returns with it. Under `LocalPress` a go to point waits (`Waiting::Point { target, pulse }`,
ADR 0070 decision 2); Enter runs it, Stop discards it. A closing client ends nothing (as a frame).

**P4. The clamp is one pure function (`src/focus.rs`; lead's point 2).** `clamp_target(target, wco, box) ->
[Millimetres; 2]`: machine target = `target + wco`, each axis clamped into `jog::TravelBox` (J6's box,
sign by `$23` and force origin), back to job file coordinates; no box (`$130`/`$131` unknown): unclamped,
GRBL's limits apply. Called once, by -io, with the settings only the connector knows; core stores its
result as `Status.target`, and the app draws W59's ring from that `Status` (it arrives with the answer, the
state being `Focusing` at once as Home's `Homing`), mapped back by `focus_ring`. So the ring **is** the
connector's target, not a second computation. *Rejected:* the app clamping with the core function in wasm:
the box needs `$23` and the after-homing force-origin test, which the wire does not carry, and two inputs
could disagree. *Amended (tester finding, Proposal):* core clamps too, in `go_to_point`, before
`Focusing` or the wait is entered, with the box -io names in `Event::Travel(Option<TravelBox>)` (sent
when it changes, reset on connect) and the reported work offset, so no `Status` of the move carries the
typed point beyond the travel; -io's clamp and `GoToTarget` stay and give the same result.
**Accepted (architect, 2026-11-25).** Still one function, now called at two layers with the same inputs
(`travel_box()` and the reported `WCO`, both from the one `GrblController` of the link, which is new per
connection, so "reset on connect" holds by construction). Core's call keeps the first `Status` honest;
-io's call is the byte-layer backstop that guards the `$J=` it writes, and its `GoToTarget` stays the
authority (it overwrites core's target, so a work offset that changed in between cannot leave the ring
and the head apart). `Event::Travel` is additive inside the process (not on the wire, no golden).
`ControllerCommand::GoTo`'s doc names the target as already clamped by core, re-clamped by -io.
`TravelBox` stays bare `[f64; 2]` (J6's debt, unchanged here); `clamp_target` must not panic on a box
with `min > max` (a `$130`/`$131` below 0 from a non-GRBL firmware): `Settings::travel` keeps only
values above 0, so `TravelBox::new` always gives `min <= max`.

**P5. GRBL bytes (`-io/src/grbl/focus.rs`, "the go to point jog and its pulse block, from the command to
Idle").** The move is a **GRBL jog in absolute work coordinates**: `M5\nM9\n` first unless `spindle_off`
(J6), then `$J=G90 G21 X<x> Y<y> F<f>` with the clamped target, owed as `Owed::Jog`, fast polling;
`JogEnded` at `Idle` with nothing owed, emitted after the `Position` of the same report; `error:15` gives
`JogFailed` (built text). A jog is refused by GRBL's soft limits instead of alarming, leaves the parser
state untouched and is cancelled by `0x85` with the position kept. The **pulse block**, written in one
write, never split:
`G1 M3 S<s>` / `G4 P<t>` / `M5 S0`, with `s = floor(power_permille x $30 / 1000)` and `t` the length in
**seconds** with three decimals (GRBL's `G4 P` is seconds; `P0.300`). It is written only when `$32=1` and
`$30` are known and `s >= 1`; else `PulseSkipped` (core treats it as a pulse left out). `ok` to `M5 S0`
gives `PulseEnded` (`Owed::Pulse`); an `error:<n>` on line 1 or 2 gives `PulseFailed` after the remaining
answers (GRBL still runs `M5 S0`); one on line 3 runs the G12 stop. Golden
`curvyo-connector-io/tests/fixtures/grbl/focus_pulse.gcode` (`M5`, `M9`, the `$J=`, the block).
- **Why `M3`, not `M4` (35 b's preference rejected).** GRBL 1.1 in laser mode sets an `M4` (dynamic)
  spindle to zero power whenever it is not moving (`spindle_set_state`: CCW in laser mode gives rpm 0), so
  an `M4` pulse at standstill fires nothing. `M3` with motion mode `G1` keeps the set power while stopped
  (GRBL's "M3 constant laser power" rule); a `$J=` does not set the parser's motion mode and GRBL starts in
  `G0`, under which laser mode disables the output, hence the bare `G1` (valid without axis words, no
  motion). 35 b's concern, a missed move burning a standing dot, is met otherwise: the pulse is written
  only after `Idle` within 0.05 mm, and the block carries its own end, so the controller switches the beam
  off after `t` even if the app, the connector or the cable is gone. To confirm on the real laser at the
  customer's try (the simulator cannot show a beam).
- **S from `$30`, not from the profile's max power.** The connector owns the power cap, and `$30` is the
  scale GRBL applies to `S`; with a profile that agrees (the built controller warning names a mismatch)
  both are the same percent. Hence `S` at most 5 % of `$30`, and at least 1 or no pulse.

**P6. Safety in the state machine (35 c).** No pulse in any state but `Focusing` after P3's arrival test,
so none in `Alarm`, `Hold`, `Door`, `Running`, `Paused`, a pending stop or a pulse already running.
**Stop** in `Focusing`: `JogCancel` as J3 (fake at once; GRBL "Stopping" until `JogEnded`), back to
`motion_from`, no pulse (criterion 10 amended: the move is a jog). Stop in `Pulsing`: `ControllerCommand::
Stop`, the G12 sequence (hold, reset, banner, `M5`, `M9`); `begin_stop` also starts when no stream exists
but an `Owed::Pulse` is in flight (as M4 for `$H`); the reset switches the laser off at once and, with no
motion, raises no alarm, so `homed` is kept (J5). A busy word (`Hold`, `Door`) or an alarm while
`Pulsing`: alarm as M3 (drops a Ready job; GRBL already stops the spindle); a busy word runs the G12 stop.
**Length cap twice:** `PulseLength` caps the dwell at 2,000 ms on decode, and core sets `deadline = now +
length + PULSE_GRACE` (1 s) on `Event::Clock` (J4); a `Clock` past it while `Pulsing` runs the G12 stop with
detail "The pulse did not end" (text UX's). **Losses:** the app's connection: nothing (no keep-alive; the
block ends itself); `ControllerLost`: `NoController` as Home (the controller has the block or was reset by
the port); the connector's shutdown while `Pulsing`: the hub sends `Stop` before closing the port, and the exit
waits for that stop to end (G12's reset, as for a running job; architect 2026-11-25: `stop_for_exit`
names "Focus point" and both exit waits include `Focusing` and `Pulsing`).
`--require-local-start`: P3's wait. A member: P11.

**P7. Profile fields (`curvyo-laser-core`; 35 a, W60 to W62).** `ProfileValues.focus_pulse: FocusPulse {
on: bool, power: PulsePermille, length: PulseMs }`, two `bounded_integer!` newtypes in `machine_profile.rs`
(1 to 50, default 10; 50 to 2,000, default 300), `on` default false (Frage 369 B), Generic laser the same.
Per mille, not a float percent (exact one decimal, no `-0.0`, as O1's µm). The UI clamps on commit and
rounds to 0.1 % and 1 ms (W60's notices); the ranges equal connector-core's by a `curvyo-editor-wasm`
test (laser-core does not depend on connector-core). **Record format 8** (`machine_profile_file.rs`): after
the default pair of record format 7 (`0150`), `"focus_pulse": false`, `"focus_pulse_power_permille": 10`, `"focus_pulse_length_ms":
300`, always written; format 5 to 7 records read the defaults and are not rewritten on read; one of the
keys in a format 5 to 7 record damages it (`deny_unknown_fields`, as ADR 0065 §3); above 8 is newer. **Page
copy** (`machine_profile_copy.rs`): the same three keys in `mfg_profile`, absent = default; a wrong type
or a value out of range reads as the default, counts one damaged value (`CopyRead` gains the flag) and is
never rewritten on read, so a damaged `focus_pulse` reads **off**. Pick, Reset, Save as and the edits write
them with the map; "Differs from profile" compares them exactly. Only the webview reads them, to build
`pulse` (P1): sent only when the copy has Focus pulse **and** Laser mode on.

**P8. Format (`curvyo-document-core`).** `CURRENT_FORMAT_VERSION` +1, to **63**: `main` is at 62 (59 is
`0150`, #468; 60 is `0145` slice 3, #475; 61 is slice 2c, #477; 62 is `0099` slice 1, #482). Needed for
ADR 0065 §4's reason: a build of 62 rewrites the whole `mfg_profile` map from its type on a
field edit and would drop the three keys silently; by the bump it refuses the file. No migration: a file
of 62 or lower opens with the defaults, unchanged. `ProfileChange` gains `FocusPulse { on }`,
`EditPulsePower`, `EditPulseLength`, one commit and undo step each, labels per W60 (`history_names.rs`,
`step_scope.rs`, `history/rows.rs`). Goldens `focus_pulse_v63.curvyo` (pulse on, 2.5 %, 500 ms; a second
page with the defaults; a third without the keys) and `focus_pulse_damaged_v63.curvyo` (`"on"`, `0`,
`2001`, `1.5`: each reads its default, counts one, saves back unchanged); `machine_profiles/
format8_focus_pulse.json` (round trip byte for byte), `format6_focus_pulse_key.json` (damaged),
`newer_format.json` moves to 9, every format 5 to 7 golden reads the defaults.

**P9. Logs and terminal (`-io`).** `jobs.log` `Focus point: <client>` when a go to point is taken (as
`Frame:`, criterion 12) and `Focus pulse: <client>` when the pulse block is written (35 c), so a press with
the pulse writes two lines: the move and the laser emission are logged apart (`LogResult::FocusPoint`,
`FocusPulse`; golden `jobs.log` gains both; a pad session's `Jog:` run ends at either). Terminal: "Focus
point sent by <client>", and for the Enter wait the line of a waiting start.

**P10. App, fake, crates.** `curvyo-app`: `connector_command` takes `"go_to_point"` with `{ x, y, pulse? }`
(fresh paired connection, `CommandAnswer`, as Home). Fake controller: the move takes injected time at the
feed and moves the fake position; the pulse is a fake dwell of `length` in injected time, then
`PulseEnded`; Stop cancels both; the fake reports `$32=1`, `$30=1000`. Simulator: `$J=G90`, `G1` without
axis words, `M3` power while idle in laser mode, `G4 P` seconds with the `ok` after the dwell, the byte
log. Crates: `curvyo-connector-core` (P2 to P4, P6, access P11, `journal.rs`: a waiting point is no
waiting Home, it carries `target`), `curvyo-connector-io` (P5, P6, P9, fake, simulator),
`curvyo-laser-core` (P7), `curvyo-document-core` (P8), `curvyo-ui-core` (labels), `curvyo-editor-wasm`
(P1, profile fields view, range test), `curvyo-app`, frontend (W53 to W64). Untouched: `-server` beyond
the terminal line in -io, `-ctl` (no member confirm, P11).

**P11. Members (Frage 380 A; ADR 0071 decision 7 amended).** `GoToPoint` is `Kind::Motion` in
`access.rs`, as `Jog`: a member gets `OwnerRequired`, maintenance mode `Maintenance`; the app has no
toggle for a member (W63). So no `Waiting::Point` for an owner's confirm, no `Confirms::Point`, no
`curvyoctl confirm-start` case. Decision 7's "`0118`'s motion waits for an owner" now holds for Home and
Frame; Go to point is owner-only like Jog. With Frage 380 B it becomes F5's path (a request id on the
wire, `Start { job_id }` confirms), additive later.

**P12. Tests, no sleeps.** Laser-core and wasm: `focus_target` per origin corner x {Machine origin, Home
then machine origin, User origin} (table, the user-origin row equals `Job::machine_point`), Head position
and no-job causes, `focus_ring` as its inverse; record and copy reading tables (P7, P8). Core: P3 per
state in both machines (`Not homed`, `Alarm`, busy words, `Ready` kept), arrival at 0.049 / 0.051 mm, no
pulse with `laser_off`, `laser_unknown`, a pending stop or a busy word, `PulseEnded`, the deadline by
`Clock`, Stop in each state, `Waiting::Point` with Enter and Stop, `clamp_target` per `$23` and box,
decode refusals for 0, 51, 49, 2,001 ms, the goldens, the access row. Simulator (`tests/grbl_focus.rs`):
bytes golden, `M5` first, no `S` above 0 before the block, the block in one write, `S` per `$30` (1000 and
255, the `s = 0` skip), `$32=0` writes no block, Stop mid-move (`0x85`, no block) and mid-pulse (reset,
`M5`, `M9`, no alarm, `homed` kept), `error:15`. Hub: both `jobs.log` lines, a second client sees
`Focusing`/`Pulsing` and may Stop. Frontend: W55 causes table, the mode reducer (W56), the pulse built from
the copy.

**Spec points fixed in `specification.md` (no customer question).** 10: a go to point's move stops with
the jog cancel, its pulse with G12 (P6). 12: Go to point is owner-only (Frage 380 A, P11); under
`--require-local-start` it still waits for Enter. 34: Home, then machine origin maps as Machine origin;
the ring is W59's at `Status.target`. 35 b: `M3` and the bare `G1` (P5); `S` from `$30`. 35 c: the end of
the pulse is the block's own `M5 S0` plus Stop, the deadline and the shutdown Stop; no keep-alive exists;
"member's press" is gone. 35 d: "What the maker sees" (ux-notes point 1). New Frage: none.

## Second slice 2e (run from line)

Architect, 2026-11-25, criteria 38 and 39 with `0132` criteria 12 and 13 (Frage 258 C, Frage 314 A by
default: interrupted jobs only), ux-notes W65 to W78 (Fragen 390, 391); one PR with `0132` slice 2 (its
notes L8 to L11 in `0132` adrs). **No new ADR, crate or dependency; wire additive in protocol 0; no
`format_version` change** (the project file is untouched; `jobs/list.json` stays `format: 1`, fields
added with `serde(default)`). Amends ADR 0071 decision 3 (one more owner-only message), as a dated line
with the story PR. Base: `main` with 2a to 2c; 2d (#484) is not needed. All is **(Proposal)** unless quoted.

**R1. Which ends interrupt a job (UX Frage 390).** Built (`0132` slice 1): a started job (running,
paused, a resume waiting) becomes an interrupted entry only through a connector restart (`stored::
restore`; the SIGTERM stop of `hub.rs`). That is Frage 390 A, the default: **nothing new is built for it.** *(Customer 2026-11-25: Frage 390 **C**; `machine/interrupt.rs` is built, see the implementation notes below.)*
With 390 B, a new `machine/interrupt.rs` ("how a started job that ends early stays listed as
interrupted") also keeps a job that ends by an alarm or a lost controller while the connector runs (end
state and `jobs.log` result line as built); with 390 C also one ended by Stop. `Done { Completed }`,
`Failed` and a job that never started are never interrupted; several interrupted entries count against
the 20 / 256 MiB limits (`0132` L2); an owner removes them (built). R4's second branch exists only with C.

**R2. What is stored at the interruption.** `Entry.interrupted_at` keeps its built meaning: the last line
GRBL **acknowledged** (`ok`, `Progress.line`). New persisted `Entry.stop: Option<StopPoint>`, `StopPoint
{ machine_position, work_offset: Option<[Millimetres; 3]>, run_from: u32, exact: bool }`: the machine's
last `MachinePosition`, the first job line of the run (1 for a whole run, n for a restarted one), and
whether the line is exact. `StoredCurrent` gains the same fields, written with its `line`; `exact` is true
only in the SIGTERM flush of `stop_for_exit` (and, with 390 B or C, at `Stopped`, `Alarm`,
`ControllerLost*`), false in L6's once-a-second writes, so a **crash** restores `exact: false` and the app
reads "near line n" (W66, `ListEntry.line_approximate`, R7). In memory only (`Item`, not serialized):
`restart: Restart`, R3 to R6 computed once when the entry becomes interrupted or is restored, and again on
`Event::Planner` (R9). The **homed flag is not stored** (criterion 39 amended): rule a reads it at the
restart; J5 clears it on every reset, alarm, loss and reconnect, so "homed now" is "homed since the last
break", and a connector restart always needs a Home. Modal state is not stored: it is rebuilt (R5).

**R3. The restart line n (criterion 39 amended; lead's point 1).** GRBL's `ok` means "in the planner",
not "burned": a stop loses up to one planner of acknowledged moves. `restart::restart_line(text, acked,
run_from, blocks) -> u32` walks back from `acked` over `blocks` motion lines (a line with an axis word in
`G0` to `G3`; an arc counts one, so the walk is never short) and returns the first of them, never below
`run_from`; `blocks` is GRBL's planner size from `$I` (`[OPT:V,15,128]`: 15), `PLANNER_BLOCKS = 16` until
read. So a few segments may burn twice, none is skipped. After a crash `acked` is the last stored line
(up to 1 s behind), so n is further back still, and `exact: false` says so. n is the **one number** of
W65: row, button, confirm and state line read `ListEntry.restart_line`. *Rejected:* `acked + 1` (as 39
read: up to 15 segments unburned); GRBL's `Ln:` (needs `N` words and a build option); `Bf:` (needs `$10`
bit 1, which Curvyo does not write, and lags a report).

**R4. The position proof (rule a), `restart::position(stop, now) -> Result<(), Cause>`** (`now` = J5's
`homed`, the last machine position and work offset). Missing now or stored: "No position from the machine"
(W40's words; a `list.json` of slice 1 has no `stop` and reads so). The **work offset** must equal the
stored one within `SAME = 0.01 mm` per axis **also after a Home** (the job's `G90` lines are work
coordinates; Curvyo writes no `G10`/`G92`, so a Home never clears it; lead's point 3): "Work offset
changed". Then `homed`, else **"Not homed"** (lead's point 4, W68 8; replaces "Home first"). Only with
Frage 390 C a second branch: no break since the stop (`Item.unbroken`, cleared by J5's events) and the
same machine position within `SAME` also prove it (a Stop keeps GRBL's count).

**R5. The modal rebuild, `curvyo-connector-core/src/modal.rs`** ("the machine state a G-code text leaves
before one of its lines"; pure, wasm32). `modal::before(text, n) -> Result<Modal, Unrebuildable>`. The
word reader (`strip`, `number`, `words`) moves from `frame_text.rs` to a new `gcode_words.rs` ("the words
of one G-code line"), refactor commit first. Lines 1 to n-1; comments, blank lines, `N` words and a `$H`
line skipped. Groups kept, the last value wins: motion `G0 G1 G2 G3 G80`; plane `G17 G18 G19`; distance
`G90 G91`, arc distance `G91.1`; units `G20 G21`; feed mode `G93 G94`; work system `G54` to `G59`; `F`;
`S`; spindle `M3 M4 M5`; coolant `M7`, `M8` (each on, both off by `M9`). Accepted without state: `G4` with
`P`, `G40`, `G61`, `M0`, `M1`. **Position** X, Y, Z in mm (`G20` values times 25.4), each `Option`: an
absolute move sets it, an incremental one adds to a known value, an unknown stays unknown; `G2`/`G3` end
points as `G1`. **Refused**, first one wins: any other G or M word (`G10`, `G28`, `G30`, `G38.x`, `G43.1`,
`G53`, `G92`, `M2`, `M30`, ...), another letter (A, B, C, ...), another `$` line (`$J=`, `$X`, ...), a word
that is not a plain number: "Cannot rebuild the state before line <n>: <word>"; X or Y unknown at line n
(a Head position job, `G91` from the top): "Starts at the head position"; n past the last line: the entry
is not offered (`restart_line` absent). Defaults before any word are GRBL's power-up ones (`G0 G17 G90 G21
G94 G54 M5 M9`, no `F`, `S0`). *Rejected:* storing the modal state (a second truth beside the text, lost
in a crash); parsing in `-io` (not pure, not wasm-checked).

**R6. The entry lines, `restart::entry(modal, z_now) -> Vec<String>`** ("the lines that take a stopped
machine to line n, laser off"). Order (criterion 38 amended to `0132` 12 d's order: the travel's `G21 G90
G1` would overwrite a state set before it):
1. `M5`, `M9` (laser and air off; criterion 9's rule holds for the travel).
2. `G21 G90 G17 G94`, and the job's work system when it set one other than `G54`.
3. the travel to the position before line n: `G1 X<x> Y<y> F3000 S0` (the 10 mm jog feed,
   `JogStep::Ten.feed()`; `G1 … S0`, never `G0`, F1; GRBL's planner caps it at `$110`/`$111`); a known Z
   as its own `G1 Z<z> F3000 S0` **before** XY when it goes up (from `z_now`, the work Z now) and after XY
   when it goes down; three decimals, never `-0.000`.
4. the rebuilt state in the job's words: `G20|G21 G90|G91 [G91.1] G17|G18|G19 G93|G94`, the motion word
   with `F<f>` when an `F` was set (`G1 F1200`; GRBL keeps a motion word without axis words as the modal
   value), `M7`/`M8` when on.
5. `S<s>` with `M3` or `M4` when the spindle was on, else `S<s>` alone (S is modal).
6. line n and the rest of the text, unchanged.
No `M3`/`M4` and no `S` above 0 before step 5; laser mode (`$32=1`, required, R8) fires nothing without
motion. Goldens per case (R11).

**R7. Wire (protocol 0, additive).** App to connector **`RestartFromLine { job_id, line }`** (`KNOWN_TYPES`
one more: 43, or 44 when #484 merges first); `line` is the n the confirm named (rule b): another n (the
entry changed under the press) is `Error { WrongState }` and a fresh `JobList`, as a stale `MoveJob`.
`ListEntry` gains, on an interrupted entry only, `restart_line: Option<u32>` (R3), `restart_cause:
Option<String>` (the connector's first failing cause of R8, omitted when a restart would run; lead's
point 1, W68: one rule, one place, the app greys with it and adds only its own "Connection lost" and
status-read texts) and `line_approximate: bool` (omitted when false; W66). Core recomputes the cause on
every event that can change it (state, word, laser flags, `homed`, positions) and pushes `JobList` only
when an entry changed (its built `last_list` comparison). **`Progress`** gains `from_line: Option<u32>`
(omitted when `None`; set through a restarted run, also in its `AwaitingLocalStart` and its `Done`) and
`entering: bool` (omitted when false): true from the accept until the head stands at the travel target
(R9), so the app reads "Moving to line n" while true and "Running from line n" after (lead's point 2,
W73); "Completed, lines n to total" from `Done` with `from_line`. `Progress.line` counts in the whole
text from n - 1. No new `State` or `ErrorCode`. Goldens in `curvyo-connector-core/tests/fixtures/
messages/`: `command_restart_from_line.json`, `job_list_restart_not_homed.json`,
`job_list_restart_near.json` (`line_approximate`, no cause), `status_restart_moving.json`,
`status_restart_running.json`, `status_done_from_line.json`; old goldens keep their bytes.

**R8. Core rules, `machine/restart.rs`** ("how the job machine takes a restart from a line"), both
machines. The connector's cause order is W68's without the app's own first two: the job text (R5);
the state (criterion 8's words; `Ready`: "Another job is ready", the restart takes the current-job place;
`Running`/`Paused`: "A job is running"; `Receiving`: "Sending"; the waits, "Homing", "Jogging",
"Framing"); `Alarm`, `NoController` and the busy words ("Controller state: <word>"); the laser mode ("Laser
mode is off ($32=0)", G34's unknown text); the position (R4: offset, then "Not homed"). `RestartFromLine`
for an unknown or not interrupted id is `UnknownJob`; a failing cause is `Error { WrongState }` with it.
Accepted: the entry leaves the list and becomes the held job (`line = n - 1`, `from_line = n`, `entering`),
`ControllerCommand::StartAt { job_id, from_line: n, entry, target }`, `Running`; `jobs.log` and
`admin.log` lines (R10). Under `LocalPress` it waits for Enter first (`Waiting::Restart`, ADR 0070
decision 2). Rights (`access.rs`): `RestartFromLine` joins `Kind::Reorder` (renamed `Kind::OwnerList`):
member `OwnerRequired`, maintenance `Maintenance`, foreground every paired client; it never waits for an
owner (W75). **An end before line n** (Stop, a discarded wait, an alarm, a loss, the travel check of R9)
puts the entry back in the list **unchanged** (W74: nothing burned; `stop` and `restart_line` as before)
and returns to `Idle` with the detail "Not started: Restart from line <n>"; no job result line. From line
n on the run is a job as any (`0127` G5 to G12, Pause, Stop, `0132` L5's chain after `Done`); how it can
be interrupted again is R1 with `run_from = n`.

**R9. `-io`.** `StartAt` streams `entry`, then lines n.. (`Stream::new_at(entry, lines, first_line = n)`:
entry lines are owed but count no job line); every line passes `prepare` (G5) before the first byte.
Homed with `$130`/`$131` and the offset known, `target` (work coordinates) is checked with
`jog::TravelBox` (F6's check and words, "Outside the travel by <n> mm", no byte); if 2d's `Event::Travel`
(#484, P4) is on `main` when this is built, the check moves into core's R8 instead, same words. **Arrival**:
`Event::EntryReached` once the travel line is answered and a report puts the work position within
`ARRIVED = 0.05 mm` of `target` (2d's constant if merged, else defined here), or once a job line past n
is answered (a report missed); core clears `entering`. `grbl/identify.rs` reads the planner size from
`[OPT:...]`, `Event::Planner { blocks }`. `hub/journal.rs` writes `StoredCurrent.stop` fields with the
line. Fake controller: `StartAt` runs from line n in injected time, `EntryReached` after its travel.

**R10. Logs.** `jobs.log`: `Restart from line <n>: <client name>` at the accept (`LogResult::Restart {
line, by }`; golden `tests/fixtures/jobs.log` gains it, old lines stay); the run's end writes the result
line as any job. `admin.log` (`0132` criterion 13): `restart-from-line`, owner, job name, n
(`ListLog::Restarted`; golden `admin.log` gains it). Terminal: "Restart <name> from line <n> by <client>".

**R11. Crates and tests (no wall-clock sleeps).** `curvyo-connector-core` (`gcode_words.rs`, `modal.rs`,
`restart.rs`, `machine/restart.rs`, `machine/interrupt.rs` only with 390 B or C, `Entry`/`StoredCurrent`
fields, R7, R8); `curvyo-connector-io` (R9, R10, `job_store`, simulator); `curvyo-connector-server`
(terminal line); `curvyo-app` (`connector_command` "restart_from_line" with `job_id` and `line`, fresh
connection); frontend (UX W65 to W78). Untouched: `curvyo-laser-core`, `curvyo-document-core`,
`curvyo-editor-wasm`, `curvyo-connector-ctl`. Core goldens `tests/fixtures/restart/<case>.gcode` and
`<case>.entry` (R6's lines for the n of the test table): `curvyo_m4`, `curvyo_m3_air` (copies of
`curvyo-laser-core` GRBL output goldens: M4 with `S` in `G1`; M3 with `M5`/`M3` around travels, `M8`),
`feed_and_s_change`, `inch_g20`, `z_up`, `z_down`, `arc_g2`, `home_first_line`; refused
`head_position_g91`, `unknown_g28`, `program_end_m2`. Tables: `restart_line` (15 and 16 blocks, arc,
`run_from` floor), `position` (homed, offset changed after Home, missing reports), the cause order per
state, `exact` per write. Machine tests `tests/restart_0118.rs` (both machines, `LocalPress`, each end
before line n back to the list unchanged, `entering` until `EntryReached`). `-io`
`tests/fixtures/jobs/list_interrupted.json` (golden with `stop`; a slice-1 file restores with `stop:
None`). Simulator `tests/grbl_restart.rs`: the entry bytes line by line, no `M3`/`M4`/`S` > 0 before step
5; Stop during the travel returns the entry; a restart after a connector restart refused "Not homed",
accepted after Home; a changed `WCO` (the test sets `G10 L20` in the simulator) refused after Home;
outside the travel writes nothing; `$32=0` refused; the crash case (`exact: false`). Hub/server: member
`OwnerRequired`, `--require-local-start` waits, both log goldens.

**Spec points fixed in `specification.md` and `0132` (no question).** 38: the order of the entry (R6),
`G1 … S0` at 3,000 mm/min (F1), the cause order and words (R8, W68; "Not homed" for "Home first"),
"Another job is ready"; 39: n backs off from the acknowledged line (R3), the homed flag is read, not
stored (R2), the work offset is compared also after a Home (R4), which ends interrupt is Frage 390; `0132`
12 a, c, d to the same words. No new Frage: UX Frage 390 is the one on which ends interrupt.

**Implementation notes (implementer, 2026-11-25, story PR).** (Proposal unless the customer answered.)
- *Frage 390 C* (customer): `machine/interrupt.rs` is the one place that decides which ends make an
  interrupted entry: a started job (past its line when it is a restart) that ends by Stop (on the
  confirming machine at `Stopped`), an alarm or a lost controller is listed as interrupted at its
  acknowledged line with the machine position and offset, `exact: true`, `run_from` its first line;
  `Done { Completed }`, `Failed` and a job that never started are not. R4's second branch is built: a
  break counter (J5's events: reset, alarm, loss, reconnect) proves "no break since the Stop", and the
  same machine position within 0.01 mm then proves the position without a Home. A new job still
  becomes current when the list holds only interrupted entries (they stand outside the order), and
  the current job is not shown twice while its Done row is also the interrupted entry.
- *Frage 391 C* (customer): the confirm reads "Restart <name> from line <n> of <total>?" only.
- *R9 arrival*: GRBL's `ok` to a job line after the entry means "in the planner", not "the head is
  there", so it does not end `entering`; `EntryReached` comes from a report within `ARRIVED` of the
  target after every entry line was answered, or the stream's end.
- *R5 arcs*: GRBL refuses `G2`/`G3` without axis words (error 26), so an arc mode cannot be set on
  its own: the entry writes `G1 F<f>` and line n must name its motion, else "Cannot rebuild the state
  before line <n>: G2".
- The fake controller reports its position after every homing (it reported none when the head
  already stood at its origin), so the position proof holds on the fake.
- The log goldens are files of their own, `jobs_restart.log` and `admin_restart.log` (the `0133`
  tester's rule keeps existing fixtures unchanged), not lines added to `jobs.log` and `admin.log`.
- The app's restart goes through `connector_list_command` (`ListCall::Restart`) and answers
  `refused` with the connector's cause (W71), not through `connector_command`.
- *`$32` read again (tester finding, safety)*: `$32` can change on the controller after connect
  (its own panel, another program). `StartAt` passes every check that needs no answer, then
  writes `$$` and waits for its `ok`; the settings read replace the cached ones (a changed laser
  mode reaches core as `LaserModeOff`/`LaserModeUnknown`, so the row's cause follows), and only
  with `$32=1` the entry's `M5` goes out. `$32=0`, an error or an empty answer end the restart
  before the entry ("Laser mode is off ($32=0)", back to the list unchanged); the next press is
  then refused at once from the fresh value. A Stop, an alarm or a reset while it waits ends it
  with nothing written but `$$`.
- *390 C's Stop proof (tester finding, safety)*: on GRBL 1.1 a soft reset during motion loses
  steps (`ALARM:3`); only a reset after the feed hold completed (`Hold:0`) keeps the position.
  The stop sequence records whether its reset followed `Hold:0`; when it did not (the hold
  limit passed, homing, a reset button before `Hold:0`), `-io` sends `ControllerReset` right
  after `Stopped`, which clears `homed` and counts a break, so the Stop proves nothing and the
  cause reads "Not homed". With `ALARM:3` seen, the alarm does the same. The proof therefore is:
  `Hold:0` before the reset, no break since, the same machine position and work offset.
