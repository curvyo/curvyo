# ADR 0026: Parametric sketch nodes and workbench state

**Status:** Proposed implementation record for the authorized document model and
persistence; `needs-customer` applies specifically to original-sketch scaling
semantics (2026-10-30), not to restarting approval of the accepted graph scope.
The customer authorized the feature and the plan's stable IDs, local coordinates,
placement and parameters/formulas. This record makes the implementation choices
reviewable; it does not reopen that authorization. External original scaling
semantics remain the specific pending customer decision; this does not block
the authorized graph, persistence and rigid-placement integration.

## Context

Sketches must remain editable through history, save/open and clone resolution.
Baking paths loses constraints. Storing the entire sketch in one last-writer-wins
register loses independent concurrent edits. Making both paths and constraints
editable creates conflicting authorities. Extend ADRs 0002, 0004, 0009, 0014,
0015 and 0016 with a distinct sketch node and derived outlines.

## Decision

### Authoritative graph and history (Proposal)

A sketch is one leaf object with ordinary style/layer flags and a separate affine
placement. Add an explicit `ObjectSnapshot::Sketch` variant whose resolved path
feeds the shared drawn-path reader. Ordinary path writers and Node commands must
refuse it unless an explicit conversion occurred; a disguised Path variant could
silently destroy the constraint graph. Its local graph owns stable entity, constraint and parameter IDs.
Persist each entity definition/seed, constraint definition and parameter
expression as its own atomic value keyed by ID. Each record couples its kind,
references and numerical payload; do not merge half of one definition with
half of another. Label and placement are separate registers. Use existing
history restore semantics at this granularity, never restore the whole graph
for a one-dimension undo. A topology operation writes its affected records in
one named history transaction; previews write no persistent operations.

Concurrent independent records merge. A missing referenced entity, incompatible
merged constraints or expression cycle produces inspectable diagnosis, not
silent constraint deletion or rejection of the entire document. Malformed
present records still fail format validation. Deletion removes references only
when the command explicitly includes them; concurrent dangling references remain
visible for repair. Undo restores IDs and references using existing conflict rules.

Persist one atomic last-valid solution checkpoint containing local geometry,
its source graph fingerprint and numerical seed/branch state. It is a derived
recovery/display record, never an independently editable graph. Validate its
structure and finite values. A matching graph reuses it; changed input is solved
from deterministic stored seeds/order. Failed solving displays the last-valid
checkpoint with a stale/conflict diagnosis. A concurrent checkpoint winner follows
the ordinary register rule and must not silently overwrite graph records.
Successful commands update graph and checkpoint together in their transaction.
Opening/reading alone never writes a new checkpoint. Test checkpoint/graph mismatch
and concurrent edit/undo explicitly; do not claim every merged graph is solvable.

### Geometry and topology (Proposal)

Canonical entities are points, lines, circular arcs, circles and ellipses.
Construction methods produce these same records; three-point input is temporary
unless the user explicitly adds persistent relationships. Polyline vertices and
segments have stable IDs, with shared point references expressing connectivity.
Rectangles and straight slots expand to primitives plus explicit constraints;
a small construction record retains their member IDs for parameter editing.
This is preferred to an opaque slot with a separate geometric interpretation.

Fillets are graph rewrites: preserve unaffected IDs, record trimmed references,
retain a virtual sharp intersection where dimensions require its intent and add
radius/tangency relationships. Virtual construction geometry is not output.
Do not retarget a sharp-corner coincidence onto distinct tangent endpoints.
If reference intent cannot be preserved, reject the whole edit with the affected
constraints. Removing a fillet restores its recorded topology only if subsequent
edits remain compatible; otherwise diagnose rather than discard later work.
Cancellation is inert; undo restores the transaction's topology and references.

### Placement, clones and conversion (Proposal)

Implemented authorized behavior solves in local coordinates and permits rigid
translation/rotation of the original through its separate placement. Existing
clone transforms can scale or mirror the resolved geometry without changing
the original constraint graph; a nonuniformly scaled clone can display an ellipse
from a source circle. This does not authorize scaling the original sketch.

**Pending customer choice — original scaling:** external placement scaling that
keeps local driving values is one proposal, not implemented behavior. Updating
driving values or retaining nonscaling originals are alternatives in the
customer question. Do not infer an answer from the affine storage representation
or existing scaled-clone behavior. Any chosen behavior must explain how local
construction dimensions relate to output measurements.

Resolve an original once per graph revision and expose its solved outline to
bounds, picking, snapping, rendering and output. Clone chains reuse that outline
through existing composed transforms and own-placement/style rules; clones own
no constraint graphs. Hidden originals still feed visible clones. Test original
conversion/deletion, chains, overrides and undo using ADRs 0015/0016 semantics.
Convert to paths preserves the object ID/style/placement and ends parametric
editing in one transaction; clones retain their source link. Emit closed loops
and approximate analytic curves within 0.01 mm in final document coordinates,
accounting for placement scale. Export includes solved outlines, never diagnostic
symbols or construction geometry.

### Format and workbench (Proposal)

The implemented codec writes format **28**, following main's snapping format 27
(`document.rs::CURRENT_FORMAT_VERSION`). Add golden round trips for graph, formulas,
checkpoint, conflict state, clones and conversion/undo. SID requires no old-file
migration. Existing readers refuse newer versions: embedded solved geometry
cannot retrofit fallback into them. Explicit conversion/export provides paths.

Vector/Sketch workbench choice, active sketch, entity selection, camera and
unfinished gesture state belong to the session, not shared document registers.
Leaving cancels the unfinished gesture; committed graph edits stay. The existing
shell gets a concrete Sketch workbench, not a speculative view/plugin framework.
The Sketches Properties pane contains status, DOF, diagnosis and parameters;
there is no bottom demo-state bar. Keep view-specific tools in the left rail and
active-tool options in the existing floating bar. Future manufacturing,
extension and simulation contexts introduce no API or implementation here.

## Consequences

Independent graph edits survive merging, but contradictions need repair UI.
Checkpoint storage costs space and requires mismatch tests. Derived geometry
cannot be edited using ordinary node tools without explicit conversion.
Scaled clones retain local source design intent; their output measurements can
differ from source dimensions. Original-sketch scaling remains undecided. No globally minimal conflict
set or automatic recovery from every nonlinear branch change is promised.
