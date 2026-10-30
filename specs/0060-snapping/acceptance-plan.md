# Independent acceptance plan: snapping

Status: Prepared from the amended specification and research before reading
production implementation (2026-10-30). Execution evidence is recorded separately
in acceptance-report.md; this remains the original planned matrix.

This plan covers all 15 criteria. Document-owned settings, fixed page fractions
and the dedicated Properties entry are customer decisions. SNAP-PAGES remains
pending; tests use the stated default of all visible pages until the customer
answers. Multi-page integration depends on the accepted page slice; absence from
this branch is a dependency, not permission to reduce page coverage. No old-file
migration or backward-compatibility work is required for this unreleased app.
Missing snapping registers in a current-format document remain AC 14 coverage.

Expected values below come from the specification, independently calculated
geometry and existing tool contracts. Executable acceptance tests will use public
APIs before implementation-diff review, in the affected crates' `tests/acceptance_0060.rs`.
Actual pointer/key sequences and browser interactions are required where the
criterion describes a tool or rail interaction. Source-text assertions alone do
not prove those behaviors. UX notes are being revised; final visual tokens and
keyboard details will be checked against their ready version before execution.

## Criterion matrix

| AC | Scenario and independent oracle | Evidence |
| --- | --- | --- |
| 1 | New document: disabled, Objects/Guides/Nodes/Page enabled, capture 8 px, release 14 px. Change every setting, save/open and open another session of the same document: values agree. A different new document retains defaults. Changes are document operations; opening/reading does not write. | Public settings/persistence tests, current-format golden file, browser reopening. |
| 2 | Across every tool and empty/single/multiple selection, Snapping rail entry remains accessible. Normal click opens settings without changing enabled/history. Shift+click toggles once without changing the open panel. Enabled blue indication remains distinguishable from panel-active state. Explicit panel switch works by keyboard. Off suppresses geometry pulls and markers. | Browser click and keyboard tests plus session input tests. |
| 3 | Approach an isolated target from both directions: capture at the configured screen-distance boundary, retain through release zone, release outside. Repeat at several zooms/device pixel ratios and with capture/release zero or equal. A stored release smaller than capture reads as capture. | Pure geometry tests with explicit tolerance; session gestures at different view scales. |
| 4 | Start captured, advance raw input in small increments past release while observing the snapped output stay fixed until release. Reverse direction and recapture only within capture. Repeated identical raw positions do not oscillate. | Multi-frame raw-pointer sequence, not successive snapped positions fed back as input. |
| 5 | Isolate each target kind and each switch. Check box edges/centres/nine points; group aggregate box; clone's placed box; actual visible guide geometry and anchors; visible path nodes; exact page fractions with nonzero origins. Hidden/inherited-hidden are excluded, locked are included, moving objects and their descendants excluded. | Public target/query tests, group/clone fixtures, actual drag tests and page integration. |
| 6 | Distinct distances choose nearest, exact ties use Nodes → Guides → Objects → Page. Independent line targets may supply x/y. An eligible node is an atomic 2D target and wins both axes when its displacement is no larger than the nearest line-only displacement. A farther node must not overwrite a closer line. | Synthetic target sets with independently computed distances, reversed input order, ghost-target cases below. |
| 7 | Exercise Select move and each resize handle, Node anchor/handle movement, Rectangle/Ellipse/Polygon/Star creation, and Pen first/subsequent placed nodes. Assert resulting geometry, not only an engine response. Shift axis-lock changes only the free axis; rotation preserves existing 22.5° stepping. Other established tool constraints remain valid. | Tool/modifier matrix below through actual session gestures, plus representative browser smoke. |
| 8 | A captured edge/guide/page target produces its actual matching line and moving-point cross in the design token, constant screen width at each zoom. A point-only target must not invent unrelated lines. Final UX §2 explicitly specifies two node-centred axis lines for a node matching both axes; these are permitted, whereas unrelated box/guide lines are not. No labels/tooltips. Release, Escape, target release, suppression and disabling clear markers. | Scene/marker assertions plus browser screenshot review and lifecycle checks. |
| 9 | Ctrl+Alt suppresses snapping throughout every applicable gesture despite enabled settings. Press during capture clears marker and follows raw input; release resumes normal candidate evaluation without a jump from stale held state. Settings do not change. | Actual key transitions during every tool class; modifier combinations with Shift/resize constraints. |
| 10 | Dedicated panel exposes enabled and four target switches plus px fields with every tool/selection; controls remain usable while disabled. Enter/blur commits once, Escape cancels under existing field rules. Release below capture is raised and the displayed value matches stored value. Negative/non-finite entries are refused without history changes. | Browser controls/keyboard tests, settings validation and history assertions. |
| 11 | With 10,000 visible objects, prepare indexed targets from the session read before drag. Per-frame bounded queries avoid whole-scene scans and document rereads. A nearby target late in document order remains discoverable. Compare equivalent snap-on/off drags; added frame cost ≤2 ms thread CPU on the reference machine. | Instrumented query/read bounds or equivalent reviewable seam; dense/late-target tests; repeatable thread-CPU benchmark, no wall-clock threshold. |
| 12 | Snapped move/resize final coordinates equal target within kernel tolerance, including at nontrivial zoom and negative coordinates. One gesture is one history step; undo/redo restores exact geometry. Cancel writes nothing. Held target and marker never persist or create standalone steps. | Session commit/cancel/history tests and persistence comparison. |
| 13 | Original and translated/rotated/overridden clones contribute their resolved boxes and nodes, not their originals' positions. Dragged clone snaps by its own box and retains its link/overrides. Exclude the moving clone without excluding an eligible stationary original or sibling clone. | Cross-page/source-clone fixtures, resolved-bounds tests, actual clone drag/undo. |
| 14 | A current-format file with snapping keys absent reads defaults without new operations, dirty state or rewrite. Wrong types/invalid ranges in stored settings produce `OpenError::Damaged`; intentional release-less-than-capture normalization remains valid. | Current-format missing/malformed fixtures, repeated-read state assertions and no-write check. |
| 15 | Resize/move a page then begin/use the next relevant frame: all six fraction lines reflect current origin/size immediately. Switch active page without changing all-visible target scope. A hidden page contributes none; fractions are derived, never stored. | Multi-page session integration and settings/root-data assertions; no fabricated page-visibility UI if the page model exposes visibility elsewhere. |

