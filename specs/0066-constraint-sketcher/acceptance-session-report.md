# Independent Session, runtime and full-gate acceptance

Status: **independent functional/runtime/performance acceptance and full CLAUDE §7 gate PASS**, 2026-10-30.
Final workspace nextest9702/9702 passed (126 skipped); frontend641 passed/74 skipped,
TypeScript and oxlint exit0 (57 warnings, not warning-free). Exact final stages below.
Reference read-only regression is closed by the strict actual-browser numeric oracle.
Focused numeric-field Alt, clone entry, saved geometry, creation variants/Fillet,
cache guards and scheduling regressions passed. Latest native chain max7.139976ms;
actual complete production browser frame warm max15.626ms, cold32.884ms reported
separately. Actual fresh Node/WASM chain and conic parity passed within1e-6mm.
Mounted ruler pan/zoom/resize/DPR checks and60 raster pixel comparisons passed.
Historical failures and earlier partial-frame timing remain below as audit evidence;
they are superseded by the explicitly scoped final batch and final gate below.
Customer story acceptance and the separately pending source-scale decision remain
customer-owned; this technical PASS does not silently decide either.

## Production runtime evidence

Linux x86_64; native Cargo release profile (optimized, workspace LTO), Node v26.12.0
running current release wasm32 production editor bindings. No spike result is used
as production proof. Outputs remain under `target/0066-runtime/`.

| Workload | Measurement and actual result |
| --- | --- |
| 100 entities: origin+33 independent fixed-center/point/line components, 33 hard distances | Actual native Session pointer hover + sketch view JSON + draw list; 10 warmups, 90 measured thread-CPU frames. Latest executed release median5.019038 ms/max6.015050 ms. All hard residuals/fixed centers checked. Stationary-component assertions passed the later debug geometry repeat. Initial debug median22.198/max26.155 ms is not delivery-profile performance. |
| Connected100: origin+50points+49lines, first chain point fixed, 49 hard lengths√34 | 30 actual native drags (20 feasible,10 beyond reach), first3 warmups,27 measured thread-CPU pointer+view+draw frames. Median9.828143/max10.988183 ms. Every hard length/fixed anchor validated; beyond-reach endpoint attains independently calculated triangle bound within1e-6 mm. |
| Same connected input, actual Node/WASM | 30 actual pointer drags with identical unrounded target/press coordinates and checked screen/document inverse mapping. Main-thread `process.threadCpuUsage`, pointer+view only (no GPU/draw bridge). Warmed27-frame maximum11.959 ms; **cold first frame30.901 ms**. Not a claim that every cold frame meets16 ms. |
| Actual numerical parity | Decode all30 Node-produced .curvyo snapshots natively, compare50 unrounded chain points each against native poses: maximum deviation2.481819080425819e-13 mm, below1e-6. Every frame retains100entities/49constraints, validates all49 lengths and fixed anchor, and is not stale. |

Reproduction from repository root (installed bindgen must match Cargo.lock):

```sh
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_chain --test acceptance_0066_session --test acceptance_0066_interactions -- --nocapture
cargo build --offline --release --target wasm32-unknown-unknown -p curvyo-editor-wasm
wasm-bindgen --target nodejs --out-dir target/0066-runtime/bindings target/wasm32-unknown-unknown/release/curvyo_editor_wasm.wasm
node curvyo-editor-wasm/tests/runtime/sketch-chain.cjs target/0066-runtime
cargo test --offline --release -p curvyo-editor-wasm --test acceptance_0066_chain s16_actual_node_wasm_unrounded_geometry_matches_native -- --ignored --nocapture
```

The native tests generate small input/pose artifacts; bindings and saved output
stay ignored under target. The tracked Node runner accepts artifact-directory and
optional bindings-path arguments. The artifact-dependent parity verifier is ignored
in default runs and **must be executed explicitly with the command above**. Geometry
assertions run in all profiles; <16ms performance assertions run in release only,
which is mandatory independent evidence, not replaced by default debug nextest.
Logs: `native-final.log`, `wasm-build.log`, `node-chain.log`, `parity.log`.

## Executed behavioral coverage

- `acceptance_0066_interactions`:4/4 release PASS after production duplicate-pick fix.
  Eight valid construction tools/variants via real pointer events; early duplicate
  three-point circle correction; polyline shared endpoints/Escape; formula stable
  rename and invalid-cycle atomicity; navigation and layer-New exactly one step.
