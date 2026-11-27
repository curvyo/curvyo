# ADRs for the cutting plotter (slice 1: MVP)

Architect, 2026-11-27. This note covers slice 1 (criteria 1 to 18) of `specification.md` as it
stood on 2026-11-27. Later slices get dated notes here once they are Ready. Everything below is a
**(Proposal)** unless it quotes an ADR, a criterion or a customer answer (Fragen 416 to 427,
428 to 431).

## ADRs this feature depends on

- [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md) is the family
  decision. This spec uses its process id `curvyo.cutter` (§1), the crate `curvyo-cutter-core`
  (§2), the records of kind `cutter` (§4), the format bump rule (§6) and the build order (§11).
- [ADR 0042](../../docs/adr/0042-page-kinds.md): a plotter page is an output page whose process is
  `curvyo.cutter`. There is no new `PageKind` value. Its size comes from the profile only (§6).
- [ADR 0046](../../docs/adr/0046-laser-operations-record.md) and
  [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3: operation nodes with
  an opaque `op_kind`, `op_settings` and members, interpreted by the page's process.
- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md) and
  [ADR 0059](../../docs/adr/0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md): mats are
  job fixtures, with a kind, params and `active`.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md): the opaque
  `mfg_profile` copy. [ADR 0038](../../docs/adr/0038-machine-profile-records.md) and
  [ADR 0062](../../docs/adr/0062-synced-per-user-libraries.md): records and sync, through ADR 0097 §4.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) §10 and §11: the pattern
  of one builder for preview and file, the Toolpath view and Export job file. `0150`: an operation
  always has numbers.

## Groundwork before slice 1 (ADR 0097 §11, done once for all three specs)

These are invisible commits on `story/machine-families`, built in this order:

1. **G1.** Skeletons of `curvyo-cutter-core`, `curvyo-embroidery-core`, `curvyo-inkstitch-core`
   and `curvyo-cnc-core`: `Cargo.toml`, `lib.rs` with `#![forbid(unsafe_code)]` and `manifest.rs`.
   Each crate gets its `deny.toml` exception (AGPL for three, `GPL-3.0-or-later` for the port) and
   an entry in `curvyo-ui-core/src/extensions.rs`.
2. **G2.** An output page per process: Add page offers the family's word, the history label is
   "Add <word> page", and each family has built-in profiles in code. Fixture kinds are written as
   `<process>/<kind>`.
3. **G3.** `record_envelope`, `record_merge` (taking read outcomes), `record_kind` and the
   `connection` value move from `curvyo-laser-core` into `curvyo-library-core`. The laser's goldens
   stay byte-identical. The profile pick filters by `kind` (criterion 2).

## Feature-local decisions

**PL1. Crate and modules.** The page word is "Plotter" (criterion 1); the stored process id stays
`curvyo.cutter`. `curvyo-cutter-core` depends on document-core, geometry-core, extension-core and
library-core. Each module has one responsibility and stays under 500 lines:

- `manifest`.
- `profile`: the body, which holds the work area, origin corner, output format (`DesignSpaceSvg`,
  `Hpgl`), force range, default pressure and speed, the mats offered and the connection.
- `profile_file`: the record on the library-core envelope.
- `builtins`: Cricut Maker 3 with id `cricut-maker-3`, and Generic HPGL cutter with id
  `generic-hpgl-cutter`. The ids are not UUIDs, as `generic-laser` is not, and are never written.
- `mat`: sizes, sheet, printed field, cut area, origin, grip and the grid drawing as outlines in mm.
- `operation`: the kinds `cut` and `draw`, the settings newtypes and their ranges.
- `job`: the moves.
- `write_svg` and `write_hpgl`.

Newtypes: `PressurePct` (1 to 100), `SpeedMmS`, `Passes` (1 to 10), `Force` and `PlotterUnit`
(`i32`, 40 per mm).

**PL2. Profile record.** The profile is a record of kind `cutter` in `machines/`. Its first
`format_version` is 9, one above the laser's 8 (ADR 0097 §4), so older builds skip it as newer.
It syncs through the existing `machines` map, and the merge uses the cutter reader's outcome
(criterion 18). A laser profile is never offered on a plotter page, and the other way round.

**PL3. What the project stores** (criterion 17; the project format number is the next free at
merge, N+1 of ADR 0097 §6, 69 today):

- the output page with process `curvyo.cutter`, and the `mfg_profile` copy;
- the mat as one job fixture of kind `curvyo.cutter/mat-12x12` or `curvyo.cutter/mat-12x24`, with
  the params `grip` (`light`, `standard`, `strong`, `fabric`) and `grid` (a boolean). Exactly one
  is `active`;
- the operations as ADR 0046 nodes with `op_kind` `cut` or `draw` and `op_settings`
  `{ pressure_pct, speed_mm_s, passes }`. The page's process decides the kind's meaning, so the
  laser's `cut` does not clash.