## Tool and modifier matrix

For each row run snapping disabled, enabled with a nearby eligible target,
Ctrl+Alt held before press, Ctrl+Alt pressed/released mid-gesture, Escape, release,
and undo/redo. Repeat relevant rows with Shift plus Ctrl+Alt together. Use the
actual tool event path, including final pointer-up coordinates.

| Tool/gesture | Moving feature and expected constraints |
| --- | --- |
| Select, one object | Each box edge, centre line and reference point can align; the selection translates rigidly. |
| Select, multiple objects/group | Features come from the selection's combined box; relative spacing and descendant transforms are preserved. No self-snap against a selected parent, child or descendant. |
| Select, Shift move | Locked coordinate remains exactly locked; only the free axis may snap. A 2D node that conflicts with the locked coordinate cannot break the constraint. |
| Select, resize | All eight handles, opposite-anchor preservation, crossing/flipping behavior and zero/nonzero dimensions follow existing resize rules. Shift aspect lock and any existing centre-resize modifier remain valid. Markers describe the final constrained result, not a rejected pre-constraint snap. |
| Node, anchor(s) | Other anchors of the edited path remain eligible; the moving anchor/selection is excluded. Multiple selected nodes preserve their existing collective-drag rules. |
| Node, handle | Nodes only: nearby box edges, guide segments and page fractions cannot pull a handle. Existing symmetric/asymmetric/angle constraints are preserved. |
| Rectangle/Ellipse creation | Start/end construction semantics and Shift square/circle constraints remain valid; final shape obeys both snap result and tool constraint. |
| Polygon/Star creation | Existing centre/radius/angle and modifier rules remain intact; test both polygon and star rather than treating them as one path. |
| Pen | First click and subsequent click snap; a close-path candidate still follows existing closure rules. Handle creation/angle modifiers retain their existing semantics. |
| Rotation regression | Snap toggle and Ctrl+Alt do not replace or double-apply existing 22.5° rotation stepping. |