- `acceptance_0066_session`:closest feasible radial point reaches(10/√2,10/√2)
  for target(20,20); release then remove Distance retains actual feasible pose.
- `acceptance_0066_lifecycle`:origin release/move/refix/undo and circle body
  translation versus radius handle PASS. Entered group-New parent/one step and
  locked/hidden layer refusal PASS after test preconditions corrected (two group
  members required; layer-row API takes JSON NodeID, not display hash).
- `acceptance_0066_document`:expanded14/14 PASS. Added forged checkpoint missing/
  extra IDs, construction membership and arc-radius rejection; style/translation
  fingerprint stability and path-writer refusal until conversion; hidden sketch
  clone own-style/unlink; converted arc/ellipse/slot analytic deviation≤0.01mm
  sampled41points/segment and expected closure. Sampling is not certified Hausdorff.

## Independent defects found

1. Connected chain beyond-reach request initially returned no preview with hard
   constraints did not converge. Producer fixed bounded feasible continuation and
   reach-bound certificate. Unchanged residual/progress/budget tests plus stronger
   independent triangle-bound endpoint check now PASS.
2. Duplicate first/second circle picks initially permanently trapped construction.
   Producer fixed correction; original pointer-sequence test now PASS.
3. Own constraint action cleared analytic selection. Horizontal succeeds,
   immediate second Horizontal fails Select one segment. UX requires retaining
   selected entities for further rules. Reproducer is
   `acceptance_0066_lifecycle::s12_dimensions_redundancy_conflict_repair_and_conversion_undo`.
   Producer fixed current-authoritative-ID selection preservation; the unchanged
   repeated-action assertion now passes, including redundancy/conflict/inspection
   and repair/undo. No reselect workaround was added.
4. Successful sketch conversion left public workbench Sketch instead
   of the accepted Vector return. New S14 assertion fails in the same lifecycle
   test. Producer fixed the successful conversion transition; independent Vector
   and single-output selection assertions now PASS, followed by conversion undo.

## Remaining before milestone completion

Re-run corrected lifecycle test and affected lint/format. Independent complete
constraint/dimension combinations, Alt/axis phase coverage, broader shape-body
checks and core adversarial ordering/angle-wrap/near-conics are not proved by these
initial tests. Deep recursive expression lifecycle remains open. UI layout/focus,
double-click dimension editing, native field undo and browser pointer routing await
frontend and real browser review. This report is scoped evidence, not AC1–18 PASS.

Latest bounded repeat: `cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_lifecycle --test acceptance_0066_interactions -- --nocapture`
ran interaction4/4 and lifecycle3/4, one conversion-workbench failure. Diagnostic
inspection assertion confirms selected constraint links the actual selected line
and does not claim FullyConstrained during conflict. Evidence: lifecycle-final.log.

## Final Rust handoff (before frontend)

`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_lifecycle --test acceptance_0066_interactions --test acceptance_0066_session --test acceptance_0066_chain -- --nocapture`
passed **11 tests**, with one artifact-dependent parity verifier ignored by default
(already explicitly executed successfully above). This repeat includes the stronger
triangle-bound and stationary-disconnected-component assertions, S12 inspection,
selection retention and S14 Vector/output selection. No known behavioral RED remains.
Debug chain timing was median409.445/max443.374ms; debug components max26.049ms.
These are recorded diagnostics, not release-performance claims.

All five new/expanded acceptance targets passed targeted Clippy `-D warnings`:
`cargo clippy --offline -p curvyo-editor-wasm --test acceptance_0066_chain --test acceptance_0066_session --test acceptance_0066_lifecycle --test acceptance_0066_interactions --message-format=short -- -D warnings`
and `cargo clippy --offline -p curvyo-document-core --test acceptance_0066_document --message-format=short -- -D warnings`.
`cargo fmt --all --check` passed.

Affected `cargo clippy --offline -p curvyo-editor-wasm -p curvyo-document-core --all-targets --message-format=short -- -D warnings`
now reports only three producer-owned lib-test findings: sketch/tests.rs:156
len assertion, sketch/drag_tests.rs:39 empty assertion, sketch/repair_tests.rs:142
104-line test. Tester handed these back without editing production-owned tests.
Legacy editor test helpers gained explicit unexpected-Sketch failure arms; existing
assertions were retained. Logs: rust-handoff-tests.log, handoff-clippy.log,
new-tests-clippy.log. No full workspace gate or actual frontend review performed.

