# Document milestone acceptance report

Status: **latest independent document acceptance16/16 PASS after revision-cache changes, plus explicit saved-browser
circle and clone geometry verifiers1/1 each PASS; focused test Clippy PASS**, 2026-10-30. Evidence:
`target/0066-runtime/final-focused-batch.log` and `target/0066-browser/saved-geometry.log`.
Prime-before-edit/import/history/style/clone guards are executed; pending Loro
transactions and detached checkout remain internal-only coverage (see latest section). Older counts below are historical milestones. This
does not claim full-story acceptance or CPU compliance.
Final workspace nextest passed9702/9702, including the current document corpus.
The complete final gate is recorded in acceptance-session-report.md.
The independent document plan preceded production document implementation review.

## Actual checks

| Command | Result |
| --- | --- |
| `cargo test --offline -p curvyo-document-core --test acceptance_0066_document --test acceptance_0017_block1_tester` | 10 independent document tests + 32 existing fixture/style tests passed; zero failed/ignored |
| `cargo clippy --offline -p curvyo-document-core --all-targets --message-format=short -- -D warnings` | Final PASS, all document test targets compile |
| `cargo fmt --all --check` | PASS after formatting document test changes |
| `cargo test --offline -p curvyo-document-core --test sketch_document --test clone_history --test clone_commands --test transform_objects` | 56 passed (6 + 15 + 23 + 12), zero failed/ignored |

Logs: `target/0066-document-acceptance-tests.log`,
`target/0066-document-acceptance-clippy.log`,
`target/0066-document-regression-tests.log`. No full workspace gate was run.

## Independent coverage

Tests: `curvyo-document-core/tests/acceptance_0066_document.rs`.

| Plan case | Executed evidence |
| --- | --- |
| D01 / AC3,10 | Formula graph survives save/open; width80/margin10 yields actual endpoint60, width100 after reopen yields endpoint80 and survives another reopen. Repeated reads leave version unchanged. |
| D02 / AC10 | Four valid v28 goldens open without read writes; two explicitly named malformed record/checkpoint goldens return Damaged. Existing fixture/style sweep retains all prior assertions and explicitly classifies those two additions. |
| D03/05 / AC11 | Concurrent different parameter edits combine to endpoint70; local undo preserves peer margin edit (endpoint50); redo restores70. |
| D04 / AC7,11 | Well-formed contradictory length remains stored, inspectable, conflicting and one history step; original resolved geometry remains. Explicit removal repairs it. |
| D06 / AC10 | Concurrent entity definition+construction changes converge as an intact winning record on both peers, never a mixed record. |
| D07/09 / AC7,10 | Handcrafted stale parameter checkpoint resolves actual updated geometry; dangling entity reference is retained with diagnosis and historical output, then explicitly repaired. Reads do not write. |
| D10 / AC5,10 | Matching-hash checkpoint falsely claiming full rank is recomputed as 2 DOF; displaced fixed-origin checkpoint is Damaged. |
| D12/13 / AC9,11 | Two-level clone chain follows formula edits; source removal preserves drawable outputs; undo restores source and subsequent edits propagate through both links. |
| D14/15 / AC8,11 | Known 90°+translation placement gives endpoint(30,40), local intent unchanged; conversion preserves exact visible vertices/object identity, takes one step, undo restores graph and placement. |
| D12/15 / AC8,9 | Existing clone magnification1000 for analytic circle/ellipse: exactly one closed contour; sampled final-coordinate deviation ≤0.01 mm; source graph unchanged. 41 samples per cubic, compared to independently calculated analytic curve points. Sampling is not a certified Hausdorff bound. |

## Defects and corrections

The new magnified ellipse case initially failed: the resolved ellipse had one
open 13-anchor subpath, unlike the circle. Inspection confirmed the underlying
full-turn ellipse iterator did not emit ClosePath. The implementer added explicit
closure for ellipses; the unchanged independent closed-contour and deviation
assertions now pass. This affected analytic ellipses at all scales, not solely
magnified clones. Initial test refinement inspected all compound subpaths before
confirming there was genuinely no closed contour; the requirement was preserved.