## Geometry and candidate traps

- **True guide curve:** cubic Bézier `(0,0), (0,100), (100,100), (100,0)` has
  midpoint `(50,75)`. A nearby pointer at `(50,78)` must snap to the true curve
  near `(50,75)`, not its endpoint chord at y=0 or control-polygon edge at y=100.
  Add a transformed curve, a straight segment, a degenerate segment and an open
  path whose last-to-first chord is not a guide. Use independently evaluated
  curve positions and a stated geometric tolerance; a coarse polyline is not
  the acceptance oracle.
- **Atomic node:** at raw `(0,0)` a node `(3,4)` has 2D displacement 5. Compare
  against line-only displacements smaller than, equal to and larger than 5.
  When the node wins, both result coordinates and marker identity belong to
  that same node. No x-from-node-A/y-from-node-B synthetic point is allowed.
- **No ghost nodes:** nodes `(1,100)` and `(100,1)` are outside an 8 px capture
  radius at raw `(0,0)` even though their separate x/y projections are close.
  With other target kinds disabled, there is no snap to `(1,1)` and no marker.
- **Mixed valid lines:** object edge x=10 plus guide y=20 can produce `(10,20)`
  when independently eligible. Its markers must identify those actual targets,
  never claim the intersection is an existing node.
- **Fractions:** for a page at `(-40,10)` sized `300 × 120`, expected x lines are
  `35,60,110,160,185,260`, expected y lines `40,50,70,90,100,130`. No x=-40 or
  y=10 start-edge target when only Page is enabled. A second offset page remains
  eligible while inactive under the pending question's default.
- **Visibility and identity:** hidden ancestor suppresses descendant targets;
  locked ancestor does not. A selected group excludes every moved descendant.
  Exclude all objects that actually move, including dependent clones following
  a moved original, even if those followers are not directly selected. When only
  a clone moves, its stationary original remains eligible. A stationary clone
  of a selected original remains eligible only when existing transform semantics
  leave it stationary. Duplicate
  coincident candidates and document-order reversal must resolve deterministically.
- **Cache invalidation:** add/delete/move/edit a target, toggle visibility/guide
  status, change clone source geometry, undo/redo, import/open and move/resize a
  page. Include document-revision changes, settings undo and peer updates.
  Derived-cache invalidation is distinct from prohibited per-frame document reads;
  stale held markers must clear/recompute when their settings or geometry change.
  The next drag uses current geometry, and page fractions update on the next
  relevant frame after a page move/resize. Never retain a deleted held-target
  marker or stale bounds. Changes during an active gesture must follow the
  specified session transaction policy without reading the document per frame.

## Settings, history and collaboration

Test all target-kind combinations sufficiently to show each disabled kind is
absent even when it would otherwise win priority. Test zero, fractional, equal,
large finite, negative, NaN and infinity zone values through public commands;
use malformed current-format fixtures for values text fields cannot represent.
Capture growth must not leave release below capture; field display and reopened
state must agree. No-op settings changes and rejected edits create no step.

Settings changes are undoable document edits; ordinary panel navigation and
capture/held/release state are not. Test undo after independent peer edits of
different settings, both merge directions, same-field convergence, save/open and
version validation. Compare stored data before and after a cancelled snapped
gesture to ensure neither target IDs, marker geometry nor page fractions leak
into persistence. Format-number allocation is an integration decision after the
page slice; do not hard-code the specification's stale proposed number.

## Properties entry and visual state

