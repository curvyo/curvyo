# 0161 CNC output, slice 1 (MVP): UX notes

Revised 2026-11-28 by `docs/design-drafts/2026-11-machine-families/ux-unify.md` (one output page for all four families): the page stays the work area with the stock as a fixture (label included); the machine fields (Z travel, Spindle max, Safe height, Output, Connection) leave the Machine block for the Machines tab (supersedes 3 "Machine block"); Stock in the Fixtures tab and Tools in its own tab as 4 and 5 say; Side and Direction are Choice strips; Profile outside and inside are in the Select bar, rows, Output, Send and the machine controls footer in the Operations tab, Toolpath only at the page label, the play bar on the canvas, white laser fields. Withdraws the `plan.md` deviation "one panel in the Document tab".

ux-engineer, 2026-11-27. Slice 1 only (criteria 1 to 20). Written from `specification.md` (Fragen 418, 425 to 428 answered), `docs/design-drafts/2026-11-machine-families/plan.md`, and checked against the laser page notes listed in `specs/0159-cutting-plotter/ux-notes.md` (header), whose sections 2 (Add page, kind chip) and 3 (shared output-page rules) apply here unchanged, plus `specs/0112-test-grid-and-materials/ux-notes.md` 7 (Materials tab rows), `specs/0125-send-to-machine/ux-notes.md` and `specs/0118-machine-control/ux-notes.md` (Send button, machine controls footer). Customer rules: the family looks and works like the laser page; no invented colour, kind is shape; no operating tips or hint labels; no success notice except the export notice; hidden, not disabled; no popups in the panel. Items marked (Proposal) are vetoable; section 13 lists them as N1 to N16.

Terminology: "tool band" is the strip a tool of diameter d sweeps; "ink" is `--toolbar-icon`; "virtual GRBL" as the spec.

## 0. What is reused

| Part | Reused from | Change for CNC |
|---|---|---|
| Add page, chip, Machine block, Output row, Connection group | `0159` 2, 3; `0098`; `0029` 8; `0064` | CNC fields; the Connection group stays (Send is in scope) |
| Fixtures tab | `0113` 3; holder look `0102` 5.2 | one section "Stock" |
| Materials tab | `MaterialsTab.tsx`, `MaterialRow.tsx` (`0112` 7) | a **Tools** tab in its place and pattern |
| Material pick and the typed lines | `MaterialPick.tsx`, `OperationSettingsBlock.tsx` (`0028` 8.3, 8.4) | a **Tool** pick; "typed" feeds; "Reset to tool" |
| Select-bar group, rows, unfolded row | `0111` 3, 6; `0029` 6.1 | two kind buttons; Side and Direction segments |
| Toolpath view, play bar | `0029` 3 to 5 | tool band, plunge dots, pass readout |
| Output group, Send, machine controls | `0029` 6.3 to 6.6; `0125` 2 to 5; `0118` 2 to 5 | no Start pick; Frame and Focus point absent |

## 1. Principle

The page is the machine's bed with the stock on it. The maker assigns outlines to a profile outside or inside with one press, sees on the canvas which side the end mill will take away, and in the Toolpath view watches the tool centre run pass by pass with the width it cuts. The job is checked like a laser job, exported like one, and sent through the connector to the virtual GRBL exactly as to a real machine: nothing in the app knows the machine is simulated.

## 2. Tokens and symbols

One new token (added to `docs/design-system.md`):

| Token | Value | Use | Measured |
|---|---|---|---|
| `--tool-band` | `--cut` at 20 % (`#D8B3BC` over `--toolbar-bg`, `#E2BDC5` over `--canvas-bg`) | the strip the tool sweeps, in the Operations and the Toolpath view | band against the stock 1.38:1 and the canvas 1.39:1 (informational, like `--grid-minor`); the `--cut` centre line on the band 3.11:1; ink on the band 5.97:1. 15 % fails the cut line (it passes at 3.37:1 but the band against the stock drops to 1.28:1, barely seen); 25 % drops the cut line to 2.86:1 |

