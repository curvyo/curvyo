# Job list on the connector: send jobs while the laser works, reorder, start

Status: Ready for slice 1 (adrs.md and ux-notes.md done 2026-11-23). Slice 1 = criteria 1 to 9, 11 and 14; criteria 12 and 13 are slice 2 after `0118` run-from-line. Fragen 301 (start press), 302 (slicing) and 303 (member flag) are applied by their defaults and vetoable at the demo
Priority: Should
Origin: Customer, first laser test, 2026-11-18. Translated: "The owner needs a job list so that jobs can be sent while the laser cutter is still working. The owner can then change the order and press start."

## User value

As the owner of a shared laser I want jobs to queue on the connector while it is busy, so that members and I can send work any time, and I can reorder it and start the next job myself (`R-MFG-LASER-002`, `R-MFG-011`).

**One-line meaning (Proposal):** the *job list* is the connector's ordered list of jobs it holds; one job is current, the rest wait; only an owner reorders and starts.

## What changes in the demo

- Send to machine while a job runs or waits no longer answers "Connector busy": the job is added to the list. The Machine controls group (`0118`) shows the list under the state line: one row per job (name, lines, size, who sent it), the current job first.
- An owner can drag a waiting job (or press up/down buttons) to change the order, remove a waiting job, and press **Start** on the first waiting job once the current one is done. A member sees the list and can add and remove its own jobs.
- Try it: connector with `--fake-controller --fake-lines-per-second`, two paired apps (`0128`), send three jobs, reorder, start.

## Questions

Frage 257 (QUEUE-MEMBER): What can a member do with the list? **Decided A by the customer (2026-11-18).**
- A: Add jobs and remove their own waiting jobs; only an owner reorders, removes others' jobs and starts (a member's Start still waits for an owner, `0128` A13).
- B: Members add only.
- C: Members add, remove and reorder any waiting job.

Frage 258 (QUEUE-RESTART): What survives a restart of the connector? **Decided C by the customer (2026-11-18).**
- A: Nothing: the list is in memory.
- B: Waiting jobs persist in the state directory (limits: 20 jobs, 256 MiB together, each 64 MiB as today; the sender gets "List full" beyond that); a job that was running at the restart shows "Interrupted at line <n>" and is never resumed or restarted by itself.
- C: As B, and the interrupted job is offered for restart from its line (needs `0118` run-from-line).

Frage 259 (QUEUE-NEXT): When a job completes, what starts next? **Decided B by the customer (2026-11-18).**
- A: Nothing: the next job waits for an owner's Start (the material is changed between jobs).
- B: The next job starts at once when an owner marked it "Run next automatically".
- C: Always the next at once.

## Acceptance criteria (Fragen 257 A, 258 C, 259 B, decided)

