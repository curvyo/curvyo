# Machine families: one output page for laser, plotter, embroidery and CNC (UX revision)

ux-engineer, 2026-11-28. Revises `specs/0159-cutting-plotter/ux-notes.md`, `specs/0160-embroidery/ux-notes.md`, `specs/0161-cnc-output/ux-notes.md` and the laser notes named in section 11. The customer tried the three MVPs (branch `story/machine-families`, commits `487bf892`, `89513959`, `a508d830`) and asked for all four families, the laser included, to work the same way. His seven points are sections 2 to 8. Checked against the built laser front end: `PageFrames.tsx`, `PlayBar.tsx`, `RingLabels.tsx`, `SelectToolbar.tsx` with `OperationKindGroup.tsx`, `panel/OperationsSection.tsx`, `OperationRow.tsx`, `OperationBody.tsx`, `OperationSettingsBlock.tsx`, `OutputGroup.tsx`, `OutputChecks.tsx`, `MaterialPick.tsx`, `OutputPick.tsx`, `MaterialsTab.tsx`, `MaterialRow.tsx`, `EntryField.tsx`, `PresetStrip.tsx`, `ui/switch.tsx` (`PanelSwitch`), `workbench/MachineBlock.tsx`, `MachineProfileList.tsx`, `MachineSizeOffer.tsx`, `ConnectorGroup.tsx`, `FixturesTab.tsx`, `panel/AddPageGroup.tsx`, `panel/PanelStrip.tsx`, and `curvyo-editor-wasm/src/session/panel_tabs.rs`, `pages_json.rs`. Customer rules apply unchanged: no popups in the panel, no hint labels, hidden not disabled, no success notice except the export notice, plain fields (not sliders) in the panel, kind is shape. Choices marked (Proposal) are vetoable at the demo and listed as U1 to U20 in section 12. The customer answered Frage 434 (C) and Frage 435 on 2026-11-28; section 13 records both, and sections 4 and 6 follow them. The rules for machines that are not in the local library and for a clean sync are ADR 0098's (`docs/adr/0098-one-output-page-for-every-machine-family.md`); this file defers to it.

## 0. What this withdraws

All three `plan.md` files list the same deviation: the family's Machine, Mat/Hoop/Stock, Tools, Operations and Output sections sit in **one panel in the Document tab** (`PlotterPanel.tsx`, `EmbroideryPanel.tsx`, `CncPanel.tsx`, drawn by early returns in `ManifestTab.tsx`). Each panel also has its own `FIELD`, `SEGMENT` and `BUTTON` styles, its own kind buttons, its own Toolpath switch and, for embroidery and CNC, its own play bar with a native range slider. This revision withdraws that deviation. The three panels, the early returns and the private styles are **deleted in the same PR** (customer rule: obsolete code goes in the PR that obsoletes it). Everything they showed moves to a laser component as listed in section 9.

Not withdrawn: the family notes' contents (mat geometry and tokens, hoop look, stitch look, tool band, checks, texts, accessible names, undo steps). They stay valid where this file does not say otherwise.

## 1. Principle

An output page is the machine's work area. Its panel has the same tabs in the same order for every family. Operations are assigned from the Select bar. The job is checked and exported at the foot of the Operations tab. The Toolpath view is switched at the page label and played on the canvas. Machines are edited in their own tab, apart from any page. A family differs only in its words, glyphs and numbers, never in where things are.

## 2. The page is the work area (point 1)

### 2.1 How the laser does it (the rule all four follow)

A laser page's size is the copy's work area (ADR 0042 §6). Only the profile pick and Reset from profile write it. The page resizes about the copy's origin corner, placements keep their global place, and fixtures are recomputed. Fixtures (bed grid, holders, pins) are drawn **inside** the page, because the bed is the work area. Nothing frames a smaller area inside the page. The page frame (`PageFrames.tsx`, 1px or 2px ink outline) is the work area's edge, and the rulers count from the page origin.

### 2.2 Per family

