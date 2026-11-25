# 0132 Job list on the connector: architecture notes

Architect, 2026-11-23, on ADR 0035, ADR 0071 and `0064` adrs C5, C6, D3 to D10. All **(Proposal)**.

**No new numbered ADR.** Fragen 257 A, 258 C, 259 B fixed the behaviour; the rest extends existing
decisions: core messages on the same socket (0071 d4, protocol 0 additive), one more state-directory
entry (d9), the role table (d3, D4), ADR 0035 §7's "one held job, else `Busy`" relaxed for
`SubmitJob`. No crate, dependency, trait or project-file change. ADR 0078 (next free) stays unused.
For the lead in the slice commit: dated amendment lines in ADR 0071 and ADR 0035 §7, index row 0071.

**L1. Slicing.** Slice 1: criteria 1 to 9, 11 (without "restart pending"), 14. 13 is only the
restart line, so it goes with 12 to slice 2, after `0118` run-from-line. Hook: an interrupted
job stays listed (`Interrupted at line <n>`), `Start` on it is `WrongState`, an owner removes it.
Slice 2 adds optional stored fields (position, offset) and "homed since" from `Event::Homed`.

**L2. Pure list core**, `curvyo-connector-core/src/job_list/` (no clock, no I/O, wasm32, no new
dependency): `mod.rs` order and limits, `rights.rs`, `chain.rs`, `stored.rs` (format and restore).
`Entry { job_id, name, kind, lines, bytes, sha256, sent_by, run_next_by: Option<Peer>,
interrupted_at: Option<u32> }`. `Machine` owns one `JobList`, glue in `machine/list.rs` (`mod.rs` is
at 494 lines). The current job stays the machine's held job; the list holds the jobs after it.
- *Arrival:* a `SubmitJob` becomes current (`Receiving`, as today) only with no current job (`Idle`,
  `Done`), no waiting entry and no other transfer; else it is a waiting transfer keyed by job id
  (`Vec<Assembly>`), listed in order of completion. `Event::ClientDisconnected` carries the job id
  the session announced, so a dropped transfer drops only its own assembly (`0125` S19).
- *Refusals:* laser mode (G34), size, hash, ASCII, `NoController` as today; Door/Check/Sleep
  (`Busy`) only for a job that would become current. `JobKind::Frame` is never listed: with a
  current job it stays `Busy` (a frame is an act, not queued work).
- *Limits:* 20 entries and 256 MiB over waiting, interrupted and in-flight transfers (announced
  `bytes`, reserved at `SubmitJob`); the current job counts alone (64 MiB). Beyond: `ListFull`.
- *Start:* `Start { job_id }` with the current job `Done` or absent and `job_id` the first waiting,
  not interrupted entry: it becomes `Ready`, then D5's rule (owner runs, member waits for an owner,
  `LocalPress` waits for Enter). Any other id or state: `WrongState`, then `JobList`.

**L3. Messages** (protocol 0, additive; existing goldens re-encode byte for byte). `JobAccepted`
gains `position: Option<u32>` (omitted when `None` = became current; `Some(n)` = n-th waiting,
1-based); `RejectReason::ListFull`. New `JobList { current: Option<ListEntry>, waiting, note }`,
`ListEntry { job_id, name, lines, bytes, sent_by, position, run_next, interrupted_at }`, `note` =
"Chain stopped: <state>"; pushed to every authenticated connection on each change and **before**
a connection's first `Status`, outside the status limiter. `Status` stays as is (it is coalesced,
the list must not be). `MoveJob { job_id, from, to }` (`WrongState` + `JobList` when the job is
not at `from`, `to` is out of range, or it is current or interrupted), `RemoveJob { job_id }`,
`SetRunNext { job_id, on }`; unknown id `UnknownJob`; no `GetJobList`, no new `ErrorCode`. Goldens:
one each, `job_accepted_waiting`, `job_list_chain_stopped`, `job_list_interrupted`.

**L4. Rights** (criterion 5). `access.rs` gains `Kind::List`: maintenance `Maintenance`, not logged
in `PairingRequired`, member: `MoveJob` `OwnerRequired`, the other two pass on; owner and foreground
paired: yes. `rights::may(action, peer, entry)`: a member removes or flags only entries whose
`sent_by` is its client name, else `OwnerRequired`; only an owner removes an interrupted entry.
Foreground mode has no owners: every paired client acts as owner there. Tests: both tables whole.

**L5. Run next** (criterion 11): `chain::next(end, first, ready, rule) -> Chain`, decided only at
the transition that ends the current job, never later. `Done { Completed }` + first waiting flagged
+ ready (connected, no alarm, no Door/Check/Sleep, laser mode on): `Start` (owner's flag runs; a
member's flag waits for an owner with `requested_by`, so `0128` A13 holds; `LocalPress` waits for
Enter). Unflagged or empty: nothing. Flagged, not ready: `Stop`. `Done { Stopped | Failed }`, alarm
or controller lost from `Running`/`Paused`: `Stop` = clear every flag, set the note (if a flag was
set); an owner's `Start` clears the note. No delayed start: a door closed later never fires the
laser. After a restart nothing ends, so nothing starts; flags act after the next manual start.
Stop ends only the current job (G12). Home (`0118`) ends no job, so it never chains.
Tests: end x flag (none, owner, member) x ready x `StartRule`; fake chain of three, 2nd fails.

