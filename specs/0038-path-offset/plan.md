# Plan for Path offset: Outset and Inset by a typed distance, with a live preview

One branch `story/path-offset`, one PR, five milestones (commits on the branch, each green). No
`format_version` change, no new crate, no new dependency, no `Cargo.toml` change. Acceptance criteria
(AC) numbers are those of `specification.md`. Built exactly as specified; `0054` (combine open paths)
amends Outset/Inset later and is not anticipated here.

## Affected crates/modules

- `curvyo-geometry-core`: new `src/offset.rs` (the kernel: `offset`, `OffsetJoin`, `OffsetCap`,
  `OffsetError`), `flatten.rs` (an open-polyline flattener next to `flatten_closed`), `lib.rs`
  (exports), `boolean.rs` (`grid_paths` and the `BooleanResult` constructor shared with `crate`),
  new `tests/offset.rs` (AC 15 to 20, properties), `tests/offset_performance.rs` (AC 3 budget).
- `curvyo-document-core`: `replace.rs`, one additive command `add_paths_above` (AC 10, 13).
- `curvyo-ui-core`: new `offset_entry.rs` (entry values, ranges, session memory, availability, the plan
  per operand with refusals and fresh anchor ids), `lib.rs` exports.
- `curvyo-render-core`: the blue hollow preview outlines, reusing the live-preview stroke (no new layer
  kind if the existing preview path fits).
- `curvyo-editor-wasm`: new `session/offset.rs` (open/close, preview cache, apply, Escape cascade,
  closing on tool/selection change and empty-canvas press), new `wasm_offset.rs`, small additive hooks
  in `session/mod.rs`, `session/keys.rs`, the render frame.
- `frontend/`: Offset text button last in the Select bar, the Offset section at the top of the Style
  tab, caption, notices and refusal texts (new `lib/offsetText.ts`), wiring in `useEditorSession.ts`.

## Milestones and tasks

### Milestone 1: the kernel (AC 6, 7, 8, 9, 11, 12, 15 to 20, 3)

- [x] 1.1 Tests first (`tests/offset.rs`): square Miter/Round/Bevel/Inset/empty/9.99 (AC 15), ring (AC 16),
  open line with the three caps and refused inset (AC 17), L-shape reflex corner (AC 18), miter limit
  on a square and a 20-degree triangle (AC 19), circle polyline (AC 20), self-crossing operand (AC 9),
  output rules: corner-only, min spacing, no collinear vertex, canonical winding (AC 11), determinism
  under operand outline order (AC 12).
- [x] 1.2 `offset.rs`: closed pipeline `grid_paths` → `normalize` → `IntOutlineOffset::outline` →
  `cleanup` → `canonical_millimetres`; open pipeline flatten → snap → `IntStrokeOffset::stroke(2|d|)`;
  integer API, `MathMode::Integer`, distance rounded to the grid; joins and caps mapped as in `adrs.md`;
  `ArcOptions.max_step` from `sqrt` only (deterministic); `miter_min_turn = min(5°, sqrt(0.04/|d|))`;
  `validate_outline`/`validate_stroke` → out of range; empty → `OffsetError::Empty` (AC 6 to 9, 11, 12).
- [x] 1.3 Property test: random grid polygons, outset contains the operand, inset lies inside it, area
  monotonic in `d` (AC 6).
- [x] 1.4 Budget test: 200 curved nodes within 50 ms in release (AC 3), in `boolean_performance` (the
  `boolean-budgets` CI job). Measured: about 20 ms at 2 mm and 4 ms at -1 mm on the gear-like operand;
  a 5 mm outset that fills its 4 mm pockets takes about 55 ms (open point for the lead).
- [x] 1.5 Golden results (`tests/offset_golden.rs`, `fixtures/offset/cases.golden`), compared natively
  and in the wasm build in CI (AC 12).
- Note: slivers narrower on average than the flattening tolerance are dropped after the offset, so a
  ring inset by half its width vanishes as AC 16 asks (two flattened circles meeting leave chords
  a few micrometres apart).

### Milestone 2: the document write (AC 10, 13)

- [ ] 2.1 Test first, then `Document::add_paths_above(pieces, label)`: each result directly above its
  operand, complete style copied, rotation 0, nothing deleted, one commit, anchor-id check, refusal
  changes nothing (AC 10, 13).

### Milestone 3: the plan in `curvyo-ui-core` (AC 1, 5, 7, 8, 10, 12, 14)

- [ ] 3.1 Tests first, then `offset_entry.rs`: `OffsetSettings` (distance with typed range ±1000 and drag
  range ±50, default 1 mm, join, cap; session memory), `offset_availability` (operands, open paths
  present → Cap shown, groups only → not offered) (AC 1).
- [ ] 3.2 `plan_offset`: operands in stacking order, one kernel call per operand, refusals Groups,
  OpenInset (offenders), OutOfRange, NoArea, AllVanished(distance), partial vanish count, Distance 0 →
  nothing; corner anchors with fresh ids (AC 5, 7, 8, 10, 12, 14).

### Milestone 4: session, wasm API and preview (AC 2, 3, 4, 5, 13)

- [ ] 4.1 Tests first, then `session/offset.rs`: open/close, set distance/join/cap → recompute preview
  (cached plan), apply writes the cached plan in one `offset_path` commit and selects the results,
  closing on Escape (first step of the cascade), Cancel, tool change, selection change and a press on
  empty canvas, refusal marks (AC 2, 4, 5, 13).
- [ ] 4.2 Preview outlines drawn blue (`--accent`) 1.5 px hollow over the unchanged originals, never
  stored (AC 2).
- [ ] 4.3 `wasm_offset.rs`: availability, open, set values, preview status (caption code), apply outcome
  (codes and counts; sentences are the host's).

### Milestone 5: the UI (AC 1, 2, 4, 5, 7, 8, 13, 14)

- [ ] 5.1 Select bar: "Offset" text button last, shown while an operand is selected, `aria-expanded` (AC 1).
- [ ] 5.2 Offset section at the top of the Style tab: Distance value field, caption, Join and Cap
  radiogroups, Apply and Cancel; two-step Enter; preview after commit/drag frame (AC 1, 2, 4, 5).
- [ ] 5.3 Notices and refusal sentences in `lib/offsetText.ts` with unit tests (AC 7, 8, 13, 14).
- [ ] 5.4 Full gate (`CLAUDE.md` §7), frontend tests and typecheck; spec `PR:` link.

## Validation

- Unit tests in `offset.rs` and acceptance tests for AC 15 to 20 in `tests/offset.rs`, with the areas and
  tolerances of the spec.
- Property test (proptest): containment and area monotonicity on random grid polygons.
- Determinism (AC 12): integer API and `MathMode::Integer`, angles computed with `sqrt` and basic
  arithmetic only; a test compares results for permuted outline order.
- Budget (AC 3): release-only assertion, 200 nodes under 50 ms.
- Manual: `cd frontend && npm run dev`, select a rectangle, press Offset, drag Distance, Apply.
