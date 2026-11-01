# Early Session architecture review

2026-10-30. **APPROVE for the Rust Session milestone** after the source corrections recorded below. Producer is
actively editing; this is a bounded review, not final feature acceptance.
No tests, builds or browser checks were run by architect.

## Findings sent directly to producer

- **S1 — parameter repair blocked by old graph evaluation.**
  `input.rs::parse_value` evaluates the entire current parameter collection
  before `sketch_parameter_text` can apply its replacement. A merged A→B→A
  cycle therefore cannot be repaired by replacing A with a literal, because
  evaluation fails before the edit. Duplicate-name repair has the same issue.
  Parse first, validate the proposed complete parameter state after replacement,
  then commit through existing delta validation. Preserve stable references.
  Follow-up: validating the entire proposed table still blocks incremental repair
  when an independent invalid component remains. In merged A↔B and C↔D, fixing
  A to a literal must commit while C/D remain diagnosed against last-valid
  geometry. Both Session validation and document delta validation need a repair
  route that validates the edited dependency closure without requiring all
  unrelated pre-existing errors to disappear. Reject newly introduced invalid
  intent; do not replace the valid checkpoint until the graph solves. S1 stays
  open until this route and its source checks are verified.
- **S2 — construction preview reads lack revision protection.**
  `draw.rs` directly consumes `sketch.preview`, unlike the revision-filtered
  drag accessor. A peer change followed by draw, before another interaction
  runs lifecycle sync, can show old construction geometry over current state.
  Guard every construction-preview read against its saved base revision and
  active sketch identity. `sketch_preview_fillet` also needs a stale-snapshot
  refusal before treating historical solution IDs as editable corner targets.

## Source checks with no blocker found

- Constructors retain presolve intent separately from candidates; Session diffs
  against the exact supplied solution base, including construction membership.
  Synchronous construction checks document revision before committing, while
  saved construction preview commits compare their stored base revision.
- Drag state retains its original graph, solution and document revision. Pointer
  goals are transformed into local coordinates; actual validated candidate
  geometry, rather than raw unreachable targets, supplies explicit record edits.
  Failed preview retains prior valid geometry; release checks revision again.
- Multiple body goals and analytic ellipse-axis helpers stay inside sketch-core.
  Helpers are not returned as persisted entities. Hard relation targets remain
  original intent; only initial guesses are prepared at requested drag positions.
  This is not evidence for universal nearest solutions or branch continuity.
- The public read-only `sketch_outline` reuse has a concrete renderer consumer.
  Derived preview paths cannot authorize ordinary path writes. Draw code uses
  existing clone preview resolution and render-core receives paths/glyphs only.
- Workbench/gesture state stays in Session, document owns completed records,
  WASM methods forward commands and serialized views. No speculative generic
  workbench registry or solver trait was introduced.
- Parameter edits retain stable IDs; bare lengths/angles receive explicit unit
  interpretation before expression construction. Origin and current entity
  identity remain distinct from historical solution topology.
- Inspected modules are below the repository's approximate 500-line guideline.
  Some multi-stage draw/drag functions exceed approximately 60 lines; their
  responsibility should remain clear or be split at final cleanup. No structural
  blocker is asserted solely for their current length during implementation.

## Remaining evidence

Re-review S1/S2 fixes. Independent tests and real browser checks must establish
input focus, revision transitions, cancellation, one-step history, stale-ID
handling, shared-clone preview, numeric repair and final pointer CPU/parity.
This review does not approve all Session behavior or complete AC1–18 coverage.

## Follow-up: partial repair and analytic snapping

S2 is closed in source: drawing uses `sketch_construction_preview`, filtered
by saved revision; fillet preview refuses stale snapshots. Tests reported by
producer were not rerun by architect.

S1 parameter-specific repair is corrected: `sketch_parameter_repair::allowed`
validates changed and previously valid dependency closures, allowing an unrelated
invalid component to remain diagnosed. The general repair route remains open:
constraint deltas are categorically excluded, so two independently dangling
constraints still cannot be removed one row at a time. Permit validated
deletion-only constraint repair on an already invalid graph, retaining the
checkpoint until remaining errors are repaired; this introduces no new relation.

Lead identified two drag snapping defects: the probe uses pointer position
instead of the actual offset-picked geometry, and excluding the whole active
sketch removes stationary analytic targets. Producer received guidance to probe
the press-time geometry plus raw translation, cache actual analytic target IDs
per revision, exclude moving targets and reuse the existing indexed resolver,
axis masks, kind flags and raw hysteresis. Source correction remains pending.

