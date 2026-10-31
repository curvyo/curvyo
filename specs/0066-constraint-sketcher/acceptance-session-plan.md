# Independent Session and workbench acceptance plan

Status: docs-only, before new Session production-code review, 2026-10-30.
Authority: accepted specification AC1–18, UX notes, UI interaction map and session
plan. This matrix extends the independent core/document plans; it adds no scope.
Implementation owns the exclusive build slot. No tests, browser or builds run here.
Public Session tests and actual browser events follow only after explicit handoff.

| Case | Criteria | Observable checks and independent oracle |
| --- | --- | --- |
| S01 Lifecycle | AC1/9–11/18 | Enter empty Sketch without creating history/object; New creates one sketch in current valid layer and one named step. Vector returns with object selected; re-entry preserves IDs/relations. Clone entry routes to original, not a copied graph. Switch sketch/workbench cancels only pending input. Undo creation, delete active object, open another file and peer removal clear stale editable IDs. Repeated navigation/inspection writes nothing. |
| S02 Shell and accessibility | AC1/2/5/17/18 | Real browser: selector above Properties, Sketches in existing vertical tabs, sketch-specific left tools, one existing options bar; no extra header/Finish/breadcrumb/constraint strip/bottom diagnostic state. Preserve collapse, camera origin and tab activation rules. Keyboard reaches named actions/variants; invalid action is focusable, aria-disabled, reason available, activation inert. No implicit ordinary Pen/Star/Polygon tools. |
| S03 Creation events | AC1/11/13–15 | Actual pointer down/move/up through public Session for Point, connected Line/Polyline, Arc, both Circle methods, both Ellipse methods, Rectangle and straight Slot. Independently inspect analytic entities, shared endpoint IDs and intended relationships, not just rendered shape. Exactly one step per completed construction, except one per completed polyline segment. Hover/picks/previews add none. Optional centre Arc/Rectangle variants are not prerequisites. |
| S04 Construction correction | AC13/14 | Collinear three-point circle/arc, duplicate picks and zero ellipse axes refuse final completion but retain earlier picks for correction. Correct final pick completes once. Polyline Enter finishes open without another step; click first closes; Escape removes pending segment only, retaining completed segments/IDs. Variant/tool changes cancel pending picks without losing committed geometry. |
| S05 Ten relations | AC2/5/7/17 | Discover and activate Coincident, Horizontal, Vertical, Parallel, Perpendicular, Tangent, Equal, Fixed, Symmetric and Midpoint for supported selections. Check independent coordinate/vector/radius residuals and stable affected IDs; reversed unordered selection equivalent. Unsupported kinds remain unavailable with reason. Follow actual supported core contract: no invented ellipse Equal or two-point H/V convenience if unsupported. Redundant versus conflicting remains distinct; no automatic rule deletion. |
| S06 Dimensions/reference | AC3/11 | Distance, Length, Radius, Diameter and Angle entered via real eligible selection; bare length uses mm, angle unit explicit, unit expressions work. Driving edits alter independent geometry; Reference reads current measurement without changing rank/geometry. Annotation double-click and keyboard panel Edit reach equivalent editor. Enter/valid Tab commit once; Escape restores. Native input undo stays local, unfocused undo uses document. |
| S07 Named formulas | AC3/10/11 | width80, margin10, spacing=width−2×margin gives60; edit width100 gives80 in actual solved endpoints and clones. Rename width preserves stable dependency IDs. Unknown/duplicate names, incompatible units, zero division and cycles retain draft/error plus last committed values/geometry; correcting commits once. No per-character history. Save/open then edit proves retained semantics. |
| S08 Origin | AC4/5/11 | Origin initially fixed: ordinary drag cannot move it. Explicit move changes its coordinates without accidentally releasing fixed state; explicit release yields expected extra freedom. Undo/redo restores coordinate and state. No fabricated Fixed relation row when model uses a flag. |
| S09 Body versus handle drag | AC6/11 | Free line/circle/noncircular ellipse body drag translates shape without changing length/radii/axes; handle drag alters only intended control. Hard-constrained drag uses actual pointer path, preserves residuals, previews without history, release adds one step and Escape restores. Explicit geometry intent only; dependent solved pose stays derived. |
| S10 Beyond-limit movement | AC6 | Explicitly constrain a point initially(10,0) by the publicly available driving Distance from fixed origin to point =10 mm. A drawn circle alone does not bind the point; no new On-constraint UI action is required. With this hard distance, target(20,20) seeks feasible point near(10/√2,10/√2), preserving distance10 and showing meaningful movement. Also reachable target(0,10) reaches goal. Staying at old start is not success merely because residual remains valid. Invalid/non-finite target or numerical failure preserves last valid geometry with truthful failure and bounded work; failure is not evidence of conflict. |
| S11 Snapping lifecycle | AC1/6/13–16, existing0060 | Stored on/off × Alt held/released, including key-only change at same pointer, idle hover, first press without hover, pending picks, pressed drag and release. Existing X/Y masks and eligible kinds respected; pinned earlier picks do not jump; no implicit persistent constraints. Real browser includes focused numeric fields forwarding permitted modifier changes without stealing native text undo. No snapping-setting/history writes from temporary inversion. |
| S12 Diagnosis/repair | AC5/7/17 | Known ranks: free point2; anchored horizontal fixed-length line0. Duplicate relation shows redundant; contradictory valid length remains inspectable while old valid outline stays visible. Remove/edit affected original ID repairs in one step; undo restores contradiction and truthful diagnosis. Solver failure shows unavailable count when no valid evidence, never false zero/conflict set. Row/geometry inspection highlights actual linked entities without document mutation. All text diagnosis lives in Sketches; colour is supplementary. |
| S13 Fillet | AC11/16 | Right-angle constrained corner, radius2: center(2,2), endpoints(2,0)/(0,2). Preview then Escape preserves original; commit preserves/remaps applicable IDs and virtual construction corner, takes one step. Oversize/incompatible fillet names implicated constraints and refuses without dropping them. Undo/redo restores both topology and relationships. |
| S14 Conversion/persistence | AC8–11 | Convert actual active sketch returns Vector with same output selected; output excludes construction, annotations and parameter controls. Closed ellipse/slot/loop remains closed. Undo restores editable sketch, dimensions, formulas and clone propagation. Save/reopen is still editable; failed/stale diagnosis remains truthful. Check ≤0.01 mm final-coordinate curve error independently. |
| S15 Production performance | AC6/12 | Native production core and actual Session pointer path, deterministic 100-entity constrained workload, repeated feasible and beyond-limit drags; thread CPU (not wall time), report sample count/max/distribution and verify hard residuals each candidate. Include Session revision/cache access. Require <16 ms per frame; prior spike numbers are not substitute evidence. |
| S16 Runtime parity/adversaries | AC12 | Execute same bounded known-answer production corpus natively and actual Node/WASM, compare by stable IDs/diagnosis and coordinates within1e-6 mm. Use existing WASM runner pattern/tooling, not only target compilation. Repeated/reordered/disconnected inputs, ellipse/arc degeneracies, tangent carrier versus finite joint, angle wrap and invalid drag retain valid classification/residuals. Record production artifact/runtime; no huge duplicate fixture corpus. |

