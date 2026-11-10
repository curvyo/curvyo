# ADRs for "Laser operations" (0101)

Architect, 2026-11-09, against the spec on `main` at `edc02815` (format 38). Updated 2026-11-09 for the customer's answers to Fragen 167 to 170 (167 B, 168 A, 169 A, 170 A). Updated 2026-11-10 after the customer's review of PR #305: the per-operation colour is dropped (decision 8, and ADR 0046 decision 1). One new ADR, [0046](../../docs/adr/0046-laser-operations-record.md) (Proposed, `needs-customer`: a node shape and format 39). No new crate, no new external dependency. One Cargo edge in `curvyo-laser-core`: `geometry-core` (already allowed by ADR 0034 §2; wasm32 like every `-core`).

## Depends on or extends

- [ADR 0046](../../docs/adr/0046-laser-operations-record.md) (new): operation nodes under the manufacturing, memberships as `member.` keys (node id, or path from a clone for a clone member), the silent drop of deleted objects' assignments (Frage 169 A), the copy and clone rules, the run rules in `curvyo-laser-core`, format 39.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §1, §2, §3, §7: amended by 0046 on acceptance. Operations are child nodes (`shape` tag); `cuts_last` is an `x.curvyo.laser.*` register (§3).
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) (its skip predicate `in_manufacturing_subtree`): unchanged; operation nodes are in the manufacturing subtree, so no drawing walk sees them.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §1 to §3: the operation rules live in `curvyo-laser-core` as plain functions; the one registration arm in `ui-core` is unchanged; no `Extension` trait.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §1, §3: the output page's kind is fixed; the operations belong to its manufacturing, not the page record.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §1, §2: a leaf's chain of ancestors and the effective hidden state; [ADR 0016](../../docs/adr/0016-clone-overrides.md) (a clone has its own placement, so the same member in two clones is two objects); [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md): a clone and a placed page are chain entries; a clone that stops being a clone is repaired as a copy.
- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) §1: the kind-aware membership read decides which objects belong to the output page; the assignment rule sits beside it, not in it.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3 (merge per key), [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) (undo of a deletion, revive), [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) (booleans, offset); the outset join of spec `0038` (Round corners).

## Replaces

Nothing. The epic (`specs/0063-manufacturing-workbenches/specification.md`, line 109) said the new ADR replaces ADR 0041. ADR 0041 is the membership lifecycle of `0097` (accepted 2026-11-05) and is not touched. The epic line now names ADR 0046, which amends ADR 0033 on acceptance.

The superseded spec `0027` cited its roles decision as `docs/adr/0041-laser-roles-keyed-by-layer.md`. That file is not in `main` and not in the closed branch's `docs/adr` tree; the number 0041 there clashes with the membership ADR, so the closed record is referred to by spec number only.

## Decisions

1. **Storage, keys, format, run rules: ADR 0046.** Operation = ordered child node of the manufacturing (`shape` `operation`). Membership = `member.<key>` on the operation: the node id for own objects, groups, placements and inner rows (by source object id); the path from the outermost clone for a clone or a clone member (`<clone id>/<member id>`). Cuts last = `x.curvyo.laser.cuts_last`. `format_version` 39. Goldens: `operations_v39.curvyo` (with clone member keys), `operations_unknown_kind_v39.curvyo`, `operations_dangling_v39.curvyo`, and `operations_unreadable_v39.curvyo` for criterion 26's unreadable record (the spec does not name the last two).

2. **The customer's answers, as built into 0046.**
   - **167 B (clone members take assignments).** A member of a clone is a read-only row in the Layers tree, in the original's layer and group order. Its assignment key is the path from the clone to the member. The bare member id would be the same key as the original's own assignment, so the path is required. Moving, restyling or regrouping the original's members keeps the key valid, because the path is made of ids. The clone itself still takes an assignment as a whole. Clone-of-clone members use the full path; the UI shows one clone level in this slice.
   - **Clone's original changes shape.** Edits that keep ids keep the keys. Deleting a node on a path (a member, a group of the original, the original itself) removes, in the same commit, every key whose path contains it; undo restores them. A clone that stops being a clone (repair as a copy, ADR 0045) loses its member keys in that commit. A key left by a merge whose path is not live is ignored on read and never repaired on read.
   - **168 A.** One Cut-out contour per target: the node or path where the nearest rule stopped.
   - **169 A (deleted object: silent drop).** A deletion removes, in the same commit, every key whose path contains the deleted node: own object, group, clone, clone member, placement, source object of an inner row (a placed page's deletion leaves the source's inner-row keys, because the source is not deleted). Undo of the deletion restores the nodes and the removed keys exactly (ADR 0014, one step). No "Missing item" row, count, or notice exists. Reads ignore keys whose path is not live. Criterion 27 is rewritten to this.
   - **170 A.** Assignment inside a placed page only in the Layers tree; the double-click of `0106` stays.
   - **149 A** (hidden drawing content, not a customer answer of this round but a rule of the spec): a hidden source leaf inside a placed page uses only its own key.

