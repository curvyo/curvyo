# Existing sketch solvers: planning research

Date: 2026-10-30. Status: research only; no solver selected or dependency added.
No candidate was downloaded, built, tested or benchmarked for this report.
Facts below come from upstream documentation, manifests or source declarations;
quality claims are upstream claims, not Curvyo acceptance evidence.

## Finding and recommended next step

Existing Rust sketch-solver candidates do provide DOF and diagnostic features.
The old assumption that no Rust candidate exists is no longer a sound reason
to choose an own solver. **Proposal:** benchmark ACS first, with ARAEL as a
second candidate and PlaneGCS/SolveSpace as reference implementations where
practical. Keep the decision open until independently owned cases pass.

## Candidate facts and unresolved questions

| Candidate | Documented geometry and diagnostics | Integration and evidence | Still unproved for Curvyo |
|---|---|---|---|
| **ACS / `acs-solver`** | Rust 2D solver; points, lines, circles, circular/elliptical arcs, ellipses and splines. Reports DOF, fully constrained entities, conflicting and redundant constraints. Temporary constraints act as soft drag goals. | MIT per upstream; Rust crate, WASM API and optional C ABI; analytical-Jacobian checks against finite differences documented. Upstream names Part3D as a consumer. | Minimal/stable conflict sets, per-entity diagnosis near singularities, branch continuity, reproducibility, latency and clean `wasm32-unknown-unknown` integration need independent checks. |
| **`brepkit-sketch`** | DogLeg solver, analytic Jacobians, QR rank/DOF; published API includes points, lines, circles and `ArcData`. Duplicate constraints do not consume extra DOF. | Standalone crate without workspace dependencies; brepkit has a WASM distribution. Upstream current license differs from older releases (below). | Ellipse constraint coverage, actionable conflict IDs, drag goals/branch stability and standalone WASM build not established by reviewed sketch API docs. |
| **ARAEL / `arael-sketch-solver`** | Sketch editor documents points, lines, circles, arcs, ellipses, usual geometric constraints and rejection explanations for contradictory/duplicate constraints. Framework exposes SVD rank/null-space analysis. | MIT repository; Rust framework plus separate sketch-solver package, browser sketcher and WASM export documented. Optional C++ numerical backend exists. | Which UI diagnostics belong to the reusable crate, stable constraint-conflict IDs, constraint-preserving drags and dependency/features suitable for our core need inspection. |
| **Solverang** | Domain-neutral graph solver documents rank/DOF/conflict diagnostics; 2D package lists points, segments, circles and geometric relations. | Apache-2.0 manifest; Rust, separate geometry packages. Current manifest references a sibling `../methodus` path dependency. | Published-version versus current-branch API differs; independent package availability, WASM, ellipses/arcs and usable conflict sets must be verified before shortlisting. |
| **`slvs` / SolveSpace** | Rust wrapper exposes circle/arc entities, success DOF and failed constraint handles. Underlying C API has dragged-parameter preferences and redundant/inconsistent/nonconvergent result states. | Rust bindings to native SolveSpace, not a pure Rust solver; wrapper needs compiler/libclang. Wrapper GPL-3.0; upstream SolveSpace is GPLv3. | No ellipse coverage established; Rust-target WASM support and safe-core boundary unresolved. Native maturity does not prove wrapper portability or non-flipping drags. |
| **PlaneGCS / FreeCAD** | C++ API includes circles, arcs, ellipses/elliptical arcs, QR diagnosis, conflicting/redundant tags and temporary lower-priority drag constraints. | Used inside FreeCAD; a separate C++→WASM/TypeScript wrapper exists. FreeCAD header SPDX LGPL-2.1-or-later; wrapper carries LGPL 2.1 license text. | No native Rust crate found in this search; adapter ownership/FFI, required FreeCAD extraction and target-specific tooling need evaluation. WASM wrapper reports limitations for some non-driving constraints. |

Sources for these candidate rows:

