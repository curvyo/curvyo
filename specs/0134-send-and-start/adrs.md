# ADRs for send and start

Architect, 2026-11-19. Covers criteria SS1 to SS12 of `specification.md`. **No new ADR, no new
crate, no new dependency, no protocol change, no `format_version` change**: `Start { job_id }`,
`JobAccepted` and `Status` exist (ADR 0035 §6), and the project file is not touched. Everything
below is a **(Proposal)** unless it quotes an ADR or a criterion. No customer question.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) §7: one job at a time, a second
  `SubmitJob` gets `Busy`. So today a send **is accepted only when the machine is free**
  (core's `submit` accepts in `Idle` and `Done` only). The connector's own rule is the fresh check
  at the press (SS9).
- [ADR 0070](../../docs/adr/0070-connector-through-the-relay.md) decision 2 and ADR 0071 decision 7
  (branch `story/connector-api`, spec `0128` A12 to A14): an owner's `Start` runs at once, a
  member's waits as `AwaitingLocalStart` with `requested_by`. In foreground mode
  `--require-local-start` holds it for Enter. Both rules live in the connector, so the app sends
  the same `Start` in every role (SS3). Nothing is bypassed.
- [`specs/0064-machine-connector/adrs.md`](../0064-machine-connector/adrs.md) B2 (`send_button` in
  `curvyo-ui-core`, no connector types there), B4 (`SendWatch`, commit at most once per job id),
  B5 (one status source per profile, `connectorLink.ts`), B7 (the webview never gets the token).
- `specs/0118-machine-control/adrs.md` (PR #399, open): the
  Machine controls group's Start (SS6), `Homing` refuses `SubmitJob` as `Busy` (M3). Start through
  `connector_command("start")` takes **whatever job is `Ready`** (`ready_job_id`). The page's
  start must not use it (S3 below).
- **Base:** after PR #399 is merged (the Output group and Machine controls of 0118).

## Feature-local decisions

**S1. The "will it start" function: `curvyo-ui-core/src/send_button.rs`**, beside `send_button`.
A ui-core enum, no connector types (B2): `SendMachine` from the connector's state spelling
(`SendMachine::parse("Idle")`, the JSON `state.name`). `Free` is `Idle` and `Done`; `Holds` is
everything else, `Homing` included; `Unknown` means no status yet. `pub fn send_starts(machine:
SendMachine) -> bool`. `SendButton` gains `starts: bool`: `enabled && send_starts(machine)`.
Callers:
- **Tooltip and accessible name** (SS9): the session's `OutputGroupView.send` carries `starts`. The
  frontend sets the machine with one wasm call `set_send_machine(spelling)` from the link store's
  last `Status`, like `set_send_link` (session state only, never in the document). The words "Send
  to machine and start" / "Send to machine" live in `connectorText.ts` (UX notes).
- **Glyph**: always `Play` (SS8), so it reads nothing.
- **Press** (S2): `curvyo-app` evaluates the same function on the **fresh** first `Status` of the
  send's own connection, never on the label.
No role and no `--require-local-start` input: neither changes what the press sends (SS3), and
the flag is not on the wire (§5: no speculative inputs). `0132` adds its list length here
(S6).

**S2. The flow: `JobAccepted`, then `Start` on the same connection, in `curvyo-app`.**
- `connector_send_page_job` (`connector_send.rs`) keeps its arguments. Its `open` closure keeps the
  first `Status` of the first connection (today `send_tracked` throws it away).
- `-io`'s `send_tracked` keeps its job, sending and following (one responsibility). It now also
  hands back the connection that got `JobAccepted`, still open (`None` on every other path). The
  app then sends `Start { job_id }` for **the id it sent** on that connection, when
  `send_starts(first status)` holds, with the existing `Connection::command(&Start, State::Ready)`.
  No fresh connection, no watcher involvement, no new `-io` option.
- The answer gains `start: Option<StartView>`, on `Resolved` and on `ResolvedElsewhere` (the send
  itself saw `JobAccepted`, so the job is known accepted even when the watcher took the
  resolution). `StartView` is `Started` (a `Status` `Running` or `AwaitingLocalStart`),
  `NotStarted { reason }` (an `Error` such as `WrongState` or `UnknownJob`, `Maintenance` after
  `0128`, or the connection dropping before the answer), or `NoAnswer` (no change within
  `COMMAND_WAIT`; the watcher's status decides).
- **No start after a dropped transfer** (`Unresolved`, or acceptance read from a fresh status by
  B4's re-read): the job stays `Ready` and the line reads "Not started: Connection lost" (SS6, no
  retry). The manual "Send job..." of the Machine controls group never starts.

**S3. Races.**
- The start names **our job id**, never the `Ready` job: core's `start` answers `UnknownJob` when
  another job is held. So a job of another app is never started by this press. Between our
  `JobAccepted` and our `Start` another app could stop our job and send its own: our `Start` is
  then refused and theirs stays `Ready`.
- Another app's `SubmitJob` while ours is held is `Busy` (ADR 0035 §7). Another app's job arriving
  between our fresh status and our `SubmitJob` makes ours `Busy`, so nothing starts (SS2).
- Another app starts our job first: our `Start` gets `WrongState`. **"Not started: <reason>" shows
  only while the watcher's `Status` still shows this job id `Ready`**, and clears on the next state
  change (as `0128` A26's "Not started"). A refusal that lost a race is never shown over a running
  job.
- `Start` never answers `Busy` (that is a `SubmitJob` reject reason only).

**S4. Commit at most once stays as built** (SS7, B4). The webview commits on `Accepted` for its
job id, from the command's answer or the watcher's `sendResolved`, whichever comes first. The
`Start` neither waits for the commit nor gates it. `SendWatch` already takes `Running`,
`AwaitingLocalStart`, `Paused` and `Done` with the id as `Accepted`, so the commit still happens
when the status limiter (5 per second) never shows `Ready`. A refused or failed start leaves the
commit done.

**S5. Owners today and after `0128`.** Until `0128` every paired app is effectively an owner: its
`Start` runs, or waits only under `--require-local-start`. After `0128` a member's `Start` waits
with `requested_by` ("Waiting for <owner> to confirm", `0128` A26). The app reads that from
`Status`; it does not need `Authenticated.role` for this story. Whichever of `0128` and `0134`
merges second has nothing to carry beyond the UX words. A member's label is the same as an
owner's, because its press does ask for the start.

**S6. `0132` (job list, not built).** With a list the connector accepts a `SubmitJob` while busy,
so "accepted" no longer means "free". The gate of S2 (`send_starts` on the fresh status) then
keeps such a send from starting (SS2). `0132`'s addendum adds the list length as a
`SendMachine` input ("free" = `Idle`/`Done` and an empty list). It also checks that `JobAccepted`
of a listed job is not followed by `Start`. Nothing of that is built now (§5).

**S7. Tests.**
- ui-core: the table over every state spelling (`NoController`, `Idle`, `Receiving`, `Ready`,
  `AwaitingLocalStart`, `Homing`, `Running`, `Paused`, `Done`, `Alarm`, unknown) times
  `enabled`, for `starts`.
- curvyo-app: **one test ties the label to the connector's rule**. For every `State`,
  `send_starts(SendMachine::parse(name))` equals "core's `Machine` accepts `SubmitJob` in it", so
  `0132` changing the accept rule fails this test until S6 is done.
- curvyo-app with `RunningServer` and the fake controller: free gives `Started` and the status
  `Running`. A held job gives `Busy` with no start. `--require-local-start` gives
  `AwaitingLocalStart`. `$32` refused uses the simulator or a core test. A start refused after
  acceptance (a second client stops the job between `JobAccepted` and `Start`, driven through the
  hub) gives `NotStarted` and the other job is untouched. A dropped connection after the last chunk
  gives no `Start`. All without sleeps (waits by statuses).
- Frontend: tooltip and name from `starts`; the "Not started" line only while the id is `Ready`.

## Spec conflicts fixed in `specification.md` (2026-11-19)

1. SS9 tied the label to SS1's owner condition, but SS3 has a member's press send `Start` too.
   The label now follows SS1's machine conditions in every role (S5). The 0125 meaning bullet
   under "Changes accepted behaviour" now says the same.
2. SS11 asked for a table over role and `--require-local-start`. Neither changes the press, and
   the flag is not on the wire. The table is now over the state (and the list once `0132` exists).
   Role and the flag are in the integration test (S7).
3. SS6 said a failed start always leaves the job `Ready`. The connector's second guard (`0127`
   G4 in `Door`/`Check`/`Sleep`, G34 for `$32`) ends a `Start` as `Done { Failed }` with its words
   before any byte. SS6 now keeps `Ready` for refusals and drops, and shows the failure for those.
4. SS5 implied that a job is sent in `Door`, `Check` or `Sleep`. The connector refuses it there as
   `Busy` (`0127` G4). SS5 now says so.

No other conflict. Not in the spec, decided here: S2 (same connection, `send_tracked` hands it
back), S3's id rule and when "Not started" shows. For the product owner: SS2's list does not
name `Homing` (`0118`). It refuses the send as `Busy` like the others, and S1 counts it as
`Holds`.
