# CNC output: a CNC output page with stock and a tool, Profile outside and inside, GRBL G-code and a virtual GRBL that runs it

Status: Ready (slice 1 MVP; later slices stay Draft until their criteria are written) (2026-11-27; `adrs.md` and `ux-notes.md` in, UX mismatches applied as Proposals; earlier: Draft (2026-11-27; customer answers to Fragen 418, 425, 426 and 427 in; Frage 428 answered A (2026-11-27), see `0159`; waits for `adrs.md` (ADR 0097 written) and UX notes))
Priority: Should
Origin: Customer request 2026-11-27 (translated): three new machine families, CNC third; the customer has **no CNC machine**, so the job is tested against a simulator (Frage 426 C: virtual GRBL, planner time, 2.5D height map, a 3D view). Customer change of 2026-11-27: **first a thin end-to-end slice through all three families, then everything else.** Plan and answers: `docs/design-drafts/2026-11-machine-families/plan.md`. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md), `0098-page-kinds`, `0101` (operations and assignment), `0029-laser-job-preview-and-output` (output group, Toolpath view and play slider, Export job file, output format on the profile, criterion 39), `0150-operation-always-has-numbers`, `0064-machine-connector` and `0127-connector-grbl-serial` (the connector, its GRBL streaming checks and the `grbl_pty` simulator the virtual GRBL grows from), `0125-send-to-machine`, `0116-library-sync`.
Related: `0139-laser-cut-path-options` (tabs, later), `0117-centre-lines` (V-carve, later), `0159-cutting-plotter` (its CutcutGo slice is tested on this virtual GRBL), `0118-machine-control`.

## Order across the three families

`0159` slice 1, then `0160` slice 1, then this spec's slice 1; after that every later slice of the three, in the order the lead sets (customer change 2026-11-27; Frage 427 C).

## User value

As a maker who wants to mill parts I want to put a drawing on a page that shows my stock, cut its outlines out with an end mill at the right side of the line, and see the job run on a simulated GRBL machine before I own or touch a real one, so that the first real cut is not a surprise.

## What other tools do

Carbide Create, Easel, Estlcam, Vectric VCarve, Fusion 360: profile, pocket, drill, V-carve; simulation of removed material; post-processors per controller. LightBurn: none for spindles. **What we do better (Proposal):** the same drawing feeds the mill and the laser, the plotter and the embroidery pages; the job is checked by a virtual GRBL that answers like the real firmware (it rejects what real GRBL rejects), and the preview and the file come from the same moves.

## Words

- **CNC page:** an output page with process id `curvyo.cnc`.
- **Stock:** the block to mill: width, height, thickness, material name; placed with its top-left corner at the page origin; Z = 0 on the stock top (Proposal). How page coordinates (Y down) map to G-code work coordinates (Y up, zero at the stock corner the maker touches off) is the architect's, see [`adrs.md`](adrs.md) (UX M6).
- **Tool:** an end mill in the page's tool list: kind (flat end mill in slice 1), diameter, flutes, feed rate, plunge rate, spindle speed, depth per pass (step-down).
- **Profile:** an operation that cuts along an outline down to a depth, in passes. **Outside:** the tool centre runs outside the outline by the tool radius (the part keeps its size). **Inside:** inside by the radius (a hole keeps its size).
- **Virtual GRBL:** a GRBL 1.1 controller in Rust that the connector talks to over a pseudo-terminal exactly as to a real machine.

## Slices

- **Slice 1: MVP** (criteria 1 to 20, below): CNC page with a generic GRBL router profile, stock, one tool, Profile outside and inside, toolpath preview, GRBL G-code, the virtual GRBL running the job through the connector with planner time. One `format_version` bump, its number assigned at the final PR (Frage 431 A).
- **Slice 2: operations** (Later; criteria before Ready): Profile on the line; tabs on profiles (from `0139`, with a tab height); Pocket (offset, step-over); Drill (peck); V-carve (medial axis, depth from the bit angle; reuses `0117`). Ball end mill, V-bit and drill in the tool list.
- **Slice 3: dialects** (Later; Frage 425 A): grblHAL, Mach3, Mach4 (M6 T, G43 H, canned cycles), LinuxCNC (M6 T, G43, G64 P, canned cycles); arcs as G2/G3 with IJK; golden files per dialect.
- **Slice 4: tool changes** (Later): grouping by tool, change order, GRBL manual change (M5, retract, M0 pause with a comment, re-zero), probe with G38.2.
- **Slice 5: material simulation** (Later; Frage 426 C): a 2.5D height map of removed material with play and scrub, warnings (rapid into stock, out of bounds, depth below the stock), and a 3D view of it.
- **Slice 6: tool library** (Later; Fragen 418 A, 433 A): tools and their feeds per material in their own `tools/` library (records of ADR 0097), synced as `0116`.

