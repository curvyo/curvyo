# Plan: cutting plotter, embroidery, CNC (draft, 2026-11-27)

Customer request of 2026-11-27: three new machine families, built one after the other, one PR at the
end (customer's choice, overrides "one PR per slice"). Branch `story/machine-families`. Implementation
on Opus. `cargo clean` done (418 GiB freed); the full gate runs once per family plus once before the PR.

Numbers: specs 0159 (plotter), 0160 (embroidery), 0161 (CNC); ADRs from 0097; Fragen from 416.

## 0. Shared groundwork (before the plotter, invisible)

- ADR 0097 "second machine family": what moves out of `curvyo-laser-core` (record envelope, library
  merge, machine profile envelope with a `family` field, material records per family) into
  `curvyo-library-core`; already planned in ADR 0038 / 0055 §7 and `docs/technical-debt.md`.
- Output page stays one page kind (`PageKind::Output`), new process ids `curvyo.cutter`,
  `curvyo.embroidery`, `curvyo.cnc` (ADR 0042 already foresees this).
- One project `format_version` bump per family (68, 69, 70 if nothing else merges first).
- A shared job timeline/simulation shape only where two families really share it (rule §5).

## 1. Cutting plotter (spec 0159, crates `curvyo-cutter-core`, device part in `curvyo-connector-io`)

What the research says:
- Cricut Maker 3: the host protocol is encrypted, no open driver; direct drive is not realistic. The
  honest route is export (SVG, at mat scale and mat origin) for Design Space. Custom firmware
  (CutcutGo, GRBL) exists but reflashes the machine.
- Open machines: HPGL (Vevor, USCutter, Skycut, Liyu, Roland via HP-GL/CAMM-GL), GPGL (Graphtec,
  Silhouette Cameo 4/5 and Portrait over USB; documented by the Inkscape Silhouette extension).

Slices:
1. Plotter output page: machine profiles (Cricut Maker 3, Silhouette Cameo 4/5, Portrait, a generic
   HPGL cutter, Graphtec), work area, **mats as fixtures** (12x12 in, 12x24 in, LightGrip, StandardGrip,
   StrongGrip, FabricGrip) with the **printed 1 in grid on/off** on the output page, matched to the mat
   (origin top left, cut area 11.5 x 11.5 in; exact offsets measured on your mat, see Frage 420).
2. Operations (tools): Cut (knife: fine-point, deep-point, rotary, knife blade with passes), Score/
   Crease ("falzen": scoring stylus, scoring wheel), Perforate (dashed cut), Draw/Write (pen; with
   single-line fonts from `0076`), Deboss, Engrave, Wavy, Foil. Per tool: pressure, speed, passes,
   blade offset (drag-knife corner compensation), overcut; tool change as a pause (clamp A/B).
3. Cutter materials: a material library like the laser's (per material and tool: pressure, speed,
   passes), synced like `0116`; a test cut (small square plus triangle) instead of a test grid.
4. Output: SVG/DXF export at mat scale for Cricut Design Space and Brother ScanNCut; HPGL and GPGL
   writers with golden files; sending over USB/serial through the connector for HPGL/GPGL machines.
5. Weed lines and weeding boxes (vinyl), kiss-cut vs. cut-through as a tool setting.

## 2. Embroidery (spec 0160, crate `curvyo-embroidery-core`)

What the research says:
- W6 EU7 (W6 N 5000 Exklusive / N 8000 Exklusive): field 150 x 250 mm, hoops 150 x 250 and 100 x 100
  (to be confirmed by you), files by USB stick; format probably JEF (Janome based), not confirmed.
- Ink/Stitch is GPL-3; Curvyo is AGPL-3.0-or-later, so a port with attribution is licence compatible
  (GPLv3 §13); pyembroidery (formats) is MIT. See Frage 422.

Slices:
1. Stitch model (stitches, jumps, trims, colour changes, stops, tie-in/tie-off) and an embroidery
   output page with hoops as fixtures (EU7 hoops), thread colours, stitch simulation (play, speed,
   stitch count, thread length, time).
2. Line stitches: running, bean, manual, zigzag, E-stitch, ripple.
3. Satin column (two rails or one path with variable width from `0110`), underlays (centre walk,
   contour, zigzag), pull compensation, short stitches.
4. Fills: tatami (angle, row spacing, max stitch length, staggers), offset/contour fill (the
   "Füllung mit Offset"), guided, meander, circular, linear gradient; fill underlay; end rows.
5. Stitch order, travel, jumps vs. trims, lock stitches, start/end points, path direction (R-MFG-EMB-005),
   right to left (see Frage 425).
