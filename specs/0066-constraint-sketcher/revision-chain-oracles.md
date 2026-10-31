# Independent distance-chain optimization witnesses

Prepared from the approved mathematical proposal before reading its implementation.
Documentation only: no new test execution, no production inspection. The prior failing
host trace remains evidence; these cases do not change the <16 ms host requirement.

All coordinates are millimetres. Compare point positions and hard distance residuals
at the existing 1e-6 tolerance. For a reachable raw goal the endpoint must equal it;
a zero-DOF/status label never substitutes for residual validation. Preserve IDs and
constraint intent. Compare final native and actual Node/WASM unrounded geometry.

## Two-link exact witnesses

Fixed A=(0,0), two driving lengths AB=BC=5. Initial B=(4,3), C=(8,0).
Drag C to (6,0). The two circle intersections for B are exactly (3,4) and
(3,-4). Endpoint displacement is equal for both; B squared displacement is 2
versus 50. The minimum-total-point-displacement candidate is therefore (3,4).
Check both distances=5 and endpoint=(6,0), not merely solver success.

Reflect the initial seed across x: B=(4,-3), C=(8,0). The preferred intersection
must reflect too: B=(3,-4), C=(6,0). This guards a hard-coded upper branch.

Continuation: from each accepted pose move C back to (8,0), giving B=(4,+3)
or (4,-3) respectively. Repeat goals 6→8→6, maintaining the nearer branch and
shared IDs with no sign flip. Reversing the stored Line start/end IDs must not
change the physical answer; permuting map insertion order must not change it.

Deterministic tie: valid folded seed A=C=(0,0), B=(5,0), lengths 5+5; goal
C=(6,0). Both B=(3,±4) have equal squared total displacement 20+36=56.
Either branch is acceptable for this unspecified tie policy, but repeated runs,
reordered input and native/WASM must choose the same physical candidate.
Do not invent a required upper/lower sign for an exact tie.

## Four-link proper-rotation witness

Fixed P0=(0,0); initial P1=(3,4), P2=(6,0), P3=(9,4), P4=(12,0).
Every adjacent length is 5. Each two-link subchain has chord length 6.
Goal P4=(8,0); let h=sqrt(20). Midpoint candidates are P2=(4,±h).
For sign s=±1, the proper-rotation candidates are:

- P1=(2 - 2*s*h/3, 8/3 + s*h/2).
- P2=(4, s*h).
- P3=(6 + 2*s*h/3, 8/3 + s*h/2).
- P4=(8,0), P0 unchanged.

Independently compute sum of all five squared point displacements from the seed
and choose its smaller candidate. Check all four lengths, both chord lengths,
and signed cross products within each two-link subchain: proper rotations must
preserve their original signed area, not reflect individual halves. Compare the
explicit formula, not values produced by the optimization under test.

## Guards and degenerate cases

- Fixed interior: additionally fix B=(4,3) in the two-link seed. A goal C=(6,0)
  cannot be satisfied exactly because its distance from B is sqrt(13), not 5.
  B must remain fixed; a claimed raw-goal solution is invalid. If a closest-feasible
  candidate is returned, its independent oracle is
  C=(4+10/sqrt(13), 3-15/sqrt(13)); otherwise retain the existing generic drag
  contract rather than assuming the fast path applies.
- Extra hard equation: add Horizontal(A,B) with a valid straight initial seed
  B=(5,0), C=(10,0). Goal C=(6,0) is incompatible with that equation and both
  length constraints. A fast candidate with B=(3,±4) must never be accepted.
  Validate all original equations and the established feasible-drag behaviour.
- Multiple goals: submit two point goals through the existing public core drag API;
  retain all hard equations and fixed anchors. Do not accidentally honor one raw
  goal while dropping the other. This is a fallback/guard case, not a new required
  rigid-two-subchain output policy.
- Tangent circle intersection: goal C=(10,0) gives unique B=(5,0); no NaN, sign
  ambiguity or duplicate-candidate instability. Return to (6,0) deterministically.
- Coincident chord endpoints: folded seed and goal at A exercise coincident circles;
  zero-chord rigid rotation is undefined. Preserve valid finite geometry and stable
  IDs, use the unchanged fallback where the exact certificate cannot be formed.
