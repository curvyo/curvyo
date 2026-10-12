# ADRs for "Click to create shapes: place a rectangle, ellipse or polygon/star with two clicks and a typed size"

Architect decisions of 2026-10-12, checked against `main` at 6841fb6. They replace the product owner's
first list of points.

**Summary.**

- No new ADR. No new crate. No new external dependency.
- No stored field and no `format_version` change. A placement writes the same kinds and registers as a
  create-drag. A rotated rectangle or ellipse uses the `rotation` register that every primitive has had
  since `0005`.
- One public API addition in `curvyo-document-core`: `Document::create_primitive(shape, rotation)`,
  decision 2. Under `CLAUDE.md` §4 the architect reviews the milestone that adds it, on the branch diff.
- Nothing here is `needs-customer`.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md):
  - §1: the placement state, the freeze, the typed resolution and the length parser are `curvyo-ui-core`
    state and pure functions.
  - §4: `render-core` draws the preview and the pivot marker. The chip is a DOM overlay for text input
    only.
  - §5: the chip's texts cross the wasm boundary as small per-interaction messages, never per frame.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §2, §3: millimetres inside, and `Length` and `Angle` newtypes. Every comparison (criteria 5 and 23)
    takes an explicit `Tolerance`.
  - §4: Y grows down, so "clockwise on screen" is a positive `Angle`.
  - §6: a rectangle and an ellipse stay primitives.
  - §9: one placement is one commit.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: the placement, the chip's texts,
  the freeze and the r/⌀ mode are ephemeral. Nothing is written before the place, and a cancel writes
  nothing (criteria 7, 35).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no format bump (criterion
  36).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) and
  [`0020` adrs.md](../0020-undo-redo/adrs.md): one commit is one step. The existing labels
  `create_rect`, `create_ellipse`, `create_polygon` and `create_star` keep their names "Draw rectangle"
  and so on (`history_names.rs`). No new label (criterion 6).
- [`0010` adrs.md](../0010-edit-interaction-polish/adrs.md):
  - Decision 3: the DOM chip holds text and focus; Rust reads the numbers. `parse_entry_number`
    accepts a comma, a leading "+" and U+2212. The surface of `MoveEntryChip.tsx` is reused.
  - Decision 4: the key gate in `session/key_table.rs`, which gains "a placement runs" (decision 4
    below).
  - Decision 6: the Escape cascade. The placement is step 2, "an operation in flight".
- [`0011` adrs.md](../0011-shape-creation-from-center/adrs.md) and its spec, "The rule":
  `create_drag_box` in `shape_tool_common.rs` is the one rectangle and ellipse drag rule. The placement
  calls it and has no second copy.
- [`0015` adrs.md](../0015-document-size-and-rulers/adrs.md): `DisplayUnit`, `Length::from_unit`, and
  `display_unit_text.rs` as the home of display-unit text parsing (criterion 24).
- [`0023` adrs.md](../0023-groups/adrs.md): `Document::set_draw_parent` puts a placed shape into the
  entered group, the same way as a dragged shape. No new code.

## Feature-local decisions

### 1. One resolving function, one placement state (2026-10-12; criteria 2, 4, 12, 14 to 19)

- **The live shape.** Each creation tool gets
  `pub fn shape_for(&self, a: Point, b: Point, modifiers: Modifiers) -> Option<CreatePreview>`. It wraps
  the tool's existing private rule:
  - `RectangleTool`: `created_bounds`, which calls `create_drag_box`;
  - `EllipseTool`: `created_frame`;
  - `PolygonStarTool`: `shape_at`, which reads the tool's mode, point count and ratio.

  `live_shape()` and `pointer_up()` are rewritten to call `shape_for`. So the drag, the placement
  preview, the second click and Enter on an untouched chip all go through one function per kind.
  `CreatePreview.centre` is renamed `pivot`: during a placement it is `Some(a)` the whole time
  (criterion 28); during a drag it is `Some(a)` only with Shift, as today. `session/draw.rs` already
  draws the pivot marker from it.