Criterion 17 asks older builds to refuse the file, so this family takes the bump. Its later slices
on this branch extend the same number (Frage 431 A).

Damage: an unknown mat kind reads as the default mat with a notice, and a pressure out of range
reads as the default with a notice. Nothing is rewritten on read.

**PL4. Shared path order.** Criterion 9 needs the laser's order from `0029`: a path runs before
every closed path that contains it, then the nearest start goes next. That order lives in
`curvyo-laser-core`'s `job_order` and `job_nesting`, tied to laser types (hatch, tabs). The generic
part moves into a new `curvyo-geometry-core::path_order` module. Its input is flattened paths, a
start point and a direction rule; its output is the order with each path's start point.

The plotter is its first user, CNC (`0161` criterion 9) and embroidery (`0160` criterion 10)
follow, and that makes four users. The laser moves onto it in the same PR in a `refactorer` pass,
with every laser golden byte-identical.

**PL5. One builder for preview and file.** `job` builds the moves in µm: work moves (Cut or Draw)
and travel. Curves are flattened with geometry-core `flatten` at 0.01 mm. Passes repeat the run.
The HPGL time is length divided by speed. For Design Space SVG there is no time, and the row
shows "Set in Design Space".

`curvyo-editor-wasm` maps the moves onto the Toolpath view's existing draw list
(`session/output/toolpath.rs`), so the front-end view and play slider are reused. There is no
Rust move type shared with the laser (ADR 0097 §3).

**PL6. Design Space SVG** (criteria 11 and 16). The writer is an own string writer in cutter-core:
a job file, not a document export.

- `width` and `height` are the mat size in `in` (`12in`), and the `viewBox` is in mm
  (`0 0 304.8 304.8`), so the coordinates are mm from the mat origin.
- Numbers have 3 fixed decimals.
- There is one `<g>` per operation, whose `id` and `inkscape:label` come from the operation name,
  made unique by a counter.
- Draw is black; Cut groups take colours from a fixed list in operation order.
- No date and no random id, so two exports are byte-identical.

**PL7. HPGL** (criteria 13 to 16):

- the commands are as in criterion 13: `IN;`, `VS` in cm/s and `FS` per operation, `PU` and `PD`;
- points are rounded half away from zero to whole plotter units (40 per mm), with the origin and Y
  axis from the profile;
- consecutive equal points are dropped after rounding;
- passes repeat each path's `PD` run;
- the file ends with `PU;SP0;`.

Goldens go in `curvyo-cutter-core/tests/fixtures/` (`cutter_hpgl_square.plt` and the SVG
counterparts). One of them is also compared in the wasm build, as
`curvyo-laser-core/tests/adjust_goldens.rs` does.

**PL8. Item geometry** (criterion 7). Leaves come from the editor's existing leaf geometry
(`session/output/leaf_geometry.rs`). A single-line font text gives its strokes as open paths, and a
TrueType text gives its glyph outlines. Draw takes the path itself whatever its fill. This is not
the medial axis of ADR 0063.

**PL9. Cut-area check** (criterion 6). cutter-core gives a pure `outside_cut_area` over the items'
bounds and the active mat's cut area. An item partly outside counts as outside. The editor feeds it
into the output group's gate (`session/output/gate_facts.rs`) as the laser's work-area check does.

**PL10. Not in slice 1.** No new dependency, no connector or protocol change (ADR 0097 §8 is
slice 6), no GPGL and no CutcutGo. Drag-knife offset and overcut are slice 2.

## Note 2026-11-27: after the UX notes

**PL11. The page is the active mat's printed field** (UX C4 and M1; amends criterion 1's "the
profile's work area" for mat profiles).

- **Page size.** A Maker 3 page is 304.8 x 304.8 mm (12 x 12 in) or 304.8 x 609.6 mm (12 x 24 in).
  Its origin is the mat origin, the field's top-left corner as fed. A matless profile (Generic
  HPGL) keeps page = work area and its Origin corner.
- **Set mat.** "Set mat" is one commit that switches the active mat fixture and writes the page
  size. The resize is the corner-anchored one of ADR 0042 §6, anchored at the top-left corner:
  placements keep their global place, fixtures are recomputed, and own items are not written.
- **Amendment.** This amends ADR 0042 §6, where only `set_manufacturing_profile` writes an output
  page's size, by this note: the active mat is the second writer, and only on pages whose profile
  has mats. The lead adds a one-line note to ADR 0042's Status.
- **Consequences:**
  - the export positions of PL6 are page positions, with no translation;
  - the cut area (0.25 in inside the field, UX C6) is a fixture outline inside the page;
  - a Maker 3 profile has no Origin corner (the mat fixes it);
  - PL3 stores no extra register, because the page size is the page's own register.
