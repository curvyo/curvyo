# Plan for Combine open paths (0054)

One branch `story/combine-open-paths`, one PR, opened when the maker can combine open, touching and
crossing paths in the app, split and break the result apart, and run a Boolean operation on it, and the
full local gate (`CLAUDE.md` §7, the frontend checks, the budget tests) is green. Milestones are commits
on the branch; none is a PR. **No `format_version` change** (`adrs.md` decision 1, criterion 29). No
new crate, no new dependency, no trait, no generic.

## Facts found while planning (differences from the spec's fact check)

- **`0038` Offset is merged** (#115, format unchanged). As built after the 2026-10-13 rework, an open
  ordinary path is not an Offset operand (`0038` criterion 55: ignored, no handle, no text), there is
  no Apply button and no caption slot, and `curvyo-ui-core` `offset_plan.rs` drops every object for
  which `boolean::is_open` holds, so a compound path with an open outline is ignored today. Criterion
  24 was written against the old `0038` (Apply, caption slot, Outset band of an open path). See the
  open question OFFSET-OPEN below; until it is answered, task 5.4 keeps today's Offset behaviour.
- **`0037` Fracture and Flatten is not built**: its part of criterion 24 goes into `0037`'s spec
  (the spec says so for unbuilt commands).
- **`format_version` is 14 on `main`** (`0049` and `0057` are merged), not 11. Nothing changes: the
  slice does not bump. A linked clone of a compound path is an operand through its resolved path
  (`combine.rs` `contours_of` already reads `CloneSnapshot::resolved`).
- **`0024` SVG is specified (Draft)** with the customer decision of 2026-10-16 "no loss report".
  Criteria 26 and 27 are requirements for `0024` and are not built here; the loss-report wording of
  criterion 27 is for `0024`'s product owner to reconcile (noted for the lead, no code).

## Design notes

- **One flattening that respects `closed`** (`adrs.md`): `flatten_closed(anchors, tol)` becomes
  `flatten(anchors, closed, tol)`; an open outline gives the polyline of its segments only. `Flat`
  carries `closed`, so `chord()` and `self_touches` do not wrap and `distance_to_polyline` skips the
  closing chord for an open outline. The Boolean grid polygon of an open outline is closed by the
  straight chord implicitly, which is exactly the fill area. Callers: `boolean.rs`, `offset.rs`,
  `outline_touch.rs`, `outline_nesting.rs`.
- **Depth by "contains"** stays inside `outline_nesting` (same signature, same callers): first-node
  test for non-touching pairs, the vertex test of `adrs.md` for touching or crossing pairs (pairs taken
  from the existing box sweep of `touching_outlines`). Open outlines get depth 0 and no parent and
  contain nothing; callers pass closed outlines only.
- **Combine has no geometric refusal left.** `CombineRefusal` keeps `NeedsTwo`, `Groups`,
  `OutOfRange` and gains `SingleNodesOnly`; `OpenPaths`, `NoArea`, `Touching`, `SelfTouching` and
  `check_touches` are deleted. `CombinePlan` gains the counts for the notice: `open`, `touching`
  (closed outlines that touch or cross), `touching_holes`, `dropped_single_nodes`, `markers`.
- **Booleans:** the kernel drops `BooleanError::OpenOperands` (a public-API change of
  `curvyo-geometry-core` decided in `adrs.md`). `curvyo-ui-core` `BooleanAvailability::OpenPaths`
  becomes a count on `Ready` for the tooltip note; the success outcome gains `closed_open` and
  `removed_open` counts.
- **Texts stay in the frontend** (`pathText.ts`, `booleanText.ts`); `curvyo-ui-core` returns counts
  only, as today. The notice duration rule (8 s above 140 characters) goes into the one existing
  duration function of `pathText.ts` and its Boolean twin; the other `*Text.ts` files are not touched.
- **Shared files stay additive** (`0021` and `0046` run in parallel): no edit to key tables, the
  Select bar or the `specs/README.md` format plan; the README gets one status cell for 0054 only.
- **Tests of withdrawn criteria.** Tests that assert `0035` criteria 9, 10, 10a, the no-area part of
  11 and `0016` criterion 15 (open refusal) assert behaviour this spec withdraws. Each one is rewritten
  to the `0054` criterion that replaces it (same input, new expected result), never deleted
  silently; the PR lists every such test with old and new criterion. Assertions of criteria that
  still hold are not touched. See question TESTS below.

## Affected crates/modules

- `curvyo-geometry-core`: `flatten.rs` (`flatten` with `closed`), `outline_touch.rs` (`Flat.closed`,
  open-aware chords, touch pairs usable for counting), `outline_nesting.rs` (contains test for
  touching pairs), `boolean.rs` (open refusal removed), `offset.rs` (call site only), `interior.rs`
  (tests only), `lib.rs`.
- `curvyo-document-core`: `subpath_codec.rs` (`closed` must be a boolean), golden file
  `tests/fixtures/compound_open.curvyo`, `document.json` view check.
- `curvyo-ui-core`: `combine.rs`, `split.rs`, `break_apart.rs`, `boolean.rs` (`is_open` kept for the
  tooltip count and Offset), `cut.rs` (test only, criterion 20), `offset_plan.rs` (only if OFFSET-OPEN
  is answered), `lib.rs`.
- `curvyo-render-core`: tests only (criteria 2, 3, overlay without closing line).
- `curvyo-editor-wasm`: `session/combine.rs`, `session/split.rs`, `session/boolean.rs`,
  `wasm_combine.rs`, `wasm_boolean.rs`, `wasm_split.rs` (new count fields), budget tests.
- `frontend`: `pathText.ts`, `booleanText.ts`, their tests.
- Docs: `specs/0035`, `0016`, `0048` amendment marks (a line pointing to 0054 on each withdrawn
  criterion), `specs/README.md` status cell, `docs/design-system.md` rows by the ux-engineer.

## Tasks

### Milestone 1: kernel (`curvyo-geometry-core`)

- [ ] 1.1 `flatten(anchors, closed, tol)`: open outline = polyline of its segments, no closing cubic;
  closed unchanged; tests for the U of criterion 2 (no vertex on the chord between (10, 0) and
  (0, 0) except the ends) and bit-identical repeat (AC 3, 21, 33)
- [ ] 1.2 `Flat` carries `closed`: `chord()`, `self_touches`, `distance_to_polyline` do not wrap for
  an open outline; `touching_outlines` documented for open outlines; tests: an open U does not touch
  itself across its gap, an open line across a closed plate touches it (AC 1, 12, 15)
- [ ] 1.3 Boolean kernel accepts open outlines: `OpenOperands` removed, region = fill area; tests: U
  plus far square gives 2 outlines with the straight edge (10, 0) to (0, 0) and 100 mm²; a straight
  open outline in a plate adds nothing; an operand of straight lines only stays `EmptyOperands`
  (AC 21, 22)
- [ ] 1.4 `outline_nesting` by contains for touching or crossing pairs: tests 7b (touch from inside
  is a hole), 7c (crossing, both shapes), 7d (coincident, both shapes), 7g (two crossing holes in a
  square), concave A with B across its notch, figure eight (AC 7, 33)
- [ ] 1.5 Fill-area pins on the kernel side: point-in-fill of the U alone and inside a square in
  both directions (`interior.rs` tests), area of 7c = 700 mm² within 1 mm² via a Union (AC 3, 7c)

### Milestone 2: file and model (`curvyo-document-core`)

- [ ] 2.1 `validate_extra_subpaths` requires `closed` to be a boolean on every extra outline; the
  first outline keeps "absent = open" unless every golden file holds it (checked, decided in the
  commit); damaged-file tests (AC 30)
- [ ] 2.2 Golden file `compound_open.curvyo` (closed, open, zero-length open, closed crossing the
  first): open, read, save, reopen node for node; earlier golden files open unchanged and are not
  rewritten; single-node outline opens and draws nothing; `document.json` lists `closed` (AC 28, 29,
  30)

### Milestone 3: Combine (`curvyo-ui-core`)

- [ ] 3.1 Remove `OpenPaths`, `NoArea`, `Touching`, `SelfTouching`, `check_touches`; each outline
  keeps its own `closed`; open outlines copied with fresh ids, direction kept; tests of criterion 8's
  five inputs and criterion 6's 2 + 2 + 4 nodes (AC 6, 8, 9, 11)
- [ ] 3.2 Windings from depth for closed outlines with area; no-area closed outlines and open
  outlines keep direction; tests 7a to 7g through the plan (AC 7, 19)
- [ ] 3.3 Single-node outlines dropped and counted; all single-node refusal `SingleNodesOnly` (AC 10)
- [ ] 3.4 Notice counts in `CombinePlan`: holes, open, touching closed outlines (self-crossing
  singular case), touching holes, markers on any operand (AC 12, 14)

### Milestone 4: Split and Break apart (`curvyo-ui-core`)

- [ ] 4.1 `split.rs`: parts from closed outlines with the new depth and touch; open outlines join by
  rule (a) touch, (b) first node in the painted area of a part's closed outlines, (c) own set;
  skip-same-set; tests of criterion 15's two inputs (AC 15, 16)
- [ ] 4.2 One-piece refusal with a `with_holes` flag for the two wordings (AC 17)
- [ ] 4.3 `break_apart.rs`: open outline gives an open path, closed a closed path, direction kept;
  single-node outlines dropped and counted; test of criterion 18 (AC 18, 19)

### Milestone 5: Booleans, Cut, Offset (`curvyo-ui-core`)

- [ ] 5.1 `boolean_availability`: open outlines are usable operands; `Ready` carries the open count
  for the tooltip note (AC 23)
- [ ] 5.2 Boolean outcome counts `closed_open` and `removed_open` (open outlines with 2+ nodes, split
  by `outline_has_area` of the closed form); test of criterion 22's plate, score line and disc (AC 21,
  22)
