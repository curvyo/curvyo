# Architecture: snapping

Date: 2026-10-30. Customer-decided: snapping settings belong to the document,
with fixed page fractions. Customer also waived old-file compatibility for this
unreleased build. Implementation decisions below are **Proposal**, reversible
within the story. No new crate, dependency, trait or generic is required.

## Existing decisions

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md): pure Rust
  interaction logic; rendering consumes explicit geometry.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  global millimetres, typed lengths, shared object tree and explicit tolerances.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md): native
  container format versioning; Loro remains internal to document-core.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md): independent
  register merging and ephemeral interaction state.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): conditional
  fieldwise restore, one operation per committed gesture.
- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md): guides
  are ordinary geometry flagged as guides, not bounding-box substitutes.

## 1. Document settings and version

Keep nine independent root registers: `snapping_enabled`, `snapping_objects`,
`snapping_guides`, `snapping_nodes`, `snapping_page`, `snapping_capture_px`,
`snapping_release_px`, `snapping_x`, `snapping_y`. Defaults: off, both coordinate
axes and all target kinds on, capture 8 px, release 14 px. Absent keys may resolve to defaults; old-format migration and historical
fixture compatibility are not obligations. Present malformed values are damage.

Zones accept finite nonnegative values, including zero for exact capture; no
unrequested upper limit. Normalize release to at least capture on reads and
writes, including independently merged fields. Keep pixel quantities distinct
from document lengths; convert using the current view scale for each resolve.
The settings writer commits only changed registers, as one labelled operation.
UI settings must reflect the authoritative normalized document after writes,
undo/redo and peer changes, not a separately authoritative session cache.

Pages has format 26; this open snapping story uses **27**. The additional axis
registers remain part of version 27: the story is unmerged and the customer
explicitly waived old-file compatibility for SID development. Regenerate current
golden fixtures and update the format comment together; version 28 would suggest
a separately released format boundary that does not exist. The independent
Loro snapshot version does not change. Keep a current-format golden round trip
and malformed-register tests; remove the assumption that zero is damage.

## 2. Targets and index

Build an immutable target collection and its spatial index from the existing
resolved document read at gesture start or the first drawing hover. No document traversal or full target
scan per frame. Refresh on an actual document geometry/settings change during
a gesture; do not interpret a modal drag as proof that peer edits cannot occur.

**Proposal:** two sorted coordinate arrays for axis-aligned box/page alignment
lines, plus a small static bounding-volume hierarchy for points and finite guide
segments/curves. Range queries on the arrays inspect only coordinates within the
capture zone of each moving feature. A balanced binary AABB hierarchy, built by
median splitting along the widest axis, queries point/curve bounds against the
capture rectangle. This needs only `Vec` and existing geometry types, no new
crate. Candidate sets can be large in genuinely dense scenes; do not silently
truncate them or introduce a target cap. Reuse an existing suitable index if
available instead of adding a second implementation.

Guide candidates retain their actual line or curve geometry. Bounds only prune
candidates; exact nearest-point geometry determines the target. Diagonal and
curved guides must never become their bounding-box edges. Reuse the geometry
kernel's projection operations with an explicit tolerance. Guide markers depict
the matched guide segment or curve, rather than a fictitious axis-aligned line.

Gather effective visible geometry, including locked references. Include group
bounds from resolved descendant geometry and clones at their resolved placement.
Exclude the dragged movement unit and its descendants; for Node editing exclude
only dragged anchors/handles as appropriate, retaining other anchors of its path.
Resolve guide and node switches independently when a guide has path anchors.

Page targets are computed, never stored: fractions `1/4, 1/3, 1/2, 2/3, 3/4, 1`
of each frame's width/height plus its global origin. No zero line or separate
centre target. **Working default pending the customer's scope answer:** all
visible page frames. Replace this default if the customer chooses active-page
scope; do not infer it from the old single-document-size API.

## 3. Resolver and raw hysteresis

Keep a pure resolver in ui-core, split into types, target/index construction,
resolution and tests. Session code supplies geometry and modifiers; renderer
code consumes marker geometry. No document writes from the resolver.

Represent moving features as actual points and eligible axis lines, with stable
identity during a gesture. Axis targets may independently resolve x and y.
Point targets are atomic two-dimensional candidates: compare actual point
movement with competing line-only correction, using the specification's
nearest-wins rule and Node, Guide, Box, Page priority on ties. Never offer a
point's x coordinate as an infinite line independently of its y coordinate.

Retain held target identity and moving-feature identity. Measure release from
the raw pointer-derived, constrained geometry, never from the previously snapped
output. For a held curve compute projection against that same curve. Keep an
atomic point held or release it atomically. Use current-scale capture/release
lengths and explicit comparison tolerances. Zero capture means only an exact
match within the kernel comparison tolerance, not snapping disabled.

Apply tool constraints before constructing eligible features, then resolve only
corrections that preserve those constraints. Shift axis lock permits only the
free axis. Resize probes include moving edges/reference points, never fixed
edges. Aspect/centre constraints must still hold after snapping. Handles query
node points only. Pen first click and later placement share the same resolver.
Preview, readout and release must consume the same resolved geometry; release
resolves the actual release-event position and modifiers, not a stale hover.

## 4. Gesture and rendering integration

Implement Select move, Select resize, Node anchors, Node handles, Rectangle,
Ellipse, Polygon/Star and Pen. Rotation retains its existing angle steps.
Ctrl+Alt bypasses snapping from current event modifiers and clears held markers;
releasing it resumes from raw geometry. Drop gesture state on release, Escape,
tool change and document replacement. Turning document snapping off immediately
removes both attraction and markers.

