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

### Page ownership revision (customer direction; detailed rules Proposal)

The customer now requires one owning Sketch object per page, direct entry from
the Sketch workbench, no New/picker, a page-centred initial origin, and no Sketch
snapping. This supersedes the earlier multiple-sketch interaction. Ownership is
an explicit page identity relationship, not containment: frames can overlap and
objects can cross their edges. Derived clones do not own a page sketch.

**Customer refinement:** the original sketch has its own dedicated source page.
Composition elsewhere uses existing linked clones of that original, preserving
one source of parametric intent. Do not offer independent original duplication
as the ordinary composition workflow. The handling of entering Sketch from an
already populated Vector page was asked separately (Q7). In the absence of a response,
implementation proceeds with reversible **(Proposal A)**: create and activate a dedicated
source page, preserving all existing content. This is a reviewable detailed entry choice,
not an inferred customer answer. Do not infer approval of mixed original/artwork pages.
This does not require a generic page-type or workbench framework.

**(Proposal)** Store one optional original `NodeId` in a dedicated page-owner
register beside the existing per-page registers. Use the ordinary register
merge/history rules. Each original also records its creation/assignment page ID
as provenance, so a losing concurrent claim remains attributable; this backlink
does not elect an owner independently. Copy remaps it to the destination page;
clones do not copy ownership claims. Validate references on read: a deleted, converted or missing
node is not an editable owner. Do not choose an unrelated spatially nearby sketch
as a replacement. Reads/navigation never repair or write this register.

An empty page exposes an ephemeral graph with a fixed local origin `(0, 0)`.
The first durable graph edit creates the original, graph/checkpoint and owner claim
in one history transaction. This includes a named parameter before any geometry; pure
view changes and cancelled input still create nothing. Its initial global translation is
`page.origin + (page.width / 2, page.height / 2)`; this also handles nonzero page
origins. A cancelled first gesture creates nothing. Undo of that first creation
removes its object and claim together; redo restores the same identities. Deleting
all geometry from an existing sketch does not implicitly delete the object.
Switching page cancels pending gestures/numeric drafts and clears entity selection
before resolving that page's owner; revision guards prevent a preview committing
to a different page or a newly competing owner.

