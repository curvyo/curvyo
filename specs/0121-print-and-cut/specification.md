# Print and cut: registration marks on a printed sheet tell the laser where the sheet lies, so the cut lands on the print

Status: Slice 1 Done (PR #457; Ready 2026-11-23; ADR 0081 Accepted with PR #457; Fragen 328 to 333 at their defaults A)
Priority: Could
Origin: Customer request of 2026-11-17 ("Print then cut: place registration marks on the page yourself, print, and have the laser or cutter find them, with the camera of `0120`, to align the cut to the printed sheet"). The slice split, the mark shape, the fit and every number are the product owner's **(Proposal)**, vetoable at the demo.

## User value

As a maker who prints a design (stickers, labels, a photo on card, a printed panel) and then cuts it out with the laser, I want to put registration marks on the artwork myself, print the sheet, lay it anywhere on the bed, tell Curvyo where the marks are, and have the cut move and turn onto the printed sheet, so that the cut follows the printed outline within half a millimetre without squaring the sheet to a jig by eye.

**What other tools do.** LightBurn: "Print and Cut" (a Print and Cut tool; the maker places target marks, prints, then picks two target points with the laser or the camera and the job is aligned to the sheet; details not re-checked in this session). Silhouette Studio and Cricut: registration marks printed on the sheet, found by the machine's optical sensor, with no manual path. Inkscape: none. We do better by: the marks are ordinary objects the maker places and moves, the drawing is never changed by the alignment (only the sheet's placement on the laser page is), the residual error is shown in mm before anything runs, and the first slice needs neither a camera nor a jog panel.

## Words (shown in the UI)

- **Sheet:** the Drawing page that holds the artwork and the marks, placed on the laser page as a placed page (`0100`). It is what you print.
- **Registration marks:** an object (kind "Registration marks") of 2 to 4 numbered marks on the sheet. They print; they are never cut, engraved or scored.
- **Mark:** a crosshair inside a circle (Frage 329 A), numbered.
- **Measured position:** where mark n lies on the bed, in machine millimetres, typed in or taken from the head position.
- **Fit:** the move, turn and uniform scale that bring the sheet's marks onto the measured positions as well as possible.
- **Residual:** per mark, the distance in mm between where the fit puts it and where it was measured. Shown as largest and as RMS.

## What the maker sees in the demo (slice 1)

1. A Drawing page with a design. In the Document tab: **Add registration marks** adds four marks, placed so they never overlap the artwork (criterion 1), as one object. Marks are moved by the normal Select tool; Delete on a mark when two are left is refused.
2. The sheet is exported as SVG and printed elsewhere at 100 %; the marks are in the SVG file, and the page picture shows them too.
3. The sheet is dragged from the page list onto the laser page (a placed page, `0100`). The laser page's Operations tab has a new **Print and cut** group: the sheet, a row per mark (1 to 4) with X and Y boxes in machine mm, and, on a connected machine, a button **Take head position** per row (work position). With two or more placed sheets with marks a **Sheet** picker chooses the sheet.
4. The maker lays the printed sheet on the bed, moves the laser dot with the machine's own panel onto the centre of mark 1, presses Take head position (or types X and Y), and repeats for the other marks.
5. Under the rows: "Fit: moved by 83.4, 41.2 mm (machine position of the sheet's top-left corner), turned 1.8 degrees, scale 100.1 %. Largest error 0.21 mm." In the tolerance (0.5 mm): normal style; above: warning style "Largest error 0.74 mm; check mark 3". **Apply** puts the placed sheet onto the fit in one undo step and is greyed while the sheet is on the fit; **Reset placement** returns it to where it stood.
6. The preview (`0029`) shows the cut moved and turned onto the sheet's position; the marks show as faint crosses (not toolpath). Start and Send work as before; the machine moves only when the maker starts the job.

## Acceptance criteria

Marks object (page side, no machine needed).