Only final geometry enters the existing operation commit; a snapped gesture
adds no extra history step. Settings changes are their own document operation.
Marker geometry remains ephemeral. render-core produces constant-screen-width
lines/curves and a cross using the existing reference colour, with viewport
clipping. Session drawing places it after artwork/reference previews and before
selection handles. No tooltip or text label.

## 5. Customer's settings UI

A dedicated **Snapping** entry in the Properties rail opens its settings on
normal click. Shift-click toggles snapping; enabled state is blue. The settings
page contains target switches and capture/release fields. This supersedes the
old Select toolbar and Document-block proposals; do not squeeze controls into
the narrow toolbar. No localStorage or per-user settings store.

## 6. Verification and completion

Reuse document format/merge tests and valid line-hysteresis tests. Replace tests
that assert bounding-box guides, one-axis point attraction, arbitrary zone caps
or missing tool support. Cover every tool's preview and release, constraints,
independent simultaneous axis targets, atomic point ties, guide geometry,
groups/clones, hidden/locked state, cancellation and undo/merge settings refresh.

Measure indexed resolution on 10,000 objects against the same unsnapped drag:
no more than 2 ms incremental **thread CPU** per frame on the reference machine.
Include representative guide curves and dense candidates; report measured
results and index build cost. Do not substitute a wall-clock benchmark or hide
an over-budget full scan behind the existing branch's amended wording.

See `review.md` for the takeover findings. Its unchecked items are outstanding
review work, not claims that any future revision still contains the defect.

## 7. Customer follow-up: axes, Alt and drawing hover (2026-10-30)

The customer requests independent horizontal/vertical snapping and snapping
before the mouse button is pressed, including first clicks and click-based
creation. **Proposal:** horizontal means X-coordinate correction; vertical
means Y-coordinate correction. This is the working default pending the optional
wording question. Two independent bool root registers default true; no bitfield,
per-user override or separate per-tool persisted switches. Existing fieldwise
history handles them. Malformed present values are damage.

### Axis eligibility

The axis mask constrains allowed displacement, not which coordinates are
considered when finding points. With X enabled/Y disabled, a node must still be
within the full two-dimensional capture/release radius and have the same Y
within the kernel comparison tolerance. Otherwise it is unreachable and is
rejected; do not turn its X into an infinite line. Both axes off yields no
correction or marker. Keep atomic-node identity; never clip a two-axis correction
after selecting an unreachable node.

Intersect the document axis mask with each tool's permitted movement direction.
For one free axis, curves use finite curve/axis intersections; diagonal aspect
constraints incompatible with that axis permit no correction. Existing Shift
axis locks still take precedence over attraction. Axis-line/guide composition
must also obey this intersection. Clear incompatible held state when the mask
or modifiers change. Capture and release continue to use raw geometry.

### Alt inversion and existing actions

The literal customer request is temporary inversion **while moving**. Apply
`effective_enabled = stored_enabled XOR Alt` to an already-running Select move;
read the current modifiers on motion, modifier change and release. This rule
also applies when Ctrl is held: during a Select move Ctrl+Alt inverts rather
than forcing off. Existing copy semantics are preserved by `ctrl && !alt`;
Ctrl alone copies, Ctrl+Alt does not. Outside Select moves the existing
Ctrl+Alt bypass remains unchanged. Neither inversion nor the effective
state is written to the document or history. Marker visibility must use the same
effective state, so Alt can show a marker while the stored switch is off.

Alt at Select press already arms a lasso; keep that hit/pick behavior. Inversion
starts only once the gesture is a move. Offset uses Alt for corner style; do not
hijack it. Pen also has existing Ctrl/Alt handling. Extending inversion globally
is unnecessary for the requested move behavior and risks unrelated changes, so
this implementation keeps drawing hover on the stored switch. No question is
needed to preserve these existing operations.

### Hover lifetime and click creation

Reuse the indexed snap context for an idle drawing hover instead of constructing
a second snapping engine. Add a small ephemeral context distinction (hover vs
gesture, or equivalent explicit intent) so the UI's drag flag does not claim a
button-down drag during hover. Cache targets by document version and exclusions;
settings/scale/mask resolve live. The first hover may build the index; subsequent
hover frames only query it. A version change rebuilds once, never a full document
read per pointer frame.

Shape hover and Pen hover, including before the first node, resolve the raw
pointer to a marker. On press recompute against that event's point/modifiers; do
not trust a stale hover or feed snapped coordinates to Select/Node hit testing.
Promote the existing drawing context to a gesture without an unnecessary rebuild
when its exclusions and document version are unchanged.

The current shape click starts the existing typed-size placement flow. Its
release must use the snapped anchor and subsequent placement-follow/confirm must
use the same resolver; an early `release_starts_placement` return must not bypass
snapping. Preserve click thresholds based on raw pointer movement, so attraction
alone cannot turn a click into a drag or invent Pen handles. Do not replace the
accepted placement interaction with an unsolicited immediate-shape command.
Clear hover markers on pointer leave, Escape, tool/document change and disabled
effective snapping; retain raw pointer state for immediate modifier recompute.

Tests must distinguish hover/no-button from drag; verify first click, simple
click placement and Pen handle-free clicks, both Alt inversion directions with
no persisted changes, all four axis combinations, nearby-but-unreachable and
distant nodes, finite curves, constrained resize, marker state, merge/undo of
axis settings and unchanged indexed frame budgets.
