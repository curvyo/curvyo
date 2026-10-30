# Sketch session integration handoff

Status: implementation navigation notes (Proposal), 2026-10-30.
Authority: accepted specification and UX notes; `ui-interaction-map.md` specifies
the control interactions. These notes do not change the accepted criteria.
Document integration and independent review precede this milestone.

## Existing boundaries

- `curvyo-editor-wasm/src/session/mod.rs` owns `Session`, `Tool`, event routing,
  held modifiers and shared object selection. Add explicit workbench state and a
  separate sketch-editing state; preserve ordinary Vector tool behavior. No trait
  registry or extension framework is needed for these two built-in contexts.
- `session/keys.rs` owns keyboard routing and the Escape cascade. Dispatch sketch
  commands before ordinary Vector creation/editing commands while Sketch is active.
  Text inputs retain their own editing and local undo behavior.
- `session/panel_tabs.rs` adapts `curvyo-ui-core` panel rules. Register Sketches
  explicitly rather than imitating a frontend-only About Trace page. Active sketch
  selection must not let the ordinary empty/nonempty edge steal the tab on every
  frame. Other property tabs remain usable for the document and sketch object's style.
- `session/draw.rs` and `frame.rs` assemble drawing from read-only snapshots.
  Keep sketch points, construction geometry, origin, constraint cues, dimensions
  and previews in overlays; only the validated outline is artwork/output.
- `session/snapping.rs` has the shared cached targets and `snap_creation_point`.
  Its current Alt-inversion tool list names the ordinary creation tools explicitly;
  extend dispatch for Sketch instead of assuming new tools inherit the behavior.
- `wasm_shape_tools.rs` illustrates the thin wasm-bindgen command surface. Reuse
  the project's record conventions for view data. Domain selection, applicability,
  expression parsing and solves stay in Rust, never duplicated in TypeScript.

## Interaction state and commit boundaries

- Entering Sketch cancels only unfinished Vector input, preserves completed edits
  and the viewport, and re-enters the selected original sketch when possible.
  A linked clone offers its original rather than an independent graph. With no
  active sketch, Sketches offers New; changing workbench alone creates no object.
- Keep active sketch ID, tool/variant, entity/constraint selection, picks, drag seed,
  last validated preview and panel input errors as ephemeral session state.
  Clear stale IDs on undo, reopen, deletion and peer changes; never retain a graph
  from a previous document through New/Open.
- Selection uses analytic entities and point references, not derived Bézier anchor
  identities. Shift extends selection. Applicability and rejection reasons are
  exposed by Rust for all ten constraints and five dimension actions.
- Creation tools call core constructors. Polyline commits completed segments while
  retaining its shared endpoint identity; Enter finishes and Escape cancels only
  its uncommitted segment. Other multi-pick tools retain correct earlier picks
  after degenerate completion input. Fillet preview remains wholly ephemeral.
- Pointer down, hover without buttons, button-down movement, release and key-only
  modifier changes all use the same current snapping rule, including Alt inversion
  and independent horizontal/vertical switches. Snapping adds no hidden constraint.
- Constrained drag uses bounded Drag solves from a compatible validated seed.
  Hard constraints remain checked; infeasible or failed goals retain a feasible
  pose. Full rank/DOF analysis is not a per-pointer-frame operation. Release runs
  the commit validation and writes one explicit geometry delta plus checkpoint;
  solved dependent point records must not become authoritative edits.
- Dimension/formula edits use typed parsing. Geometry and resolved parameter
  values remain at the last valid state after invalid expressions. Numerical
  conflict and parse failure are different diagnosis paths. Retain the user's
  invalid input for correction, with the relevant row/entry identified.
- Workbench navigation, hover, selection, previews, diagnosis and tool/variant
  settings add no document history. A completed creation, drag, constraint edit,
  dimension/parameter edit or fillet is one named step using existing history.

## Existing frontend shell

- `frontend/src/components/panel/PropertiesPanel.tsx` owns the right dock and its
  collapse/focus behavior. Place Workbench above that dock, with full names when
  expanded and the approved compact accessible choice when collapsed. Preserve
  the current panel width and camera-origin behavior.
- Add a Sketches body and vertical tab using existing panel components/tokens;
  show sketch identity, creation/re-entry, DOF/status, applicable constraints,
  editable dimensions, parameters and repair/inspection here. No diagnostic data
  moves into the bottom application status bar.
- Existing tool cards expose the active workbench's tools. The ordinary Pen,
  Star/Polygon and Trace controls remain Vector tools. The existing single top
  floating options location renders Sketch tool variants/settings in place.
- Frontend hooks read the Rust view and forward actions. Keep keyboard focus,
  inline entry and accessibility behavior consistent with existing panel fields.
  Do not introduce study-only state buttons, a header, Finish bar or another rail.

## Required next evidence

Use focused Rust session tests for entry/re-entry, tool routing, all constructor
gestures, selection/applicability, origin release/move, constraint and dimension
editing, parameter propagation/error recovery, constrained drag cancellation,
one-step history and source/clone behavior. Include key-only Alt changes.

Use actual browser tests for Vector/Sketch switching, visible tools and single
options bar, property collapse, focus/keyboard editing, dimension double-click,
conflict inspection/repair and save/reopen. A static mockup and `cargo check`
alone do not establish these behaviors. Verify production native/WASM runtime
agreement and the actual 100-entity thread-CPU drag workload before the final gate.
Source-scaling behavior stays pending the specific customer decision; do not
silently select a policy while implementing the independent interactions.
