# Independent document acceptance matrix

Status: planned before document implementation review, 2026-10-30.
Basis: accepted `specification.md` and `document-plan.md`; complements the existing
acceptance plan and bounded core report. No document production code was read and
no tests or builds were run for this planning step. Executable cases await the
explicit tester handoff and stable public document API.

Scaling is an outstanding customer decision. Only rigid translation/rotation is
specified below; neither an assumed dimension-scaling policy nor a new refusal
policy is an acceptance oracle. Format 28 is the planned next version after 27;
verify its final allocation against the approved model before writing fixtures.

| Case | Criteria | Setup and independent expected result |
| --- | --- | --- |
| D01 Current-format persistence | AC3/10 | Formula plate: width 80 mm, margin 10 mm, spacing width−2×margin=60 mm; analytic circle/ellipse, construction records, driving and reference dimensions. Save/open retains object, entity, constraint and parameter IDs, expressions and units, origin, style, placement, last-valid result and editability. Change width to 100 after reopen: spacing and actual geometry become 80. Version is 28 if allocation remains approved; no old-reader/migration requirement. |
| D02 Golden inventory | AC8–10/16 | Golden files cover editable plate, slot/fillet virtual corner, sketch with clone chain, dangling/conflicting graph with valid historical checkpoint, stale checkpoint, and converted path. Inspect semantic content and geometry independently of codec roundtrip; classify every malformed fixture explicitly rather than skip by broad prefix. Newer unsupported version refuses; current archive members agree with manifest. |
| D03 Per-ID edit/history | AC3/11 | Edit one parameter, one dimension, one entity seed and remove one constraint separately. Each successful operation is one named history step; only intended authoritative records change, solved dependants live in checkpoint. No-op, preview, reads and diagnosis add none. Undo/redo restores associated intent, geometry and IDs; remove/revive sketch restores same editable identity. |
| D04 Local invalid draft versus contradictory intent | AC3/7/11 | Locally malformed structural records and invalid formulas (unknown reference, unit error or cycle) are refused atomically; a UI draft may remain available for correction. A well-formed contradictory constraint or dimension is instead retained as inspectable, explicitly removable intent in a named undoable step, while the last-valid geometry remains unchanged. Diagnose affected original constraint IDs; removal repairs the sketch, and undo/redo restores both intent and truthful diagnosis without publishing invalid geometry. Invalid fillet is refused without losing existing relationships. Post-merge well-formed cycles/dangling references remain repairable stored intent. No blanket rejection of contradictions is permitted. |
| D05 Independent concurrent edits | AC3/10/11 | Two peers edit different parameter IDs from shared base; merge both orders retains both intents, deterministic resolved geometry, and no writes on read. Undo peer A's own edit after merge preserves peer B's record and recomputes/validates the resolved view safely. |
| D06 Same-record atomicity | AC10/11 | Peers concurrently change one entity definition and its construction flag, or one formula/dimension record. Result is one complete winning record, never mixed fields. Both merge orders converge. Checkpoint conflict must not overwrite authoritative intent. |
| D07 Delete/reference race | AC7/10 | Peer A deletes an entity/parameter while B adds a relationship referencing it. Merge retains well-formed dangling intent and diagnosis; no silent deletion or whole-file rejection. Repair/removal is undoable. A formula cycle created only by combining valid peer edits is retained and repairable. |
| D08 Structural corruption | AC10 | Wrong scalar/container record type, malformed entity/constraint variant, wrong field type, non-finite geometry, malformed checkpoint payload or impossible structural ID record rejects import as damaged. Missing referenced IDs, well-typed cyclic formulas and contradictions remain distinct repair states. Do not conflate successful JSON decoding with domain validation. |
| D09 Stale checkpoint recovery | AC5/7/10/12 | Retain old checkpoint while changing width intent 80→100; match authoritative result 100, not stale 80. Repeat reads and save/open produce no operations or history. On impossible new intent, retain authentic last-valid outline with truthful stale/conflict/failure diagnosis, never fabricated valid zero DOF. Test both checkpoint-winning merge orders. |
| D10 Forged matching checkpoint | AC5/7/10 | Construct a checkpoint claiming matching fingerprint/FullyConstrained but with displaced fixed origin, wrong dimension residual, missing/extra candidate entity IDs, changed construction membership or inconsistent arc endpoint. Hash match/status text cannot authenticate candidate geometry. Reject structurally malformed data; otherwise validate/recompute or expose failure, never publish forged geometry as current valid solution. Keep authentic historical fallback separate from forged data. |
| D11 Fingerprint intent boundary | AC10/12 | Equivalent authoritative maps inserted in different orders resolve identically. Changing entity/constraint/parameter intent invalidates stale checkpoint. Style and rigid placement changes preserve local graph and local solution; neither can make forged geometry trusted. Contact seed remains derived checkpoint state; removing its constraint cannot leave authoritative dangling seed data. |
| D12 Clone propagation | AC9 | Original → clone → clone chain with distinct rigid placements and style overrides. Width change updates every linked shape; original retains editable graph, clones remain linked outputs without independent graphs. Verify exact line endpoints/bounds after known transforms, inherited versus explicit styles, and source ID preservation. |
| D13 Clone lifecycle | AC9/11 | Hidden source, chain source removal, unlink and source conversion follow existing clone rules. Resolved visible shape/overrides survive required materialization; graph is not silently copied into clones. Undo restores source identity and live links; later parameter edit proves restored propagation. Cycle/orphan handling never publishes non-finite output. Compare pre-existing clone contract rather than invent new behavior. |
| D14 Rigid placement | AC8–10 | Translate local plate by (30,−20) mm then rotate 90° about a specified known pivot. Independently calculate world endpoints; local dimensions, graph records and parameter values remain identical. Repeated read/save/open cannot apply placement twice. History restores exact placement and local intent. No scaling assertion before customer decision. |
| D15 Conversion accuracy/history | AC8/11 | Convert translated/rotated circle, noncircular ellipse, arc, closed slot and loop with attached spur. Check analytic-to-Bézier deviation ≤0.01 mm in final coordinates using independent dense sampling plus known extrema; record sampling limitations. Closed contour stays closed, spur retained, construction/annotations absent. Preserve ID/style/source links; remove parametric records coherently; one undo restores editable graph and linked propagation. |
| D16 Writer boundaries | AC8/9 | Ordinary path-writing/node-edit entry points cannot silently alter a Sketch through its derived outline. Exercise documented explicit conversion path, then verify ordinary path editing works and conversion undo restores sketch semantics. No unsupported new UI behavior inferred. |
| D17 Stable reads/repeatability | AC10/12 | Repeated resolves, reopen and peer-order convergence agree within 1e-6 mm for identical seeds/input. Count operations/history before and after. Clone chain consumers receive consistent one-original solution. Native/WASM agreement requires executed runtime comparison; build success alone is not parity evidence. |