## Browser proof and remaining document gaps

Browser checks must drive actual canvas pointer and keyboard events and inspect
persisted output or exposed authoritative state. Calling only a Session method or
asserting source text does not prove host event routing, focus or layout. At minimum
cover S01/02, one corrected degenerate construction, polyline cancellation, both
dimension editors, formula correction/rename, conflict removal, Alt key-only,
conversion undo and save/reopen. UX review uses fresh production screenshots.

Carry forward document-report gaps explicitly: forged missing/extra candidate IDs,
construction membership/arc residuals; fingerprint order/style/placement boundary;
sketch-specific own-style, hidden source and unlink; converted ellipse/arc/slot
accuracy rather than only resolved clone sampling; path-writer refusal until
explicit conversion. Existing tests remain intact. Source-scaling policy remains
undecided; existing clone magnification is a valid separate contract.

Final report maps all18 criteria to actually executed evidence and gaps, separating
public API, browser, CPU and WASM runtime proof. Full gate runs once at final handoff,
after independent failures are resolved; tester never patches production or relaxes
an oracle to make the current implementation pass. Artifacts stay under target/0066-*.

## Bounded runtime harness preparation

Public declarations inspected after this matrix was written; production tests were
not read. No harness code, build or runtime execution belongs to this planning step.

1. Generate one deterministic v28 document fixture using public Document/Sketch
   commands, retaining stable IDs and known initial coordinates. Use 100 analytic
   entities with explicitly stated counts (points versus boundary entities), several
   constrained components, and at least one movable point with driving Distance10
   to fixed origin. Record entity/constraint counts; do not silently substitute a
   100-point unconstrained cloud. Fixture creation is outside measured frames.
2. Native opens identical bytes with `Session::open(peer, bytes)`, enters the named
   sketch, selects Select and drives `pointer_down`, `pointer_hover`, `pointer_up`
   and `modifiers_changed`; inspect `sketch_preview`, committed document result and
   `sketch_view_json`. Include `draw_list()` in a separately named complete-frame
   measurement. Existing `tests/cpu_time/mod.rs::cpu_time_of` uses Linux thread CPU.
