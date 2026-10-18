# Plan for Document template and guide paths (stage 1)

Branch `story/template-and-guide-paths`, one PR for the whole slice at the end (`CLAUDE.md` §4),
opened by the lead after the reviews. Inputs: `specification.md` (stage 1 criteria), `adrs.md`
(decisions 1 to 7), ADR 0019. Stage 2 (SVG source, criterion 21 and the SVG part of 22 and 24)
is not built.

State of `main` at 24bdc571: `CURRENT_FORMAT_VERSION` is 14; this slice takes **15** (provisional:
the `0021` build also needs a number, so the number lives in one commit, see M1).

## Affected crates/modules

- `curvyo-document-core`
  - `node_registers.rs`: key `guide` (bool, absent false), read into `NodeRegisters::guide`,
    validation (a non-bool is damage).
  - `node_read.rs`: `NodeRead::guide`, the effective flag: a leaf's own register, a linked
    clone's root's (ADR 0019 §2, criterion 34); `is_output_node` (visible and not a guide), the
    one output rule (`adrs.md` decision 2, criterion 41).
  - new `guides.rs`: `set_guide(ids, on)` (one commit `set_guide`; groups give their paths and
    shapes; clones and containers are skipped), `is_markable`.
  - new `template.rs`: `template_nodes()` (read of a source: its guides, else every visible
    path and shape, clones resolved to paths, names kept) and `load_template(nodes, anchor_ids)`
    (one commit `load_template`, nodes as guides at the top of the draw parent).
  - `replace.rs`: Boolean, Combine (`replace_with_path`) give a guide only when every operand is
    one; Split, Break apart, Cut (`replace_with_pieces`) keep the source's flag.
    `path_topology.rs`: Join of two paths keeps the flag only when both are guides.
  - `document.rs`: format version 15 and its doc comment; `document.json` writes `guide: true`
    only on guides.
  - `history/preview.rs`: `HistoricalDocument::guides` (criterion 29).
  - `tests/fixtures/`: `guide_v15.curvyo`, `guide_damaged_v15.curvyo`, the template sources
    `template_key_ring.curvyo`, `template_two_circles.curvyo`, `template_four_lines.curvyo`,
    `template_mixed.curvyo`.
- `curvyo-render-core`: new `guide_style.rs` (the marking style: one stroke run per guide,
  1 screen px, 2 / 2 px dashes restarting per outline, `GUIDE` colour); `artwork.rs`
  `build_artwork_with_guides` (guides drawn at their place in the order); `theme.rs` `GUIDE`.
- `curvyo-ui-core`: `history_names.rs` ("Guide", "Template"); `object_bounds.rs`
  `content_bounds` leaves guides out (criterion 13); `viewport.rs` `show_area` (the first view
  after a load). The button state (enabled, pressed) is `document-core`'s `GuideMarking`, so
  `select_bar.rs` is not touched.
- `curvyo-editor-wasm`
  - `session/object_cache.rs`: the set of guide ids in the read; pickable copies of guides have
    no fill (criterion 7).
  - new `session/guides.rs`: the Guide button's state and press; Template load (64 MiB limit,
    errors, notice, selection, view).
  - `session/draw.rs`, `frame.rs`: artwork with guides, history preview with guides.
  - `wasm_select_bar.rs`: `guide_state`, `toggle_guide`; `wasm_document.rs`: `load_template`.
- `frontend`: `GuideButton.tsx` in the Select bar after the Flip group; Template button in the
  File tab (third row, beside Close); `--guide` token; `pick_project` reused for the dialog.
- `docs/design-system.md`: row "Guide (`0046`)".

## Milestones (one commit each, pushed)

### M1 Guide register, commands, format (core) — criteria 1, 2, 26, 28, 30, 31, 34, 41
- [x] Test first: register read and validation; `set_guide` on paths, shapes, groups, a clone
  (skipped), one commit and undo step; unmark removes the key; merge of mark vs. move both ways;
  last writer wins.
- [x] `NodeRead::guide` with the clone rule; `is_output_node`.
- [x] Boolean/Combine/Split/Join rule (criterion 34); Duplicate keeps the flag (already, by
  `copy_map`; tested).
- [x] `document.json` key; format version 15 with doc comment; golden `guide_v15.curvyo`
  (path, circle, compound path with a hole, clone of a guide) and `guide_damaged_v15.curvyo`;
  save twice byte-identical; older fixtures open unchanged.
- [x] `HistoricalDocument::guides`; history names "Guide" and "Template".

### M2 Template loader (core) — criteria 20, 22, 23, 24 (stage 1), 33
- [x] Test first against the four fixtures: guides only when the source has some, else every
  visible path and shape; clones as resolved paths; coordinates unchanged; names kept; one
  commit, one undo step; loading twice adds twice; 5,000 nodes and 100 000 mm.
- [x] `template_nodes`, `load_template`.

### M3 Drawing, picking, Select bar, load in the session — criteria 5, 7, 8, 9, 10, 13, 19 (session part), 29, 33
- [x] render-core: marking style test first (one layer per guide, colour, width, dash period on
  screen at 1 % and 6400 %, fill and own stroke not drawn, z-order kept).
- [x] Session: guides drawn, hidden layer hides them, history preview draws them; click inside a
  closed guide does not select it, a click on the line does; marquee and Select all include it.
- [x] Fit to content ignores guides; guide-only document leaves Fit as "no objects". (There is
  no "Zoom to document" command yet; the first view after a load zooms out to the page and the
  guides, `Viewport::show_area`.)
- [x] Guide button state (disabled, pressed, mixed) and press (one step "Guide", selection
  kept).
- [x] `load_template(bytes)`: 64 MiB, not a project, damaged, too new; nodes selected, notice
  "Template loaded.", view shows page and guides.

### M4 Frontend and docs — criteria 9, 10, 14, 19, 37
- [x] Guide button (Lucide `SquareDashed`, 28 px, tooltip and name "Guide", `aria-pressed`,
  `aria-disabled`, focus back to canvas after a mouse press).
- [x] Template button (File tab, beside Close, tooltip "Template"), desktop `pick_project`,
  browser picker; errors through the existing error dialog. The loader lives in
  `document-core/src/template.rs` as `Document::template_nodes` and `Document::load_template`
  (`adrs.md` decision 4 names it `copy_nodes_as_guides`; the split keeps the anchor-id minting
  with the session).
- [x] `--guide` token, `docs/design-system.md` row. Measured: `#15803D` has 2.5:1 on
  `--pasteboard-bg`, so the token is `#166534` (3.6:1 there, 5.8:1 on `--canvas-bg`).
- [x] Frontend tests (`node --test`, the repo's runner) for the words and outcomes of the two
  buttons; the 0051 File tab tests list the sixth button.

## Validation

Full gate of `CLAUDE.md` §7 plus clippy for wasm32 on `editor-wasm`, `render-core`,
`document-core`, and the frontend `tsc`, `oxlint`, `vitest`, `build`.