Exercise four independent states: snapping off/panel closed; off/panel open;
on/panel closed; on/panel open. Normal click preserves enabled; Shift+click
preserves the previously open panel. Keyboard focus followed by Shift+Enter or
Shift+Space must provide the UX-defined equivalent toggle without double firing
or changing panel selection. Enter/Space without Shift follows normal activation.
Test event propagation from a child icon, not only the outer entry surface.
The subtle enabled-blue cue must not substitute for accessible enabled state,
focus indication or active-panel indication. Input editing keeps native text
selection/undo; document shortcuts still work from non-input panel controls.

## Performance proof

Construct 10,000 objects with both sparse and dense local neighborhoods. Put the
only eligible nearby target at the beginning and at the end of document order;
results must match. Include many far-away nodes and guides so a fixed candidate
cap cannot accidentally pass. Validate bounded spatial queries around every
moving feature and no repeated document-read/index rebuild in pointer updates;
report missing observability as a test seam, not an assumed success.

Measure thread CPU for matched drag traces with snapping off/on, after separately
recording index preparation. Report machine, build profile, warmup, samples and
incremental per-frame cost; retain the ≤2 ms requirement. Index construction must
not be hidden inside an unmeasured per-frame path. A finite capacity or arbitrary
target-order truncation is not an acceptable optimization. Geometry/query
correctness assertions accompany timings, so an empty or stale index cannot
satisfy the budget by doing no work.

## Execution and reporting

Docs-only planning is complete when this matrix is recorded. Do not claim PASS
before executable acceptance tests, the complete `CLAUDE.md` §7 gate and the
required browser/UX review finish. Later review adds white-box error paths without
weakening this independent oracle. Coordinate exclusive build/test and browser
ownership with the lead; no production changes belong to the tester. Report each
criterion's named evidence, exact commands, failures and unresolved coverage.

## Independent PR #225 feedback plan — snapshot before production review

Prepared 2026-10-30 from amended specification AC 1–18, ADR §7 and UX §10,
without reading the feedback implementation or its diff. Docs-only handoff;
none of the checks below is claimed executed. Earlier acceptance results cover
the earlier scope only. AC9's Select-move rule is explicitly superseded; retain
the prior Ctrl+Alt suppression oracle outside established Select moves.

### Document settings and controls (AC 1, 2, 10, 14, 16)

- Exercise all four axis combinations `(X,Y) = (on,on), (on,off), (off,on),
  (off,off)` through document commands, session and actual panel controls.
  Defaults are both true; editing one flag never enables global snapping or
  changes the other axis/target-kind flags. Both-off is valid while global-on
  remains stored/on in the rail. Controls remain usable while global-off.
- Save/open preserves each combination. Current-format absent axis registers
  read true without operations, dirty state or history. Present malformed
  values (string, number, null or container instead of bool) fail with Damaged;
  do not add old-format migration obligations. Verify the final allocated
  format version against current documents rather than assuming a new bump.
- Each changed flag is one ordinary settings history step; no-op is none.
  Undo/redo restores flags and immediately invalidates incompatible held state.
  Peers changing different axes merge independently in both directions; undo
  one local edit preserves the peer axis. Same-field conflicts converge.
- Browser: visible/accessibility labels Horizontal (X), Vertical (Y), pressed
  states, keyboard activation, native field/global undo and unchanged rail
  navigation. Labels are the documented safe default pending the customer's
  optional wording answer. Coordinate semantics are explicit: X correction
  ordinarily produces a vertical reference line, not a horizontal one.

### Reachable geometry, not clipped output (AC 3–8, 12, 16)

Use independently calculated raw point `(99,99)` and line targets x=100/y=100:
expected results are `(100,100)`, `(100,99)`, `(99,100)`, `(99,99)` for the four
masks above. Disabled coordinates remain raw to an explicit kernel tolerance;
no held target or marker represents a disabled correction. Both-off produces
no marker. Repeat capture, hold and release from raw coordinates at several
screen scales, and change mask while held at the same pointer position.

- Atomic node `(100,100)` is reachable with both axes. With X-only it is
  unreachable from `(99,99)` even though its X is near. Node `(100,99)` is
  reachable with X-only; node `(100,999)` is not. Mirror for Y-only. Verify full
  spatial capture/release eligibility and absence of synthetic projections.
