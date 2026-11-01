# Independent acceptance plan: customer interaction revision

Status (2026-10-31): **bounded revised functional/runtime acceptance PASS; full final gate pending root GO**.
Independent strict complete-host CPU: cold7.890 ms, all19post-first maximum7.921 ms;
actual Native/Node-WASM chain/conics/OnAxis/PerpendicularAt/suffix/tangent parity PASS.
The real standalone Arc host failure is now independently fixed; unchanged two-case
repro, context-switch and normal dimension double-click regressions PASS. Focused
changed-test Clippy and formatting PASS. Details, historical REDs and justified oracle
corrections follow below. This is not a completed full-gate or customer-acceptance claim.
The original pre-source plan follows; executed evidence is recorded at the end. This plan was derived from the
revised specification AC1–23, revision-ux-plan.md and ADR0026 before reading changed
production source. Previous PR228 test/CI results do not accept this revision.
No build, test or browser was run when this plan was written. The lead's observed right-button
arc creation and Tab-to-Workbench failures are regression inputs, not findings
independently reproduced here. Public API support will be inspected at handoff;
no guessed APIs or production edits are introduced now.

## Independent oracles and execution boundaries

Use new revision-specific acceptance files, preserving previous evidence as history.
Where old tests assert superseded multi-sketch/snapping behavior, identify the exact
customer-authorized contract change instead of silently weakening their assertions.
Actual browser input must exercise real mouse buttons, keyboard focus and saved
files; Session tests alone cannot prove event routing. Decode saved documents through
the public codec and check numeric geometry, IDs, relationships, ownership and history.
No source-text checks or entity-kind existence alone establish behavior.

Local analytic comparisons use explicit tolerances: 1e-6 mm for expected coordinates,
lengths and native/WASM parity, dimensionless normalized cross/dot residual <=1e-8
for tangent/perpendicular witnesses; conversion remains <=0.01 mm in final coordinates.
Browser screen/document mapping is measured from the actual camera; use deliberate
exact numeric inputs for geometric known answers. UI tolerance tests use the exposed
screen-space inference boundary, not an invented customer threshold. Repeat at two
zooms and offset page locations to distinguish local-axis inference from global axes.

## Bounded acceptance matrix

| Case / criteria | Journey and required observable result |
| --- | --- |
| R01 — AC1/4/11/18/19 | On a fresh page, choose Sketch directly. No New/picker; empty editable local graph is available without object/history writes. Hover, tool/mode changes, enter/leave and cancelled first picks remain inert. First valid segment atomically creates exactly one owner, fixed origin, graph/checkpoint and named history step. Undo removes object+claim together; redo restores the same IDs. Vector sees one object; re-entry restores graph. |
| R02 — AC4/19 | Page origin(300,-40), size200×100 gives initial sketch translation(400,10) and local origin(0,0), independent of pan/zoom. A local(10,20) point lands globally(410,30). A differently sized second page owns a distinct graph; switching during pending picks/numeric entry cancels only unfinished input and clears entity selection. Returning cannot commit stale input to either owner. |
| R03 — AC4/10/11/19 | Resize does not silently recenter the existing graph. Exercise ordinary page-content movement separately and assert its established transform applies exactly once. Delete all geometry retains existing owner. Delete/convert original clears only its own current claim; first subsequent creation establishes a new owner. Delete page retains artwork and never assigns its owner to a nearby overlapping page; undo restores association. Roundtrip preserves associations and local/global distinction. |
| R04 — AC9/10/11/19 | Copy original to empty page remaps graph IDs and creates independent owner; occupied-page paste refuses atomically with explanation and no history. Multiple derived clones remain allowed without owner claims; clone entry resolves original. Two peers concurrently materialize one page: merge order converges on one editable owner; losing original retains intact graph as diagnosed visible non-owner. No read/entry repair writes or silent merging. Undo/delete/recreate races cannot clear a peer's replacement owner; clones of either original stay resolvable. |
| R05 — AC1/14/15/20 | Actual gesture completion for Point, Line, Rectangle, Arc, both Circle/Ellipse variants and Slot. Check known geometry, not names alone: rectangle30×20, circle radius5, noncircular ellipse semiaxes10/4, slot spacing20/width6. Three-point degenerate/duplicate picks refuse completion but retain earlier correct picks. Slot boundaries connect and satisfy tangent/width/spacing constraints; values remain editable. |
| R06 — AC13/20 | One uninterrupted polyline commits line→tangent arc→line→perpendicular line and exercises both perpendicular-arc sides. Each confirmed segment adds one named step, shared endpoint IDs and the explicitly requested relation. Alt press cycles once; repeated keydown does not cycle again; release/repress does. Mode changes never commit or lose pending anchor. Top-bar choices produce equivalent geometry/relations. Compare tangent direction with arc radius at the join, perpendicular dot products, and expected side signs; do not use solver status as sole oracle. |
| R07 — AC13/22 | Resume an explicitly clicked eligible open line endpoint, then arc endpoint; visible hover and discoverable selected-endpoint action lead to the same identity reuse. No near-coincident duplicate point. Junction does not guess a branch. Click chain start closes with shared identity and closed output; Enter finishes open only outside numeric confirmation. Keep customer's unreproduced one-segment sequence open: baseline four clicks/three lines is insufficient; cover mixed modes, numeric confirm, finish/resume and tool re-entry. |
| R08 — AC11/13/16/20 | Cancellation table below covers canvas, numeric draft, held drag, each multi-pick phase, valid Fillet preview and invalid corrected draft. Escape/right never commits. Right press/release/contextmenu is one cancellation and never a left pick; test Arc start→right second→left third specifically. Completed polyline segments remain, live segment/first anchor disappears. Subsequent Escape returns Select. Idle right menu remains. Switch tool/workbench/page cancels unfinished state only. |
| R09 — AC3/13/20/23 | Tab from active canvas enters relevant numeric field with value selected; never Workbench. Tab/Shift+Tab cycle visible order: line length/angle, rectangle width/height, circular radius/angle where relevant. Enter valid30mm/45° line creates expected vector(30/√2,30/√2) once and continues appropriately. Invalid expression keeps focus/error/last valid geometry. Native input undo edits text without undoing document. Escape discards draft/restores canvas preview first, next Escape cancels construction. Right-click canvas cancels draft+construction. |
| R10 — AC21 | Sketch result is invariant across stored Vector snapping enabled false/true, four X/Y masks and each target kind, with nearby object/page/grid/guide candidates deliberately distinct from raw pointer and local axes. Cover drawing idle preview, first press without hover, held drag, no-button continuation and selected-geometry move. No Vector snap marker/held-Alt inversion in Sketch. Vector behavior/settings persist unchanged after switching back. |
| R11 — AC11/21 | Near local X axis preview proposed y=0; near Y preview x=0; intersection both. Cancel adds no constraint/history. Confirm persists inspectable solver-valid axis relation in the same creation step; editing/removing it is undoable. Off-axis input gets no arbitrary object/grid/angle inference. At two zooms verify screen-space boundary, offset page/local frame and axis feedback; fixed origin, intentional endpoint reuse and chosen-mode relations remain distinct mechanisms. |
| R12 — AC13/20/21 | Alt modes in each applicable drawing phase, with canvas and numeric focus: one press, repeat, release, Ctrl/Shift combinations, AltGr and system chords. Mode cycle retains pending input and has no persisted setting/history effect. Native text/system shortcuts remain scoped; established Select Alt behavior is unchanged. Do not assert any mandatory M alias. |
| R13 — AC2/17/18/22 | All ten named constraints discoverable in grouped left tools, settings only in single top bar. Eligible preselection seeds picks; incompatible member remains visible and prevents silent partial apply. Empty preselection collects references without dragging them. Independent known-answer witnesses for coincident, H/V, parallel/perpendicular, tangent, equal, fixed, symmetric, midpoint; verify relationship ID, participant highlight and one history step. No creation-button wall in right inspector. |
| R14 — AC3/17/20/23 | Line→length, two points→distance, two lines→angle, circle/arc→explicit radius/diameter choice; no-selection interactive picks equivalent. Place annotation then enter value/expression. Circle radius editing is directly discoverable. Existing row/annotation double-click and keyboard edit same relationship ID, never duplicate. Reference shows current measured read-only value, not nominal draft; switching driving is explicit. |
| R15 — AC3/7/10/11/18 | Separate fx Parameters tab owns names/expressions/units/resolved values; width80, margin10, spacing=width−2margin gives60, width100 gives80 in actual saved geometry and clone. Rename retains references. Unknown names/cycle/unit mismatch preserve last valid values/shape, identify affected entry and retain focus; correction works with native text undo. Sketches retains diagnosis/links, not duplicate parameter editors. Roundtrip and undo restore formulas/IDs. |
| R16 — AC5/7/17/22 | Fixed valid geometry has truthful zero DOF; diameter-only free-centre circle is not fully constrained and can translate. Duplicate compatible constraint gives redundancy, contradiction retains last valid geometry with inspectable/removable IDs, numerical failure never falsely becomes conflict/zero. Selected relationship highlights participants; compatible edits preserve selection, deletion clears only nonexistent IDs. Diagnose and repair via keyboard/canvas/row with one step and undo. |
| R17 — AC22 | Point versus edge versus annotation distinct, empty unfilled interior clears rather than selecting distant outline, Shift toggles, marquee containment/crossing and Ctrl removal match existing semantics. Overlapping tied hits can be cycled without moving geometry; point precedence only inside point tolerance. Constraint picking has same eligibility/hit ordering, independent of draw order. Check selected IDs plus visible feedback and unchanged packed geometry/history. |
| R18 — AC8/9/10/11/16 | Fillet valid preview→invalid radius→Commit refused; correction commits once with retained/remapped original constraint intent. Cancel restores corner; undo restores topology. Convert selected owner to paths preserves ID/style/placement and clones, clears ownership, switches usable Vector selection; undo restores editable owner. Closed outlines and scaled-clone approximation <=0.01mm; no origin/annotations/construction geometry in output. Original scaling remains undecided and untested as a new policy. |
| R19 — AC1/2/3/5/18/23 | Actual browser shell at normal/narrow height and collapsed pane: styled existing Workbench control above Properties; Sketches and distinct fx tab; grouped left tools; exactly one contextual top bar; no native demo selector, extra header/breadcrumb/Finish bar, second toolbar or bottom state strip. Keyboard focus visible; workbench switching preserves collapse/camera. No ordinary Pen/Star/Polygon copied into Sketch. Save screenshots only from working production UI, clearly separate from static storyboard. |
| R20 — AC6/12 | Reuse bounded independent100-entity connected chain and independent-components inputs, retaining all hard lengths/fixed references and meaningful closest-feasible movement. Re-execute actual revised Session pointer/view/draw release thread CPU and production browser complete-frame trace if changed dispatch/render risk; warmed<16ms, cold explicitly separate, no idle subtraction. Replay identical native/actual Node-WASM full-precision chain and nonlinear conic cases <=1e-6. Previous timings/parity are baseline evidence, not an automatic revision PASS. |