Deleting or converting the owning original clears its claim in the same command,
only if that claim still names this original (never clear a peer's new owner);
the next creation can materialize a new owner. Preserve existing clone repair
and conversion semantics. Page deletion retains global artwork, as existing
spatial-page behavior does: its owner is not reassigned to another page. Undoing
page deletion restores the association. Initial centring is not a live equation:
page resize never silently recentres the sketch. Existing explicit page-content
movement still applies its ordinary object transform exactly once.

The normal composition command creates/pastes a linked clone and never copies
page ownership. Existing clone placement, chains and overrides remain available,
including several clones on a composition page. The lower-level independent-copy
API may retain remapped graph IDs and occupied-page refusal as guarded document
infrastructure, but is not authorization to expose an original-paste composition
flow. Never silently merge graphs or convert to paths to bypass the singleton.
Explicit original conversion remains a separate intentional operation.

Concurrent first creation can produce distinct Loro tree nodes. The page-owner
register deterministically selects one; its losing node must remain intact as a
visible, diagnosed non-owner conflict object, not a second editable page sketch.
Never discard its graph, silently combine graphs, or write a repair during read.
Use existing diagnosis/repair surfaces, not a new conflict-management UI.
Explicit resolution may remove a conflict object or convert it to ordinary paths;
ordinary clone links to either original remain resolvable. Test this collision,
its undo and deletion/recreation races. This exceptional merge state is not a
second-sketch creation command or a hidden picker.

Keep these additions in the unreleased format 28 and extend current-format
goldens/merge/history fixtures; SID old-file migration remains unnecessary.
No source-original scaling policy is implied by page ownership.

### Axis constraint representation (Proposal)

Use an explicit `Relation::OnAxis { point, axis: Axis::X | Axis::Y }`.
`X` means the point lies on the local X axis (`y = 0`); `Y` means the
local Y axis (`x = 0`). These are the fixed local coordinate-frame axes, not
lines attached to the mutable origin entity. Releasing/moving that point does
not translate the frame or move other axis-constrained points. Original rigid
placement moves the complete frame and graph together. Thus the origin point
starts fixed at local zero but is not an implicit dependency of `OnAxis`.

Alternative 1: extend Horizontal/Vertical with point-pair overloads. This makes
the existing line relation ambiguous and still needs a zero-frame reference.
Alternative 2: a general coordinate-value relation with an expression. That
adds editable coordinate dimensions beyond this automatic zero-axis operation.
The concrete enum is smaller; do not introduce hidden fixed/helper geometry.

Validate that `point` references an existing Point and reject reference-only
mode for this geometric relation. Expand to a hard ACS coordinate equation with
value zero, never a temporary drag goal. Independently check the corresponding
solved local coordinate with the existing explicit length residual tolerance.
At the intersection emit two separately identified, inspectable/removable
relations. Preserve original IDs in diagnostic mapping and include the point in
reference traversal, selection, deletion/repair and serialized relation tags.
Tangent contact seeds are unaffected. Extend format-28 goldens, conflict/repair,
released-origin and native/WASM parity tests; no new backend or tolerance policy.

### Perpendicular continuation at an endpoint (Proposal)

Keep the existing `Perpendicular(line, line)` contract unchanged. Add the
explicit `PerpendicularAt { a, b, point }` relation for connected Line/Arc pairs
containing an arc. Validate that `point` is an actual shared endpoint identity
of both entities, rather than infer a joint from near-coincident coordinates.
Unsupported entity types and reference-only mode are rejected. The shared point
and both entities participate in reference traversal/remapping/deletion repair.

The hard relation means perpendicular analytic tangent directions at that joint.
Independently check their normalized dot product against the existing angular
residual tolerance and reject degenerate directions. For Line/Arc, ACS 0.1.19's
native `normal(line, arc)` equation places the arc centre on the line carrier;
shared endpoint structure makes this exactly the required joint relation.
For Arc/Arc, perpendicular radius vectors from each centre to the joint are
equivalent to perpendicular tangents. Map them to ACS perpendicular operands.
Its JSON vocabulary requires line operands: transient descriptors reference only
the existing centre/joint point IDs, adding no unknowns, points or persisted
construction entities. These are equation operands, not hidden sketch geometry.
Map their equation diagnostics back to the single original constraint ID.

Do not freeze the initial angle or insert visible/hidden construction normal
lines into the domain graph. Later dimensions must re-solve the hard relation.
Left/right perpendicular arc modes choose the initial analytic branch/seed;
the relation itself is the same perpendicular joint condition. Test both mode
branches, both mixed entity orders, Arc/Arc, later dimension changes, malformed
joints, remap/history and native/WASM parity. This is not a generic curve-angle
feature or an expansion to unrelated ellipse selections.

### Dimension label placement (Proposal)

Persist optional label positions as document-owned metadata keyed by stable
constraint ID, separate from the core `Constraint` and `Sketch` graph. Each
position is one atomic finite local-coordinate pair in millimetres. Missing
metadata uses the existing automatic placement. Do not include these records in
the solver input, graph fingerprint or last-valid numerical checkpoint: moving
a label changes presentation, never geometry, DOF or solved branch selection.

The placement gesture is ephemeral Session state until the dimension is
confirmed. Commit the dimension and its label position in one history step;
cancel/error writes neither. If an existing position-edit interaction changes a
label, that edit uses undoable metadata with no persistent pointer-preview writes;
this representation does not add a new label-drag tool to scope. Use existing per-field
history guards for concurrent moves. Label-only revisions refresh presentation
but reuse validated graph/checkpoint solve results.

Normal relation deletion removes its position record in the same transaction.
After merges, display metadata only for IDs in the current authoritative graph;
orphan metadata cannot resurrect a relationship or a historical checkpoint label.
Retain repairable orphan metadata without read-time writes, so concurrent undo
and repair can use ordinary register semantics. Copies remap label keys with
their constraint IDs; original rigid placement transforms local label positions.
Derived clones do not acquire independently editable annotation records and
production output omits these labels, like other Sketch diagnosis overlays.

Alternative: adding position to core `Constraint` couples presentation edits to
solver validation/fingerprints and wastes recomputation. Session-only placement
would lose the user's confirmed layout on save/open and provide no document undo.
Use this small document map instead; no generic annotation subsystem/dependency.
Keep unreleased format 28 and extend golden round trips, copy/remap, deletion,
undo/merge and label-only no-extra-solve tests.

### Original transforms and derived clones (Proposal)

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

Note 2026-10-31: format 28 shipped in `main` with PR #228. Customer revision 4 adds the
relation variants `HorizontalDistance` and `VerticalDistance` (signed `b - a` along the local
X or Y axis), so `CURRENT_FORMAT_VERSION` is now **29**. A version-28 reader would refuse the
new records, so it must refuse the whole file as too new. Migration from 28 is none: no
version-28 record changes shape. The `sketch_*_v28.curvyo` goldens stay as older-version
fixtures that still open; current goldens are `sketch_*_v29.curvyo`. Details:
`specs/0066-constraint-sketcher/adrs.md`. The mentions of "unreleased format 28" above
record the state before PR #228.

Vector/Sketch workbench choice, active sketch, entity selection, camera and
unfinished gesture state belong to the session, not shared document registers.
Leaving cancels the unfinished gesture; committed graph edits stay. The existing
shell gets a concrete Sketch workbench, not a speculative view/plugin framework.
The Sketches Properties pane contains status, DOF, diagnosis and existing
relationships; named parameters/formulas move to the separate fx tab.
there is no bottom demo-state bar. Keep view-specific tools in the left rail and
active-tool options in the existing floating bar. Future manufacturing,
extension and simulation contexts introduce no API or implementation here.

Sketch bypasses the Vector snapping engine, settings, axis masks, targets and
Alt inversion. Vector may still snap to a Sketch object's resolved outline.
Only previewed axis auto-constraints become solver-validated
persistent relations on confirmation. Endpoint continuation/closure uses explicit
shared identities; chosen tangent/perpendicular modes express explicit intent.
Alt cycles applicable drawing modes without committing geometry; Tab enters and
cycles numeric fields. Neither shortcut changes persisted graph/model policy.

## Consequences

Independent graph edits survive merging, but contradictions need repair UI.
Checkpoint storage costs space and requires mismatch tests. Derived geometry
cannot be edited using ordinary node tools without explicit conversion.
Scaled clones retain local source design intent; their output measurements can
differ from source dimensions. Original-sketch scaling remains undecided. No globally minimal conflict
set or automatic recovery from every nonlinear branch change is promised.
