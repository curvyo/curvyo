# Sketch document integration plan

Status: bounded document acceptance passed, 2026-10-30; see `acceptance-document-report.md`. The earlier core checkpoint
passed 32 core tests, including 11 independent acceptance cases, before this document phase.
Basis: ADR0025/0026, `integration-notes.md`, current object/clone/codec/history source.
External scaling remains pending the customer answer relayed by the lead; the ADR's
proposed scale meaning is not authorization to implement it in this step.

## 1. Define stored records and read results — AC9–12

- Add `sketch_model.rs`: boxed `SketchSnapshot` with object ID, graph, placement,
  style, resolved `PathSnapshot`, checkpoint provenance and inspectable diagnosis.
  Add `ObjectSnapshot::Sketch` in `primitive_model.rs`, not a disguised Path.
- Add `sketch_codec.rs` plus a bounded checkpoint/fingerprint module; register
  `shape = "sketch"` through `shape_codec.rs::LeafKind` and strict tag dispatch.
- Authoritative graph collections are Loro map containers keyed by stable IDs.
  Each entry is one **plain atomic value**, not a nested editable Loro container.
  An entity value includes its definition/seed **and construction flag together**;
  a constraint or parameter definition is likewise indivisible. Origin, label and
  placement have separately specified registers. No whole-graph LWW register.
- Store derived `contact_seeds` only inside one atomic last-valid checkpoint with
  solved local geometry and its input fingerprint. Do not duplicate it into the
  authoritative entity/constraint maps or make cached paths editable.
- Canonical encoding uses explicitly ordered typed fields and BTreeMap ID order.
  Hash canonical authoritative graph bytes with existing workspace SHA-256 and a
  domain/schema prefix. Exclude style, placement and checkpoint data. Never use
  `DefaultHasher`, map iteration accidents or debug formatting as a fingerprint.
- Add sketch-core dependency to `curvyo-document-core/Cargo.toml`; sha2 is already
  present. Keep unit/path conversion in document-core; no dependency cycle.

Tests first: atomic entity+construction encoding, stable fingerprint across map
insertion orders, intent change invalidation, placement/style exclusion, seed
checkpoint roundtrip and malformed atomic/container record rejection.

## 2. Read, validate and resolve once — AC7–10, AC12

- Extend `path_codec.rs::validate_object_node` with sketch structural validation.
  Present records with malformed type/payload/non-finite numbers fail import.
  Well-formed records with missing references, cycles or contradictory equations
  can result from valid merges: retain them with repair diagnosis, not whole-file
  refusal or silent deletion. Keep structural decode separate from graph solving.
- Extend `object_read.rs::raw_from_meta` and `clone_read.rs::RawLeaf` before path
  fallback. Resolve an original once in the shared object read, then reuse its
  validated outline for every clone and downstream read-only consumer.
- A matching checkpoint requires structural, finite and candidate-invariant
  validation through a public core candidate guard (or solve+validate); matching
  SHA-256 alone is not proof of correct geometry. A mismatch
  triggers a deterministic solve from graph seeds/last-valid branch information.
  Success is an ephemeral resolved view; failure retains last-valid outline and
  marks diagnosis stale/conflicting/failed accurately. Reads never write Loro ops.
- Cache by graph fingerprint plus relevant checkpoint seed/provenance. Transform
  local outline using placement separately; a translation must not rerun solving.

Tests: no-write repeated reads/open, mismatch recovery, failed mismatch fallback,
missing-reference merge, parameter cycle merge, fixed-geometry contradiction,
malformed checkpoint, stale checkpoint winner and one solve shared by clone chains.

## 3. Commands, history and peer merge — AC3–7, AC11, AC13–17

- Add `sketch_commands.rs` for creation and explicit graph edits. Validate/solve
  candidates before changing live state; write affected authoritative records and
  a successful checkpoint in one named transaction. Previews remain ephemeral.
- Dimension/parameter edits write only their changed intent records; dependent
  solved coordinates belong in the checkpoint. Direct geometry edits update the
  explicitly affected entity seeds. Do not rewrite every graph record per solve.
- Use existing `history/node_value.rs::Node::Value` and `history/merge.rs` atomic
  restore behavior; nested collection containers recurse by ID, individual records
  do not. Add Sketch kind reporting in `history/restore.rs::kind_of` and its public
  kind enum. Review object-specific restore/revival, not only ordinary undo.
- Ensure constraint removal clears its derived contact seed in the new checkpoint.
  Topology operations preserve unrelated IDs; virtual corners stay construction
  records. Malformed edits are refused before mutation. Well-formed contradictory constraint
  edits remain inspectable records with the prior validated checkpoint visible.