- [ ] 5.3 Cut: test that a compound path with open outlines is refused as every compound path (AC 20)
- [ ] 5.4 Offset: today's behaviour kept and pinned by a test (compound path with an open outline is
  ignored like an open path), unless OFFSET-OPEN is answered otherwise (AC 24, as far as `0038` as
  built allows)

### Milestone 6: the slice in the app (`curvyo-editor-wasm`, `frontend`, `curvyo-render-core`)

- [ ] 6.1 `editor-wasm`: new counts through `wasm_combine.rs`, `wasm_split.rs`, `wasm_boolean.rs`;
  removed refusal variants; one undo step per command with its label, undo restores ids, direction
  and `closed` (AC 12, 31)
- [ ] 6.2 Frontend texts: Combine notice sentences in the order of the UX notes, single-node and
  markers sentences, Split one-piece wording, Break apart removed-outline sentence, Boolean notice and
  tooltip note, Combine note "Replaces the selection." with an open path selected; 8 s above 140
  characters (AC 10, 12, 13, 14, 17, 18, 21, 22, 23)
- [ ] 6.3 Render and hit tests: open outline stroked open (criterion 2 pixels), fill of the U in both
  directions (criterion 3), hit and selection box (criterion 4), no closing line in the refusal and
  drag preview outlines (UX notes); Node tool unchanged for compound paths (AC 2, 3, 4, 5, 25)
