# Plan for send and start

Implementer, 2026-11-20. Base: `main` at `7c289c55` (0118 merged). PR #420 (pause while homing)
is open; it touches `frontend/src/lib/machineControls.ts`, so this branch keeps its changes there
small.

## Affected crates/modules

- `curvyo-ui-core/src/send_button.rs`: `SendMachine`, `send_starts`, `SendButton.starts`.
- `curvyo-editor-wasm/src/session/output/send.rs`, `wasm_output.rs`: `set_send_machine`, the
  view's `starts` and `job` (the rebuild generation, P15), `send_will_start`.
- `curvyo-connector-io/src/send_tracked.rs`: `send_tracked_open` hands back the connection that
  got `JobAccepted`; `send_tracked` keeps its shape.
- `curvyo-app/src/connector_send.rs` (+ a new `connector_start.rs` for the start step): the
  `start` argument, the first status of the send's own connection, `Start { our id }` on that
  connection, `StartView` in the answer.
- Frontend: `SendToMachine.tsx` (`CirclePlay`, armed look, keys), `useSendJob.ts` (arm, both
  presses, fresh status), `connectorLink.ts` (`start` flag, "Starting", "Not started"),
  `sendText.ts` (names), `sendArm.ts` (press rules), `MachineControls.tsx` (the "Not started"
  line; `machineControls.ts` is untouched for PR #420).
- Docs: `docs/design-system.md` rows, `0125` / `0118` UX note lines, spec status.

## Tasks

- [x] 1. ui-core: `SendMachine::parse(state, controller word)`, `send_starts`, `SendButton.starts`;
  table test over every state spelling x word x `enabled` (SS9, SS11, SS5).
- [x] 2. editor-wasm: `set_send_machine`, `starts` and `job` in `SendView`, `send_will_start`
  (SS9, SS13 P4, P15); session tests.
- [x] 3. connector-io: `send_tracked_open` returns the accepting connection (SS1).
- [x] 4. app: `start` argument; send `Start` with our own job id on the accepting connection when
  the maker's press asked for it and `send_starts(first status)` holds; `StartView` `Started`,
  `NotStarted { reason }`, `NoAnswer`; never `connector_command("start")`; the commit rule is not
  touched (SS1 to SS7). Tests: label rule = core's submit rule for every `State`; fake connector:
  free -> started, held -> busy no start, `--require-local-start` -> waiting, `$32` refused,
  start refused after acceptance -> `NotStarted` and job stays `Ready`, no start without the flag.
- [x] 5. frontend: `CirclePlay`; names per state; armed press through `useDeleteArm` (one armed
  action in the app), 400 ms guard, 4 s, Escape, press elsewhere, focus leaving, status/checks/job
  change; repeated keydown ignored (P14); arming press keeps focus (P9); P4 at both presses
  (P12, P13); busy "Sending" / "Starting" (P5); footer Start greyed while starting (P6); "Not
  started: <reason>" under the Ready line only while our job is `Ready` (SS6, P7); polite status
  region for the arm. Tests in `node --test` for the pure parts (SS9, SS13).
- [x] 6. docs: design-system rows "Send to machine button" and "Armed action" (accent case), the
  `0125` / `0118` UX note lines, spec status (SS12).

## Validation

- Full gate of `CLAUDE.md` §7, `cargo deny`, `scripts/check_docs.py`.
- The new tests 20 times under CPU burners (waits by statuses, never sleeps).
- Real app with `curvyo-connector-server --fake-controller` (tester); the real laser for timing of
  `Start` after `JobAccepted` and `--require-local-start` at the machine.

## Deviations from the addendum (Proposal, vetoable at the demo)

- **`connector_send_page_job` gains `start: bool`** (addendum S2 said it keeps its arguments). The
  host must know what the maker's press asked for: without it a press named "Send to machine"
  that meets a free first status would start (UX P4: never "only send" into "and start").
- **`SendMachine::parse(state, controller_word)`**: a controller word (`Door`, `Check`, `Sleep`,
  any other) makes the machine `Holds`, so nothing is armed there (SS5). The app reads the word
  from the status detail "Controller state: <word>".
- **The start answer is read by its own wait** (`connector_start::start_own_job`, 2 s bound)
  instead of `Connection::command`: `command` takes the first status whose state differs from
  `Ready`, which can be another client's change (a Stop's `Idle`) and would hide our `Error`. The
  wait takes our `Error` or a status that shows our job id past `Ready`.
- **`--armed-accent`** (PR #396, on `main`): the armed look uses it, an alias of `--editor-accent`.
- **UX 6 (b) is not reachable today**: in `Door`, `Check`, `Sleep` the connector refuses the send
  as `Busy`, so no job is held and "Not started: Controller state: <word>" never shows; the
  refusal reads as `0125` ("Connector busy").
- **"Not started" ends** on a state change, the next send of the page, or the footer's Start; an
  arming press alone does not end it.
