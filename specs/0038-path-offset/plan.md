# Plan for Path offset: Outset and Inset by dragging a handle, with curves kept as curves

Rework of 2026-10-13 for the specification rewritten on 2026-10-12, amended on 2026-10-13 after the
customer tried the rework (56 criteria, some removed). One branch `story/path-offset`, one PR (#115,
reworked; no new PR), milestones each a green commit pushed to the branch. No `format_version`
change, no new crate, no new dependency, no `Cargo.toml` change. AC numbers are those of
`specification.md`.

**2026-10-13 amendment:** M1 to M4 (kernel, primitives, write, drag rules) stay. M5 and M6 (the
Offset bar, Apply, Ends, the status hint) are replaced by M5' to M9' below; the old M5/M6/M7 task
lists are kept for history, struck where the amendment removed what they built.

## What stays from the first build, what goes

- **Stays:** the `i_overlay` topology work (integer grid, `MathMode::Integer`, the polyline offsetter
  as the per-outline fallback of AC 39), the refusal codes and refusal outlines, `history_names`
  (`offset_path` → "Offset"), the blue hollow preview stroke, the golden and property tests (adapted
  to curve results), the session's preview cache keyed by document version and values.
- **Goes (no dead code):** the Select bar "Offset" button, the Properties panel Offset section
  (`OffsetSection.tsx`, its `PropertiesPanel` wiring, `useOffset.ts`), the entry open/close state
  of `session/offset.rs`, the drag range of ±50 mm, `add_paths_above` (replaced by the one offset
  write, which places copies below or above and changes originals), polyline-only result tests
  whose criteria no longer exist.

## Affected crates/modules

- `curvyo-geometry-core`: `offset.rs` stays the entry point and returns curve outlines; new
  `offset_pieces.rs` (the exact raw offset of one operand: parallel lines, `kurbo::offset::offset_cubic`,
  join arcs as cubics, miter/miter-clip/bevel lines, caps), new `offset_rebuild.rs` (flatten the raw
  offset with a tag per vertex, `i_overlay` overlay with the Positive rule decides topology, map every
  ring back onto the exact pieces, the per-ring polyline fallback and its deviation check),
  `lib.rs` exports.
- `curvyo-document-core`: `replace.rs`, one command `write_offset` (copies below/above their source,
  changed originals keep id and place, primitive fields or conversion to a path) in one commit,
  replacing `add_paths_above`.
- `curvyo-ui-core`: `offset_entry.rs` (bar values: unsigned distance 0.01..1000 mm, Direction, Corners,
  Ends, Result; codes), new `offset_primitive.rs` (the "stays a primitive" table), new `offset_drag.rs`
  (handle placement, d from the drag, snap series and snap to the bar value), `offset_plan.rs` (per
  operand and side: primitive or curve path, vanish counts, refusals, the writes for New object /
  Change original / Both / Shift+Ctrl, z-order).
- `curvyo-render-core`: `offset_preview.rs` draws curve outlines (flattened for the stroke), the four
  handles and the dashed grown box of a large selection.
- `curvyo-editor-wasm`: `Tool::Offset`; `session/offset.rs` (bar values, handle hit, drag with
  modifiers, preview per frame with drop of stale work, release writes one commit, Escape/other
  button cancel, Apply route, selection in the tool); `wasm_offset.rs` (state for the bar, readout and
  notice as codes and numbers); key **O**.
- `frontend/`: the "Path tools" rail card with the Offset tool, the Offset bar (`OffsetToolbar.tsx`),
  the readout chip, the notice; sentences in `lib/offsetText.ts`; removal of the Select bar button and
  the panel section.
- Docs: `docs/design-system.md` rows, `docs/technical-debt.md`, `adrs.md` implementer notes.

## Milestones and tasks

### M1 Curves stay curves: the kernel (AC 28, 30, 31, 33 to 39, 44 to 49)

- [x] 1.1 Tests first (`tests/offset.rs`, `tests/acceptance_0038.rs` adapted): circle path → 4 smooth
  cubics (35), rounded rectangle path → 8 segments, sharp square Round → 8, Miter → 4 lines (36),
  ellipse path ≤ 8 cubics within 0.01 mm of the exact offset (37), gear ≤ 400 segments and ≤ the
  polyline count (38), fallback counter printed and zero on the numeric tests (39), node rules (33),
  distance to the exact offset by sampling (34), numeric tests on areas of the curve result (45 to 49).
