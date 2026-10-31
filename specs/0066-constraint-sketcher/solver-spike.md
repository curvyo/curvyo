# Solver feasibility experiment

Status: feasibility checkpoint complete; ACS0.1.19 selected provisionally for the
validated adapter. Failed-solve latency and final acceptance remain open.
Date: 2026-10-30.

The isolated manifest, lockfile, corpus and raw output live in
`target/0066-solver-spike/`. Reviewable copies will accompany the final report.
No production crate, document API or format changes are part of this experiment.
ACS `acs-solver = 0.1.19` is the evaluated candidate (research initially named
0.1.4). ARAEL is the fallback if ACS fails the required contract. Licences and exact resolved dependencies are checked
from downloaded manifests, not inferred from a repository name.

## Required evidence

- Ten geometric constraints and five numeric dimension meanings, with known
  coordinates and independent rank/DOF expectations.
- Circular and elliptical geometry, tangency, degeneracy and branch continuity.
- Original IDs for redundant/conflicting constraints; hard constraints during drag.
- Native and WASM builds and coordinate agreement within 1e-6 mm.
- Thread CPU measurements for 100 entities, separating solves and diagnosis.
- Explicit failures and untested combinations; no inferred performance result.

## Results

Current results and limitations are in the feasibility checkpoint below.

### Initial native result (superseded by final expanded corpus below)

ACS 0.1.19 (MIT) replaces research-pinned 0.1.4: Cargo reported the newer patch.
The initial release test passed 28 case records in one aggregate test. Cases cover
free point/circle/arc/ellipse DOF (2/3/5/5), fixed and fully constrained geometry,
all ten geometric relation meanings, all five dimension meanings, original
conflict/duplicate IDs, soft drag versus hard constraints and line tangency to a
circle, arc and ellipse. The zero-length seed is observational only, not a success
claim. Formula parsing and product state preservation are not solver features tested here.

Initial 100-entity rigid chain, 10 samples, Linux thread CPU time:
solve without analysis median 1.225304 ms / max 1.344782 ms; with analysis median
3.020932 ms / max 3.255550 ms. This starts already solved and uses an impossible
soft goal: **it does not prove moving geometry or the complete 16 ms frame budget**.
Moving and failed-solve workloads, branch continuity and WASM execution remain pending.

## Feasibility checkpoint: provisional ACS integration

**Recommendation:** use pinned ACS 0.1.19 behind the validated pure-core adapter
of ADR 0025. This is evidence to begin integration, not final story acceptance.
The reviewable experiment and raw results are in [`spike/`](spike/README.md).
No production changes were needed for this experiment. Native release tests,
WASM release compilation and actual Node WASM execution passed. The aggregate
corpus has **50 records**, including four deliberate input/capability rejections;
46 records exercise solving, diagnostics or repeated motion. Native versus WASM
compares 1,596 numeric values, maximum difference **1.4210854715202004e-14**.
This establishes the tested Linux/native and Node/WASM pair, not every platform.

### Contract established and adapter obligations

- Known DOF: free point 2, circle 3, circular arc 5, ellipse 5; fixed point 0;
  full line 0. Tester's twenty rectangles plus twenty circles have DOF20 with
  independent widths, DOF1 with equal widths, DOF0 after a changed width dimension.
  Their 100 backing points are additional geometry, explicitly counted.
- Ten relation meanings and five dimension meanings have analytic cases.
  Fixed geometry with a nonzero horizontal residual is `failed`, even with DOF0:
  the adapter must never treat zero DOF alone as a successful solution.
- Duplicate and conflicting dimensions return original IDs. Missing references
  and duplicate IDs are rejected. Validate before using the lower-level API,
  which is not the same guarded boundary as the JSON entry point tested here.
- Soft drag keeps hard constraints. A movable 100-entity articulated chain was
  dragged for twenty successive frames, retaining each previous solution as seed.
  Each of its 49 lengths and the reachable cursor endpoint is independently checked.
- Zero-length initial geometry with a positive length dimension converges to
  the actual required distance; angle wrap, tiny radii and near-circular ellipse
  cases return finite numeric geometry. This is not universal singularity proof.
- Line/circle, line/arc, circle/circle and line/ellipse tangency pass. Native
  ellipse/circle and ellipse/ellipse requests are explicitly unsupported.
  Exact adapter construction **passes**: a shared contact point lies on both
  curves and is the endpoint of a line tangent to both; a unit helper-line length
  removes its arbitrary scale. Off-axis and rotated analytic contacts pass,
  with DOF0 and expected coordinates. A common line alone would be insufficient.
  Production must hide helper entities, map their diagnostics to original IDs,
  preserve the chosen branch and validate contact and normals independently.
- Fillet tangent geometry at three radii passes; topology remapping and slot
  editing/history are product obligations, not proven by these numerical cases.
- Never publish a failed candidate as geometry: last-valid retention, repair,
  formula typing, undo and collaboration remain Curvyo responsibilities.

### Thread CPU results

Release build; Linux x86_64; exact compiler and CPU in `spike/evidence/platform.txt`.
JSON request/response work is included. Ten samples per static workload; twenty
successive moving frames. No wall-clock performance assertions are used.

| Workload | Analysis | Median ms | Max ms |
|---|---:|---:|---:|
| Rigid 100-entity chain, impossible soft cursor | off | 1.212 | 1.240 |
| Same | on | 2.648 | 2.714 |
| Moving articulated 100-entity chain, 20 frames | off | 4.602 | 10.416 |
| P100 equal rectangles, actual width change | off | 8.505 | 8.865 |
| Same | on | 10.344 | 10.474 |
| Contradictory 100-entity chain | off | 40.439 | 42.226 |
| Same | on | 42.759 | 44.207 |

The successful tested motion fits 16 ms; **failed solves do not**. Do not claim
AC6 complete or a universal frame budget. Production must measure bounded drag
iteration, retain the last-valid preview on failure, and verify recovery without
loosening hard residuals. Full diagnostics need not run on every pointer frame;
never display stale diagnosis as current certainty.

### Dependencies and fallback

ACS 0.1.19 declares MIT in its downloaded manifest and licence. The exact 46-package
experiment inventory (including its own unlicensed private harness) is retained in
`spike/evidence/dependencies.json`; production dependency-policy checks remain due.
No C ABI feature was enabled. The optional upstream C ABI contains unsafe code;
this experiment used the Rust API, with nalgebra and wasm-bindgen dependencies.
WASM compilation and execution succeeded with wasm-bindgen 0.2.129.

ARAEL 0.9.3 was downloaded for bounded fallback inspection (MIT manifest), not
built or benchmarked. Its generic arc-pair tangent residual uses centre distance
and effective radii along the centre line; that is not independent proof of exact
general ellipse contact. Since the ACS common-contact construction passed, no
fallback selection or comparative performance claim is made.

Open before final acceptance: larger/dense adversarial graphs, repeatability under
ordering and disconnected components, branch-preserving ellipse drags, exact
normal checks in the production adapter, bounded failure latency, full slot/fillet
reference preservation, per-entity freedom classification and native platforms
beyond Linux. No candidate is claimed universally robust or globally minimal in
its conflict reports.