## Cancellation/host-event cross-section

For each applicable tool, enumerate zero picks, first anchor, intermediate picks,
hover preview, held left drag, numeric draft (valid/invalid), and ready preview.
Use fresh sketches when testing no-first-object writes; use an existing completed
segment when testing retention. At each phase capture owner/object count, graph IDs,
geometry, constraints, undo length, active tool and focused element before/after.
Run Escape, then subsequent Escape separately. For right button deliver down, up
and contextmenu and assert no extra transition between them; the next valid left
sequence must still need exactly the proper picks. Include right-down while a left
creation drag is active and numeric focus on the canvas-context cancellation path.
Do not simulate right input by invoking a left-click Session method.

## Evidence handoff and unresolved boundaries

Each row will receive exact test/script names, executed counts and artifact paths at
handoff. Suggested new files: `acceptance_0066_revision_document.rs`,
`acceptance_0066_revision_session.rs`, and browser `sketch-revision.cjs`; create them
only against verified public APIs, not invented signatures. Keep all generated data
under `target/0066-revision-*`. Build/browser execution awaits explicit slot handoff.

ADR0026 owner-register/conflict/copy details remain marked Proposal and are the
reviewable architectural oracle, not independently invented product scope. Source
scaling remains a customer decision; no inference from clone transforms. Customer's
specific one-segment/cancellation reproduction remains open despite one successful
basic sequence. Only the bounded document milestone below has since executed; no revised Session/browser acceptance is claimed.


## Independent document milestone — PASS, 2026-10-30

Latest customer direction now requires a dedicated source page and Linked clones for
composition. Earlier R04 independent-copy wording is retained only as the prior
proposal history: it is **not** a requirement for the normal composition workflow.
Raw unowned creation/lower-level duplication infrastructure may remain available;
this milestone does not demand a second editable original on a composition page.
Session entry into a populated page (Q7) and all new UI flows remain outside this pass.

New independent `curvyo-document-core/tests/acceptance_0066_revision_document.rs`
contains9 executed public-API cases derived from R01–04/R10 before reading producer
assertions. Public ownership API signatures/contract and existing independent test
conventions were inspected; no producer test assertions were used as oracles.

| Case | Executed evidence |
| --- | --- |
| R01/R02 | First owner is one named history transaction; page offset plus half size positions both localzero and knownendpoint; undo removes claim/object, redo/reopen preserve IDs/graph/placement. |
| R01 refusal | Occupied or missing page refuses without version/history changes or graph replacement. |
| R04 clone | Clone owns no page; source link and edited45mm endpoint coordinates agree with source; target page stays ownerless on roundtrip. |
| R04 collision | Two peers create distinct graphs; exchange converges on one owner, both graphs survive intact, loser is diagnosed and rejects mutation without writes; deleting/undoing loser cannot clear winner. |
| R03 lifecycle | Delete/undo/redo/recreate and object-only undo/redo restore or clear the correct owner. Conversion clears claim; object-only history restores same graph/owner and redoes conversion. |
| R03 page | Page deletion retains sketch artwork with non-owner diagnosis; page undo restores association. Resize with no content transform retains placement; explicit content move shifts every anchor exactly once. |
| R10 format28 | Valid owner golden opens/reopens; malformed present claim and assignment goldens both specifically return OpenError::Damaged. |

Exact affected commands/results:

```sh
cargo test --offline -p curvyo-document-core --test acceptance_0066_revision_document
# PASS9, target/0066-revision-document.log
cargo test --offline -p curvyo-document-core --test acceptance_0017_block1_tester every_older_fixture
# PASS1, target/0066-revision-inventory.log
cargo test --offline -p curvyo-editor-wasm --test acceptance_0020_tester_more c36_old_files
# PASS1, target/0066-revision-history.log
cargo clippy --offline -p curvyo-document-core --test acceptance_0066_revision_document --test acceptance_0017_block1_tester -- -D warnings
cargo clippy --offline -p curvyo-editor-wasm --test acceptance_0020_tester_more -- -D warnings
cargo fmt --all --check
# All PASS; target/0066-revision-{clippy,history-clippy,fmt}.log
```

Fixture glue adds only exact three new filenames to current-header classification
and exact two damaged names to strict Damaged assertions; pre-existing legacy
coalescing, valid-file style/geometry assertions and unknown-label checks remain.
An initial new test compared entire clone/source PathSnapshots, including distinct
object IDs. The corrected oracle checks the clone source link, identical anchor
geometry and independently known coordinates; this was a test precondition error,
not a production defect. All nine final cases passed unchanged production.

Gaps intentionally left for the next milestone: lazy Session entry/first cancel,
source-page navigation and populated-page choice, hostile raw sync import beyond
packed damaged-file rejection, additional concurrent undo/recreation race permutations,
actual browser event routing, numerical/render performance and native/WASM replay.
No full workspace gate was run and no acceptance of the revised whole feature is implied.

## Independent Session/axis milestone — RED, 2026-10-30

