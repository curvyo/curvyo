# Actual frontend handoff

Status: Implemented; focused checks and production-browser workflow passed.
Independent end-to-end acceptance, final UX review and full repository gate remain open.
Original non-rigid source scaling remains the separate pending customer decision.

## Review entry points

- `frontend/src/hooks/useSketch.ts`, `lib/sketchView.ts`: Rust view plus ephemeral host drafts.
- `components/sketch/SketchesPanel.tsx`, `SketchParameters.tsx`, `SketchField.tsx`:
  relationships, dimensions/formulas, parameters, diagnosis, inspection and native text editing.
- `SketchTools.tsx`, `SketchAnnotations.tsx`: existing left rail/one options owner and
  screen annotations; annotation events never reach drawing gestures.
- `App.tsx`, `PropertiesPanel.tsx`, `Canvas.tsx`, `useEditorSession.ts`: workbench selection,
  collapsed-panel preservation, Trace/Vector routing isolation, current annotation positions.
- `session/sketch/inspect_links.rs`, `numeric.rs`, `wasm_sketch.rs`: concrete Rust inspection
  and typed Fillet preview; no host geometry/formula interpretation.
- `curvyo-sketch-core/src/solver_input.rs`: full-entity classification includes defining
  point variables, preserving the separately computed global rank/DOF. Architect approved.

No new dependency, format change or extra shell surface was introduced in this milestone.
Geometry disclosure is collapsed by default, retains every entity and has native keyboard access.

## Checks actually run

- `cargo test -p curvyo-editor-wasm --lib session::sketch`: 20 passed.
- `cargo test -p curvyo-sketch-core --test solver`: 8 passed; circle/line classification
  regression first failed, then passed without changing global DOF expectations.
- `cargo clippy -p curvyo-sketch-core -p curvyo-editor-wasm --lib --tests --no-deps -- -D warnings`: passed.
- `cargo clippy --target wasm32-unknown-unknown -p curvyo-sketch-core -p curvyo-editor-wasm -- -D warnings`: passed.
- `cargo fmt --all --check`, `git diff --check`: passed.
- Frontend `npm run build:wasm`, `npx tsc -b --noEmit`, `npm test`: passed (108 tests).
- `npx oxlint`: exit0, existing warnings remain; no new Sketch warnings.
- `frontend/tests/browser/sketch-workbench.cjs`: passed actual creation, named formula,
  dimension, Escape discard, repeat Edit focus, active-sketch draft isolation, conversion
  and undo, collapsed workbench switching, hidden Vector shortcut protection; zero page errors.

Logs: `target/0066-runtime/frontend-*`, `entity-rank-{red,green}.log`.
Browser log: `target/0066-browser/workflow.log`.
Actual production screenshot: `target/0066-browser/plate.png` (1440×1000).

## Browser handoff

Vite is running on `http://127.0.0.1:5173/` (tool session30999).
Runtime launch, from repository root:

```sh
NODE_PATH=target/browser-check/node_modules \
PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers \
LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib \
node frontend/tests/browser/sketch-workbench.cjs
```

The local sandbox requires escalation for localhost server/Chromium processes.
No browser/test/build process remains active at handoff except that reusable Vite server.
Independent coverage should exercise the full accepted geometry/relation matrix, conflict
repair, persistence/export/clones and browser keyboard/modifier cases; this focused host
workflow does not replace that matrix or earlier independent CPU/WASM parity evidence.
