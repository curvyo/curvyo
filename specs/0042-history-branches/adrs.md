# ADRs for "History branches"

Branches are not stored: they fall out of the `0020` stack machine run over the log. Preview reads
the live document under a short guarded checkout and keeps plain copies; Live and Clone are
restore-engine writes. **No new crate, no new
dependency, no new register, no `format_version` bump.** The persisted parts it relies on are
[ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) (accepted 2026-10-10). Builds
on `0020` and `0041` merged.

**Amended 2026-10-10 after the Loro spike** (`docs/spikes/loro-history-primitives.md`, PR #85):
decisions 6 and 7, milestone 2 and the risks changed; criterion 21 needs new figures from the PO.

## Depends on

- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1 (state at a version), §2 (header
  keys `v`, `o`; continuation), §3 (step ids), §6 (stack machine, branches, main-line order),
  §8 (previews from the live document, no fork), §12 (checkout guard), §13 (preview budget);
  questions 6 (go to version over other people's steps) and the
  Document-scope rule behind spec Q3 (`revert_to` reverts everybody, ADR 0014 option C).
- [ADR 0009 §1, §2, §4](../../docs/adr/0009-concurrent-editing-semantics.md): the log only grows;
  a preview is local view state; reads run against an immutable version.
- [`0020` adrs.md](../0020-undo-redo/adrs.md) decisions 2, 5, 9; [`0041`
  adrs.md](../0041-object-history/adrs.md) decisions 1 to 5.
- The blue-and-black preview rule of `specs/0009-unified-object-editing/` (criterion 17).

## Feature-local decisions (2026-10-10)

1. **Branches are derived** (1 to 6). The stack machine, fed one peer's steps in that peer's order
   (Document scope) or one object's timeline (Object scope), leaves the redo side's steps as a
   branch whenever a do step arrives; the fork point is the top of the undo side at that moment.
   Branch id = its newest step's id (3). Redo at a fork brings back the most recently left branch,
   which is what a plain stack does (2, spec Q1 A). Nothing about branches is saved (6).
2. **Peers** (5): a step of another peer is placed on the main line by (Lamport, peer) and never
   enters this peer's machine, so it cannot start a branch; each peer's own undo marks produce that
   peer's branches.
3. **Version of a step** = the frontiers of its last op ("right after step S"); before = the deps of
   its first change. Every row therefore has two exact versions, the same on every replica.
4. **Go to version** (10): read the object at the version and write it with no three-way check (a
   deliberate return, 15), header `go_to_version;s=<n>;o=<NodeId>;v=<StepRef>`. With ADR 0014 Q6 B
   it is refused when a later step on the object is by someone else.
5. **Exploring** (11 to 13) uses the continuation of ADR 0014 §2: each further press on the same
   object, with nothing committed in between, repeats the header with the new `v`. One step, one
   stack entry, one row; its before-version is that of the first press, so Ctrl+Z and Restore (12)
   are a plain undo of it. Selecting another object, switching tab or any other edit ends it,
   because the next commit has a new `s` (13).
6. **Preview** (17 to 21), *amended 2026-10-10, no fork*: `Document::preview_at(version, scope)`
   checks out the live document through the guard of ADR 0014 §12, copies the objects the scope
   needs (one object, or all for the Document scope) into a `HistoricalDocument` of plain
   `ObjectSnapshot`s plus the version, and returns to latest in the same call. The renderer draws
   the snapshots; there is no `LoroDoc` to edit, which is what "read-only" means here (`fork()`
   would hand back an editable document with a new peer). Each further row is a new call. The live
   document keeps committing and merging between calls; "N new steps since" compares its frontiers
   with the snapshot's version. Nothing is written or replicated (20). Rows more than 1,000 steps
   back from the current version show the long-operation state before the call.
7. **Clone version** (22 to 24): read the object at the version through the checkout guard (for a
   Delete row: at the step's before-version), create a new node with that state, fresh anchor ids from the caller's minter,
   and `clone_of = "<source>@<version>"` (`0041` decision 5), in one step
   `clone_version;s=<n>;o=<source>;v=<StepRef>`. Inherited rows follow the causal past of that
   version, so a branch's clone inherits that branch's path and not the others (23). Groups wait
   (25, spec Q5 A).
8. **Lanes** (8, 9) are presentation: `ui-core/src/history_lanes.rs`, a pure function from rows,
   statuses and fork points to lane indices (main line plus three, the rest folded).

## Placement

`curvyo-document-core/src/history/`: branch output of `stack_machine.rs`, `preview.rs`
(`HistoricalDocument`), go-to and clone writes in `restore.rs`; `curvyo-render-core`: drawing a
`HistoricalDocument` object as the blue preview; `curvyo-ui-core`: `history_lanes.rs`, preview chip
and banner texts; `curvyo-editor-wasm`: preview and Live state in `session/history.rs`; frontend:
lanes, switch, Restore, Clone buttons.

## Milestones (one branch `story/history-branches`, one PR)

1. Branch derivation and row states, lanes and keyboard (1 to 9). Architect review of the branch
   diff (public history API).
2. Preview, object and document (14, 16 to 21). Measure 21 here at 10,000 steps: rows 10, 100,
   1,000 and 10,000 back, warm and right after Open, once on wasm.
3. Live: go to version, exploring, Restore (10 to 13, 15).
4. Clone version, also from the Document scope and of deleted objects (22 to 27).

## What can start now and what waits

Nothing before `0041` merges. No milestone waits for a customer answer (accepted 2026-10-10).
Milestone 3 runs on the spec's Q2 A (Preview default, Live by switch) and ADR 0014 Q6 A. The Document scope gets no Live
mode (spec Q3 A) because a document-wide go-to would be `revert_to`, which reverts every peer.

## Risks

- Criterion 21 as written (any row within 300 ms at 10,000 steps) does not hold: the spike
  measured 125 to 250 ms for a row 1,000 steps back but 0.3 to 0.85 s at 5,000 and up to 2.6 s at
  20,000, and 0.5 to 1.1 s for any preview from a fork. Decided (ADR 0014 §13): 300 ms for rows up
  to 1,000 steps back; further back the long-operation state and 1.5 s at 10,000 steps back.
  **The PO rewords criterion 21** to these figures.
- The first checkout after Open builds Loro's history cache (about 260 ms, +48 MB at 154,000 ops).
  It is paid by the first preview behind the busy state and freed when the History tab closes.
- Up and Down through far rows re-checks out per row. If milestone 2 measures it too slow, the
  `HistoricalDocument` of the previous rows is kept (snapshots are small) instead of re-reading.
- Lamport order for concurrent steps can differ from wall-clock order; the list shows times, so a
  row may look out of order by a few seconds. Accepted: the same order on every replica matters
  more.

## Notes

**2026-10-12, architect review of milestone 1** (`58914d3`):

- **Peers in the Object scope.** Decision 2 holds for the Document scope only. The Object scope
  runs the one machine of `0041` over the object's timeline, every author's steps included (ADR
  0014 §6 "per object", `0041` criterion 11 "a new do step on the same object (by anyone)", this
  spec's criterion 28). So a peer's step on an object does end a pending object redo and leaves the
  object-undone step on a branch of that object's list; a row reading "Undone" there would promise
  a Ctrl+Shift+U that `0041` criterion 11 already refuses. Steps taken back with Ctrl+Z keep the
  author's own machine in both scopes, so a peer never branches someone else's Ctrl+Z. No ADR
  change. Criterion 5 is read as "in the Document scope"; the product owner adds that qualifier
  and one sentence for the Object scope.
- **Placement.** Go to version and clone version get their own module
  `curvyo-document-core/src/history/version_write.rs` instead of `restore.rs` (464 lines), with
  one responsibility: write an object's state at a past version as a new step. It reuses the
  guarded checkout read (§12), `revive.rs` (§11) and the restore write path; it adds no second
  write engine (ADR 0014 §1: one read, different write rules).
- **Branch marks are not stored.** `BranchMark` and `HistoryBranch` are derived per call;
  `format_version` and the saved file are unchanged (criterion 33).

**2026-10-12, architect review of milestones 2 to 4** (`6571ba9..d224ab9`):

- **Exploration continuation.** A further go-to press repeats the header with a new `v` (same
  label, `s` and `o`); `step_header::continues` joins it in the step index and in `find_step`, and
  the step names its latest version. This is what decision 5 says; ADR 0014 §2 ("repeats the header
  verbatim") gets a one-line amendment for this one exception. An older build reads each press as
  its own step with an unknown label: a longer list there, never a wrong drawing.
- **One write engine.** `version_write.rs` reads through the checkout guard (`Detached`) and
  writes through `merge::restore_map`, the restore write path, with "now" as the from-side so every
  field takes the version's value. A clone creates a new node, so `revive.rs` has nothing to do; a
  go to on a deleted object is refused. The condition of the milestone 1 note is met. A go to sets
  the object's meta map only; its tree parent and z-position stay where they are now.
- **Inherited rows along the path.** Inherited rows leave out steps their author had taken back by
  the copy's version (`StepIndex::machine_within`), for a clone of a version and for `0041`'s
  Duplicate alike: one lineage rule, as criterion 23 asks. For Duplicate this hides rows that
  `0041` criterion 24 listed; it goes to the customer with the demo (default: keep the new rule).
- **No format change.** Go to and clone are ordinary commits with headers; clone writes the
  existing `clone_of`; `format_version` is unchanged.

**2026-10-14, customer feedback 2026-10-14** (PR #119; ADR 0014 §8 amended the same day):

- **(a) Apply.** Enter or a double-click on a row applies its version `V`, in the Document scope
  to the whole document and in the Object scope to the selected object; there is no Live mode and
  no Mode switch. An apply is one run of the existing restore engine (`restore_plan.rs`,
  `restore.rs`, `revive.rs`) with before = `V`, after = now, over the objects touched by any step
  not in `V`'s past (one object in the Object scope), committed as one step
  `go_to_version;s=;t=;v=[;o=]` (no `o` for the document) that Ctrl+Z takes back. Since after =
  now, every field takes `V`'s value: other peers' later steps are overwritten (ADR 0014 Q6 A),
  and the notice counts them as in criterion 15. Objects created since `V` are removed, objects
  deleted since `V` are revived with their `NodeId`, tree parent, order and the layer and group
  flags of `0050` (ADR 0015) go back to `V`. Root keys go back except `history_wiped`; `clone_of`
  and `history_floor` are not touched. The apply is an ordinary do step: rows after `V` keep their
  state, the new row reads "Go to version a1b2c3d", and a later Ctrl+Z, new step and redo follow
  the stack machine as for any step. Long-operation state as for undo: `V` more than
  `FAR_STEP_OPS` operations behind, or more than `SMALL_STEP_OPS` operations to write; budgets of
  ADR 0014 §13 for large steps. `go_to_version` in `version_write.rs` is replaced by this run; Clone
  version stays there.
- **(b) Exploration is dead.** Every apply is its own step. The `continues` exception, its uses in
  `step_index.rs` and `step_log.rs`, and the ADR 0014 §2 amendment of 2026-10-12 are removed (the
  amendment never reached `main`). This supersedes decision 5 and the "Exploration continuation"
  bullet of the 2026-10-12 note. Decision 4's header loses nothing; `o` becomes optional.
- **(c) Stored form.** Unchanged: ordinary commits with a header, revives through the existing
  injected move, no new key, no `format_version` bump.
- **(d) Dead code to delete.** Core: `step_header::continues` (back to plain message equality),
  `Document::last_step_if_current`, `Document::commit_joining`, `GoToReport::continued`, the
  `explore` parameter and the go-to body of `version_write.rs`. editor-wasm: `VersionState` and
  exploring, the Restore path, the Mode flag in `PreviewState` and the Mode getters and setters in
  `wasm_history.rs`, a single-press go to. ui-core: `RESTORED_NOTICE` and any Mode text. Frontend:
  `HistoryModeRow.tsx`, `modeCaption` and the Mode tooltip in `lib/historyPreview.ts`, the Restore
  button and the caption slot, `SAVED_IN_FILE_LINE` in `lib/historyRows.ts` and its row. The tests
  of all of these go with them. The PO rewrites criteria 10 to 14, 17 and 19 and spec Q2 and Q3.
- **Object preview** draws only the previewed version: the session leaves the live object out of
  the draw list while the preview lasts (render and session only, no API in core).