New `curvyo-editor-wasm/tests/acceptance_0066_revision_session.rs` was derived from
R01/R02/R08/R10/R11, using current public Session APIs without producer assertions.
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_revision_session`
ran5 tests:4 passed,1 failed (`target/0066-revision-session.log`).

- PASS: empty direct entry, hover/draw-list/view reads and Arc cancellation/navigation
  create no owner/history writes.
- PASS: Arc first/second-pick phases cancel with the actual `sketch_secondary_press`
  API or Escape, preserve no partial geometry, consume secondary once and require a
  fresh valid pick sequence. No host right-button routing claim.
- PASS: Vector enabled false/true ×four axis masks do not alter off-axis point
  creation or free-point drag; stored settings stay unchanged (8 combinations).
- PASS: proposed6-screen-pixel band checked at4px inside/8px outside and two zooms;
  local X/Y/intersection infer exact y0/x0/both with persisted OnAxis constraints.
  Off-axis input gets no constraint; preview draw/view creates no history and save/open
  retains graph. Numeric residuals use1e-6mm. Rendering presence alone does not prove
  the host's visible inference mark.
- **RED R02:** empty direct Sketch entry→Point→numeric(10mm,20mm) refuses with
  `Create or select a sketch first`. Repeated with actual hover before completion:
  same failure (`target/0066-revision-session-numeric-red.log`). Expected lazy first
  owner/geometry commit. Later numeric-circle assertions have not executed and are
  not claimed PASS. Independent oracle unchanged; producer fix requested.

Focused new-test Clippy with `-- -D warnings` passed; rustfmt applied. Initial test
compile correction used Result::is_ok_and (not Option::is_some_and); a lint-only
variable-name correction followed. No production edits or full gate. Tester released
slot for fix. Page switching/source-clone entry and remaining UI/mode/Tab milestones
are still pending; old New/global-origin/Alt-snapping tests are not rewritten yet.

### Session/axis correction handback — PASS8/8, 2026-10-30

Producer fixed lazy numeric/parameter first-commit routing. The independent R02
then reached its previously unexecuted final world-coordinate assertion: that
assertion incorrectly expected a standalone Point among exported path anchors.
The established contour contract omits standalone/definition points. Only this
oracle was corrected to find the solved analytic Point and transform it using
snapshot.placement, preserving exact world(centre+(10,20)), local coordinates,
circle centre/radius5 and undo counts1/2. No output-semantics change was requested.
The original lazy-create refusal was a real production defect; this later assertion
was a test error. Neither is retrospectively reported as an earlier PASS.

The complete revised Session suite now actually passes8/8:

- Prior empty/secondary/axis/settings cases remain intact.
- R02 numeric Point first commit and subsequent picked-centre numeric Circle pass.
- Page switching cancels pending Arc picks and cannot commit stale input to either
  page; previous owner's graph is retained and switching adds no history.
- Entering a Linked clone from the composition page selects the source page and
  original, without history or a second ownership claim.
- Invalid first parameter leaves no object/history/version writes; valid width80mm
  atomically creates owner+parameter once and survives save/open at80mm.

Executed commands:

```sh
cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_revision_session
# 8 passed; target/0066-revision-session-final.log
cargo clippy --offline -p curvyo-editor-wasm --test acceptance_0066_revision_session -- -D warnings
# PASS; target/0066-revision-session-clippy-final.log
cargo fmt --all --check
# PASS; target/0066-revision-session-fmt-final.log
```

No host WASM/TypeScript/browser or full-gate PASS is implied. Mode cycling, Tab,
constraint/dimension picking, selection and fx remain subsequent milestones.
Keep old tests until bridge semantics stabilize; specifically reconcile
`acceptance_0066_interactions::s01_navigation_and_layer_new_have_exactly_one_step`,
`acceptance_0066_lifecycle::s01_group_new_one_step_and_inert_layer_refusal` (old New
operation), and `interactions::s11_rectangle_key_only_alt_preserves_anchor_axes_and_history_in_both_phases`
(old Sketch Alt-snapping) against the explicitly revised customer contract.
Their shared New/global-coordinate setup and lifecycle origin/body-drag coordinate
oracles also require targeted frame-aware review; no bulk assertion rewrite done.

## Independent mode/metadata/selection milestone — PASS, 2026-10-30

Executed bounded native cases from the previously derived continuation corpus:

| New/current target | Result and independent scope |
| --- | --- |
| sketch-core `acceptance_0066_revision_modes` |4/4 PASS: all six modes with exact endpoint/centre/radius/directed-sweep witnesses and shared IDs; outward START/END tangents for both lines/arcs; all three curved modes retain relation IDs and exact analytical endpoints after radius6→10 with explicit fixed endpoint-x reference; mixed chain closes on existing start ID. |
| document-core `acceptance_0066_revision_document` |11/11 PASS (prior9+2): NaN/infinite/missing dimension label refuses before any version/history mutation; valid/no-op label edits preserve graph, solution and checkpoint; roundtrip exact local position; lower-level copy remaps label keys to actual copied constraint IDs; constraint delete/undo restores labels; valid/malformed label goldens distinguished. Infrastructure copy test does not authorize independent-copy composition UI. |
| editor-wasm `acceptance_0066_revision_session` |11/11 PASS (prior8+3): typed tangent arc radius6 and subsequent tangent line length8 then mixed closure, Alt once/repeat/blocked-input/Ctrl scoping, standalone Arc from Line then Arc endpoint identity and one-step undo/redo, nearest point/edge hit, Shift-add and Ctrl-removal marquee preserving geometry/history. |
| Exact fixture sweeps |document0017 `every_older_fixture`1/1; editor0020 `c36_old_files`1/1. Only explicit valid current-header label filename and malformed-label filename added; all legacy assertions retained. |

Exact commands (repository root):

```sh
cargo test --offline -p curvyo-sketch-core --test acceptance_0066_revision_modes
cargo test --offline -p curvyo-document-core --test acceptance_0066_revision_document
cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_revision_session
cargo test --offline -p curvyo-document-core --test acceptance_0017_block1_tester every_older_fixture
cargo test --offline -p curvyo-editor-wasm --test acceptance_0020_tester_more c36_old_files
cargo clippy --offline -p curvyo-sketch-core --test acceptance_0066_revision_modes -- -D warnings
cargo clippy --offline -p curvyo-document-core --test acceptance_0066_revision_document --test acceptance_0017_block1_tester -- -D warnings
cargo clippy --offline -p curvyo-editor-wasm --test acceptance_0066_revision_session --test acceptance_0020_tester_more -- -D warnings
cargo fmt --all --check
```

All final commands passed. Logs: `target/0066-revision-modes.log`,
`0066-revision-labels.log`, `0066-revision-session-modes.log`,
`0066-revision-label-{inventory,history}.log`,
`0066-revision-{modes,label,modes-session}-clippy.log`,
`0066-revision-modes-fmt.log` (all under target).

One test-harness correction: the standalone-Arc undo test initially sent Ctrl+Z
and assumed immediate completion. The actual API returned `UndoBusy`, not failure;
its documented follow-up `Session::undo()` (and corresponding redo) is now performed
before comparing complete before/after graphs. No assertion was relaxed and no
production fix resulted. A helper's join/end arguments were grouped for Clippy;
geometry assertions are unchanged. No producer test assertions supplied the oracles.

Still unexecuted here: actual host Alt/Tab/native text focus, all six mode setter
paths through the visible menu, every resume endpoint through pointer hit routing,
full marquee/lasso/tie-cycling cross-product, rendered label placement/reference text,
WASM parity/performance and full gate after revision. Known host Radius-preview
wording remains queued, not classified as a core failure. The two-empty-virtual-page
numeric-draft case below is mandatory for actual browser handoff: ownerNone→None
page switch must clear draft/error/focus context, with saved geometry/history proving
no leak. Session cancellation alone does not prove React draft reset.

## Independent revised host milestone — scoped PASS11 journeys, 2026-10-30

Exclusive local fallback used because no remote tester VM is available. The lead
explicitly transferred the sole browser/build/test slot; all browser contexts and
browser processes launched by this batch were closed. Production preview1421 remains
available for the next owner. No performance run or full gate in this milestone.

Two new tracked independent scripts execute actual menu/mouse/keyboard events and
inspect downloaded `.curvyo` document.json, without hidden application API calls:

- `frontend/tests/browser/sketch-revision-independent.cjs`:10 journeys PASS.
  All six visible mode-menu choices produce exact line or circle/arc geometry,
  independent centre/radius/directed quarter-turn/tangent witnesses and shared
  endpoint IDs. Scale is derived from the actual saved first100-screen-pixel line;
  all subsequent screen deltas map through that known scale. Non-free modes retain
  persistent relationships. No existence-only arc assertion.
- Actual Tab→Length, Tab→Angle, Shift+Tab→Length focus; two Escape presses cancel
  first construction without creating an object. A new numeric30mm/45deg line saves
  exactly(30/√2,30/√2) endpoint delta. Right-button down/up/contextmenu at Arc pick
  phase1 or2 followed by one left click cannot finish an arc or create an owner.
- Two empty pages with no owner: invalid numeric draft on first cannot leak into
  second; second starts empty fields, valid(7mm,9mm) commits exactly one saved Sketch
  with that analytic point. This checks actual DOM values and saved geometry;
  it does not independently count the host history rows.
- `frontend/tests/browser/sketch-revision-source-page.cjs`:1 journey PASS.
  Create circle, return Vector, Linked clone, activate another page, enter Sketch:
  active page returns to source and saved document contains one original+one clone.
  Actual rendered page-frame centre is(575.984375,500.0078125) in1440×1000 viewport,
  safely in usable canvas rather than a sliver. Middle-button pan shifts frame by
  (80,40)screen pixels; selecting Sketch again preserves exact frame position.
  Screenshot visually inspected: local origin cross and axes visible near canvas
  centre. Frame evidence is recorded independently from any hidden Session view.

Reproduce from repository root:

```sh
export CURVYO_BROWSER_URL=http://127.0.0.1:1421/
export NODE_PATH=target/browser-check/node_modules
export PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers
export LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib
node frontend/tests/browser/sketch-revision-independent.cjs
node frontend/tests/browser/sketch-revision-source-page.cjs
```

Logs: `target/0066-revision-independent-browser.log` and
`target/0066-revision-source-browser.log`. Saved geometry/screenshots/result JSON:
`target/0066-browser/revision-independent/`. Both scripts accept
`CURVYO_REVISION_OUTPUT`; neither leaves an active browser. All executed journeys
reported no JavaScript errors.

A source-entry test initially timed out reading page rows while Sketches auto-opened
and hid the Document panel. Corrected only the test precondition: explicitly open
Document before reading its active-page row. Final clone/source/pan assertions are
unchanged; this was not a product failure.

Still open for revised actual-host evidence: native input undo, every reverse endpoint
resume and standalone Arc through host hit routing, no-Vector-snap settings matrix,
selection tie/lasso/Shift/Ctrl/Alt, dimension/Reference/fx workflows beyond producer
coverage, and UX's queued parameter-unit popup finding. Previous native cases remain
valid evidence at their stated scope; producer host script is not relabelled as this
independent pass. UX gets the browser next; runtime/parity/performance and final gate
await later explicit handoff.

### Prepared next independent batch (not executed)

Ownership: UX has released; implementer owns the source/build/browser slot for the
bounded unit-selector and Workbench fixes. This plan is documentation only. A fresh
explicit handoff is required before any test edits or execution.

1. Reconcile obsolete setup only in `acceptance_0066_interactions`, `lifecycle`,
   `session`, `chain`, and `conics`. Replace explicit New/picker and unowned identity
   fixtures with lazy page ownership; map local pointer positions through actual
   snapshot placement. Keep history counts at the actual first commit, original
   geometry/radius/fixed-point/continuity assertions, and all tolerances. Specifically
   inventory interactions S01/S11 and lifecycle S01/S08/S09: old group-New and Sketch
   Alt-snapping expectations are superseded, not evidence of current acceptance.
   Record each replacement against the revised criterion rather than bulk rewriting.
2. Regenerate owned runtime fixtures and context metadata (owner/page/placement).
   Native and Node receive identical local goals, IDs, and initial graphs. Retain the
   connected 100-entity/49-length chain and independent 100-entity/33-radius workload,
   unrounded arrays, fixed anchors, global reach-bound and meaningful movement tests.
   Node must select the recorded owner and transform local→document→screen explicitly.
3. Add a small native/actual Node-WASM relation corpus: OnAxis plus an edited driving
   distance; PerpendicularAt line→arc in both branches with radius 6→10, and arc→arc.
   Compare all solved points/radii at ≤1e-6, independently check axis/normal witnesses,
   fixed geometry and retained relation IDs. Existing conic and chain parity remain
   mandatory; compiling a new relation is not runtime proof.
4. Remaining host journeys: focused native input undo; no Vector-snap setting/mask
   influence; reverse START/END resume and standalone Arc by real hit routing;
   selection tie/Shift/Ctrl/Alt; dimension placement and readonly Reference; fx with
   the corrected inline unit control. Include empty-page draft focus/error/history
   isolation where earlier saved-geometry/value evidence did not establish it. Reuse
   the already-passing six-mode, cancel, numeric-line and clone/source reveal evidence;
   repeat only where the final host changes affect it. Stop promptly on a product RED.
5. Preserve the 20-input complete-frame trace, actual canvas+overlay Paint boundaries,
   saved moved geometry and explicit native artifact verifier. Cold is row 0; **every
   row 1..19 is warm** (`rows[1:]`), each <16 ms. No discarded first three, CPU baseline
   subtraction, division or timing retries to select a favorable sample. Report cold
   separately. Use fresh output directories and current production artifacts.

Mandatory bounded commands after handoff (fresh artifact directory parameterization
will replace stale fixed output paths in the tracked runners):

```sh
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_session --test acceptance_0066_chain -- --nocapture --test-threads=1
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_conics s16_native -- --nocapture
cargo build --offline --release --target wasm32-unknown-unknown -p curvyo-editor-wasm
wasm-bindgen --target nodejs --out-dir target/0066-runtime/bindings target/wasm32-unknown-unknown/release/curvyo_editor_wasm.wasm
node curvyo-editor-wasm/tests/runtime/sketch-chain.cjs target/0066-runtime
node curvyo-editor-wasm/tests/runtime/sketch-conics.cjs target/0066-runtime
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_chain s16_actual -- --ignored --nocapture
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_conics s16_actual -- --ignored --nocapture
node frontend/tests/browser/sketch-frame-trace.cjs "$FIXTURE" "$OUTPUT"
python3 frontend/tests/browser/sketch-frame-trace.py "$OUTPUT"
CURVYO_0066_HOST_ARTIFACT="$OUTPUT/chain-host.curvyo" cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_chain s15_actual_host -- --ignored --nocapture
```

Run affected 0066 tests and changed-test lint before the once-final workspace gate.
The gate itself still needs root's explicit GO after functional/runtime/performance
results. New relation test names and their exact execution commands will be recorded
when implemented; this preparation does not claim any new PASS.

### Independent revised runtime milestone — CPU FAIL, parity PASS

Current production execution used the page-owned fixture at
`target/0066-runtime-revision/`, generated from unchanged independent local graphs.
The native pointer coordinates now pass through actual snapshot placement. Recorded
owner/page/origin/size/placement context feeds the Node and host runners; no old picker
or guessed owner is used. This is revised runtime evidence, not the historical gate.

- Release native connected 100-entity workload: 27 measured frames after three explicit
  native warmups, median 6.528342 ms, maximum 6.641948 ms, pointer+view+draw thread CPU.
  Independent 100-entity workload: 90 measured after ten explicit warmups, median
  2.117929 ms, maximum 2.238042 ms. All original hard geometry/progress assertions pass.
- Five native test functions passed across chain/conics/session/revision-runtime.
  New corpus executes OnAxis driving distance 10→20 mm, line→perpendicular arc left
  and right, and arc→arc perpendicular continuation, each radius 6→10 mm. Independent
  centre/end/radius/axis/normal witnesses, fixed points and relation IDs pass.
- Fresh release WASM was built and Node bindings regenerated. All three actual Node
  runners executed. Three explicit ignored artifact-verifier tests then PASS: chain
  30×50 full-precision positions maximum deviation 5.217087155978987e-12 mm; nonlinear
  conics and new four-case relation corpus points/radii ≤1e-6 mm.
- Node v26.12.0 chain pointer+view CPU (no draw bridge): cold 30.895 ms, maximum after
  the existing three explicit Node warmups 15.957 ms. This is not the host-frame gate.
- **Strict host frame CPU FAIL:** production Chromium/SwiftShader, all 20 actual inputs,
  cold frame 0 = 32.253 ms; every frame 1..19 included; frame 14 = **20.094 ms**.
  This is the only warm frame ≥16 ms. Its input and completed-frame task are the same
  20.094 ms renderer task. No subtraction, division, sample exclusion or timing retry.
  Frame 1 = 14.745 ms and frame 2 = 15.856 ms are retained, not silently warmed away.
- Actual saved-artifact verifier PASS (1 test): endpoint
  (241.15025484623925,13.799699194431923) mm, all 49 hard lengths/fixed anchor/nonstale
  geometry and >5 mm movement retained. Therefore this is an actual drag, not an idle
  trace. First harness attempt aborted before tracing because plain wheel pans; the
  corrected documented Ctrl+wheel zoom made the translated endpoint reachable.

Exact native command:
`cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_session --test acceptance_0066_chain --test acceptance_0066_conics --test acceptance_0066_revision_runtime -- --nocapture --test-threads=1`.
Then the build/bindgen/Node commands in the prepared matrix used
`target/0066-runtime-revision`; additionally ran
`node curvyo-editor-wasm/tests/runtime/sketch-relations.cjs target/0066-runtime-revision`.
Explicit verifier:
`cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_chain --test acceptance_0066_conics --test acceptance_0066_revision_runtime -- --ignored --skip s15_actual_host --nocapture`.
Host trace/analyzer used output `target/0066-browser/revision-final-runtime`; saved
movement verifier used its `chain-host.curvyo` via `CURVYO_0066_HOST_ARTIFACT`.

Logs: `target/0066-revision-runtime-native-final.log`, `0066-revision-wasm-build.log`,
`0066-revision-node-{chain,conics,relations}.log`, `0066-revision-parity.log`,
`0066-revision-host-{trace,cpu,geometry}.log` (all under target).
Full raw trace, 20-frame JSON, callback source identities, screenshot and saved document
are under `target/0066-browser/revision-final-runtime/`.

Prepared but NOT EXECUTED: interactions/lifecycle obsolete New/global-frame/Alt-snap
oracle reconciliation. These test-only edits are formatted, pending their affected run
and lint; no PASS is claimed for them. Remaining host matrix and full gate did not run.
Exclusive execution slot released for the confirmed CPU failure; no product edits by
this tester and no repeat benchmark attempts after the valid failing trace.

### Independent suffix correction replay — runtime PASS, standalone Arc host FAIL

Fresh producer suffix source was exercised independently, not inferred from its report.
No repeat timing selection: the one valid independent trace has cold **7.890 ms**,
maximum **7.921 ms over all 19 rows[1:]**, and all 20 input→updated canvas/overlay
Paint frames retained. Saved-artifact verifier PASS with the original 49 lengths,
fixed anchor and moved endpoint oracle. Artifacts:
`target/0066-browser/revision-suffix-independent/`; log prefix
`target/0066-revision-suffix-host-{trace,cpu,geometry}.log`.

Native release workloads PASS: connected100 median1.974421/max2.296992 ms
(pointer+view+draw, 27 samples after3 declared warmups), independent100
median2.0608/max2.389842 ms (90 after10). Seven native test functions pass, including
actual suffix body-drag and tangent-boundary fixtures. Four actual Node runners execute
chain, conics, revised relations and suffix/tangent gestures; five explicit artifact
verifier tests PASS, all unrounded point/radius comparisons ≤1e-6. The suffix witness
checks both raw targets (6,0),(9,4), every 5mm link and expected prefix joint(3,4),
not just the last endpoint. Logs `0066-revision-suffix-{native,parity,node-*}.log`.
Separate Node pointer+view telemetry retains cold30.910 ms and frame25 warm17.932 ms
(raw goal500,200; no GPU/draw bridge). This diagnostic outlier is not hidden or used
as a replacement for the native and complete-host delivery CPU gates.

New independent `acceptance_0066_revision_chain.rs`: 5 tests PASS, including full
three-target suffix, reflected/reversed branches, proper-rotation whole-pose choice,
fixed/crosslink/gap/incoherent guards and 18 invalid first/later NaN/±Infinity targets.
Original singular oracle failed: endpoint(9.999999542321001,1.3847e-10), joint
(4.999999771160479,0.001512744244997), both hard length residuals0 and raw-goal
error4.576790196e-7. Architect/root confirmed the ideal joint1e-6 expectation was
an invalid linear propagation at a double root. Only that oracle now uses the actual
endpoint C: midpoint C/2 plus signed perpendicular height sqrt(25−|C|²/4).
Raw-goal/hard-length and actual native/WASM all-coordinate1e-6 bounds remain intact.
The original failure log is retained as `target/0066-revision-chain-red.log`.

Historical interaction/lifecycle reconciliation: 10/10 PASS. Replace obsolete New
with lazy first durable edit; populated group stays intact on its original page while
source-page creation and first graph edit have distinct undo steps. Inert Vector-layer
refusal remains asserted. Existing geometry uses actual page placement; Rectangle
ignores all Vector snapping settings. Existing-owner preview tests explicitly open an
owned fixture rather than accidentally testing a new unsupported lazy-preview seam.
Layer preservation checks use contains_node/children: object_ids excludes layers.

Actual host PASS:
- Four outward tangent continuations from START/END of Lines and Arcs; saved shared
  endpoint IDs and numerical tangent geometry.
- Native numeric undo/redo between explicit20mm and6mm drafts; saved6mm Circle,
  measured readonly Radius Reference, separate fx/inline unit control.
- All eight stored-snapping/axis-mask cases save identical raw Sketch Point positions.
- Distinct overlapping-circle tie cycling, Shift addition, Ctrl+Alt marquee removal,
  unchanged saved graph. The initial exact-duplicate circle fixture only created two
  circles (duplicate is a no-op); corrected to distinct intersecting circles with saved
  count3 and separated clicks. Initial field undo assumed an empty transient default;
  corrected to two explicit user drafts, preserving native undo/redo assertions.

**Remaining real host FAIL:** standalone Arc from a Line endpoint. Fresh Sketch:
Polyline clicks(350,300),(450,300), Escape; Save; choose Arc / Three points; click
existing endpoint(450,300), then(500,350),(550,300). No Arc commits. Actual UI
labels Start at secondclick(500,350), Through at thirdclick(550,300), awaiting End.
The first endpoint click was ignored. Native r13 equivalent passes; host proof does not.
Screenshot/DOM and before/after saved graphs are in
`target/0066-browser/revision-final-host/arc-from-450-*`; log
`target/0066-revision-final-standalone.log`. Reproduced with pre-Escape screenshot,
so Escape did not cause the missing commit. The graph remains the original single Line.
Tracked repro: `CURVYO_HOST_PHASE=standalone` with
`frontend/tests/browser/sketch-revision-final-host.cjs`.

No full gate. Browser contexts are closed and the execution slot is released for this
actual host defect. Focused Clippy found only three new test naming/semicolon lints;
those edits are applied, but the final retry remains pending and is not claimed PASS.

### Final bounded host correction verification — PASS, full gate pending

Frontend-only semantic-context reset was checked on the fresh production preview.
The independent standalone Arc script ran **unchanged, without a delay workaround**:
Line→Arc and Arc→Arc both PASS saved shared IDs, centre/end coordinates and radius.
Log: `target/0066-revision-arc-fixed-independent.log` (2 cases).
Added fast Workbench→Vector→Sketch / tool / Arc-variant switches preserve the first
centre click and its exact geometric effect. Normal dimension annotation doubleclick,
Escape, immediate re-doubleclick focus and typed8mm edit PASS (2 further cases), log
`target/0066-revision-context-fixed-independent.log`.

The initial dimension saved-geometry check read the raw graph's Circle radius6 instead
of the solved radius after its new Radius=8 constraint. `document.json` intentionally
stores intent/seed, not the resolved candidate. Corrected the browser oracle to assert
saved Radius literal8 plus displayed measured8; a separate explicitly executed native
public-codec artifact test independently unpacks the same download and requires the
resolved Circle radius8 at1e-6. That test PASS1/1:
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_revision_runtime host_dimension_doubleclick -- --ignored --nocapture`.
Log: `target/0066-revision-dimension-artifact.log`. No production change was needed for
this test-oracle correction. Original failed assertions and screenshots remain above.

