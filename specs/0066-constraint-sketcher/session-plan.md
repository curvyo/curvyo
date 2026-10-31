# Session and workbench implementation plan

Status: Session and frontend implemented; independent runtime/browser acceptance and full gate passed. Customer acceptance remains pending.
Scope: accepted AC1–18, within the same story branch and final end-to-end PR.
Authority: specification, ADR0025/0026, `session-integration-notes.md`,
`ui-interaction-map.md`, and the independently reviewed document milestone.
Document and Rust Session milestones were reviewed on the same unpublished story
branch. Independent runtime/CPU acceptance preceded frontend wiring. Final delivery
combines the verified end-to-end feature in one commit; historical milestone checks
are recorded below and current evidence is in `acceptance-session-report.md`.

## State and ownership

**Implementation detail (Proposal):** introduce explicit `Workbench::{Vector, Sketch}`
and a separate sketch interaction state; do not overload ordinary Pen/Node state.
The session owns active sketch ID, tool/variant, selection, picks, drafts, drag seed,
last valid preview and transient errors. The document owns only completed intent.
UI-core owns pure selection/applicability and annotation/view calculations; sketch-core
owns analytic construction, constraints, formulas and numerical solving. TypeScript
renders Rust-provided state and forwards commands; it never computes sketch geometry.

Use current `SketchSnapshot.graph` for identity and relationship authority and
`solution` for solved local coordinates. When stale, historical solution-only IDs
are not editable targets. Convert document pointer coordinates through placement's
inverse; pan/zoom stays in document coordinates. Keep fixed origin distinct from a
removable Fixed relationship. Changing workbench creates no object or history.

## 1. Close concrete domain API gaps first — AC2–4, AC7, AC11, AC13–17

- [x] Add operation results retaining the **pre-solve intended change separately
  from the solved candidate**. Current constructors/fillet return only solved Sketch.
  Capture operation intent before solving; derive record deltas against the same
  before-operation resolved graph, then apply only those records to live intent.
  Never diff all solved positions against stored intent. Keep existing constructor
  wrappers for existing callers/tests while the session consumes explicit edits.
- [x] Add incremental segment/close-chain support with caller-minted shared endpoint
  IDs; repeated calls to whole-polyline construction must not duplicate prior points.
- [x] Add pure relation-reference traversal and selection applicability with concrete
  refusal reasons for all ten constraints and five dimensions. Respect actual core
  entity combinations; Equal does not silently invent ellipse semantics. H/V can use
  the accepted segment selection; do not expose an unsupported convenience route.
- [x] Provide explicit reference-aware entity deletion and fillet failure diagnostics.
  Deletion identifies/removes dependent relationships in the same operation; failed
  fillet identifies implicated stable constraint IDs without parsing human strings.
- [x] Test creation/fillet deltas with pre-existing solved-but-unpersisted dependent
  coordinates, shared polyline IDs, reference deletion and preserved fixed targets.

These are implementation gaps inside accepted operations, not scope questions.
Source scaling remains the one pending customer decision; it does not block them.

## 2. Session lifecycle and thin bridge — AC1, AC4, AC9–11, AC18

- [ ] Test first: navigation writes nothing, New creates one step, selected original
  re-enters, a clone routes to its original, switching cancels unfinished input only,
  and undo/delete/open/peer changes cannot retain stale editable IDs.
- [x] Extend `session/mod.rs`, `keys.rs`, `panel_tabs.rs`; keep new implementation in
  bounded `session/sketch/{state,lifecycle,commands,view}.rs` modules.
- [ ] Add `wasm_sketch.rs` forwarding workbench, active sketch, tool/variant, creation,
  constraint/dimension/parameter, inspection and conversion actions to session methods.
  Expose a typed serializable view with IDs, action availability/reasons, input errors,
  resolved quantities, diagnosis and selection. Do not expose ACS or Loro to the host.
- [ ] Preserve existing native/local input undo, document undo chords, focus return,
  current drawing layer, property collapse and camera-origin compensation.

## 3. Analytic picking, construction and snapping — AC1, AC4, AC13–16

- [ ] Test real down/move/up and hover routes for Point, connected/open/closed Polyline,
  three-point Arc, both Circle/Ellipse variants, Rectangle, straight Slot and Fillet.
  Cover degenerate last picks, Enter completion, Escape and variant changes.
