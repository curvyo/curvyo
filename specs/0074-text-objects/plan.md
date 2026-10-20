# Plan for Text objects

Implementer, 2026-10-20, branch `story/text-objects` from `main` at `e189889`. Built under
[ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) (accepted 2026-10-20,
wasm budget 1.3 MiB, raised to 1.4 MiB by the customer later on 2026-10-20). One PR for the slice; the milestones below are commits on the branch,
squashed to at most five self-contained commits before the PR.

## Affected crates/modules

- **`curvyo-text-core` (new, ADR 0020 §2):** `face` (parse and validate a face from bytes, metrics,
  cap height), `face_store` (`FaceStore`, `FaceKey`, content hash), `outline` (glyph outline to
  `kurbo::BezPath` in em units, exact quadratic-to-cubic elevation, point limit, per-face cache),
  `shape` (harfrust shaping of one line, clusters), `layout` (lines, line height, line box, caret
  boundaries, hit to caret), `limits` (named hostile-input limits). Depends on `kurbo`, `harfrust
  =0.14.0`, `skrifa =0.48.0`, `sha2`, `thiserror`. No workspace crate.
- **`curvyo-document-core`:** `text_model` (`TextSnapshot`, `FontRef`, `MAX_TEXT_CHARS`, size
  range), `text_codec` (registers `text_string` as `LoroText`, `text_font`, `text_size`,
  `text_placement`; refusal of damaged values), `text_write` (create, edit string by diff, set
  font, set size), the transform rule in `transform_objects` (uniform scale into the size),
  `ObjectSnapshot::Text` and an arm in every exhaustive match, `CURRENT_FORMAT_VERSION` 16 to 17,
  golden `tests/fixtures/text_v17.curvyo`.
- **`curvyo-storage-io`:** module `fonts` (`fontdb` listing of the Curvyo folder and the system
  fonts, skipped-file count, bytes by id). **`curvyo-app`:** Tauri commands `font_list`,
  `system_font_list`, `font_bytes` (`tauri::ipc::Response`).
- **`frontend/public/fonts/`:** Source Sans 3, Source Serif 4, Great Vibes (unmodified files,
  `OFL.txt`, `font.toml`), plus `index.json` for the browser list. Licence test in
  `curvyo-text-core/tests/included_fonts.rs`.
- **`curvyo-ui-core`:** `text_edit` (edit session: string, caret, selection, preedit, in-session
  undo, commit by diff), `text_tool` (press, click on a text, Enter entry, exit rules), `text_convert`
  (Object to path: group of one compound path per glyph; refusal for missing faces), `font_match`
  (rule a, b, c of ADR 0020 §1), text arms in hit test, bounds, conversion (path commands convert
  texts first), layers rows, history names.
- **`curvyo-render-core`:** drawing a text through its outlines (fill and stroke as a path), the
  caret, selection highlight and preedit underline, the line box.
- **`curvyo-editor-wasm`:** `wasm_text` (face bytes in once per face, text tool messages, Text bar
  state, Fonts tab state), the key table (`T`, Enter, Shift+Ctrl+T), session wiring.
- **`frontend`:** Text tool button, `TextBar` (Font button, Size field), the Select bar Text group,
  `FontsTab` (blocks, search, virtualised tree), the hidden textarea sink, the missing-font tags.

## Milestones and tasks

### Milestone 0: docs
- [x] 0.1 UX notes into the spec, criteria 1, 7, 10, 11, 15, 19, 29 amended, design-system rows,
  status Ready (all criteria; docs only)

### Milestone 1: `curvyo-text-core`
- [x] 1.1 Crate skeleton, workspace member, `deny.toml` exception, pinned dependencies; wasm32 CI
  picks it up by the `-core` suffix (no list to edit) (AC 33)
- [x] 1.2 `FaceStore::add` and `Face`: parse from bytes, `MAX_FACE_BYTES`, content hash, index in a
  collection; variable fonts at the default instance (AC 21, 32, 33)
- [x] 1.3 Cap height (measured H, else OS/2, else 0.7 em) and line metrics (hhea, or typo with
  `USE_TYPO_METRICS`) (AC 4, 12)