Runtime parity was executed before the final selection/conversion-only fixes; those
changes did not alter numerical geometry. Rebuild final WASM and replay the tracked
Node/parity commands during final feature verification to remove artifact-version
ambiguity. Remaining independent relation/dimension/Alt/adversarial and browser
cases listed above remain explicit final-acceptance work, not an inferred PASS.

## Frontend-handoff verification: new open Fillet defect

Before beginning browser driving, independent S13 reproduced the architect's
source finding: valid2mm Fillet preview, invalid unknown-parameter text (Err), then
Commit still commits the old valid preview. Test
`acceptance_0066_lifecycle::s13_invalid_typed_radius_cannot_commit_previous_valid_fillet_preview`
fails before its no-history/no-geometry-change and corrected1mm checks. Command:
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_lifecycle s13 -- --nocapture`.
Log `target/0066-runtime/fillet-stale-preview.log`. Tester released the exclusive
slot to producer for repair; no production edit or assertion relaxation. Earlier
bounded handoff PASS remains historical evidence; final acceptance is blocked here.

## Final browser/runtime batch (2026-10-30; host performance investigation open)

- S13 stale typed Fillet preview: unchanged independent regression now PASS1/1;
  `cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_lifecycle s13 -- --nocapture`.
- `frontend/tests/browser/sketch-acceptance.cjs` executed against current WASM:
  actual annotation double-click, diameter Enter/Escape, invalid expression refusal,
  native input undo without document undo, free-centre/non-full diagnosis, conflicting
  dimension/row repair, Save. Explicit ignored document verifier
  `cargo test --offline -p curvyo-document-core --test acceptance_0066_document d01_actual_browser -- --ignored --nocapture`
  PASS1/1: unpacked solved radius12mm (committed diameter24), tolerance1e-6.
- `frontend/tests/browser/sketch-snapping-acceptance.cjs` PASS8 browser cases:
  Sketch Point same-pointer Alt, both stored states and four axis masks, exact saved
  point coordinates and unchanged stored settings. Raw disabled axes compare against
  an independently saved no-axis baseline; CSS frame rounding is not a geometry oracle.
- New public Session S11 PASS: Rectangle no-button following and button-held phases,
  key-only Alt on/off, both stored states, four masks, pinned anchor and no intermediate
  history. Construction preview is committed through its public API for exact geometry;
  `sketch_preview()` is a constrained-drag-only accessor and was not used as an oracle.
- Actual nonlinear runtime corpus `acceptance_0066_conics.rs` plus tracked
  `tests/runtime/sketch-conics.cjs`: fixed-centre/focus45° ellipse tangent to the
  parallel line3mm away yields minor radius3; circle Radius edit yields5mm. Native
  Session commands and identical actual Node/WASM commands PASS; native unpack compares
  all unrounded point/radius values≤1e-6. Generated bindings were refreshed from current
  production release WASM. Reproduce with `cargo test --offline -p curvyo-editor-wasm
  --test acceptance_0066_conics s16_native -- --nocapture`, the same wasm-bindgen command
  above, `node curvyo-editor-wasm/tests/runtime/sketch-conics.cjs`, then
  `cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_conics s16_actual -- --ignored --nocapture`.

Host CPU evidence is **not green**. CDP `Performance.enable(timeDomain:threadTicks)`
works. The DEV/SwiftShader two-rAF TaskDuration probe recorded cold91.891ms and
warm-after3 max72.272ms (raw second/third samples69.146/79.433ms). This is not a
single-frame delivery measurement. A fresh `npx vite build`, served with
`npx vite preview --host 127.0.0.1 --port 1421`, then the tracked
`frontend/tests/browser/sketch-host-cpu.cjs` measured production ThreadTime from
actual pointer move through the first rAF after the application's renderLoop:
cold46.725ms; warm-after3 max28.469ms; every later sample21.831–29.035ms.
Matching idle intervals2.944–4.005ms are reported separately, never subtracted.
Protocol tasks remain included and the endpoint may precede paint; no precise full
single-frame PASS is claimed. Explicit ignored native host-artifact check PASS1/1
confirms meaningful actual endpoint movement near(241.15,13.8), all49 hard lengths
and fixed origin, so this was not an idle/non-hit workload. CPU profiling is a
separate run, excluded from these timing results.
Artifacts: `target/0066-browser/{host-cpu.json,host-cpu-production.json,host-geometry.log,
host-profile.cpuprofile}`, functional screenshots and saved files in the same folder.
Browser commands use `CURVYO_BROWSER_URL`, `NODE_PATH=target/browser-check/node_modules`,
`PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers`,
`LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib`.
Full gate has not started. Remaining browser journeys and production host profiling
must be resolved before a whole-feature acceptance verdict.

Final focused command before performance handoff:
`cargo test --offline -p curvyo-sketch-core --test acceptance_0066_core -p curvyo-editor-wasm --test acceptance_0066_interactions --test acceptance_0066_lifecycle --test acceptance_0066_conics -p curvyo-document-core --test acceptance_0066_document -- --nocapture`
PASS: core19, document15(+1 artifact test ignored), interactions5, lifecycle5,
conicnative1(+1 parity test ignored). The two ignored artifact checks were explicitly
executed separately and PASS; host geometry also explicitly PASS. Focused all-new-test
Clippy PASS, including chain artifact test. Logs `target/0066-runtime/final-focused-batch.log`
and `final-batch-clippy.log`.
Separate profiler sampled inclusive attribution across20 moves: pointer_hover232.9ms,
render127.4ms (draw_list110.8ms), sketch_view_json76.8ms (annotations25.2ms),
modifiers_changed29.1ms, cursor_hint4.0ms. These are sampling estimates, not thread-CPU
budget measurements; profile timing JSON is explicitly named `host-profile-timing-not-gate.json`.
Exact committed host endpoint(241.15007349650062,13.799961543083187) proves real motion.
Functional additions above are green, but whole-feature acceptance remains pending
production host performance diagnosis and remaining browser coverage/full gate.

## Cache/leaf-subscription verification batch (in progress, current production preview1421)

B1 actual browser creation completed for Circle3 (duplicate pick then correction),
ellipse3, ordinary ellipse, arc, slot, Polyline Escape, typed Circle/Rectangle/Ellipse.
Added tracked independent geometry helper: saved Rectangle30×20, ellipse major/minor20×10,
Circle3 radius50px×25.4/96, ellipse3 minor30px×25.4/96/sqrt(1−.4²), ordinary ellipse
50/30pixel axes, arc radius/sweep/endpoints, slot30pixel radii and100pixel segment/centre
spacing. All seven saved-artifact analytical checks executed PASS at100%96dpi.
Typed Fillet browser now includes pre-existing Vertical/Horizontal IDs: valid preview,
invalid text+Apply retains zero arcs and both constraints, corrected1mm commits one arc
and retains both original IDs. PASS (`remaining-fillet-relations.log`).

Actual Reference save/open succeeded after synchronizing on the asynchronous Open
workbench reset. However the earlier test wrongly treated Reference row Enter opening
an editor as success. Accepted UX requires read-only measured output: strengthened
regression now **RED**, input count1 instead of0; nominal99mm obscures measured21.17mm.
Domain Reference/rank oracles remain unchanged. Evidence `reference-readonly-red.log`;
producer fix is queued. This supersedes the prior broad Reference UI PASS wording.

Actual Vector→Linked clone→Sketch enters original ID, Radius15 edit, Save PASS;
`sketch-clone-acceptance.cjs` produces `clone-entry.curvyo` for the native linked-geometry
verifier. Leaf regression `sketch-acceptance.cjs` rerun PASS: actual annotation edit,
invalid draft, native undo, conflict repair and saved circle. Exact current results
are separated from preparation and earlier artifact versions.

Latest scheduling-sensitive actual Point Alt/axes rerun PASS8cases (`leaf-snapping.log`).
Native unpack of actual browser clone artifact PASS1/1: one editable original, one
linked clone, solved radius15 and clone span30mm within1e-6; `clone-geometry.log`.
Focused document acceptance Clippy PASS (`cache-independent-clippy.log`).
No independent final trace or full gate ran in this batch: known performance issue
and Reference UI RED are handed back first. Prepared production trace finalization,
focused numeric-field Alt and final current-artifact runtime replay remain explicit.

Reference oracle clarification before independent fix rerun: the accepted contract is
read-only **measured quantity**, not a fixed two-decimal string. Existing panel
`quantityText` uses8 significant digits, while annotation text rounds differently.
The browser test now parses the actual `output` millimetre quantity and compares
against independent80pixels×25.4/96 =21.166666…mm within1e-6mm. It still forbids the
expression input and requires the checked/enabled Reference checkbox, retaining
explicit switching to driving. This test-only adjustment is not a production formatting
change and does not permit displaying the nominal99mm. Not executed during producer slot.

## Final independent functional/runtime handoff

All planned bounded checks in this batch PASS. No additional performance attempt
followed the single completed independent trace. A first connection-refused attempt
collected no samples; the production preview was restarted at1421.

| Evidence | Actual result |
| --- | --- |
| Strict Reference browser (`CURVYO_REMAINING_PHASE=reference`) | PASS: no expression field, measured21.166666…mm within1e-6, checked/enabled explicit Reference mode control |
| `sketch-field-alt.cjs` | PASS2 stored states: focused numeric draft/focus retained, unchanged-pointer Alt changes actual canvas preview, release restores identical geometry region; saved rectangle30×20 and unchanged flags independently verified |
| `ruler-cache.cjs` against source-module server1420 | PASS60 cached/uncached pixel comparisons across orientation, length, pointer leave and DPR1/1.5/2, existing≤2channel-rounding tolerance |
| `ruler-view-acceptance.cjs` against production1421 | PASS mounted ruler pixel invalidation on real middle-button pan, Ctrl-wheel zoom, viewport resize; DPR1/2 backing dimensions |
| `cargo test --release -p curvyo-editor-wasm --test acceptance_0066_session --test acceptance_0066_chain -- --nocapture --test-threads=1` | PASS3;2 artifact-dependent tests ignored in this command and explicitly executed below. Connected100:27 measured frames median5.80038/max7.139976ms. Independent100:90 measured frames median1.507406/max1.777638ms. All hard geometric oracles retained |
| Current production Node chain | PASS30 frames, pointer+view CPU cold25.903ms, warm-after3 max9.091ms (other warmup frames9.931/9.905ms); no draw bridge claimed |
| Explicit chain native artifact verifier | PASS1:30×50 full-precision positions, max native/WASM difference2.481819080425819e-13mm |
| Current conic native/Node/explicit verifier | PASS native1+verifier1: rotated ellipse tangent and radius edit, every point/radius within1e-6mm |
| Final actual host saved artifact verifier | PASS1: endpoint(241.15007349650062,13.799961543083187),49 hard lengths and fixed origin retained |

Reproducible complete-frame browser proof:
`CURVYO_BROWSER_URL=http://127.0.0.1:1421/ NODE_PATH=target/browser-check/node_modules PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib node frontend/tests/browser/sketch-frame-trace.cjs target/0066-runtime/chain.curvyo target/0066-browser/final-independent`
then `python3 frontend/tests/browser/sketch-frame-trace.py target/0066-browser/final-independent`.
The tracked analyzer requires20 inputs and actual renderer-main task thread durations,
checks nonoverlap, sums all tasks through first Paint after BOTH canvas render and
React overlay mutation, and enforces warm maximum<16ms without idle subtraction.
Actual cold32.884ms; all19 later samples11.097–15.626ms; warm-after3 max15.626ms.
This is the production optimized bundle on Linux x86_64 Chromium/SwiftShader, not a
wall-time or profiler-sampling estimate. Cold startup is explicitly outside the
warmed budget claim and remains a limitation worth tracking.

