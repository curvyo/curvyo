# ADRs for "Transform polish: silent conversion, click-and-double-click sliders, a visible reference box for the typed move"

Written by the product owner at the lead's request to list what the feature touches; the architect replaced the "to decide" lines with the decisions below after the build review. **No new ADR, no new crate, no new dependency, no trait, no generic, and no `format_version` change are expected.** A path is an existing object kind; the box, the marks and the reference are view state. Reference state: `main` plus #80 (`story/multi-object-transform`); the picker comes from #78.

## Depends on

- [ADR 0001 §1, §3, §5](../../docs/adr/0001-ui-framework-and-canvas-rendering.md): what a skew or stretch converts, the reference geometry (the nine points, the hit radius rule) and the slider value mapping stay in Rust (`curvyo-ui-core`); the box and marks are drawn by `render-core` from the decoration input; the DOM chip and the sliders only hold text, focus and gestures.
- [ADR 0002 §5](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): the per-node affine is still not built, so a skew has nowhere to live on a primitive and the conversion to a path is the consequence (P3, a stored matrix, stays deferred). [§9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): the conversion and the transform are one commit.
- [ADR 0009 §2, §3](../../docs/adr/0009-concurrent-editing-semantics.md): the box, the marks and the chosen reference are ephemeral and never written (criterion 49). A peer's concurrent edit of a primitive that this commit converts is the same case as a peer edit against "Object to path" today.
- [ADR 0004 §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md): no new key and no new meaning of a key, so no bump (criterion 14).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) (accepted 2026-10-10) and [`0020` adrs.md](../0020-undo-redo/adrs.md): a skew or stretch with a conversion is one step with the gesture's label (criterion 15).
- [`0019` adrs.md](../0019-multi-object-transform/adrs.md): decision 1 (`group_box.rs`, `group_transform.rs`, `Document::transform_objects`) is extended, and the sentence "Kinds are preserved; no conversion" of decision 2 is **superseded**: skew and non-uniform stretch of a uniform-only selection reached no per-object function before; now they do.
- [`0010` adrs.md](../0010-edit-interaction-polish/adrs.md) decision 3: the DOM chip holds the text, the mode and the focus; the reading of the numbers is `curvyo-ui-core::move_entry`. The reference is an extra input of that one function.
- [`0008`](../0008-object-transform-refinements/specification.md) decision P1 (customer, 2026-10-06; no ADR was written for it) is reversed by the customer's request of 2026-10-10.
- [`0044` adrs.md](../0044-editing-quick-wins/adrs.md): the nudge uses the same bounds (`object_outline_bounds`, `GroupSelection`); it does not change.

## Feature-local decisions

- 2026-10-10 (customer): a lone polygon or star that is stretched converts silently, like one in a multi-selection (spec question 2 B, criterion 12). The reference of the typed move has nine points; the Reference control is a 3 x 3 radio grid (question 8 B, criteria 39 to 41). Neither changes the file format: the conversion writes an existing kind, the reference is view state.
- 2026-10-10 (customer): 0047 and 0020 may be built in either order.

Decisions (architect, 2026-10-11, recorded after the build review; as built on `story/transform-polish`):

1. **Where the conversion lives.** No new writer: `Document::transform_objects` (from `0019`) already takes a path result for a primitive in its one commit. A single object reaches it through `commit_resize`, which routes a path result for a live primitive to `commit_group`. Label `transform_objects`, never `convert_to_paths`.
2. **"Needs conversion".** `conversion::converts_on_stretch` is the one predicate, and `conversion::converts_for` the one rule by handle, behind `converted_for_object` (one object) and `converted_for_handle` (a group): skew converts every primitive; resize converts polygon and star, and in a group the non-aligned rectangle and ellipse. `is_uniform_only` is gone, and `HandleSpec` has no kind branches and no `skew` option any more: skew handles are shown for every kind.
3. **The reference input.** `ReferencePoint` (nine values, `reference_point.rs`) is view state of `MoveEntry`, reset to `Centre` on every open. `resolve_at` / `commit_at` take the point a mark click names. Deviation: the mark press is not a `PressTarget` in `ui-core`. The frontend asks `Session::move_mark_at` before `pointer_down` and applies the 3 px / 8 px rule itself. Accepted for this slice, with a debt item in `docs/technical-debt.md`.
4. **Box and marks.** A separate `render-core::ReferenceBoxInput`, not a `DecorationInput` item. The box is drawn below the member boxes, not above them as the UX notes say (UX review decides). The move drag reads the start bounds once, at the press (`MoveDrag::new`); a frame adds an offset.
5. **Sliders.** One pure classifier (`frontend/src/lib/sliderGesture.ts`) and one hook (`useSliderPointer`, which also takes Escape during a press). S1 and S2 became `role="slider"` elements; the native range inputs are gone. Fixed 500 ms double-click interval.
6. **Module size.** As decided: the reference code is in new modules; `move_entry.rs` is 284 non-test lines.
7. **Collisions.** As stated; expect conflicts with `story/undo-redo` in `keys.rs`, `draw.rs`, `session/mod.rs` and `useEditorSession.ts`.

History (`0020`, merged 2026-10-11): a skew or stretch that converts is one step named after the gesture, "Skew" or "Resize" (the batch labels `skew_path`, `resize_path`, `transform_skew`, `transform_resize` of the table in `curvyo-ui-core::history_names`; no new label). A typed move with a reference, and a click on a mark, are one "Move" step (`translate_objects`); the mark click goes through `Session::recording` like the chip's Enter. Tests: `curvyo-editor-wasm/tests/transform_polish_undo.rs` (undo and redo are byte-equal).

## Flagged to the PO

Nothing yet; the architect adds here.
