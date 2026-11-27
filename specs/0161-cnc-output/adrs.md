# ADRs for CNC output (slice 1: MVP)

Architect, 2026-11-27. This note covers slice 1 (criteria 1 to 20) of `specification.md` as it
stood on 2026-11-27. Later slices get dated notes here once they are Ready. Everything below is a
**(Proposal)** unless it quotes an ADR, a criterion or a customer answer (Fragen 425, 426 C,
431 A).

## ADRs this feature depends on

- [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md) is the family
  decision. This spec uses its process id `curvyo.cnc` (§1), the crate `curvyo-cnc-core` (§2),
  records of kind `cnc` with a connection (§4), the format rule (§6), the virtual GRBL in
  `curvyo-connector-core` served on a pty (§8), and CNC tools in `tools/` for slice 6 (§5, Frage
  431 A).
- [ADR 0042](../../docs/adr/0042-page-kinds.md), [ADR 0046](../../docs/adr/0046-laser-operations-record.md),
  [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md) and
  [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md), as in
  `specs/0159-cutting-plotter/adrs.md`.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §4: offsets on
  `i_overlay`'s offsetter through `curvyo-geometry-core`.
- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) and `0127`: sending, and the line
  checks.

## Groundwork

G1 to G3 of ADR 0097 §11 come first (see `specs/0159-cutting-plotter/adrs.md`). G3 matters most
here, because Send (criterion 19) reads the connector address from a CNC profile record. PL4's
geometry-core `path_order` from the plotter MVP is used here.

## Feature-local decisions

**CN1. `curvyo-cnc-core` modules.** Each has one responsibility and stays under 500 lines:

- `manifest`: the word "CNC".
- `profile`, `profile_file` and `builtins`: Generic GRBL router with id `generic-grbl-router`, as
  criterion 2. The output format is a closed enum, `Grbl` only in slice 1. Records are of kind
  `cnc`, first `format_version` 9 in `machines/`, with the library-core `connection`.
- `stock`: one job fixture of kind `curvyo.cnc/stock` with params `width_mm`, `height_mm`,
  `thickness_mm` and `material`, anchored at the page origin. A fixture is never output, is drawn
  and feeds the gate, which is what criterion 3 asks.
- `tools`: the page's tool list.
- `profile_toolpath`: the Profile operation.
- `job`: 3D moves (rapid, plunge, feed).
- `write_grbl`.

Newtypes: `Depth`, `ToolDiameter`, `FeedMmMin`, `Rpm` and `StepDown`, all in mm or per minute.

**CN2. What the project stores** (criterion 20; the next free format at merge, N+3 of ADR 0097 §6,
71 today):

- the output page with process `curvyo.cnc`, the `mfg_profile` copy, and the stock fixture;
- the tool list as one atomic register `x.curvyo.cnc.tools` on the manufacturing node. It is a
  list of `{ id, kind: "flat", diameter_mm, flutes, feed_mm_min, plunge_mm_min, rpm,
  step_down_mm }`; `id` is a UUID made from caller random bytes, as record ids are;
- Profile operations as ADR 0046 nodes with `op_kind` `profile`. `op_options` hold
  `{ side, depth_mm, tool, direction }`, and `op_settings` hold the typed feed overrides
  (criterion 5).

The tool library of slice 6 (records in `tools/`) will copy a tool into this list, as the profile
copy does. A deleted library tool never changes a project.

Damage: a tool diameter of 0 reads as the default with a notice, and so does a depth beyond the
stock.

**CN3. The Profile toolpath** (criteria 6 to 9):

- The tool centre path is a geometry-core offset by the tool radius with round joins: outward for
  Outside, inward for Inside.
- An inner offset that vanishes is `TooNarrow` and blocks Export. An open path is `OpenPath`, and
  is skipped.
- Climb and Conventional set the path direction through geometry-core `direction` (spindle
  clockwise).
- There are `ceil(d / s)` passes at equal steps, the last at exactly `-d`.
- The order is holes first, then nearest-next, through `path_order`.
- Tolerances: 0.01 mm for the offset and for flattening.

**CN4. The GRBL writer** (criteria 12 to 14):

- the lines are as criterion 12: `G21 G90 G17 G94`, `M3 S`, safe height, plunge, feed and
  retract, then `M5` and `M30`;
- moves are `G1` only in slice 1, with no arcs, so `cavalier_contours` is not needed (ADR 0097
  §12);
- numbers have at most 3 decimals and no exponent, with `-0` written as `0`;
- lines are at most 79 characters, with no `?`, `!` or `~` outside comments;
- two exports are byte-identical.

The goldens are checked with `curvyo-connector-core`'s own line check (a dev-dependency) so the
two never disagree.