## Latest source follow-up

S1 is now closed for the reviewed parameter/constraint repair routes: the
document permits deletion-only constraint deltas on an already invalid graph
while validating remaining records; unrelated errors keep last-valid output.
Parameter repair retains dependency-closure guards. Entity topology repair can
use an explicit coherent multi-entity deletion operation; arbitrary partial
entity repairs are not asserted as supported. S2 remains source-closed.

Two snapping findings remain open:
- **S3:** `sketch/snapping.rs::add_stationary_sketch_targets` traverses fixed
  anchors as ordinary moving references. Distance(origin,P) therefore propagates
  through the fixed origin to independent siblings and removes legitimate
  stationary targets. Stop moving-dependency expansion across fixed nonmoving
  anchors without misclassifying a merely fixed radius as a fixed whole curve.
- **S4:** the handle probe uses actual handle position plus pointer translation,
  but `drag.rs` supplies the corrected pointer itself to `DragHandle`. An offset
  handle pick therefore still misses the snapped target. Derive the solver goal
  from the same actual handle feature plus raw translation and snap correction.

Analytic targets are gathered once at gesture start and appended to the existing
index; no alternate snapping resolver was introduced. Point/body offset probing
is corrected. Producer-reported 14 Session tests were not rerun by architect.

## Final bounded milestone recheck

S1–S4 are closed in current source. Parameter text parsing no longer evaluates
the old complete table; document repair permits guarded partial parameter fixes
and deletion-only constraint repair. Construction preview reads compare saved
revision, and stale fillet input is refused. Moving-target expansion excludes
intrinsically fixed points and explicit Fixed entities/defining points, and
incidence alone no longer spreads movement into stationary endpoints. Handle
solver goals now use the original actual handle plus the same local corrected
pointer displacement as the snap probe, preserving off-centre pick offsets.

Producer reports 15 focused Session tests passing, including fixed-origin sibling
targets and offset ellipse handles. Architect inspected source only, without
rerunning those tests. No remaining S1–S4 blocker prevents Rust handoff to the
independent tester. This approval does not establish complete AC6 performance,
native/WASM runtime parity, frontend/browser UX or whole-feature acceptance.

## Runtime defect follow-up: feasible dragging

The independent connected 100-entity workload exposed a failed unreachable-goal
preview. Retaining the previous feasible pose as the primary seed restored valid
movement, but its first beyond-reach frame took about 137 ms. Profiling isolated
the general second-order solve rather than the view or draw-list reads.

**(Proposal)** The reviewed bounded optimization recognizes a single-point goal
with a simple hard-distance path to a fixed anchor. It constructs a candidate at
the path's triangle-inequality reach bound, preserves disconnected geometry,
validates every original hard constraint and verifies attainment of the original
goal's distance lower bound with an explicit tolerance. Unsupported cases retain
the general solver. A failed optional retry preserves a valid first candidate.
The architect approved this source change: fixed-anchor provenance, evaluated
lengths, complete validation and raw-goal semantics remain intact. No model,
format, dependency or acceptance-criterion change is involved.

Producer evidence: the unchanged release chain test passes all 30 gestures;
27 measured frames have median 10.245 ms and maximum 11.104 ms of thread CPU
including pointer update, Properties view and draw list. At frame 20, goal error
decreases from 311.778 mm to the proven 252.800 mm lower bound. These results
still require independent repetition and actual native/WASM runtime comparison;
this source approval does not establish frontend or complete feature acceptance.

## Selection retention follow-up

Source APPROVE: `sync_sketch` still cancels transient previews, drag/pick state
and edit focus on revision changes, but intersects entity/constraint selection
with the current authoritative `snapshot.graph`. Historical checkpoint-only IDs
cannot survive that intersection. Missing/deleted/converted active sketches clear
selection; explicit entry and New still reset selection. Own constraint actions
can retain their subjects for subsequent actions. No model/API/dependency change.
Lead reports independent S12 changed from RED to PASS and producer reports 16
Session guards plus Clippy passing; architect inspected source without rerunning.

## Analytic freedom classification correction

Source APPROVE: `map_fully_constrained(sketch, response)` now requires the
backend's own-variable classification plus every defining point's classification
for circles, ellipses and arcs. Lines require both endpoints, since ACS assigns
no variables to the line record; points use their own classification. Thus a
fixed diameter alone cannot classify a freely translating circle as fully
constrained. `point_ids` includes ellipse centre/focus and arc centre/start/end.
This preserves the existing boolean/null view contract; no selected-entity DOF
count or sum of correlated freedoms is added. Producer reports eight focused
solver tests passing; architect inspected source only, without rerunning tests.