3. Actual Node loads production bindings generated with `wasm-bindgen --target
   nodejs` into `target/0066-runtime/`. Public `WasmSession::open`, `enter_sketch`,
   `set_sketch_tool`, `sketch_select_entity`, `pointer_down/hover/up` and
   `modifiers_changed` expose the same interaction route. The bridge pointer inputs
   are screen coordinates: establish matching camera/viewport or convert using
   `screen_to_document`; never compare mismatched coordinate frames. Obtain
   diagnostics from `sketch_view_json`, persist via `pack`, and compare resulting
   analytic coordinates by ID to the native run. If the view omits candidate
   coordinates, decode packed output with the native test reader; do not infer
   parity from rounded display strings or canvas pixels.
4. Node v26.12.0 exposes `process.threadCpuUsage`: measure main-thread user+system
   deltas per warmed frame, retaining maximum and distribution. This supplements
   native CPU evidence; `process.cpuUsage` and `performance.now` are not AC6 thread
   CPU substitutes. Time pointer solving plus required view/cache reads explicitly;
   no browser/GPU timing claim. No production API is invented merely for testing.
5. For pure-core parity, prefer the existing CI pattern using
   `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test
   --target wasm32-unknown-unknown -p curvyo-sketch-core --test <bounded runtime test>`
   once a test-only harness exists. Installed bindgen/runner availability is setup
   evidence only. Compare actual numerical outputs from native and Node, not just
   separate successful builds or old spike outputs; no extra dependency assumed.
6. Include fixed-distance reachable(0,10) and unreachable(20,20) requests from
   (10,0), meaningful closest-feasible movement, release, then remove Distance.
   Geometry must remain at the actual released feasible pose, not resurrect the
   unreachable raw-pointer target. Also repeat with disconnected/reordered inputs,
   angle wrap, degenerate arc/ellipse/contact and invalid drag cases from S16.

S01 executable coverage must include New sketch inside an existing active layer
and entered group, with valid/locked/hidden contexts following existing editability
rules. Correct parent and exactly one named step are required for successful New;
a root-only one-step check cannot detect an accidental second reparenting step.

## Remaining actual-browser matrix after host scheduling optimization

Prepared, not yet executed. A leaf-subscription/scheduling change justifies rerunning
focus and modifier cases even where domain tests already passed. Keep domain
relation/dimension combinatorics in the independent19-case core corpus.

| Batch | Remaining actual host proof | Existing evidence to retain |
|---|---|---|
| B1 | Circle3 and ellipse3 dropdown variants; ordinary ellipse, arc, slot; corrected duplicate pick; Polyline Escape keeps completed segments; typed Point/Circle/Rectangle/ellipse controls and one invalid typed correction | Independent native creation8variants, corrected duplicate pick and Polyline tests PASS; previous browser Rectangle/Circle smoke already passed |
| B2 | Fillet preview→invalid typed radius→refused Apply→correct radius→commit, Escape/undo topology; independently inspect saved analytic graph | Unchanged S13 public regression now PASS; core fillet topology/radius-edit witnesses PASS |
| B3 | Reference toggle/measurement changes neither geometry nor global DOF; annotation Enter vs double-click; focused native undo, invalid draft/error correction, parameter rename/formula and focused Alt after leaf updates | Existing independent browser diameter edit/native undo/conflict repair/saved solved circle PASS; rerun only because subscription scheduling changes this route |
| B4 | Actual File/Open of saved edited sketch remains editable; Vector selection→Linked clone→Sketch entry edits original, clone remains linked; conversion+undo and collapsed workbench retained | Document clone/style/unlink/conversion fixtures PASS; producer browser conversion/undo/collapse PASS; independent File/Open chain succeeded during host probe |
| B5 | Existing actual Point Alt both stored states×four masks plus pending creation field key-only Alt, button-held and release update; no stored flag/history changes | Point8browser cases PASS; public Rectangle64phase/state/mask cases PASS; new host scheduling warrants bounded rerun |
| B6 | Production host100entity actual gesture CPU, independently validate saved motion/all49lengths, final current native/Node chain replay | Prior native+Node CPU/parity PASS with recorded cold sample; actual production host partial-frame overrun is open, not a pass |

Prepared script `frontend/tests/browser/sketch-remaining-acceptance.cjs` covers B1/B2
and Reference/save-open portions of B3/B4 with actual events and saved outputs. B4
clone entry requires the actual selected-object UI route and remains explicitly
pending until executed; no hidden Session helper substitutes for browser proof.
Existing `sketch-acceptance.cjs`, `sketch-snapping-acceptance.cjs` and producer
`sketch-workbench.cjs` are reused for scheduling-sensitive overlap instead of copies.
