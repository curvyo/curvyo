# ADRs for "Snapping"

Snapping is seven registers in the document root, one format bump, one pure engine in
`curvyo-ui-core`, one target list built at the start of a drag, one marker in
`curvyo-render-core` and the existing panel and bar controls. **No new crate, no new
dependency, no new ADR in `docs/adr/`, no trait, no generic.** Reference state: `main` at
`db776a01`, `format_version` 25. These are feature-local decisions, not accepted
architecture; they live here like `specs/0040-document-background/adrs.md`.

## Depends on

- [ADR 0002 §1, §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  the model is our own types and the Loro backing stays inside `document-core`. Snapping is
  a property of the document root, not a node, so §5 (tree, z-order, per-node style) does not
  apply.
- [ADR 0002 §3](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): every
  geometric comparison takes an explicit tolerance. The two zones are that tolerance,
  converted from screen pixels by the caller; the engine has no global epsilon.
- [ADR 0004 §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md): a build that
  cannot honour stored data must not open the file silently. Snapping settings change what
  every drag does, so a version-25 build opening a version-26 file would work against the
  file's stated intent without a word; the version goes up (decision 2).
- [ADR 0009 §3](../../docs/adr/0009-crdt-substrate-loro.md): per-field last-writer-wins root
  registers, one commit per user operation.
- [ADR 0001 §1, §2](../../docs/adr/0001-workspace-and-crate-boundaries.md): the interaction
  layer is plain state and pure functions, wasm-compatible; the renderer only draws what it is
  given.
- `specs/0040-document-background/adrs.md` decisions 1 to 5: the pattern for root registers,
  lenient read, strict `validate`, equal writes nothing and one commit.
- `specs/0046-document-shapes/`: the `guide` register and the guide drawing this slice reads.
- `specs/0047-transform-polish/`: the reference marks and the draw-list layer order the marker
  joins.

## Decision 1: seven root registers, defaults in the reader

`curvyo-document-core/src/snapping.rs` adds the value type and the registers:

| Register | Type | Absent means |
|---|---|---|
| `snapping_enabled` | bool | `false` (off) |
| `snapping_objects` | bool | `true` |
| `snapping_guides` | bool | `true` |
| `snapping_nodes` | bool | `true` |
| `snapping_page` | bool | `true` |
| `snapping_capture_px` | number | `8.0` |
| `snapping_release_px` | number | `14.0` |

- The read is lenient and writes nothing: an absent or malformed register reads as its
  default (a merged replica can hold one before validation). A file without any of them opens
  at the defaults and is not rewritten by opening (criterion 14).
- Strict `validate` on open refuses a present register of the wrong Loro shape or an
  out-of-range value as `OpenError::Damaged`, exactly as the background does. Valid range for
  both zones: finite, 1 to 100 px. A stored release smaller than the stored capture is **not**
  damage: it reads as the capture size (criterion 3), because it can arise from a legitimate
  pair of edits and the rule belongs to the behaviour, not to the file's integrity.
- `Document::snapping()` and `set_snapping(value)` mirror `background()` and
  `set_background()`: one commit labelled `set_document_snapping`, only the registers that
  differ are written, and an equal value writes nothing and returns `false`.
- All seven are top-level registers, not a nested map or a list: a peer editing two settings
  concurrently keeps both, and a future reader can ignore a single key. A nested map edited by
  two peers element-wise is the failure mode criterion 45 of `0040` avoids for the colour.
- The on/off switch and the four target flags are booleans, not one bit field, for the same
  reason and because a bit field is unreadable in `document.json`.

Where the value sits in the file: `document.json` gains a `snapping` object with the seven
members, next to `background` (`DocumentJsonView`). It is a view for people and tests; Loro is
the storage.

## Decision 2: `format_version` 26, golden file, no migration

`CURRENT_FORMAT_VERSION` goes from 25 to 26 with a comment paragraph in the style of the
existing ones. The number is **provisional until this PR merges**: if another story takes 26
first, this one rebases and renumbers itself, its fixture and its notes (the standing rule in
`document.rs`).

- Why a bump even though an old build would keep the unknown registers: it would not honour
  them. A version-25 build opens the file, ignores `snapping_enabled: true`, and the maker
  sees a document whose stated behaviour is missing with no message. It could then save from
  the old build after moving things unsnapped, and no later reader can tell. That is the
  silent partial read ADR 0004 §9 forbids.
- Migration from 25 is none: absent registers are the defaults. A version-25 file opens
  unchanged and byte-identical on disk, and a version-26 file with defaults omitted is a
  pure version change.
- New golden `tests/fixtures/snapping_v26.curvyo`: a document with a rect, a path, a guide and
  non-default values (on, objects off, capture 6, release 20), round-tripped with every
  component compared. Damaged fixtures: a non-boolean `snapping_enabled`, a string
  `snapping_capture_px`, a zero capture, a NaN release, and a container where a plain value
  belongs; each opens as `Damaged`.
- `specs/README.md` gets the row for 26 and the 0060 row moves from Draft to Ready.

## Decision 3: one pure engine in `curvyo-ui-core`

`curvyo-ui-core/src/snapping.rs` holds no document access and no rendering. Its types:

- `SnapSettings`: the seven stored values plus the target switches, already resolved (release
  raised to capture).
- `SnapTargets`: a flat list built once per drag, each entry `{ axis: Both | Vertical |
  Horizontal, point: Point, kind: TargetKind, extent: (Point, Point) }`; `TargetKind` is
  `Node`, `Guide`, `Box`, `Page` in the tie order.
- `SnapProbe`: the moving features of the current frame, per axis: the raw coordinates that
  may be pulled (edges, centre, corners, one anchor, one handle endpoint) and the same
  coordinates for drawing the cross.
- `SnapState`: the one held target per axis (kind, index into the target list, the target
  coordinate), ephemeral.
- `SnapResult`: the displacement to add to the raw position (`Vec2`), the held targets and the
  marker geometry (one or two lines, one cross). `None` when nothing holds.

The resolver is a pure function `resolve(settings, targets, raw, probes, state) -> SnapResult`
called from every move event. It takes tolerances in document millimetres; converting the
stored pixels with the current `view.scale()` is the caller's job, so the engine has no view
type and is unit-testable in document space.

Why here and not in `curvyo-editor-wasm`: the tool state machines that must call it live in
`curvyo-ui-core` (Select move and resize, Node anchors and handles, the shape tools, Pen). The
session only supplies the list and draws the result.

Why one function and not one per tool: the capture, release, priority and marker rules must be
identical everywhere; a per-tool copy would drift. A tool passes only the features it wants
snapped, which is how "handles snap to nodes only" is expressed (a filtered target list).

## Decision 4: hysteresis per axis, measured from the raw point

Per axis at most one target holds. Each frame, in this order:

1. If a held target exists and the raw coordinate is still within **release** of the held
   target coordinate, keep it. No other target on that axis may capture in the same frame.
2. Otherwise clear it and collect every enabled target on that axis whose distance from the
   raw coordinate is at most **capture**. The smallest distance wins; on exact equality the
   kind order decides (`Node`, `Guide`, `Box`, `Page`); on equality within a kind the list
   order (document order) decides, so the result is deterministic.
3. The output coordinate is the target coordinate, exactly; the displacement is that
   coordinate minus the raw coordinate.

Both distances are measured from the **raw** (unsnapped) coordinate, never from the snapped
one (criterion 4): a fast or slow sweep behaves the same, and a held target cannot keep itself
alive by its own output.

A node target is a point that may snap both axes. It is offered to both axis scans as a point
target; when it wins both axes it produces one cross and two lines, the same as two line
targets that happen to meet. It does not get a special "wins both axes" bonus: criterion 6
says it "wins both axes when its displacement is not larger than the nearest line-only
displacement", which is exactly nearest-wins applied per axis. The engine also exposes a
`both` scan for callers that move one point (a node or handle drag): it first looks for the
nearest node point within capture in both axes and returns a two-axis snap at once; if none,
it falls back to the per-axis rule.

The release zone is read as `max(release, capture)` (decision 1), so a target can never be
lost inside its own capture zone.

## Decision 5: the target list is gathered once at the press

The list is built in `curvyo-ui-core` by `SnapTargets::gather(...)`, from data the session
already holds:

- boxes: `read.visible` (effectively visible leaves, locked included), each through
  `object_bounds`; a group's or clone's own box is already what that function returns for the
  leaf the maker sees. Excluded: the dragged objects and every object hidden (`read.inert`
  holds hidden **and** locked, so the excluded set is computed from `visible` for the drag
  selection, not from `inert`).
- guides: the leaves whose id is in `read.guides`, through `object_outline_bounds` for the
  line extent and `PathSnapshot` anchors for the points. A clone of a guide inherits the
  flag; it contributes its own drawn geometry.
- nodes: every anchor of every visible path (first outline and later outlines), excluding the
  dragged ones.
- page: six fractions of `document.size()` on each axis, computed at gather time. The drag is
  modal, so the size cannot change under it; criterion 15's "same frame" applies to the next
  drag and to the lines' drawing, which reads the size each frame.

Why once: criterion 11. A 10,000-object document reads nothing per frame; a scan of a few
ten-thousand flat entries is one pass per axis per move event, inside the 2 ms budget. There
is no spatial index in the codebase and this slice does not add one; if the scan ever exceeds
the budget, an index is a separate decision.

The pack is dropped on release, Escape, tool change and a new press. Nothing is cached across
drags, so a moved object's new position is a target on the next drag.

## Decision 6: marker in `curvyo-render-core`, drawn by the session

`curvyo-render-core/src/snap_marker.rs` turns the engine's marker geometry into `DrawList`
quads: 1 device px lines with a 1 px white casing, a 12 x 12 px cross, constant screen width,
clipped to the viewport. The token is the existing `REFERENCE_MARK` (`#E5484D`): the reference
marks and the snap marker both mean "this exact place", the selection stays blue and the guide
green. No new colour token.

`session/draw.rs` appends the marker after the live previews and the reference box and before
the selection box, handles and the DOM overlays. The session owns the ephemeral `SnapResult`
of the running drag; no tool stores it in the document.

## Decision 7: Ctrl+Alt suspends, read from the event

Snapping is off while Ctrl+Alt is held, whatever the stored switch says (criterion 9). Both
flags already reach every tool in `Modifiers` (`curvyo-ui-core::modifiers`), and the Select
tool already uses Alt for the pick cycle and Ctrl for copy and angle snapping; the combination
is free in every tool. The modifier is a Proposal and may be vetoed at the demo; if it is,
only this rule and its UX notes change.

The event's own modifier state is used, not a remembered one: holding the keys during a drag
suspends in the same frame, releasing resumes at the next move. A suspended drag keeps the
last committed position; it does not jump back.

## Decision 8: page fractions are computed, never stored

`PAGE_FRACTIONS: [f64; 6] = [0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0]`. The customer's
"33.3 %" and "66.6 %" are thirds, not decimal 0.333 and 0.666: the line must coincide with the
thirds Inkscape and our own `0061-align-and-distribute` use, and 33.3 % of a 297 mm page is
98.901 mm while a third is 99 mm. The labels in the UI keep the customer's words.

The lines are `width * f` and `height * f` for the current `DocumentSize`; nothing is stored,
so a resize moves them in the same frame and a peer's resize moves them too (criterion 15).
There is no 0 % line (the customer listed six values), no grid and no separate centre line:
50 % is the centre. The right and bottom page edges are the 100 % lines.

## Decision 9: one history step, no snap in the model

A snapped move or resize calls the same commit as an unsnapped one (`translate_objects`,
`set_object_frame` and friends), with the exact target coordinate. The snap adds no step,
wrapper, marker record or metadata. The settings write is its own step. Undo of a snapped
drag restores the position before the drag, as any drag does; undo does not "unsnap".

## Decision 10: the settings are read once per session start, then kept live

`Session` reads `document.snapping()` at open and after every settings write, and passes the
resolved settings into the tools. A peer's setting change reaches the session through the
existing document-change path (the same one that refreshes the background), which re-reads the
registers. The UI's pressed state and the panel's fields read that value; there is no separate
cache that could disagree with the file.

## Decision 11: what this slice does not do

No grid, no path intersections, no tangent/perpendicular points, no text baselines, no
smart-guide measuring, no editable fraction list, no per-user override, no snapping section in
a future Settings tab, no snap during rotate or skew. Rotation keeps its 22.5° stops
(`0008`). Each of these would be its own spec.

## Milestones

The four milestones are in `plan.md`; each leaves the local gate green and is a commit on
`story/snapping`, not a PR.
