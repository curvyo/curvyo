# Plan for the laser workbench shell

Implementer, 2026-11-02. Branch `story/0063-laser-workbench`. ADRs 0032, 0033, 0034.

## Affected crates/modules

- **New `curvyo-extension-core`** (ADR 0034 §1, §4): `Manifest`, `Contribution::Workbench`,
  `Tab`, `Block` (`Params`, `FixtureList`, `Notice`), `ToolboxCard`, `FixtureKind`, `Param`,
  `ParamKind::Length`, `ParamValue`; JSON for the front end; `CHANGELOG.md`.
- **New `curvyo-laser-core`** (ADR 0034 §1): `manifest()`, the Engraved grid outline
  (spacing, line width, origin offset over the work area), the overhang notice text.
- **`curvyo-document-core`**: `manufacturing.rs` (commands: add, delete, rename-free placeholder,
  work area, fixtures), `manufacturing_codec.rs` (registers, open validation),
  `manufacturing_read.rs` (snapshot, placement resolution), the skip predicate in
  `group_tree::children_of` at the root, owned pages out of `pages()`, `format_version` 35,
  `document.json`, goldens `laser_workbench_v35.curvyo` and `manufacturing_unknown_process_v35.curvyo`.
- **`curvyo-ui-core`**: `extensions.rs` (compiled-in list, dispatch by id, `extensions.toml`
  parse/write), `PanelTab::Extension`.
- **`curvyo-editor-wasm`**: `Workbench::Manufacturing(NodeId)`, workbench list and manifest JSON,
  manufacturing view JSON, laser frame drawing (output page, fixtures, placements), read-only
  notice, per-workbench view, wasm commands; CPU-time test.
- **`frontend`**: `workbenches/registry.ts`, `WorkbenchSwitcher`, Add manufacturing group,
  `ManifestWorkbench` (Laser tab body, View toolbox card), Vector and Sketch behind the registry,
  Extensions group in the File tab, status bar text; vitest for the registry.
- **docs**: `docs/design-system.md` rows, spec status.

## Tasks

- [x] 1. `curvyo-extension-core`: contract types, JSON, value validation against the schema;
  unit tests (fulfils AC 11).
- [x] 2. `curvyo-laser-core`: manifest (tab Laser: Notice, Params work area, FixtureList; card
  View; fixture kind Engraved grid), grid outline, overhang text; unit tests (AC 8, 9, 15).
- [x] 3. document-core skip predicate `is_manufacturing_node` at the root walk, one test per
  consumer (Layers rows/children, read_objects/hit test/marquee/select all, page membership,
  export json objects, repairs) (AC 4, 10).
- [x] 4. document-core manufacturing commands: add (node + owned page 400 x 300 + placement of
  the first drawing page, one step), delete (node + page, one step, undo restores), work area
  (one step, page follows), fixtures add/remove/visible/params (one step each) (AC 2, 3, 5, 9,
  10, 13).
- [x] 5. document-core read: `manufacturings()`, placement resolution (page members, guides and
  hidden out, unclipped), owned page excluded from `pages()` (AC 4, 5, 6, 8).
- [x] 6. document-core format: open validation of manufacturing subtrees and owners, unknown
  process kept unchanged, `format_version` 35, `document.json`, two goldens (AC 12).
- [x] 7. ui-core `extensions.rs`: list, dispatch (`fixture_outlines`, validation), disabled list
  in `extensions.toml`; `PanelTab::Extension` (AC 11).
- [x] 8. editor-wasm session: `Workbench::Manufacturing`, switching with its own view (fit to the
  output page), panel tabs Laser/File/History, laser frame drawing, read-only notice on press
  and edit keys, commands, extension enable state, workbench list and views as JSON; tests
  (AC 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 13).
- [x] 9. editor-wasm CPU-time test: 1,000 objects, opening Laser and one drawing edit (AC 14).
- [x] 10. frontend: registry, switcher row, Add manufacturing group, `ManifestWorkbench`
  (blocks `Notice`, `Params`, `FixtureList`; View card), Extensions group, status bar;
  `TracePresetMenu name="Workbench"` removed; vitest with a dummy manifest (AC 1, 4, 7, 9, 11).
- [x] 11. docs: design-system rows (switcher, Add manufacturing group, schema-rendered tab,
  Extensions group), spec status; full gate (AC 15 by construction: no new dependency, no I/O).

## Validation

- Goldens `laser_workbench_v35.curvyo` (2 fixtures, one hidden, 600 x 400 mm, one placement) and
  `manufacturing_unknown_process_v35.curvyo` (opens, disabled, saves back with equal content).
- CPU-time budget test in `curvyo-editor-wasm/tests/manufacturing_budget.rs` (release-asserted
  like the other budgets).
- Manual check in the browser build: add Laser, edit in Vector, switch, fixtures, extension switch.

## Deviations and notes (implementer, 2026-11-03)

- **Skip predicate placement:** the one predicate lives at the root walk
  (`group_tree::children_of(None)`, `leaves_under`) and in `path_codec::leaf_exists`, so every
  consumer that walks the drawing or takes a node id skips manufacturing subtrees without its
  own check; `Document::is_in_manufacturing` is the id-level form.
- **Placement resolution** runs in `curvyo-ui-core::placement_view` (object bounds live there),
  over the session's one drawing read, cached per read; not in `document-core`'s read pass.
- **Fixture lines** are drawn one device pixel wide in the grid ink; the stored line width is
  kept for the later engraving job.
- **Fixture trash** deletes at once like the manufacturing trash (undoable), no inline arm.
- **Pan and Zoom** did not exist as tools; the View card's Pan and Zoom are view modes of the
  front end (H, Z), Fit page fits the output page.
- **Overhang notice** is in mm only (no display-unit conversion yet).
- Tester tests touched only where a format bump forced it, as earlier bumps did: version pins
  relaxed (`acceptance_0070_tester`, `acceptance_0024_slice2_tester`), the 0093 goldens added to
  the later-build exclusions of `acceptance_0050_tester`.