- [ ] 6.4 Rewrite the tests of withdrawn criteria to their `0054` replacements, list them for the PR
  (AC 8, 21)
- [ ] 6.5 Budget tests with the four generated inputs and golden counts, release build, in the
  `boolean-budgets` job (AC 32, 33)

### Milestone 7: docs

- [ ] 7.1 Amendment marks in `0035`, `0016`, `0048` specs; `specs/README.md` status cell; this plan
  checked off; `docs/design-system.md` rows by the ux-engineer in the PR (no AC; Definition of Done)

## Validation

- Unit tests per kernel function with the spec's numeric cases (1.1 to 1.5), including bit-identical
  repeats for criterion 33.
- Golden file `compound_open.curvyo` (criterion 29).
- The editor-wasm acceptance-style tests drive the whole command through the session, as `0035` and
  `0048` did, and the budget tests of criterion 32 in release mode.
- `wasm32-unknown-unknown` build of every core crate (the contains test uses `+ - * /` only).
- Manual check in the browser build: plate with a score line, Combine, Split, Break apart, Union with
  an open U; screenshot for the demo.

## Open questions (for the lead)

- **OFFSET-OPEN:** criterion 24 assumes the pre-rework `0038`. Options: A keep today (a compound path
  with an open outline is ignored by Offset like an open path, no text); B it is an operand and both
  directions work on its fill area (Q2's alternative, "prefer permitting"); C Inset only, Outset
  ignores it. Default A until answered (no new behaviour against customer decision 4 or `0038`
  criterion 55).
- **TESTS:** tests by the tester that assert withdrawn criteria are rewritten to the replacing
  `0054` criterion and listed in the PR. Default: yes.
