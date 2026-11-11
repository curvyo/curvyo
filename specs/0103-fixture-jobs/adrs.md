# 0103 fixture jobs: architecture notes

Status: Proposed (architect, 2026-11-11). Assumes Frage 176 A, 177 B and 178 B (the defaults, unanswered). Decisions marked **(Proposal)** are the architect's and vetoable at the demo. The file shape is [ADR 0052](../../docs/adr/0052-laser-fixture-jobs-in-the-project-file.md) (`needs-customer`, accepted with the story PR).

Depends on and extends:
- [ADR 0046](../../docs/adr/0046-laser-operations-record.md): the operation node, its registers and the Unknown rule (reused by shape, not by parent); its `member.` keys are **not** used (decision 1).
- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md): the fixture node and its slot children (a fixture may hold job nodes beside its slots; §2, §3, §7 get notes on acceptance).
- [ADR 0050](../../docs/adr/0050-laser-fixture-set-in-machine-profile.md): the profile's fixture set (record format 3 with `jobs`; §1, §2, §8 get notes).
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3, §4, §7: opaque kinds, unknown children, the bump rule.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) skip predicate: a job is covered at depth 2 under the manufacturing (decision 2).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) (one history step per commit; undo revives the tree): what keeps the slot sweep and the job deletes in one undo step.
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) and `0106` (`step_scope`, decision 7).

## 1. Storage: a job is a child of its fixture, its ticks are keys on the job

A job is a node, `shape = "fixture_job"`, a child of its fixture node. Its registers are `name`, `op_kind`, `op_show` and `op_output`, the four of `0101` (`op_show`, `op_output` as booleans). Its ticks are `target.<part key>` keys with value `true`. The full rules are in ADR 0052 §1 to §4.

Why a child of the fixture (not of the manufacturing, not registers keyed by the fixture): the tree delete of `delete_fixture` removes the jobs and their ticks in one step, undo revives them, and the order among a fixture's jobs is the child order, as for slots (ADR 0049 §3). The two other shapes need a sweep on fixture delete or a second order (ADR 0052 options B and C).

Why `target.` and not `member.`: the work reader, the work sweep of ADR 0046 §3 and the nearest rule all scan `member.` keys of the operations. Criterion 10 needs fixture parts and work items never to share a key or a row, so the prefix separates them by construction.

Part keys (ADR 0052 §2): a whole node part (the holder's board, a slot, a clamp's outline, a stop) is the node's id; a grid's lines and tapes are the grid's id with `.lines` and `.tapes`; a screw's head and thread are its id with `.head` and `.thread`. The Targets list and the run order follow the fixture's part order, not the key order.

**Registers and defaults (Proposal, differs from `0101`):**
- `op_output` absent reads **off** for a job (ADR 0046 reads absent as on). The writer always stores `false` on a new job, so absence only happens in a damaged or hand-made file, and it must never run.
- There is no `op_margin`. A fixture job row has no settings field (customer, 2026-11-10), so a Cut-out job runs with margin 0. The lead's brief listed `op_margin` for reuse; this is the one register not reused, and the spec's criterion 19 needs the note (§9, S5).

**Reads** (`curvyo-document-core/src/fixture_job_codec.rs`): a job snapshot carries `kind` (`None` when absent or not a string), `show`, `output`, its live target keys, and a damage count. A key whose node is not the job's fixture or one of its readable slot children is left out and is not damage (merge leftovers, criterion 12). A key whose suffix is not a part of the kind, or whose value is not `true`, is kept and counted, then ignored by the run rule. Reads never write.

**Writes** (`curvyo-document-core/src/fixture_job.rs`): ten commands, each one commit (labels in §7). Each checks that the node is a `fixture_job` under a fixture; a work operation id, a slot id or a fixture id is refused, so the job commands cannot touch the work (criteria 9, 10). `remove_slot` deletes, in the same commit, the `target.<slot id>` keys of the holder's jobs (the only sweep the feature adds; at most the holder's jobs are scanned). Moving, resizing or renaming a slot writes nothing (criterion 11).

## 2. `format_version` 41, and the reads that must change

