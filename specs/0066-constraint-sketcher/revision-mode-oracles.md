# Independent continuation geometry oracles

Status: **planned, not executed**, 2026-10-30. Derived from revised AC13/19/20/21
and analytic geometry before reading continuation implementation or its tests.
No Rust tests, build, browser or production code changes accompany this corpus.
Coordinates below are sketch-local millimetres. Numeric/point tolerance1e-6mm;
normalized tangent cross/dot residual tolerance1e-8; angular tolerance1e-8rad.
These tolerances are test comparison tolerances, not additional product limits.

## Frame, endpoint identities and direction

Previous line is A=(-10,0)→B=(0,0), with existing IDs `a`,`b`,`previous`.
Continue from B using outward unit direction e=(1,0), left normal n=(0,1).
All new first endpoints must reference **existing `b`**, not merely a new point
within tolerance. Expected result contains one new endpoint `c` and one segment;
mode relations reference actual previous/new entities and remain inspectable.
No arbitrary coincident duplicate, guide, grid or Vector snap target is involved.

For other poses map (u,v) to B+u·e+v·n; this permits the same independent corpus
on translated/rotated sketches without deriving an oracle from production output.
Point/curve direction is mathematical, not screen-Y orientation. UI left/right
labels must be mapped explicitly to these directions before asserting their names.
Do not guess that display order or positive screen-Y selects a mathematical branch.

## Six explicit modes: one simple known answer each

| Mode intent | New endpoint C | Required geometry and join witness |
| --- | --- | --- |
| Free line | (6,4) | Segment B→C, length√52; shared `b`. No tangent/perpendicular relation inferred from unrelated targets. |
| Perpendicular line | (0,8) | Segment B→C, length8; unit outgoing direction n. Dot(e,n)=0; persistent perpendicular relation to `previous`. The negative branch (0,-8) is equally perpendicular and follows the indicated pointer side. |
| Tangent line | (8,0) | Segment B→C, length8; outgoing direction e. Cross(e,C−B)=0 and dot(e,C−B)>0. Persist requested tangent/parallel direction intent supported by the analytic line tuple; do not require a guessed internal relation variant. |
| Tangent arc | (6,6) | Circle centre O=(0,6), radius6. Directed B→C sweep+π/2 (CCW); incoming radius B−O=(0,-6), outgoing tangent at B=e. At C tangent=n. Persistent tangency and selected branch. |
| Perpendicular arc, outgoing+n | (6,6) | O=(6,0), radius6; directed B→C sweep−π/2 (CW). At B tangent+n, perpendicular to e; at C tangent+e. Shares `b`, persists requested perpendicular intent, preserves this branch. |
| Perpendicular arc, outgoing−n | (6,-6) | O=(6,0), radius6; directed B→C sweep+π/2 (CCW). At B tangent−n; at C tangent+e. Distinct branch from previous row, not a second label for the same sweep. |

Circle witness for every arc: |B−O|=|C−O|=r, with positive finite radius.
CCW tangent at point P is normalized(-(P−O).y,(P−O).x); CW is its negation.
Compare the tangent **at the shared start**, not the chord B→C. Chord angle alone
cannot prove tangency/perpendicularity. If representation stores positive sweeps
plus a direction flag, normalize that representation before comparing the directed
arc; no specific field layout is assumed.

Same endpoint branch-disambiguation extension: for O=(6,0), B=(0,0), C=(6,6),
outgoing+n selects CW quarter-turn; outgoing−n selects CCW three-quarter-turn.
Both lie on the same supporting circle, so matching centre/radius/endpoints alone
is insufficient. If a mode intentionally restricts its admissible endpoint region,
it must explicitly refuse the incompatible pick rather than silently flip branch;
which regions are accepted should be reviewed against the visible mode contract.
Do not turn this extension into an undocumented universal major-arc requirement.

## Existing arcs and reverse endpoint continuation

Use previous arc centre O=(0,0), radius10, START A=(10,0), END B=(0,10),
directed CCW quarter-turn. Ordinary forward tangent is (0,1) at A and (-1,0) at B.