Current runtime reproduction: `wasm-bindgen --target nodejs --out-dir target/0066-runtime/bindings target/wasm32-unknown-unknown/release/curvyo_editor_wasm.wasm`,
`node curvyo-editor-wasm/tests/runtime/sketch-chain.cjs`,
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_conics s16_native -- --nocapture`,
`node curvyo-editor-wasm/tests/runtime/sketch-conics.cjs`,
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_conics s16_actual -- --ignored --nocapture`,
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_chain s16_actual -- --ignored --nocapture`,
`CURVYO_0066_HOST_ARTIFACT="$PWD/target/0066-browser/final-independent/chain-host.curvyo" cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_chain s15_actual_host -- --ignored --nocapture`.
Logs: `target/0066-runtime/final-{ci-performance,chain-node,chain-parity,conics-native,conics-node,conics-parity}.log`,
`target/0066-browser/{reference-final,focused-alt,focused-alt-geometry,ruler-view-final,ruler-pixels-final,final-host-geometry}.log`.

| AC | Independent acceptance evidence (bounded representative cases) |
| --- | --- |
|1| Session gesture/lifecycle tests; actual variants/typed browser, workbench restoration |
|2| Core ten-relation analytical residual cases; public eligibility/action and UI selection |
|3| Core five dimensions, formulas/units/cycles; document dependent roundtrip; actual annotation/panel editing, read-only Reference, native undo |
|4| Session fixed-origin drag refusal/release/move/refix/undo |
|5| Core known DOF/residual/fixed guards; Session diagnosis; actual free-centre/non-full UI |
|6| Reachable/beyond-limit and committed feasible pose; actual100entity native, Node and complete host-frame measurements above |
|7| Current-ID selection, redundant/conflicting original IDs, last-valid geometry and undoable repair; actual conflict-row removal |
|8| Document analytic conversion≤.01mm, closure, style/identity/history; Session Vector/selection and browser conversion/undo |
|9| Document chains/style/hidden-source/unlink/source removal; actual Linked clone→original edit and independent saved clone span |
|10| v28 valid/damaged goldens, forged checkpoint guards, formula roundtrip and accepted256-term formula; actual Save/Open |
|11| Session one-step creation/drag/constraint/origin/convert; document undo/peer/history and no read/preview writes |
|12| Current actual native/Node unrounded chain+conic parity; repeated/disconnected/angle-wrap known-answer core cases |
|13| Public Polyline shared IDs, completed-segment preservation/Enter; actual Escape artifact retains2segments |
|14| Circle/Ellipse variants, corrected degenerate pick, analytic saved radii/axes and typed geometry |
|15| Slot connected analytic topology, circle endpoint residuals, centre spacing/radii from actual saved browser gesture |
|16| Core virtual corner/radius edit, Session stale-preview invalidation, actual typed invalidation/correction with retained original relation IDs |
|17| Defining-point entity freedom, current selected IDs/linked rows, trustworthy global DOF, actual read-only measurement/diagnosis |
|18| Actual existing shell/variants/collapse/Vector route and retained rulers, UX review separately |

Scope limits: representative tuples rather than every relation×entity permutation;
pending raw Loro transactions/detached checkout have no public test seam and retain
internal producer coverage; no guarantee for arbitrarily deep formulas; cold frame
cost is above16ms. Source non-rigid scaling remains the customer's separate open
policy decision. Full gate awaits explicit lead handoff.


## Final quality gate — in progress, 2026-10-30

Logs and current stage: `target/0066-gate/status.md`.

- `cargo fmt --all --check`: PASS initially; final repeat pending after test-only glue.
- `cargo clippy --workspace --all-targets --message-format=short -- -D warnings`: PASS
  (`02-clippy-final.log`). Eighteen strict unexpected-Sketch match arms across thirteen
  legacy test files preserve every old assertion. Producer moved the unchanged UI
  sketch test module to EOF to satisfy the test-module-order lint.
- First `env -u WEBKIT_DISABLE_DMABUF_RENDERER cargo nextest run --workspace --status-level fail`:
  1852 passed, one stale `drawing_insert` format-range failure,126 skipped,7849 not run.
  The range now includes accepted format28; history/roundtrip assertions are unchanged.
- Full inventory retry with `--no-fail-fast`: **9702 run,9688 passed,14 failed,126 skipped**
  in448.206s (`03-nextest-final.log`). This is not a passing broad run.
- Thirteen failures: exact tab names/counts/labels in editor0043/0050/0051/history-tab
  and UI0043/0050/0051; editor0024 exact format27; editor0020 legacy-history sweep
  treating timestamp0 as proof of legacy. Corrected expected Sketches entries and
  format28 preserve old assertions. Six explicitly named new sketch fixtures use
  current headers; old-fixture coalescing and every unknown-label assertion remain.
  The dangling fixture additionally must contain exactly two linked_clone rows with
  distinct step IDs and row IDs.
- Fourteenth failure exposes **missing production operation names** for add_sketch,
  edit_sketch and convert_sketch_to_path. Producer handoff requested; gate paused.
  The same guard also scanned three external test-only modules; its explicit exclusion
  now asserts their exact cfg(test) declarations before omitting them. All production
  commit-label checks remain. No production edits by the tester.
- Remaining: corrected affected suites/guard, affected test Clippy, all10 WASM packages
  build/Clippy, cargo deny, warning-denied rustdoc, frontend full gate, final fmt.


### Gate retry closure and dependency findings

Producer added only the three missing operation-name mappings. Corrected affected
suites passed241 tests (2 existing ignored); affected document/editor/UI all-target
Clippy and final fmt passed. The final complete workspace command
`env -u WEBKIT_DISABLE_DMABUF_RENDERER cargo nextest run --workspace --status-level fail --no-fail-fast`
then passed **9702/9702,126 skipped,450.711s** (`03-nextest-green.log`).
All nine core crates plus editor WASM build and warning-denied Clippy passed.

`cargo deny check` failed exit3 (`06-deny.log`): document-core's new sketch-core
path dependency has no version, violating the existing wildcard ban; transitive
paste1.0.15 through simba0.9.1/nalgebra0.33.3/acs-solver0.1.19 has unmaintained
advisory RUSTSEC-2024-0436. Licenses and sources passed. Tester made no dependency
or policy edits and released the slot for disposition. Rustdoc/frontend pending;
full gate remains FAIL until these findings are resolved and remaining checks pass.


## Final gate closure — PASS, 2026-10-30

All stages below were actually executed locally. Earlier failures and their bounded
fixes above are audit history, superseded by these green results. No production
source was edited by the tester. Source history-name mappings were fixed by the
producer. The lead added the explicit local dependency version0.1.0 and the reviewed
informational RUSTSEC-2024-0436 exception under existing policy; package IDs/versions
match initial metadata and Cargo.lock has no diff. No solver version/features changed.

| Exact command | Final result / evidence under target/0066-gate |
| --- | --- |
| `cargo fmt --all --check` | PASS,01-fmt-final.log |
| `cargo clippy --workspace --all-targets --message-format=short -- -D warnings` | PASS,02-clippy-final.log; after bounded corrections, document/editor/UI all-target Clippy also PASS02-clippy-corrections.log |
| `env -u WEBKIT_DISABLE_DMABUF_RENDERER cargo nextest run --workspace --status-level fail --no-fail-fast` | **9702 passed,0 failed,126 skipped**,450.711s;03-nextest-green.log |
| `cargo build -q --target wasm32-unknown-unknown` with the package flags below | PASS,04-wasm-build.log |
| `cargo clippy --target wasm32-unknown-unknown` with the same package flags, followed by `-- -D warnings` | PASS,05-wasm-clippy.log |
| `cargo deny check` | PASS advisories/bans/licenses/sources;06-deny.log contains final retry, initial diagnostic details preserved above |
| `RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps` | PASS,07-rustdoc.log |
| `cd frontend && npm run build:wasm` | PASS,release rebuild38.11s;08-frontend-wasm.log |
| `cd frontend && npx tsc -b --noEmit` | PASS,09-typescript.log |
| `cd frontend && npm test` |715 total,641 passed,74 skipped,0 failed;10-frontend-tests.log |
| `cd frontend && npx oxlint` | PASS exit0,57 warnings;11-oxlint.log. No claim of zero warnings. |

WASM package flags (nine core crates plus editor):

```sh
-p curvyo-document-core -p curvyo-editor-wasm -p curvyo-geometry-core \
-p curvyo-image-core -p curvyo-render-core -p curvyo-sketch-core \
-p curvyo-svg-core -p curvyo-text-core -p curvyo-ui-core -p curvyo-vectorize-core
```

Corrected affected test targets also passed independently before the final complete
nextest run: document drawing_insert4; editor0020/0024/0043/0050/0051/history-tab205
(2 ignored); UI0043/0050/0051/history-name guard32. Total241 passed,2 ignored.
Previously documented artifact-dependent runtime and saved-browser tests were explicitly
executed separately; default nextest skips do not replace those results. No browser or
benchmark repeat was needed for the final three history labels or policy-only disposition.

Oxlint warnings remain visible in the raw log; the prescribed command does not deny
warnings. Native and WASM Clippy and rustdoc explicitly deny warnings and passed.
Windows/macOS nightly runs and customer story acceptance are outside this local gate.