Focused final strict Clippy PASS:
`cargo clippy --offline -p curvyo-sketch-core --test acceptance_0066_revision_chain -p curvyo-editor-wasm --test acceptance_0066_revision_runtime --test acceptance_0066_chain --test acceptance_0066_conics --test acceptance_0066_session --test acceptance_0066_interactions --test acceptance_0066_lifecycle -- -D warnings`.
Explicit `rustfmt --edition 2024 --check` on those seven changed test files PASS.
Logs: `target/0066-revision-final-test-{clippy,fmt}.log`.

No unresolved independently reproduced functional failure remains in this bounded
matrix. Browser contexts and all verification commands have ended. The final frontend
fix resets click history only on semantic context changes; it does not change core,
WASM or pointer-rate solving, so the preceding independent runtime/parity/strict-frame
measurements retain their stated scope and were not repeated without new risk.
Full CLAUDE7 gate has NOT yet run for the revised story; awaiting root's explicit GO.

### Final revised full gate — running

Exclusive local checkout fallback: the requested separate tester VM was unavailable.
No other agent builds, tests or drives a browser during this gate. Commands run
sequentially via `target/0066-revision-gate/run.py`, stopping at first failure;
`status.json`, `commands.log` and numbered stage logs retain exact outcomes.
Current metadata selects ten WASM packages: document-core, editor-wasm,
geometry-core, image-core, render-core, sketch-core, svg-core, text-core, ui-core,
vectorize-core (all with `curvyo-` prefix).

