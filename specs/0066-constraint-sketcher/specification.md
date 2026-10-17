# Constraint sketcher: construct geometry from constraints and dimensions, and keep it parametric

Status: Draft (a sketch for the customer; not Ready: it needs a solver decision, a model decision for parametric objects, core feature decided 2026-10-17)
Priority: Should (large; the customer: "also a lot of work")
Origin: Customer (request of 2026-10-17, translated): "I would like to be able to construct. Similar to SolveSpace or a sketch in FreeCAD you should be able to create geometry with constraints to specify origin, radii, sizes etc. Super practical for CNC milling and laser cutting. But also a lot of work :)"

## User value

As a maker who cuts and mills parts that must fit (a bracket, a box joint, a gear plate), I want to say what is true of the part ("this line is 40 mm", "these two circles have the same radius", "this arc touches that line") and let the program find the shape, so that I change one number later and the whole part follows, instead of nudging nodes until it looks right.

**What other tools do.**
- **SolveSpace**: a 2D/3D constraint sketch, a small own solver, shows the remaining degrees of freedom (DOF) as a number, constraints are points, lines and dimensions you add to selected entities. Over-constraint is reported with a list of the conflicting constraints.
- **FreeCAD Sketcher**: a Sketch is a separate object inside a Part workbench; geometry is drawn, then constrained (coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, symmetric, distance, radius, angle); the solver (planegcs) colours the geometry (white free, green fully constrained, red conflict); a sketch is an input for Pad or Pocket.
- **Fusion and Onshape**: the same model; Fusion paints under-constrained geometry blue and fully constrained black; dimensions can be driven by named parameters and formulas.
- **Inkscape, Illustrator, LightBurn**: none. This is the gap the customer sees.

## Model (Proposal)

- A **sketch** is an object in the document with its own points, lines, arcs, circles, a list of constraints and a list of named dimensions. It lives in one layer like a path.
- The **solved result** is shown as ordinary geometry. **Convert to paths** ends the parametric part and leaves normal paths (undo restores the sketch); until then the sketch is selectable, movable and styleable as one object, like a compound path.
- A sketch can be the original of a **linked clone** (`0049`, `0057`): the clone follows the solved shape live, and a changed dimension changes every clone.
- **Driving dimensions** are the dimensions of `0070` that are allowed to change the sketch (a number you type moves the geometry); **reference dimensions** only show a value.
- The solver is a function from (entities, constraints) to solved coordinates, with no clock, file or thread, so it belongs in a `*-core` crate and builds for `wasm32` (`CLAUDE.md` §6).

## Solver approach (options; the architect's notes)

| Option | For | Against |
|---|---|---|
| A. Own Newton / Levenberg-Marquardt solver on a sparse Jacobian, with a DOF count from the rank | Full control, wasm-clean, about 2 to 3 kLOC for the usual 2D constraint set, no foreign licence | We own the hard cases: redundant and conflicting constraints, drag stability, convergence near singular states |
| B. An existing Rust crate | Less code | The architect found no mature, maintained, permissively licensed 2D sketch solver crate in Rust (to be confirmed by a short spike before any ADR); a general optimisation crate does not give DOF or conflict diagnosis |
| C. Bind planegcs (FreeCAD, LGPL-2.1+) or SolveSpace's solver (GPL-3.0) | Proven | C++ in a wasm and desktop build; licences conflict with the dependency rules of ADR 0006 or need review; no `forbid(unsafe_code)` path |
Recommendation: A, behind a small input/output contract, with a spike on `spike/sketch-solver` that proves 12 constraint kinds and a 30-entity sketch before the ADR is written.

## Acceptance criteria (as far as clear)

1. Given the Sketch tool, then the maker draws points, lines, arcs, circles and rectangles into one sketch; every entity is constrainable. Options of the tool are in the top bar; nothing is added to the right panel for it.
2. Given constraints, then at least these exist as symbol buttons in the top bar: coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric, midpoint. Pressing one with a valid selection applies it; with an invalid selection the button is dimmed with a tooltip that says why (it stays focusable).
3. Given dimensions, then distance, length, radius, diameter and angle can be added by selecting entities and typing a value with a unit (`R-EDIT-017`; mm is the default); the value can be edited later by double-click on the dimension.
4. Given the origin, then the sketch has an origin point that is fixed by default and may be moved or released.
5. Given the sketch, then the **DOF count** is shown in the top bar as a number; geometry is coloured free (neutral), fully constrained (a second colour) and conflicting (red); colour is never the only signal (the count and a tooltip carry the same).
6. Given a drag of a free point or entity, then the rest follows the constraints at pointer speed (under 16 ms per frame for 100 entities, the budget is confirmed by the spike); a drag that would break a constraint stops at the limit and nothing is refused.
7. Given a conflict or a redundant constraint, then the offending constraints are highlighted together and the maker can delete any one of them; the sketch keeps the last valid solution meanwhile.
8. Given Convert to paths, then the result is ordinary paths and arcs-as-Bézier at the kernel tolerance (0.01 mm), closed where the sketch loops close; one history step.
9. Given a dimension changed in a sketch that has linked clones, then the clones follow.
10. Given the file, then a sketch is a new node kind with its constraints stored as data; a `format_version` bump and a reader rule (older readers keep the solved geometry as a path) are decided by the ADR.
11. Given undo, then every constraint, dimension edit and drag is one history step with an operation name.
12. Given determinism, then the same sketch solves to the same coordinates on every platform to 1e-6 mm.

## Out of scope

- 3D sketches, assemblies, parametric 3D parts and any CAD kernel.
- Formulas and named parameter tables (a later entry; a driving dimension holds a number for now).
- Automatic constraint inference while drawing (Proposal for a later entry; the first version has the maker apply constraints).
- Splines with constraints on control points other than coincident and fixed.
- DXF/SVG import that infers constraints.

## Decided (customer, 2026-10-17)

- **SKETCH-HOME: core** (customer: "by feeling"). The sketch node, the solver, the Sketch tool and its Properties block are built in, not an extension. It is not the first extension of `0065`.

## Open questions (customer; each has a default)

Frage 1 (SKETCH-SOLVER): Own solver or a binding?
- A (default): own Newton/LM solver after a spike. B: bind planegcs (LGPL) and check the licence. C: wait for a crate.
- Default if unanswered: A, the spike only; no ADR before the customer reads it.

Frage 2 (SKETCH-START): What is the smallest first slice?
- A (default): lines, circles, arcs; coincident, horizontal, vertical, equal, tangent; length, radius, angle; DOF count; Convert to paths. B: add symmetric, midpoint, patterns. C: all of criteria 1 to 12 at once.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-042 (new)
Depends on: `0049-linked-clone`, `0070-measure-and-dimension` (the dimensions), `0016-boolean-operations` (kernel tolerance); needs ADRs: document model (parametric node, `needs-customer`), geometry/solver
Related: `0057-clone-overrides`, `0060-snapping` (snap to sketch points), `0063-manufacturing-workbenches` (CNC use)
ADRs: a new ADR for the sketch node and solver, `needs-customer`
PR: -