Existing: cutting moves `--cut`; rapids `--lane-line` dashed 4 on 3 (the laser's travel); plunge dots, rings, head and stock outline ink; stock ground `--toolbar-bg` (the holder board, `--cut` on it 4.30:1).

| Control | Symbol (16px custom on the 16 grid unless Lucide is named) |
|---|---|
| Profile outside | an 8px ink square inset 4px, around it a 1.5px `--cut` rounded square inset 1px |
| Profile inside | a 1.5px ink square inset 1px, inside it a 1.5px `--cut` rounded square inset 4px |
| Flat end mill (Tools tab rows, Tool pick) | a 4px-wide vertical shank with two diagonal flute strokes, flat end, ink |
| Tools tab | the flat end mill glyph |
| Stock (Fixtures tab on CNC pages) | the holder glyph of `0102` 4.0 |
| Plunge | a 5px ink dot with a 1px white casing (canvas only) |

## 3. The page and the Document tab (criteria 1 to 3)

- **Page** = the profile's work area (300 x 300 mm), not editable, origin as the Origin corner control (built control), default **top left** so the stock's top-left corner sits at the page origin as the spec's Words say (N13, M6).
- **Stock** on the canvas (N1): the `0102` 5.2 board look, opaque `--toolbar-bg`, 1.5px ink outline with casing, from the page origin; label "Plywood · 10 mm" 11px ink in its corner at the origin side, drawn when the stock is at least 80 device px wide. Not hit, not selectable; its corners and centre are snap targets.
- **Machine block**: profile row and state row (built); "Work area" line; Origin corner; **Z travel** (mm), **Spindle max** (rpm), **Safe height** (mm above the stock): the `0150` field look and commit rules, History "Edit Z travel", "Edit spindle max", "Edit safe height"; **Output** row "GRBL"; then the **Connection** group as built (`0064`). Laser-only rows absent (`0159` 3).

## 4. Fixtures tab: the Stock section (criterion 3)

- Header "Fixtures", CNC chip. One section, heading "Stock". Rows (bar number field look, label inside left, unit inside right):
  1. **Width** (112, "mm") and **Height** (112, "mm") in one row.
  2. **Thickness** (112, "mm").
  3. **Material** (228, a text field, 1 to 40 characters, "Plywood").
- Ranges: Width and Height 1 to the work area; Thickness 0.1 to the Z travel. Outside: the typed text stays, validation chip, nothing written (`0111` 6.4) (criterion 3 "refused with the limit"). Each commit one step "Edit stock".

## 5. Tools tab (criterion 4)

- Strip entry after Operations, in the Materials tab's place (Materials is laser-only): the end mill glyph, name and tooltip "Tools"; only on CNC pages. Header "Tools", CNC chip.
- **Rows** (`0112` 7.3 look, 40px, 48 coarse): line 1 glyph, name ("Flat 3.175 mm"; F2 or double-click renames, 1 to 40 characters), armed trash; line 2 (11px muted) "2 flutes · 800 mm/min · 12,000 rpm · 1 mm down". Creation order, no arrows.
- **+ Tool**: the `0113` 4.2 outline text button ("Tool"); adds "Flat 3.175 mm" with the defaults of criterion 4, unfolded, focus on Diameter; one step "Add tool". The name follows the diameter until the maker renames it (N2).
- **Unfolded tool** (body indented 16px, accent bar): Diameter (112, "mm") and Flutes (112, no unit) in one row; Feed (228, "mm/min"); Plunge (228, "mm/min"); Spindle (228, "rpm", up to the profile's Spindle max); Step-down (112, "mm"). One step "Edit tool" per commit.
- **Delete** (permit over refuse, N3): the armed trash of `0081`. A tool used by operations: the arming status reads "Press again to delete Flat 6 mm. Profile 2 then uses Flat 3.175 mm."; the second press removes it and moves those operations to the first remaining tool, one step "Remove tool". The last tool has no trash (hidden): a page always has a tool.

## 6. Operations: Profile (criteria 5 to 9)

### 6.1 Kind buttons

The `0111` 3 Operation group with **two** buttons, **Profile outside** and **Profile inside** (glyphs of section 2), both making the kind Profile with that Side (N4, M1). The reuse rule of `0111` 3.3 reads "the last Profile with that Side". Names and tooltips "Profile outside", "Profile inside".

### 6.2 Rows

- No Cuts last switch (absent); criterion 9 orders holes before their outline inside one operation, the list orders operations.
- Row of `0029` 6.1: line 2 run ring, "Profile · Outside · 3 items", time at the right (planner time, criterion 18); line 3 "Ø 3.175 mm, 10 mm deep, 4 passes, 800 mm/min" with "typed" after a typed feed ("800 mm/min typed").

### 6.3 Unfolded row (228px, `0111` 6.2 layout; the Tool pick where the laser has the Material pick)

1. **Tool** pick: `MaterialPick` look, full width x 28, end mill glyph, the tool's name at the right; inline list of the page's tools; one step "Use tool".
2. **Side**: `SegmentGroup` "Side", "Outside", "Inside"; one step "Edit settings".
3. **Depth** (112, "mm", 0.1 to the stock thickness, default the thickness) and a plain muted line at its right "4 passes" (the count from Depth and the tool's step-down, criterion 8; a state, not a tip).
4. **Direction**: `SegmentGroup` "Direction", "Conventional", "Climb"; default Conventional.
5. **Feed** (228, "mm/min") and **Plunge** (228, "mm/min"): the tool's values until typed; a typed field shows the `0028` 8.4 line "Tool 800" under it and the text button "Reset to tool" under the pair (N5).
- Commit, validation chip, one step "Edit settings" per commit (`0111` 6.4).

### 6.4 Operations view look (canvas)

- The item's outline: solid `--cut` 1 device px (it is cut along it).
- The **tool band** on the milled side: a strip one tool diameter wide (drawing scale) outside the outline for Outside, inside for Inside, in `--tool-band` (N6). The side is therefore visible before any toolpath is built.
- Not assigned, Output off, Show off as `0101` 13.2. Kind chips (`0101` 13.3) with the two glyphs.

## 7. Toolpath view and play bar (criteria 11, 17, 18)

- The `0029` 3 toggle on the page label. Drawn: the **tool centre path** solid `--cut` 1 device px over its **tool band** (the diameter, round caps and joins, `--tool-band`); **rapids** `--lane-line` dashed; a **plunge dot** where each pass starts; rings and arrowheads as `0029` 4. Passes at different depths share one drawn path (N7).
- **Head**: the `0029` ring, plus a 1px ink circle of the tool's diameter at the drawing's scale when it is at least 16 screen px across.
- **Play bar**: the laser's, time axis from the virtual GRBL's planner (criterion 18). Readout "2 min 3 s of 7 min 40 s · Z −5.00 mm" (the head's depth; "Z" and the minus as typed, tabular) (N8, M2).
- After the head: pale, as `0029` P7 (a cut path does not hide an earlier one the way stitches do).

## 8. Output group, Export, Send (criteria 10, 12 to 14, 19)

After the list, as `0029` 6.3: heading "Output", **machine line** "Generic GRBL router · GRBL", **checks**, **total**, the button row **Export job file** and, while the profile names a connector address, **Send to machine** (`0125` 2). No Start pick.

- **Checks** (`0029` 6.4 look and buttons):
  - block "2 items lie outside the stock." (criterion 10)
  - block "Profile 1 leaves the 300 × 300 mm work area." (the toolpath, criterion 10)
  - block "Profile 2 uses another tool. One tool per job for now." (criterion 4)
  - block "Profile 1: 1 item is too narrow for this tool." (criterion 7; the operation name unfolds the row, as `0029` P15, M3)
  - warn "Profile 1: 1 open path is not milled." (criterion 6)
  - warn "The machine is Generic GRBL router; its limits may not match yours."
  - the not-assigned, hidden and Output-off warns of `0029` 10.
- **Total**: "About 7 min 40 s", then "Runs 2 operations, 3 items." and "Cut 2.41 m · rapid 1.10 m · 8 passes".
- **Export**: save dialog "<project> - <page>.nc", file type "GRBL G-code (.nc)" (M4); notice and errors as `0159` 3. No step.
- **Send and machine controls**: the `0125` button, disabled states and state texts, and the `0118` footer (state line, progress, Pause, Stop, Home, Jog) as built; **Frame** and **Focus point** are absent on CNC pages (laser-only, `0159` 3). Done reads as for the laser.

## 9. The virtual GRBL in the app (criteria 15 to 19)

Nothing new in the app. The maker (or the demo) starts the virtual GRBL from the command line, starts the connector on its port and pairs it as any GRBL machine (`0127`, `0064`); the Connection group, the state line and the footer then behave as with real hardware, including GRBL's own error texts on a refused line (`0125` 5). The real-time option is a command-line flag, not a control (N9). No "simulated" badge: the profile name and the connector's device name already say which machine it is.

## 10. Keyboard

As `0159` 10, plus: Stock fields (Tab order Width, Height, Thickness, Material); Tools tab rows (list keys of `0112` 9), + Tool, the unfolded fields; Tool pick (Enter opens, arrows, Enter picks, Escape closes); Side and Direction segments (one Tab stop each, Left and Right). No new shortcut.

## 11. Texts (each at most 70 characters)

| Text | Kind |
|---|---|
| "2 items lie outside the stock." / "Profile 1 leaves the 300 × 300 mm work area." | block |
| "Profile 2 uses another tool. One tool per job for now." | block |
| "Profile 1: 1 item is too narrow for this tool." | block |
| "Profile 1: 1 open path is not milled." | warn |
| "The machine is Generic GRBL router; its limits may not match yours." | warn |
| "Width is 1 to 300 mm." / "Thickness is 0.1 to 60.0 mm." / "Depth is 0.1 to 10.0 mm." / "Diameter is 0.1 to 25.0 mm." / "Spindle is 0 to 12,000 rpm." | validation chip (limits from the copy and the stock) |
| "Press again to delete Flat 6 mm. Profile 2 then uses Flat 3.175 mm." | arming status |
| "Exported Parts - CNC 1.nc." | export notice |

## 12. Accessible names, undo steps, states

- Names: Stock fields "Stock width, millimetres" and so on; Tools tab "Tools"; tool rows "<name>, <line 2>"; Tool pick "Tool"; radiogroups "Side", "Direction"; kind buttons "Profile outside", "Profile inside"; readout valuetext "2 min 3 s of 7 min 40 s, Z minus 5 millimetres".
- Steps: "Add CNC page", "Edit stock", "Add tool", "Edit tool", "Rename tool", "Remove tool", "Assign to <name>", "Use tool", "Edit settings", "Reset to tool", "Edit Z travel", "Edit spindle max", "Edit safe height". None for views, slider, Play, picks opened, export.
- **Damaged values** (criterion 20): read as default with a notice.
- **Large**: 500 items with 10 passes: the Toolpath view draws one path per item (passes shared), the band as one stroked path; the planner time is computed off the frame, the readout shows "–" until it arrives (N10).

## 13. Decisions for veto (Proposal)

- **N1** Stock in the holder-board look, labelled with material and thickness.
- **N2** A new tool is named from its diameter until renamed.
- **N3** Deleting a used tool moves its operations to the first remaining tool; the last tool cannot be deleted (no trash).
- **N4** Two Select-bar buttons, Profile outside and Profile inside.
- **N5** Feed and Plunge follow the tool until typed, with "Tool 800" and "Reset to tool" (the material pattern).
- **N6** The Operations view shows the tool band on the milled side.
- **N7** Passes share one drawn path; the readout gives the depth.
- **N8** Play-bar readout adds the head's Z.
- **N9** No app control or badge for the virtual GRBL.
- **N10** Readout "–" until the planner time arrives.
- **N11** Tools tab in the Materials tab's place, CNC pages only.
- **N12** `--tool-band` at 20 % of `--cut`.
- **N13** Origin corner default top left (the stock's corner at the page origin).
- **N14** Depth shows its pass count beside it.
- **N15** Frame and Focus point absent on CNC pages.
- **N16** No Cuts last switch on CNC pages.

## 14. Mismatches for the product owner and the architect

- **M1 (criterion 5):** one kind Profile with a Side pick stays; the Select bar offers it as two buttons (N4). Architect: what "Outside" means for an item with holes (criterion 6 says the part keeps its size, so its holes must keep theirs: holes are offset inward). Please state it in the criterion.
- **M2 (criterion 11):** "cutting moves in one colour per operation" conflicts with the accepted laser rule (no operation colours; rings carry identity). Proposal: every cutting move `--cut`, operations told apart by their run rings and the selected row; the readout shows the head's Z.
- **M3 (criterion 7):** "Too narrow for this tool" on the row: shown as a block check whose operation button unfolds the row (`0029` P15), not as a row line.
- **M4 (criterion 12):** file name `<page name>.nc` differs from the laser's "<project> - <page>" (as `0159` M6). The laser's GRBL export uses `.gcode`; `.nc` here as the spec says.
- **M5:** the spec's "UX notes" section should point to this file.
- **M6 (Words "Stock", architect):** the stock's top-left corner is at the page origin and Z = 0 on its top; a real GRBL router's machine origin is usually front left with Y growing away from the operator. The page keeps the spec's top-left (N13); the export transform from page coordinates (Y down) to G-code work coordinates (Y up, zero at the stock corner the maker touches off) is the architect's to state.

## 15. Design system

Token `--tool-band` added now. Rows at acceptance: "Stock (`0161`)", "Tools tab (`0161`)", "Tool pick and typed feeds (`0161`)", "Profile look and tool band (`0161`)", "Play bar Z readout (`0161`)".

## 16. Implementer checklist

- Add page CNC (`0159` 2); page = work area; Machine block CNC fields with the Connection group.
- Fixtures tab Stock section; stock drawn from the origin.
- Tools tab: rows, + Tool, unfolded fields, armed delete with reassignment, no trash on the last tool.
- Select bar Profile outside and inside; rows; unfolded Tool pick, Side, Depth with pass count, Direction, typed feeds.
- Operations view: outline plus tool band on the milled side; Toolpath view: centre path, band, rapids, plunge dots, head with tool circle, Z readout.
- Output group checks, total, Export ".nc", Send and the footer without Frame and Focus point.
