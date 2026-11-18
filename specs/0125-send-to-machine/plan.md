# Plan for Send to machine (0125, slice 3 of 0064)

Criteria S1 to S19. Design: architect addendum B1 to B8 in `specs/0064-machine-connector/adrs.md`;
UI: `ux-notes.md`. Branch `story/send-to-machine`, stacked on `story/machine-connector` (PR #375).
Commits (conventional, scope = crate, grouped by layer): core logic, document/session commit,
app send and watcher, frontend, tests and docs. No new crate, no new dependency.

## Affected crates/modules
- `curvyo-ui-core`: `send_button.rs` (enabling rule, cause order), `history_names.rs` and
  `step_scope.rs` (label `send_job`).
- `curvyo-connector-core`: `send_watch.rs` (`SendWatch`, commit-once).
- `curvyo-document-core`: `Document::export_job` takes the commit label; `SEND_JOB_COMMIT_LABEL`.
- `curvyo-editor-wasm`: `session/output/export.rs` (shared `hand_out`/`commit`), `send.rs` (send
  state, `send_job`, `send_job_done`, `send_job_failed`, link), `view_json.rs` (`send` in the group
  view), `wasm_output.rs` (wasm calls). Dev-dependencies (native only) for the S17 test.
- `curvyo-connector-io`: `Connection::send_job(job_id, ..)`, `new_job_id` pub, watcher retry rule (B5).
- `curvyo-app`: `connector_send.rs` (send command, `SendWatch` registry), `connector.rs` (watcher map).
- `frontend`: `lib/connectorLink.ts` (shared store), `useConnector` reads it, Output group Send button,
  state line and bar, `useSendJob`.
- `docs/design-system.md`: the rows named in `ux-notes.md` section 10.

## Tasks
- [x] 1. `send_button` with every cause and the cause order; Export disabled implies Send disabled (S1 to S5, S8, S16)
- [x] 2. `SendWatch`: Accepted/Rejected/NotReceived, reconnect, replayed status commits once (S11, S16, S19)
- [x] 3. Commit label `send_job` "Send job to machine", `Document::export_job(label, ..)` (S10, S12)
- [x] 4. Session `hand_out`/`commit` shared by export and send: same bytes, one undo step, Lasered stays
        after undo, unchanged second send adds no step, notice only for fixture jobs (S6, S10, S12)
- [x] 5. Output group view carries `send`; `set_send_link` (S1 to S5, S8)
- [x] 6. `Connection::send_job(job_id, ..)`, watcher retry rule (S11, S14)
- [x] 7. `curvyo-app/src/connector_send.rs`: send command, `SendWatch` registry, watcher map, `sendResolved` (S6, S7, S11, S14)
- [x] 8. Frontend: `connectorLink.ts` store shared by the Connector row and the Output group; Send
        button, state line, thin bar, live region, browser state (S1 to S5, S7 to S9, S13 to S15)
- [x] 9. S17 integration test against the fake connector; S19 cut-transfer test; S16 project bytes unchanged
- [x] 10. Design-system rows (S18); full gate and `cargo deny`

## Validation
Unit tests per task; `curvyo-editor-wasm/tests/send_to_machine.rs` drives the whole path against
`RunningServer` with the fake controller (waits on statuses and line counts, no sleeps). The
tester drives the Output group once in the running app and once in the browser build.

## Notes
`specification.md` stays `Status: Ready`; `Status: Done (PR #n)` is set after the PR number exists.

## Deviations and Proposals (build, 2026-11-17)
- The connector address and profile id come from the library record of the page's profile copy
  (`copy.id`), not from the copy: the copy never stores a connection field (`machine_profile_copy.rs`).
- `Connection::send_job` keeps its signature; `send_job_with_id` is new (the slice-2 tests keep
  calling `send_job`). `Connection::close` is new (used by the send tests to kill a connection).
- Cause order follows the addendum B2 (a blocking check first); UX notes U4 now carries the same order.
- The notice after an accepted send is the fixture line alone (UX U11), 3 s.
- S16 "project bytes": pinned as "a refused send writes nothing (bytes before = after)" and
  `CURRENT_FORMAT_VERSION` stays 48; an accepted send changes the project by design (S10).
- `watch` takes a `send_pending` predicate and ends on an idle connector (B5); the slice-2 end-to-end
  test passes `|| true` to keep its "retry after a restart" path.
- Unresolved send (tester, 2026-11-17, Proposal): the link store bounds it by `UNRESOLVED_SEND_MS` = 30 s on an
  injected timer; then "Timed out", `Session::send_job_timed_out` moves the pending send to a one-slot late
  commit (dropped by the next `send_job` or `send_job_failed`), the watcher is untouched (B5).