## Frontend handoff Rust API review

This section records the earlier handoff; the customer-authorized interaction
revision has a separate source verdict below.

Source-only review of `inspection.rs`, `inspect_links.rs`, `numeric.rs`,
`construction_preview.rs`, `lifecycle.rs` and `wasm_sketch.rs`: inspection uses
current authoritative IDs and a finite iterative parameter dependency closure,
including cyclic merged graphs. Selection/reveal changes are ephemeral; typed
values and units remain Rust-owned. Conversion records one explicit document
operation and switches back to Vector selection. No new persistence authority,
format, dependency or original-source scaling policy is introduced.

**S5 closed — source APPROVE:** Both typed and direct Fillet preview entry points
now clear preview and base before any fallible snapshot, parsing, evaluation or
selection check. Numeric Fillet Apply delegates to that same typed entry point
and commits only on success. Rejected latest input therefore cannot leave the
previous radius committable, independently of frontend button disabling.
Lead reports unchanged independent S13 passing; producer reports 21 Session
tests including invalid Apply inputs and a corrected single commit, plus affected
Clippy and rebuilt WASM. Architect verified source without rerunning those checks.
No remaining blocker was identified in this bounded Rust API review.

Architect ran no build, test or browser. The frontend handoff's reported checks
remain attributed to the producer; full story acceptance remains separate.

## Customer interaction revision: bounded architecture verdict

**APPROVE the reviewed Rust/domain/document API subset.** Page ownership uses
explicit live claims and provenance, with read-only conflict diagnosis and
matching-owner deletion guards. Object-only history retains only relevant claim
keys while preserving fieldwise peer guards. Independent copy remaps graph and
presentation identities; linked clones remain derived artwork, not new owners.
Raw document creation remains infrastructure; normal Session creation is page-aware.

`OnAxis` binds to fixed local frame coordinates, not the movable origin entity.
`PerpendicularAt` validates actual shared endpoints and maps to exact equations
over existing point variables; normalized residuals remain independently checked.
Continuation stores the relation in intent before solving. Centre-defined and
shared-endpoint arcs retain canonical analytic records. Axis implication uses
explicit fixed coordinates/relations rather than treating free seed alignment as
authority; augmented inferred candidates now pass independent validation.

Empty previews remain Session/render data without fabricated document objects.
Dimension label metadata is separate from solver intent/checkpoint fingerprints;
read-cache reuse refreshes positions, current graph IDs filter orphan metadata,
copy remaps keys and deletion/conversion clean up records. The previously flagged
finite-position gap is closed: local-coordinate conversion rejects nonfinite
values and dimension commit checks the saved position before its writing batch.
This matters because batching groups history but does not roll back failed writes.

The reviewed core dependency boundary is unchanged; no new dependency is required
by this revision. Existing long typed-dispatch/lifecycle functions need their PR
justifications, not a late behavior refactor. Architect performed source inspection
only. Lead reports independent core/document/Session checks and fixture sweeps;
producer reports 67 core and 43 Session tests plus affected native/WASM Clippy.
These reports were not rerun here. This approval does not establish final browser
journeys, revised host performance/parity, full gate or customer acceptance.

## Proposed feasible-chain certificate

**Source design APPROVE for a bounded experiment, not implementation acceptance.**
For the existing certified simple positive-hard-distance chain and a single free
endpoint goal, split its previous feasible pose at a deterministic interior joint.
Intersect circles whose radii are the two current subchain chords; proper rigid
rotations of the subchains can reach the unchanged raw endpoint goal while
preserving their internal distances. A fully validated candidate reaching that
goal within the existing explicit residual tolerance attains the positional
objective's zero lower bound numerically. Do not claim symbolic exact arithmetic.

Compare both valid intersections using whole-chain displacement from the previous
seed and deterministic ties. Degenerate chords/intersections, unsupported goals
or any failed full original-intent validation fall through to unchanged ACS;
failure of this restricted construction is not proof of infeasibility. Preserve
disconnected geometry, fixed intent, checkpoint seeds and the existing unreachable
reach certificate. No entity-count branch, clipped goal, new unknown, relaxed
tolerance or reduced solver budget is authorized. Repeated branch/reversal,
degeneracy, additional-constraint and fallback tests plus native/WASM parity and
the unchanged actual-host budget still need implementation evidence.

