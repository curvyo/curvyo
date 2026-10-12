# ADRs for "Combine open paths: compound paths with open outlines"

Architect review, 2026-10-12. Most of this slice already exists below the commands: the stored
form has had an open or closed state per outline since `0016` (format 8), and the reader, the
renderer, the hit test, the bounds, the Boolean availability, Split and Break apart all honour
it per outline today. What is new is that Combine writes `closed: false`, that the commands
which decide by geometry (Combine's touch and depth tests, Split's part rule) skip or attach
open outlines, and new refusal and notice texts. **No new ADR, no new crate, no new dependency,
no trait, no generic, and no `format_version` bump** (decision 1 below, flagged for the
customer).

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §5, §9: Combine, Split and Break apart replace tree nodes in one commit, as in `0035` and
    `0048`; nothing about the tree changes.
  - §6: an outline is a run of cubic segments, open or closed; an open outline is the same
    shape as an ordinary open path.
  - §10: the SVG subset maps one `<path>` with several subpaths to one compound path, `Z` per
    closed outline (criteria 24, 25 are requirements for `0024`).
  - §11 with [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: a
    format bump is needed only when a reader of the current version would misread the file.
    Decision 1 applies that rule.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §4 and
  [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §7: display
  tessellation stays in `curvyo-render-core` (lyon, `FillRule::NonZero`). The fill of criterion 3
  is what `stroke::build_fill_path` already does: every outline one subpath, an open one closed
  by a straight chord, stored direction kept. No renderer change, only tests (criteria 2, 3).
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §1, §2: the
  touch, nesting and point-in-region tests stay in `curvyo-geometry-core`; the Boolean kernel
  is not called with open outlines (criterion 21).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: each outline's `closed`
  stays a last-writer-wins register; no new merge rule.
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md): compound path =
  `extra_subpaths`, nonzero fill fixed with no `fill_rule` register, refusal pattern.
- [`specs/0035-combine-and-break-apart/adrs.md`](../0035-combine-and-break-apart/adrs.md) and
  [`specs/0048-split-compound-path/adrs.md`](../0048-split-compound-path/adrs.md):
  `outline_touch`, `outline_nesting`, `replace_with_pieces`, `plan_split`, `plan_break_apart`,
  the generated performance inputs and the `boolean-budgets` CI job.

## Feature-local decisions

- **2026-10-12: decision 1, no `format_version` bump; the stored shape does not change.** The
  first outline's `closed` (`path_codec`) and each `extra_subpaths` element's `closed`
  (`subpath_codec`) exist since format 8. A build at format 11 already reads them
  (`read_extra_subpaths`), paints the fill chord-closed and the stroke open
  (`render-core/stroke.rs`), hit-tests and boxes per outline, counts the object as open for the
  Boolean operations and Combine (`boolean.rs` `is_open`, over every outline), refuses Cut on
  it, shows it in the Node tool without editing, and Break apart copies the flag. The one
  difference in an earlier build: its Split treats an open outline as a closed region when it
  forms parts. That grouping differs from criterion 15, but every node, handle and open state
  is kept, so nothing is lost or misread. A bump would lock out every earlier build, including
  peers on other machines (ADR 0004 sync), for no data safety gain. Criterion 26 is amended to
  match. *Rejected:* bump to the next free number, as the spec first said; it buys only the
  Split grouping in earlier builds. Customer question below.
- **2026-10-12: stricter validation on open, no shape change (criterion 27).**
  `validate_extra_subpaths` also requires every element's `closed` to be a boolean (every writer
  since format 8 wrote one). The first outline's `closed` keeps today's reading (absent means
  open) unless every golden file from format 1 to 11 holds it; then the same check applies to
  it. An outline with one anchor stays valid (`subpath_codec` already refuses only zero
  anchors); the renderer and the hit test already skip it. `document.json` already writes
  `closed` per outline (serde of `SubpathSnapshot`). Golden file: `compound_open.curvyo` at the
  current version, with a closed, an open and a zero-length open outline.
- **2026-10-12: Combine (`curvyo-ui-core/src/combine.rs`).** The `OpenPaths` refusal and the
  `is_open` use in Combine's availability go; Booleans keep `is_open`. The `Outline::new(t, true)`
  call and the `(…, true)` write become the operand's own flag. The touch test
  (`touching_outlines`), the self-crossing test and `outline_nesting` receive the **closed**
  outlines only; open outlines skip all geometry and are copied with their anchors, ids freshly
  minted and direction kept, in operand order (criteria 6 to 8, 30). Depth-based reversal
  applies to closed outlines only. Single-node outlines are dropped before planning (criterion
  10); the count rides in the plan for the notice.
- **2026-10-12: Split part rule (criterion 15) in `split.rs`, one new kernel need.** Regions form
  from closed outlines as today. Open outlines then join the same union-find: (a) touch pairs
  between an open outline and any outline, (b) else the region whose painted area holds its
  first node (`interior.rs`, nonzero over that region's closed outlines), (c) else its own set.
  `outline_touch` today flattens every outline with `flatten_closed`; for (a) it must flatten
  an open outline **without** its closing chord, or a "U" touches whatever its chord crosses.
  That is a parameter by the existing `Outline.closed` flag, not a new function. Pairs already
  in one set are skipped before the exact test, so a hatch stays near-linear.
- **2026-10-12: Booleans (criterion 21).** `is_open` is unchanged; only the refusal carries
  whether an offender is a compound path, to pick the longer sentence. Fracture, Flatten and
  Offset (`0037`, `0038`) are Ready, not built: they take criterion 22 into their own specs and
  reuse `is_open`.
- **2026-10-12: budget (criterion 29).** Same generator and job as `0035`/`0048`. Combine (b)
  runs no touch test at all (every outline open), so it is linear. Split on a crossing grid is
  not budgeted by the spec; the skip-same-set rule above keeps it from going quadratic in the
  exact test, the bounding-box sweep remains.

## Interaction with other slices

- **`0050` (building now, takes format 12):** no shared format number any more (decision 1). Code
  overlap is small (both touch `curvyo-document-core` and the ui-core panel rows); the build
  order after `0050` stays for that reason only.
- **`0049` (next, takes 13 after `0050`):** a clone copies the root's `closed` and
  `extra_subpaths` verbatim and draws through the same `draw_path`, so clones of a compound path
  with open outlines need nothing extra. The overlap is in the commands that remove and replace
  paths (`replace.rs`, Combine/Split operands); `0054` before `0049` as the customer set.

## Flagged

- **Customer, document model / persistence (default A):** A, no bump: an earlier build still
  opens a project with open outlines and shows it correctly; only its Split groups open lines
  differently. B, bump: an earlier build refuses the file. Recommendation A.
- No new ADR: the model change was made by `0016` (format 8); this slice only starts writing a
  value the format already holds.
