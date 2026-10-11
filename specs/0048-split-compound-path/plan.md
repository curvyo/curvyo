# Plan for Split (0048) and Cut (0036), one slice

One branch `story/split-and-cut`, one PR, opened when the maker can press Combine, Split, Break apart and
Cut in the app and the full local gate is green (`CLAUDE.md` §7, the frontend checks, the budget tests).
Milestones are commits on the branch; none is a PR. **No `format_version` change** (pieces are ordinary
paths; `adrs.md` of both specs). No new dependency, no new crate.

`specs/0036-cut-at-crossings/plan.md` points here; the criteria numbers below are those of the spec named
in each task (0048 or 0036).

## Design notes

- **The rename is the first change.** The built Break apart (holes stay with their piece) becomes Split,
  with the touch rule of 0048 criterion 4 on top. `curvyo-ui-core/src/break_apart.rs` becomes `split.rs`
  (`plan_split`, `SplitPlan`, `SplitRefusal`); the new Break apart is a new, much smaller
  `break_apart.rs`. Both are decided by one availability function (0048 criterion 1).
- **One replace command for every piece-making command** (`adrs.md` of 0048):
  `Document::break_apart` becomes `Document::replace_with_pieces(removed, pieces, fill, label)`; the label
  is the caller's. `replace_with_path` (Combine, Boolean) stays.
- **Cut does the curve work in `curvyo-geometry-core/src/curve_intersections.rs`** (`adrs.md` of 0036):
  candidate segment pairs by a sweep, recursive de Casteljau subdivision until both pieces are flat,
  exact chord intersection, overlap ends for shared stretches. Only `+ - * /` and `sqrt`, so the result
  is bit-identical on every target. The kernel returns, per contour, the sorted cut parameters
  `(segment, t)`; ends of an open contour are not reported (they add no piece). The split of a contour
  into pieces is `curvyo-ui-core/src/cut_pieces.rs`, on `subdivide_at_parameter`.
- **Tests of 0035 that name Break apart change by name only.** Call sites of `apply_break_apart` /
  `plan_break_apart` / `Document::break_apart` that test the old behaviour (holes stay) move to the
  Split names with the same assertions; the 0035 criterion texts that the customer's decision of
  2026-10-10 changed are marked in the 0035 spec. No assertion is weakened.
- **Frontend stays local:** `PathCommands`, `PathGlyphs`, `pathText.ts`, the Path part of
  `useRailCommands.ts`, the two `apply*` wrappers in `useEditorSession.ts`. Nothing in the panel files
  (0051 runs in parallel).

## Affected crates/modules

- `curvyo-geometry-core`: new `curve_intersections.rs` (cut parameters), `flatten.rs` (one helper
  becomes `pub(crate)`), `outline_touch.rs` unchanged, `lib.rs` exports.
- `curvyo-document-core`: `replace.rs` (`replace_with_pieces`, `PieceFill`), `lib.rs`.
- `curvyo-ui-core`: `split.rs` (renamed), `break_apart.rs` (new), `cut.rs`, `cut_pieces.rs`,
  `combine.rs` (`PathAvailability`), `history_names.rs`, `lib.rs`.
- `curvyo-editor-wasm`: `session/split.rs` (Split and Break apart), `session/cut.rs`,
  `session/combine.rs` (Combine only), `wasm_split.rs`, `wasm_cut.rs`, `wasm_combine.rs`
  (availability), `lib.rs`.
- `frontend`: the Path card (four buttons), glyphs, `pathText.ts`, notices, tests.
- CI: the `boolean-budgets` job gets the new release tests.
- Docs: `specs/README.md`, the specs' status lines, `0035` amendment note, `docs/design-system.md` where
  the build differs, `docs/technical-debt.md` if a limit is found.

## Tasks

### Milestone 1: Split and Break apart (0048)

- [x] 1.1 `document-core`: `replace_with_pieces` with `PieceFill`; refusal-writes-nothing tests, label pin
  (0048 criteria 5, 6, 8, 11, 12; 0036 criteria 6, 7)
- [x] 1.2 `ui-core`: `split.rs` = old plan + touch rule (union-find over `touching_outlines`), one-part
  refusal (0048 criteria 3, 4, 5, 7, 9, 16)
- [x] 1.3 `ui-core`: new `break_apart.rs`, one closed path per outline, hole flag from the nesting
  (0048 criteria 11, 13)
- [x] 1.4 `ui-core`: `PathAvailability` with `split`, `break_apart`, `cut` from one function (0048
  criterion 1); `history_names` entries `split`, `cut_at_crossings`; guard test stays green