Only Nextest receives `env -u WEBKIT_DISABLE_DMABUF_RENDERER`. This host exports
that variable as1, whereas the existing desktop test
`webkit_dmabuf_env_var_tests::is_unset_by_default_in_a_fresh_test_process` explicitly
requires it absent to verify the normal fresh-process relaunch detector
(`curvyo-app/src/main.rs`). Unsetting restores that test's documented precondition;
no assertion, filter, test selection, skip or count is changed. Separately, execution
outside the sandbox permits the signal pipe used by existing forked property tests
(previously blocked with EPERM). These environment accommodations do not bypass a
failing check; all workspace tests remain selected and unchanged.

Gate correction: first Nextest run stopped after1338/9790 executed (1337PASS,
1FAIL,130skipped;8452notrun). `acceptance_0050_tester::ac46_files_of_earlier_builds_open_without_layers_or_flags`
classified named current28 `sketch_page_owner_v28.curvyo` as an earlier nameless file.
Named-golden inventory also found valid `sketch_dimension_label_v28.curvyo` and
malformed `sketch_page_claim_invalid_v28.curvyo`. With root approval, only the two
exact valid filenames AND a Sketch leaf expect exact name `Some("Sketch")`;
every other file/node still expects None. All layer/visibility/locking/opacity/export
assertions remain untouched; malformed input remains refused by unpack. Focused ac46
PASS1/1, changed-test strict Clippy and rustfmt PASS. No production change. The initial
failed log/status is retained separately; the complete unfiltered Nextest run was
restarted before any remaining gate stage. Test-only initial compilation omitted a
qualified ObjectSnapshot path, corrected before that focused PASS.

## Revision 2 acceptance (2026-10-31)

Verdict: **FAIL** (3 production defects, 3 spec/decision deviations). Tester: Claude Opus 5.5, exclusive local checkout
(no separate tester VM), branch `story/constraint-sketcher`. Tests were written from AC24-AC35 and the
"Customer decisions" before the implementation was read.