**L6. Persistence** (`-io`, module `job_store`). `<state dir>/jobs/` (`0700`): `<job_id>.gcode`
(`0600`, at acceptance), `list.json` (`format: 1`, golden; current with `started`, `line`; temp,
fsync, rename). All writes on one ordered writer thread, **never under the hub lock** (a 64 MiB
write must not stall the serial thread mid-job); `Job.text` becomes `Arc<str>`. `JobAccepted`
precedes durability (a crash then loses the job; the guide says so). The line is written at most
once per second and GRBL acks before it moves, so `<n>` is approximate; slice 2 picks the back-off.
Restore (pure `stored::restore(list, texts)`, table test): never started current comes back first
in the list; started (running, paused, resume waiting) becomes interrupted at its line; `Done` and
`Receiving` are dropped; flags kept; nothing starts. The SIGTERM stop (C11) marks the job
interrupted first. A missing text, a failed `Assembly` check or an unparsable entry: dropped with
`admin.log` `job-dropped` (name, `<id> <cause>`), the rest stays; unreadable `list.json`: all
dropped, `job-list-dropped`; orphan texts deleted. Also logged: `job-removed` (an owner removing
another's job), `run-next-set`. Reset (A19) deletes `jobs/`: `state_dir::empty` removes subfolders.
Texts stay in RAM (about 320 MiB at most); the service guide names it, a debt entry covers 512 MB.

**L7. Clients.** `SendWatch`: `NotReceived` only from a `JobList` without the job, `Accepted` also
from one with it (`send_tracked_open` reads the first `JobList`). `0134` S6: `SendMachine` gains the
waiting count; no `Start` after a `JobAccepted` with a position. Hub's `submitted` goes per session.

## Questions for the customer

Frage 302 (QUEUE-SLICE): Restart from the interrupted line needs `0118` run-from-line first.
A: ship the list now (interrupted job shown, removable), restart from its line as a second PR.
B: one PR after `0118` run-from-line. Recommendation A. Default if unanswered: A.

Frage 303 (QUEUE-MEMBER-FLAG): A member marks its own job "Run next automatically".
A: it follows the job before it but waits for an owner's confirmation, like every member start.
B: it starts at once, as if an owner marked it. C: only owners mark jobs.
Recommendation A (one member start rule, `0128` A13). Default if unanswered: A.

## Slice 2 (restart from the interrupted line, criteria 12 and 13)

Architect, 2026-11-25. Built in one PR with `0118` sub-slice 2e; the decisions are `0118` adrs R1 to R11
("Second slice 2e"), ux-notes W65 to W78 (`0118`) and section 11 here. **No new ADR, crate, dependency or
`format_version`**; protocol 0 additive; `list.json` stays `format: 1`. All is **(Proposal)**.

**L8. What slice 2 adds to the list core.** `Entry.stop: Option<StopPoint>` (machine position, work
offset, `run_from`, `exact`) and the same fields on `StoredCurrent` (`0118` R2); `Item.restart` (the
restart line, the rebuild and its cause, computed once per interrupted entry and on `Event::Planner`,
R3, R5). `stored::restore` fills `restart` and keeps `exact` (false after a crash). Which ends make an
entry interrupted is UX Frage 390, answered C by the customer 2026-11-25: a connector restart (L6), Stop, an alarm or a lost controller (`0118` adrs, implementation notes; `machine/interrupt.rs`).
L1's "homed since from `Event::Homed`" is 2a's J5, built.

**L9. Messages** (amends L3). `RestartFromLine { job_id, line }` (owner only, `0118` R7, R8);
`ListEntry` gains `restart_line`, `restart_cause`, `line_approximate` on an interrupted entry; `Progress`
gains `from_line`, `entering`. A stale `line` answers `WrongState` and a fresh `JobList`, as `MoveJob`.

**L10. Rights and chain** (amends L4, L5). `access.rs`: `Kind::Reorder` becomes `Kind::OwnerList` and
takes `RestartFromLine` (member `OwnerRequired`; foreground every paired client). Criterion 11's "a
restart is pending" is the restart waiting for Enter (`Waiting::Restart`) or entering (`Progress.entering`):
the machine is not `Done` then, so `chain::next` is never asked; a restarted job that ends `Done` chains
as an owner's Start (L5). Interrupted entries are never `first_waiting` (built).

**L11. Logs** (criterion 13). `admin.log` `restart-from-line` (owner, job name, line n), `jobs.log`
`Restart from line <n>: <client>`; goldens in `curvyo-connector-io/tests/fixtures/`. Restore after a crash
logs nothing new (L6's `job-dropped` stays).
