# Plan for mirror and negative scale

One PR on `story/mirror-and-negative-scale`, built as four milestones (customer rule of
2026-10-09: one PR per slice, opened when the customer can try it end to end). Each
milestone is one commit with its tests and is pushed to the branch; the PR opens after M4
with the full gate green.

State found on `main` (2f2859b): `0049-linked-clone` is merged, so criterion 25 applies
(a clone's matrix takes the signed map G through `clone_gesture::map_clone` and
`settle_clone_matrices`). `GroupMap::Scale` already carries signed factors, and
`PathSnapshot::scaled_along` already maps anchors and handle vectors with any sign. The
clamps at 0 sit in `transform_math::resized_extent`, its Ctrl branch and
`stroke_or_radius_factor`; a single object's rectangle and ellipse are rebuilt from the
resized local box (which is why the box must be normalised and the mirror applied after).

## Affected crates/modules

- `curvyo-ui-core`:
  - `transform_math.rs`: signed `resize_local_box` (no `.max(0.0)`), normalised `min <= max`,
    Ctrl takes the magnitude of the dominant axis and the sign of each free factor, zero
    recovery (extent under 0.0005 mm becomes 0.01 mm with the prior sign), `ExtentSigns`
    (the sign of the last frame whose extent was not 0) and `ZeroRecovery` (which axes were
    set to 0.01 mm); `stroke_or_radius_factor` is `sqrt(|sx*sy|)`.
  - new `mirror.rs` (one sentence: the reflection of one object about the lines through a
    point along a frame's axes): path anchors and handles, path `rotation` 2φ − θ, rectangle
    centre, `rotation` and radii at their physical corners, ellipse, polygon and star by the
    first-vertex angle (the `rotation` register is kept).
  - `group_transform.rs`: `is_stretch` on magnitudes; `scale_object` scales by the
    magnitudes and then mirrors per negative factor.
  - `transform_primitive.rs` / `transform_drag.rs`: the single object resize scales by the
    magnitudes about the anchor and mirrors along its own box axes; drags keep
    `ExtentSigns`, updated in `pointer_moved`.
  - `select_tool*.rs`: `pointer_up` returns the `ZeroRecovery` of a committed resize; live
    signs for the readout; the flip entry point.
  - new `flip.rs` (one sentence: the signed scale map of a flip and its no-op test): the map
    about the TBB centre, the result, the no-op test, the commit with labels
    `flip_horizontal` / `flip_vertical`.
  - `transform_entry.rs`, `group_entry.rs`: any finite W or H, `InvalidReason::NotPositive`
    deleted, typed 0 becomes +0.01 mm, linked fields carry the sign, "Too large" on the
    magnitude.
  - `history_names.rs`: "Flip horizontally", "Flip vertically".
  - a text module for the signed size readout, the mirror line and the notices.
- `curvyo-document-core`: no change expected (the commits exist: `transform_objects`
  writes primitives, paths, `rotation` and clone matrices; `resize_rect` etc. for one object).
  If a change turns out to be needed, the work stops for an architect review.
- `curvyo-editor-wasm`: flip exports, keys H and V through the gate of `0010`, the notice
  (`take_transform_notice`), the signed readout and its mirror line, the chip's note line,
  the wasm move entry reason table without `NotPositive`.
- `frontend`: the Flip group in `SelectToolbar.tsx` (two icon buttons, custom glyphs,
  `aria-disabled` with nothing selected, tooltips), the notices, the readout second line, the
  chip note line and field description, "Must be above 0" removed.
- `docs/design-system.md`: the rows of criterion 30; the shortcut table (H, V).

## Tasks

### Milestone 1: signed resize and the mirror rule (`curvyo-ui-core`)

- [x] 1.1 Test first, `transform_math`: free corner, edge, Ctrl, Shift, Ctrl+Shift across
  zero; normalised box; zero recovery with prior signs; stroke factor of negative factors
  (AC 6, 7, 8, 9, 13, 15, 23).
- [x] 1.2 Test first, `mirror.rs`: path (anchors, handles, kinds, order, compound holes,
  `rotation`), rectangle (four radii at physical corners, rotation −θ, 90° case), ellipse,
  circle, polygon, star (AC 18 to 22).
- [x] 1.3 Group map: magnitudes then mirror; `is_stretch` on magnitudes; clones keep the
  signed G (AC 12, 18, 21, 24, 25).
- [x] 1.4 Single object: magnitudes about the anchor, mirror along its own axes; rotated
  rectangle example of AC 12; handles by position after the result (AC 6, 7, 11, 12).
- [x] 1.5 Drag state: `ExtentSigns` per drag, updated per frame; preview and release use the
  same signs; `pointer_up` reports `ZeroRecovery` (AC 10, 15, 28 g h i j).
- [x] 1.6 Superseded tests updated to the new rule (`0005` criterion 13, `0019` criterion 22
  first sentence, `0005` stroke factor clamp): named in the PR.

### Milestone 2: the flip command and typed values (`curvyo-ui-core`)

- [x] 2.1 Test first, `flip.rs`: map about the TBB centre, AC 2 example (two rectangles), AC 3
  no-op (circle, horizontal line, symmetric path), AC 22 triangle example, AC 25 clone
  example; one commit with the History names (AC 2, 3, 22, 25, 29).
- [x] 2.2 Typed W/H: negative and zero, linked sign, prefill positive, "Too large" on the
  magnitude; AC 14 and 17 examples, single and group entry (AC 14, 15, 17).
- [x] 2.3 Arithmetic properties of AC 28 a to f over every kind, at ±1e5 mm (AC 28).
- [x] 2.4 Text: signed size readout (U+2212, two decimals under 0.1 mm), mirror line, notices
  of AC 3 and 15 (AC 10, 15).

### Milestone 3: the Select bar, keys, readout and chip (`curvyo-editor-wasm`, `frontend`)

- [ ] 3.1 wasm: `flip_horizontal`, `flip_vertical`, `flip_enabled`, the notice and the
  announcement; a flip closes an open chip without writing (AC 2, 3, 5).
- [ ] 3.2 Keys H and V in the Select tool through the `0010` gate; hint "Select an object
  first" with nothing selected; nothing in other tools (AC 4).
- [ ] 3.3 Readout: signed sizes and the mirror line for one object and a group (AC 10).
- [ ] 3.4 Chip: no "Must be above 0", note line for a negative value, field description;
  notice after Enter with a typed 0 and after a release on 0 (AC 14, 15).
- [ ] 3.5 Frontend Flip group: placement after the switches, glyphs, `aria-disabled`,
  tooltips, announcements, no-op notice under the button (AC 1, 2, 3).

### Milestone 4: file, budget and docs

- [ ] 4.1 File round trip: flipped, scaled-over-zero and typed-negative objects save and reopen
  equal, no negative stored length, no `format_version` change (AC 26).
- [ ] 4.2 `#[ignore]` benchmarks of AC 27 (group box, preview step, flip and release commit
  against a move); WASM times measured for the PR description (AC 27).
- [ ] 4.3 `docs/design-system.md` rows of AC 30 and the shortcut table; spec status line.

## Validation

- Unit tests next to each module, integration tests in `curvyo-ui-core/tests/` and
  `curvyo-editor-wasm/tests/` for every numbered example of the specification.
- Property tests (proptest, already in the workspace) for AC 28 a to d over random objects.
- Full gate of `CLAUDE.md` §7 before the push of M4; manual check in the app (`npm run tauri
  dev` or the browser build): drag through zero in every mode, flip buttons, H and V, typed
  negative and 0.

## Interaction with 0057 clone-overrides (PR #138, not merged)

0057 adds own-placement clones that every transform writer must hold back
(`clone_matrices_after` takes a hold set; every writer is listed in `every_writer` in
`tests/clone_held_back.rs`). The flip is a new writer, built here against `main` as it is
today: it writes through `group_transform::commit_group`, as a drag does. Whichever of
0056 and 0057 merges second wires the flip into the hold rule and adds it to `every_writer`.
