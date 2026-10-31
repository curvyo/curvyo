# Sketch core handoff

Status: first production core milestone, 2026-10-30; not feature acceptance.
Boundary: ADR 0025. This report covers the core milestone; document, history,
editor, renderer and format integration are subsequent work on the same branch.

## Public model and solver

`curvyo-sketch-core` is a pure workspace crate with `forbid(unsafe_code)`, no
Curvyo crate dependencies, and the existing kurbo/serde/thiserror dependencies.
The only new numerical dependency is exact MIT `acs-solver = 0.1.19`.

- `Sketch::new(origin_id)` creates a fixed local origin. `entities`, `constraints`
  and `parameters` are deterministic maps keyed by distinct stable ID types.
- `Entity` contains Point, Line, Circle, Arc and Ellipse. Lines/arcs share point
  references; ellipses use centre/focus/minor radius. Coordinates use `Length`,
  angles use `Angle`; `Position::mm` is the explicit adapter convenience.
- `construction` hides geometry from output without removing its constraint
  intent. Fillets retain original lines and the original virtual corner this way.
- `contact_seeds` is derived branch state. Store it in the fingerprinted solved
  checkpoint, not as a separately editable graph authority. Remove its entry
  when removing or replacing the associated tangent constraint.
- `Relation` implements the ten geometric meanings and five dimension meanings;
  internal `On` and `Sweep` support analytic construction. A reference-only
  dimension is `Constraint { reference: true, .. }`; use `measure` to display it.
- `Expression::parse` resolves parameter names to stable IDs. Names must be
  referenceable; arithmetic supports +, -, *, /, signs and parentheses with
  length/angle/scalar typing. Units: mm, cm, m, in, rad, deg/°. Evaluation and
  dependency traversal are iterative; cycles, unknown names and invalid units fail.
- `solve(&Sketch, &SolveMode)` never mutates input. Edit mode returns current DOF,
  original conflict/redundancy IDs and per-entity fully-constrained IDs. Drag mode
  uses a soft point goal, twelve backend iterations, no rank analysis and explicit
  `Preview` status. A preview never masquerades as current full diagnosis.
- Only independently validated geometry appears in `SolveResult.candidate`.
  Preserve the previous checkpoint when it is absent or the call errors.
  Fixed parameters, implicit arc endpoints, dimensions, geometric residuals and
  analytic tangent contacts are checked independently. Zero DOF is not success.

Generic Tangent uses analytic carriers; shared endpoint joints remain actual
finite contacts. `On` remains finite. General ellipse/arc contacts use hidden
contact geometry with original-ID mapping. Solver identities never persist.

## Construction and output

`create_polyline`, `create_rectangle`, `create_circle`, `create_circle_three_points`,
`create_ellipse`, `create_ellipse_three_points`, `create_arc_three_points`,
`create_slot` and `fillet` return new sketches. Prefixes are caller-allocated
operation identities; collisions fail. Three-point ellipses pass through the
actual third pick rather than treating its perpendicular offset as an axis radius.

`fillet` currently handles joined straight segments, preserving their old IDs as
construction references. Oversized radii fail without input mutation. This is
not yet evidence for every proposed fillet editing/repair workflow.

`outline(&Sketch, tolerance)` returns kurbo `BezPath`; document integration maps
that result to existing path geometry. Point geometry, construction entities,
parameters and dimensions are not output. Explicit Coincident relations join
contours; merely nearby coordinates do not. Closed cycles survive attached spurs.
Use the validated solved checkpoint as input, never failed candidate intent.

## Checks actually run

- `cargo test --offline -p curvyo-sketch-core`: 21 tests passed, including an
  eleven-case geometric relation matrix, radius/diameter/reference cases,
  dimension propagation, 10,000-parameter dependency chain, construction and
  injected-invalid-candidate guards.
- `cargo clippy --offline -p curvyo-sketch-core --all-targets -- -D warnings`: passed.
- `cargo build --offline --target wasm32-unknown-unknown -p curvyo-sketch-core`: passed.
- `cargo clippy --offline --target wasm32-unknown-unknown -p curvyo-sketch-core -- -D warnings`: passed.
- `cargo fmt -p curvyo-sketch-core -- --check`: passed.
- Independent tester: `cargo test --offline -p curvyo-sketch-core` passed 32/32,
  including eleven separately written acceptance cases; affected Clippy, format
  and `cargo deny check licenses` passed. See `acceptance-core-report.md`.
- Architect approved the public core API for document integration after reviewing
  the candidate guards; see `review-core-architecture.md`.

Native/WASM runtime comparison and thread-CPU numbers currently come from the
separate feasibility corpus; production-runtime performance is not yet proven.

## Remaining before story acceptance

- Extend independent acceptance coverage through document and editor integration.
- Production drag performance, bounded failure recovery, repeatability under
  ordering/disconnected components and large graphs; no geometry limits added.
- Broader rotated/near-circular helper branch continuity, diagnostics for helper
  equation groups, arc carrier versus finite joint regressions and slot/fillet
  radius-change/topology repair histories.
- Deep expression-tree lifecycle/serialization stress beyond iterative evaluation;
  no arbitrary customer-visible expression-length limit is introduced.
- Full document/checkpoint encoding, import validation, undo, peer merge, clones,
  conversion/export, placement/scaling decision and UI/browser interactions.
- Dependency-policy check and full repository gate before the end-to-end PR.