6. Patterns: fill patterns and pattern stitches ("schöne Muster").
7. Embroidery fonts (lettering): a font format, a starter set, letters routed for few trims.
8. Formats: write JEF, DST, EXP, PES/PEC, VP3, XXX, U01 first; read the same; golden files each.
9. Embroidery materials: fabric presets (density, underlay, pull compensation, stabiliser), synced.

## 3. CNC (spec 0161, crate `curvyo-cnc-core`, simulator in `curvyo-connector-io`)

Slices:
1. CNC output page: machine profile (work area, spindle range, Z travel), stock (size, thickness,
   material), tool library (end mill flat/ball, V-bit with angle, drill; diameter, flutes, feeds).
2. Operations: Profile (inside/outside/on line, step-down, tabs from `0139`), Pocket (offset, step-over),
   Drill (peck), V-carve (medial axis, depth from bit angle; reuses `0117` centre lines). Toolpath preview.
3. G-code per dialect: GRBL (no M6: M5, retract, M0 pause, comment, re-zero), Mach3, Mach4 (M6 T,
   G43 H, canned cycles), LinuxCNC (M6 T, G43, G64 P, canned cycles); arcs as IJK only; golden files.
4. Tool changes: grouping by tool, change order, manual change with pause and probe (G38.2).
5. Simulation: a virtual GRBL controller in Rust (speaks the GRBL serial protocol to our connector,
   rejects unsupported codes like real GRBL, planner with acceleration for time) plus a 2.5D height-map
   of removed material with play/scrub and warnings (rapid into stock, out of bounds).

## Order and gates

0 -> 1 (plotter) -> 2 (embroidery) -> 3 (CNC); per family: PO spec, architect ADR, UX notes, plan,
build test-first, tester; full gate once per family; one PR with all three at the end.
Biggest risks: size (each family is several weeks of normal work), embroidery quality versus
Ink/Stitch, unverifiable hardware (Cricut only by export; no CNC; W6 file format unconfirmed).

## Customer answers (2026-11-27)

- 416 B, and CutcutGo supported too, but the customer does not reflash his machine (CutcutGo output is
  built and tested only against golden files and the GRBL simulator, never on hardware). B means a
  reverse-engineering slice of the Maker 3 host protocol; it needs USB captures from the customer's
  machine and may fail (encrypted protocol); the SVG export for Design Space is built either way.
- 417 A (Silhouette Cameo 4/5, Portrait over GPGL, generic HPGL); the customer has no other plotter,
  so only the Cricut is tested on hardware.
- 418 A (libraries for cutter materials, embroidery fabrics, CNC tools; synced).
- 419 A (mats from published sizes, matched to a photo of the customer's mat).
- 420 DST (the W6 EU7 reads DST); hoops 150 x 250 and 100 x 100 mm as default until confirmed.
- 421 C (port Ink/Stitch into its own crate marked GPL-3, attribution per file).
- 422 C and 423: lettering comes from the normal font library (TrueType and single-line fonts, plus
  Ink/Stitch fonts added to that library where their licence allows); no extra font things in the
  embroidery view. "Right to left" is read as stitch direction (423 A, default) (Proposal).
- 424 A (write and read JEF, DST, EXP, PES/PEC, VP3, XXX, U01).
- 425 A (GRBL, grblHAL, Mach3, Mach4, LinuxCNC).
- 426 C (virtual GRBL, planner time, 2.5D height map, plus a 3D view).
- 427 C (core of each family first; a branch push after each family to look at; one PR at the end).
- 430 LICENCE-CONSEQUENCE A: Curvyo stays AGPL for good, also for commercial use; the Ink/Stitch port is accepted.
- 431 MF-BRANCH-FILES A: files saved by branch-push builds may not open after the merge; format numbers are assigned at the final PR.
- 432 MF-EMB-READ A: an imported embroidery file becomes paths with a Manual stitch; viewing matters, editing is secondary.
- 433 MF-TOOLS-DIR A: an own `tools/` library for CNC tools, synced.
- MF-REC-KIND: default A (technical): the record key stays `kind` (`kind = "cutter"` etc.), ADR 0097.

## Order change (customer, 2026-11-27)

First an early end-to-end thin slice through all three families, then everything else:
1. Groundwork needed by the three MVPs only.
2. MVP plotter: output page, Cricut Maker 3 mat with grid toggle, Cut and Draw, SVG export for
   Design Space and an HPGL file.
3. MVP embroidery: output page, EU7 hoop, running stitch, tatami fill, simple satin, stitch preview,
   DST export.
4. MVP CNC: output page, stock and one tool, Profile inside/outside, GRBL G-code, virtual GRBL
   simulator with toolpath preview.
5. Branch push, customer tries the three MVPs.
6. Then the remaining slices of each family (order agreed after the MVP demo); one PR at the end.