## Acceptance criteria

### Slice 1: MVP

Tolerances: toolpath geometry 0.01 mm; flattening at most 0.01 mm from the exact curve; written numbers with 3 decimals in mm.

**Page, machine, stock, tool**

1. Given Add page, then it offers **CNC**; a new CNC page takes the profile **Generic GRBL router** (Proposal) and its page size is the profile's work area, not editable. Test: a page of kind Output with process `curvyo.cnc`.
2. Given the built-in profile Generic GRBL router, then it has output format GRBL, work area 300 x 300 mm (Proposal), Z travel 60 mm, spindle 0 to 12000 rpm, rapid and maximum feed and acceleration per axis (GRBL defaults, Proposal), safe height 5 mm above the stock. It can be copied and edited like a laser profile (record `kind = "cnc"`, ADR 0097) and syncs as `0116`.
3. Given a CNC page, then it has one **Stock** (width, height, thickness, material name; default 200 x 150 x 10 mm, "Plywood", Proposal) drawn on the page from the page origin, and Stock larger than the work area is refused with the limit. Each change is one undo step.
4. Given a CNC page, then it has a tool list with one tool by default: flat end mill 3.175 mm (1/8 in), 2 flutes, feed 800 mm/min, plunge 300 mm/min, 12000 rpm, step-down 1 mm (Proposal). The maker can edit it and add more tools; every value has a range and is refused outside it. Slice 1 runs a job with one tool only: a page whose operations use two different tools blocks Export with "One tool per job for now" (Proposal; tool changes are slice 4).

**Operation**

