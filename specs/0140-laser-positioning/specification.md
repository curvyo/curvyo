# Laser positioning: a looping frame with fine nudging, and stored zero and reference points as the job's start

Status: Draft (decisions of 2026-11-19 applied; not Ready: blocked on the connector protocol of `0064` and the output of `0029` (rubber-band Frame is also still to specify in `0118`); `adrs.md` and UX notes are missing)
Priority: Should
Origin: Customer (2026-11-19, translated): (12) the laser boundary run (frame) gets a loop mode with a toggle button so the laser keeps tracing the outline continuously to position the work, and the object can be nudged finely meanwhile; (13) store a zero point or several reference points and use them as the start origin of a job. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker placing a part on a sheet I want the laser to keep drawing the job's outline with its red dot while I nudge the part on screen in small steps, and to store the points of my jig or my sheet corner on the machine, so that I place by eye in a few seconds and a repeat job starts exactly where the last one did.

**What other tools do.** LightBurn: Frame (one run) and a "Continuous" frame option in its Frame control that repeats until stopped (name not re-checked here); Start From: Absolute Coords, User Origin, Current Position, with a nine-point Job Origin and "Set Origin" (the exact controls were not re-checked in this session). LaserGRBL: a frame button. We do better: the loop re-sends the box when the part is nudged, so what the laser draws always matches the screen, and the stored points are named and kept per machine.

## Words

