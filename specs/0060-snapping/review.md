# Snapping takeover review

Date: 2026-10-30. Initial verdict: **CHANGES REQUESTED** on the inherited implementation.
Final structural verdict: **APPROVE** after the composition fix (see final checkpoint).
Checked R-items below mean the source finding is addressed with the reported
focused evidence; they do not replace independent acceptance or the full gate.
Source review itself was read-only. Lead-reported baseline: `cargo check -p
curvyo-editor-wasm` passes with six warnings; `cargo test -p curvyo-ui-core
--lib snapping` fails compilation with 30 errors from changed tool signatures. Reuse the document
register/default/format infrastructure and valid hysteresis tests. Rework target
collection and resolution; complete the existing tool pipeline instead of
rewriting the application. Close items only with source review and relevant
verification evidence.

- [x] R1 — `session/select.rs`: resolve snapping during move preview, not only
  release. Preview, readout and final commit must use the same resolved geometry.
- [x] R2 — Tool coverage: add Select resize, Node anchor/handle and Pen first and
  subsequent placement integration. Handles use node-only targets.
- [x] R3 — `ui-core/snapping.rs::push_guide`: replace guide bounding-box edges
  with exact segment/curve projection after spatial candidate filtering.
- [x] R4 — `resolve_axis`: replace independent point-coordinate attraction with
  atomic 2D capture/release and comparison against line-only correction.
- [x] R5 — `SnapTargets::gather`: include group bounds and correctly exclude
  dragged descendants; use resolved clone geometry and effective visibility.
- [x] R6 — `select_tool/move_drag.rs`: preserve the locked axis under Shift;
  apply only corrections permitted by the tool's constraints.
- [x] R7 — Shape creation: exclude fixed edges, resolve movable geometry under
  aspect/centre constraints, and keep preview and committed coordinates equal.
- [x] R8 — Rendering/lifecycle: implement markers and clear them on release,
  Escape, tool change, document replacement, disable and Ctrl+Alt suspension.
- [x] R9 — Session settings: remove stale separate authority; refresh normalized
  values after writes, undo/redo and peer changes, including active gestures.
- [x] R10 — Document zones: remove the unapproved 1–100 px restriction. Accept
  finite nonnegative values; release resolves to at least capture; zero is valid.
- [x] R11 — Page fractions: use page origins and sizes after pages merges;
  working default is all visible frames, pending the customer's scope answer.
- [x] R12 — Performance: replace per-frame full scans with prebuilt indexed
  candidate queries; demonstrate the 10,000-object incremental 2 ms thread-CPU
  budget without arbitrary candidate truncation.
- [x] R13 — Engineering: split the 1,105-line engine by responsibility, correct
  unsafe unwrap assumptions and stale documentation links, retain core purity.
  Existing tool tests fail to compile after snap-offset parameters were added.
  Prefer retaining unsnapped public wrappers with existing semantics and using
  an explicit constrained-geometry integration path; do not weaken assertions
  or mechanically change all tests without validating the API contract.
- [x] R14 — UI/format completion: dedicated Snapping Properties rail entry;
  normal click opens, Shift-click toggles blue. Remove inherited narrow-toolbar
  UI. Renumber snapping to the next free format (27 after pages takes 26),
  regenerate current fixtures, and remove obsolete old-file obligations.

## Implementation sequence (Proposal)

1. Fix settings authority/validation and establish target/index/resolver tests
   for actual guides, atomic points and constrained correction (R3–R6, R9–R13).
2. Wire every gesture's preview and release through that resolver, including
   fresh release modifiers and state cleanup; then render markers (R1, R2, R7, R8).
3. Integrate the Properties rail settings and page frames, reconcile format
   version, then run focused acceptance tests and the measured budget (R11, R12, R14).
4. Architect reviews the resulting boundaries; tester performs the full gate.

## Engine follow-up review (2026-10-30)

Read-only review of `snapping/{collection,index,targets,resolve}.rs` after phase A;
27 focused tests passing is implementer-reported, not rerun by the architect.

- R3: source now retains finite guide cubic geometry, computes kernel projection
  with an explicit 1e-9 mm tolerance and uses curve bounds only for pruning.
  Guide anchors may independently contribute to the node target kind.
- R4: point candidates now query both coordinates, capture by Euclidean distance
  and retain/release a shared target identity atomically; distant ghost-axis
  attraction is removed. Remaining boundary review: zero capture and index
  ranges must use the agreed explicit comparison tolerance consistently.
- R5: descendant and following-clone closure uses an adjacency queue, including
  transitive followers; group bounds are aggregated. Remaining issue: a moving
  child must also exclude ancestor aggregate boxes containing it, without
  suppressing stationary sibling leaf targets.
- R12: actual sorted axis range queries plus a static AABB tree are present;
  queries prune by bounds and never truncate by document order or target count.
  The measured 10,000-object CPU budget is still outstanding.
- R13: engine responsibilities are split across modules. Tool integration/API
  compatibility and the rest of the engineering checks remain open.

These are partial source findings; checklist rows stay open until their full
acceptance and regression evidence is available. Tool/UI completion is not
assessed by this engine-only review.