| Explicit endpoint resumed | Required outward direction | Tangent-line known endpoint |
| --- | --- | --- |
| Arc END `b` | forward tangent(-1,0) | C=(-8,10), reuse `b` |
| Arc START `a` | **negative** forward tangent(0,-1) | C=(10,-8), reuse `a` |
| Line END `b` of (-10,0)→(0,0) | (1,0) | C=(8,0), reuse `b` |
| Line START `a` of (-10,0)→(0,0) | (-1,0) | C=(-18,0), reuse `a` |

Both existing endpoint selection and explicit selected-endpoint Resume action use
these same outward directions. Previous entity orientation/identity must remain
unchanged. Reversing a line's stored endpoints or an arc's stored direction while
preserving its geometric endpoint should produce the same physical continuation.
Apply the six-mode local-frame table with this outward e and its left normal.
For example tangent arc from the arc END has new O=(-0,4), C=(-6,4), r6,
CCW quarter-turn; from arc START, e=(0,-1), n=(1,0), new O=(16,0),
C=(16,-6), r6, CCW quarter-turn. Previous arc stays centred(0,0), radius10.
A shared junction with several eligible outgoing branches must not guess which
previous segment defines e; an explicit choice is required by the revision plan.

## Later dimension edits: prove persisted curved-mode intent

For each of the three curved rows, preserve previous fixed line A→B and stable
join/constraint IDs. Set a driving radius from6 to10 after creation. For the tangent
arc edit fixture use initial C=(4.8,2.4), O=(0,6), r6 instead of the quarter-turn
endpoint, avoiding an initial double root at the circle extreme. To obtain an
independent endpoint oracle, retain endpoint local x=4.8 for that tangent fixture
and x=6 for the two perpendicular fixtures as a separate explicit
test constraint/dimension (not an automatically inferred external reference).
Select the branch continuously connected to the original valid arc.

- Tangent arc: O=(0,10); C=(4.8,10−√76.96), approximately
  (4.8,1.22731512021548). Tangent at B remains e. The other intersection
  y=10+√76.96 is a different branch; retain the initially lower intersection.
- Perpendicular arc outgoing+n: O=(10,0); C=(6,+√84), approximately
  (6,9.16515138991168). Tangent at B remains+n and directed sweep remains CW.
- Perpendicular arc outgoing−n: O=(10,0); C=(6,−√84). Tangent at B remains−n
  and directed sweep remains CCW.

If the public dimension/pick API cannot yet express fixed endpoint x, retain the
same radius-edit test with free endpoint and assert circle residuals, fixed join,
correct tangent/perpendicular relation and branch sign, but do **not** claim the
unique endpoint numbers above. Explicitly record that weaker geometric scope.
In either form require same relation IDs, visible/editable constraint intent,
one history step for radius edit, unchanged previous line, and undo restoring
original6mm radius/topology. A new preview that merely redraws the initial tangent
without persisting the relation fails this test.

## Mixed chain, closure and cancellation boundaries

Start A=(-10,0)→B=(0,0) free line; append tangent arc B→C=(6,6), centre(0,6),
r6; append tangent line C→D=(6,12). Known joins reuse `b`,`c`, and arc tangent
at C is(0,1). Then explicitly choose free line D→A to close: final endpoint is
existing `a`, no near-coincident duplicate; output loop is closed and all committed
segments remain constrainable. Each segment is one named history step.

Open variant stops after D with Enter outside numeric entry. Resume from D and
from A separately using outward directions(0,1) and(-1,0). Keep committed segments
through Escape/right-cancel of the next preview. Numeric Enter confirms one step
rather than ending the chain; key repeat/Alt mode dispatch, Tab/focus and right
button down/up/contextmenu remain the separate host-event matrix in
acceptance-revision-report.md. This document adds geometry oracles, not host PASS.

## Public API mapping — read-only preparation, not executed

Reviewed bounded API/source files during the producer's exclusive build slot;
no producer assertions, tests, build or browser were used for this mapping. Source
is still changing, so verify signatures again at explicit handoff.