## Execution and evidence rules

Use public commands and snapshots where available; corruption and peer-race tests
may construct serialized/Loro fixtures through documented format helpers. Do not
reuse solver residual helpers as an independent geometric oracle. Use explicit
length/angle tolerances and known-answer geometry. Preserve exact old fixture
assertions, adding only justified format/record fields. An authentic historical
checkpoint may describe prior intent; it must not be labelled current valid merely
because its outline remains available.

Record exact command counts, each failure and its correction, plus coverage gaps.
These document tests do not replace the 100-entity thread-CPU workload, UI behavior,
full relation matrix or final full-story gate. Production defects go to the
implementer; the tester does not patch production or relax accepted criteria.

## Production solver verification follow-up (planned, no execution)

- **AC12 actual runtime parity:** execute identical serialized inputs against the
  current production `curvyo-sketch-core` on native and actual Node/WASM, using
  existing target tooling or a minimal test-only harness. Reuse analytically known
  spike inputs, not spike outcomes as proof. Compare stable IDs, diagnosis and each
  candidate coordinate within 1e-6 mm; dimensions and hard residuals must satisfy
  independent oracles on both sides. Record artifact revision and runtime command.
- **AC6 production CPU:** a deterministic 100-entity constrained sketch with free
  motion; repeated feasible and beyond-limit drag requests, fixed seed and warmup.
  Measure per-frame thread CPU with existing test timing support, reporting sample
  count, maximum and distribution against <16 ms. Check hard constraints after
  every candidate; do not replace CPU timing with wall time or compile duration.
- **Determinism:** repeat a small bounded corpus from identical starts, reverse
  insertion order and include disconnected constrained components. Compare by
  stable ID, not vector order; no artificial branch equivalence is assumed for
  different starting configurations. Test native repeats as well as cross-runtime.
- **Degenerate/contact boundaries:** near-zero arc sweep, coincident or nearly
  circular ellipse foci, collapsed radii/axes, tangent carrier versus explicit
  finite contact, and angles straddling −π/+π. Valid cases obey independent
  residual/normal checks; invalid cases return explicit error/failure without
  publishing invalid FullyConstrained geometry or mutating input.
- **Invalid drag boundedness:** non-finite target, unknown/fixed target and
  infeasible multi-constraint request terminate with bounded measured work or
  explicit refusal, preserve last-valid state, and distinguish numerical failure
  from proven conflict. No arbitrary new product timeout is introduced; report
  observed timing and any nontermination as a defect.

This is a bounded production-core verification matrix, not a request to duplicate
all solver examples. Implementation and execution wait for the explicit exclusive
slot handoff; absent runtime or CPU evidence remains an open acceptance item.

## Revision-cache guard supplement (planned before optimization verification)

Prime the public snapshot/outline read twice before each mutation below; record
version/history before and after repeated reads. Reuse the existing known line
formula60→80 and independent transformed/conic oracles rather than solver internals.

1. Local parameter edit, undo and redo after priming: graph, solution, status,
   diagnosis and current IDs must all describe the selected revision. No old entity
   survives delete/undo/redo merely because an earlier cached snapshot contained it.
2. Peer import after priming: independent parameter edits combine to the known value;
   an imported contradiction retains current intent and authentic last-valid geometry,
   then removal repairs. Checkout is checked only through an existing public seam;
   never add a production cache/testing API solely for acceptance.
3. Pending uncommitted mutation: if an existing public Loro/document mutation seam
   exposes this state, prime, mutate without commit, read, commit, read again. Neither
   a cache hit nor publication under the old frontier may hide pending intent. If
   inaccessible publicly, explicitly cite producer/internal guard coverage and mark
   independent black-box coverage unavailable; do not simulate it with an unrelated
   committed edit or claim operation-count equivalence proves pending correctness.
4. Prime before style/rigid placement and amplified clone edits: local analytic
   geometry remains correct, world output/style reflects current records, and maximum
   composed clone stretch still controls final-coordinate approximation≤0.01mm.
   Reuse D12/D15 sampling rather than invent source scaling policy.
5. Prime valid instance then open/import malformed or forged checkpoint variants:
   no cache entry from another document/revision grants trust to mismatched IDs,
   false status/rank, fixed-point displacement or invalid arc/construction data.
   Existing explicit damaged fixtures remain strict; repairable intent stays inspectable.

Bounded execution: extend two or three existing tests with prime-before-mutation and
one sequence that combines undo/import/current-ID changes; rerun existing document
acceptance. Cache reads must not write history, versions or persisted derived fields.