## Gesture follow-up review (2026-10-30)

Bounded source review during active integration; no tests run by architect.
Ancestor aggregate-box exclusion now has a separate upward pass, preserving
stationary sibling leaves. Capture/release and index candidate ranges now use
1e-9 mm boundary tolerance. Both earlier source issues are addressed.

Move previews and release now invoke the resolver; resize exposes a moving
feature/direction; shapes probe the movable endpoint under their constraints.
Node anchors/handles and Pen placement now have adapters. These are integration
observations, not completed acceptance claims.

Actionable issues sent to the implementer:

- R2/R7: Pen release applies `snap_press_offset` in both `pen_handle_point`
  and `pen_pointer_up`. Remove the duplicate and test snapped click/handle-drag
  final geometry against its preview.
- R2/R8: Node and shape release adapters resolve using `self.held` modifiers,
  but the release arguments are not synchronized first as Select does. Ensure
  Ctrl+Alt suspension uses the release event's state.
- R9/R11: active resolves refresh settings, but the gathered targets remain
  filtered by the kinds enabled at press and geometry is frozen. Rebuild on
  actual document/target-setting changes so newly enabled kinds and peer/page
  edits take effect; do not rebuild on every pointer event.

Node Shift was investigated: it affects the existing Bend gesture, not anchor
or handle position resolution. No new axis-lock behavior is requested for them.
The full constrained-tool, lifecycle and performance verdict remains pending.

## Post-integration source review (2026-10-30)

The earlier Pen double offset is removed. Common pointer release now updates
held Shift/Ctrl before dispatch. Target collections gather all kinds, resolve
with current flags/zoom, and rebuild on document-version changes with preserved
movement exclusions; held indices are cleared on rebuild. Page fractions now
include each page's origin and dimensions through `gather_pages`.

One concrete constrained-resize issue remains: `select_tool/snapping.rs` derives
its Ctrl aspect direction from the original corner signs, ignoring the signs
of `resized.sx`/`sy`. After mirroring only one axis, the valid aspect ray changes
quadrant. For a 20 by 20 box, southeast resize to (-10, 10) permits a correction
toward (-12, 12), but the current (+1, +1) direction rejects it. Derive the
constraint from signed resized geometry and verify preview/release equality,
including a rotated case (R6/R7).

Current session tests assert useful final coordinates and preview equality for
basic moves, nodes and unconstrained corner resize, plus live geometry/settings
refresh. These do not alone prove mirrored/rotated aspect or centred resizing.
Sparse-index CPU numbers reported by the implementer do not substitute for the
still-pending representative dense-curve benchmark. No architect build/test run.

## Final structural review checkpoint (2026-10-30)

**CHANGES REQUESTED: compatible guide/edge composition (R4, AC6).** Current
finite guides are represented as atomic two-axis targets. For raw (99, 99), a
box vertical edge at x=100 and a horizontal finite guide at y=100, projection
onto the guide yields (99, 100). Its one-millimetre correction wins by guide
priority over the box's one-millimetre correction, replacing both axes and
losing the compatible x snap. AC6 requires simultaneous edge/guide axes. Compose
compatible finite-guide and line constraints without making node targets
independent ghost axes; test both orientations and retained-state behavior.
This source finding was sent to implementer, lead and independent tester.

Other reviewed boundaries are sound: document registers remain sole authority;
format is 27 with renamed current fixtures; renderer receives its own plain
segment type rather than depending on ui-core; production snapping modules are
under 500 lines; no new dependency, unsafe code, unlinked TODO or unchecked
production unwrap was found in the bounded changed modules. The 829-line test
module is test-only. Curve/index scans have no target cap.

Reported evidence, not rerun by architect: engine 31/31, session 10/10,
format 17/17, pages 13/13; sparse 10k incremental 0.0069 ms/frame and dense-curve
10k 0.0044 ms/frame thread CPU; renderer widths at three zooms; lead browser
interaction checks. Independent acceptance and full gate remain tester-owned.
Existing earlier checklist items are not blanket acceptance claims; final source
approval waits for the composition issue above.

## Final checkpoint: structural approval (2026-10-30)

**APPROVE.** The AC6 composition blocker is addressed in `snapping/compose.rs`.
All nearby indexed finite-guide candidates are considered without a cap; exact
finite intersections reuse the constrained solver. Compatible guide/axis
constraints compose before atomic-point distance competition. A retained guide
keeps its original target and feature identities through private curve-hold
state; switching to constrained resolution clears obsolete curve state. Markers
retain the actual curve plus the independent axis.

Composition priority is a reversible implementation choice: compatible
independently eligible axes are combined before comparing atomic node targets.
The strongest farther-guide regression starts with the axis already held; a
nearer finite guide that ends before it cannot displace that retained axis.

Lead/implementer-reported focused evidence after the fix, not rerun here:
independent core acceptance 10/10, engine 32/32, session 10/10; performance 2/2
with sparse/dense incremental thread CPU 0.0096/0.0055 ms per frame; wasm build
reported ready. Earlier format 17/17, page 13/13 and browser reports remain
separately attributed above. All fourteen takeover source findings are closed;
independent final acceptance and full quality gate remain tester-owned.