- **The placement.** A new module `curvyo-ui-core/src/shape_placement.rs`, with the doc sentence "The
  state of one click-to-create placement, from its click until it is placed or cancelled." It holds:
  - one `ShapePlacement` type for all three tools: an enum `PlacementKind { Rect, Ellipse, PolyStar }`,
    no trait and no generic;
  - A;
  - `Phase::Following { pointer, modifiers }` or `Phase::Frozen(Kept)`;
  - the chip's field texts and which fields were edited;
  - the r/⌀ mode.

  It shows the fields from the live `CreatePreview` (criterion 12). It freezes on the first edit
  (criterion 14). `Kept` records:
  - the live width and height, or rx and ry, or radius;
  - the direction signs, Shift centred, Ctrl linked;
  - the polygon angle.

  `ShapePlacement::result(live, unit) -> Result<Placed, FieldError>` returns `Placed { shape, rotation }`:
  - with no field edited, it is the live shape (criterion 21);
  - otherwise it is the typed and kept values.

  `FieldError` names the field and the existing `InvalidReason` ("Enter a number", "Must be more than
  0", "Too large"). "Too large" is `transform_commit::is_sane`, the ±1e7 mm limit of `0010` criterion 11.
- **The geometry.** The typed geometry goes in a submodule, `shape_placement/resolve.rs`, as pure
  functions:
  - the box from A in the kept direction, or centred with Shift;
  - the Ctrl link;
  - the turn about A (decision 2).

  Tests first in both files, using the examples of criteria 16 to 19 and 23 as given.
- **The session.** It owns `placement: Option<ShapePlacement>`; only one creation tool is active. The
  placement code is a new `curvyo-editor-wasm/src/session/placement.rs`, and `session/shapes.rs` stays
  the create-drag:
  - **Click or drag (criterion 1).** `shape_pointer_up` measures the press-to-release distance against
    `Session::drag_threshold()` (3 px at the current zoom; 8 px for touch, see below). Under the
    threshold the tool's drag is dropped (`escape()`) and a placement starts with the release's
    modifiers. At or over the threshold, the tool's `pointer_up` runs as today.
  - **Touch.** The 8 px needs the pointer kind. `Session::set_touch_input(bool)` is cached like `held`,
    and the frontend calls it before each press. This follows the pattern of `modifiers_changed`
    (`0047` decision 3 left the rule in the frontend; this one is in Rust so criterion 1 is tested).
  - **Second press (criteria 4, 5, 22).** The second press is decided in `pointer_down`. The release
    that follows meets no tool drag and does nothing.
  - **Commit.** The commit is `document.create_primitive(shape, rotation)`. Then `select_single(id)` and
    `set_tool(Select)`, as in `shape_pointer_up`.
  - **Enter (criterion 26).** `key_table.rs` gets `KeyAction::StartPlacement`. It applies to a creation
    tool with the gate open and no placement running. A runs at `pointer_position`, or at the viewport
    centre when the pointer is outside.
- **wasm and frontend.**
  - `wasm_shape_tools.rs` (85 lines) gets the placement calls:
    - the chip view (labels, texts, placeholder, link glyph, hint line, invalid field and message);
    - `placement_edit(field, text)`;
    - `placement_toggle_diameter()`;
    - `placement_place()`;
    - `cancel_placement()`.
  - `wasm_api.rs` gets no line.
  - Frontend: a new `PlacementChip.tsx`. If the chip surface and its error line are inline in
    `MoveEntryChip.tsx`, they move into a shared component in this slice. They are not copied.

### 2. Rotation and the pivot (2026-10-12; criteria 16 (b), 18; spec Question 3)

- **The method.** `Document::create_primitive(&self, shape: Shape, rotation: Angle) -> NodeId`
  (`curvyo-document-core/src/shapes.rs`) makes one commit:
  - it writes the keys the matching `create_*` writes today;
  - it writes the `rotation` register only when it is not zero, as `convert_to_paths` does;
  - it uses the kind's existing label.

  The four `create_*` methods become one-line calls with `Angle` zero. Their 650 call sites, most of
  them tests, stay unchanged.
- **Option rejected:** a `rotation` parameter on `create_rect` and `create_ellipse`. It touches about
  650 call sites for one new caller.
- **Option rejected:** create, then rotate. That is two commits and two steps, against ADR 0002 §9 and
  criterion 6.
- **The stored shape.** The register's meaning is unchanged: a turn about the frame's own centre
  (`PrimitiveSnapshot::rotation`). The placement stores the unturned box with its centre moved to
  `c' = A + R(θ)(c − A)`, plus `rotation = θ`. This was checked against criterion 16 (b): centre
  (110, 70). It was also checked against criterion 18: centre (40, 60). Update the doc comment "a
  Select-tool rotate drag is the only writer".
- **Polygon and star.** A typed Angle goes into `StarFrame.angle` (`0010` decision 1). Their `rotation`
  register stays 0, as a create-drag writes it.

### 3. The length parser (2026-10-12; criterion 24)

- `pub fn parse_length(text: &str, unit: DisplayUnit) -> Option<Length>` goes in `display_unit_text.rs`.
  It is built on `parse_entry_number` and `Length::from_unit`.
- It accepts an optional trailing `mm`, `cm` or `in`: any letter case, with or without a space before
  it. Any other trailing text is `None`.
- The Angle field uses `parse_entry_number(text, true)`, which already accepts "°".
- The move, size, angle and skew chips are not changed (spec Question 9).
- Property tests: comma equals point, and "x in" is `x * 25.4` mm.

### 4. The key gate (2026-10-12; criteria 8, 20)

- `Session::operation_in_flight()` (`session/keys.rs`) gains `|| self.placement.is_some()`. That one
  line gates everything criterion 8 lists:
  - the tool letters, M, R, S, K, Delete and Ctrl+A;
  - the group chords;
  - the history keys (`history_action`);
  - the file chords of `0051`, because `Session::file_chord` reads the same function.

  The parameterised gate tests in `keys.rs` get a "placement" row next to "drag".
- **Space.** The canvas `onKeyDown` already returns for a form control (`isFormControl`), so Space typed
  in a chip field never reaches the pan. The chip consumes Space for r/⌀ with `preventDefault`. No Rust
  change.

### 5. Focus and modifiers (2026-10-12; criterion 3)

- The route stays the window-level listener, `ModifierTracker` →
  `applyModifiers` → `Session::modifiers_changed` (`session/select.rs`). That function also forwards to
  an unfrozen placement.
- The chip must be rendered inside the canvas container, as `MoveEntryChip` is in `Canvas.tsx`.
  Otherwise `onContainerBlur` resets the modifiers when focus moves into the chip.
- A window blur still resets the tracked modifiers to none (`0014`), so an unfrozen placement then
  follows with no modifier. It is not cancelled (criterion 7): `pointer_cancelled` acts only with the
  button down.

### 6. Cancel triggers (2026-10-12; criteria 7, 25)

- **In Rust, tested:**
  - Escape from the canvas: `cancel_drag_in_flight`'s creation-tool arm becomes
    `shape_escape() || cancel_placement()`, which returns `CancelledDrag`;
  - every tool switch: `set_tool` calls `cancel_placement()`;
  - Escape in a chip field: the chip calls `cancel_placement()` and puts focus back on the canvas.
- **In the frontend, which knows where a press lands:**
  - a secondary-button press on the canvas;
  - a press outside the canvas and the chip.

  One capture-phase `pointerdown` listener, mounted only while a placement runs, calls
  `cancel_placement()`. It does not call it for a target inside the chip or inside the active tool's
  bar (a `data-` attribute on the bar root).
- Presses on the sidebar strip of `0051` and on the Layers panel of `0050` are outside presses: they
  cancel the placement before their own action runs.

### 7. Performance (2026-10-12; criterion 32)

- **Reference desktop:** the customer's Linux machine, under Tauri, WebKitGTK and WebGL2. It gets one
  frame-rate check in the real window, as in `0009` and `0019`.
- **Gate:** an `#[ignore]` benchmark in a native release build:
  - the placement preview frame of `Session::draw_list()` with a 1024-point star takes at most 1.1
    times the create-drag frame of the same star (it is the same path);
  - one `ShapePlacement::result` per keystroke takes under 1 ms.
- The absolute 16.7 ms belongs to the real-window check. A miss there goes to the debt item "Canvas
  performance on Linux/WebKitGTK: measured once, renderer not cached", not to this slice.

### 8. Tests at risk (2026-10-12)

- **Session-level tests that press and release a creation tool closer than 3 px.** These are rewritten
  to "nothing created, a placement runs, Escape cancels it", not deleted:
  - `session/shapes.rs::a_press_without_movement_creates_nothing_and_keeps_the_selection`;
  - `tests/fix_select_created_shape.rs::cancelled_or_zero_size_creates_select_nothing`;
  - possibly `tests/acceptance_0005_reverify.rs::zero_size_and_degenerate_drags_keep_a_valid_document`
    and `tests/shape_creation_from_center.rs`. Their drags of under 0.8 mm at 100 % are now clicks.

  The implementer runs the suite once at the start and lists every failure in `plan.md`.
- **Tool-level tests stay.** `curvyo-ui-core/tests/acceptance_0003.rs::ac1_a_plain_click_with_no_movement_creates_nothing`
  and the tool unit tests keep their meaning: a tool's `pointer_up` with A equal to B is still `NoOp`,
  and the click decision is the session's.
- **Key gate tests.** `session/keys.rs` gets the placement row (decision 4) and the `StartPlacement`
  row for Enter.

### 9. Interactions with `0050`, `0051` and `0053` (2026-10-12)

- **`0050` (layers, being built).**
  - Its criterion 35, the active layer as the creation target, reaches `create_primitive` through
    `set_draw_parent`, as long as `0050` does not change the `create_*` signatures.
  - Its criterion 23, which refuses drawing on a locked or hidden active layer, must also refuse at the
    first click of a placement: no placement starts, and the same notice shows. Both the drag start and
    the placement start call one session check. Whichever slice merges second wires that check into
    the other path.
  - Shared files: `session/keys.rs`, `session/key_table.rs`, `session/mod.rs` (`set_tool`) and
    `document-core/src/shapes.rs`. These are merge conflicts, not design conflicts.
- **`0051` (merged in #104).** The file chord gate is covered by decision 4. A sidebar tab press cancels
  the placement (decision 6).
- **`0053`.** Shared files: `key_table.rs` (Enter here, Ctrl+A there) and `session/keys.rs`.
- **Order.** `CLAUDE.md` §4 runs two implementers in parallel only on features that touch different
  crates. `0050`, `0052` and `0053` all touch `curvyo-document-core`, `curvyo-ui-core`,
  `curvyo-editor-wasm` and the frontend. So this slice starts after `0050` merges and is not built in
  parallel with `0053`.

### Product owner defaults (2026-10-12)

Until the customer answers the spec's open questions:

- Enter and a second click both place.
- The first edit freezes the placement.
- The angle turns the shape about A.
- The second click places at the press.

## Flagged to the PO

Each item has a default. None is a customer question.

1. **Criterion 7, window blur.** Add: "a window blur releases Shift and Ctrl for an unfrozen placement
   (`0014`'s rule for held modifiers)". The default is decision 5. *Resolved 2026-10-12: added to
   criterion 7.*
2. **Criterion 32 and the reference desktop.** Name the gate of decision 7 in the criterion: a relative
   benchmark, plus one real-window check. A miss of the absolute 16.7 ms is recorded under the existing
   renderer-cache debt item. *Resolved 2026-10-12: criterion 32 names the gate.*
3. **Spec "Links".** Replace "`specs/0047-transform-polish/` (the move chip components)" with `0010`
   decision 3. `0047`'s `adrs.md` decides nothing about the chip parts. *Resolved 2026-10-12.*