### Tests

- `curvyo-editor-wasm/tests/acceptance_0066_rev2.rs` (43 tests, public `Session`/`sketch_view_json`/`draw_list` routes),
  `curvyo-ui-core/tests/acceptance_0066_rev2.rs` (7 layout tests: 151 annotations, symbol overlap, dimension anatomy).
- AC29 16 ms budget measured as thread CPU (`tests/cpu_time`); asserted in release only (debug prints). Release run:
  max 0.12 ms point, 0.18 line, 0.15 circle, 0.23 arc, 0.20 ellipse, 0.33 rectangle, 0.58 slot (pointer+view+draw).

| AC | Result | Tests / notes |
|---|---|---|
| 24 rail, tooltips, apply with preselection | PASS (tooltip clause see D5) | `ac24_*` (11 buttons in order; all 10 constraints + dimension apply, one history step, symbol appears) |
| 25 Alt first segment | PASS | `ac25_*`: 6 modes on segment 1, same set at segments 2, 3, 5; repeat, field focus, AltGr, Shift+Alt ignored; no history. "Pending numeric input kept" not testable natively (browser: sketch-field-alt) |
| 26 arc both sides | PASS | `ac26_*`: segments 1,2,3,5,8 x tangent / perpendicular left/right: preview drawn at every 12 deg (draw list), degenerate on the line (both rays) flagged and commit refused, committed arcs opposite sweep on each side |
| 27 capture | PASS (D6: 12/16 px, spec says 10) | `ac27_*`: marker "Endpoint", zoom 0.5-20, ignores Vector snapping, reuse of endpoint id, chain close = one point, nearest wins, arc tool |
| 28 Ctrl lock | PASS | `ac28_*`: 14 raw angles equal `snap_angle` table, length kept; 0/90/180/270 add H/V, 45/22/135 add nothing; pending H shown before commit; no effect in tangent/perp modes; capture wins; slot |
| 29 drag all kinds | PASS | `ac29_*`: point, line (body, endpoint), circle, arc, ellipse, rectangle edge, slot side; one history step; Escape restores; fixed stays; constrained (H, radius) |
| 30 cursors | PASS | `ac30_*`: grab/grabbing/grab, Escape, crosshair in tools, default over fixed point and constraint symbol (a dimension text is draggable, so `grab` there is correct) |
| 31 dimension annotations | **FAIL** (D1) | `ac31_*`: permanent "50 mm", "R 25 mm", "⌀ 50 mm", reference "(63.25 mm)", expression "80 mm", ui-core anatomy and constant px offset pass; angle text is "45°", spec says "45 deg" |
| 32 symbols, thinning | **FAIL** (D2, D3) | names, no overlap, select = constraint selected, 150 no thinning, 151 thins, selected and conflicting stay full; hovered dot cannot be hovered (D2); redundant named "conflicting" (D3) |
| 33 entry at object | PASS | `ac33_*`: near midpoint, not on the cursor, follows pan, inside canvas at 6 edge/corner positions, fields per kind, edit at text |
| 34 Alt, no bar | PASS | `ac34_*`, no compound tool creates redundant/conflicting constraints, browser scripts (no options bar) |
| 35 variants | PASS | `ac35_*`: Circle, Arc, Ellipse cycle with Alt before the first pick, label names it, after a pick a hint and picks kept |

### Browser (Chromium, production preview :1421; `target/0066-browser`)

PASS: rev2-m2, rev2-m3 (`rev2-m3/plate-fixed.png`, copy of `plate-dimensions-symbols.png`), sketch-workbench, sketch-acceptance,
sketch-clone-acceptance, sketch-field-alt, sketch-revision-host, sketch-revision-independent (10 journeys), sketch-revision-final-host
(default minus Reference, standalone, contexts, selection), sketch-frame-trace (cold 9.6 ms, warm max 8.2 ms < 16 ms, analyzer PASS),
sketch-revision-source-page, sketch-snapping-acceptance. Native verifiers PASS: `d01_actual_browser`, `d12_actual_browser`,
`s15_actual_host` (absolute `CURVYO_0066_HOST_ARTIFACT`). FAIL (one oracle each, rest of the journey passes):
sketch-remaining-acceptance and sketch-revision-final-host default phase: **D4** a Reference field is not `readonly`.
Not rewritten: `sketch-host-cpu.cjs` (informational, superseded by sketch-frame-trace). All rewrites keep the original oracles; changes:
modes by Alt, numeric by Tab entry box, constraints by rail, Active-sketch/New/Apply removed. `sketch-revision-independent` failed once in
a batch run (race after Enter), then 7/7 after waiting for the box to detach. Frame trace needed a zoom/pan change: dimensions under
16 px are hidden (by design) and at 1920 px the old input points lay under the Properties panel.

### Gate (run3, `target/0066-revision-gate/run3`)

fmt PASS; clippy PASS; nextest 9913 run, 9911 passed, 2 failed (my AC31/AC32 findings), 130 skipped (a third test, D3, was added after
the run: 3 expected failures); wasm build and wasm clippy (10 packages) PASS; deny PASS; rustdoc PASS; frontend build:wasm, tsc PASS;
`npm test` 717 tests, 643 pass, 74 skipped, 0 fail; oxlint PASS.

### Defects (production code untouched)

- **D1 AC31 angle text.** Reproduce: two lines, select both, `sketch_apply_action("angle","45",false)`; annotation text is `45°`,
  spec AC31 says `45 deg`. Likely a spec typo; decide which is right. Test `ac31_angle_dimension_text_is_45_deg`.
- **D2 AC32 hovered annotation in thinned mode.** With 151 annotations (151 Horizontal via Ctrl lock) hovering a dot symbol or its line
  hovers/reveals nothing, so "hovered stays visible" cannot happen. Test `ac32_151_annotations_...`, last assertion.
- **D3 AC32 redundant symbol.** Apply Horizontal twice to one line: constraints list says `redundant:true, conflicting:false`, the symbol
  says `conflicting:true`, name "Horizontal constraint, conflicting". Test `ac32_redundant_symbol_carries_a_non_colour_marker`.
- **D4 Reference field.** In the entry box a Reference dimension field is not `readonly` (old oracle), and typing a key into it closes the
  box. Ticking Reference leaves focus on the checkbox, so Enter does nothing until the field is focused again (UX nit).
- **D5 AC24 tooltip shortcut.** Tooltips name the tool ("Horizontal: ...") but no shortcut; UX notes say no shortcut letters, spec says "its shortcut". Spec/UX conflict.
- **D6 AC27 radius.** Implemented 12 px in / 16 px out (UX notes, plan); spec text says 10 px (Proposal). Tested as 12/16.

## Revision 2 retest (2026-10-31)

Commits under test: 47540562, fb0c4e8d, d707102e. Logs: `target/0066-revision-gate/run4` (gate), `run4/browser` (scripts).

| AC | Result | Note |
|---|---|---|
| 24 | PASS | tooltips name the tool, no shortcut letters (spec updated) |
| 25 | PASS | unchanged |
| 26 | PASS | arc both sides |
| 27 | PASS | capture radius 12/16 px (spec updated) |
| 28 | PASS | Ctrl lock |
| 29 | PASS | drag all kinds |
| 30 | PASS | cursors |
| 31 | PASS | dimension annotations, angle text `45°` (spec updated), redundant dimension texts named apart |
| 32 | PASS | symbols: hovered thinned dot reachable (D2 fixed), redundant symbol carries red + `!` and a name distinct from conflicting (D3 fixed) |
| 33 | PASS | entry at object, box kept when pointer moves onto it (D4 fixed) |
| 34 | PASS | Alt, no bar |
| 35 | PASS | variants |

Rev2 Rust tests: `curvyo-editor-wasm` 43 + `curvyo-ui-core` 7, all pass (D1, D2, D3 tests included, none weakened; D1 oracle is `45°`).
Test change: `sketch-remaining-acceptance.cjs` reference phase now focuses the readonly field and presses a real key (`keyboard.press('9')`), keeping the oracles (value unchanged, box stays open, readonly attribute). It passes, so the old D4 "box closes on key" is fixed too.

Gate run4: fmt, clippy, nextest (9917 run, 9917 passed, 130 skipped), wasm build + wasm clippy, deny, rustdoc, frontend build:wasm, tsc, npm test (74 skipped, 0 fail), oxlint all exit 0.

Browser (Chromium, preview :1421, rebuilt dist): PASS rev2-m2, sketch-workbench, sketch-acceptance, sketch-clone-acceptance, sketch-field-alt,
sketch-revision-host, sketch-revision-independent, sketch-revision-source-page, sketch-snapping-acceptance, sketch-remaining-acceptance,
sketch-revision-final-host (default, standalone, contexts, selection), sketch-frame-trace + analyzer (exit 0).
sketch-rev2-m3: failed once in the batch (timeout waiting for the `Length 50 mm` annotation button after Enter at line 46), then 2/2 PASS on rerun. Treated as a timing flake of the script, not reproduced; worth a wait-for-settle if it recurs.