### Feasible-chain source follow-up

**APPROVE the implemented certificate subset.** `drag_reach::candidate` dispatches
only the existing single-goal/simple-chain case; `drag_chords::candidate` uses a
deterministic interior split and both proper-rotation intersections. It compares
whole-pose displacement with stable branch ordering. Scale-aware conditioning
checks reject zero chords, coincident centres and near-tangent intersections;
unsupported cases return to the unchanged ACS path. No clipped target or new
unknown is introduced. Each result must have finite displacement, endpoint error
at most 1e-6 mm and pass complete `validate_candidate`; the outer original-intent
guard in `solve_seeded` remains. Nonpath records and contact seeds stay unchanged.
The previous unreachable reach-bound certificate remains intact.

Producer reports the existing core suite and six focused certificate cases
passing. Architect inspected current source only, without running those tests.
Independent oracles, native/WASM parity and actual-host performance still require
their own execution; this source approval does not establish their results.

### Proposed coherent-tail extension

**Design APPROVE for a bounded experiment; source review still required.** Actual
host picking selected the final segment body, producing two positional goals.
Do not change picking or the workload to force a single point. A certified simple
distance chain may admit the same construction when its complete goal-ID set is
exactly a contiguous suffix and all targets request one coherent translation of
the previous pose. Solve the prefix to its first goal using the two rotations,
then translate the remaining suffix rigidly, moving the shared joint only once.

Validate the combined full graph, never a prefix with tail constraints discarded.
Check every original raw target within the existing explicit goal tolerance;
meeting all goals attains the sum-squared objective's zero lower bound numerically.
Compare complete-pose displacement for both branches and retain deterministic
ties. Fixed tail geometry, extra constraints, incoherent/noncontiguous/foreign
goals, handles and degeneracies must either satisfy all original guards or fall
through unchanged ACS. No goal-count or benchmark-specific condition is permitted.
Test two- and longer-goal suffixes plus invalid eligibility/fixed/crosslink cases;
rerun native/WASM parity and the unchanged actual-host workload before acceptance.

### Coherent-tail initial source review

**Finite-target finding closed; source APPROVE for the certificate subset.**
`drag_chords::translated_tail` now rejects any nonfinite raw x/y coordinate before
membership, coherence or distance comparisons. Malformed goals cannot exploit
NaN comparison behavior; ordinary adapter finite-goal rejection remains in place
after certificate refusal. The focused regression covers first/later goals,
both coordinates and NaN/positive/negative infinity. Per-goal residual checks,
exact suffix membership, combined transforms, whole-pose branch cost and full
hard validation remain intact. Producer reports 80 core tests and strict
all-target Clippy passing; architect inspected source/tests without execution.
Independent test execution, native/WASM parity and host performance remain pending.

## Revision 2 review (2026-10-31)

Scope: `git log 380f0112..443950bc` (M1 tangency side, M2 capture/lock/mode label/entry,
M3 constraint rail, dimension layout, constraint symbols). Committed HEAD only; the
uncommitted working-tree edits in `curvyo-ui-core/src/sketch_dimension_layout.rs` were not
reviewed. Source read only, nothing built or run.

**Verdict: APPROVE WITH CHANGES.**

What holds:
- Boundaries: no `Cargo.toml`/`Cargo.lock` change; `curvyo-ui-core` and `curvyo-render-core`
  stay independent of each other and free of UI/IO crates; layout is pure geometry in document
  millimetres from a `ViewTransform`, so it builds for wasm32 unchanged.
- Format: no `curvyo-document-core` change; `CURRENT_FORMAT_VERSION` stays 28.
- M1: `tangency::internal_joint` decides the side from the current pose; the `internal` flag is
  part of the ACS 0.1.19 relation schema (`constraint_catalog.rs`), and `validate` uses the
  same predicate, so check and solve agree.
- Wasm surface: `set_sketch_mode`, `set_sketch_circle_dimension` and
  `sketch_numeric_values_json` have no remaining frontend caller; the three new bindings are
  thin and return `Result` where the session can refuse.
- No production `unwrap`/`expect`; all new ones are in `#[cfg(test)]`.
- Rail cost: `sketch_rail` runs inside `sketch_view_json`, which the AC29 CPU-time budget test
  (`tests/sketch_kind_drag_budget.rs:128`) already exercises at 100 entities.
