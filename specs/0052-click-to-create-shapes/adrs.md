# ADRs for "Click to create shapes: place a rectangle, ellipse or polygon/star with two clicks and a typed size"

First list of points by the product owner (2026-10-12), for the architect to confirm or replace with decisions. **New ADR expected: none.** No new crate, no new dependency, no new stored field and no `format_version` change: a placement writes the same object kinds and fields as a create-drag, and everything else (the placement state, the chip, the freeze, the r/⌀ mode) is view state.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1, §3: the live shape, the freeze, the typed-value resolution and the length parser live in Rust (`curvyo-ui-core`); `render-core` draws the preview and the anchor marker from the decoration input as for a create-drag; the DOM chip holds text, focus and the r/⌀ toggle only.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §3, §4: millimetres internally, the display unit only at the surface (criterion 24); every comparison with the geometric tolerance (criterion 23, "at or below the geometric tolerance"). §9: one placement is one commit.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: the placement, the chip's text and the freeze are ephemeral; nothing is written before the place, and a cancel writes nothing (criteria 7, 35).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no new key, no bump (criterion 36).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) and [`0020` adrs.md](../0020-undo-redo/adrs.md): a placement is one step with the existing labels `create_rect`, `create_ellipse`, `create_polygon`, `create_star` (criterion 6); no new label.
- [`0010` adrs.md](../0010-edit-interaction-polish/adrs.md) decision 3 (the DOM chip holds text and focus, the reading of the numbers is Rust) and the key gate of criterion 55, which gains "a placement runs".
- [`0047` adrs.md](../0047-transform-polish/adrs.md): the move chip's frontend parts (`EntryField`, the chip surface, the error line) are reused, not copied.
- [`0011`](../0011-shape-creation-from-center/specification.md) "The rule": the live shape is that rule's function, called with the pointer; the placement does not get a second implementation.
- [`0023` adrs.md](../0023-groups/adrs.md): a placed shape goes into the context like a dragged one.

## Points for the architect (to decide)

1. **One resolving function.** The create-drag, the placement preview, the second click and the typed result must come from one function per kind (criteria 2, 4, 15 to 19). Proposed input: A, a "size source" (the pointer, or typed and kept values), the kept modifiers, the kept direction, the angle. Where it lives (`shape_tool_common.rs` today holds the drag rule) and whether the three tools share the placement state machine is yours.
2. **The rotation pivot.** A typed Angle turns the rectangle and the ellipse about A (criterion 16 (b), 18; spec Question 3). The stored primitive keeps its centre, size and `rotation`; the function computes the centre after the turn. No new field.
3. **The length parser.** Criterion 24 accepts the display unit, mm, cm and in as suffixes and converts. One parser in `curvyo-ui-core` (next to `move_entry`'s number reading, which accepts a decimal comma and U+2212); whether the move chip adopts it later is spec Question 9, not this slice.
4. **The key gate.** "A placement runs" joins the operations in flight of `0010` criterion 55 (`session/keys.rs`), including the `0051` chords and the group chords (criterion 8). Space inside the placement chip is consumed by the chip (criterion 20), so the pan-on-Space path must not see it while the chip has focus.
5. **Focus and modifiers.** Focus sits in a DOM field during the placement, but Shift and Ctrl must still reach the session until the freeze (criterion 3). The window-level key listener the modifier badges use (`0010`) is the proposed route.
6. **Cancel triggers.** Criterion 7 lists them (Escape, secondary button, tool switch, a press outside the canvas, the chip and the tool's own bar). The tool's own bar must not cancel (criterion 25); the frontend knows where a press lands, the session decides.
7. **Performance budget.** Criterion 32: a 1024-point star preview at 60 fps; the same path as today's create-drag preview, so no new budget work is expected. Name the reference desktop.
8. **Tests at risk.** Acceptance tests that assert "a click creates nothing" (`acceptance_0003*`, `acceptance_0009*`, the creation-tool tests in `curvyo-ui-core`) are rewritten for criterion 1 (a click now starts a placement), not deleted. A test of a 1 or 2 px drag that expects a tiny shape changes to "starts a placement".

## Feature-local decisions

- 2026-10-12 (product owner, defaults of the spec's open questions until the customer answers): both Enter and the second click place; the first edit freezes; the angle turns about A; the second click places at the press.

## Flagged to the PO

Nothing yet; the architect adds here.