1. Given a connector holding a current job (any state from Receiving to Paused), when another client sends a job, then it is accepted into the list in order of arrival, the answer is `JobAccepted` with its position, and the current job is untouched; "Connector busy" no longer answers a send (only "List full" or the existing refusals).
2. Given waiting jobs, then `Status` (or a new `JobList` message, additive, protocol stays 0, golden file) carries the list: job id, name, lines, size, sender, position; the Machine controls group shows it with the same state words as `0064` Q13 to Q17.
3. Given an owner, when it moves a waiting job, then the order changes for all clients at once; the current job cannot be moved; a stale position answers `Error { WrongState }` and the list shown is refreshed.
4. Given the current job is Done or the list has no current job, when an owner presses Start, then the first waiting job becomes current and runs as `0064` Q14 (under `--require-local-start` as before); a member's Start waits for an owner (`0128` A13). Nothing starts by itself unless the job carries the mark of criterion 11 (Frage 259 B).
5. Given a member, then it adds jobs and removes its own waiting jobs, and gets `Error { OwnerRequired }` for move, others' removal and a confirm (Frage 257 A, one pure function table in core as `0128` A9).
6. Given a connector restart with three waiting jobs, then they are back in the same order with their sender names, a job that ran shows "Interrupted at line <n>" and cannot be started again by Start; it can only be restarted by the guarded restart of criterion 12, or removed or kept as history by an owner; a corrupt stored job is dropped with a line in `admin.log`, the rest stays; the 20 job / 256 MiB limit is tested (Frage 258 C).
7. Given `Stop`, then it stops only the current job (as `0127` G12) and the list keeps the rest; given Reset (`0128` A19), the list is deleted with the state directory.
8. Given the app's Output group (`0125` S13), then it still shows the connector's current job state; `0125` S8 is rewritten (see below). The page's "counts as Lasered" rule (S10, S11) stays at `JobAccepted`.
9. Tests without wall-clock sleeps (`CLAUDE.md` §5): list core in `curvyo-connector-core` (pure, table tests), integration with the fake controller (three jobs, reorder, start, restart), golden files of the new messages.
11. Run next automatically (Frage 259 B, Proposal for the details): every job has a flag "Run next automatically", default off, set by an owner (a member may set it only on its own job, an owner may change any). Given the current job ends Done and the first waiting job carries the flag, then it becomes current and starts without a press, under `--require-local-start` as before (the start at the machine stays required). Given the current job ends Failed, Stopped or in Alarm, or the connection to the controller is lost, then the chain is cancelled: no job starts after it, the flags of the waiting jobs are cleared and the list says so ("Chain stopped: <state>"); only an owner's Start continues. A job never starts automatically when the controller is not Idle or when a restart (criterion 12) is pending. The flag survives a connector restart but nothing starts by itself after a restart. Tests: table of end state x flag x `--require-local-start`; chain of three on the fake controller with a failure in the second. The connector does not know materials or fixtures; the owner decides by setting the flag.
12. Restart from the interrupted line (Frage 258 C; Proposal for the safety rules, flagged to the lead: needs the `0118` run-from-line and a customer look at the wording). The interrupted job offers **Restart from line <n>** to an owner only, never automatically, never for a member, never by Start. It is valid only when all of these hold, each refused with its cause in words:
    a. the machine position is trusted: the controller is homed since the interruption (the connector has seen `$H` succeed on this controller session) or reports the same work coordinate offset and the same machine position as stored at the interruption; GRBL reset, a power loss, an Alarm or a reconnect since the interruption make the position unknown and the restart is refused until the owner homes ("Home first");
    b. the owner confirms explicitly with the line named ("Restart <name> from line <n>?" with the confirm button carrying the number; armed press of `0081`, no default button);
    c. the laser is off and stays off until the first line of the job text: the connector sends `M5` first, and the start state of line n is reconstructed from the job text before it (the last modal values of distance mode `G90`/`G91`, units, plane, feed `F`, spindle `S`, motion mode and coolant `M7`/`M8`/`M9`, `M3`/`M4` is held back and re-sent after the move), never assumed;
    d. the move to the start of line n is a travel at a safe feed in absolute mode with the laser off, then the modal state is set, then `M3`/`M4` with the stored `S` resumes, then line n; if the job text before line n cannot be reconstructed (unknown modal word, G-code the connector does not parse), the restart is refused.
    Tests on the fake controller: refused after reset, power loss and Alarm; refused without confirmation; accepted after Home with the preamble checked line by line (laser off during the move); modal reconstruction table (G91, feed change, S change, M8 before line n).
13. `admin.log` records every restart from a line with the owner, job and line.
14. Gate green, `docs/design-system.md` row for the list.

## Changes accepted behaviour

- `0064` P13 and `0125` S8: "a new send while the connector holds a job that is not finished is refused with Connector busy" becomes "is added to the list". Reason: customer request of 2026-11-18. The held-job rule for Start/Stop stays.
- `0125` S19 (a cut transfer must not stay `Receiving`) is unchanged and tested again with a non-empty list.
- `0127` G12 and `0064` Q14: a job that was interrupted is no longer final; it can be restarted from its line by an owner under the guard of criterion 12 (Frage 258 C). Reason: customer, 2026-11-18.
- `0128` owner and member roles are extended by the list rights above; no change to pairing.

## Needed before Ready

`architect`: ADR for the list (storage in the state directory, message shape, limits; depends on ADR 0071); `ux-engineer`: ux-notes for the list in the Machine controls group (drag and buttons, 244 px column, keyboard).

## Out of scope

Resuming an interrupted job without the guard of criterion 12, starting automatically after a restart of the connector, a chain across Failed, Stopped or Alarm, scheduling at a time, several machines per connector, sending from the relay, editing a job on the connector, priority beyond order, history of finished jobs beyond `jobs.log`.

## Links

Requirements: R-MFG-LASER-002, R-MFG-011
Depends on: `specs/0128-connector-api-and-ctl/` (owner and member), `specs/0118-machine-control/` (where the list is shown; run-from-line for criterion 12), `specs/0127-connector-grbl-serial/`
Related: `specs/0125-send-to-machine/`, `specs/0064-machine-connector/`
PR: -