A `fixture_job` child is a new node shape under a fixture. Build 40 refuses a fixture with a non-slot child (`child_is_valid` in `manufacturing_codec.rs`, and the format 40 note in `document.rs`), so without a bump a build of 40 would refuse a job as damage. The bump makes it a version refusal (ADR 0033 §7). Migration: none. Goldens: `fixture_jobs_v41.curvyo`, `fixture_jobs_unknown_kind_v41.curvyo`, `fixture_jobs_damaged_v41.curvyo` (ADR 0052 §6); the v40 and v39 goldens stay and must open unchanged. `CURRENT_FORMAT_VERSION` becomes 41, with a new note in its doc block. Next free number: no remote branch claims 41 (checked 2026-11-11; `story/pen-modes-variable-stroke` is at 40 with no bump); ADR 0033 §7 says the number is taken at merge.

Code points the implementer must not miss (each one is a silent bug otherwise):
- `read_fixture` (`manufacturing_fixtures.rs`) counts every non-slot child as one damaged slot. A job child must be skipped, not counted, or every holder with a job reports damage (criterion 33 notice).
- `child_is_valid` / `slot_is_valid` (`manufacturing_codec.rs`) accept a `fixture_job` child with no children and readable registers; any other child is still refused.
- `in_manufacturing_subtree` (`manufacturing_codec.rs`) covers jobs by depth; its doc comment names jobs now (text only).
- `remove_slot` sweeps the job ticks; `delete_fixture` needs nothing.
- Slot commands already check the shape (`slot_meta`); `fixture_meta` checks the parent is the manufacturing, so a job id is refused by both. Job commands check the mirror image.

## 3. The profile's fixture set: record format 3