1. Given a Drawing page, then its Document tab has the button **Add registration marks** (no Insert group exists); one press adds an object of 4 numbered marks, as one undoable step, placed so that no mark overlaps the artwork **(Proposal, customer may veto)**: the artwork's box is the box of all objects on the page without marks objects; with the page corners inset 10 mm as candidates, if each candidate centre is at least 10 mm outside the artwork's box the marks go to the page corners; otherwise they go to the corners of the artwork's box grown by 10 mm, clamped to 10 mm inside the page edge when that would leave the page (the page corners when the page is empty). A test places marks around a fixture with a full-page design and one with a small design and checks both rules. The object is a single selection item; its marks are moved one by one by the Select tool when the maker enters the object (same entering as a group, `0050`) and the numbers stay.
2. Given a mark, then it is a circle of 6 mm diameter with a crosshair of 8 mm (the diameter plus 2 mm), line width 0.25 mm, black, with its number in a digit half the diameter high (3 mm), 3.5 mm beside the circle; the diameter (3 to 20 mm) and the line width (0.05 to 1 mm) are numeric boxes of the object and change every mark of it in one step (Frage 329 A); a transform moves a mark's centre only, size and line are never scaled or turned. A mark counts as its crosshair centre; the maker never has to find a centre by eye on the print.
3. Given the marks object, then it holds 2, 3 or 4 marks (add up to 4 by a button); Delete on a mark when 2 are left is refused with "Registration marks need at least 2 marks."; the object can be deleted as a whole.
4. Given the SVG export and the page picture, then the marks are in the file at their position; given a Laser page job (`0029`), then no mark contributes a single burn move, in any operation kind, whether the marks object is on the sheet or the sheet is placed (a test counts zero moves from a fixture with marks and a Cut operation over everything).
5. Given the laser page's canvas and preview, then marks of a placed sheet are drawn as thin faint crosses in a colour no operation uses and are not selectable as burn items, in the Drawn view as well as the preview; Operations list their parent object as "Registration marks (not cut)".
6. Given the marks object, then it can be copied and pasted and cloned (`0049`) like a group; a second marks object on the same page is allowed but the Print and cut group of a laser page uses the first one of the placed sheet (the row names it; a second shows "Not used").

Measuring (laser page).