5. Given a CNC page, then its Operations tab offers the kind **Profile** (meaning: Frage 428) with a **Side** pick (Outside, Inside; default Outside), **Depth** (0.1 mm to the stock thickness, default the stock thickness), the tool, and **Direction** (Climb, Conventional; default Conventional, Proposal), with assignment as on a laser page (`0101`). It always has numbers (as `0150`): the tool's feeds unless typed on the operation.
6. Given Profile Outside on a 50 x 30 mm rectangle with the 3.175 mm tool, then the tool centre path is the outline offset outward by 1.5875 mm within 0.01 mm, with round corners of that radius; Inside on a 20 mm circle gives a circle of 16.825 mm diameter. Profile takes closed outlines only; an open path in a Profile operation is listed with "Open path: not milled" and skipped (Proposal). For an item with holes, Outside mills the **outer contour only**, offset outward; its holes are milled as Inside (offset inward), so the part and its holes keep their size **(UX M1)**. Test: a 50 mm square with a 20 mm round hole gives the outer path offset outward and a 16.825 mm circle.
7. Given an Inside profile on an outline whose inner offset vanishes (a slot narrower than the tool), then the item is skipped and the output group shows a block check "Too narrow for this tool" whose operation button unfolds the row (as `0029`), and Export is blocked until it is fixed or removed **(Proposal, UX M3)**.
8. Given Depth d and step-down s, then the profile runs ceil(d / s) passes at depths of equal steps, the last at exactly -d. Test: depth 10, step-down 3: passes at -2.5, -5, -7.5, -10.
9. Given several items and operations, then operations run in list order; inside one, inner outlines (holes) are milled before the outline that contains them (Proposal, so a part is cut free last), otherwise nearest-next.
10. Given a page with items outside the stock or a toolpath outside the work area, then the output group shows the check and Export is blocked (as the laser's work-area check).

**Preview**

11. Given the Toolpath view (`0029`), then it shows the tool centre paths from the same builder that writes the file: every cutting move in the cut look (red, the accepted laser rule: no operation colours), operations told apart by their numbered run rings and the selected row **(UX M2)**, rapids dashed, plunges as a dot, the tool width as a faint band on the cut (Proposal), the head's Z and the current pass in the readout, and the total time from the virtual GRBL's planner (criterion 18). The play slider moves a tool marker along the job.

**Output**

12. Given output format GRBL, when Export job file is pressed, then the app saves `<project> - <page>.nc` (the laser export's rule, as `0159` criterion 11): a comment header (page, tool, stock), `G21 G90 G17 G94`, `M3 S<rpm>`, a rapid to safe height, then per pass: rapid over the start, plunge at the plunge rate to the pass depth, cutting moves at the feed rate as `G1` lines (flattened within 0.01 mm), retract to safe height; at the end `M5`, rapid to safe height, `M30`. No `M6`, no tool length offset, no canned cycles (GRBL 1.1 has none of them). Test: golden files `cnc_grbl_profile_outside.nc`, `cnc_grbl_profile_inside.nc`, `cnc_grbl_two_parts.nc` in `tests/fixtures/`.
13. Given every line written, then it is at most 79 characters and contains none of `?`, `!`, `~` outside comments (the connector's checks of `0127`); numbers have at most 3 decimals and no exponent. Test: all golden lines pass the connector's line check.
14. Given an unchanged page, then two exports are byte-identical.

**Virtual GRBL**

15. Given `cargo run -p curvyo-connector-io --example grbl_virtual` (name: architect), then it opens a pseudo-terminal, prints its path, and behaves as GRBL 1.1 on it: welcome line, `ok` and `error:<n>` replies, `?` status reports with state, machine position and feed, `!` hold, `~` resume, `0x18` reset, `$$` settings, homing `$H`, alarm states and `$X`. The connector (`curvyo-connector-server --serial <path>`) runs a job against it as against a real machine (`0127`).
16. Given a line real GRBL 1.1 rejects, then the virtual GRBL answers with the same error number: an unsupported command such as `M6` or `G81` (error 20), a move beyond the soft limits with soft limits on (alarm 2), a missing feed rate on `G1` (error 22), a line over 80 characters (error 11 or the connector refuses it first). Test: one case per error against a table taken from the GRBL 1.1 documentation.
17. Given a job streamed to it, then it tracks position and spindle state and reports the moves so a test can compare them with the job's moves; the final position equals the last move within 0.001 mm.
18. Given a job, then the virtual GRBL's planner computes the run time with the profile's acceleration and maximum feeds (trapezoid profiles, junction deviation as GRBL's default 0.01 mm), and the Toolpath view's time (criterion 11) equals that time within 1 %. The simulation runs faster than real time (Proposal: as fast as the host allows, with a real-time option for the demo).
19. Given a CNC page with the connector paired to the virtual GRBL, then Send to machine (`0125`) streams the job and the Machine controls of `0118` show state, line and progress, as for the laser. Test: end to end, `cnc_grbl_two_parts.nc` completes with `Done { Completed }`.

**Format**

20. Given a project with a CNC page, then the file stores the page's process, profile copy, stock, tool list and Profile operations under a `format_version` assigned at the final PR (Frage 431 A; files saved by branch-push builds may not open after the merge); the build before refuses it with the newer-version message; a file of the format before opens unchanged. Golden: `cnc_page_v<N>.curvyo` and `cnc_page_damaged_v<N>.curvyo` (tool diameter 0, depth beyond the stock: read as default with a notice).

## What changes accepted behaviour

- Add page gains the CNC kind; the profile record gains `kind = "cnc"` (ADR 0097).
- The connector's `grbl_pty` simulator gains a sibling (or grows) into the virtual GRBL; its laser tests stay as they are.

## Out of scope

- 3D (non-2.5D) machining, 4th axis, adaptive clearing, rest machining.
- Probing, tool changes, more than one tool per job (slice 4).
- Any real machine test (the customer has none).
- Spindle control beyond M3/M5 and S; coolant.

## Customer answers 2026-11-27 (machine families, all three specs)

- Frage 430 (LICENCE-CONSEQUENCE) A: Curvyo stays AGPL-3.0-or-later for good, commercial use included; the Ink/Stitch port as a GPL-3 crate is accepted.
- Frage 431 (MF-BRANCH-FILES) A: files saved by builds of the branch pushes may not open after the merge; the `format_version` numbers are assigned at the final PR.
- Frage 432 (MF-EMB-READ) A: an imported embroidery file becomes paths with a Manual stitch; viewing it is what matters, editing is secondary.
- Frage 433 (MF-TOOLS-DIR) A: CNC tools get their own `tools/` library, synced as `0116`.
- MF-REC-KIND: default A (technical, architect): the record key stays `kind` (`kind = "cutter"`, `"embroidery"`, `"cnc"`), no `family` key ([ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md)).

## UX notes

See [`ux-notes.md`](ux-notes.md) (mismatches M1 to M5 applied above as Proposals; M6 in `adrs.md`).

## Links

Requirements: R-MFG-001 to R-MFG-003, R-MFG-009, R-MFG-011, R-MFG-CNC-001, R-MFG-CNC-003 to R-MFG-CNC-009 (R-MFG-CNC-002 in slice 2), R-MAT-001, R-MAT-005. Plan: `docs/design-drafts/2026-11-machine-families/plan.md`. ADR: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md). PR: -
