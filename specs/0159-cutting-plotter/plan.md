# Plan for the cutting plotter

## Groundwork (ADR 0097 §11 steps G1 to G3, done once for `0159`, `0160` and `0161`)

Invisible commits on `story/machine-families`; no criterion of slice 1 is met by them alone, they
are what M1 to M3 build on. The shared path order (`adrs.md` PL4) is a feature-local decision of
slice 1, not groundwork, and comes with M1.

### Affected crates/modules

- new: `curvyo-cutter-core`, `curvyo-embroidery-core`, `curvyo-cnc-core` (AGPL-3.0-or-later),
  `curvyo-inkstitch-core` (GPL-3.0-or-later, `LICENSE`, `NOTICE.md`, SPDX headers, no
  dependencies yet); workspace `Cargo.toml`, `deny.toml`.
- `curvyo-ui-core`: `extensions.rs` (registry), `history_names.rs`, `step_scope.rs`.
- `curvyo-editor-wasm`: `session/manufacturing/commands.rs` (Add page per family).
- `curvyo-library-core`: new `record_envelope`, `record_merge`, `record_kind`, `connection`.
- `curvyo-laser-core`: `record_envelope` and `library_merge` leave; `machine_profile_file` and
  `material_record_file` sort by `kind` first; `Connection` and `BaudRate` re-exported.

### Tasks

- [x] G1. Four crate skeletons with `#![forbid(unsafe_code)]`, each family with `manifest.rs`
  (`PROCESS_ID` `curvyo.cutter`, `curvyo.embroidery`, `curvyo.cnc`; words "Plotter",
  "Embroidery", "CNC" per the adrs.md files) and a compiled-in `Extension` in `EXTENSIONS`;
  `deny.toml` exceptions; wasm32 build of each (ADR 0097 §2, §7, §10; prepares AC 1).
- [x] G2. Built-in profiles in code per family (Cricut Maker 3 and Generic HPGL cutter; W6 EU7;
  Generic GRBL router) with id, name and page size; Add page of a family process takes its
  default built-in's size, writes no laser copy **(Proposal: the family copy is M1 to M3's)**, and
  commits as "Add plotter page", "Add embroidery page", "Add CNC page" (ADR 0097 §1, §11;
  prepares AC 1, 2).
- [x] G3. Record envelope, merge (taking read outcomes), `kind` dispatch and the `connection`
  value move into `curvyo-library-core`; the laser readers skip another family's record silently
  and an unknown kind as newer; laser goldens byte-identical (ADR 0097 §3, §4; prepares AC 2, 18).
  The family record readers and writers (`profile_file`, format 9) come with M1 to M3.

### Validation

- `git diff --exit-code -- curvyo-laser-core/tests/fixtures` empty after G3; the laser merge tests
  pass unchanged.
- `cargo build --target wasm32-unknown-unknown -p` each new crate; `cargo deny check licenses`.
- Full `CLAUDE.md` §7 gate once at the end.

## Slice 1: MVP (criteria 1 to 18)

### Affected crates/modules

- `curvyo-geometry-core`: new `point_um`, `path_nesting`, `path_order` (PL4); `curvyo-laser-core`
  `job_order` runs on them, `job_nesting` leaves, `PointUm` is re-exported.
- `curvyo-cutter-core`: `numbers`, `profile`, `profile_file` (format 9), `profile_copy`,
  `builtins`, `mat`, `mat_print`, `operation`, `job`, `cut_area`, `write_svg`, `write_hpgl`;
  goldens in `tests/fixtures/`, also compared on wasm32 (`tests/writer_goldens.rs`).
- `curvyo-editor-wasm`: new `session/plotter/` (`commands`, `operations`, `job`, `draw`,
  `export`, `view_json`, `tests`) and `wasm_plotter.rs`; hooks in Add page, the manufacturing
  view, the frame (mat under the page), the draw list (Toolpath view), Export job file, the
  machine library load and the library sync; `active_manufacturing` is the laser's only.