7. Given a laser page with a placed Drawing page that has a marks object, then its Operations tab shows the Print and cut group (otherwise the group is absent, not disabled); it lists the marks in number order (of the selected sheet: with two or more placed sheets with marks the group has a **Sheet** picker, default the selected placed sheet; the measures belong to the placed sheet, so each sheet keeps its own; slice 1 supports one sheet at a time) with X and Y boxes (machine mm, 0.01 mm steps, plain number entry, accepts a minus sign and a decimal point or comma).
8. Given a connected machine reporting its position (`0064`, `0127` G4), then each row has **Take head position** that fills the row with the machine's **work** position (the frame the job's absolute moves use, ADR 0081 decision 8; not the machine position of `0140` criterion 10), rounded to 1 micrometre, and the button is disabled with the cause "Machine is moving" while the state is Running, Homing or Jog (the connector reports a jog as Idle: "Machine is moving"), with the cause "Controller state: Door", "Controller state: Check" or "Controller state: Sleep" in those states, and with the cause "No position from the machine" in any other state without a position; without a connector the buttons are absent and typing works (also for a file-only maker who reads the position from the machine's own display).
9. Given a row, then a **Clear** button empties it; a row with only one of X and Y filled counts as empty. Each row edit, Clear and Take head position is one undo step.
10. Given fewer than 2 filled rows, then the group shows "Measure at least 2 marks" and Apply is disabled; given a filled row without a matching mark (the maker deleted mark 3 on the sheet), then the row says "Mark 3 is gone from the sheet" and does not count; given a stored measure that cannot be read, then the group says "The stored measures could not be read." with empty rows and the next measure write replaces it.

Fit.

11. Given 2 to 4 measured marks, then the fit is the least-squares similarity transform (translation, rotation, one uniform scale) from the marks' positions on the sheet (the placed sheet's own coordinates, so the existing placement does not matter) to the measured machine positions; with 2 marks the fit is exact and the residual 0 (the group says "2 marks: no error check"); collinear marks are a valid fit (they only switch off the mirror check).
12. Given the fit, then the group shows moved by X and Y (mm: the fit's machine position of the sheet's top-left corner, **Proposal**), turned by an angle (degrees, 0.01), scale (percent, 0.01), the largest residual and the RMS residual in mm. Fixtures with known answers pass: marks transformed by a known rotation, scale and offset with no noise give residuals below 1e-6 mm; the same with 0.2 mm of fixed noise give the exact residuals computed by the test, within 1e-6 mm.
13. Given the largest residual, then it is compared with a tolerance of **0.5 mm** (a number box in the group, 0.05 to 5 mm) **(Proposal)**: not above, normal style; above, warning style and the name of the worst mark, and Apply still works (permit over refuse, `0029` criterion 4 style).
14. Given a fit whose scale is outside 98 % to 102 %, or that would mirror the sheet (3 or more marks measured in reverse order), then the group warns "Scale 94.20 %: was the sheet printed at 100 %?" or "The marks look mirrored: check the numbers", and Apply still works. Given two measured positions within 1 mm, then the group warns "Marks 2 and 4 are measured at the same place" and Apply still works. Given the laser page's Start is Head position, then the group warns "Start is Head position: the cut will not follow the sheet" and Apply still works.
15. Given geometry comparisons, then each uses an explicit tolerance and lengths are millimetre newtypes (`CLAUDE.md` §5); the fit is a pure function in a `*-core` crate (the architect names it) with no I/O, builds for wasm, and has unit tests for 2, 3 and 4 marks, collinear marks (a valid fit, not mirrored), all marks or all measures within 1 mm ("Marks too close to fit", no result) and a repeated measure of one mark.

Apply and job.