- ACS: [upstream README](https://github.com/angelobartolome/acs),
  [published crate 0.1.4](https://docs.rs/crate/acs-solver/0.1.4),
  [current published API](https://docs.rs/acs-solver/latest/acs/solver/struct.ConstraintSolver.html).
- Brepkit: [published sketch API](https://docs.rs/brepkit-sketch/latest/brepkit_sketch/),
  [upstream README and license history](https://github.com/andymai/brepkit/blob/main/README.md).
  The README states versions through 2.129.x were MIT OR Apache-2.0, while
  3.0.0+ use AGPL-3.0-only with a commercial option. Check the exact selected
  sketch version and its own manifest; do not reuse an obsolete permissive label.
- ARAEL: [upstream packages, sketch editor and rank documentation](https://github.com/harakas/arael).
- Solverang: [current README](https://raw.githubusercontent.com/akiselev/solverang/master/README.md),
  [current manifest](https://raw.githubusercontent.com/akiselev/solverang/master/Cargo.toml).
- SolveSpace: [Rust wrapper](https://github.com/thekakkun/rust_slvs),
  [Rust API 0.6.0](https://docs.rs/slvs/0.6.0/slvs/),
  [solver C API](https://github.com/solvespace/solvespace/blob/master/include/slvs.h),
  [upstream library description](https://solvespace.com/library.pl).
- PlaneGCS: [FreeCAD solver header](https://raw.githubusercontent.com/FreeCAD/FreeCAD/main/src/Mod/Sketcher/App/planegcs/GCS.h),
  [WASM wrapper](https://github.com/Salusoft89/planegcs),
  [wrapper license](https://raw.githubusercontent.com/Salusoft89/planegcs/main/LICENSE).

License identifiers describe retrieved sources only. This report makes no legal
compatibility conclusion; an exact dependency/license review remains required.
Published docs, examples and application integration are maturity signals, not
proof of maintenance capacity, numerical correctness or our interaction budget.
`latest` URLs are discovery aids: record exact versions/revisions in the spike.

## Related kernels checked

[Truck](https://github.com/ricosjp/truck) presents a Rust CAD kernel, not a
documented reusable sketch constraint solver with conflict diagnosis in the
reviewed overview. This search does not establish that no related solver exists.
[Fornjot](https://github.com/hannobraun/fornjot) explicitly says development has
ended; its repository was archived on 2026-06-19. Neither is a stronger initial
candidate than an explicit 2D solver for this feature.

## What the benchmark must establish before an ADR

1. Map the accepted geometry and every constraint kind, including dimensions,
   circular/elliptical arc semantics, signed distances, fixed origin and units.
   Ellipse support is required by the customer's revised entity scope; test its
   constraint combinations rather than accepting a geometry-type declaration alone.
2. Check independent expected DOF on under/full/redundant/conflicting cases;
   distinguish numerical rank loss at a singular configuration from global
   geometric freedom. DOF zero alone is not evidence that residuals are valid.
3. Require original constraint IDs for diagnosis; test duplicate dimensions,
   incompatible radii and small conflicting groups. A conflict report is not
   automatically a minimal conflict set or a safe suggestion to delete any ID.
4. Drag closed loops through small and large steps, near tangent/degenerate
   states and competing branches. Verify hard constraints, last valid solution,
   recovery after failure and stable entity/constraint identities. SolveSpace's
   [own tutorial](https://solvespace.com/constraints.pl) explicitly documents
   solution flips and the usefulness of smaller incremental steps.
5. Measure the specification's 100-entity interaction target using thread CPU
   time, separating solve from rank/conflict analysis. Include dense systems and
   failed solves; do not extrapolate application/kernel marketing benchmarks.
6. Build the exact candidate feature set on desktop and Rust's wasm32 target;
   inspect foreign code, unsafe boundaries, transitive licenses, allocations and
   cancellation/iteration control. Compare repeatability to AC12's tolerance.
7. Use the same independently checked corpus for all shortlisted engines;
   retain Curvyo document/history/geometry authority outside solver results.

The product contract remains ours even when the numerical solver is reused:
document model, dimensions, diagnostics presentation, undo, geometry conversion,
clone behaviour and committing only validated candidate solutions.