3. **Geometry rules, sharpened (no behaviour change; the PO adopts the wording).**
   - Criterion 21 says "each closed area runs as a filled area" and "an open path runs as a line", but does not say what a compound path with one closed and one open outline does under Engrave. Decision (carried from the closed `0027` criterion 8): a leaf with at least one closed outline is one filled area, its open outlines closed by a straight line (`0054`); a leaf whose outlines are all open is a line. Fill is by the leaf's own fill rule, whatever its fill switch.
   - Cut and Score take each outline as drawn; an open path stays open (criteria 19, 20).
   - Cut-out: the outer boundary only, from geometry-core `boolean` union of the item's closed outlines, grown by `op_margin` through geometry-core `offset` with the outset join of `0038`; zero skips the offset (criterion 23).
   - Images as in criteria 19 to 22; pixels are `0029`.
   - Style never enters the run (criterion 24).
   - Tolerances are passed in from the caller and come from the geometry-core tolerance type; no literal tolerance in `curvyo-laser-core`.

4. **Counting unit: output-page leaves.** Counts (criterion 17) and budgets (criterion 29) count output-capable leaves, a clone member included, drawn or hidden by their own eye (a hidden leaf still runs, 142 B; "items run while hidden" is the part of M that is not drawn), each counted once. The glossary's "item" stays the assignable node for the tick and the drag. The PO should say so in the glossary.