Tests: each edit one step, no-op/preview zero steps, undo/redo and object undo;
peer changes to a different parameter survive local undo, same-record conflict
stays atomic, delete versus concurrent reference remains diagnosable, revival
restores graph IDs, checkpoint mismatch after conditional restore remains safe.

## 4. Placement, clones and explicit conversion — AC8–11

- Wire `primitive_model.rs` style/ID/rotation/translated/rotated arms and document
  `objects.rs::translate_meta`/rotation dispatch. Reuse existing affine/path maths
  to move the resolved outline; graph dimensions remain local and unchanged.
  Prepare shared affine cache mechanics, but defer new external scale behavior.
- Extend `clone_read.rs::walk/assemble` root handling to a sketch's resolved path
  while retaining its original Sketch snapshot. Preserve own-placement/style,
  hidden-source behavior, stacking order, orphan/cycle and finite-outline checks.
- Audit `clones.rs`, `clone_write.rs::origin_is_live`, limited source reads and
  `clone_limits_hold`; creating a clone must not convert its parametric source.
- Extend `clone_repair.rs`, deletion/history removal and unlink to materialize only
  resolved paths where existing rules require it. Never copy a graph into a clone.
- Extend `shapes.rs::convert_to_paths(_uncommitted)` with coherent sketch cleanup:
  preserve object ID/style/source links, bake placement once, emit closed subpaths,
  remove sketch tag/records/checkpoint together and retain one-step undo. Scale-aware
  local approximation must meet 0.01 mm in final coordinates after placement.
- Audit path-writing callers separately from `drawn_path` readers. Node editing,
  joins and boolean/path replacement cannot write anchors into a Sketch leaf merely
  because it exposes a path. Require existing explicit conversion semantics.

Tests: translation/rotation preserve graph bytes; clone chains and overrides,
source conversion/deletion/undo, unlink, hidden sources, original identity;
writer refusal, coincident closed contours with spurs, conversion geometry/style
and no double transform. Scaling tests wait for the customer's agreed meaning.

## 5. Format, JSON and validation handoff — AC8–12

- Recheck `document.rs::CURRENT_FORMAT_VERSION` (currently 27), allocate next once,
  update readable `ObjectJson`/`export_json`; JSON stays non-authoritative.
- Keep `container.rs::pack/unpack` members derived from the same state; manifest
  checks and newer-format refusal remain. No old-file migration or reader retrofit.
- Add goldens: editable formula-driven plate, slot/fillet construction records,
  sketch+clone chain, conflicting/dangling graph with valid checkpoint, stale
  checkpoint and converted result. Add malformed record/checkpoint fixtures and
  update the explicit fixture inventory without weakening prior assertions.
- Run focused document/core tests plus affected clone/history/conversion regression
  suites, native and WASM Clippy/builds. Hand off codec/model review and independent
  acceptance before editor/WASM/UI wiring; reserve the full gate for story delivery.


## Document milestone evidence (in progress)

- [x] Public core candidate guard and seeded solving: 2 focused tests.
- [x] Atomic records, SHA-256 provenance, explicit Sketch variant and format 28.
- [x] R3 changed intent wins over old checkpoint; R4 displayed diagnosis recomputed.
- [x] Read-only cache avoids repeated solving; raw decoding/residual validation still runs
  before cache lookup. Measure actual session cost in AC6 rather than assume a budget.
- [x] Public creation/delta, rigid placement, clone chains, conversion/deletion undo.
- [x] Structural/merge/diagnostic tests: 6/6; public tests: 3/3 at first milestone.
- [x] Six current-format goldens generated and read back; 2 fixture tests passed.
- [x] Finish conflict/preview tests and affected regression/Clippy/WASM checks.
- [x] Refine one shared clone root for final-coordinate tolerance, including hidden chains.

Final checks and API contract: `document-api.md` (453 document unit tests, six new
integration tests, 103 clone/history/transform regressions; core34, UI871/render179).
- [ ] Independent document acceptance and final source review handoff.

### Independent ellipse regression repair

The tester found that kurbo's full-turn ellipse iterator omits `ClosePath`. The core
ellipse branch now explicitly closes its analytic contour; cubic geometry and open
arc/chain behavior are unchanged. The unchanged independent
`d12_d15_magnified_circle_and_ellipse_clones_meet_final_space_tolerance` passes.
Core tests: 34 passed; document sketch unit tests: 9 passed; public sketch tests: 6
passed. Library Clippy and library test-profile Clippy pass. All-target Clippy still
stops on legacy exhaustive-match helpers owned by the tester (0053, style_format,
0007); full formatting also reports the tester-owned 0003 indentation. Production
unit lint findings were removed by moving `write_sketch` before the test module and
removing a redundant compile-time format assertion (golden checks remain intact).
Evidence: `target/0066-logs/ellipse-*.log`. Independent verification remains pending.