- [ ] Add UI-core sketch hit testing/selection and session `picking`, `creation`,
  `construction_preview` modules. Pick IDs belong to analytic entities/points,
  never to approximate Bézier anchors. Shift extends selection.
- [ ] Reuse cached snapping for hover, first press, drag, release and key-only changes;
  Alt inversion, axis switches and eligible kinds apply. Fixed earlier picks do not
  jump. Snapping never creates unrequested persistent constraints.
- [ ] Polyline commits each completed segment; Enter/end-chain adds no extra step.
  Other completed constructions/fillets commit once; cancellation writes nothing.
  Optional centre-based Arc/Rectangle variants are not extra required families.

## 4. Dragging, dimensions, parameters and diagnostics — AC2–7, AC11–12, AC17

- [ ] Test bounded Drag solving from a compatible seed, failed goal retention,
  cancellation and exactly one release commit via `apply_sketch_preview`.
  Only explicit user geometry records enter delta; dependent pose remains checkpoint.
- [ ] Test distinct direct point/handle goals versus body translation. A free line,
  circle or ellipse dragged by its body preserves its unconstrained shape; translating
  only one defining point must not deform it. Body intent explicitly translates its
  defining geometry; handle intent changes only the intended shape control.
- [ ] Add concrete multiple temporary point goals if required by these body gestures
  (no solver framework or speculative trait). Constrained dragging seeks meaningful
  closest-feasible movement for an unreachable goal, rather than freezing at the start.
  Test beyond-limit movement, hard residuals, cancel, release and undo. Only explicit
  body/handle geometry deltas become intent; dependent solved positions remain in the
  checkpoint. Preserve last-valid geometry only when no valid preview is available.
- [x] Add session `drag`, `constraints`, `dimensions`, `parameters`, `inspection`.
  Full rank analysis runs at edit/revision boundaries, never each pointer frame.
- [x] Numeric dimension input defaults to mm (angle uses its explicit unit); classify
  a bare number before constructing typed Quantity. Do not append a unit blindly to
  formulas. Parse names to stable IDs, preserve references on rename, display expression
  separately from resolved value, and distinguish driving/reference dimensions.
- [ ] Invalid expression drafts retain their text and identify the entry to repair;
  stored valid values remain. Contradictory valid relationships stay inspectable with
  last-valid geometry. Expose conflict/redundancy IDs separately from solver failure.
- [x] Render origin, construction geometry, annotations and highlights as overlays
  through `session/draw.rs`/`frame.rs` and bounded render-core sketch modules. Output
  artwork contains only validated contours; inspection/navigation writes no history.
  Use text plus visual cues; do not invent motion vectors from a DOF count.

## 5. Existing frontend shell end to end — AC1–5, AC7, AC17–18

- [ ] Wire `useEditorSession.ts` and bridge types to Rust state/commands.
- [ ] Add the Workbench selector above `PropertiesPanel.tsx`, including compact
  accessible V/S when collapsed; add Sketches to the existing vertical tab rules.
- [ ] Add Sketches picker/New, selection/actions, dimensions, parameters, status and
  inspect/repair controls using existing fields/tokens. Invalid actions remain
  focusable with `aria-disabled` and a reason. No hardcoded mock values/ranges.
- [ ] Adapt `ToolRail.tsx` and the existing single options-bar owner to sketch tools
  and named variants; ordinary Vector controls remain unchanged outside Sketch.
  No header, Finish strip, second constraint toolbar or bottom diagnostic bar.
- [ ] Browser regressions: switch/re-enter, collapsed panel, roving focus, input undo,
  dimension double-click + keyboard equivalent, invalid formula correction,
  conflict removal, key-only Alt during construction, save/reopen and conversion undo.

## Evidence and handoff

- [ ] Focused session matrix covers every accepted operation and all AC1–18 paths;
  independent tester adds acceptance without deriving assertions from implementation.
- [ ] Measure the **actual session pointer path** for the specified 100-entity workload
  using thread CPU time, including cache access; verify every hard residual. Spike
  numbers do not substitute for this result. Compare native/WASM pose within 1e-6 mm.
- [ ] Run affected checks during work, then hand the exclusive slot to tester for the
  full gate once; obtain UX and architecture review before the one end-to-end PR.