**CN5. The virtual GRBL** (criteria 15 to 18):

- **Responder.** `curvyo-connector-core::virtual_grbl` is pure and builds for wasm32. It takes
  bytes and the time the caller passes in, and returns bytes. It covers the GRBL 1.1 welcome line,
  `ok` and `error:N` from a table taken from the GRBL 1.1 documentation, `?` status, `!`, `~`,
  `0x18`, `$$`, `$H`, `$X`, soft-limit alarm 2, and the executed moves with their times. It
  rejects `M6` and `G81` with error 20.
- **Planner.** One pure function, `plan_time(gcode, &GrblSettings)`: trapezoid speed profiles, and
  junction deviation 0.01 mm. The settings come from the CNC profile (max rate, acceleration and
  travel per axis).
- **Example.** `curvyo-connector-io/examples/grbl_virtual.rs` (the name criterion 15 asks for)
  serves the responder on a pseudo-terminal, with the same pty setup as `grbl_pty.rs`, which stays
  for the laser's tests. By default it advances its own clock as fast as the host allows;
  `--real-time` is for the demo. The connector opens the path with its existing `grbl-serial`
  controller (`--serial <path>`), so the real controller code runs against it.
- **Preview time.** The Toolpath view's time (criteria 11 and 18) calls the same `plan_time` from
  `curvyo-editor-wasm`, which already depends on `curvyo-connector-core`. The two times are
  therefore identical by construction, not merely within 1 %.

**CN6. Send** (criterion 19). `session/output/send.rs` reads the connection from the page's
profile record through the library-core `connection` value, for any family that has one. The job
is G-code, so the connector protocol is unchanged in slice 1 (`PROTOCOL_VERSION` stays 0; ADR 0097
§8's `dialect` arrives with the plotter's send slice).

**CN7. Preview** (criterion 11). `curvyo-editor-wasm` maps the moves onto the Toolpath view's
existing draw list:

- cutting moves in one colour per operation, and rapids dashed;
- plunges as dots, and the tool width as a faint band;
- the pass depth in the readout.

The play slider of `0029` is reused. The 2.5D height map and the 3D view are slice 5 (Frage 426 C;
the height map goes in cnc-core, ADR 0097 §8).

**CN8. Checks** (criteria 4, 7 and 10): one tool per job, too narrow, outside the stock and
outside the work area. They are pure functions in cnc-core and feed the output gate as on the
plotter (PL9).

**CN9. Not in slice 1.** No new dependency, and no other dialects (grblHAL, Mach3, Mach4 and
LinuxCNC are slice 3, Frage 425 A). No tool changes and no `tools/` records.

## Note 2026-11-27: after the UX notes

**CN10. Work coordinates** (UX M6, N13). The page keeps the spec's picture: page origin top left,
Y down, and the stock's top-left corner at the page origin.

- **Work zero.** The G-code is written in work coordinates (G54) with **work zero at the stock's
  front-left corner on its top face**, as GRBL practice has it (the corner the maker touches off).
  "Front" is the page's bottom edge, the operator side, as in every CAM view from above.
- **Transform.** A page point `(x, y)` (mm) maps to `X = x`, `Y = stock_height - y`, and depths
  map to `Z = -depth`, with Z = 0 on the stock top. Rapids go to `+safe height`.
- **Code.** The pure functions `page_to_work` and `work_to_page` in `cnc-core` are the only place
  this happens. The writer uses the first; the Toolpath view and the virtual GRBL comparison
  (criterion 17) use the second. The header comment names the zero: `(Work zero: front left
  corner of the stock, Z 0 on top)`.
- **Origin corner.** The profile's Origin corner is not used by the CNC output: the work offset is
  the maker's touch-off, not the machine's homing corner. Following the UX rule that a field which
  changes nothing is removed (0159 C13), the CNC Machine block should not show it. This is a
  finding for UX and the product owner. The work-area check of criterion 10 stays page-based.
- **Virtual GRBL.** It starts with machine position = work position (no G54 offset) and GRBL's
  default `$20=0` (soft limits off), so slice 1 jobs run without a touch-off step. The soft-limit
  case of criterion 16 sets `$20=1` in its own test.
- **Later.** A choice of another work-zero corner is a later slice if the customer asks for one;
  none is built speculatively.

**CN11. Profile on a part with holes** (UX M1, criterion 6). Profile offsets the item's filled
region (its fill rule), not each outline on its own. Outside grows the region by the tool radius,
so the outer boundary moves outward and holes shrink: the part and its holes keep their drawn size.
Inside shrinks it, so the outer boundary moves inward and holes grow. A region piece that vanishes
is the `TooNarrow` case of CN3. Holes still run before their outer boundary (CN3). The product
owner can state this in criterion 6.
