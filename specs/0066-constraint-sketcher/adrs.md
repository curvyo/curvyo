# Architecture for constraint sketcher

2026-10-30. Customer authorized implementation after the view review; the bottom
status/demo bar is removed. Status and diagnosis belong in Sketches Properties.
The authorized interaction revision supersedes multiple-sketch creation: one
owning Sketch object per page, centred initial local origin, direct workbench
entry, separate fx parameters, no Sketch snapping, Alt drawing-mode cycling and
Tab numeric entry. Detailed lifecycle/collision rules are proposals in ADR 0026.
The latest customer direction gives each original a dedicated source page;
composition elsewhere uses linked clones. Automatic dedicated-page creation when
entering Sketch from a populated Vector page remains the pending question's
recommended proposal, not an assumed answer. Independent original-copy APIs do
not define the normal product composition flow.
Earlier pending-scope statements in planning files need reconciliation by their
owners with the accepted plan; they do not revoke the latest authorization.

- [ADR 0025](../../docs/adr/0025-sketch-core-and-solver.md): proposed pure sketch
  core, concrete solver boundary and feasibility gate. ACS 0.1.19 is the
  provisionally reviewed production adapter after the recorded feasibility
  checkpoint and production validation; full story acceptance remains separate.
  Its informational transitive `paste` advisory has a reviewed, narrowly scoped
  exception disposition documented in ADR 0025; the lead owns the gate rerun.
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): proposed,
  implementation record for the authorized model/persistence; needs-customer
  specifically for original-sketch scaling. Stable local graph IDs, individual
  atomic records, last-valid checkpoint, topology repair, placement, clone
  outlines, format and concrete session-only workbench state.
- ADRs 0002/0009/0014: units, per-register collaboration and history restoration.
- ADRs 0015/0016: clone source resolution, atomic links, overrides and repair.
- ADR 0004: current-version validation and refusal of newer files; no automatic
  older-reader path fallback. SID old-file migration is explicitly unnecessary.
- ADR 0003: path approximation tolerance is distinct from solver residual and
  cross-platform comparison tolerances.

## Reviewable implementation choices (Proposal)

No new customer question is implied by the already accepted feature/model plan.
External original scaling semantics remain the specific pending customer
decision; rigid placement and graph/persistence integration are authorized.
Other reviewable details in ADR0026: constraints/parameters merge per atomic record;
invalid merged graphs show last-valid geometry; slots use constrained primitives;
fillets preserve references or leave the sketch unchanged. These choices do not
introduce extra features or a general view framework.

The initial core milestone and document integration have scoped source approval;
this does not establish full feature acceptance. Remaining numerical failures
require correction or explicit product discussion, never weakened criteria. Tests must
cover native/WASM agreement, all accepted entity/constraint combinations,
conflicts, singularities, repeated drags and the 100-entity CPU budget. See ADR0025
for attributed feasibility measurements and core-api.md for implementer-reported
checks; no full-feature PASS or final performance acceptance is claimed here.

Format fact: current story writes 28 after main's format 27; old-file migration
is not required. Scaled linked clones retain existing transform semantics and
do not imply approval of original-sketch scaling.

Note 2026-10-31 (revision 3 review): format 28 is unreleased, so it gains relation variants
without a version bump, inside ADR 0026's relation vocabulary: `SymmetricPoint` (mirror about a
centre point; `Symmetric.axis` is a line again), `AxisAngle` (directed angle from local +X to a
line, ACS `direction`) and `EllipseRadius` (major or minor semi-axis). Typed line/slot angles and
ellipse radii are driving dimensions (AC39). ACS 0.1.19 has no major-radius relation (the major
radius is derived); it is encoded as an operand point on the major axis end (`ellipse_axis`) at
the typed distance from the centre. The minor radius uses `equal` on the ellipse's `radmin`.

Note 2026-10-31 (customer revision 4, lead decision A): format 28 shipped in `main` with PR #228,
so the new relation variants `HorizontalDistance` and `VerticalDistance` (signed `b - a` along the
local X or Y axis, ACS `horizontal_distance` / `vertical_distance`) bump `CURRENT_FORMAT_VERSION`
to 29 under the rule in `document.rs`. No version-28 record changes shape; migration is none and
the version-28 goldens still open. Goldens: `sketch_*_v29.curvyo`.