- [ ] Keep commands, logs and temporary artifacts under `target/0066-*` inside the
  repository. Update this checklist and `plan.md` only from actual observed results.

Initial evidence: core36/36 + all-target core Clippy; two lifecycle tests passed
(new/navigation/re-entry/undo and explicit construction/clone original routing).
Commands and logs: `target/0066-logs/session-*.log`. Remaining lifecycle/event
routes and independent acceptance are still open.

Rust milestone evidence (current): core43 PASS; fourteen focused session cases PASS.
The broad editor483/UI874 unit suites passed before the final two cases; final
affected checks are recorded below once complete. `session-api.md` maps the public
bridge and implementation modules. Actual session CPU/native-WASM numerical
parity and frontend/browser input/focus behavior remain independent acceptance work.
S1 regression: two independent merged parameter cycles can be repaired per row;
new cycles remain refused. Two dangling relationships can likewise be removed
individually. S2: construction preview read and commit both reject stale revisions.

### Bounded Rust handoff checks

- `cargo test -p curvyo-sketch-core --offline`: 43 passed (`session-core-latest.log`).
- `cargo test -p curvyo-editor-wasm -p curvyo-ui-core -p curvyo-render-core --lib --offline`:
  editor485, UI874, renderer179 passed (`session-final-regression.log`).
- Final fixed-origin snapping refinement and test split then checked with
  `cargo test -p curvyo-editor-wasm --lib session::sketch --offline`: 15 passed
  (`session-surface-tests.log`). This includes offset handle presses and stationary
  siblings connected only through a fixed origin.
- Native affected library Clippy passed for editor/UI/render/sketch/document crates
  (`session-final-clippy.log`).
- `cargo build -p curvyo-editor-wasm --target wasm32-unknown-unknown --offline` passed
  (`session-final-wasm-build.log`); final WASM library Clippy with `-D warnings`
  passed after the snapping refinement (`session-final-wasm-clippy.log`).
- The ordinary unit tab inventory now includes Sketches (eight tabs); independent
  acceptance files were not changed. Additional exact frontend/tab inventory
  fixtures are the independent tester's review, not weakened assertions.

All logs above are under `target/0066-logs/`. Full gate, CPU/parity acceptance,
frontend workbench wiring and browser interaction remain open.
- Unchanged independent document acceptance rerun after partial-repair guard:
  `cargo test -p curvyo-document-core --test acceptance_0066_document --offline`:
  10 passed (`session-document-regression.log`). `cargo fmt --all --check` passed.

## 6. Frontend wiring sequence after independent Rust handback

Preparation only: no frontend production changes or checks are claimed here.
Preserve the rebased milestone facts above and the current Trace Mask integration.
Use `ui-interaction-map.md` for accepted interaction details and `session-api.md`
for domain boundaries. Optional centre-based Arc/Rectangle variants remain optional;
the supported Rust variants are the controls to wire first.

1. **Session hook and bridge types (AC1, AC4, AC17–18).** Add typed host records for
   `sketch_view_json`, with workbench/tool, active sketch, choices, entities, actions,
   constraints, parameters, annotations and diagnostics. Refresh the view through the
   existing event/command refresh owner after pointer, modifier, history, open and
   peer changes. Add thin callbacks for the methods below; preserve current Trace
   callbacks. Rust remains the only authority for geometry, eligibility, IDs, formulas,
   units, measured values, solver status and preview acceptance. Host state stores
   only uncommitted input drafts, focus and presentation preferences.
2. **Existing Properties dock (AC1, AC18).** Add Vector/Sketch choice above its current
   body/tab strip, with compact accessible V/S when collapsed. `set_workbench` must
   preserve collapse/camera compensation. Add Sketches to the existing tab registry,
   roving focus and content switch. Keep all status/error/repair presentation inside
   Sketches; no extra header, footer, diagnostic strip or toolbar.
3. **Sketches navigation and domain rows (AC2–5, AC7, AC17).** Wire picker/New/re-entry,
   selected geometry, origin Fixed toggle, construction membership, the fifteen
   Rust-provided actions and their refusal reasons. Then add dimension/reference
   rows and parameter name/expression/unit/resolved-value rows. Invalid actions stay
   focusable with `aria-disabled` and cannot dispatch. Keep failed text drafts in their
   row with adjacent errors; successful commits re-read normalized Rust values.
   Removing a selected constraint keeps a surviving row focused. Dependency/diagnosis
   inspection uses stable IDs; never derive freedom arrows or conflict IDs from text.
