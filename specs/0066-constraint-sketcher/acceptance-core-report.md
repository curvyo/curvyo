# Core milestone acceptance report

Status: **latest independent core acceptance19/19 PASS; focused test Clippy PASS**.
The final focused batch is in `target/0066-runtime/final-focused-batch.log`; older
counts below are historical milestones. Final workspace nextest passed9702/9702
(including this current core corpus); all core/editor WASM build and Clippy passed.
The complete final gate is recorded in acceptance-session-report.md.
This is not acceptance of the full 18-criterion feature. No document, editor or UI integration is claimed.

The tester wrote the acceptance plan before production code existed and derived the independent
public-API tests from that plan and `core-api.md`. The test implementation does not use internal
solver helpers or ACS result values as its oracle. Production files were not edited by the tester.

## Executed checks

| Command | Actual result |
| --- | --- |
| `cargo test --offline -p curvyo-sketch-core --test acceptance_0066_core -- --nocapture` | Initially 8/8; expanded matrix 10/11, then 11/11 after tolerance correction described below |
| `cargo test --offline -p curvyo-sketch-core` | 32 passed: 3 unit, 11 independent acceptance, 18 existing integration; 0 failed, 0 ignored; 0 doctests |
| `cargo clippy --offline -p curvyo-sketch-core --all-targets --message-format=short -- -D warnings` | PASS after correcting test-only naming and borrowed-argument lint findings |
| `cargo test --offline -p curvyo-sketch-core --test acceptance_0066_core` | Final formatted/linted acceptance file: 11 passed, 0 failed, 0 ignored |
| `cargo fmt -p curvyo-sketch-core --check` | Final PASS |
| `cargo deny check licenses` | Initial FAIL for missing first-party `curvyo-sketch-core` exception; final PASS (`licenses ok`) after lead added the existing ADR0006 first-party policy entry; third-party allowlist unchanged |

No full workspace gate, browser, CPU benchmark or WASM runtime test was run in this bounded handoff.
The previously reported solver spike is not used as proof of production runtime parity or performance.

## Independent evidence

All tests are in `curvyo-sketch-core/tests/acceptance_0066_core.rs`.

| Criteria / test suffix | Independent oracle and scope |
| --- | --- |
| AC4/5 `analytical_point_and_line_rank_and_residuals` | Free point 2 DOF; two points 4; fixed anchor 2; horizontal 1; prescribed length 0. Checks resulting 10 mm length, horizontal residual, unchanged anchor and immutable input. |
| AC5 `circle_and_non_circular_ellipse_have_known_freedom` | Circle 3→1→0 DOF and noncircular ellipse 5→3→1→0 under successive independent fixes. |
| AC5/7 `duplicate_and_contradiction_ids_never_publish_invalid_zero_dof` | Duplicate horizontal constraint reports original redundant IDs; invalid fully fixed geometry cannot publish a candidate or FullyConstrained status. Caller retains last valid snapshot. |
| AC6 `drag_fixed_and_circle_locus_keeps_hard_constraints` | Incompatible drag remains on exact 10 mm locus; fixed origin unchanged; Preview has no rank classification; input immutable. |
| AC7 `contradictory_dimensions_name_original_constraints_and_repair` | Contradictory 10/20 mm lengths report Conflicting with original constraint IDs and no candidate; removing contradiction restores known coordinates. |
| AC3 `parameter_formula_units_rename_cycles_and_failure_are_atomic` | Width minus twice margin, rename by stable reference, inch/mm conversion, unknown/malformed formulas, dimension mismatch, zero division and dependency cycle; previous state retained. |
| AC3/10 `deep_parameter_graph_roundtrip_and_invalid_data` | 3,000-node dependency chain survives JSON roundtrip and yields 3,000; malformed quantity rejected; decoded negative radius rejected by validation and solve. |
| AC14 `three_point_curves_pass_actual_non_axis_third_pick` | Circle center (1,1), radius √2; ellipse semiaxes 5/3 and independent implicit residual at third pick (3,2.4); collinear circle rejected. |
| AC8/15 `slot_outline_has_exact_topology_dimensions_and_area` | 20×6 centreline slot: bounds −3..23/−3..3, one closed contour, area 120+9π within 0.03 mm². |
| AC16 `fillet_preserves_original_relationships_and_oversize_refuses_without_mutation` | Right-angle fillet radius 2: center (2,2), endpoints (2,0)/(0,2); old constraints and virtual construction lines preserved; radius 20 refused without mutation. |
| AC8 `outline_explicit_coincidence_closes_cycle_and_preserves_attached_spur` | Equal coordinates alone do not join distinct endpoints; explicit coincidence closes square, retains 100 mm² area and attached spur. |