All-target compilation exposed previously unhandled Sketch variants in legacy
acceptance/test helpers. Added explicit unexpected-Sketch failure arms where the
scenario cannot contain a sketch; the fixture style reader returns sketch style
so valid new fixtures receive the existing style assertions. No prior assertion
was removed or relaxed. Production-only lint issues (test module order, constant
assertion) were corrected by the implementer; tester changes stayed in tests/docs.
Minor new-test API/iterator/naming issues were corrected before final checks.

## Explicit remaining gaps

D08 structural malformed coverage is representative, not exhaustive. D10 does not
yet forge every extra/missing ID, construction membership or arc invariant. D11
fingerprint ordering/boundary is not independently exhaustive. D13 own-style,
hidden-source and explicit-unlink cases rely on existing affected regressions and
producer evidence, not new independent sketch-specific assertions. D15 independent
conversion uses a line; the curve deviation case checks resolved clones, not every
converted curve kind. D16 writer-boundary acceptance remains pending. D17 actual
native/Node-WASM runtime parity and 100-entity thread-CPU drag remain planned for
later. No source-scaling policy was assumed; clone magnification exercises the
existing clone placement contract only. UI/history navigation and complete AC1–18
acceptance remain for the end-to-end milestone.

## Session-stage independent gap closure

The later tester-owned command
`cargo test --offline -p curvyo-document-core --test acceptance_0066_document -- --nocapture`
passed **14/14**, including all original ten tests and four additional cases.
Evidence: `target/0066-runtime/document-gaps.log`.

- D10 now forges missing/extra candidate entity IDs, changed construction membership
  and inconsistent arc radius: all four checkpoints are refused as Damaged.
- D11/D16 now verify style and rigid translation preserve the graph fingerprint;
  direct anchor writing to Sketch refuses without changing version, then works
  after explicit conversion.
- D13 now checks hidden source updates, clone own-style retention, explicit unlink
  preserving exact resolved coordinates/style, and independence after later source edit.
- D15 now explicitly converts arc, ellipse and slot under known rigid placement,
  checks expected open/closed topology and sampled analytic deviation≤0.01 mm in
  final coordinates (41 samples per segment; still not a certified Hausdorff bound).

These close the corresponding representative gaps above; fingerprint insertion
order and exhaustive malformed-record permutations remain untested independently.
Actual production native/Node-WASM geometry parity and CPU scopes are recorded in
`acceptance-session-report.md`; they do not establish frontend behavior or full gate.

Final bounded addition: `d01_deep_accepted_formula_must_reopen_and_remain_editable`
executed in an isolated test child with256 parser-valid additive terms. The actual
Document accepted it; pack/unpack, dependent width edit80→100 and a second roundtrip
all PASS. This specifically tests the real document codec, not serde recursion rejection.
Log `target/0066-runtime/document-deep.log`; exact filtered command is
`cargo test --offline -p curvyo-document-core --test acceptance_0066_document d01_deep -- --nocapture`.
Actual browser saved-circle verifier also explicitly executed PASS1/1 (see Session report).

## Independent revision-cache verification

Current optimized production: `cargo test --offline -p curvyo-document-core --test acceptance_0066_document -- --nocapture`
PASS16/16 (+1 artifact-dependent ignored at that run), log `target/0066-runtime/cache-independent.log`.
Prime repeated snapshot reads before local edit, peer import, undo/redo, style/placement,
and amplified clone resolution; verify no version/history writes. New current-ID test
checks deleted entity disappears from both authoritative and solved records, reappears
on undo, disappears on redo. Existing forged candidate/status/arc/construction fixtures,
formula persistence, clone tolerance and deep accepted formula roundtrip remain intact.
Pending Loro mutation and detached checkout are crate-private, not exposed by public
Document seams; independent black-box proof is unavailable for those two internal states.
The report does not mislabel ordinary committed edits as pending/checkout coverage.