| Family | Page | Drawn outside the page (pasteboard) | Drawn inside the page | Who writes the size |
|---|---|---|---|---|
| Laser | the copy's work area (unchanged) | nothing | fixtures (`0102`, `0113`) | profile pick, Reset |
| Plotter, mat profile | **the active mat's cut area** (U1): 12 x 12 in mat: 292.1 x 292.1 mm; 12 x 24 in mat: 292.1 x 596.9 mm. Origin at the cut area's top-left corner | the mat: sheet, notch, the printed field's outer 6.35 mm band, the cm and inch numbers, the grip word (`0159` 5.1 to 5.4 geometry and tokens unchanged) | the mat at full grip colour and its printed grid and centre cross (they run on under the page edge); no veil, no cut-area line (the page frame is that line) | profile pick, Reset, **Set mat** |
| Plotter, matless (Generic HPGL) | the work area 600 x 1000 mm, Origin corner (unchanged) | nothing | nothing | profile pick, Reset |
| Embroidery | **the active hoop's sewing field** (U2): 150 x 250 mm or 100 x 100 mm. No 20 mm margin | the hoop frame of `0160` E2: the 12 mm band 8 mm outside the field, rounded corners 14 and 10 mm, the left mounting tab (the MVP's square, tabless frame is replaced) | the template marks: a 6 mm tick inward at each side's middle, the 10 mm centre cross | profile pick, Reset, **Set hoop**, which resizes **about the page centre** (the hoop centre is the DST origin) (U3) |
| CNC | the work area 300 x 300 mm (unchanged), Origin corner | nothing | the **stock as a fixture**: the `0102` 5.2 holder-board look from the page origin, label "Plywood · 10 mm" in its origin corner when it is at least 80 device px wide (the MVP lacks the label) | profile pick, Reset |

- **Draw order** for the plotter, bottom to top: pasteboard, sheet with its printing, page (mat colour inside), grid and cross over it, items, origin marker, selection. On a plotter page with a mat, and on an embroidery page, the document background is not drawn inside the page. On a plotter page the mat is the ground. On an embroidery page the fabric is the page's own `--canvas-bg`.
- **Checks** keep their texts. "outside the mat's cut area", "outside the hoop" and "outside the work area" now all mean "outside the page".
- **Snap targets**: the page corners, side middles and centre on every family, as for any page. Plotter: the grid and the centre cross only while snapping to the grid is on (`0159` C11). CNC: the stock's corners and centre.
- **Export origin** does not follow the page origin. The Design Space SVG still counts from the mat origin, 6.35 mm left of and above the page origin. The DST still counts from the hoop centre, which is the page centre. G-code still counts from the stock corner. The architect states this (section 10, A1).

## 3. Add page: kind symbols (point 2)

- The Kind switch in `AddPageGroup.tsx` stays a `role="radiogroup"` named "Kind" with one Tab stop, Left and Right moving and selecting. Each segment is now a **32 x 32px symbol button** (36 x 36 coarse) with a **24px Lucide glyph** at 1.5px absolute stroke (U4). There is **no word**. The kind word is the accessible name (`aria-label`) and the whole tooltip (Radix, 400 ms, `side="bottom"`).
- **One row, 4px apart**: 6 x 32 + 5 x 4 = 212px fits in the group's 228px content width, so `0159` C1 "two to a row" is withdrawn. A seventh kind (248px) would wrap whole buttons onto a second row (the Bar overflow rule).
- Selected looks like the `0098` segment (`--toolbar-icon-active-bg`, `-fg`). Unselected: no fill, hover `--editor-accent-hover`, the focus-visible ring.
- Glyphs: Drawing `PenTool`, Sketch `DraftingCompass`, Laser `Zap`, Plotter `Slice`, Embroidery `Spool`, CNC `Drill` (all already in `workbenches/iconNames.ts`).
- The caption "Kind" stays (it names the control). The Machine list, Cancel and Add are unchanged. The machine list now also remembers the last profile per kind (`0159` C3, missing in the MVP).

## 4. The Machines tab (point 3)

### 4.1 On the page: the Machine block shrinks to a picker

The Document tab of every output page (laser included) keeps a **Machine** group with:

1. **Profile row (28px)**: the `MachineBlock` profile button (flex, name truncated, `ChevronDown`/`ChevronUp`, the inline list of the family's machines; a pick is one step "Pick profile"). Then 4px and a **"…" symbol button** 28 x 28 (36 coarse), Lucide `Ellipsis` 16px. Its name is "Edit machine <name>" and its tooltip "Edit machine", as "Edit material" opens the Materials tab. A press switches to the Machines tab (`documentPanel.pressTab("machines")`, the way `TestGridBar` opens "materials"), unfolds that machine's row and moves focus to its name. The "…" replaces Save as profile (`BookmarkPlus`) in this row.
2. **State row** (only while a state applies):
   - "Differs from profile" with **Reset from profile** (`BookmarkCheck` as built, one step). It shows when the maker chose Keep pages after a Machines-tab edit (4.5), and for a copy changed by a library reload or sync, which does not ask.
   - "Not in your machines" with **Save as machine** (`BookmarkPlus`, one step; the `0026` §5 rules). This replaces "Not in your profiles" (U5). A workspace page may use a machine that is not in this computer's library. The page then runs from its copy, everything works, and nothing is added to the library or the sync until Save as machine is pressed. What the sync writes is ADR 0098's.
   - Update profile leaves the page: no page edit can make the copy differ any more (U6).
3. The inline list's rows keep name and work area. Their Duplicate, Rename, `Ruler` (edit work area) and Delete actions move to the Machines tab (U7).

Everything else in today's `MachineBlock` leaves the page. That is the Work area line, Machine size offer, Origin corner, Max feed, Max power, Default power, Default speed, Laser mode, focus pulse, Motorised Z axis, Air assist, Output and the Connection group. Motorised Z axis and Air assist become machine capabilities. Their per-job use moves to the operation (Frage 435, section 6). The same goes for the families' machine fields: plotter Cut area, Max speed, Default pressure, Default speed and Output; embroidery Hoop, Speed, Max stitch and Output; CNC Z travel, Spindle max, Safe height, connector address and Output. **Place page** stays in the Document tab, directly under the Machine group.

### 4.2 The tab

- **Strip entry** (U8): directly after Fixtures (CNC: after Tools), so it is the last of the output-page tabs and comes before Style, Layers and the rest of the strip. Lucide `Factory` (unused so far), name and tooltip "Machines". Shown on **every output page** of an enabled family, absent on drawing and sketch pages (like Operations, Materials and Fixtures). It is a new `PanelTab::Machines` and a new `PanelContent` "machines".
- **Header** "Machines" with the page's kind chip.
- **Sections, one per family** in the order Laser, Plotter, Embroidery, CNC, each headed by the family's kind chip and a count ("3"). The active page's family section is first and unfolded. The others are folded to their 28px heading row: a press or Enter unfolds, the `0113` Machine sets fold look (U9). A family whose extension is off has no section.
- **Rows**: the `MaterialRowView` look (`0112` 7.3), 40px (48 coarse). Line 1 has the name (F2 or double-click renames in place, 1 to 60 characters), the library sync toggle (`0116`), the armed trash (`0081`), no arrows. Line 2 (11px muted) has the work area and format: "400 × 300 mm · GRBL", "Cricut Maker 3 · Design Space SVG", "150 × 250 mm · DST". Order: the family's built-ins first, then the maker's machines by name, case-insensitive (the `0026` order).
- **Add**: one `0113` 4.2 outline text button per family section, "+ Machine". It duplicates the family's built-in as "<name> copy", unfolds it and focuses Name. Every row has a **Duplicate** symbol button (`Copy`, Ctrl+D), as `0026` §3 (U10).
- **Built-in rows** (Generic laser, Cricut Maker 3, Generic HPGL cutter, W6 EU7, Generic GRBL router): Duplicate only, no trash, no rename. Their body shows every value as a plain text line in the Work area look (label left 12px muted, value right 13px tabular), with no fields, because they cannot be edited (hidden, not disabled).
- **Delete** (the `0081` armed trash): armed text "Press again to delete <name>. Pages keep their copy." It is one undo step like every edit in the tab, and armed like every trash (`0081`). A page that used the machine then shows "Not in your machines".
- **Body** of a maker's machine (indented 16px, accent bar, one open at a time): the family's fields of section 4.3, then the **Connection** group (`ConnectorGroup` as built, for laser and CNC; plotter sending is a later slice). Every field edit is **one undo step** with the library journal, as a Materials-tab edit: Enter or blur commits the record, Escape reverts, Ctrl+Z restores the record. History names "Edit <field>" ("Edit max feed"), and for the list "Add machine", "Duplicate machine", "Rename machine", "Delete machine". The Connection group stays a plain write without a step, as built. What the pages do after an edit is 4.5.

### 4.3 Machine fields per family (top to bottom; `EntryField`, label inside left, unit inside right)

| Family | Fields |
|---|---|
| Laser | Width and Height (112 each, mm), the Machine size offer row (`MachineSizeOffer`, `0131`) under them, Origin corner (`CornerControl`), Max feed (mm/min), Max power (S-max), Default power (%) and Default speed (mm/min) in one row (the `0150` pair), Laser mode (`PanelSwitch`) with Focus pulse, Pulse power, Pulse length under it as built, the capabilities **Motorised Z axis** and **Air assist** (`PanelSwitch` each, "the machine has it"; Frage 435), Output (plain line, read-only) |
| Plotter, mat profile | Mats (plain line "12 × 12 in, 12 × 24 in"), and on an HPGL profile Max speed (mm/s), Default pressure (%) and Default speed (mm/s), Output (plain line). On a Design Space SVG profile the three numbers are absent (`0159` C13) |
| Plotter, matless | Width and Height, Origin corner, Max speed, Default pressure, Default speed, Output |
| Embroidery | Hoops (plain line "150 × 250 mm, 100 × 100 mm"), Speed (stitches/min, 100 to 1,500), Max stitch (plain line "12.1 mm"), Output (plain line "DST") |
| CNC | Width and Height, Origin corner, Z travel (mm), Spindle max (rpm), Safe height (mm), Output (plain line "GRBL") |

Ranges, clamps and notices are those of `0026`, `0150`, `0159` 4, `0160` 3 and `0161` 3. Clamp notices ("Default speed limited to 800 mm/s.") show under the field as built.

### 4.4 Machines tab: keys, names, texts, states

- **Keys.** The "…" is a Tab stop right after the profile button. In the tab, the Tab order is: each section heading (Enter or Space folds and unfolds), its "+ Machine", its rows (one Tab stop per list, the `0026` §3 row keys), then the unfolded body's fields in the order of 4.3. The row keys are Up and Down, Home and End, Enter to unfold, F2 to rename, Ctrl+D to duplicate, and Delete to arm, with Delete or Enter on the armed row to remove. Escape in a field reverts it. Focus after a delete goes to the row that took its place, else the one above (`0026` §3). Leaving the tab by the strip keeps the unfolded row for the next visit in the session.
- **Names.** Heading buttons "<family> machines, 3" with `aria-expanded`. Rows "<name>, <line 2>". Duplicate "Duplicate <name>". Fields as `0026` §2 and the family notes ("Z travel, millimetres").
- **Write failure.** "Could not save the machine. Nothing was changed." (the built `WRITE_FAILED` alert with the new word) shows under the field that wrote, `role="alert"`, 3 s. The field keeps the typed text.
- **Library notices** ("1 machine could not be read. It was left as it is.", "1 machine is from a newer version. It was left as it is.") sit at the top of their family's section as a Notice block, a state that stays while it applies. A damaged or newer record is listed as a row in the `MaterialRowView` damaged or newer look: warning glyph, reason in line 2, no trash, no body.
- **Only the built-in.** The section shows the built-in row and "+ Machine". There is no empty-state text.
- **Undo.** Field edits, add, duplicate, rename and delete are steps (4.2). The sync toggle and the Connection group are not. No notice after a success.

### 4.5 Follow the pages? (Frage 434 C)

After a committed edit in a machine's body, when pages of the open project use that machine and now differ from it, the body asks inline. There is no dialog and no popup. The house pattern is the **Inline confirm** (`0020`, `0045`) in its non-destructive form, as Apply in the Sync switch row:

- **Place**: a block at the end of the machine's fields, above the Connection group. It never sits above the field being edited, so nothing moves under the pointer. It has `role="group"` named by its sentence and the `--toolbar-bg` ground of the Notice look.
- **Sentence** (12px `--toolbar-icon`): "2 pages use Bed laser. Update them too?" ("1 page uses …").
- **Buttons**, equal width, 8px apart: **Keep pages** on the left (text button) and **Update pages** on the right (the Apply look: `--toolbar-icon-active-bg` ground, white label). Update pages ignores presses for its first 400 ms.
- **Focus** is not taken (the maker may be typing the next field). A polite status reads the sentence once. Tab reaches the block after the last field.
- **Update pages** writes every differing page's copy with that family's writer (pages resize with the anchor of section 2). It is **one undo step**, "Update 2 pages", after the edit's own step. Undo returns the pages, and a second undo returns the record. A page whose new size cannot be laid out keeps its copy and shows "Differs from profile".
- **Keep pages**, Escape on the block, folding or leaving the body, or a tab change closes it without writing. Those pages then show "Differs from profile" with Reset.
- **Several edits in a row** share one block: its count is the pages differing now, and one press updates them all. The block goes by itself when no page differs (for example after undo).
- **Not asked**: a library reload or a sync from another device (no edit here, no step to carry it), and edits to a machine no open page uses.
- Texts at most 70 characters; History name "Update <n> pages" (Proposal).

## 5. Operations in the Select bar (point 4)

- `OperationKindGroup` is drawn after the switches on **every output page** for a selection. Its rules are those of `0111` 3 and 4: press assigns to the last operation of that kind, Shift creates a new one, digits 1 to 9 assign to operation n, Escape leaves an entered group. Inside an entered group the bar holds only this group. The Tabs toggle stays laser-only.
- Buttons per family. The glyphs are those of the family notes (`0159` 7.1, `0160` 2, `0161` 2) and become cases of `OperationGlyph`. The name and tooltip is the kind word:

| Family | Buttons in bar order |
|---|---|
| Laser | Cut, Score, Engrave, Engrave + outline, Cut out, Image (unchanged) |
| Plotter | Cut (the laser's Cut glyph), Draw |
| Embroidery | Running stitch, Tatami fill, Satin |
| CNC | Profile outside, Profile inside (one kind Profile with its Side, `0161` N4) |

- The panel's kind buttons (Cut and Draw, the three stitch buttons, the two Profile buttons) are removed. The Operations tab has none, as on a laser page.

## 6. The Operations tab and Output (point 5)

- **Strip**: Operations right after Document on every output page (the `0101` place), then Materials (laser only), Fixtures, Tools (CNC only), Machines.
- **Body**: the laser `OperationsSection`. That is the row list (`OperationRowView`, 3 lines), the one unfolded row (`OperationBody`), and the `outputGroup` slot with `OutputGroup` at the foot. The laser-only parts are absent on family pages: fixture jobs, print-and-cut slots and the test-grid rows.
- **Cuts last** switch: laser as built; plotter present, default on, Marking (Draw) before Cutting (Cut) (`0159` C12); embroidery and CNC absent (list order is run order, `0160` E16, `0161` N16).
- **Rows**: line 1 has the glyph, name, Show, Output, trash, Up and Down. Embroidery adds the 12px thread swatch after the glyph (`0160` E5). Line 2 has the run ring, "<Kind> · n items" and the time. Line 3 has the numbers as the family notes give them (plotter "40 %, 200 mm/s, 2 ×" or "Set in Design Space"; embroidery "2.5 mm, 1 ×"; CNC "Ø 3.175 mm, 10 mm deep, 4 passes, 800 mm/min"). A Design Space row does not unfold (`0159` C13).
- **Unfolded row** (228px): see section 8 for the control per field. Contents per family as `0159` 7.2, `0160` 5.3 and `0161` 6.3.
- **Capability settings on the operation** (Frage 435, laser): **Air assist** is a 28px Radix `Toggle` (the `OperationOptionsPart` look, Lucide `Wind` 16px, name and tooltip "Air assist", `aria-pressed`) among the operation-only settings of every kind. **Z offset** is the built field (`0111` criterion 21). Each shows only while the page's machine has that capability (hidden, not disabled). Each change is one step "Edit settings". Air assist defaults to on for a new operation while the machine has it, so a job exported today is unchanged (Proposal U21). A stored value on a machine without the capability is kept and ignored, with the `0029` criterion 5 warn for Z offset and the same warn for Air assist: "Cut 1: air assist is set but the machine has none." The G-code switches the air per operation (`M8` before its first burn move, `M9` when the next one has it off); the spec owner states it. The machine line's " · air assist" (`0029` criterion 2) then names the capability.
- **Output group** at the foot (`OutputGroup`, the `0029` 6.3 look): heading "Output", then the machine line "<machine> · <format>", the checks (`OutputChecks`, the select and unfold buttons as built), the total, and the button row.
  - **Export job file** is the one Primary button. Its save dialog is "<project> - <page>.<ext>" with the profile's file type:

    | Profile format | File type | Extension |
    |---|---|---|
    | Design Space SVG (Cricut) | Design Space SVG | `.svg` |
    | HPGL (generic cutter) | HPGL | `.plt` |
    | DST (W6 EU7) | Tajima DST | `.dst` |
    | GRBL (CNC) | GRBL G-code | `.nc` |
    | laser | unchanged | unchanged |

    The notice and error are as `0029` 7.
  - **Send to machine** (`0125`) beside Export: laser as built and CNC. On CNC pages the **machine controls footer** (`MachineControls`, `0118`) shows under the column while the profile has a connector address, without Frame and Focus point (`0161` N15; the MVP lacks the footer).
  - **No Start pick** on family pages.
- **File tab, Export SVG** (`ExportSvgSection`) is untouched. It exports the drawing on every page kind and is not the job export. The Design Space SVG lives only in the Output group.

## 7. Toolpath view: page-label toggles only (point 6)

- The **Toolpath view** switch leaves every panel. The two page-label toggles of `PageFrames.tsx` are the only switches: Operations view (`ListTree`, pressed by default) and Toolpath view (`Route`), mutually exclusive, session state with no step. `pages_json.rs` already emits both for any page of an enabled output process, so family pages get them from the same code. The implementer checks this in the browser, and only the panel switches are removed.
- The **play bar** is the laser's `PlayBar`, anchored under the page by `PageFrames` while Toolpath view is pressed, with the run rings of `RingLabels`. It has one Play/Pause, one slider (the Ratio-slider look and pointer rules) and one readout line. The embroidery and CNC panel play bars and their native range sliders are deleted.
- Per family, only the axis and readout differ. The session sends them as it does for the laser:

| Family | Slider axis | Readout | Bar extras |
|---|---|---|---|
| Laser | time | "1 min 2 s of 3 min 40 s" | none |
| Plotter, HPGL | time | as the laser | none |
| Plotter, Design Space | path length (`0159` C14) | "0.62 of 1.80 m" | none |
| Embroidery | stitch number (`0160` E10, E11: arrows step one stitch) | "1,204 of 8,960 · ■ 2 of 3" with a 10px thread swatch | **Speed** "1×", "4×", "16×" after the readout as the bar's own segments (`--value-fill` pressed, the Choice-strip rule for bars), default 16×; bar minimum width 440px (`0160` E13, E14) |
| CNC | time (planner) | "2 min 3 s of 7 min 40 s · Z −5.00 mm" (`0161` N8) | none |

## 8. Controls: the laser's, white fields (point 7)

The MVP family panels' `FIELD` is `bg-transparent`, so the grey panel (`--panel-bg` `#DCDCE0`) shows through. The laser's fields (`EntryField`, `machineStyles.ts` `FIELD`) have the **white ground** of the design system's "Bar number field" and "Value field". The fix is a fact, not a choice: every family control becomes the laser control below, and the private styles are deleted.

| MVP family control | Becomes | Where |
|---|---|---|
| number or text field, label above, red text under it | `EntryField`: 28px, white, 1px `--field-border`, label inside left, unit inside right, the validation chip (`CHIP_IS_MESSAGE`), commit on Enter or blur, a cleared field reverts. Widths 112 (two in a row) or 228 | unfolded rows, Fixtures tab (Stock), Machines tab, Tools tab |
| private `SEGMENT` radiogroups (Mat size, Grip, Hoop, Repeats, Side, Direction, tool pick) | **`PresetStrip`**, the Choice strip of the Dither strip (1px ink-at-60 % border, cells 1px apart, pressed `--toolbar-icon-active-bg`/`-fg`, one Tab stop, arrows select). Grip cells carry the 12px grip swatch before the word, two cells to a row | Fixtures tab, unfolded rows |
| tool pick (CNC) | `MaterialPick` look (full width x 28, end mill glyph, name at the right, inline list) | CNC unfolded row |
| thread pick | `OutputPick` look with the 12px swatch and hex, the inline `ColourPicker` under it (`0160` 5.3) | embroidery unfolded row |
| on/off rows (Mat grid, Cuts last, machine switches) | `PanelSwitch` | as above |
| row toggles | the 28px Radix `Toggle` of `OperationOptionsPart` | where a family later adds one |
| private `BUTTON` (Export, + Tool, Reset to tool) | Export: the `OutputGroup` Primary. Text buttons: the `0113` 4.2 outline text button. "Reset to tool": the "Reset to material" button of `OperationSettingsBlock` | as above |
| native `<input type="range">` play sliders | none in the panel; the canvas `PlayBar` slider (section 7) | canvas |

## 9. Per family: what is reused and what moves where

| Was (MVP, Document-tab panel) | Laser (unchanged) | Plotter | Embroidery | CNC |
|---|---|---|---|---|
| Machine section | `MachineBlock`, now picker, "…" and state row (4.1); fields to the Machines tab | same | same | same |
| Mat / Hoop / Stock section | Fixtures tab: Machine sets, Job fixtures | **Fixtures tab**, section "Mat": Mat size strip, Grip strip, Mat grid switch (`0159` 6 with the controls of 8). Absent on a matless profile | **Fixtures tab**, section "Hoop": hoop strip (`0160` 4; over eight hoops the `OutputPick` list) | **Fixtures tab**, section "Stock": Width and Height (112), Thickness (112), Material (228), `EntryField`s, step "Edit stock" (`0161` 4) |
| Tools section | none | none | none | **Tools tab** (`0161` 5: `MaterialsTab`/`MaterialRowView` pattern, + Tool, rename, armed delete with reassignment) |
| kind buttons in the panel | Select bar | Select bar, Cut and Draw | Select bar, three stitch kinds | Select bar, Profile outside and inside |
| operation rows | Operations tab, `OperationsSection` | same, Cuts last on | same, thread swatch, no Cuts last | same, no Cuts last |
| Output section | foot of the Operations tab, `OutputGroup` | same, Export SVG or PLT | same, Export DST | same, Export NC, Send, `MachineControls` footer |
| Toolpath switch | page label only | page label only | page label only | page label only |
| panel play bar | canvas `PlayBar` | canvas `PlayBar` | canvas `PlayBar` with Speed | canvas `PlayBar` with Z |

Deleted in the same PR: `PlotterPanel.tsx`, `EmbroideryPanel.tsx`, `CncPanel.tsx`, the three early returns in `ManifestTab.tsx`, their private `HEADING`/`ROW`/`SEGMENT`/`BUTTON`/`FIELD`/`ICON` constants, the panel `PlayBar` and `Segments` functions, and the frontend tests that assert the panel layout (rewritten for the tabs).

## 10. Seams for the architect and the implementer (not customer questions)

- **A1 Page size writers** (amends ADR 0042 §6 and `0159` PL11):
  - Plotter page = the cut area, resized by Set mat from the cut area's top-left corner.
  - Embroidery page = the active hoop's sewing field, resized by Set hoop about the page centre. This is the first centre-anchored resize; placements keep their global place.
  - The writers translate the exports: Design Space SVG by +6.35 mm on both axes to the mat origin; DST by the page centre (as now); G-code unchanged.
  - The goldens' file bytes should not change for the same item at the same place on the mat or hoop. Test literals move.
- **A2 Tabs** (`panel_tabs.rs`): `tab_allowed` lets Operations, Materials and Fixtures through on laser pages only. Operations and Fixtures open to every enabled output family, Materials stays laser-only, Tools is CNC-only, and Machines is new on every output page. `PanelContent` gets "machines" (and "tools" if it is not there yet).
- **A3 One view shape per panel part.** The laser components read `operations_view_json` (rows, `select_bar_kinds`, unfolded body), `output_group_json` and the play-bar view. `active_operations()` is laser-only today, and the frontend's `OPERATION_KINDS`/`BUTTON_KINDS` list only laser kinds. A family page must answer those views in the same shape, with the family kinds added to the kind union, `KIND_WORDS` and `OperationGlyph`. The alternative is an adapter from the family views into the components' props. Which of the two is the architect's choice. Either way the components stay the laser's, and no family-specific copy of a component is written.
- **A4 Machines tab model.** The tab reads the library's records of all four families (ADR 0097 envelope, `kind` dispatch). Machines not in the local library and the sync rules are ADR 0098's.
- **A5 Undo and follow.** Machine edits are journal steps (ADR 0098 decision 4). Frage 434 C is ADR 0098 decision 5 A, run as its own step on Update pages instead of inside the edit's batch. The ADR notes C as "not prepared" because it read C as a popup. The inline block of 4.5 is not a popup, so the ADR's note needs that amendment.
- **A6 Frage 435.** Air assist per operation is a new operation setting (stored, ranged, written to G-code per operation), so there is an operation format change. Z offset per operation exists already.

## 11. Pointers added to the earlier notes

- `specs/0159-cutting-plotter/ux-notes.md`, `specs/0160-embroidery/ux-notes.md`, `specs/0161-cnc-output/ux-notes.md`: a revision note at the top naming the superseded sections.
- Laser: `specs/0026-machine-profile/ux-notes.md` (§1, §2 fields and §3 row actions move to the Machines tab), `specs/0098-page-kinds/ux-notes.md` (§2 Kind switch symbols; §9 Machine block), `specs/0111-laser-operation-tools/ux-notes.md` (§3 group per family), `specs/0131-controller-bed-size/ux-notes.md` (the offer row moves into the Machines tab), `specs/0029-laser-job-preview-and-output/ux-notes.md` (Air assist per operation, Frage 435).
- `docs/design-system.md`: rows amended or added (section 15).

## 12. Decisions for veto (Proposal)

- **U1** Plotter page = the active mat's cut area (11.5 x 11.5 in or 11.5 x 23.5 in), origin at its top-left. The rulers count from the cut area, so the mat's printed cm 1 tick sits at 3.65 mm on the ruler. Supersedes `0159` C4.
- **U2** Embroidery page = the active hoop's sewing field; the hoop frame on the pasteboard. Supersedes `0160` 3 "page = largest field + 20 mm" and M5.
- **U3** Set hoop resizes about the page centre.
- **U4** Kind switch: 32 x 32px symbol buttons with 24px glyphs, one row, word only as tooltip and accessible name.
- **U5** New texts say "machine" ("Not in your machines", "Save as machine"); the built "Differs from profile" and "Reset from profile" keep their words.
- **U6** Update profile leaves the page (nothing on the page edits the copy).
- **U7** Duplicate, Rename, Edit work area and Delete leave the page's inline list for the Machines tab.
- **U8** Machines tab: after Fixtures (CNC: Tools), before Style, Lucide `Factory`, on output pages only.
- **U9** One section per family, the page's family first and unfolded.
- **U10** "+ Machine" duplicates the family built-in; Duplicate stays on every row.
- **U11** Built-in bodies are plain text lines, not read-only fields.
- **U12** Machine deletion is armed; pages keep their copy.
- **U13** The "…" is named "Edit machine <name>" and opens the row unfolded.
- **U14** Plotter grid and centre cross run on under the page edge; no veil inside the page.
- **U15** The CNC stock sits at the page origin (not movable in this revision) and shows its label.
- **U16** Mat size, Grip, Hoop, Repeats, Side and Direction are Choice strips, not segment groups or inline lists.
- **U17** The Tools tab sits between Fixtures and Machines.
- **U18** The embroidery Speed segments sit in the canvas play bar.
- **U19** Default power and Default speed are machine fields (they leave the page with the rest). Air assist and Motorised Z are machine capabilities by Frage 435.
- **U20** Add page remembers the last machine per kind (`0159` C3, now built).
- **U21** An operation's Air assist defaults to on while the machine has the capability.
- **U22** The follow block (4.5) sits after the fields, takes no focus, and Update pages is its own undo step.

## 13. Customer answers (2026-11-28)

- **Frage 434 (MF-MACHINE-FOLLOW): C.** After a Machines-tab edit the tab asks inline whether every page using that machine follows (4.5). A workspace may use machines that are not in the local library, and the sync must stay clean; the rules are ADR 0098's.
- **Frage 435: Air assist and Motorised Z are machine capabilities** edited in the Machines tab. Air assist on or off and Z offset are operation settings, shown only while the page's machine has the capability (section 6).
- **Machines-tab edits are undo steps**, as in the Materials tab (4.2, 4.4).

## 14. Implementer checklist

1. Tabs (A2): Operations and Fixtures on every output page, Tools on CNC, the new Machines tab; strip glyph `Factory`.
2. Machines tab (4.2 to 4.5): family sections, rows, + Machine, Duplicate, rename, armed delete, sync toggle, bodies with `EntryField`, `PanelSwitch`, `CornerControl`, `MachineSizeOffer` and `ConnectorGroup`; built-ins as text lines; every edit an undo step; the inline follow block with Update pages and Keep pages.
3. Document tab (4.1): Machine group = profile button, "…" opening the Machines tab, state row; Place page under it; every field gone from the page.
4. Add page (3): 32px symbol segments in one row; last machine per kind.
5. Page size and drawing (2.2, A1): plotter cut-area page with the mat around it; embroidery field page with the frame around it; CNC stock label.
6. Fixtures tab (9): Mat, Hoop and Stock sections with the controls of 8.
7. Select bar (5): family kind buttons and glyphs in `OperationKindGroup`.
8. Operations tab (6): `OperationsSection` with family rows and unfolded bodies; on the laser the Air assist toggle and Z offset only with the capability; `OutputGroup` with checks, total, Export per format, Send and the footer on CNC.
9. Toolpath (7): panel switches gone; canvas `PlayBar` with the family axis, readout, Speed and Z.
10. Delete the three panels and the `ManifestTab` branches (0); rewrite the panel tests against the tabs.

## 15. Design system (rows changed with this revision)

- Amended: "Add page group (`0098`)" (Kind switch as 32px symbol buttons, one row); "Kind chip" (Plotter `Slice`, Embroidery `Spool`, CNC `Drill`); "Machine block (profile picker)" (picker, "…", state row only); "Laser page Document tab (`0098`)" (Machine group without fields); "Machine size offer (`0131`)" (in the Machines tab).
- Added: "Machines tab (machine families)"; "Output page of any family" (tabs and Select bar the same for all four, page = work area).
- No new token.