- [x] 1.2 `offset_pieces.rs`: raw offset loops of a closed outline (positive outer, negative hole) and of
  an open line (both sides and caps); joins only where a gap opens, the corner point otherwise; arcs
  one cubic per ≤ 90° (≤ 45° above 37 mm); miter-clip at 4 d.
- [x] 1.3 `offset_rebuild.rs`: tagged flattening, Positive overlay, mapping runs back to pieces, cut
  at crossings (Newton on the two exact pieces), smooth/corner by tangent (0.1°), collinear lines
  merged, nodes closer than 0.001 mm merged, deviation check against the ring, polyline fallback.
- [x] 1.4 Operands whose outlines cross (31) or touch take the polyline route for the whole operand.
- [x] 1.5 Determinism (44): golden fixture extended with curved cases (circle, ellipse, rounded
  rectangle paths, the gear), compared natively and in the wasm build; budget test prints segment
  counts and fallback count; 200 nodes within 50 ms at 2 mm and −1 mm (18).

Measured (release): the gear offsets in about 1 ms at 2, -1 and 5 mm; 180 segments at 2 mm;
the fallback fired for 0 of 135 outlines over the test shapes.

### M2 Primitives stay primitives (AC 40, 41, 43, 50, 51)

- [x] 2.1 Tests first, then `offset_primitive.rs`: rectangle (sharp, radii, Bevel → path), circle,
  ellipse (deterministic sampled deviation check, else path), polygon (Miter stays, Round/Bevel path),
  star (scale while no tip or inner corner is clipped), vanishing, rotation kept.

### M3 The write (AC 16, 20, 21, 22, 42)

- [x] 3.1 Tests first, then `Document::write_offset`: copies with the source's complete style below
  (outset) or above (inset) the source; Change original keeps id, name and place (path geometry
  replaced and rotation 0; primitive fields set; primitive → path in place); one commit
  `offset_path`; refusal changes nothing. `add_paths_above` and its tests go.

### M4 The plan and the drag rules in `curvyo-ui-core` (AC 6, 7, 9, 10, 11, 19, 20, 22, 26, 29, 30, 32)

- [x] 4.1 `offset_entry.rs`: bar values and codes, Direction/Result, modifier table (19, 26).
- [x] 4.2 `offset_drag.rs`: handle positions (12 px out on the diagonal, 24 px minimum box, rotated box
  for one rotated object), `d = (dx sx + dy sy) / 2` in the box frame, snap series ≥ 2 px, snap to the
  bar value within 6 px, limit 1000 mm (9, 10, 11).
- [x] 4.3 `offset_plan.rs`: per operand outer/inner results (primitive or path), vanish counts, the
  open-path rules (30), groups (32), out of range and no area, and the list of writes (20 to 23).

### M5 Session, wasm API and canvas (superseded in part by M5' to M8') (AC 2, 9 to 19, 23, 25 to 27)

- [x] 5.1 Tests first, then `Tool::Offset` and `session/offset.rs`: handles, drag, modifiers at
  release, inset stop at the last non-empty d, preview cache, release = one commit and the new
  selection, Escape and the other button cancel, Apply route, click/marquee/Ctrl+A selection, no move.
- [x] 5.2 Render: curve preview (blue, white casing), handles, dashed grown box over 2,000 nodes.
- [x] 5.3 `wasm_offset.rs`: bar state, readout state (side, d, result line, stays/becomes), notice.

### M6 The UI (superseded by M5' and M8') (AC 1 to 8, 13, 15, 24, 25)

- [x] 6.1 Remove the Select bar button and the panel section (3).
- [x] 6.2 "Path tools" card with the Offset tool, key O, active look shared with the Tools card (1, 2).
- [x] 6.3 Offset bar, always all controls, greyed with reasons, wraps at 800 x 600, keyboard route
  (4 to 8, 25); readout chip (15); notice (24); double-click focuses Distance (13).
- [x] 6.4 `offsetText.ts` sentences with unit tests.

### M7 Docs and gate (first rework)

- [x] 7.1 Design-system rows (Offset section removed, Path tools card, Offset bar, Offset handles),
  technical-debt entry updated, `adrs.md` implementer notes.
- [x] 7.2 Full gate (`CLAUDE.md` §7), frontend tests and `tsc`, once.

### M5' Remove what the 2026-10-13 amendment removes (AC 3 to 5, 15, 30, 42, 47, 55)

