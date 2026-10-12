# Plan for History branches (0042)

One branch `story/history-branches`, one PR, opened when the maker can try the slice end to end
in the app (fork and branch rows with lanes, Preview and Live on an object, the document preview
with its banner, Restore previous state, Clone of any version from either scope) and the full
local gate is green (`CLAUDE.md` §7, the frontend checks, the history budget tests). The
milestones below are commits on the branch, each green, pushed after each; none is a PR.

## Affected crates/modules

- `curvyo-document-core` (`src/history/`):
  - `stack_machine.rs`: the machine also records fork points and branches (`Status::OnBranch`
    exists; it gains the fork point and the branch a dropped step belongs to).
  - new `branches.rs`: branches of one scope derived from a machine run (id = newest step, fork
    point, steps in order, nested forks), the "N steps" count; pure, no Loro.
  - `rows.rs`, `object_rows.rs`, `object_timeline.rs`: rows carry the branch they are on.
  - new `preview.rs`: `Document::preview_at(version, scope)` through the `checkout.rs` guard,
    returning a `HistoricalDocument` of plain `ObjectSnapshot`s plus its version.
  - new `version_write.rs`: go to version (with exploration as a header continuation) and clone
    version (`clone_of`, Delete rows read at the before-version). `adrs.md` places these in
    `restore.rs`; that module is at 464 lines, so they get their own module (a module split
    inside the crate, no API difference).
  - `step_header.rs`: labels `go_to_version`, `clone_version` and key `v`.
  - `mod.rs`: additive `mod` and `pub use` lines only.
- `curvyo-render-core`: new `history_preview.rs`: an object snapshot as the blue preview outline
  with the 40 % ghost of its fill and stroke (reuses `live_preview.rs` styling, theme tokens
  `--preview-new`, `--preview-ghost`); the document preview draws snapshots plain.
- `curvyo-ui-core`: new `history_lanes.rs` (rows, statuses, fork points to lane indices and
  per-row gutter segments, main lane plus three, the rest folded); new `history_branch_text.rs`
  (branch header, accessible lane name, readout, banner, notices, Mode captions and tooltips).
  `history_notice.rs` is near 500 lines and stays as it is. `lib.rs`: additive lines.
- `curvyo-editor-wasm`: new `session/history_preview.rs` (preview state, Mode switch per session,
  end-of-preview rules, "N new steps since"), new `session/history_version.rs` (go to, exploration,
  Restore, clone, the gate of criteria 16 and 26); `session/history_rows.rs` and
  `session/object_history_rows.rs` (branch fields, lanes); `session/keys.rs` (one additive check:
  editing keys are ignored during a document preview); `wasm_history.rs` (new entry points);
  `session/mod.rs`: additive lines.
- `frontend`: `components/panel/HistoryTab.tsx` (wiring only), new `HistoryLanes.tsx` (per-row
  gutter SVG), new `HistoryModeRow.tsx` (Mode switch and caption slot with Restore), new
  `HistoryPreviewBanner.tsx` (banner, frame), `lib/historyRows.ts`, `hooks/useHistoryList.ts`
  (header rows, folding, Document-scope disclosure), the canvas component (frame, arrow pointer,
  inert tool rail during a document preview).
- Not touched (0050 runs in parallel): the sidebar strip, the panel registry, the tool enum, the
  key table entries. Shared files get additive lines only.
- Docs: this feature's `specification.md` (status and PR link at the end), `docs/technical-debt.md`
  if a budget of criterion 21 needs the cached-snapshot fallback; `docs/design-system.md` only if a
  token is added.

## Format

**No change.** `format_version` stays as it is (criterion 33, ADR 0014 Q2 A). Branches, previews
and the Mode switch store nothing. Go to and clone version are ordinary commits with a header
(`go_to_version;...;o=;v=`, `clone_version;...;o=;v=`); clone writes the existing optional key
`clone_of` of `0041`. No migration. No new golden fixture; a round-trip test (Save, Open) asserts
equal branches, ids, states and the unchanged `format_version`, and an older-build test reads a
file with the two new labels as legacy-labelled steps.

## Tasks

### Milestone 1: branches, row states, lanes, keyboard
- [x] 1.1 Test first: the stack machine reports fork point and branch for each dropped step;
  redo at a fork returns the most recently left branch (AC 1, 2, 28).
- [x] 1.2 `branches.rs`: branch id, fork point, step count, nested forks; one peer's machine per
  Document scope, one object's timeline per Object scope; peers' steps never start a branch;
  main-line order by (Lamport, peer) (AC 3, 5, 28, 29).
- [x] 1.3 Rows carry status and branch in both scopes; wiped steps and their branches leave the
  list (AC 4, 7).
- [x] 1.4 Save and Open give the same branches; two-replica test (AC 5, 6, 29).
- [x] 1.5 `history_lanes.rs`: lane per row, fork connectors, header rows, fold of the fourth and
  later branches; texts and accessible names in `history_branch_text.rs` (AC 3, 4, 8, 9).
- [x] 1.6 wasm rows with branch and lane fields; frontend lanes, header rows, folding, the word
  "Branch", Enter folds (AC 4, 8, 9, 27). Left/Right into the row action moves to 4.4: the row
  actions (Clone, the objects disclosure) arrive with milestone 4.