- Attribution: source commit, files, authors, licence, changes and check date are recorded;
  the derived glyphs stay LGPL-2.1-or-later at file level, which can be combined with
  AGPL-3.0-or-later (LGPL-2.1 §3 to GPL-2-or-later, then GPL-3 §13 to AGPL-3). ADR 0006 §5
  ("licence recorded next to it") is met by the header of `constraintIcons.tsx`.

Required changes:
1. `docs/third-party-assets.md:21-23`: LGPL-2.1 §1 wants recipients to get a copy of the licence.
   Add the text as `docs/licenses/LGPL-2.1.txt` (verbatim from gnu.org), link it from this entry,
   and state that `constraintIcons.tsx` is under LGPL-2.1-or-later as a whole (not AGPL).
2. `curvyo-render-core/src/sketch_annotation_marks.rs:1,102`: the module has two
   responsibilities (dimension marks, solved stroke; §5 one responsibility per module). Move
   `build_sketch_solved` and `SOLVED_PX` to `sketch_overlay.rs`.
3. `curvyo-render-core/src/sketch_annotation_marks.rs:79` and `sketch_overlay.rs:94`: two copies
   of the same dash loop (4/3 and 2/2 px). Make one `glyphs::dashed_line(from, to, width, on, off,
   color)` and call it from both.
4. `curvyo-editor-wasm/src/session/sketch/relation_tools.rs:7`: `set_sketch_circle_dimension` is
   now only called from `mode_tests.rs:385,395` (§5 delete dead code). Delete it and drive those
   tests through the M-key path in `modes.rs:49`.
5. `curvyo-ui-core/src/lib.rs:481`: `THINNING_LIMIT` is re-exported but has no caller outside the
   module. Make it private (no speculative public API).
6. §5 functions over ~60 lines, new in this range: `curvyo-ui-core/src/sketch_dimension_layout.rs:192`
   `linear` (76), `session/sketch/annotations.rs:86` `sketch_dimension_records` (99),
   `session/sketch/symbols.rs:45` `sketch_symbol_records` (90), `session/sketch/first_segment.rs:67`
   `first_segment` (73); grown: `construction_preview.rs:7` (77 to 92). Split `linear`
   (public-crate core logic: arrows/ticks choice into its own function) and the two `*_records`
   functions (one function per dimension or constraint kind group); justify the remaining ones in
   the PR review guide.
7. `curvyo-ui-core/src/sketch_dimension_layout.rs`: the working copy is at 511 lines. Before the
   commit, move the text-box/overlap helpers (`text_half_px`, `break_around_text`,
   `box_interval`, `free_text`) into their own module to stay under 500.

Recommended (not blocking):
- Name the inline tolerances `1e-9` (`session/sketch/lock.rs:23,163`, `entry.rs:153`) and `1e-12`
  (`curvyo-sketch-core/src/tangency.rs:29,57,78`) as constants like the existing `TOLERANCE`.
- `curvyo-ui-core/src/sketch_tool_use.rs:71` `for kind in 0..5` silently goes stale when `Entity`
  gains a variant; derive the bound from `kind_index` or iterate a const array of kinds.
- `session/mod.rs` (+1) and `session/keys.rs` (+3) grow further over 500 lines; the existing
  `docs/technical-debt.md` entry covers them, add a dated line.
- Architect follow-up: a dated note in `adrs.md` for the new `curvyo-ui-core` public layout API
  and the removed wasm bindings.

## Revision 3 review (2026-10-31)

Scope: commits 022e6e46 (`curvyo-sketch-core`: `construct_joined`, `construct_preview_joined`,
`PointOn`, `SymmetricAxis`, `HorizontalPoints`, `VerticalPoints`, `Symmetric` about a point,
v28 goldens) and 2bb8d9b6 (`curvyo-editor-wasm`/`curvyo-ui-core`: capture on every pick,
follow state, typed sizes, selection-first constraints, `sketch_open_entry`,
`sketch_complete_fields`). Source read only, nothing built or run.

**Verdict: CHANGES REQUESTED.**

What holds:
- Boundaries: no `Cargo.toml`/`Cargo.lock` change; `joins.rs` is pure and builds for wasm32;
  `@axis-x`/`@axis-y` resolve to `Target::Axis` before any record lookup
  (`curvyo-ui-core/src/sketch_actions.rs:84`) and only ever become `OnAxis` or `SymmetricAxis`,
  never a stored `EntityId`.
