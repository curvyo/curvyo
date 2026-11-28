# ADRs for embroidery (slice 1: MVP)

Architect, 2026-11-27. This note covers slice 1 (criteria 1 to 20) of `specification.md` as it
stood on 2026-11-27. Later slices get dated notes here once they are Ready. Everything below is a
**(Proposal)** unless it quotes an ADR, a criterion or a customer answer (Fragen 420 to 424,
428 A, 430 A).

## ADRs this feature depends on

- [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md) is the family
  decision. This spec uses its process id `curvyo.embroidery` (§1), the crates
  `curvyo-embroidery-core` and `curvyo-inkstitch-core` (§2), the port's licence mechanics (§7;
  Fragen 421 C and 430 A), records of kind `embroidery` (§4), the format rule (§6) and reading as
  paths with a Manual stitch (§9, Frage 432 A, slice 10).
- [ADR 0042](../../docs/adr/0042-page-kinds.md), [ADR 0046](../../docs/adr/0046-laser-operations-record.md),
  [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md) and
  [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md), as in
  `specs/0159-cutting-plotter/adrs.md`.
- [ADR 0006](../../docs/adr/0006-license.md) (note of 2026-11-27): Curvyo stays AGPL; the port
  crate is the one GPL exception.

## Groundwork

G1 to G3 of ADR 0097 §11 come first, once for all three specs (see
`specs/0159-cutting-plotter/adrs.md`). After G1 the port crate exists as an empty skeleton with
`LICENSE` and `NOTICE.md`. The plotter MVP comes before this slice, and its PL4 `path_order` is
used here.

## Feature-local decisions

**EM1. `curvyo-inkstitch-core`** (criterion 11).

- The exact SPDX id is **`GPL-3.0-or-later`**. Ink/Stitch's file headers say "Licensed under the
  GNU GPL version 3.0 or later" (checked 2026-11-27), so the crate's `license` field and every
  file's header use that id.
- Slice 1 modules, each ported from the named Ink/Stitch source:
  - `running_stitch`: `lib/stitches/running_stitch.py` (even spacing, repeats 1, 3 and 5);
  - `tatami_fill`: `lib/stitches/fill.py` and `tatami_fill.py` (rows, staggers, travel inside the
    area);
  - `satin_zigzag`: the zigzag of `lib/elements/satin_column.py`, for one line of constant width;
  - `ties`: Ink/Stitch's default lock stitches.
- The exact upstream paths and the pinned commit go into `NOTICE.md` when each file is ported.
- Input is flattened polygons and polylines as `kurbo::Point` in mm, with the port's own `Mm`
  newtype for parameters and explicit tolerances. Output is needle points in mm with a jump flag.
- Row clipping uses `i_overlay` `=9.0.1` (already in the tree). If its line clip does not cover
  the rows, the port uses its own scanline intersection. Travel graphs are the port's own code;
  `petgraph` only if that code outgrows one module (ADR 0097 §12).
- No `curvyo-*` dependency, and no new dependency (`kurbo` and `i_overlay` are locked).

**EM2. `curvyo-embroidery-core` modules.** Each has one responsibility and stays under 500 lines:

- `manifest`: the word "Embroidery".
- `profile`, `profile_file` and `builtins`: W6 EU7 with id `w6-eu7`, output DST, the hoops' 150 x
  250 and 100 x 100 mm sewing fields, a maximum stitch of 12.1 mm and 600 stitches per minute.
  Records are of kind `embroidery`, first `format_version` 9 in `machines/`.
- `hoop`: fixture kinds `curvyo.embroidery/hoop-150x250` and `hoop-100x100`, centred on the page.
- `stitch`: the stitch model. A `Stitch { at, kind }` has `at` in whole tenths of a mm (the
  `TenthMm` newtype) and a kind of `Normal`, `Jump`, `Trim`, `ColourChange`, `Stop` or `End`.
- `plan`: the blocks of criterion 10: operations in list order, items nearest-next through
  geometry-core `path_order`, tie-in and tie-off, trim above 5 mm and jump below, a colour change
  between colours, and an end.
- `dst_write` and `dst_read`: the reader is public but used only by tests in slice 1. Slice 10
  turns its stitches into paths with a Manual stitch operation (Frage 432 A).
- `summary`: stitch count, colours, trims, thread length and time.

Embroidery-core decides everything that is not derived from Ink/Stitch: the narrow-satin rule of
criterion 9, operation ranges, block order and the hoop check.