- [x] 1.7 Budget: 50 branches, 10,000 steps, list opens within 200 ms (AC 8).
  Commit `feat(document-core): derive history branches`; architect review of the branch diff
  (public history API).

### Milestone 2: Preview, object and document
- [ ] 2.1 Test first: `preview_at` returns plain snapshots, attaches again on error, writes and
  replicates nothing; a peer's step arriving meanwhile is merged, the snapshot is unchanged and the
  "since" count grows (AC 20, 30, 32).
- [ ] 2.2 render-core: blue outline plus 40 % ghost for an object snapshot; plain drawing of a
  document snapshot (AC 17, 19).
- [ ] 2.3 Session preview state: Mode switch (default Preview, kept for the session, Object scope
  only), every end-of-preview rule, gate of criterion 16, editing keys ignored in a document
  preview (AC 14, 16, 18, 19).
- [ ] 2.4 Frontend: Mode row and caption slot, readout in the notice slot (`role="status"`),
  banner with Back to live and "(N new steps since)", inset frame, inert rail, arrow pointer;
  preview by press, Enter or Space, Up/Down move it only while one shows (AC 14, 17, 18, 19, 21).
- [ ] 2.5 Budgets (release, assert 4x): rows 10, 100, 1,000 back within 300 ms; long-operation
  state beyond 1,000 and 1.5 s at 10,000 back; warm and right after Open; once on wasm (AC 21).

### Milestone 3: Live, exploring, Restore
- [ ] 3.1 Test first: go to version writes one step with header `o`, `v`; no write when equal;
  works on main line and branch; a later step forks (AC 10, 31).
- [ ] 3.2 Exploration as header continuation: five presses, one stack entry, one row, Ctrl+Z to
  the state before the first press; ends on selection change, tab change or any edit (AC 11, 13,
  31).
- [ ] 3.3 Over a peer's later steps: written anyway, notice names authors and count (AC 15).
- [ ] 3.4 Restore previous state in the caption slot, only while the exploration is top of stack
  and the object selected, never takes focus; notice and Undone mark (AC 12, 14).

### Milestone 4: Clone version
- [ ] 4.1 Test first: clone of a version on the main line and on a branch equals that version,
  new id, on top, selected, one step, Ctrl+Z removes it, original unchanged (AC 22, 31, 33).
- [ ] 4.2 The clone's timeline: "Clone of ... at version ...", inherited rows along the path to
  that version only (AC 23).
- [ ] 4.3 Document scope disclosure: touched objects (20, then "and N more"), one open at a time;
  clone of a deleted object reads the before-version; draw, resize, delete, Save, Open, clone
  test (AC 24).
- [ ] 4.4 No Clone for groups; gate of criterion 26; ids selectable in rows; Left and Right move
  focus into and out of the row action (AC 9, 25, 26, 27).

### Milestone 5: slice check and PR
- [ ] 5.1 Full gate (§7) plus frontend checks; browser walk-through of every criterion in WebKitGTK
  MiniBrowser under Xvfb; `/no-slop` on user-facing text.
- [ ] 5.2 PR per `CLAUDE.md` §9: What changes in the demo, Review guide, no new dependencies.

## Notes from the build

- Milestone 1 (2026-10-12): the Document scope runs one stack machine per peer, so a peer's step
  never starts a branch there (AC 5). The Object scope keeps the one machine of `0041` over the
  object's timeline: an object undo followed by anybody's step on the object leaves the undone step
  on a branch, because that is also when Ctrl+Shift+U stops being able to bring it back; a row
  that read "Undone" there would promise a redo that does not exist (AC 4). Ctrl+Z take-backs in
  the Object scope follow the author's own machine.
- A branch whose fork point is not in the list (it forked before the first listed step, or the
  fork point was wiped) is named "Branch from the start".
- Three tester assertions written before `0042` expected the placeholder "Undone" for a step left
  by a newer step (`acceptance_0041_tester` c11, `acceptance_0020_tester` c22,
  `acceptance_0020_tester_more` c55's model). They now expect the branch state of AC 1 and 4; the
  intent of each test is unchanged.
- The "N more branches" row stays in the list while unfolded (with `aria-expanded`), so it can be
  folded again; the revealed headers sit above their own newest rows. A branch revealed from it and
  unfolded takes a lane by the same rule, the rightmost one when all three are taken.
- Folding a branch also hides the branches that fork from it.

## Validation

- Core logic test-first as unit tests next to the code; story-level tests in
  `curvyo-document-core/tests/history_branches.rs` (fork, redo at fork, two replicas, Save and
  Open, wipe, go to, exploration, clone, clone from Delete after reopen), all comparing
  `document.json`.
- Budgets in a release-mode test file next to `tests/undo_budgets.rs`, asserting four times each
  figure of criteria 8 and 21; the wasm figure measured once by hand in the browser and written
  into the PR.
- Lanes and texts: pure tests in `curvyo-ui-core`; frontend unit tests for header folding,
  disclosure and keyboard.
- Manual end-to-end in the app under WebKitGTK MiniBrowser and Xvfb (no Playwright here).
- Every build and test command runs with `ulimit -v 8000000; export CARGO_BUILD_JOBS=4` and
  `cargo nextest run -j 4`.