- A finite diagonal guide from `(90,90)` to `(110,110)` intersects X-only motion
  through raw `(99,100)` at `(100,100)`, leaving raw Y=100 unchanged. Repeat with
  swapped axes, curved Bézier intersections and a shortened segment that cannot
  intersect the allowed motion line. Never extrapolate endpoints or substitute
  closest unconstrained projection. Mixed guide/box composition obeys the mask.
- Shift-locked moves intersect axis mask with the free coordinate. When only
  the locked axis is enabled, snapping cannot change the move. Compare actual
  committed geometry to an equivalent unsnapped constrained gesture.
- Aspect-constrained diagonal resize cannot snap if its required coupled
  correction changes a disabled axis. Test ordinary and mirrored resize;
  preserve opposite/centre anchor and aspect ratio. A permitted edge-only
  resize may snap on its enabled axis. Preview and release agree, including
  release at a new raw position and changed final modifiers.
- Exercise actual creation geometry with axis masks for Rectangle, Ellipse,
  Polygon and Star; default dimensions, centre/corner reference and existing
  Shift constraints remain as before. Node anchor/handle interactions retain
  existing eligible-target rules; disabled axes cannot be restored by nodes.

### Temporary Alt inversion during an established move (AC 9, 12, 16)

First select and begin a genuine object move without Alt, away from parameter
handles. For each stored-enabled value, place raw moving geometry just inside
capture and record its unsnapped position. Press/release Alt **without another
pointer event**: stored-off must snap only while held, stored-on must stop
snapping while held, then restore from the unchanged raw input. Check live
preview and marker as well as final geometry; a release with changed modifiers
uses that event's effective state. Repeat beyond capture but inside a previously
held release zone to expose stale held-state reuse when re-enabling.

- Modifier changes never write settings/history, alter the rail badge/panel,
  create geometry or change selection. One completed move remains one history
  step; Escape restores geometry with no step. Save/read during preview never
  persists effective-enabled, held target or marker state.
- Ctrl+Alt during an already-running move uses the same XOR truth table and
  creates no copy. Ctrl without Alt retains existing copy behavior. Check
  Shift+Alt and Shift+Ctrl+Alt with both axis masks: only free/enabled movement
  may snap. Alt never enables a disabled target kind or disabled axis.
- Both axes off remains unsnapped under Alt inversion. End/cancel the move,
  clear modifiers/focus and start a new gesture: no inversion is latched.
- Alt at initial Select press still starts the existing lasso/selection action,
  rather than moving the hit object; overlapping Alt-click behavior remains.
  Resize, Node, Pen, Offset and idle creation hover do not receive move-only
  inversion. Existing Ctrl+Alt bypass and tool-specific modifiers remain in
  those contexts. These are preservation tests, not new interaction scope.

### Idle hover, first press and placement (AC 7, 8, 11, 17, 18)

For each of Rectangle, Ellipse, Polygon, Star and Pen, start with no gesture or
button held. Send hover near an eligible target and assert a marker, corrected
prospective point and no object/history/register change. An idle hover must not
claim an active drag. Stored-off plus Alt alone remains off; move inversion is
not global. Off-target hover releases using raw capture/release geometry.

- First press after hover agrees with the current resolved point. Also press
  with **no prior hover**, and hover at A then press at distinct raw B: resolve
  the press event, never consume stale A. Repeat after settings/mask or target
  changes. Start a creation drag at the corrected press and assert final shape
  against independently calculated or equivalent exact-coordinate geometry.
- Pen: prospective first node and later rubber-band endpoint snap without a
  button down; ordinary click places the displayed point without spurious
  handles. Test click-only raw motion below threshold with attraction larger
  than the click threshold: snapping must not invent a handle/drag. Existing
  continue/join/close and angle constraints retain precedence; live preview
  endpoint is not its own target.