- [x] 1.4 Shaping with `kern`, `liga`, `calt`; `.notdef` kept (AC 5, 6)
- [x] 1.5 Outlines: exact quad-to-cubic, CFF cubics kept, `MAX_GLYPH_POINTS`, per-face cache (AC 23, 32)
- [x] 1.6 Layout: lines, left alignment, line box, cluster boundaries, caret hit (AC 3, 4, 10, 25)
- [x] 1.7 Hostile-input property test over mutated fixtures in `tests/fixtures/fonts/`, a cargo-fuzz
  target in `curvyo-text-core/fuzz/` (not a workspace member) (AC 32)

### Milestone 2: document model (stop for the architect review afterwards)
- [x] 2.1 `TextSnapshot`, `FontRef`, `ObjectSnapshot::Text`, every exhaustive match (AC 28)
- [x] 2.2 Codec and writes: create, string by Loro text diff, font, size; `MAX_TEXT_CHARS` (AC 13, 28, 31)
- [x] 2.3 Transform rule: uniform scale into the size (with the clamp remainder), else into the
  placement (AC 26)
- [x] 2.4 Layers name from the first 24 characters (AC 27)
- [x] 2.5 `format_version` 17, golden `text_v17.curvyo` (one line, two lines, rotated, stretched,
  missing face), refusal of damaged registers (AC 28)

### Milestone 3: fonts on the host and the included fonts
- [x] 3.1 `curvyo-storage-io::fonts`: Curvyo folder (created if missing, non-recursive, TTC
  faces, skipped count), system fonts off the UI thread (AC 16, 19, 20, 21)
- [x] 3.2 Tauri commands and the frontend platform layer (AC 19, 20, 33)
- [x] 3.3 (moved into milestone 1: the engine tests use them as fixtures) Included fonts with licence texts and `font.toml`; licence test (AC 2 of the customer
  decisions, ADR 0020 §4)

### Milestone 4: ui-core and render-core
- [x] 4.1 Edit session: messages, caret by clusters, selection, preedit, in-session undo, limit,
  tabs to spaces, one commit by diff, empty means no node (AC 3, 7, 8, 9, 31)
- [x] 4.2 Text tool: click, drag as click, click on a text, Select double-click, Enter entry, exits,
  locked layer (AC 2, 7, 10)
- [x] 4.3 Font and Size changes, next-text state, first-use default (AC 12, 13, 14)
- [x] 4.4 Object to path and path commands converting texts; refusal for missing faces (AC 22, 24, 29)
- [x] 4.5 Font matching and missing faces (AC 29, 30)
- [x] 4.6 Hit test, selection box, marquee and lasso, layers rows (AC 25, 27)
- [x] 4.7 render-core: text outlines, caret, selection, preedit, line box (AC 3)

### Milestone 5: editor-wasm and frontend
- [x] 5.1 Face bytes once per face per session; Text tool and edit-session bindings (AC 3, 33)
- [x] 5.2 Text bar and Select bar Text group (AC 11, 12, 13)
- [x] 5.3 Fonts tab: blocks, search, tree keyboard, current mark, missing block (AC 15 to 18, 29)
- [x] 5.4 Hidden textarea sink: keys, IME, clipboard; canvas caret (AC 3)
- [x] 5.5 Tool rail slot, `T`, Shift+Ctrl+T, missing-font tag (AC 1, 15, 29)

## Validation

- One focused test per acceptance criterion (lean tests, customer rule); the core criteria (4, 5,
  6, 12, 23, 26, 28, 31, 32) are tested in `curvyo-text-core` and `curvyo-document-core`.
- Golden file `curvyo-document-core/tests/fixtures/text_v17.curvyo` for the format.
- Fixture fonts: the included fonts in `frontend/public/fonts/` (Source Serif 4 Regular TTF for
  kerning and the fi ligature, Source Sans 3 Regular OTF for CFF); hostile files are made from them
  inside the tests (truncations, offsets past the end, self-referencing composites, mutations).