4. **Left rail and one options owner (AC4, AC13–16, AC18).** Switch the existing rail's
   contents by workbench and feed the existing single floating options-bar owner.
   Expose the implemented named variants and relevant numeric fields, including the
   Select/Constraints inspection choice and Fillet preview/commit. No new global tool
   letters, second constraint bar or permanent instructional banner. Rust receives
   picks through the existing canvas pointer pipeline and evaluates typed construction.
5. **Canvas annotations and editing (AC3–5, AC7).** Render annotation labels at Rust's
   screen positions as a sibling overlay within the existing canvas container. Avoid
   adding a second child to an `asChild` trigger. Selection and double-click dispatch
   `sketch_inspect_constraint`; the same row Edit/Enter route opens the same draft.
   Keep mathematical highlights/geometry in the Rust draw list. Convert to paths calls
   the domain command then returns to Vector with the converted object selected.
6. **Input/focus and verification (AC1, AC3–4, AC17–18).** Reuse native input undo and
   document history routing outside text fields. Enter commits once; Escape restores
   the draft/cancels pending work before clearing selection. Valid Tab commit advances
   focus. Modifier key events must bubble to the existing tracker even from numeric
   entry controls: verify key-only Alt during idle hover, pressed construction and
   pending picks without pointer movement. Pointer commands return canvas focus;
   keyboard activation preserves control focus where the existing pattern requires it.

| Host action | Existing Rust bridge method |
| --- | --- |
| Switch / create / select sketch | `set_workbench`, `new_sketch`, `enter_sketch` |
| Tool and named variant | `set_sketch_tool` |
| Entity / relationship inspection | `sketch_select_entity`, `sketch_inspect_constraint` |
| Constraint or dimension creation | `sketch_apply_action` |
| Edit / remove dimension or constraint | `sketch_dimension_text`, `sketch_remove_constraint` |
| Add / edit / remove parameter | `sketch_add_parameter`, `sketch_parameter_text`, `sketch_remove_parameter` |
| Origin / construction / deletion | `sketch_fix_origin`, `sketch_selection_construction`, `sketch_delete_selection` |
| Numeric creation / fillet | `sketch_complete_numeric`, `sketch_preview_fillet`, `sketch_commit_preview` |
| Explicit conversion | `sketch_convert_to_path` |
| Read-only presentation | `sketch_view_json` |

Before exposing inspection/reveal or entry-focus behavior, check that the real bridge
record/command supports that behavior. Any concrete missing domain result is repaired in
Rust within the accepted scope; the host must not invent it or expose a mock button.

Focused frontend verification after wiring: TypeScript build, existing frontend tests
and lint, plus meaningful interaction regressions for dock collapse/workbench state,
row draft cancellation/native undo, modifier propagation, annotation edit equivalence
and constraint deletion focus. One browser owner then verifies the complete dimensioned
plate/holes demo, named formulas, conflict repair, creation variants, Alt, save/reopen,
clone re-entry and conversion undo. This is additional to independent CPU/parity and
the final tester-owned full gate; it does not replace either.

### Independent runtime defect corrections

- Feasible prior geometry is the primary backend seed; raw requested goals remain
  unchanged. Stagnation fallback cannot discard a valid primary pose on error.
- **Optimization (Proposal):** `drag_reach.rs` certifies the exact triangle-inequality
  reach bound for one endpoint of an unambiguous Distance chain. Every original hard
  constraint is independently validated; nonpath entities remain unchanged.
- Unchanged release acceptance command:
  `cargo test -p curvyo-editor-wasm --release --test acceptance_0066_chain --test acceptance_0066_interactions --offline -- --nocapture`.
  Chain1/1 and interactions4/4 passed (`target/0066-runtime/certified-reach.log`).
  Frame20 goal error311.77794983mm →252.799837866mm, matching the proven global bound.
  Pointer+view+draw Linux thread CPU: n27, median10.244546ms, max11.103949ms (<16ms).