- Shape simple click enters the existing typed-size placement flow (0052), not
  an immediate unsolicited object. Verify snapped initial anchor, unchanged
  default/typed dimensions and placement reference. Follow with pointer-based
  placement preview, click confirm and keyboard confirm in separate cases;
  all resolve the actual prospective coordinate and use current masks/state.
  Typed exact coordinates remain exact. Preview adds no history; final accepted
  creation is one normal step, undo/redo exact, cancellation none.
- Lifecycle, separately for hover and active placement: leaving canvas, Escape,
  tool change, document replacement/open, disabling global snapping, both axes
  off, capture/focus loss and target removal clear incompatible markers promptly.
  Pointer-up clears the old drag marker; a later real hover may create a new
  marker. Moving into settings creates no artwork. Re-entry starts from current
  raw input and current targets, without a stale capture or extra history step.

### Cache and performance proof (AC 11, 17, 18)

Use 10,000 visible objects with a nearby target late in document order. Measure
1000 no-button hover frames versus equivalent raw hover with thread CPU, after
separately reporting initial index construction. Keep the incremental <=2ms
budget; assert nonempty correct results so an empty/stale cache cannot pass.
Consecutive unchanged hover events must reuse the indexed session read, and
hover-to-press promotion must not rebuild solely because a button went down.
Document/target changes rebuild once when necessary; settings, mask, scale and
modifier changes re-resolve correctly without per-pointer full document reads.
Prefer an existing observable cache/read seam; if absent, identify exactly what
is structurally reviewed versus measured rather than claiming instrumented proof.

### Execution handoff and reporting

After explicit exclusive-slot handoff, add executable acceptance independently
from this snapshot, retain the original 22 tests except narrowly justified AC9
amendments, and send real defects to implementer/lead before changing any oracle.
No production edits by tester. Final report maps all 18 criteria to actual test,
browser, review and performance evidence, distinguishes historical evidence and
new verification, records exact commands/counts and states remaining gaps.

### Post-plan review-derived regression additions

After executable handoff, the architect supplied a scale-invariance defect in
constraint compatibility: proportional direction vectors must not allow drift
on a disabled axis merely because their unnormalized components are small.
Independent public engine coverage now tests both masks at scales 1e-8/1/1e8.
This is white-box-derived supplementary coverage, not part of the pre-code oracle.

UX also identified Ctrl+Alt move feedback claiming Copy despite no copied object.
Public acceptance reproduced the plus-badge failure before the fix and now checks
both badge and live readout, Ctrl-only restoration and initial Ctrl+Alt lasso's
remove badge. The original commit/history assertions remain intact.

## Independent creation-Alt extension plan — pre-code snapshot

Prepared 2026-10-30 from the customer's latest clarification and amended AC9,
16–18, before reading this extension's production changes. This supersedes the
previous plan's creation-hover stored-only and creation Ctrl+Alt-suppression
oracles. Select initial-press/lasso, Select resize, Node and other unrelated
modifier contracts remain unchanged. No new document field, format or dependency
is requested. Prior gate results do not verify this extension.

### Tool/phase/state matrix

Exercise Pen, Rectangle, Ellipse, Polygon and Star separately with stored-enabled
false and true. In every eligible phase assert effective-enabled = stored XOR
held-Alt, both with Ctrl absent and present; do not apply the old Ctrl+Alt bypass
in these creation contexts. Preserve each tool's existing Ctrl/Shift constraints
by comparing to an exact-coordinate unsnapped reference with identical modifiers.

