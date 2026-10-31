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