**EM3. Rounding without drift** (criteria 15 and 17). The plan rounds each absolute needle point
to 0.1 mm once, and the DST deltas are differences of rounded absolutes. The error therefore
never accumulates, and the read-back matches within 0.1 mm. A step over 12.1 mm in X or Y is split
into jumps of at most 12.1 mm.

**EM4. DST encoding** (criteria 14 and 16).

- The 512-byte header is as criterion 14: `LA:` takes 16 characters cut on a character boundary,
  then the counts and extents in 0.1 mm, padded with spaces, then `0x1A`.
- Records are the standard 3-byte balanced-ternary encoding.
- A trim is the jump sequence fixed by the golden file and checked on the EU7 at the demo, a colour
  change is the stop record, and the file ends with the end record.
- pyembroidery (MIT) is the reference. A function transliterated from it carries the MIT notice in
  its file and in the crate's notice.
- No colours are written, and two exports are byte-identical.

**EM5. What the project stores** (criterion 20; the next free format at merge, N+2 of ADR 0097 §6,
70 today):

- the output page with process `curvyo.embroidery`, and the `mfg_profile` copy;
- the active hoop as one job fixture;
- operations as ADR 0046 nodes with `op_kind` `running`, `tatami` or `satin`. The thread colour is
  the operation node's existing colour text. `op_settings` hold
  `{ stitch_length_mm, repeats }`, `{ angle_deg, row_spacing_mm, max_stitch_mm }` or
  `{ zigzag_spacing_mm }`.

Damage: an unknown hoop reads as the default with a notice, and a value out of range reads as the
default with a notice. Older builds refuse the file because the number goes up.

**EM6. Preview and play** (criteria 12 and 13). The stitch list from `plan` is the single source.
`curvyo-editor-wasm` maps it onto the Toolpath view's draw list: stitches in thread colour, jumps
dashed, and trims and colour changes as marks. The 0029 play slider is reused, and its position is
session state. The summary comes from `summary` over the same list, so it equals the DST
read-back.

**EM7. Hoop check** (criterion 4). A pure `outside_field` over the needle points and the active
field feeds the output gate, as on the plotter (PL9).

**EM8. Licence surface.** Only the stitch list crosses from the port into `curvyo-editor-wasm`;
the port has no types in the editor's API. The front end's licence view lists Ink/Stitch
(GPL-3.0-or-later, with link) and pyembroidery (MIT); UX places it. `cargo deny` allows
`GPL-3.0-or-later` for `curvyo-inkstitch-core` only.

**EM9. wasm.** Both crates are on the CI wasm32 list automatically (ADR 0097 §10). One DST golden
and one port golden (tatami rows) are also compared in the wasm build.

## Note 2026-11-27: after the UX notes

**EM10. DST coordinates count from the hoop centre** (UX M4).

- **Page.** The page is the largest sewing field plus 20 mm on each side, 190 x 290 mm for the
  EU7 (UX M5). Every hoop is centred on it, and "Set hoop" does not resize the page.
- **Translation.** The export maps a page point `(x, y)` (mm, Y down) to the DST position
  `(x - cx, cy - y)` with Y up, where `(cx, cy)` is the active hoop's centre. That centre is the
  page centre. The 0.1 mm rounding of EM3 then applies to these positions.
- **Start.** The machine starts at (0, 0), the hoop centre (Tajima convention). The file begins
  with jumps from (0, 0) to the first stitch when that stitch is elsewhere, and the header extents
  `+X`, `-X`, `+Y`, `-Y` are measured from (0, 0).
- **Demo check.** The golden design `embroidery_mvp_two_colours` gets a 10 mm cross at the page
  centre. On the EU7 it must land on the hoop template's centre cross; that confirms the W6 reads
  DST from the hoop centre.
- **Hoop check.** The check of EM7 uses the same centred field, so screen and file agree. Nothing
  on screen shows DST coordinates.

## Note 2026-11-28: ADR 0098 (UX revision `ux-unify.md`)

- **EM10's page and Set hoop sentences are superseded** by ADR 0098 decision 1. The page is the
  active hoop's sewing field, with no 20 mm margin (`PAGE_MARGIN_MM` and `page_mm` go). Set hoop,
  Pick and Reset resize about the page centre (`PageAnchor::Centre`). The DST translation from
  the page centre is unchanged.
- The panel tabs, the shared views (`KindView` stitch kinds, the row's `thread`, `PlayView` `step`
  and `speed`) and the Machines tab are ADR 0098 decisions 2 to 4. The MVP's `EmbroideryPanel`
  is deleted.
