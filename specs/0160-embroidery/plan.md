# Plan for embroidery

## Slice 1: MVP (criteria 1 to 20)

### Affected crates/modules

- `curvyo-inkstitch-core` (GPL-3.0-or-later, `adrs.md` EM1): `units` (`Mm`, `Angle`),
  `running_stitch`, `tatami_fill`, `satin_zigzag`, `ties`; `kurbo` the only dependency;
  `NOTICE.md` pins upstream `42e771e9` and lists each ported file with its changes.
- `curvyo-embroidery-core` (EM2): `stitch` (`TenthMm`, `StitchPoint`, `Stitch`), `hoop`,
  `profile`, `profile_file` (record format 9, kind `embroidery`), `profile_copy`, `builtins`,
  `operation`, `plan`, `emit`, `summary`, `dst_write`, `dst_read`; goldens in `tests/fixtures/`,
  also compared on wasm32 (`tests/dst_goldens.rs`, CI `boolean-wasm-golden` job); `NOTICE.md` for
  the two functions transliterated from pyembroidery (MIT, `c16d1b46`).
- `curvyo-editor-wasm`: new `session/embroidery/` (`commands`, `operations`, `job`, `draw`,
  `export`, `view_json`, `tests`) and `wasm_embroidery.rs`; hooks in Add page, the manufacturing
  view, the frame (hoop under the page), the draw list (Toolpath view), Export job file, the
  machine library load and the library sync.
- `curvyo-document-core`: `CURRENT_FORMAT_VERSION` 70 (doc only, the records are opaque); test
  literals renumbered. Goldens `embroidery_page_v70.curvyo`, `embroidery_page_damaged_v70.curvyo`
  in `curvyo-editor-wasm/tests/fixtures/`.
- `curvyo-ui-core`: history names Set hoop, Edit speed; step scopes.
- `frontend`: `EmbroideryPanel` (drawn by `ManifestTab` when the view has `embroidery`),
  `lib/embroideryView.ts`, the embroidery machine list of Add page, the embroidery commands in
  `useWorkbenches`.

### Tasks

- [x] E1. Port: running stitch with repeats, tatami fill (grating, sections, staggers), simple
  satin, half-stitch ties; geometry tests in mm before rounding (7, 8, 9, 10, 11).
- [x] E2. Stitch model in tenths from the hoop centre, Y up; hoops and the centred field check;
  W6 EU7 built-in, record format 9 with merge, profile copy (1, 2, 3, 4).
- [x] E3. Operation kinds, numbers with Ink/Stitch defaults, ranges and messages, thread colour
  (5, 6).
- [x] E4. Plan: items through the port, ties, rounding once (no drift), nearest-next items, trims
  above 5 mm, jumps, colour changes, steps over 12.1 mm split, end (7 to 10, 15).
- [x] E5. Summary; DST writer (header, balanced ternary, trim as jumps 2, -4, 2, stop record) and
  test reader; goldens `embroidery_running.dst`, `embroidery_tatami.dst`, `embroidery_satin.dst`,
  `embroidery_mvp_two_colours.dst` with read-back, extents, 30 mm travel, byte-identical (12, 14 to
  18).
- [x] E6. Session: Add page with copy and hoops, Pick and Save as profile, Edit speed, Set hoop
  (1 to 3).
- [x] E7. Session: the three kinds, numbers and thread, plan from the leaves with stroke widths,
  checks outside the hoop, narrow satin and no area (4 to 6, 9).
- [x] E8. Session: hoop drawing, Toolpath view in thread colours up to the play position, play as
  session state, Export job file `<project> - <page>.dst` without a step (3, 12 to 14, 18).
- [x] E9. Library load and sync of embroidery records by their own merge (2).
- [x] E10. Format 70 with the two goldens (20).
- [x] E11. Frontend panel with play bar, Add page machine list (1 to 6, 12 to 14 visible).
- [ ] Criterion 19 is the customer's sewn test on the EU7 (fact 3).

### Deviations (Proposal, for the UX and architect review)

- The thread colour is stored as `thread_colour` in `op_settings`: an operation node has no
  colour register (EM5 assumed one). A new operation's thread is its first item's fill colour for
  a Tatami fill and its stroke colour for Running stitch and Satin.
- Tatami sections are joined by jumps (criterion 8 allows it); the travel graph of Ink/Stitch's
  `tatami_fill.py` is not ported. Every run (a line, an outline, a fill section) is a block with
  its own tie-in and tie-off; a travel over 5 mm inside one item is also trimmed.
- The half-stitch tie is ported verbatim: three needle points away from the start (0.5, 1, 0.5
  lock lengths of 0.5 to 1.5 mm) and back, four short stitches; criterion 10 reads it as "three
  short stitches".
- DST `ST:` counts the records after the header (a trim is three), `+X`/`-X`/`+Y`/`-Y` are the
  needle points' extents with the hoop centre included; a colour change writes no trim before it.
- The embroidery Machine, Hoop, Operations and Output sections are one panel in the Document tab
  (as the plotter's), not split over the Fixtures and Operations tabs; the kind buttons are the
  panel's buttons, the play bar sits in the Output section.
- The hoop frame has square corners and no mounting tab; trims are an ink cross on a white
  disc, a colour change a disc of the new thread at its point (not the filled run ring).
- The licence view of EM8 does not exist in the front end yet; Ink/Stitch and pyembroidery are
  credited in the two crates' `NOTICE.md` files.
- The stitch plan is rebuilt for every view and frame (no cache): fine for the MVP designs, a
  cache is needed for designs of tens of thousands of stitches.

### Validation

- `cargo nextest run -p curvyo-inkstitch-core -p curvyo-embroidery-core`;
  `cargo test --target wasm32-unknown-unknown -p curvyo-embroidery-core --test dst_goldens`.
- `cargo nextest run -p curvyo-editor-wasm -E 'test(/embroidery/)'` and
  `--test embroidery_page_format`.
- `frontend/tests/embroidery_0160.test.ts`; the full `CLAUDE.md` §7 gate once at the end.
