# Plan for CNC output

## Slice 1: MVP (criteria 1 to 20)

### Affected crates/modules

- `curvyo-cnc-core` (CN1): `units` (`ToolDiameter`, `Depth`, `StepDown`, `FeedMmMin`,
  `AccelMmS2`, `Rpm`, `Flutes`), `work` (`page_to_work`, `work_to_page`, CN10), `profile`,
  `profile_file` (record format 9, kind `cnc`), `profile_copy`, `builtins`, `stock`, `tools`,
  `operation`, `profile_toolpath`, `job`, `checks`, `write_grbl`; goldens in `tests/fixtures/`.
- `curvyo-connector-core`: new `virtual_grbl` (`codes`, `settings`, `parse`, `planner`, `status`,
  `system`, the responder); the line check moves from connector-io's `grbl::prepare` into
  `gcode_line` (re-exported unchanged).
- `curvyo-connector-io`: `examples/grbl_virtual.rs`; `SerialSettings::spindle` and
  `GrblController::with_spindle`; `tests/grbl_virtual_cnc.rs`. `curvyo-connector-server`:
  `--spindle`.
- `curvyo-document-core`: `CURRENT_FORMAT_VERSION` 71; `Document::set_process_register` (the
  `x.curvyo.cnc.tools` register of CN2); history labels. Test literals renumbered.
- `curvyo-editor-wasm`: new `session/cnc/` (`commands`, `operations`, `job`, `draw`, `export`,
  `view_json`, `tests`) and `wasm_cnc.rs`; hooks in Add page (the generic family page path is
  gone), the manufacturing view, the frame (stock under the page), the draw list, Export job file,
  the machine library load and the library sync; `curvyo-connector-core` becomes a normal
  dependency (the planner). Goldens `cnc_page_v71.curvyo`, `cnc_page_damaged_v71.curvyo`.
- `curvyo-ui-core`: history names and step scopes of the stock, tools and machine fields.
- `frontend`: `CncPanel` (drawn by `ManifestTab` when the view has `cnc`), `lib/cncView.ts`, the
  CNC machine list of Add page and the CNC commands with Send in `useWorkbenches`.

### Tasks

- [x] C1. Units, work coordinates, Generic GRBL router, record format 9 with merge, copy (2).
- [x] C2. Stock fixture with ranges and messages; tool list register with ranges (3, 4).
- [x] C3. Profile settings in `op_options` and `op_settings`, passes in whole micrometres (5, 8).
- [x] C4. Profile toolpath: filled region offset by the radius with round joins, open path and too
  narrow reported, holes first then nearest-next, Climb and Conventional in work coordinates
  (6, 7, 9).
- [x] C5. GRBL writer and goldens `cnc_grbl_profile_outside.nc`, `cnc_grbl_profile_inside.nc`,
  `cnc_grbl_two_parts.nc`, byte-identical, line rules (12 to 14).
- [x] C6. Virtual GRBL responder, error and alarm table, settings, status, hold, reset, homing,
  jog, character counting, 15-block planner, `plan_time` (15 to 18).
- [x] C7. `grbl_virtual` example; the two-parts golden through the grbl-serial controller to
  `Done { Completed }` with `--spindle` (13, 15, 19).
- [x] C8. Session: Add page with the copy and the stock, Pick, Save as profile, connector address,
  machine fields, stock, tools (1 to 4).
- [x] C9. Session: Profile outside and inside, settings, checks (outside the stock, outside the
  work area, another tool, too narrow, open path), Toolpath view with band, dots, head and play,
  time from `plan_time`, Export `<project> - <page>.nc`, Send with Export's bytes (4 to 14, 18, 19).
- [x] C10. Library load and sync of CNC records (2); format 71 with the two goldens (20).
- [x] C11. Frontend panel and Add page machine list (1 to 12, 19 visible).

### Deviations (Proposal, for the UX and architect review)

- **Connector spindle mode.** The grbl-serial controller refuses every job while GRBL reports
  `$32=0` (Frage 240 B), and a router runs with laser mode off. `curvyo-connector-server
  --serial <port> --spindle` treats `$32=0` as expected; without the flag the laser guard is
  unchanged. The service mode (`connector.toml`) has no such field yet. This changes accepted
  behaviour only behind the new flag; the architect may prefer the `dialect` of ADR 0097 §8.
- The line check lives in `curvyo-connector-io`, not in connector-core as CN4 says; it moved into
  `curvyo-connector-core::gcode_line` so the virtual GRBL and the goldens share it.
- Holes are found by nesting (even-odd) for every item; the item's own fill rule is not read
  (CN11 says its fill rule). Same result for every ordinary drawing.
- The plunge goes from the safe height to the pass depth at the plunge feed (criterion 12 read
  literally); no rapid down to a clearance plane first.
- The CNC Machine, Stock, Tools, Operations and Output sections are one panel in the Document tab
  (as the plotter's and embroidery's), not split over a Fixtures, a Tools and the Operations tab;
  the Profile buttons are the panel's. The Tool pick is a segment row, not the material pick look.
  Tools cannot be renamed yet (the name follows the diameter, UX N2).
- Send in the panel: a Send button and the connector's state line with the job's line; the 0118
  machine controls footer (Pause, Stop, Home, Jog) is not shown on CNC pages yet. The connector
  address is a field on a saved profile (a built-in has none), not the 0064 Connection group.
- The Toolpath view draws the tool band on the first pass only (passes share the drawn path, UX
  N7) and has no arrowheads; the stock label "Plywood · 10 mm" is not drawn.
- The virtual GRBL treats `G2`/`G3` as straight moves to their end (the writer writes none), runs
  a hold as an immediate stop with GRBL's deceleration time, and ignores `$C` check mode.
- Planner time: the whole program is planned between spindle changes (infinite look-ahead); the
  virtual GRBL replans its 15 queued blocks as GRBL does; criterion 18's test asserts within 1 %.

### Validation

- `cargo nextest run -p curvyo-cnc-core -p curvyo-connector-core`; `cargo nextest run -p
  curvyo-connector-io -E 'binary(grbl_virtual_cnc)'`.
- `cargo nextest run -p curvyo-editor-wasm -E 'test(/cnc/)'` and `--test cnc_page_format`.
- `frontend/tests/cnc_0161.test.ts`; the full `CLAUDE.md` §7 gate once at the end.
