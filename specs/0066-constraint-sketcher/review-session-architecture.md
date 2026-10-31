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