Verdict: PASS, AC24-35. No open defects (D1-D6 resolved).

## Revision 2 final check (2026-10-31)

Commits under test: b2030888..b1acaf74 (UX review changes). Logs: `target/0066-revision-gate/run5` (gate), `run5/browser` (scripts). Review item 8 stays open by lead decision ("Endpoint" label kept per AC27).

New independent tests (no existing test weakened): `curvyo-editor-wasm/tests/acceptance_0066_rev2.rs` +11 (`rev2_review2_*`), `curvyo-ui-core/tests/acceptance_0066_rev2.rs` +2, new `curvyo-render-core/tests/acceptance_0066_rev2.rs` +3.

| Review item | AC | Test | Result |
|---|---|---|---|
| 1 small geometry symbols hidden unless hovered/selected; R text off the outline | 32 | `rev2_review2_symbols_of_small_geometry_*`, `..._radius_text_does_not_lie_on_the_circle_outline` | PASS |
| 2 point+line distance (either order, canvas picks, 1 undo step, driving) | 23, 24 | `..._measures_a_point_to_a_line_in_either_pick_order`, `..._picking_a_point_then_a_line_on_the_canvas...` | PASS |
| 2 refused picks say what is needed (tooltip, mode label) | 24 | `..._a_selection_no_kind_fits...`, `..._a_refused_canvas_pick_says_why_on_the_mode_label` | PASS |
| 3 arc Radius, circle Diameter, Alt switches and wraps, label names kind, created kind matches | 34 | `..._arc_defaults_to_radius_circle_to_diameter...` | PASS |
| 4 Ctrl lock: preview H badge, angle lock badge only while Ctrl holds a free direction | 28 | `..._ctrl_lock_shows_the_angle_badge_and_a_preview_h_badge` | PASS |
| 5 per-field entry refusal, message not in Sketches | 20, 33 | `..._a_refused_entry_marks_only_its_field...` | PASS |
| 6 unsolvable dimension marked "!", box open with reason, last valid geometry kept | 7, 33 | `..._an_unsolvable_dimension_is_marked...` | PASS |
| 7 refused canvas pick on the mode label | 33 | the two refusal tests above | PASS |
| 9 badge clear of text box and point glyph | 32 | ui-core `review2_a_badge_never_lies_on_a_dimension_text_box`, `review2_a_point_badge_does_not_cover_its_point` | PASS |
| 10 redundant dimension dashed, driving colour, apart from reference | 5, 32 | render-core 3 tests, editor `..._redundant_dimension_is_marked_apart_from_conflicting` | PASS |

Not asserted: guide-line style and badge glyph pixels (4a, 4c) beyond the plate screenshots; `aria-invalid` on one field (frontend DOM).

Gate run5: fmt, clippy, nextest (9945 run, 9945 passed, 130 skipped), wasm build and clippy, deny, rustdoc, build:wasm, tsc, npm test (74 skipped, 0 fail), oxlint, vite build all exit 0.

Browser (Chromium, preview :1421): PASS rev2-m2, rev2-m3, rev2-plate (`["50 mm","R 25 mm","R 3.99 mm","25 mm","12.5 mm"]`), workbench, acceptance, clone-acceptance, field-alt, revision-host, revision-independent, revision-source-page, snapping-acceptance, remaining-acceptance, revision-final-host (default, standalone, selection), frame trace + analyzer.
Flake: revision-final-host `contexts` failed once (script line 46, expression field not yet focused after the dimension double-click), then passed 3/3. The focus assertion has no wait; not reproduced; worth retry-until-focused.

Verdict: PASS, AC24-35 and review items 1-7, 9, 10. No open product defects. Open by decision: item 8.

## Revision 3 acceptance (2026-10-31)

Tester: independent tests written from AC36-AC41 (defaults R3-Q1 A, R3-Q2 A, R3-Q3 A) before the
diff was read; file `curvyo-editor-wasm/tests/acceptance_0066_rev3.rs` (27 tests, 1 ignored), all
through pointer, key, rail and the host's Tab/Enter calls (`sketch_open_entry`, `sketch_complete_fields`).

| AC | Test (acceptance_0066_rev3) | Result |
|---|---|---|
| 36 | `ac36_customer_scenario_centre_arc_refills_the_gap_on_the_two_free_endpoints` (arc, tangent line, 2 tangent arcs closed on start, delete segment 2, centre-first Arc; marker before click, identity reuse, only centre new, no new constraints) | PASS |
| 36 | `ac36_every_tool_and_variant_captures_a_nearby_endpoint_on_its_final_picks` (12 tool/variant cases x click and follow) | PASS |
| 36 | `ac36_drag_release_and_follow_click_both_reuse_the_endpoint_for_the_end_pick` | PASS |
| 37 | `ac37_coincident_*`: point+line (infinite, dof -1, one step, kind named), armed picks both orders, X/Y axis, circle, arc, ellipse, point+point; extent not enforced, drag, undo, save | PASS |
| 37 | `ac37_symmetric_*`: two points + line, + X/Y axis, + point; positions 1e-6, dof -2 | PASS |
| 38 | `ac38_follow_and_drag_paths_give_identical_geometry_for_every_tool` (9 tools/variants) | PASS |
| 38 | `ac38_release_under_4_px_follows_and_4_px_or_more_commits_at_every_zoom` (3.5/4.5 px at 3 zooms) | PASS |
| 38 | `ac38_escape_in_the_follow_state_cancels_every_shape_without_history`, `ac38_alt_cycling_and_ctrl_lock_work_in_the_follow_state` | PASS |
| 39 | `ac39_rectangle_in_follow_state_typed_width_only_is_one_dimension_and_dof_3`, `..._both_typed_..._press_drag_path_matches`, `..._untyped_enter_creates_no_dimension`, `..._escape_in_the_entry...`, `..._circle_radius_line_length_slot_fields...`, `..._ellipse_both_radii_fields` | PASS |
| 39 | `ac39_known_open_typed_angle_and_ellipse_radii_become_dimensions` (`#[ignore]`) | FAIL, known open: typed line Angle gives only the Length dimension |
| 40 | `ac40_every_valid_preselection_applies_at_once_in_one_history_step` (26 selections: result, one step, tool state unchanged, nothing new selected, undo) | PASS |
| 40 | `ac40_dimension_preselection_applies_at_once_in_one_history_step` | FAIL: Dimension with a valid selection only arms (state "arm", no constraint, no step). AC40 lists it among the apply-at-once tools; revision 2 armed it for label placement. Product decision needed. |
| 40 | `ac40_invalid_preselection_adds_nothing_and_waits_with_a_hint` | FAIL: nothing added and the tool waits (`constraint_tool` set), but no on-canvas hint anywhere in `sketch_view_json` (`canvas.mode_label` null, no annotation, `error` null) for 9 invalid selections |
| 41 | all of the above use the real event path | PASS |

