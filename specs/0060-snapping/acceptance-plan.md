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