16. Given Apply, then the placed sheet's placement on the laser page (move, turn, scale of `0100`) is set to the fit, as one undo step; the Drawing page, its objects and the marks are not changed (a test compares the Drawing page's serialised bytes before and after); the previous placement is kept as the Reset target until the next Apply or until the sheet is moved by hand, and **Reset placement** (offered only while the sheet is on the fit) restores it exactly. Apply is greyed while the sheet is on the fit. Given a fit that leaves no overlap with the laser page, then Apply is refused with "The fit puts the sheet off the laser page."; given a fit whose scale is outside 50 % to 200 %, then Apply is not offered and the group says "The fit is not usable: scale 31.00 %."
17. Given the sheet placed with the fit, then the preview and the job (`0029`) come from the placed sheet as for any placed page; the output files for the same project and settings stay byte-identical (`0029` criterion 24) and a GRBL golden `print_and_cut_cut_line.gcode` in `curvyo-laser-core/tests/fixtures/output/grbl/` shows a square of 40 mm placed by a fit of 90 degrees and an offset (the numbers are in the test).
18. Given the maker moves, turns or scales the sheet by hand after Apply, then the group says "Placement differs from the fit" with **Apply again**; the measured values are kept.
19. Given Save and reopen, then the measured positions (X and Y per mark, tolerance, the Reset matrix) are stored on the placed sheet's placement on the laser page and the Print and cut group shows the same state, including the fit recomputed from them; a project without them opens as before; a copy of a placed sheet does not copy its measures **(Proposal)**.
20. Given the whole slice, then no command is sent to the machine by the group but position reads; moving the head is not part of this slice (the maker's own panel or `0118` jog).

Privacy and safety.

21. Given R-SYS-007, then nothing leaves the machine; the group reads the position from the maker's own connector only.
22. Given a job after Apply on a sheet with the warning (criteria 13, 14), then the existing run guard (`0064` criterion 9, "require a local start") and the job's work-area check (`0029` criterion 4) apply unchanged; a sheet that puts burn geometry outside the work area gets the existing "Outside the work area" warning.

## Later slices (listed, not in slice 1)

- **Slice 2, jog and go to mark** (needs the jog buttons of `0118`): Go to mark n moves the head, laser off, to the mark's expected position (from the current fit or the nominal position) at the machine's frame speed, after an explicit press per move; a 4th mark checks the fit.
- **Slice 3, camera** (needs `0120`): see Dependencies. The marks are found in the snapshot, by click or automatically, and perspective is corrected by the camera's pixel-to-bed map (`0120`); the sheet fit stays a similarity (ADR 0081 decision 10).
- **Slice 4, print and cutter output**: PDF export or a print dialog with crop bars; a cutting plotter's own marks and its optical sensor (the plotter family, R-MFG-CUT-002; one machine family at a time, `CLAUDE.md` §6).
- **Affine fit**: shear and non-uniform scale (a printer that stretches one axis) needs a general placement of a placed page, which `0100` does not have; deferred (Frage 331 C); it needs a second fit function and its own warnings, no placement or format change (ADR 0081).

## Dependencies: what slice 1 needs

- **`0120` camera-calibration is not needed.** The slice measures with the head: typed numbers or the head position. (Frage 333 A below fixes the mark as its own object kind.) This gives exactly what the customer wants (a cut on the print) at a small cost, and the same fit and the same Apply serve the camera later: the camera only replaces where the numbers come from.
- **What slice 3 will need from `0120`:** (a) a camera image mapped to bed millimetres (the calibration's image-to-bed transform, including lens distortion), with a stated accuracy in mm that 0120 measures and shows (print and cut needs at most 0.3 mm, a Proposal); (b) a still snapshot for the maker to click the marks (live image later); (c) a way to ask "where is this pixel in machine mm" as a pure function, so the marks' pixel positions turn into measured positions with no new fit code; (d) for automatic detection, the camera's exposure and focus fixed during a measure (0120's problem). Until `0120` has (a), slice 3 is not Ready.
- **What slice 1 needs from `0118` machine control:** nothing, but the position read already built in `0064`/`0127` (G4, work position in the status report; if the connector reports only machine position, the architect decides the offset). Jog and Go to mark (slice 2) need `0118`'s jog.
- **Needs from `0100`/`0098`:** the placed page with move, turn and scale, and a Drawing page kind (both Done).
- **Related:** `0140` (Store position of criterion 10 is the same read; if both are built, share it), `0029` (job origin and Start from: Apply changes where the sheet is, not how the job starts; Head position start stays independent), `0063` (page kinds: a plotter kind would add the cutter family later).

## File and format notes

- A new leaf kind `registration_mark` (centre, number, diameter, line width) inside an ordinary group tagged as the marks object, in the Drawing page's document model (Frage 333 A), and the Print and cut register `x.curvyo.laser.print_and_cut` on the placed sheet's placement on the laser page (up to four measured positions in micrometres, the tolerance, the Reset matrix). Needs a `format_version` change and goldens `print_and_cut_v55.curvyo` plus `print_and_cut_damaged_v55.curvyo`; an older build refuses the file (ADR 0081 decision 9; no silent plain shapes that could be cut). Format 55 is "next free at merge" (main is at 54).
- SVG export writes the marks as ordinary shapes with `data-curvyo-kind="registration-mark"` **(Proposal)**; SVG import does not restore the kind (plain shapes).
- The job needs no new field: it is derived from the placed sheet (`0029` criterion 32).

## Out of scope

- A camera, detection of marks in an image, perspective correction (slice 3 and `0120`).
- Moving the head from the app, "go to mark", frame to the sheet (`0118`, slice 2).
- A PDF or a print dialog, crop bars, bleed, colour management for the print (Frage 330, slice 4).
- A cutting plotter or its optical sensor, other machine families.
- Automatic sheet outline detection (the contour of a printed shape), edge finding, touch-probe alignment.
- Shear and non-uniform scale from the marks (Frage 331).
- Correcting the red pointer's offset to the beam (the maker is responsible; the machine's profile may get a pointer offset in a later spec).
- Registration marks on Sketch pages or inside a clone.
- Several sheets measured at once on one laser page (the Sheet picker switches; slice 1 fits one at a time).