- [x] 5'.1 Kernel: open-path offsetting goes (`OffsetCap`, `open_loop`, caps, `stroke_line`,
  `flatten_open`); an open outline is refused as `OffsetError::OpenPath`; their tests and the two
  open-arc golden cases go. Closed-path results stay bit-identical (the golden fixture's closed
  cases unchanged) (55).
- [x] 5'.2 `curvyo-ui-core`: Ends, `InsetOfOpenPaths`, `open_left_out` and the kind note
  ("Stays a rectangle" / "Becomes a path") go; open paths are not operands (`offset_operands`,
  `plan_offset`); the bar's values become `OffsetValues` (the chip's remembered values) (15, 42, 55).
- [x] 5'.3 Session and wasm: Ends, the bar focus, the kind code and the open-path stop go; a
  selection of only open paths draws no handles (5, 55).
- [x] 5'.4 Frontend: `OffsetToolbar.tsx` (the top bar with Apply) and its wiring go; the status
  text and the Ends words leave `offsetText.ts` (3, 4).

### M6' Session: standing preview, modifiers, corner style by double-click (AC 6, 10, 13, 15, 19, 52, 56)

- [x] 6'.1 Tests first: the standing preview shows on selection and tool switch for the
  remembered values, after a write for the new selection, and is the outline a write gives (52);
  the press keeps it until the 3 px dead zone is left (ux-rework 3).
- [x] 6'.2 A drag never changes Direction; the dragged distance becomes the remembered one on a
  written release only (6, 10). Shift = Both, Ctrl = the other Mode, Shift+Ctrl = the original
  takes the dragged side (19).
- [x] 6'.3 A double-click on a corner handle steps a pending corner style Round, Miter, Bevel
  (from the pending one or the remembered Corners); the standing preview and the next drag use
  it; a written release consumes it, a cancelled drag keeps it, a tool switch drops it (13, 56).
- [x] 6'.4 The readout state carries the effective corner style for line 1 and the mode for
  line 2 (15).

### M7' Canvas and wasm: the center handle and opening the chip (AC 9, 25, 53)

- [x] 7'.1 Tests first: the center handle at the operand box center, its 12 px hit area claims
  the press (no marquee, no deselect), hidden during a corner drag (53).
- [x] 7'.2 A double-click on it, or O or Enter with the tool active and an operand selected,
  opens the chip; O without an operand keeps selecting the tool; gated like every shortcut (25, 53).
- [x] 7'.3 Render: the center handle (Move handle shell, Offset glyph, hover, active while the
  chip is open). wasm: chip open/close, Enter writes the values (refusals keep it open), the
  chip's anchor (center and the screen box around the operand box and the standing preview).

### M8' Frontend: the Offset entry chip, tooltip, readout (AC 1, 6, 15, 24, 25, 53, 54)

- [x] 8'.1 `OffsetEntryChip.tsx`: Distance (Up/Down 0.1, Shift 1, Ctrl 0.01 mm), Direction pill,
  Corners and Mode (Lucide `Plus`, `RefreshCw`) radio groups; Tab loop, roving tabindex, focus,
  Enter/Escape, message card, placement below the result (ux-rework 2) (25, 53, 54).
- [x] 8'.2 Tooltip second line, readout lines with the corner style, the corner-style chip after
  a double-click, hint chips for the corner and center handles (1, 15, 56).
- [x] 8'.3 `offsetText.ts` and `offsetChipPlacement` unit tests (15, 24, 54).

### M9' Docs and gate

- [x] 9'.1 `adrs.md` implementer note (open paths refused by the kernel), design-system rows
  checked against the build, technical-debt entry if any.
- [x] 9'.2 Full gate (`CLAUDE.md` §7), frontend tests and `tsc`, a headless browser check.

## Validation

- Kernel: unit and acceptance tests with exact segment counts and sampled distance to the exact offset
  (lines and arcs analytic; cubics by dense sampling of the source curve's normals), golden fixture
  natively and in wasm, property test (containment, monotonic area), budget test in release.
- Primitives: table-driven tests against the formulas and against the curve kernel on the same shape
  converted to a path (the two agree within 0.01 mm where both apply).
- Session: unit tests of the drag state machine with synthetic pointer events.
- Manual: `cd frontend && npm run dev`, press O, select a circle (the standing preview shows), drag a
  corner handle out with and without Shift and Ctrl, double-click a corner handle, press O to open
  the chip, Enter.