- Format: `CURRENT_FORMAT_VERSION` stays 28. It was raised on this branch (main writes an older
  version) and has not been released, so extending 28 needs no bump and no migration (ADR 0004,
  ADR 0026 format fact). The plate golden carries all four new variants plus `Symmetric` about a
  point and asserts a clean under-constrained solve (`sketch_fixtures.rs:74-83`); the other
  three v28 goldens were regenerated from the same graph.
- Record validation covers every new variant (`validation.rs:154-209`); copy remapping covers
  every new id field (`remap.rs:76,93-100`) and is tested
  (`tests/point_on_and_symmetry.rs:241`); residual check and solver encoding agree per variant.
- Public API: `construct_joined` keeps `construct` as the empty-join case, errors are
  `SketchError`, no production `unwrap`/`expect`, no new trait or generic.
- Wasm surface: `sketch_complete_numeric` binding removed with no frontend caller left;
  both new bindings are thin.

Required changes:
1. `curvyo-sketch-core/src/model.rs:121-128` (document model, written format): `Symmetric.axis`
   now names either a mirror line or a centre point, and three places sniff the kind with
   `if s.point(axis).is_ok()` (`validation.rs:192`, `residuals.rs:230`,
   `solver_relations.rs:110`). A field called `axis` that holds a centre is a format wart we
   would carry forever once 28 ships; `SymmetricAxis` already shows the pattern. Add
   `Relation::SymmetricPoint { source, image, center }`, restore `Symmetric.axis` to lines only,
   emit the new variant from `curvyo-ui-core/src/sketch_actions.rs:255`, update the wasm
   symbol/view mappings, `sketch_fixtures.rs:142` and regenerate the v28 goldens. Format stays 28.
2. `curvyo-sketch-core/src/joins.rs:117` `relation_ids_mut` is a second copy of
   `Relation::entity_ids` (`references.rs:36`) and a third walk next to `remap.rs:74`; the
   reference walk is `references.rs`'s responsibility (§5 one responsibility per module).
   Move it to `references.rs` as `Relation::entity_ids_mut` beside `Entity::point_ids_mut`, and
   let `remap::relation` use it plus the expression rename, so the next variant is added in one
   place.
3. `curvyo-sketch-core/tests/point_on_and_symmetry.rs`: no negative record test for the new
   variants (DoD, record validation is the format guard). Add `validate_records` refusals for
   `PointOn` whose curve is a point, `Symmetric` whose axis is a circle, `SymmetricAxis` and
   `HorizontalPoints`/`VerticalPoints` naming a line, and (after 1) `SymmetricPoint` whose
   centre is a line.
4. `curvyo-editor-wasm/src/session/sketch/typed_dimensions.rs:111` `sketch_typed_relations` is
   78 lines (§5 ~60) and keys dimensions on UI label strings ("Width", "Angle"). Replace the
   label with a small `EntryField` enum produced by the stage that owns the labels
   (`entry.rs:194-216`) and split the mapping per construction kind.
5. AC39 is not met for line/slot Angle and ellipse radii (`typed_dimensions.rs:109-110`
   comment): the criterion says every typed value becomes a driving dimension and the DOF
   reflects exactly the typed fields. Implement the recommendation below in this story.

Recommended (not blocking):
- `curvyo-editor-wasm/src/session/sketch/creation.rs:92,217`: inline `1e-9` duplicates
  `CAPTURED_MM` (`capture.rs:12`); reuse it. `joins.rs:5` `PICK_TOLERANCE_MM` and `CAPTURED_MM`
  as `Length` instead of bare `f64` (§5 units are newtypes).
- `curvyo-sketch-core/src/solver_relations.rs:140` `_ => Value::Null` pushes a null relation to
  ACS if the outer dispatch ever drifts; return `Err(SketchError::Invalid(..))` instead.
- `curvyo-editor-wasm/src/wasm_sketch.rs:203`: pass one slice of `(text, typed)` into
  `Session::sketch_complete_fields` instead of two parallel slices that `zip` silently truncates.
- `PointOn` (infinite line, full curve) vs `On` (finite) differ only by name; since 28 is
  unreleased, consider the clearer persisted name `OnExtended` now, or document the
  distinction in the `On` doc comment.
- Reserve the `@` prefix: `validate_records` refuses entity IDs starting with `@`, so a record
  can never be shadowed by a selection-only axis ID.