## Corrections and outstanding coverage

The initial fillet test compared floating-point endpoint coordinates exactly. The independently
specified geometry tolerance is now explicit (1e-6 mm); no expected endpoint or product behavior
changed. Two test-only clippy findings were corrected without suppressing the affected lints.
No production defect was found by these eleven cases. The license policy rejection was sent to the lead, corrected in `deny.toml`, and independently rechecked successfully.

Remaining feature coverage includes all UI/view/tool behavior (AC1/2/11/12), document/history/clones
and output integration (AC9/10/13/17/18), broader relation combinations including independent ellipse
normal/tangency witnesses, fillet edit-after-construction, malformed deep expression trees, and the
planned 100-entity thread-CPU measurement. The long dependency chain tests graph depth, not arbitrary
recursive expression AST deserialization. Native/WASM parity remains unverified here. These gaps
remain in the acceptance plan and must not be represented as full-story PASS.

## Independent final-acceptance expansion

`cargo test --offline -p curvyo-sketch-core --test acceptance_0066_core -- --nocapture`
now passes **17/17** (log `target/0066-runtime/core-final-acceptance.log`). Six added
cases cover the remaining supported relation tuples with independent residuals:
parallel cross product, perpendicular dot product, vertical x alignment, coincidence,
midpoint, symmetry reflection, equal circle radii and line/ellipse tangency with
fixed centre/focus and known minor radius3. Together with earlier H/Fixed/Distance
cases this covers all ten named relation families, without claiming every tuple.

Diameter8 yields radius4; Angleπ/3 with length5 yields endpoint(2.5,5√3/2).
A reference length99 changes neither rank nor solved length5. A diameter-only circle
with free centre retains2DOF, can translate and is not fully-constrained per entity;
fixing centre makes it fully constrained. A line with free endpoint is not labelled
fully constrained merely because the line record itself owns no numerical variables.

Further cases edit an existing fillet radius2→1 while retaining original constraints;
repeat and reorder disconnected input with near-circular ellipse focus distance1e-8;
and run a2048-node Negate expression through evaluation, serialization/deserialization
and drop in an isolated test subprocess. Rejection of deep decode is permitted;
process failure is not. This last case proves safe handling of that input, not an
unlimited recursive-AST guarantee. Dedicated angle-wrap coverage remains pending.
These are actual native test results, not upstream or producer assertions.

Final small follow-up: split the angle/reference and entity-freedom test into two
independent tests to satisfy Clippy's function-size limit, preserving all assertions.
Added `ac3_angle_crosses_signed_pi_boundary_without_false_conflict`: radius10 line,
fixed x-axis, initial angles±(π−.02), target∓(π−.01), independent sine/cosine endpoint
oracles. Filtered native execution PASS1/1 (`target/0066-runtime/angle-wrap.log`).
This brings the source corpus to19 tests; the latest whole-corpus run remains17/17
before this split/addition, so19/19 is not yet claimed. Actual production WASM conic
geometry parity is independently executed in the Session runtime corpus/report.

Subsequent focused final batch actually ran the complete current core corpus:19/19 PASS;
Clippy for this target PASS. Evidence `target/0066-runtime/final-focused-batch.log` and
`final-batch-clippy.log`. The earlier17/17 count remains historical evidence only.