## Decided questions

None by the customer yet. **Default taken, customer may veto** (each at its default A, recommended; the spec is built on these and a veto at the demo changes the story): Frage 328 A, 329 A, 330 A, 331 A, 332 A, 333 A (below).

## Open questions (all with a default taken, customer may veto)

**Frage 333 (PNC-MODEL): What is a registration mark in the file?** (from ADR 0081, `needs-customer`)
- A: its own object kind (centre, number, size, line width), drawn by Curvyo, inside an ordinary group; never burnt, always printed. Files with marks need the new app version.
- B: ordinary shapes (circle, cross, number text) grouped and tagged; editable like any shape, but the centre is no longer certain and every laser and export path must look for the tag.
- C: not asked.
- Recommendation: A (exact centre, one "never burns" rule). Default if unanswered: A. Default taken, customer may veto.

**Frage 328 (PNC-SHEET): Where does the printed sheet live?**
- A: a Drawing page (artwork and marks); the laser page holds it as a placed page and Apply sets the placed page's placement. The cut and the print come from one design; the drawing is never touched.
- B: the laser page itself holds the artwork and the marks. Simpler, but printing prints the whole bed.
- C: a new page kind "Print and cut". More UI, more format.
- Recommendation: A. Default if unanswered: A.

**Frage 329 (PNC-MARKS): Which mark?**
- A: crosshair inside a circle, numbered, 2 to 4 marks, one shape only.
- B: L-shaped corner marks only (the plotter style).
- C: both, a choice per object.
- Recommendation: A (the centre is unambiguous for a dot on a printed line; one shape tests faster; corner marks are slice 4 with the plotter). Default if unanswered: A.

**Frage 330 (PNC-PRINT): How is the sheet printed in slice 1?**
- A: export SVG (the page picture too) and print in any other program at 100 %; PDF and a print dialog later (slice 4).
- B: build PDF export now.
- C: use the browser's print dialog now.
- Recommendation: A (PDF is its own spec and not needed to try the cut). Default if unanswered: A.

**Frage 331 (PNC-FIT): What does the fit allow?**
- A: move, turn and uniform scale (a similarity) from 2 to 4 marks, with the residual and the 0.5 mm tolerance shown.
- B: move and turn only, scale fixed to 100 %.
- C: a full affine (shear and unequal scale too), which needs a general placement for placed pages.
- Recommendation: A (a printer's scale error is real, shear is rare; C only after an ADR). Default if unanswered: A.

**Frage 332 (PNC-FIRST): Which slice first?**
- A: slice 1 as written (typed or taken head position), camera later.
- B: wait for `0120` and build with the camera only.
- C: slice 1 and slice 2 (jog and Go to mark) together, after `0118` jog.
- Recommendation: A (it needs only what exists and the customer can cut on a print in the first demo). Default if unanswered: A.

## Needed before Ready

Done: `adrs.md` and ADR 0081 (architect, Proposed, `needs-customer` for the document model and file format; accepted with the slice 1 PR), `ux-notes.md` (UX engineer, section 12 reconciled here).

## UX notes

See `ux-notes.md` and the drafts in `docs/design-drafts/2026-11-print-and-cut/`.

## Links

Requirements: R-MFG-CUT-002 (registration-mark alignment), R-MFG-LASER-003; R-SYS-007
ADR: 0081 (Proposed). Depends on: `0100` (placed pages), `0098` (page kinds), `0029` (job, preview), `0064`/`0127` (position read, optional)
Related: `0118` (jog, slice 2), `0120` (camera, slice 3), `0122`, `0140`, `0049`, `0050`, `0063`
PR: none yet
