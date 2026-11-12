# 0111 Laser redesign drafts (ux-engineer, 2026-11-12, revised after the customer's answers)

Static screens from the customer's redesign after the PR #333 demo; real tokens, 280px panel body plus its 32px strip, PNG beside each HTML. No spec yet; they supersede `specs/0028-material-test-library/drafts/` (sliders, Judge steps). Dashed outline in a draft = new control.

- `01-select-bar-operations-and-groups`: the five kind buttons as symbols only (tooltip = name) in the Select bar, only for a selection on a laser page; a group selected with one click; a group entered by double-click (read-only crumb, outside dimmed); C: the Operations view toggle at the page label, off.
- `02-test-grid-tool`: Test grid as third tool of the Offset/Measure card; bar row 1 kind, test id, material, every-cell values, Cell size (no captions); row 2 Columns and Rows, each a unit; narrow bar: Rows wraps whole; Engrave variant with the material list open.
- `03-operations-tab-test-grid-rows`: a 5 × 5 test as 26 ordinary rows, one cell unfolded; an Engrave operation unfolded with material values above a hairline and operation-only settings below.
- `04-materials-tab`: list, selected material with per-laser sets (power, speed, passes, interval), Tests by id; new material, a set typed by hand, armed trash.
- `05-test-detail`: one-tap judge table, note, photo (file, camera, crop), "Place on <page>" with the same id, not-lasered test on a page.
- `06-fixtures-tab`: own tab; Machine sets folded (On this page check, in-place rename, armed trash, arrows and drag), a set unfolded, the empty states; Job fixtures (this page only).
- `07-pin-and-screw`: symbol-only add bars, Pin (diameter, Columns and Rows units), gridded Screw, jobs Centre mark (with Size) and Diameters; the pin grid's handles and chip on the canvas.
- `08-job-fixture-centre-snap-and-lock`: placing a pin by centre snapping (0060 marker), Lock in the row and the Select bar, the locked look.
- `09-apply-fixtures-from-page`: on the target page, pick the source page inline, preview, applied copies.

**Decided by the customer (not questions):**
- "Lasered" is set only by Curvyo's output (0029); no button, an internal test hook covers the id rule. Generated operations are ordinary and hand-editable. After lasering, a change of the grid fields or of its operations by hand gives the grid a new id. Test id: 5 characters [0-9A-Z], random, unique against every id in the material store, deleted tests included.
- Frage 204 A: a change in the grid bar regenerates its operations and overwrites hand edits (one undo restores).
- Frage 205 B: a kind button assigns the selection to the page's last operation of that kind, a new one only if none exists; Shift+press always new.
- Frage 206 A: the test grid only moves; no resize handles, size from Cell size and Count.
- Select bar kind buttons are symbols only; the name is in the tooltip and the accessible name.
- Grid bar: test id and labels are always burned (no Burn test ID, no label options); plain fields, no sliders; no captions for kind, test, material, size; gap constant 2 mm; Passes (the every-cell values) and Cell size in row 1; Columns and Rows share one row, each wraps as a whole.
- The Operations/Drawn view switch leaves the tab and becomes a toggle with the Operations symbol (`ListTree`) at the page name on the canvas.
- Crosshatch (`#` toggle), Bidirectional fill (two-arrow toggle), Scan angle, Overscanning (default 2 %) and Z offset live on the operation, not on the material. Z offset is shown only when the laser has a motorised Z axis (machine capability); a focus test comes later.
- Material sets hold power, speed, passes and (Engrave) interval only; amends Frage 194 A. Sets can be typed by hand.

- Fixtures (2026-11-12): own tab, Pages/Document tab without the block. Frage 207 B: a machine has named fixture sets, a laser page turns on any number of them; plus job fixtures of the page only. Pin kind; pins and screws single or gridded; jobs Centre mark (Frage 209 A, cross in a circle, size settable) and Diameters (pin: its circle; screw: head and thread). Job fixtures placed by centre snapping and lockable.
- Frage 210: job fixtures are never mirrored (the design is flipped with the existing tools, out of scope). "Apply fixtures from page…" on the target laser page copies another page's job fixtures, plain and independent, source picked inline.

**Proposals (vetoable):**
- P1 Select bar: the five symbols sit right after the settings switches. Inside an entered group the bar holds only them.
- P2 Group with differently assigned members: the group counts as one item; the row of an operation holding some members shows the mixed tick and "Group 3 in part" in line 2; a press on it assigns the rest. Members are never named in the tab.
- P4 Material choice in the tool bar: a dropdown under the bar (the only popup of a bar); "New material" is named in the Materials tab.
- P5 Cell operation names "<column value> · <row value> · <id>", labels "<id> labels"; unfolded rows show a source line (material, test, cell).
- P6 Tabs on a laser page: Materials (`SwatchBook`) and Fixtures (Holder symbol) after Operations.
- P7 Sets per laser: the page's laser first and open, others folded; "+ <kind> set" per missing kind.
- P8 Test detail replaces the material body (back button with the material's name); the best cell carries check + accent frame.
- P9 Fixtures: slots unfold one level in (superseded in part by P14).
- P10 Operations view toggle: one per laser page at its canvas label, 20 px, `aria-pressed`, name and tooltip "Operations view"; pressed by default (operation looks), unpressed = the drawing's own style; session state, not saved (as 0101 criterion 18).
- P11 Crosshatch, Bidirectional fill, Scan angle and Overscanning only on Engrave and Engrave + outline; Z offset on every kind. Defaults: off, on, 0°, 2 %, 0 mm. Overscanning in % of the speed-derived run-out, as LightBurn.
- P12 Test grid: an Engrave test carries the Crosshatch toggle in row 1; the other operation-only settings take the defaults. The label operation takes the material's Engrave set for the page's laser, else 20 % and 3000 mm/min; text 3 mm.
- P13 A best cell copies only the set values (power, speed, passes, interval) into the material, never the operation-only settings.
- P14 Fixtures tab: Machine sets first, Job fixtures second; set rows folded, check "On this page", count "2 of 3 on this page" in the heading; the page keeps a copy of the sets (Frage 175 A), Update profile writes them ("Fixture sets in profile: N").
- P15 Add bars symbol-only everywhere in the tab; Grid only in sets (hidden while the set has one); job fixtures offer Pin, Screw, Holder, Clamp, Stop. Pin default 6 mm; Spacing hidden at count 1; pin and screw grids do not turn.
- P16 Screw: Head and Thread as fields in its row beside the 0102 knobs (changes 0102 criterion 38). Centre mark default 4 mm. Both jobs run as Score; the toggle is the job's Output (Frage 178 B applies).
- P17 A kind press in Job fixtures arms placement (fixture at the pointer, click drops, Escape ends); Lock in the row and as the only Select bar control of a fixture; locked = no drag, nudge, handles; lock badge.
- P18 Apply: lists only other laser pages with job fixtures, with counts; dashed blue preview; copies added after the page's own, same positions, Lock kept; one undo step.

**Open question**

Frage 211 (SET-ACTIVE): Does a fixture inside a set keep its own "On the bed" check?
- A: no; the set's "On this page" is the only switch, the eye still hides a fixture (a fixture used only sometimes gets its own set).
- B: yes; each fixture keeps its check (0102 criterion 5) inside its set, two levels of switching.
- Recommendation: A (one switch, less clutter). Default if unanswered: A.