Decision: typed Angle and ellipse radii (recommendation, no customer question).
- Options: A. add two relation variants now in format 28: `AxisAngle { line, value }`
  (directed angle from local +X to the line's start-to-end direction; the slot uses a side
  line; solver reuses `axis_operand` with an ACS angle relation, no new unknowns) and
  `EllipseRadius { curve, axis: EllipseAxis /* Major | Minor */, value }`, each with
  validation, residual, remap, symbol/rail kind, a negative record test and a plate-golden
  record. B. keep geometry-only and ask the customer to narrow AC39. C. emulate with existing
  relations (an `Angle` against a construction line on the axis, `Distance` centre-focus):
  rejected, it adds visible helper records and the focal distance is not the radius the maker
  typed.
- Recommendation: A. It is the literal behaviour of accepted AC39, so it adds no scope and needs
  no new customer decision; B would change accepted behaviour, which is the customer's (§3).
  Adding relation variants extends the relation vocabulary already authorized under ADR 0026,
  inside an unreleased format, and is reversible in one PR: no new ADR and no `needs-customer`.
  The architect records it as a dated note in `specs/0066-constraint-sketcher/adrs.md`. Before
  coding, the implementer confirms ACS 0.1.19 has an ellipse major/minor radius relation; if it
  has none, the residual is encoded through an operand distance (centre to an operand point on
  the axis) and the finding goes back to the architect before an alternative is chosen.

## Revision 4 review (2026-10-31)

Scope: commits f892f448 (`curvyo-sketch-core`/`curvyo-document-core`: `HorizontalDistance`,
`VerticalDistance`, format 29, v29 goldens), e6037854 (`curvyo-ui-core`: axis distance actions,
projected dimension layout) and 5038d599 (`curvyo-editor-wasm`: return to Select, rail tools).
Source read only, nothing built or run.

**Verdict: CHANGES REQUESTED.**