5. **Session code: a feature-owned module, `curvyo-editor-wasm/src/session/operations/`** (finding 1 of `docs/2026-11-06-architecture-review.md`).
   - Why not `session/manufacturing/`: it already holds the placement, machine, extension and placed-page state (six files). Operations have their own state (the tab's view, the Operations or Drawn switch, the drag preview, the two caches) and their own transitions.
   - Layout: `mod.rs` (the state struct and its public methods), `commands.rs` (the commit labels and the calls into document-core), `index.rs` (the two caches), `view_json.rs` (the Operations tab JSON, decision 6). Each file one responsibility and under 500 lines.
   - `Session` gets one field, `operations: OperationsState`. Its methods read the document through public `Document` methods only, and call `curvyo-laser-core` for every rule. They never touch another feature's private fields.
   - Cross-feature sequencing is explicit in the transitions that need it: page activation (`session/pages.rs`) and `sync_workbench` call `self.operations.on_page_changed()`; extension switch-off calls the same; moving an item off the page and deleting an object call the operations' key removal in the same commit as the change.
   - Commit labels (criteria 2 to 9 and the drag): "Add operation", "Rename operation", "Delete operation", "Assign to <name>", "Remove from <name>", "Cuts last off", "Cuts last on". Proposals: "Move operation" (drag and Alt+Up/Down), "Set margin" (Cut-out margin field). A deletion keeps its own label as built.

6. **The view JSON contract, with a shared fixture test (finding 3).**
   - Rust: `curvyo-editor-wasm/src/session/operations/view_json.rs` is the only place that builds the Operations tab JSON. It is serde-typed: each kind, section and count is a field or a closed enum, never a free string, with an explicit `unknown` variant for kinds this build does not know. The summary carries no missing count (Frage 169 A).
   - TypeScript: one decoder, `frontend/src/lib/operationsView.ts`, is the only place that reads it. No `as` casts; each field is checked at runtime, and an unknown kind goes to the `unknown` variant. The hook and components take the decoded type only. No generated types: two files and a fixture are simpler than a generator.
   - Shared fixtures: `curvyo-editor-wasm/tests/fixtures/operations_view/*.json`, one per representative state: `empty`, `five_kinds_cuts_last`, `cuts_last_off`, `unknown_and_clone_members`, `placed_and_clone_rows`.
   - Rust test (`curvyo-editor-wasm/tests/operations_view_fixtures.rs`) builds each state, serialises it and compares it with the file. A drift fails; the test never rewrites a fixture.
   - TypeScript test (`frontend/tests/operationsView.test.ts`) decodes each file with the same decoder and asserts the expected shape. It also rejects a copy with a wrong enum value.
   - A change to the contract is one change in three places: the Rust type, the TypeScript decoder, and the fixtures both tests read.

7. **Budgets (criterion 29), and how they are kept.** Values as the spec: (a) press assigning 1,000 items within 16 ms; (b) summary after any change on 1,000 items and 50 operations within 16 ms; (c) Operations tab first rows with 50 operations and 1,000 items within 100 ms; (d) each drag pointer move among 50 operations within 16 ms. Thread CPU in `curvyo-editor-wasm/tests/operations_budget.rs` with `tests/cpu_time/`, release build, in the CI `performance-budgets` job (`.github/workflows/ci.yml`, line 501), never wall time.
   - **Two caches, not one.** The chain index (which keys each leaf has, clone paths included) changes only when the drawing tree changes; the membership index and the summary change with every assignment or order change. A full drawing read per change would make budgets (a) and (b) fail: spec `0050` adrs, decision 3, measured 10,000 objects in 150 ms, so 1,000 cost about 15 ms before any work. The index is kept as `PlacedCache` does (`session/manufacturing/frame.rs`), keyed on the drawing read it was computed from.
   - (d) costs nothing in the document: a drag moves session state only (the preview line and the target index), and one commit runs on drop.
   - Deletion's key scan (ADR 0046 rule 3) is outside these budgets; the consequences of ADR 0046 say so.
   - If CI measures a budget over, the number is not changed silently: a dated note here names the measured value and the cause.

8. **Look: the kind decides, no colour per operation (customer, PR #305 review, 2026-11-10).** An operation has no colour, so there is no `op_color` register, no colour enum and no `--op-1` to `--op-8` tokens. The look of a leaf comes from its operation's kind, and the state of the leaf (Not assigned, Unknown).
   - **Look enum, `curvyo-render-core` (`Look`, Proposal).** One enum, unchanged in its six variants: solid line, dashed line, area fill, area with line, outer 2 px line, muted dashed (Not assigned and Unknown). The mapping from kind to variant is done by the caller in `curvyo-editor-wasm/src/session/operations/`, which reads `OperationKind` from `curvyo-laser-core` and passes a `Look` to `curvyo-render-core`. Neither core depends on the other (ADR 0034). Two users justify the enum: the Operations look and the Drawn view's guide look. No trait.
   - **Red and hatch are two fixed role tokens, not per-operation colours.** `--op-cut` (red: every Cut line) and `--op-hatch` (pale orange: the hatching of filled or engraved areas, Engrave and Engrave + Outline). Their light and dark values, and the 3:1 rule of criterion 18, belong to the ux-engineer (`ux-notes.md`); they are mirrored in `curvyo-render-core/src/theme.rs` like `--guide`, and the existing theme test is extended to them.
   - **Symbol and hatch are draw-list data in `curvyo-render-core`** (Proposal). The draw list gets a glyph item (a `Glyph` enum with one value per kind, five values) drawn at the object, and a hatch fill pattern (angle and spacing as draw-list values, spacing in screen pixels). `curvyo-render-core` owns the symbol and hatch data; `curvyo-laser-core` owns no drawing data. The Operations tab shows the same symbols through the icon set of `docs/design-system.md`; its decoder reads only the kind string (decision 6).
   - Nothing else in this ADR, or in ADR 0046, depends on a colour. The one exception is the two role tokens above, which are not per operation.

9. **Output-page membership of drawn objects is unchanged** (ADR 0024 §1, ADR 0043 §1). The Operations tab uses it to list which objects are on the output page; it does not change which objects are output-page members.

## Changes made in this step

- `docs/adr/0046-laser-operations-record.md` rewritten for Frage 167 B and 169 A. The index is not touched; the next free ADR number is 0046 (0044 reserved for `0105`, 0045 accepted).
- `specs/0101-laser-operations/adrs.md` (this file), updated for the answers of 2026-11-09.
- `specs/0101-laser-operations/specification.md`: criteria 11, 12, 25, 27; the glossary item; the table row for `0106` 18 (clone rows); the Fragen 167 to 170 blocks marked answered; the UX paragraph and the Links ADR line.
- `specs/0101-laser-operations/ux-notes.md`: section 6.4 (clone rows and member rows, chips), section 7 (no missing line), section 8 (rewritten: unknown operations and deleted objects), section 10 (accessible names), section 11 (the undo label list), section 14 and 15 (checklist and focus), section 16 (point 9), and the Frage line.
- `specs/0063-manufacturing-workbenches/specification.md`, line 109: names ADR 0046; ADR 0041 is not replaced.
- 2026-11-10 (PR #305 review): `op_color` removed from ADR 0046 decision 1; the goldens are regenerated without it; `adrs.md` decisions 3, 5 and 8 lose the colour (decision 8 rewritten: kind decides, `--op-cut` and `--op-hatch` are the only colour tokens). Commit label "Set operation colour" removed.

## Left for the product owner (spec `0101`)

- Criterion 13: the placed-page hidden rule (decision 2, 149 A).
- Criterion 21: the Engrave sentence of decision 3.
- The glossary "item" versus the counting unit of decision 4.
- The Format line: "stored in the output page's record" should say "in the manufacturing's child nodes (operations) and its `x.curvyo.laser.cuts_last` register".
- The spec's criteria that name a per-operation colour, the `op-*` tokens or the 3:1 rule for each operation colour (criterion 18 and any criterion on colour in the Operations tab or on canvas) must be rewritten to the kind-based look of decision 8.

## Questions for the customer

- **Accept ADR 0046** (document model and format: operation nodes, `member.` keys, clone member paths, format 39). This is the one blocking answer; the feature cannot be built without it.
