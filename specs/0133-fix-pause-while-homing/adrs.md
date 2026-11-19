# ADRs for the fix "a Pause pressed while the machine homes is forgotten"

Architect, 2026-11-19. Covers criteria PH1 to PH11 of `specification.md`. **No new ADR, no new
crate, no new dependency, no `format_version` change.** One additive wire field inside protocol 0
(ADR 0035 §6, not released). Everything below is a **(Proposal)** unless it quotes an ADR or a
criterion. No customer question.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) §6, §7: `Status.pause_pending` is
  one more protocol-0 field (additive, golden file); `Pause` stays idempotent, `Stop` is never
  held.
- [ADR 0070](../../docs/adr/0070-connector-through-the-relay.md) decision 2 and ADR 0071 decision 7
  (branch `story/connector-api`, spec `0128`): `Resume` counts as a start because it restarts
  motion. A `Resume` that only **cancels a pending pause** restarts nothing (the job never stood
  still), so it is not a start and is not held for a member (P7).
- [`specs/0064-machine-connector/adrs.md`](../0064-machine-connector/adrs.md) C2 (the
  `Controller` trait: events of `command` are fed to the `Machine` in the same lock, in order),
  C3 (the pure GRBL part in `-io/src/grbl/`), C4 (the unconfirmed-Pause rule of G29, amended
  here by P4), C5 (core's confirmed controls), C6 (additive `Status` fields), C7 (laser off in a
  hold).
