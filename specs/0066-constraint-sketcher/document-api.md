# Document integration handoff

Status: Implemented; independent document acceptance 16/16 and full gate passed, 2026-10-30. See acceptance-document-report.md and acceptance-session-report.md for current evidence; milestone checks below retain their original scope.
No workbench/session UI is delivered by this internal milestone. One end-to-end
story PR remains the delivery unit; source scaling still awaits the customer.

## Public contract

- `ObjectSnapshot::Sketch(Box<SketchSnapshot>)` is distinct from an editable path.
  `drawn_path()` is read-only output; Node/path transform, replacement, offset
  mutation and stroke conversion refuse the parametric source until conversion.
- `Document::create_sketch(&Sketch, Affine, &Style)` creates one object/step.
- `apply_sketch_delta(id, &SketchDelta)` writes only the explicit entity,
  constraint and parameter records. Entity + construction is one atomic record.
- `apply_sketch_preview(id, &delta, &validated_pose)` preserves the drag branch
  without writing dependent solved coordinates into unrelated intent records.
- `convert_sketch_to_path(id, &fresh_anchor_ids)` bakes the visible compound path
  under the same object identity; undo restores the parametric records.
- `Document::sketch(id)` exposes current graph, derived analytic `solution`,
  resolved path, checkpoint provenance, DOF/status, conflicting/redundant IDs,
  fully constrained entity IDs, stale flag and diagnosis. A stale solution can
  contain historical topology: it is not current editable intent.
- Well-formed contradictory dimensions/relations remain inspectable/removable;
  the old validated geometry stays visible. Malformed edits and invalid formulas
  are refused before writes. Merge-created missing references remain repairable.
- Core `validate_candidate(intent, candidate)` checks topology/intent plus hard
  residuals. `solve_seeded(intent, seed, mode)` preserves fixed targets. Document
  `seed_for` overlays authoritative entity edits since checkpoint.intent before
  reusing unchanged solved geometry; local explicit delta wins last.

## Stored format and read cost

Format 28 adds `shape="sketch"`, origin, placement, three per-ID Loro map
containers, and one derived checkpoint. Each collection entry is an atomic plain
map value, not a nested editable container. SHA-256 hashes canonical ordered
intent with a schema prefix; placement/style/contact seeds are excluded.
Checkpoint includes its input graph solely as validation provenance. The hash
alone never authorizes geometry or diagnosis. Matching/fallback geometry passes
core residual validation; displayed rank is recomputed, not trusted from storage.

Document-owned cache avoids repeated solver/rank work for object, node-list and
normal clone queries. Rigid placement/style changes reuse the analytic solution.
Decoding, hashing and checkpoint residual validation still precede cache lookup;
AC6 session CPU evidence must include actual revision access cost. Historical and
internal raw reads may resolve without that cache. Reads never write Loro ops.

A clone chain reuses one root outline refined for the largest composed linear
stretch, including hidden sources. Conservative product norms bound final-space
curve error at 0.01 mm without solving clones or choosing source-scale semantics.
The regression was red at 0.6004307104 mm and green after refinement.

## Checks run

- `cargo test -p curvyo-sketch-core --offline`: 34 passed (including 11 independent
  core acceptance tests and two new checkpoint tests).
- Document `--lib`: 453 passed, two existing ignored. New public `sketch_document`:
  six passed, covering roundtrip/undo, clone chains, delete/convert undo,
  inspectable conflicts, preview-only dependent geometry and scaled hidden roots.
- Seven affected document suites (`linked_clone`, `clone_commands`, `clone_history`,
  `clone_overrides`, `clone_held_back`, `transform_objects`, `history_preview`):
  103 passed. Ordinary test helpers gained exhaustive Sketch arms, no assertions
  were removed or weakened. Independent acceptance files were not edited.
- UI/render `--lib`: 871/179 passed. Editor native `cargo check` passed.
- Affected native library Clippy and new integration-test Clippy passed.
- WASM build and Clippy for sketch-core, document-core and editor-wasm passed.
- Workspace formatting check and `git diff --check` passed.

Logs are in the ignored `target/0066-logs/sketch-{core-final,final-regression,ui-regression,document-clippy,
document-wasm,document-wasm-clippy,newtests-clippy}.log`.
Full story gate and independent document acceptance remain tester-owned.

## Fixtures and next phase

Six files in `curvyo-document-core/tests/fixtures/`, generated and reopened by
`sketch_fixtures` unit tests with `CURVYO_WRITE_FIXTURES=1`:
`sketch_plate_v28.curvyo`, `sketch_slot_v28.curvyo`,
`sketch_converted_v28.curvyo`, `sketch_dangling_v28.curvyo` are valid;
`sketch_checkpoint_invalid_v28.curvyo` and `sketch_record_invalid_v28.curvyo`
are deliberately damaged. The tester should extend the explicit fixture inventory
and update version/enum acceptance oracles with their existing assertions intact.

R3 stale seed priority and R4 untrusted diagnostics have direct regressions and
architectural source approval. The clone tolerance amendment also has source
approval. Next: independent document tests, then session/workbench integration
from `session-integration-notes.md` and `ui-interaction-map.md`; do not assume
actual pointer CPU performance or complete AC1–18 acceptance from this milestone.
