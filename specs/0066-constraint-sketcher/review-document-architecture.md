# Early document architecture review

2026-10-30. **APPROVE for continued integration**, scoped to the inspected source.
No builds/tests run by architect. Producer is actively integrating; this is not
final feature, format or performance acceptance.

## Closed findings

- **R3: stale seed overrode peer intent.** `sketch_read::seed_for` compares each
  current entity with `checkpoint.intent`. Changed records replace old candidate
  geometry in the seed. Both read and `apply_sketch_delta` use this function;
  the latter applies its explicit delta afterward. The original checkpoint
  provenance remains unchanged. Source fix verified; independent concurrent-edit
  and no-write regression execution remains tester evidence.
- **R4: saved diagnostic classification was trusted.** `sketch_codec::read`
  rejects failure classifications on valid checkpoints and unknown diagnostic
  IDs. `sketch_read::read` recomputes classification using current graph and seed;
  saved numeric DOF and fully-constrained sets do not determine the displayed
  result. Its result is retained in the ephemeral document cache. Source fix
  verified; crafted valid-geometry/false-diagnosis fixture execution remains
  tester evidence.

## Verified source boundaries

- `SketchEntityRecord` couples geometry with construction membership in one
  atomic value. Entity, constraint and parameter collections use separate keys;
  explicit deltas write only listed records, not solver-dependent coordinates.
- Checkpoint intent, candidate and fingerprint form one atomic derived value.
  Codec checks the input fingerprint and independently validates candidate
  intent/residuals. Hash equality alone is not accepted as geometric proof.
- Fingerprinting excludes contact seeds and naturally excludes placement/style.
  `validate_records` separates malformed individual records from repairable
  missing references, duplicate names and dependency cycles.
- Failed solving retains the independently validated saved outline with stale
  state and diagnosis. A missing checkpoint yields empty output, not invented
  geometry. Read paths perform no persistent writes or history commits.
- `Document::sketch` caches graph/checkpoint results and remaps placement/style
  without re-solving unchanged intent. `read_objects` and cached single-clone
  reads use this entry. Cache identity includes checkpoint changes as well as
  graph changes, protecting seed/branch consistency.
- `ObjectSnapshot::Sketch` remains distinct; clone roots accept resolved sketch
  paths. Ordinary primitive conversion rejects the kind. Explicit sketch
  conversion keeps object identity and writes a path with fresh anchors while
  removing sketch registers. Delete/history clone repair uses resolved geometry.
- Format constant is 28. Codec/container validation, golden fixtures, fieldwise
  undo and concurrent repair still require independent acceptance execution.
- External scaling remains a separate customer decision; the inspected command
  path currently admits rigid placements only.

## Integration follow-up, not a new scope requirement

The cache currently calls full codec decoding, fingerprinting and candidate
residual validation before looking up an unchanged graph/checkpoint. It avoids
repeated numerical solving but is not a constant-cost revision fast path.
Measure actual session usage; move validation behind a revision guard if hot
reads would repeat it. Raw tree-only clone reads can bypass the document cache;
keep their use outside per-frame rendering or route them through the cached read.
No performance PASS is inferred from the existence of this cache.

Remaining reviews include the final writer dispatch inventory, clone override
and deletion/undo cases, malformed and merged-format corpus, no-write read
assertions and actual interactive CPU measurements. No new blocker was found
in this bounded follow-up after R3/R4 source corrections.

## Clone magnification follow-up

Source review APPROVE: `SketchSnapshot.solution` retains derived local analytic
geometry (including historical topology when stale), separate from authoritative
`graph`. `refine_sketch_roots` takes the maximum composed-clone Frobenius norm
across resolved chains without visibility filtering. `outline_scaled` combines
this conservative bound with the source placement norm; it generates one shared
root outline before clone transforms, with no additional solves. This preserves
the 0.01 mm final-coordinate approximation budget without approving original
sketch scaling semantics. Producer reports a 100× then 10× clone-chain regression
failed at 0.6004307 mm radial error before the fix and passed afterward; architect
did not rerun it. No new source blocker found in this bounded follow-up.