What holds:
- Format: the bump 28 -> 29 is required (28 shipped with PR #228) and correctly documented at
  `curvyo-document-core/src/document.rs:289-296`; no record changes shape, migration is none.
  The plate golden carries both new variants (`sketch_fixtures.rs:124-148`), all eleven current
  goldens were regenerated as v29, and `format_28_sketch_goldens_still_open`
  (`sketch_fixtures.rs:377`) keeps every v28 golden, including `slot_sweeps`, opening or refused
  as before. ADR 0026 carries a dated note.
- One sign convention end to end: `measure.rs:32-37` (`b - a`), ACS 0.1.19
  `horizontal_distance`/`vertical_distance` (`Difference`, residual `p2 - p1 - value`), and the
  independent residual check, which goes through `measure` for every expression relation.
- Validation: both operands must be points (`validation.rs:260-263`); driving values must be
  `>= 0` for axis distances only (`validation.rs:51-57`), other lengths stay `> 0`. Remap and
  references go through `entity_ids`/`entity_ids_mut`/`expression_mut` (`references.rs:51-52,
  90-91,119-120,136-137,156-157`), so copies and the round trip are covered
  (`tests/axis_distance.rs:142`).
- Creation stores the left or lower point first (`sketch_actions.rs:153-160`), tested in both
  pick orders; display uses `abs` (`annotations.rs:276-280`).
- Boundaries: no dependency change; the new core code is pure and wasm32-safe; no production
  `unwrap`/`expect`, no new trait, generic or crate. `tool_return.rs` has one responsibility.

Required changes:
1. `curvyo-editor-wasm/src/session/sketch/input.rs:236-244` (AC46, keep the side): a reference
   axis distance follows the geometry, so after a drag its signed measure can be negative while
   the text shows `abs`. Turning it into a driving dimension (or entering a value) with that
   positive text makes the solver flip the points to the other side. When the relation is an
   axis distance and `measure` on the current solution is negative, swap `a` and `b` before
   storing. Add a session test: reference horizontal distance, drag `b` left of `a`, make it
   driving at the shown value, assert `b` stays left.
2. `curvyo-editor-wasm/src/session/sketch/annotations.rs:361-450` (§5 function ~60 lines):
   `dimension_geometry` grew to 89 lines. Move the new arm (395-410) into
   `fn axis_distance_geometry(..)` and, if still over, the radius/diameter arms likewise.
3. `curvyo-ui-core/src/sketch_dimension_layout.rs` (504 non-test lines, was 435) and
   `curvyo-editor-wasm/src/session/sketch/annotations.rs` (522 non-test lines, was ~496) crossed
   or grew past 500 (§5 module size). Move the `Projected` layout (`sketch_dimension_layout.rs:
   163ff`) into its own `#[path]` module as `sketch_dimension_text.rs` already is, and move
   `value_text`/`kind_name` out of `annotations.rs`; or justify both in the PR description and
   add one entry to `docs/technical-debt.md`.

Recommended (not blocking):
- `curvyo-sketch-core/src/validation.rs:260`: refuse `a == b` for axis distances. Picking the
  origin and the matching axis yields `(origin, origin)` (`sketch_actions.rs:140-145`), a record
  that is vacuous at 0 and unsolvable otherwise; reject it in `sketch_relation` too.
- `curvyo-document-core/tests/fixtures`: add one damaged v29 golden with a negative axis
  distance value, so the new `>= 0` rule is guarded at the file level, not only in
  `tests/axis_distance.rs:114`.
- `curvyo-sketch-core/src/validation.rs:54`: name the zero bound
  (`value >= 0.0` is a domain bound, not a geometric comparison; a one-line comment suffices).

## Revision 5 review (2026-11-01)

Scope: commits 2e44daf3 (`curvyo-ui-core`: point pick priority, `sketch_pick_order`, Symmetric
centre, dimension text between extension lines) and 300bc2c1 (`curvyo-editor-wasm`: axes over
dimension lines, last rail pick is the Symmetric centre). Source read only, nothing built or run.

**Verdict: CHANGES REQUESTED.**

What holds:
- Boundaries: no dependency, crate, trait, generic or format change (format stays 29); new core
  code is pure and wasm32-safe; no production `unwrap`/`expect`.
- Annotation glitch (AC49): `text_along` is shared by the linear and projected layouts, the
  re-layout at `sketch_dimension_layout.rs:139` stays inside the bounded 4-pass loop, and
  `sketch_dimension_layout.rs` is at 471 non-test lines.
- Axis picking (AC52): `sketch_axes_at` replaces the inline test in `sketch_axis_press`, and
  `sketch_annotation_yields` puts the hover and press rule in one place.
- `sketch_pick_order` is acceptable: it shares `ranked` with `sketch_hits`, has one caller
  (`selection.rs:50`) and a different job (all candidates for the Alt cycle, not the winners).

Required changes:
1. `curvyo-ui-core/src/sketch.rs:14-16,90` (point reach as 10/6 of the caller's tolerance): not
   acceptable. It hides a caller convention (every caller passes 6 px) inside a public core API:
   a caller passing another tolerance silently gets another point reach (the tests at
   `sketch.rs:285,296` already pass 0.1 mm), and the 10 px that AC50 names appears nowhere.
   Fix, without a new type and with the same arity, following `layout_dimensions` (`px` = mm per
   screen pixel): `sketch_hit`, `sketch_hits` and `sketch_pick_order` take `px: Length` (one
   screen pixel in document mm); `curvyo-ui-core` owns `SKETCH_EDGE_PICK_PX = 6.0` and
   `SKETCH_POINT_PICK_PX = 10.0`. The callers pass `Length(1.0 / self.view().scale())`, which also
   removes the repeated `6.0` literal (`drag.rs:49,165`, `cursor.rs:41`,
   `annotation_input.rs:106`, `selection.rs:54,87`).
2. `curvyo-editor-wasm/src/session/sketch/rail.rs:167-206` (CLAUDE.md §6, domain logic in core):
   the armed rule (three points, the last is the centre, building `SymmetricPoint`) lives in
   `-wasm`, while the preselection rule lives in `curvyo-ui-core/src/sketch_actions.rs:344`.
   That gives two Symmetric rules in two crates. Move the relation building into
   `sketch_actions.rs` as one function (e.g.
   `pub fn sketch_symmetric_about(graph: &Sketch, picks: &[EntityId], centre: &EntityId) -> Option<Relation>`)
   with a unit test. The session keeps only the check on the tool and the apply.
3. `curvyo-ui-core/src/sketch_actions.rs:306,363` (§5 units are newtypes): `CENTRE_TIE_MM: f64`
   is compared against `.distance(..).0`. Use `const CENTRE_TIE: Length = Length(1e-6)` and keep
   the scores as `Length`.

Recommended (not blocking):
- `curvyo-editor-wasm/src/session/sketch/drag.rs:45-66` cycles through `sketch_hits` (ties
  only), and `selection.rs:50` cycles through `sketch_pick_order`. If AC34 means one cycle,
  use `sketch_pick_order` in both places and drop `sketch_hits` from the public API.