| Phase | Independent oracle |
| --- | --- |
| Idle hover, no button | Hover once at raw near target, then Alt down/up without pointer motion. Prospective point and marker alternate between raw and target as dictated by XOR, including default stored-off. No drag/history/artwork/settings change. |
| First press, no earlier hover | Set modifiers before first press; resolve the actual press coordinate under XOR. Both Alt and Ctrl+Alt must work without a cached hover. Pen first node and shape placement anchor reflect the permitted axes and targets. |
| Simple click and button-up placement-follow | First click pins its resolved start. Follow to a second raw point, then Alt down/up without motion. Only following candidate changes; placement preview and final second-click/Enter result match the effective state. |
| Button-down creation drag | Press fixes the resolved start. Move to candidate, toggle Alt with stationary pointer, then release under either held state. Preview and release agree; pinned start never moves when snapping toggles. |
| Pen after placed anchor | First/earlier nodes stay fixed; rubber-band endpoint updates on key-only changes. Next click commits the currently resolved endpoint, without accidental handles, duplicate anchors or extra history. Existing close/join/continue and angular constraints retain precedence. |
| Frozen typed-size placement | Freeze valid typed dimensions/coordinates after initial anchor. Repeated Alt/Ctrl+Alt transitions and subsequent hover cannot move the pinned anchor or typed geometry. Confirm yields the same exact geometry as the unsnapped frozen reference. |

Concrete page-only oracle: raw first point `(53,75)` resolves to `(52.5,74.25)`
when effectively on; raw second point `(104,149)` resolves to `(105,148.5)`.
Test both inversion directions and release restoration without feeding snapped
output back as raw input. Hover at A then press B also recomputes B. For shape
kinds whose placement reference is centre/radius rather than opposite corners,
compare the complete resulting geometry with the same tool fed exact reference
coordinates, not an invented shared rectangle convention.

### Fixed state, masks and lifecycle

- During a drag or placement, record the pinned start before each key-only
  transition and assert it remains identical afterward. For Pen record prior
  anchors and handles. Do not merely assert a marker changes while committed
  geometry accidentally jumps.
- Repeat representative phases with X-only, Y-only and both axes off, and with
  the nearby target kind disabled. Alt cannot re-enable disabled axes/kinds.
  An unreachable node remains unreachable; current tool constraints still bound
  movement. Both axes off yields no correction or snap marker under either Alt
  state. Maintain full two-dimensional point eligibility and finite-guide rules.
- Read stored settings, history and serialized document before/after hover and
  modifier transitions. Panel enabled/rail blue state remains stored state,
  never temporary effective state. No copy badge/readout, extra object or history
  entry is introduced by Ctrl+Alt creation. Final accepted creation is one step;
  undo/redo restores exact geometry, cancellation creates none.
- Losing capture, Escape, leave, tool/document change and modifier release clear
  incompatible held state. Starting the next gesture must not retain inversion.
  Releasing Alt with a stationary pointer restores stored behavior immediately;
  entering/releasing beyond capture may not resurrect a stale held target.
- Keep the previous Select move inversion/no-copy regression and initial Alt
  lasso preservation. Explicitly preserve non-creation resize/Node suppression
  and tool behavior; do not widen the customer's scope by changing those tools.

### Verification and evidence ownership

At executable handoff, inspect and narrowly amend existing creation suppression
and stored-only hover tests to the new explicit AC9 contract. Retain assertions
for geometry, constraints, history, cancellation and persisted state; document
why each changed oracle is now wrong under the customer's clarification.
Add public Session tests for same-pointer hover/drag/follow refresh and fixed
starts, and real browser key sequences for visible marker/panel behavior and
creation result. Benchmark/index assertions remain applicable; key-only refresh
must reuse the current context rather than introduce a document read per event.
Run focused tests before the full gate, coordinate the sole build/browser slot,
and report actual coverage separately from any unexercised matrix combination.

### Creation-extension implementation handoff observations

Lead's actual browser found C3 before handoff: the focused Width placement field
stopped Alt key propagation, so button-up following width stayed52.2 despite
Alt. The corrected PlacementChip modifier propagation is exercised by the
reusable browser script; public Session tests alone cannot prove DOM routing.
This is explicitly browser-derived supplementary coverage after the plan.

During independent lifecycle testing, Escape returned the idle drawing tool to
Select as designed. The test now asserts that transition and explicitly re-enters
the original tool/mode before subsequent creation-Alt assertions. A separate
fresh hover→disable→Alt test verifies context rebuilding without Escape; it
passed without a production change. No Escape behavior was weakened.