- [`specs/0118-machine-control/adrs.md`](../0118-machine-control/adrs.md) (PR #399, open) M3, M4:
  the state `Homing` is entered **only** by `Message::Home` in core's `machine/home.rs`; a job's own
  first-line `$H` (`0029` criterion 38) stays `Running` with the controller word `Home`. The 0129
  loss-clock pause (`during_homing`, commit `217ab6ec`) is unchanged.
- **Base:** PH3 needs `State::Homing`, so `fix/pause-while-homing` starts from `main` after PR #399
  is merged (or from `story/machine-control` and waits for it).

## Feature-local decisions

**P1. Cause (architect's reading, the implementer confirms it first, PH10).** In a job, GRBL
answers `$H` only when homing ends and ignores `!` while it homes. Core's confirming machine
does not change state on `Pause` (C5), so no `Status` changes and the app shows nothing.
`GrblController::pause` writes `!` and sets `Hold::Requested(at + 2 s)`; `on_time` drops it to
`Hold::Running` after 2 s (G29, "GRBL ignores `!` while homing"), or the `ok` comes first and the
request is dropped later; either way `fill` streams on. The pause is dropped **by design**.

**P2. Wire: `Status.pause_pending: bool`** (`message.rs`), `#[serde(default, skip_serializing_if
= "std::ops::Not::not")]`: omitted when false, so every existing golden decodes and re-encodes
byte for byte; new golden `status_pause_pending.json` (a `Running` job, `detail` "Controller
state: Home", `pause_pending: true`). Top level beside `detail`, as PH1 asks. Meaning (doc
comment): "a `Pause` was taken and waits for the controller's first moment to hold". Every
`Message::Status { .. }` literal gains `pause_pending: false` (mechanical, about 125 sites, as G20
allowed for `detail`); no other assertion changes. TypeScript `StatusMessage` gains
`pause_pending?: boolean`.

**P3. Core (`curvyo-connector-core`, pure, wasm32).** The controller decides when a pause has to
wait, not core: core does not know which line is in flight, and a rule keyed on the word `Home`
would miss the window between writing `$H` and the first report (0118 M1 rejected state from
words for the same reason).
- New internal event `Event::PauseDeferred` ("the controller cannot hold now and holds at its
  first safe moment"). In `Running` without a stop pending it sets `pause_pending` and pushes a
  `Status`; anywhere else it is ignored. `GrblController::command(Pause)` returns it, so the
  answer to the `Pause` is that `Status` (PH1).
- `pause_pending` is cleared by: `Held` (state `Paused`, PH2); `Resume` in `Running` (clears it,
  pushes `Status`, sends `ControllerCommand::Resume`, PH4); `Stop` (the stop starts as built; the
  shown detail is "Stopping", PH5); and every state other than `Running` (`enter()` clears it as
  it clears `stop_pending`: `Done`, `Alarm`, `NoController`, PH6, PH9).
- `Pause` in `Running` with `pause_pending`: nothing (idempotent, PH4, P17). `Resume` in `Running`
  without it stays a no-op.
- `Pause` in `Homing` (the Home button, with or without a kept `Ready` job): `Error { WrongState }`
  with the message "No job to pause" (`machine/home.rs`), nothing remembered (PH3). Other refusals
  keep the generic sentence.
- `Machine::new` (the fake controller) never sees `PauseDeferred`: its `Pause` holds at once.

**P4. GRBL (`curvyo-connector-io/src/grbl/`).** `stream.rs` owns the rule; `controller.rs` (about
470 lines) only dispatches.
- `Hold` gains `Deferred` ("`Pause` arrived while the job's `$H` is in flight; nothing written").
  `pause()`: if `stream.homing()` and the hold is `Running`, the hold becomes `Deferred`, **no `!` is
  written** and `Event::PauseDeferred` is pushed (PH1). Otherwise as G10.
- **The pending pause is applied before the next line, in the same `advance`.** `$H`'s `ok` is read
  in the `on_line` loop (`on_answer`), and `fill` runs only after that loop. When `on_answer` frees
  the `$H` (`stream.answer()`) and the hold is `Deferred`, it writes `!` and sets the hold to
  `Requested` **without a drop limit**. `fill` returns at once for every hold other than `Running`,
  so no job line can follow `$H` (PH2). One `Stream` method returns "send the hold now", so the
  check and the hand-over sit next to `fill`'s own hold check.
- Why `!` after the `ok`: GRBL is then `Idle` with an empty planner and enters `Hold:0` at once
  (the simulator models it). From there it is GRBL's own feed hold: `Held` and `Paused` by G10,
  C7's laser-off rule (`A:` field, `0x9E`), `Resume` by `~` and `Run`/`Idle` (G11), and `Stop` from
  `Paused` starting at the reset (G12), all as built. `pause_pending` stays true until `Held`.
- **G29 amended (PH7):** the 2 s drop applies only to a `!` written while a line other than `$H` is
  in flight. While `$H` runs no `!` is written, so nothing can be dropped. The `!` after the `ok` is
  never dropped: if `Hold:0` does not come, the job stays `Running` with `pause_pending`, no line
  goes out, and `Resume`, `Stop` or the loss rule (G15) end it.
- `resume()`: `Deferred` becomes `Running`, nothing written (homing goes on, then streaming);
  `Requested` without limit writes `~` and becomes `Resuming` (G11 confirms). `begin_stop`:
  `Deferred` counts as "no hold sent", homing as today, so the stop is the reset without a feed
  hold (G9, PH5).
- Failure (PH6): `error:<n>` to `$H` fails the job (`begin_stop` with the cause), `ALARM:8`/`9`
  raises the alarm, and a banner, a reset or a lost link end the stream. The `Deferred` hold goes
  with the stream; core's `enter()` clears the flag. The state is never `Paused`.
- **Laser:** no job line is written between the press and `Resume` (fill blocked, PH2). After the
  `ok` the hold is GRBL's, with C7's guarantee. Jobs with `$32=0` or unknown never start (G34).
  Stop, loss and alarm keep their `0x18`/`M5`/`M9` paths.

**P5. Fake controller and simulator.** No change to either; no new flag (`CLAUDE.md` §5). The
simulator already has the silent homing of `0129` (`quiet_homing`, `homing_steps`, time in
simulator steps), ignores `!` while homing as GRBL does, and holds at once on `!` in `Idle`. The
fake controller does not parse `$H` and holds at once: its tests cover PH3 with the Home
button's `FAKE_HOMING` (0118 M6) and that a normal Pause is unchanged.

**P6. App and frontend.** No `curvyo-app` change (the watcher forwards `Status` unchanged, B5).
`connectorText.ts` / `machineControls.ts` (PR #399): the state line reads "Pause after homing"
while `pause_pending`. Pause shows the pending look (UX notes) and Resume is enabled in `Running`
with `pause_pending` and in `Paused`. All of it comes from `Status`, so a second app reads the same
(PH8).

**P7. Rule for `0128` (owners and members).** Whichever merges second carries it. A member's
`Pause` is taken at once (A13, Frage 249 A), deferred pause included. A member's `Resume` while
`pause_pending` only cancels and runs at once, since nothing stood still. A member's `Resume` from
`Paused` waits for an owner as A13 says.

**P8. Tests (no wall-clock sleeps; stepped time).**
- Core (`confirming_machine.rs`): `PauseDeferred` in `Running` sets the flag (one `Status`),
  elsewhere ignored; a second `Pause` gives nothing; `Held` gives `Paused` with the flag false;
  `Resume`, `Stop`, `Alarm`, `JobFailedWith`, `ControllerLost` clear it; `Pause` in `Homing`
  with and without a `Ready` job is `WrongState` "No job to pause"; golden `status_pause_pending.json`.
- `-io` with the simulator (`quiet_homing`, `homing_steps` larger than the 2 s hold limit in
  steps): PH1, no `!` in the byte log between `$H\n` and its `ok`. PH2: the received lines end
  with `$H`, then `!` and `Hold:0`, `Paused` with `job.line` equal to `$H`'s line, spindle off
  and power 0 throughout. PH4: Resume before the `ok` (no `!`, no `~`, streaming after homing),
  Resume after `Paused` (`~`). PH5: Stop while deferred (reset, no `!`). PH6: alarm 8,
  `error:5` with `$22=0`, a reset during homing, a lost port. PH7: a normal pause, and an
  unconfirmed pause on a motion line, unchanged. PH9 in the pty end-to-end test.
- `stream.rs` unit tests: `fill` writes nothing while `Deferred`; the `$H` answer hands over
  exactly once.
- Frontend: state-line and button tables with `pause_pending`.

## Spec conflicts fixed in `specification.md` (2026-11-19)

1. The test preamble asked to extend the simulator with a silent homing "measured in simulated
   lines". It has existed since `0129` (`quiet_homing`, `homing_steps`, counted in simulator
   steps). Reworded (P5).
2. PH1 allowed `pause_pending` in the state `Homing`, but PH3 refuses `Pause` there, and a job's
   `$H` never enters `Homing` (0118 M4). It also said "line 1", but the count before `$H`'s `ok`
   is the lines acknowledged so far (G8). PH1 now names `Running` with the controller word `Home`.
3. PH2 said "the line count at the line after `$H`" and did not say when `Paused` is reached.
   Now: `job.line` is `$H`'s line number (lines acknowledged, G8), so Resume goes on with the
   next line. The connector writes the real-time `!` (not a line) after the `ok`, and the state
   is `Paused` at GRBL's `Hold:0` (P4).

No other conflict. Not in the spec, decided here: P3 (the controller decides, `PauseDeferred`),
the drop-free `!` after the `ok` (P4), the member rule (P7). For the product owner: `0064` P17/Q15
and `0127` G29 are amended as "Changes accepted behaviour" says; addendum C4's unconfirmed-Pause
sentence gets a pointer to P4 in the fix PR.