- [x] 1.5 `editor-wasm`: `session/split.rs`, `wasm_split.rs`, selection afterwards, refusal marks, label
  `split` / `break_apart` pinned (0048 criteria 6, 8, 9, 12, 13)
- [x] 1.6 Existing 0035 tests moved to the Split names (same assertions); golden file
  `compound_overlapping_squares.curvyo` (0048 criterion 4); Combine then Split and back (criterion 10)
- [x] 1.7 Budget test with the four generated inputs of 0048 criterion 15, golden counts, part search
  under 500 ms (0048 criteria 15, 16)
- [x] 1.8 Frontend: Path card with Combine, Split, Break apart; texts, glyphs (Split = the redrawn
  built art, Break apart = no hole), notices, tests (0048 criteria 1, 2, 8, 9, 12, 13)

### Milestone 2: Cut (0036)

- [x] 2.1 `geometry-core`: `curve_intersections.rs`: chord meeting (crossing, touch, T, collinear overlap),
  recursive subdivision, self-intersection, merge of overlap ends, snap to nodes; unit tests for
  lines and curves, a property test on random cubic pairs (0036 criteria 2, 4, 11, 12)
- [x] 2.2 `ui-core`: `cut_pieces.rs`, split of a contour into open pieces on `subdivide_at_parameter`
  (0036 criteria 3, 4, 5)
- [x] 2.3 `ui-core`: `cut.rs`: operands (paths, primitives), refusals (groups, compound, nothing crosses,
  out of range, more than 10000 pieces), plan (0036 criteria 1, 2, 6, 8, 9, 10)
- [x] 2.4 `editor-wasm`: `session/cut.rs`, `wasm_cut.rs`, label `cut_at_crossings`, selection afterwards,
  red outline of compound offenders (0036 criteria 2, 7, 9)
- [x] 2.5 Tests of 0036 criteria 12 and 12a (two lines, circle and line, rotated circle, figure eight,
  T, collinear, overlapping circles) and the E1 to E6 sheet of 0048 criterion 14 through the session
- [x] 2.6 Budget: the 50 x 50 grid, 5100 pieces under 2 s in release (0036 criterion 13); the
  `boolean-budgets` CI job runs the new release tests
- [x] 2.7 Frontend: Cut button, glyph (X of four stubs), texts, notice "pieces", refusal outline
  (0036 criteria 1, 7, 8, 9, 10)

### Milestone 3: docs and gate

- [x] 3.1 Docs: README rows and statuses, 0035 amendment note, design-system differences
- [x] 3.2 Full gate (`CLAUDE.md` §7 for the whole workspace, all targets; frontend `tsc`, oxlint, tests;
  release budgets), merge `origin/main`, PR draft for the lead

## Decisions made while building

- **`PathAvailability`** has one `compound` flag for Split and Break apart (criterion 1: one
  function) and `cut`; `BreakApartPlan` and `SplitPlan` hold `removed` and a flat `pieces` list.
- **A figure eight is cut at both passes of its crossing** and gives two pieces; the spec text of
  0036 criterion 12 says one (see the note in `specs/0036-cut-at-crossings/adrs.md`).
- **The curve module does its own cubic arithmetic** instead of `kurbo`'s, so that the result is
  bit-identical on every target; it is three files (`curve_chords.rs`, `curve_intersections.rs`,
  `curve_settle.rs`), and crossings are refined by Newton's method.
- **Glyphs:** the hole of the Split glyph is 4 x 4 (the brief said about 3 x 3; two pixels wide at
  1x does not read), the stubs of Cut end 2 units from the centre.
- **Path card height** is 180px in `railCard.ts` (four buttons).
- The tester-written 0035 tests that checked the old Break apart were renamed to Split; nothing was
  weakened, and the fuzz test of the path tools also presses Split and Cut now.

## Validation

- Unit tests per module; the sheet E1 to E6 as one session test; golden files in `tests/fixtures/`.
- Property tests: random cubic pairs (every cut point within 0.01 mm of both curves, every split half
  within 1e-6 mm of its original); random compound paths (Split and Break apart lose no outline).
- Release budgets in the `boolean-budgets` job: 0048 criterion 15 inputs and the 0036 grid.
- `wasm32-unknown-unknown` build of the touched core crates (`geometry-core`, `document-core`, `ui-core`).
- Browser check of the maker's route is left to the UX review (no Browser pane in this run).