| Oracle | Actual callable/readable seam and independent check |
| --- | --- |
| Six geometric modes | Public `construct(&Sketch,prefix,&Construction::Continue { previous,start,end,position,mode })` returns separate `intent` and `candidate`. All six `ContinuationMode` variants are public. Assert new segment endpoint IDs and retained input IDs, then recompute centre/radius/join tangent residuals from candidate coordinates; assert the corresponding relation in intent. `construct_preview` permits same transient oracle without document history. |
| Reverse START/END | Public `endpoint_tangent(&Sketch,&previous,&endpoint)` can be checked against four exact vectors above, followed by full `construct`. Canonical Arc representation stores CCW geometry: construction may swap stored start/end to represent the requested clockwise traversal. Compare traversal from the known shared join ID, never assume stored start equals gesture start. This does not weaken branch checks. |
| Real continuous Session route | `set_sketch_tool(Polyline)`, actual pointer down/hover/up, `set_sketch_mode` keys `free-line`, `perpendicular-line`, `tangent-line`, `tangent-arc`, `perpendicular-arc-left`, `perpendicular-arc-right`; inspect packed Document graph/solution. Public view JSON exposes mode, eligibility/reason and pending_picks. Use actual `key_down(KeyInput {key:"Alt",repeat,..})` for cycling, not direct mode setting as proof of keyboard routing. |
| Existing endpoint Resume | `sketch_select_entity(&id,false)` followed by `set_sketch_tool(Polyline)` reaches selected-endpoint resume; actual pointer picks additionally test hover/hit dispatch. Private resume helper need not be exposed. Stable graph IDs and unchanged previous geometry prove reuse. Test START and END for both line and canonical Arc. |
| Typed mode construction | `sketch_complete_numeric(first,second)` dispatches Polyline to mode-aware numeric_chain. Free line interprets length/angle; constrained lines retain mode direction; curved modes interpret first as radius and require actual hover to choose branch. Keep explicit mode and pointer state in setup; no nonexistent numeric endpoint setter is needed. Validate typed length/radius and persisted relation independently. |
| Later radius edit | Session `sketch_select_entity` plus `sketch_apply_action("radius","6 mm",false)` establishes a radius dimension, and `sketch_dimension_text(existingID,"10 mm",false)` edits that same relationship. Packed graph identifies the created constraint ID. Core `Relation`/`Constraint` plus public solve permit the exact endpoint-x fixture using explicit fixed construction references; no new public UI coordinate-constraint feature is required. Session free-endpoint variant must be labelled as invariant-only, as specified above. |
| Context dimension placement | `set_sketch_constraint_tool("dimension")`, entity selection/pointer picks, pointer placement, then `sketch_apply_action(kind,text,reference)`. View JSON exposes dimension_action/dimension_position and annotations. Packed `SketchSnapshot.labels` and public `Document::set_sketch_label(owner,&constraint,localPosition)` make exact local position, one-step history, save/open, and graph/checkpoint nonmutation testable. A missing placement must refuse without creating a relationship. |
| Selection modifiers | `modifiers_changed`, pointer down/hover/up and view JSON entities[].selected expose actual selected IDs without a private-state seam. Public UI helpers `sketch_hits`, `sketch_marquee`, `sketch_lasso` can independently test hit ordering/containment; they cannot replace Session tests for Shift/Ctrl/Alt dispatch. Pack before/after verifies no accidental geometry/history changes. |

No public seam blocker was identified for the listed core/Session acceptance cases.
Important limitations: selected-endpoint API proof does not prove discoverability
of the actual Resume button; typed Session calls do not prove Tab/focus/native input
undo; label storage does not prove visible annotation placement or reference read-only
UI; mode setters do not prove one Alt press or ignored repeat in the host. These need
actual event-driven browser checks after the host bridge is delivered.

Source-level observations for test design, not executed findings:

- Negative-signed arc construction swaps endpoints and stores a positive CCW sweep.
  Independent tests must orient using the shared endpoint and requested traversal.
- TangentLine after Line records Parallel; PerpendicularArc and line-after-arc use
  PerpendicularAt. Assert equivalent persistent geometric intent rather than requiring
  one guessed enum for every tuple. Later solve/edit must still preserve the witness.
- `sketch_dimension_action` currently suggests diameter for one circular edge; the
  public explicit action accepts radius too. Actual host radius/diameter choice is
  still a required future UI check, not established by the existence of the action.
- New current-format fixtures reported by the producer: valid
  `sketch_dimension_label_v28.curvyo` and malformed `sketch_label_invalid_v28.curvyo`.
  Extend only exact fixture inventories at test handoff; no broad naming exemption.

No implementation correctness verdict is made by this API orientation. Independent
known-answer tests remain pending the next explicit source-test/build handoff.