- Stage diagnosis before correction: frame20 solve130.56ms, view3.09ms, draw2.29ms;
  retained in `target/0066-runtime/chain-stage-profile.log`. Temporary profiler source
  is outside test inventory at `target/0066-runtime/chain-stage-profile.rs`.
- Shared multi-pick guard refuses a duplicate second point before storing it, keeping
  the valid first pick. Final-pick failures still preserve valid preceding picks.
- Core drag/reach regressions7/7 passed, including disconnected geometry, fixed joint
  and extra Horizontal constraints. Architect approved the bounded optimization.
- Independent runtime rerun/parity and frontend work remain open; no full gate claimed.

### Selection continuity correction

Revision synchronization now intersects entity/constraint selection with the current
active sketch's authoritative graph, instead of clearing every ID after an edit.
Pending previews still cancel on revision change; object re-entry/New still reset
selection. Historical checkpoint-only entities cannot survive that intersection.
The unchanged independent `acceptance_0066_lifecycle::s12_dimensions_redundancy_conflict_repair_and_conversion_undo`
passes: successive constraints reuse selection, conflict repair and conversion undo work.
All16 focused session tests pass, including a new deletion/constraint removal/undo/New
selection guard (`target/0066-runtime/selection-{retention,guards}.log`).

### Conversion lifecycle correction

Successful `sketch_convert_to_path` now returns to Vector/Select with the converted
same-ID object selected. The workbench change is ephemeral, so conversion remains
one undo step; undo restores the editable graph and re-entry resolves that sketch.
Failure leaves the workbench and history untouched. The unchanged independent S12/S14
lifecycle path passes; all17 focused Session tests pass including success/failure,
exact selection, one-step history and undo/re-entry guards. Evidence:
`target/0066-runtime/conversion-{lifecycle,guards,clippy}.log`.

### Actual frontend implementation (AC1–7, AC13–18)

- Existing shell owns the workbench selector, collapsed V/S selector, Sketch left rail,
  one options bar and Sketches tab. Vector Trace/key routes are scoped to Vector.
- Rust view JSON supplies geometry, eligibility, diagnostics and expressions. Hosts keep
  only drafts. Draft subtrees are keyed by active sketch; Escape discards drafts and
  preserves native text undo/modifier propagation. Annotation presses never reach creation.
- Added concrete Rust inspection commands for free geometry, selected relationships and
  transitive parameter dependencies; reveal pans only offscreen selection. Typed Fillet
  preview parses through the existing unit/formula boundary.
- Actual browser workflow created a rectangle, parameter `plate_width = 80 mm`, Length
  driven by that parameter, circle and Diameter12mm. Draft cancellation/repeated Edit
  focus also passed. Evidence: `frontend/tests/browser/sketch-workbench.cjs` and `target/0066-browser/plate.png`.
- Browser evidence exposed ACS's own-variable-only primitive classification. Corrected
  full-entity classification includes every defining point (lines have only endpoint
  variables). Red/green core regressions preserve global DOF2 for fixed-radius/free-center
  circle and DOF3 for length-only free line; fully fixed variants remain classified full.
- Checks: focused Session20/20; solver8/8; affected native Clippy; `npx tsc -b --noEmit`;
  frontend108/108; `npx oxlint` (existing unrelated warnings, no new Sketch warnings).
  WASM regeneration and affected wasm32 Clippy passed. Saved browser regression also covers
  sketch-scoped drafts, conversion/undo, collapsed workbench switching and hidden Vector
  shortcuts. No full gate or final independent UX/acceptance PASS claimed.

### S5 rejected typed Fillet preview

Every typed Fillet request invalidates the older preview/base before parsing. Numeric
Apply uses the same path; invalid names, units, negative radii and empty input cannot
commit an older valid radius. Errors remain visible; corrected input creates a fresh
preview and commits once. Direct radius preview also clears earlier eligibility before
selection/revision checks. No document/history write occurs on refusal.

Unchanged independent `acceptance_0066_lifecycle::s13_invalid_typed_radius_cannot_commit_previous_valid_fillet_preview`
passes; focused Session21/21 and affected editor Clippy pass. Logs:
`target/0066-runtime/fillet-{stale-preview-fixed,session-guards,clippy,wasm}.log`.

WASM regeneration (`npm run build:wasm`) passed after the S5 correction.