- Unreachable goal C=(20,20): global reach is 10; the closest endpoint is
  (10/sqrt(2),10/sqrt(2)), with B=(5/sqrt(2),5/sqrt(2)). This existing closest-feasible
  contract must remain meaningful movement, not freezing at the previous seed.
- Append an unrelated fixed component and verify its positions/constraints remain
  identical for all successful and refused drags.

## Existing corpus remains mandatory

Keep the connected 100-entity chain's 49 lengths sqrt(34), fixed anchor, feasible
raw goals, far-goal triangle-bound oracle and all 30 full-precision parity frames.
Keep the disconnected 100-entity workload and its untouched 32 other components.
Run actual native release pointer+view+draw CPU and fresh actual Node/WASM parity.
The production host trace still has exactly 20 inputs; cold row 0 is reported
separately, every row 1..19 must be <16 ms through updated canvas and overlay Paint.
Saved-artifact residual/movement verification remains mandatory. A new core change
requires fresh runtime and host evidence; old passing artifacts do not prove it.

Remaining independent host journeys, revised legacy-test compilation/lint and the
once-final gate remain outstanding as listed in acceptance-revision-report.md.

## Contiguous translated suffix certificate (independent extension)

Prepared before reading the suffix implementation. The actual host body drag supplies
multiple point goals; do not replace it with a single endpoint handle or require the
generic solver for a now-certified coherent suffix.

Use four links of length 5 and fixed P0=(0,0): P1=(4,3), P2=(8,0),
P3=(11,4), P4=(14,0). Give **all three** suffix goals P2=(6,0), P3=(9,4),
P4=(12,0), the common translation (-2,0). The prefix P0→P1→P2 is the earlier
analytic two-circle witness. Its candidates are P1=(3,4) and P1=(3,-4).
The other points are fixed by the raw goals. Total squared displacement over the
whole chain is respectively 2+4+4+4=14 and 50+4+4+4=62, so the upper candidate
must win. Verify every raw goal individually at 1e-6, all four hard lengths=5,
P0 unchanged, and the tail's vectors P3-P2=(3,4), P4-P3=(3,-4) unchanged.
A correct P4 alone is insufficient: P2 and P3 are independent obligations.

Reflect the entire seed and goals in x: the lower prefix intersection must win,
with the same costs. Reverse the coherent translation back to the original three
suffix goals; P1 must return to (4,±3) on its corresponding branch. Reorder goal
records and entity insertion order without changing physical output. Preserve
all point/line/constraint IDs. Native and actual Node/WASM use identical goal arrays.

Guard witnesses (fallback eligibility is distinct from a required fallback pose):

- Gap: supply only goals P2=(6,0), P4=(12,0), omitting P3. This is not an exact
  contiguous suffix goal set. A feasible generic result exists (for example P3=(9,4)
  with the upper prefix), but no test should require that particular free-joint pose.
  Validate both supplied goals where accepted and every original hard equation.
- Incoherent: supply P2=(6,0), P3=(10,4), P4=(12,0). Adjacent raw-goal distances
  are sqrt(32) and sqrt(20), not 5, so all three cannot be reached simultaneously.
  Never report an exact raw-goal certificate; any fallback candidate must preserve
  hard lengths/fixed points. Do not accept a single raw-goal check as proof.
- Fixed suffix member: additionally fix P3=(11,4) in the valid initial graph, then
  request the coherent translated suffix above. P3 must remain fixed; the raw-goal
  set is infeasible. A claimed exact suffix result is invalid.
- Cross-link: add the valid initial driving distance P0→P3=sqrt(137). The proposed
  translated P3=(9,4) has distance sqrt(97), so the otherwise-valid chain shortcut
  must not bypass the additional hard equation.
- Foreign component: add a disconnected fixed point and include it in the goal set.
  Never silently drop that goal or move the fixed point; retain the established
  generic failure/feasible-result contract and unchanged component geometry.
- Non-suffix/multiple handle goals retain their existing semantics. The new accepted
  case is exactly a contiguous suffix with common translation and validated raw
  goals, not an authorization to reinterpret arbitrary multi-selection as a body drag.

The genuine 100-entity host workload remains segment-body picking with its actual
multiple goals. Keep saved geometry and all 49 hard-length assertions; do not change
picking, the 20 input positions, or rows[1:] timing eligibility to exercise the shortcut.