- **Frame:** the machine runs the bounding box of the job at low power or with the laser off (R-MFG-LASER-003, `0064` criterion 5).
- **Loop:** the frame repeats until stopped (a toggle button).
- **Reference point:** a named point in **machine coordinates** (mm from the machine's origin corner). One is the **zero point** if the maker names it so.
- **Job start:** how the job's coordinates meet the machine: Absolute (the page's origin corner is the machine's), From a stored point, or From the current head position (Frage 283).

## Acceptance criteria

Frame loop (idea 12; needs `0064` and a connected machine).

1. Given a laser page with a connected machine (`0064`) and a job with output, then the Frame button of the job control has a toggle **Loop** beside it (symbol only, pressed state). Off: Frame runs the outline once as in `0064`. On: Frame runs the outline, then again, without a pause longer than the machine's own travel, until Stop, until the toggle is released (the run ends at the current corner, finishing the lap **(Proposal)**) or until the connection is lost.
2. Given the loop is running, then the laser is on at **frame power only**: the machine profile's frame power (0 % means laser off, the red pointer or none) at the frame speed; the job's own power and speed are never used; a test with a fake connector asserts that no command in the frame stream sets power above the frame power.
3. Given Stop, pause, a lost connection, a closed window, or a quit, then the laser is switched off and the connector stops the frame (the job-state rule of `0064` criterion 6: stop works from a fresh connection).
4. Given the loop running, when the maker changes the job (moves, nudges, scales or rotates the selection or the job's items, or changes the job start), then the frame is re-planned from the new box and the connector switches to it at the end of the current lap or within 300 ms **(Proposal)**, whichever is later, without leaving the loop; no stale box is ever sent after the change has been drawn on screen for longer than 500 ms of thread CPU time of planning.
5. Given the **preview framing is running** (the Frame loop of criterion 1, or the rubber-band frame of `0118` once built) and a laser page, then **Alt+Arrow nudges the selection by 1 mm and Ctrl+Alt+Arrow by 0.1 mm** (Frage 284 A; customer: not general object nudging). A nudge is one undo step per key press, coalesced into one step when keys repeat within 400 ms **(Proposal)**; typed move (`0047`) works as always. The plain arrows keep the steps of `0044` (1 mm, Shift 10 mm) as today.
6. Given no frame is running, then Alt+Arrow and Ctrl+Alt+Arrow do nothing (no nudge on any page kind, and no `preventDefault`: the `0044` rule for Alt/Ctrl+Alt with arrows stays unchanged).
   **Key conflict check (2026-11-19):** the arrow table of `0044` ("Shift allowed; Ctrl, Cmd, Alt: ignored with no `preventDefault`") has no Alt or Ctrl+Alt entry, and `0047` has none either, so no existing binding is taken; this slice adds one row to that table, active only while a frame runs, with `preventDefault` only then. `0060` uses Ctrl+Alt only as a snapping bypass during a pointer gesture; no pointer gesture runs while the key nudges, so there is no overlap. In a browser, Alt+Left/Right is the browser's history; the loop needs a connector (desktop, criterion 7), so it is not an issue there.
7. Given a browser build without a connector, then the Loop toggle is absent (hidden, not disabled).
8. Given the whole loop, then it is safe by default: frame power 0 % unless the maker set it for this machine; the machine profile's "require a local start" (`0064` criterion 9) also guards the first Frame.

Stored points (idea 13; the page parts need no connector).

9. Given a machine profile (`0026`, `0039`), then it has a list **Reference points**: name, X and Y in mm in machine coordinates, kept with the profile, thus synced like the profile (`0116`). Add (a typed point), Rename, Delete (inline confirm), reorder. **Page points (Frage 282 C):** a laser page has the same list of its own points, kept in the page's laser data, not synced; the Start from picker (criterion 11) offers both lists, profile points first, each marked Profile or Page; Store position (criterion 10) adds to the profile list by default and to the page list by a second button.
10. Given a connected machine reporting its position (`0064` criterion 5), then a button **Store position** adds the head's current machine position as a point named "Point <n>"; the maker jogs the head by the machine's own panel or by the app's jog (not in this slice) and presses it. Without a connector the button is absent.
11. Given the laser page's job start (Frage 283), then the page shows a **Start from** row: Absolute, a named reference point (a picker of the profile's and the page's points), or Current position; and a nine-position anchor (the job box's corner, edge middle or centre that is placed on the point) **(Proposal; default: the box's left-bottom corner with the Absolute mode keeps today's behaviour)**.
12. Given a start from a stored point, then the output's coordinates are offset so that the chosen anchor of the job's box sits on the point; the preview (`0029`) shows the box at that machine position inside the work area; a point that puts any part of the job outside the work area gets the warning style and "Outside the work area by 12 mm" and the job still runs or exports (permit over refuse; the machine's own limits stop it).
13. Given the page's work area drawn on the canvas (`0093`, `0102`), then each stored point shows as a small cross with its name at its machine position, hidden by default and shown with an eye in the Reference points list; the origin marker of `0102` stays the profile's origin.
14. Given a fixture grid or pin (`0102`, `0113`), then **Proposal:** a fixture's origin may be adopted as a reference point by one button on the fixture row ("Use as reference point"): the point is a plain copy of the position, not a link. Delete this criterion if the customer says no (Frage 282).
15. Given save and reopen, then the profile's points are in the profile, the page's points and the page's start choice are in the page's laser data; a file without them opens as Absolute, unchanged; `format_version` change by the architect.
16. Given `R-SYS-007`, then no point data leaves the machine except through `0116` sync of the maker's own profile (page points are not synced) and `0064` to the maker's own connector.

## Out of scope

- A jog panel in the app, camera alignment, automatic edge or origin finding, rotary and multi-axis setups, a "go to point" button (a Proposal for later; the frame already moves to the box).
- G-code dialect details for "Current position" (`0029` decides how to make a job relative to the head).
- Frame shapes other than the bounding box (convex hull, rotated box).
- Remote machine safety beyond `0064` (CONN-SAFETY).

## Decisions

Customer, 2026-11-19:

- **Frage 282 (REF-STORE): C.** Both: reference points in the machine profile (all pages of that laser, synced with `0116`) and per page (criterion 9 gains the page list, below).
- **Frage 283 (REF-START): A.** Absolute, From a stored point and From the current head position, with the nine-position anchor (criterion 11).
- **Frage 284 (FRAME-NUDGE): A, narrowed.** Alt+Arrow, but only while the preview framing or rubber-band frame is running, not general object nudging. Default step 1 mm; 0.1 mm with Ctrl+Alt+Arrow (criteria 5 and 6).

## Still open before Ready

- Blocked: `0064` (protocol, position, frame), `0029` (output), and the rubber-band Frame of `0118` (still to specify).
- `adrs.md` (architect) and UX notes (the Loop toggle, the Reference points lists, the Start from row).

## UX notes

(ux-engineer, before Ready.) The Loop toggle in the job control (the shell of `0093`, the connector UI of `0064`), the Reference points list in the machine profile, the Start from row on the page.

## Links

Requirements: R-MFG-LASER-006 (frame loop and nudging), R-MFG-LASER-007 (stored points) (new); R-MFG-LASER-003, R-MFG-011
Depends on: `0064` (protocol, position, frame), `0029` (output), `0093`, `0039`
Related: `0102`, `0113`, `0015` (rulers and origin: the document origin, not the machine's), `0044`, `0116`
PR: none yet