- `curvyo-document-core`: `CURRENT_FORMAT_VERSION` 69 (doc only, the records are opaque); test
  literals renumbered. Goldens `plotter_page_v69.curvyo`, `plotter_page_damaged_v69.curvyo` in
  `curvyo-editor-wasm/tests/fixtures/`.
- `curvyo-ui-core`: history names Set mat, Set grip, Mat grid, Edit default pressure, Edit max
  speed; step scopes.
- `frontend`: the shared output page of ADR 0098 (the panel is deleted), `MatLabels`,
  `AddPageGroup` machine list per family, icons Slice, Spool, Drill, `--mat-*` tokens.

### Tasks

- [x] M1a. path_order in geometry-core, laser moved onto it, laser goldens byte-identical (9).
- [x] M1b. cutter-core numbers, profile, record format 9 with merge, copy, built-ins (2, 8, 18).
- [x] M1c. mats: sizes, grips, field, cut area, sheet, notch, printing (3, 4, 5).
- [x] M1d. Cut and Draw settings with ranges and messages (7, 8).
- [x] M1e. job (flatten 0.0095 mm, order, passes), lengths and time (9, 10, 14, 15).
- [x] M1f. Design Space SVG and HPGL writers, goldens `cutter_mat12_cut_draw.svg`,
  `cutter_mat24_cut.svg`, `cutter_hpgl_square.plt`, `cutter_hpgl_cut_draw.plt` (11, 13 to 16).
- [x] M1g. session: Add page with copy and mats, Pick and Save as profile, defaults, Set mat
  resizing the page from the mat origin, Set grip, Mat grid (1 to 5, 8).
- [x] M1h. session: Cut and Draw assignment, Draw before Cut, Edit settings (7 to 9).
- [x] M1i. session: cut-area check, Toolpath view from the job's moves, Export job file under the
  laser name rule without a history step (6, 10, 11, 13, 16).
- [x] M1j. library load and sync of cutter records by their own merge (18).
- [x] M1k. format 69 with the two goldens (17).
- [x] M1l. frontend panel, mat labels, Add page machine list (1 to 11 visible).

### Revision 2026-11-28 (ADR 0098, `docs/design-drafts/2026-11-machine-families/ux-unify.md`)

- [x] U1. The page is the machine work area (the active mat's cut area, the mat drawn around it, `PAGE_ORIGIN_ON_MAT` maps to the mat for the SVG); writer goldens unchanged.
- [x] U2. The family panel is withdrawn and deleted: the page uses the laser's Operations tab
  (rows, unfolded body, output group with Export), Select-bar kinds, the Fixtures tab (Mat), the
  Machines tab, the page-label Toolpath toggle and the canvas play bar.
- [x] U3. The canvas play bar on the job's time (HPGL) or path length (Design Space, UX C14) with
  the head drawn (`curvyo-cutter-core` `PlayAxis`); Cuts last on by default (C12, the format-39
  `x.<process>.cuts_last` register, Draw in Marking before Cut in Cutting); Add page preselects the
  machine the last plotter page was added with (C3, U20).
- [x] U4. Every Machines-tab edit, add, duplicate, rename and delete is one undo step with the
  library journal (ADR 0098 decision 4; all four families).

### Deviations (Proposal, for the UX review)

- Draw moves in the Toolpath view use the laser's Score stroke (1.5 px ink, dotted) instead of a
  solid ink stroke; no run-ring digits yet.
- The remembered machine per kind (UX C3, U20) is kept for the session only; machine-choice.json
  holds the laser's pick alone (a per-kind shape would change that per-user file's format).

### Validation

- `cargo nextest run -p curvyo-cutter-core -p curvyo-geometry-core -p curvyo-laser-core`;
  `git diff --exit-code -- curvyo-laser-core/tests/fixtures`.
- `cargo nextest run -p curvyo-editor-wasm -E 'test(/plotter/)'` and `--test plotter_page_format`.
- `frontend/tests/plotter_0159.test.ts`; the full `CLAUDE.md` §7 gate once at the end.