- Hostile input: a property test in the gate; a 600 s soak run (`hostile.rs`, ignored test)
  recorded in the PR.
- Speed (AC 3, 17, 20): budgets measured on the dev VM `impa` (8 vCPU Xeon), release build.
- Wasm size: release editor wasm before and after, against the budget (1.4 MiB).

## Dated notes

- 2026-10-20: the Pencil (`0032`) is on hold, so the Text tool takes the next free slot (7) of
  the Tools card (lead).
- 2026-10-20: criterion 19 names `app_data_dir()/fonts/` (ADR 0020 §3), not the `directories`
  crate, as `adrs.md` decision 1 asked.
- 2026-10-20: `MAX_TEXT_CHARS` lives in `curvyo-document-core` (it is the register's limit);
  `curvyo-text-core` has no workspace dependency, so its shaping cap `MAX_SHAPE_CHARS` is its own
  constant and a ui-core test asserts the two are equal.
- 2026-10-20: the content hash (SHA-256 of the file bytes) is computed by `FaceStore::add` with
  `sha2` (already in `Cargo.lock` through Tauri; MIT OR Apache-2.0), so the browser build hashes
  the included fonts it fetches.
- 2026-10-20: criterion 12's test reads "the height of the capital H above the baseline" (the
  Terms definition), not the bounding-box height: Great Vibes' H has a swash 4.3 mm below the
  baseline at 20 mm, so its box is 24.3 mm high while its cap height is exactly 20 mm. For every
  face whose H sits on the baseline the two are the same.
- 2026-10-20: no `cargo fuzz` target: it needs a nightly toolchain and the workspace pins stable
  1.99.0. The time-boxed run of ADR 0020 §2 is an ignored stable test
  (`curvyo-text-core/tests/hostile.rs`, `ac32_soak_mutations`) built on the spike's harness, with
  table-targeted mutations and a 2 s per-case hang check; the gate runs the 48-case property test.
- 2026-10-20: no font files under `tests/fixtures/fonts/`: a truncated or mutated copy of an OFL
  font with a Reserved Font Name is a modified font, so the hostile files are made in memory by
  the tests from the shipped ones.
- 2026-10-20 (milestone 2): the read model `TextSnapshot` carries a derived `outline:
  Option<Arc<TextOutline>>` (the glyphs as one compound path in document mm and the four corners
  of the line box), `#[serde(skip)]`, `None` as read, filled by the editor session through
  `curvyo-text-core` (milestone 4). It is mapped with the text by `mapped_by`, so previews of a
  move, rotate, skew or stretch need no new layout. `ObjectSnapshot::drawn_path()` returns a
  path's, a clone's or a text's drawn outline, so drawing, Boolean, Combine, Flip and the refusal
  outline read a text like a path; hit test, marquee and the selection box use the line box
  (criterion 25). Nothing of it is stored (ADR 0020 §1).
- 2026-10-20 (milestone 2): `ObjectSnapshot::Text(Box<TextSnapshot>)` is boxed (256 bytes against
  152 to 168 for the other kinds; clippy `large_enum_variant`), as a clone boxes its chain.
  `ObjectSnapshot::rotation()` is no longer `const` (a text's angle is computed from its
  placement).
- 2026-10-20 (milestone 2): undo, redo and object restore go through the history's three-way
  restore, which flattened unknown containers to plain values. It now knows a text container
  (`Node::Text`): a string goes back as its difference, only where nobody typed since, so the
  string stays a `LoroText`. Duplicate copies the container likewise.
- 2026-10-20 (milestone 2): a text is not a clone source (spec "Out of scope": `0049` takes
  paths); Linked clone refuses it as not clonable. A template document copies a text as a text.
- 2026-10-20 (milestone 2): commit labels `add_text`, `edit_text`, `change_font`,
  `change_text_size` with the names "Add text", "Edit text", "Change font", "Change text size".
  A move, rotate or resize of a text writes under the existing labels.
- 2026-10-20 (milestone 2): the golden file alternates "Add text" with "Change text size" (each
  text is made 1 mm larger, then set): a file must not hold adjacent steps with one label
  (`0020` criterion 36).