`format_version` 3 adds `jobs` to each fixture entry (ADR 0052 §7). Reads: format 1 has no set, format 2 has fixtures without jobs (a `jobs` key there is damage), format 3 requires `jobs`. Writes are always format 3 and never happen on read. Part tokens in the record are `board`, `clamp`, `stop`, `grid.lines`, `grid.tapes`, `screw.head`, `screw.thread` and `slot:<n>` (0-based, among the entry's slots); the grammar and the checks are in `curvyo-laser-core` (`machine_profile_fixtures.rs` and `fixture_part.rs`), the node ids are never in the record.

- **Output is never stored.** The entry has no `output` key (a key written by hand is an unknown key and damages the record). Pick profile and Reset from profile write every job with `op_output` = `false` (criterion 23, Proposal).
- **Copy.** `FixtureRecord` gains `jobs`. The copy maps a token to the new fixture's slot node by index, in the same commit as the fixtures (labels `pick_profile`, `reset_profile`, unchanged). A token that has no counterpart cannot occur in a readable record (it damages the record), so the copy never drops a target: the notice of criterion 23 for drops is removed (§9, S6).
- **Differs from profile** compares jobs in order: kind, name, Show and the set of target tokens. Output is not compared.
- **Limits.** 100 jobs and 200 targets per set; the record text limit goes from 64 KiB to 128 KiB (a maximal set in the pretty form is above 64 KiB). A golden at the maximum must read and fit.

## 4. Run order and the count: one pure function in `curvyo-laser-core`

Frage 176 A: all running fixture jobs run before the work of the page. The rule is one function in `curvyo-laser-core/src/operation_sequence.rs` (extended; the module's sentence becomes "The run order of the laser's runs: the fixture block, then the operations under the Cuts last switch"):

```rust
pub struct FixtureJobRead {
    pub kind: KindRead,          // ADR 0046 §1; Unknown runs nothing
    pub output: bool,            // op_output, never on by default
    pub fixture_active: bool,    // 0102 criterion 5
    pub parts: Vec<FixturePart>, // ticked parts that read, in part order
}
/// Positions (in the given order) of the jobs that run, in run order.
pub fn fixture_block(jobs: &[FixtureJobRead], cuts_last: bool) -> Vec<usize>;
```

- **Running:** `fixture_active && output && kind known && !parts.is_empty()` (criterion 13, the eye is view only).
- **Order:** the input is fixture creation order, then job order (criterion 3). Among the running jobs, `run_sequence` (the same function as the work) applies with `cuts_last`: with Cuts last on, Marking jobs run before Cutting jobs, else list order (Proposal: "Marking before Cutting as on the work", criterion 14).
- **Geometry per part:** `kind_run(kind, part_shape(part))` from `operation_kind_run.rs`, with the part shapes of the new `fixture_part.rs`: closed outlines for the board, a slot, a clamp, a stop and the screw's two circles; open lines for the grid's lines and tapes. Cut-out per part with margin 0 (§1).
- **The count** is `fixture_block(..).len()`: jobs, not parts. "1 fixture job runs." and "<F> fixture jobs run." come from this number; the work `Summary` (`operation_summary.rs`) is leaf-based and does not change.
- **The summary line rule** (UX J7, accepted here): the work line "Runs N operations" is left out when no work operation runs; "Nothing runs." shows only when neither the work nor the block runs. The rule lives in the view (`operations/view_json.rs`), not in the core.
- **Frage 176 B or C** would change one call (B) or add one per-job field (C). Neither is built.
- **`fixture_part.rs`** is a new module: which parts a kind has (in Targets-list order, from the fixture's kind and its readable slots), each part's shape, and the grammar of the keys (`from_key(kind, slot index or whole, suffix)`) and of the profile tokens. `fixture_kind.rs` is 382 lines; the parts are a separate responsibility.

## 5. "Output turns off after the page is output" (Frage 178 B)

What exists in the code for "output" today:
- The **Output switch** of an operation (`op_output`, `0101`): "what runs". It is a per-row switch with one undo step ("Output on or off"). Nothing reads it to output anything.
- The **output page** (ADR 0042): a page owned by a manufacturing, a drawing page holding placements and fixtures. "Output" here means the page, not an act.
- **SVG export** (`curvyo-editor-wasm/src/session/svg_export.rs`): the drawing's export. It is not machine output and excludes fixtures (`0102` criterion 7).
- No G-code, no machine command, no command that "outputs" a page. `specs/0028` and `specs/0029` are not written yet.

So in 0103 the state change does not exist yet, because its trigger does not exist. What 0103 implements:
- The **Output switch of a fixture job**: `set_fixture_job_output_on` / `_off`, one undo step each (criteria 4, 5). Nothing switches a job off by itself.
- Nothing else. No command, no rule, no notice for criterion 6 (§9, S2).

The contract for `0029` (written here so the step is built once):
- The page output is **one command, one commit** in `curvyo-document-core`. Its commit writes the output record and sets `op_output` = `false` on every job whose `op_output` is `true` (`fixture_job` nodes under the manufacturing's fixtures). One undo step restores both; a separate step is not allowed (a second step would let an undo keep the record and drop the switch-off, which is the failure criterion 6 prevents).
- The rule is "every job with Output on": no kind, no part, no fixture state changes it (a job of an inactive fixture is switched off too, so that a later reactivation does not run it).
- The notice of UX J8 ("Output turned off on N fixture jobs.") belongs to the same command.
- A session command "turn off after output" is rejected: it would be a second step and a second place to forget.

## 6. The Operations tab: one feature module, its rows, its commands, its view

**Owner.** The jobs are feature-owned by `curvyo-editor-wasm/src/session/fixtures/jobs/` (new directory; each file under 500 lines): `mod.rs` (the methods the other session modules call), `commands.rs` (the ten commands), `view_json.rs` (the section's serde types), `parts.rs` (the Parts list and the look of a part), `tests.rs`. A job belongs to its fixture, so the fixtures module owns it; the Operations tab only embeds the view.

**Reuse from `0101` (`session/operations/`):**
- Kind words and glyphs: `look.rs` (`glyph_of`, the label of `OperationKind`) become `pub(in crate::session)`.
- The name of a new job: `next_operation_name` (`curvyo-laser-core/src/operation_naming.rs`), given the names of the page's jobs of that kind. This gives "<Kind> <N>" with the lowest free N per kind on the page (criterion 2).
- The run and section rules: `run_sequence`, `kind_run`, `KindRead` (laser core).
- The armed trash and the rename field: frontend, as `0101` does (`OperationsSection.tsx`); no wasm state.

**Not reused, on purpose:**
- The `0101` commands (`operation_meta`, `set_operation_switch`, `move_operation`): they require a manufacturing parent, so they refuse a job node. That refusal is what keeps the two sets apart (criterion 9).
- The `0101` row type (`RowView`): its margin and its settings do not apply to a job.

**One state field.** `FixtureState` gains `job: Option<NodeId>`: the job whose Parts list is open (session view state, not saved). The notice of a job command is the existing `FixtureState.notice`.

**Reads.** `Document::fixture_jobs(manufacturing)` walks the manufacturing's fixtures and their job children, once per view build. No cache: at most a few hundred nodes, and a cache would need an invalidation on every job write. If the budget of §8 fails, the cache is keyed by the document version, not by the job list.

**View contract** (`OperationsView` gains `fixture_jobs`, built only in `fixtures/jobs/view_json.rs`):

```text
fixture_jobs: {
  count: N,                              // jobs in the section (header "Fixture jobs (N)")
  groups: [ { fixture: id, name, glyph,  // creation order of the Fixtures block
              jobs: [ { id, name,
                        kind: { text, glyph, word } | null,   // null: Unknown
                        show, output,
                        fixture_active,  // false: "not on the bed"
                        runs,            // the fixture_block answer
                        parts_ticked, parts_total,
                        position, of } ] } ],   // for the arrows; of = jobs in the group
  parts: null | { job: id, rows: [ { token, name, ticked, select: id,
                                     state: "running" | "output_off" | "show_off"
                                          | "not_on_bed" | "unticked",
                                     kind: text | null } ] },
  running: N,                          // jobs that run: the summary's "1 fixture job runs."
  running_fixtures: [id],              // their fixtures, for the summary's count button
  damaged: N,                          // damaged job values, all fixtures (criterion 22)
  damage_notices: [text]               // "<n> values damaged on <name> job.", one per fixture
}
```

- `state` carries the look of a part (criterion 18, UX section 6): the canvas reads `state` and `kind`, and does not compute the rule itself.
- The part `kind` is the job's kind text for a running part and `null` otherwise.
- **Shared fixtures.** Three files in `curvyo-editor-wasm/tests/fixtures/fixture_jobs_view/`: `empty.json` (no job, the empty line), `two_fixtures.json` (a holder with a running job and a ticked slot, a grid job with Output off, an inactive holder), `unknown_and_show_off.json` (an Unknown job, a part of a job with Show off). `tests/fixture_jobs_view_contract.rs` builds the states and compares them; `CURVYO_UPDATE_GOLDEN=1` rewrites them. The frontend decoder (`frontend/src/lib/fixtureJobsView.ts`) reads the same files. A shape change updates the files in the same PR.

## 7. Edit context: the new labels

Every label of §1 and §5 is **free** (`FREE_STEPS`, `curvyo-ui-core/src/step_scope.rs`): the commands write only fixture job nodes under a fixture, never the selected objects, so a move-only selection may tick a part. This is the rule `0102` applied to its fixture commands ("they only write the manufacturing's fixture nodes"). The `0101` operation labels sit in `MOVE_STEPS`; both lists allow a move-only selection, so the choice changes no behaviour, only the label of the classification. Labels and undo names (`curvyo-ui-core/src/history_names.rs`, fixed per label):

| Label | Undo name |
|---|---|
| `add_fixture_job` | Add fixture job |
| `rename_fixture_job` | Rename fixture job |
| `delete_fixture_job` | Delete fixture job |
| `set_fixture_job_show_on` / `_off` | Show on / Show off |
| `set_fixture_job_output_on` / `_off` | Output on / Output off |
| `move_fixture_job` | Move fixture job |
| `assign_fixture_target` | Assign to fixture job |
| `remove_fixture_target` | Remove from fixture job |

The names carry no job name: `operation_name(label)` is fixed per label and no name-carrying label exists for `0101` either (§10, P1). The guard test of `step_scope.rs` (each label in exactly one list, each named) covers the new labels once they are added to both lists. `Remove fixture` (`delete_fixture`) already exists.

## 8. Budgets (criterion 26, thread CPU time)

Measured with `cpu_time_of` (`curvyo-editor-wasm/tests/cpu_time/mod.rs`) in a new `curvyo-editor-wasm/tests/fixture_jobs_budget.rs`, the shape of `operations_budget.rs`. Figures confirmed here:

- **(a) One tick:** one `assign_fixture_target` command plus the rebuilt Operations view on a page with 50 jobs and 200 ticked parts: **16 ms**. (The spec's "one press ticking 200 targets" describes no control; §9, S8.)
- **(b) Summary after any change:** the `fixture_block` answer and the work summary on a page with 50 jobs and 1,000 work items: **16 ms**. The test also asserts that a job command does not rebuild the output-page leaf cache (`0101` `index.rs`): a job write touches no drawing node.
- **(c) Section, first rows:** the Operations view with 50 jobs: **100 ms**. The DOM windowing (UX section 14) is outside these numbers.

## 9. Spec changes this build needs (for the product owner)

- **S1. Criteria 4 and 9, undo names.** "Assign to <job name>" and "Remove from <job name>" become "Assign to fixture job" and "Remove from fixture job" (fixed labels, §7). Show and Output keep the spec's words.
- **S2. Criterion 6 moves to `0029`.** Nothing in `0103` outputs a page (§5). Frage 178 B stays the target; its acceptance and its notice (UX J8) are `0029`'s. `0103` acceptance covers the Output switch only.
- **S3. Criteria 8 and 11, part keys.** The part keys are `target.<node id>`, with `.lines`, `.tapes`, `.head`, `.thread` for the grid and the screw; the parts are not "the grid's node" alone. The Changes table row for ADR 0046 §2 ("the same key form as ADR 0046 §2") is wrong: the prefix is `target.`, and the path form is not used.
- **S4. Criterion 14, run order.** "Marking runs before Cutting as on the work" becomes "with Cuts last on, Marking before Cutting as on the work; with it off, list order" (§4).
- **S5. Criterion 19, margin.** A Cut-out fixture job runs with margin 0: no margin field, no `op_margin` (§1). Add one sentence.
- **S6. Criteria 23 and 24, the profile.** The profile holds no Output (not "never stored as on"). A token that names no part of the copied fixture cannot be read, so the record is damaged and nothing is dropped; the "dropped with one notice" sentence goes.
- **S7. Criterion 24, format 2.** A `jobs` key in a format 2 entry damages the record; a format 3 entry requires `jobs`.
- **S8. Criterion 26 (a).** "One press ticking 200 targets" becomes "one tick on a page with 50 jobs and 200 ticked parts" (§8).
- **S9. Format section.** Build 40 refuses a file with a job (not "keeps or drops by accident"). Limits: 100 jobs and 200 targets per set; record text limit 128 KiB. `newer_format.json` becomes format 4; `format2_fixtures.json` writes format 3.
- **S10. Criterion 22.** Consistent as written (Output absent and damaged both read off, Show absent and damaged both read on). The note for the reader: absent Output reads off for jobs, unlike `0101`'s operations.

## 10. Proposals and conflicts for the lead (decide before the build)

- **P1. Undo names (UX J9 against criterion 4).** UX asks for "Hide Cut 1" and "Turn off Output of Cut 1". The history names are fixed per label (`operation_name`, `history_names.rs`); no name-carrying label exists for `0101` either. Recommendation: the fixed names of §7 (the spec's words). A name-carrying label is a new mechanism for both features and belongs to a later story.
- **P2. One word for the list of parts.** The spec says "Targets list" and "target"; UX says "Parts list". Recommendation: the UI says "Parts", the glossary keeps "target" for the stored tick.
- **P3. No margin on jobs.** The lead's brief listed `op_margin` for reuse. Not taken (§1, S5), because a settings field is out of scope for the row.
- **P4. Hidden buttons (UX J3) against criterion 2 (disabled buttons).** A house-rule question for the product owner; no architecture consequence.
- **P5. "Never dimmed as Not assigned" (criterion 18) against UX J5.** The view carries `state` per part (§6), so either look is a view decision, not a core change.
- **P6. Stale text found on the way (not this feature).** `curvyo-document-core/src/operation.rs` keeps a doc comment for the removed colour ("Sets the colour text...") and a label doc for "colour pick", both dead after `op_color` was dropped; ADR 0051 says "stays 39" while main is at 40. Both are small fixes for a chore branch, not for `0103`.