Browser (Chromium, vite dev :5173, rebuilt wasm, logs `target/0066-revision-gate/run6/browser-*.log`):
PASS rev3, rev2-plate, rev2-m2, rev2-m3, acceptance, workbench, remaining-acceptance, geometry-oracles,
field-alt, snapping-acceptance, revision-source-page. FAIL: `sketch-clone-acceptance` (circle created with a
typed radius 10 now carries a driving R 10 by AC39, so the script's later R 15 conflicts: "Conflicts with other
constraints"; script is stale against AC39, not a product defect) and `sketch-revision-independent`
(journey 7, line 40: in the second Polyline round, Tab after click+move moves focus to the Workbench button
and no entry opens; deterministic in 2 runs, not reproduced by a reduced script, cause not found: triage).

Gate run6 (`target/0066-revision-gate/run6`): fmt, clippy, wasm build and clippy, deny, rustdoc,
build:wasm, tsc, npm test, oxlint exit 0. nextest (--no-fail-fast, env -u WEBKIT_DISABLE_DMABUF_RENDERER):
10089 run, 10087 passed, 2 failed (the two AC40 tests above), 143 skipped.

Observation: a press within 12 px of the sketch origin captures the origin for press and release, so a
rectangle dragged from the origin by under 12 px commits nothing; not in the spec, no action taken.

Verdict: FAIL. Open: AC40 invalid-selection hint, AC40 Dimension preselection (decision), AC39 typed Angle
and ellipse radii (known), two browser scripts to triage or update.

## Revision 3 retest (2026-10-31)

Verdict: FAIL (2 defects). Gate logs: `target/0066-revision-gate/run7/`. Branch head at start 34201d5b.

| Item | Test / script | Result |
|---|---|---|
| AC39 | `ac39_known_open_typed_angle_and_ellipse_radii_become_dimensions` (`#[ignore]` removed) | PASS |
| AC24/AC40 | `ac24_dimension_rail_tool_applies_at_once_with_a_segment_and_creates_a_dimension` (renamed; one step, measured length, box on the new dimension, typed 50 mm is a second step, no second Length) | PASS |
| AC40 | `rev2_review2_dimension_measures_a_point_to_a_line_in_either_pick_order` (applies at once, 60 mm in the box, 40 mm typed via `sketch_dimension_text`, 2 steps) | PASS |
| AC7/UX 6 | `rev2_review2_an_unsolvable_dimension_is_marked_keeps_its_box_and_can_be_undone` (second press = redundant Length, then edited to 30 mm) | FAIL, defect 1 |
| AC40 | `ac40_one_selected_point_then_dimension_then_a_second_point_gives_a_point_distance` (new) | FAIL, defect 2 |
| rest | all other `acceptance_0066_*` Rust tests (workspace nextest: 10099 run, 10097 pass, 142 skipped) | PASS |
| browser | all 15 Sketch scripts and `sketch-frame-trace` | PASS after the script updates below |
| browser | `sketch-host-cpu.cjs` | stale, fails before any Sketch step: `selectOption` on the Workbench control, which is a menu button now (informational probe, unrelated to rev 3) |

Gate: fmt, clippy host, wasm32 build and clippy (core crates + editor-wasm), cargo deny, cargo doc, `build:wasm`, `tsc -b`, `npm test` (650 pass, 94 skipped), oxlint green. nextest red only for the two tests above.

Defects
1. A value typed into the edit box that makes the dimension conflict closes the box silently. Repro: draw a 20 mm line, select it, press Dimension (box opens, Length 20); select the line again, press Dimension (second Length, redundant); `sketch_dimension_text(second, "30 mm", false)` returns `Ok(true)`. Both dimensions are marked `! 20 mm`, `conflicting: true`, but `canvas.entry` is `null`: the reason is never shown. Same after a double-click edit (`sketch_edit_dimension`, then `sketch_dimension_text`). `sketch_check_entered` is meant to keep the box with the reason; the implementer's test covers only the `sketch_apply_action` path. The edited dimension's annotation also still reads `! 20 mm` (measured) while its expression is `30 mm`.
2. AC40 "valid start kept as first picks": select point p1, press Dimension (arms, no step), click point p2: the selection becomes p2 only and no distance is created (a label click afterwards does nothing). The click replaces the first point instead of completing the pair.

Note: `dimension_action` in the view stays "distance" after an at-once press (it follows the selection), so it is no arming indicator; the plan's "no longer set" is inexact (my test checks `mode_label` null instead).

Script updates: `sketch-clone-acceptance.cjs` (double-click the existing R 10 text, fill 15 mm), `sketch-rev2-m3.cjs` (segment selected: Dimension applies at once, type into the open box, wait for `Length 25 mm`), `sketch-remaining-acceptance.cjs`, `sketch-revision-host.cjs`, `sketch-revision-final-host.cjs` (nothing selected: Dimension arms, circle picked on its outline; `sketch-revision-host` also accepts `Fixed · Redundant` because the typed radius is now driving). `sketch-workbench`, `sketch-acceptance`, `sketch-rev2-plate`, `sketch-rev3` needed no change.

## Revision 4 acceptance (2026-10-31)

Verdict: PASS (one minor finding, one interpretation for the PO). Production code under test:
f892f448..5038d599 (head f32c7f0b adds docs only). Tests written from AC42-AC48 before the diff was
read, all through pointer, key, rail and the host calls (`sketch_complete_fields`,
`sketch_dimension_text`, `sketch_edit_dimension` as `SketchAnnotations.tsx` calls it on double-click).

| AC | Test | Result |
|---|---|---|
| 42 | `acceptance_0066_rev4::ac42_every_drawing_tool_returns_to_select_with_exactly_the_new_geometry` (Point, Rectangle, Slot, Circle x2, Ellipse x2, Arc x2: tool Select, selection replaced by the new shape, drag moves it as its own step, commit = one step) | PASS |
| 42 | `ac42_fillet_returns_to_select_...` (host Enter on the Fillet entry) | PASS |
| 42 | `ac42_polyline_stays_while_the_chain_is_open_...`, `..._enter_ends_...`, `..._closing_on_the_start_...`, `ac42_escape_without_a_segment_keeps_the_previous_selection` | PASS |
| 43 | `ac43_selection_first_apply_returns_to_select_with_the_referred_geometry` (15 tool/selection cases), `ac43_pick_completion_returns_to_select` (5), `ac43_refused_use_leaves_the_tool_active` (7, with hint) | PASS |
| 44 | `ac44_rail_groups_the_two_axis_tools_after_dimension_with_names`, `ac44_axis_distances_for_points_line_origin_and_axes` (point-point, line, origin, X/Y axis; aligned Dimension unchanged), `ac44_alt_does_not_turn_dimension_into_an_axis_distance` | PASS |
| 45 | `ac45_horizontal_value_drives_projection_only_and_removes_one_dof` (30 mm, dy unchanged 1e-6, DOF -1, undo), `ac45_vertical_...`, `ac45_annotation_names_the_kind_and_reference_is_parenthesised` (↔/↕, accessible name, "(↔ …)"), `ac45_double_click_opens_the_edit_and_expressions_are_accepted`, `ac45_save_reopen_keeps_..._and_convert_omits_annotations` | PASS |
| 46 | `ac46_side_is_kept_zero_aligns_negative_and_invalid_refuse` (both kinds, reversed pair, 25 → 0 → 40 keeps side, -5 mm and unknown name refused, no step, entry kept) | PASS |
| 47 | `ac47_shift_click_selects_two_endpoints_without_the_line` (+ Ctrl marquee removes), `ac47_marquee_selects_exactly_two_points` (no step, no movement) | PASS |
| 48 | `ac48_three_tools_on_the_same_two_points` (50/40/30 mm, kinds, one step each, Select), `ac48_selection_built_by_shift_click_then_applied_at_once` | PASS |
| edges | `edge_point_left_of_origin_keeps_its_side_...`, `edge_axis_parallel_to_the_measured_direction_is_refused`, `edge_coincident_points_start_at_zero_and_huge_values_do_not_panic` | PASS |
| fmt 29 | `curvyo-document-core/tests/acceptance_0066_format29.rs`: v29 goldens open as declared and round-trip, plate holds both kinds; v28 goldens open, re-save writes 29; a file with an axis distance declares 29 (a 28 reader's gate refuses; the gate is proven with 30 → `FormatTooNew`); malformed records written into the Loro snapshot: missing field, unknown kind, null value refuse the file; negative value, line operand, missing point, Angle value open with the sketch stale (same handling as the existing `Distance`) | PASS |
| 12 | native/wasm solve agreement not re-run for the new kinds (no browser oracle); `sketch-rev4.cjs` checks the saved signed relations | not covered here |

Browser (Chromium, vite dev :5173, rebuilt wasm, logs `target/0066-revision-gate/run10/browser/`): all 17
Sketch scripts PASS incl. `sketch-rev4` and `sketch-frame-trace`, after test-only updates for AC42/AC44:
`sketch-rev2-m3` (13 rail buttons), `sketch-remaining-acceptance`, `sketch-revision-final-host`,
`sketch-revision-host` (clear the returned selection so Dimension arms / Arc starts clean).
`sketch-host-cpu` not run (stale since revision 3, see above).

Gate run10 (`target/0066-revision-gate/run10`): fmt, clippy host, wasm32 build and clippy (9 core crates +
editor-wasm), deny, rustdoc, build:wasm, tsc, npm test (754, 658 pass, 96 skipped, 0 fail), oxlint
(warnings only) exit 0. nextest (env -u WEBKIT_DISABLE_DMABUF_RENDERER, --no-fail-fast; run as
non-wasm crates + editor-wasm in 4 partitions because one call exceeds the 10 min tool limit):
10213 run, 10213 passed. `pages two_hundred_pages_cpu_budget`: switch average 4.04 ms (add 15.3 ms, draw 4.96 ms).

Finding 1 (minor, UX): AC42 leaves a new free point selected; choosing Arc and pressing Alt for the
centre-first variant then shows the error "Choose an open endpoint with one adjoining line or arc"
in the mode label although the user only switched variant. Repro: Point tool, click; Arc tool;
hover; Alt → `canvas.mode_label.error` is that text (empty selection: no error). The resume-from-selected-
endpoint logic runs on a lone point without a curve; it should stay silent there.

Interpretation for PO (AC42 "exactly the new geometry, all entities the commit created"): the
selection holds the created curves (rectangle/slot members, circle, arc, chain lines), not their
defining points and centres; free points are selected when the shape is a point. Tests assert this
reading (AC22 keeps points and edges distinct targets). Observation: an unknown extra field in a
relation record is ignored on open for all kinds (pre-existing, not revision 4).