- Open for milestone 4: criterion 8 for an *existing* text emptied in edit mode. "Removed and no
  history step" would make the removal impossible to undo; the safe default is one step "Edit
  text" that deletes the node (undo brings the text back).
- 2026-10-20 (architect review m2): fixes 1 to 3 and should-fixes 5 to 8 applied
  (`MAX_STORED_TEXT_CHARS` = 100,000 for open and layout, lower-case hashes only, a turn writes
  no size, `Mm` in the layout API, the point limit stops drawing, cap height 0.05 to 2.0 em,
  `proptest` in the workspace). Item 4: `TextOutline::substitute`; every path command (Boolean,
  Combine, Split, Break apart, Cut, Offset, Fracture, Flatten) refuses texts that do not convert
  (`TextRefusal`, all or nothing) and plans the others as the compound path of their glyphs
  (`text_convert::texts_as_paths`), deleting the text node in the command's own step.
- 2026-10-20 (milestone 4): `curvyo-render-core` does not depend on `curvyo-text-core`: the
  session lays texts out (`text_fonts.rs`) and render-core draws the outline it gets, so render
  stays font-free. ADR 0020 §2 named render-core as a caller; nothing it needs is missing.
- 2026-10-20 (milestone 3): `fontdb` names no subfamily, so a host face's style name is made
  from its stretch, weight and slant ("Condensed Light Italic", "Regular"); the included fonts
  take theirs from `font.toml`, which the licence test checks against the name table.
- 2026-10-20 (milestone 5): the refusal of a command on texts with missing faces is posted in
  the canvas notice slot (the slot the other rail and bar commands use), not 4 px under the
  button; a text whose face is still loading refuses with "Fonts are still loading. Nothing was
  changed." (a sentence the UX notes did not name).
- 2026-10-20 (milestone 5): the Fonts tab is a sixth panel tab; the tester's tab-list tests of
  `0043`, `0050` and `0051` were extended by "fonts" (nothing removed or weakened).
- Open: the resize readout's second line "Size 24.0 mm" (UX notes §5) is not built; the Size
  field shows the new size after the release. A History-tab Undo during edit mode does not end edit
  mode first (Save does: it writes the session as its step and edit mode goes on). The Fonts tab keeps every face it loaded (no
  eviction past the visible rows plus 10).
- 2026-10-20 (wasm size): the slice adds 1,414 KiB to the release editor wasm (496 KiB gzip):
  the engine about 1,290 KiB (inside the 1.3 MiB budget), the slice's own code about 120 KiB, so
  83 KiB over 1.3 MiB in total. Breakdown in `docs/technical-debt.md`; open for the customer.
- 2026-10-20 (rebase onto `0037`): Fracture and Flatten take the same text refusal and
  conversion (`stack_plan` callers `plan_fracture`, `plan_flatten`).
- 2026-10-20 (customer): the wasm budget is 1.4 MiB. After the tester review fixes the slice
  adds 1,426 KiB (501 KiB gzip), inside it with 7 KiB to spare (`docs/technical-debt.md`).
- 2026-10-20 (tester review): Object to path selects the result's leaves (as Ctrl+G); every
  history jump (Undo, Redo, Go to version, Clone version, Wipe) ends edit mode first and writes
  its step; a text whose node went meanwhile is kept as a new text, and a write that still fails
  posts "The text could not be written. Nothing was changed."; the host fonts are listed on
  entering the Text tool and showing the Fonts tab only, picks name a face by its file and index;
  colour and bitmap faces are left out by their tables; font files over 64 MiB are not read; the
  Fonts tab holds at most 64 loaded faces; `session/text.rs` is split into `text.rs` (edit mode),
  `text_bar.rs` (Text bar, Fonts tab, font lists) and `text_path.rs` (Object to path, marks).
- 2026-10-20 (UX review): the sink is mounted while the Text tool is active and takes focus in
  the release of the press that starts edit mode, so the next keys never reach the canvas; the
  Text tool draws the hover box of the text under the pointer; the Fonts tab pins the header of
  the block at the scroll top; the Object to path refusal is posted 4 px under its button.
