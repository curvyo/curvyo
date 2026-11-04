# Plan for manufacturing roles

Implementer, 2026-11-04. Branch `story/0027-manufacturing-roles`. ADR 0041 and `adrs.md` notes 1
to 8, built for Frage 140 A, 141 A and 142 A. UX notes items 2 to 5 as the lead accepted them
(notice above the rows, white casing under Cut lines drawn above Engrave, 2 px Engrave lines for
all-open leaves, read-only lock glyph). No new crate, no new dependency, no `format_version`
bump (stays 36).

## Output page ground (UX notes item 7, confirmed in code)

The output page is drawn by `build_page_areas` with `Session::shown_background()`, that is the
**drawing's document background** (`frame.rs`), not a fixed `--canvas-bg`. For a new document
this is `DocumentBackground::DEFAULT`, `#E8E8EB` = `--canvas-bg`, so the contrast figures of UX
notes §5 hold by default. A maker who gives the drawing another background (or None, the
checkerboard) gets that ground on the output page too; the role tokens are not re-measured for
it. Noted for the UX review; nothing changed here.

## Affected crates/modules

- **`curvyo-document-core`**: `NodeId::to_register_text` / `NodeId::from_register_text` (the
  `peer:counter` text of `page.<id>.owner`); `Document::set_process_register(id, key, value,
  label)` in `manufacturing.rs` (one step, no-op when unchanged, refused outside
  `x.<process>.`); the open check refuses a container value under an `x.*` key of a
  manufacturing; golden `laser_roles_v36.curvyo`.
- **`curvyo-laser-core`**: new module `roles` (`Role`, `RoleTarget`, `RoleRead`, key text,
  read from `process_values`, the output rule of ADR 0041 §6, the notice texts, commit label
  `set_role`); manifest gets `Block::Layers { label: "Layers" }` after Machine.
- **`curvyo-extension-core`**: `Block::Layers { label }`, `CHANGELOG.md`.
- **`curvyo-ui-core`**: history name `set_role` → "Set role"; document-level label.
- **`curvyo-render-core`**: module `role_look` (`Look::{Guide, Muted, Outline, Area}`) and
  `build_artwork_with_looks`; `build_artwork_with_guides` becomes a wrapper; `ImageQuad::grey`;
  theme tokens `ROLE_CUT`, `ROLE_ENGRAVE`, `ROLE_IGNORE` mirrored in `index.css` (test).
- **`curvyo-editor-wasm`**: `session/manufacturing/roles.rs` (leaf-to-layer map cached per read,
  Layers block view, `set_layer_role`, Role view switch), the Laser frame draws looks, the image
  pipeline's grey flag, wasm bindings; budget tests in `tests/manufacturing_budget.rs`.
- **`frontend`**: `LayersBlock.tsx` (rows, role control with roving focus, Role view switch,
  legend, notice, windowing over 60 rows), `lib/roleRowKeys.ts` + test, `useWorkbenches` and
  registry types, tokens in `index.css`.

## Tasks

- [x] 1. document-core: register text of a node id, `set_process_register` with undo, no-op and
  namespace refusal, open check for container values, merge per layer (AC 4, 5, 17, 18, 20).
- [x] 2. laser-core `roles`: parse to Known/Unknown/Unassigned, key text, output rule, notice
  texts within 70 characters (AC 3, 5, 11, 13, 14, 15, 18).
- [x] 3. extension-core `Block::Layers`, laser manifest, changelog; history name (AC 1, 4).
- [x] 4. render-core looks: Cut 1 px cased outline, Engrave area by fill rule or 2 px line for
  all-open leaves, Muted dashed 4/3, images grey under Area and frame otherwise, Area then Muted
  then Outline order, style and opacity ignored (AC 6, 7, 8).
- [x] 5. editor-wasm session: leaf-to-layer map, looks in the Laser frame (hidden absent, guides
  and dimensions absent, clones by their own layer), Role view switch, Layers block view JSON,
  `set_layer_role` one step, drawing never written (AC 1 to 4, 6 to 17, 20, 22, 23).
- [x] 6. golden `laser_roles_v36.curvyo` and the round trip with stale and unknown entries; old
  goldens open with no role (AC 19).
- [x] 7. budget tests `manufacturing_budget.rs` (AC 21).
- [x] 8. GPU grey flag for engraved images (AC 8 f).
- [x] 9. frontend Layers block, role control, roving focus keys (tested helper), Role view
  switch, legend, notice, windowing (AC 1, 2, 6, 14, 15, 16, 18).
- [x] 10. browser check end to end, screenshots; docs; full gate (all).

## Validation

- Unit tests per module in laser-core and render-core; document-core integration test
  `tests/laser_roles.rs` with the golden (`CURVYO_WRITE_FIXTURES=1` rewrites it).
- Session tests in `session/manufacturing/roles_tests.rs` for the criteria a session can show.
- CPU-time budgets in `curvyo-editor-wasm/tests/manufacturing_budget.rs` (release in CI).
- `frontend/tests/roleRowKeys.test.ts` for the key map and the row window.
- Browser run of the demo build: layers, roles, Role view, hidden layer, undo, reload.
- Full gate of `CLAUDE.md` §7 once at the end.

## Notes for the lead

- `Look` has four arms instead of `adrs.md` note 3's three: the guide marking (2/2 dash) and the
  muted Ignore marking (4/3 dash, UX notes §5) differ in dash, so `Marking(color)` would need a
  dash parameter anyway. (Proposal)
- The draw order "Engrave, then Ignore, then Cut" holds within one placement; two overlapping
  placements stack as placements.
