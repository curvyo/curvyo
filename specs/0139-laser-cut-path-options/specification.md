# Laser cut path options: tabs that hold a part in, start and end of the cut, lead-in, lead-out and overcut

Status: Draft (decisions of 2026-11-19 applied; not Ready: `adrs.md` and UX notes are missing; depends on `0101`, `0111`, output on `0029`)
Priority: Should
Origin: Customer (2026-11-19, translated): (8) tabs or bridges that interrupt a cut so the part does not fall out, width adjustable; (10) advanced laser operation options (Z offset, start point, end point, lead-in, lead-out, overcut), assignable per operation or item. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker cutting a part out of a sheet I want to leave small bridges so the part stays in the sheet until I free it, and to say where each cut starts and ends and how the beam enters and leaves, so that parts do not shift or fall onto the burning sheet and the start mark does not ruin a visible edge.

**What other tools do.** LightBurn: Tabs/Bridges per shape (count, size, skip inner shapes) and, in the Cut Settings editor, Lead-in/Lead-out, Overcut and Start point per shape (the exact LightBurn field names were not re-checked in this session). Fusion 360 CAM: tabs on 2D contours, lead-in and lead-out arcs. Inkscape has none (extensions only). We do better: tabs are plain handles on the path with a width in mm, set on the item or by one operation default, and what the operation draws shows exactly where the gaps are.

## Words

- **Tab (bridge):** a short stretch of a cut path that is **not** cut. The path is closed in the drawing and in the document; only the output leaves the gap.
- **Start point:** where the cut of a closed path begins (a position on the path, shown as a small marker with an arrowhead for the direction).
- **Lead-in, lead-out:** a short extra stretch before the start and after the end that begins or ends **off the part's edge** (inside the waste side for an outer cut), so the entry mark does not land on the edge.
- **Overcut:** the beam continues past the point where a closed path meets its start by a typed distance, so the cut closes cleanly.
- **Z offset** is `0111` criterion 21 (per operation, only with a motorised Z axis). Not repeated here (duplicate).

## Acceptance criteria

Slice A: tabs (customer idea 8).

1. Given a Cut or Cut-out operation on a laser page (`0101`, `0111`), then its unfolded row shows a **Tabs** block: Count (0 to 20, default 0), Width in mm (0.1 to 20, default 0.5 **(Proposal)**), and a switch "Skip inner shapes" (off). Count 0 means none.
2. Given Count n above 0, then n tabs are placed evenly by length on each closed path of the operation's items; given a path shorter than n times 4 times the tab width, then it gets fewer tabs (never overlapping) and the row's summary line says so.
3. Given a selected item that has the operation, then each of its tabs shows a handle on the path: a drag slides it along the path (snapping to nodes within 8 screen px, Alt disables), a click on the path with the Tabs tool of the Select bar's operation group adds one, a click on a handle with Delete removes it; the item then has **its own tabs**, listed as "Own tabs (3)" with a "Back to the operation's" button (the override rule of `0057`).
4. Given a tab, then its width is the operation's Width unless the tab has its own value, typed in an entry chip opened by a double-click on its handle (the entry chip of `0052`); width in mm only.
5. Given the Operations view (`0111` criterion 15), then tabs are drawn as a short gap in the red cut line with a small bar across it; given the normal view they are not drawn at all. They never change the geometry: Object to path, boolean operations and SVG export ignore them.
6. Given the output (`0029`, when built), then every tab is a gap in the cut: the laser is off between the two ends, no power, no dwell; a tab on a curve is measured along the curve (arc length) within 0.01 mm; the travel across the gap is rapid.
7. Given an open path, then tabs apply to it too (a gap in a line); given a Score, Engrave or fill, then the block is absent (hidden, not disabled).
8. Given a group or a placed page (`0100`) that holds the operation, then tabs are per leaf; a linked clone has its own set (it is an item).
9. Given save and reopen, then the operation's defaults and the items' own tabs survive (positions as arc-length fractions of their path, so a path edit moves them with it **(Proposal; architect)**); a file without them opens unchanged.
10. Given any tab change, then it is one undo step with its operation name ("Add tab", "Move tab", "Tab width").

Slice B: path options (customer idea 10).

11. Given a Cut or Cut-out operation row, then it also shows **Lead-in**, **Lead-out** (each: None, Line, Arc; Length 0.1 to 20 mm, default 1 mm **(Proposal)**) and **Overcut** (0 to 10 mm, default 0). Lead-in and lead-out apply to closed paths and start on the waste side: outside the part for an outer outline, inside the hole for a hole (decided by nesting and winding) **(Proposal)**. The values are set on the operation and overridable per item (Frage 280 A): a selected item may carry its own Lead-in, Lead-out and Overcut, listed like own tabs with a "Back to the operation's" button (the override rule of `0057`).
12. Given a closed path, then its **start point** is a marker the maker drags along the path; default is the first node as stored (today's behaviour of the output, not a change). Given a selected item with a start point of its own, then it is listed like own tabs with a "Back to the operation's" button. Start point is per item only (an operation cannot place one point for many shapes).
13. Given an open path, then the cut direction is its start; "Reverse direction" is a button on the item (the path direction display of `R-MFG-EMB-005` is the same tool; when it exists this reuses it). **End point (Frage 281 A, revisitable at the demo):** for an open path the end is the other end, chosen by Reverse direction; for a closed path the end is where the overcut ends (start point plus the overcut distance along the path); there is no separate free end marker and no early-stop gap (that is what tabs are).
14. Given the Operations view, then start point, direction arrowhead, lead-in and lead-out are drawn as thin dashed segments in the operation look (two colours only, `0101` criterion 18); in the normal view only a selected item's start marker is drawn.
15. Given a lead-in or lead-out that would cross another cut path of the same operation within 0.01 mm, then it is drawn in the warning style and the row says "Lead-in crosses a cut"; the job still runs (permit over refuse).
16. Given the output (`0029`), then lead-in and lead-out are cut at the operation's power and speed, overcut continues along the path for its distance; the numbers are tested with fixed geometry; there is no G-code in this slice's own tests beyond a plain-data model of the paths (the G-code slice 0029 consumes it).
17. Given save and reopen, then the operation's values and the items' start points survive in the page's laser data; `format_version` change by the architect.
18. Given the operation-only settings rule of `0111`, then none of these values belongs to a material record or a machine profile.

## Out of scope

- Z offset (in `0111`), lead-in with ramps for the CNC (`R-MFG-CNC-*`), tabs on engraved or filled areas, automatic "tabs only where the part is heavy", corner power and kerf compensation.
- The bridges for islands in lettering (`0068`/`R-FONT-009`): those are drawn geometry in the design, not output gaps.
- The G-code slice `0029` (this entry only holds the data and the preview).
- Scripted or imported tab positions from SVG.

## Decisions

Customer, 2026-11-19:

- **Frage 279 (TABS-WHERE): A.** An operation default (count and width) plus handles per item (criteria 1 to 4).
- **Frage 280 (PATHOPT-LEVEL): A.** Lead-in, lead-out and overcut on the operation, overridable per item (criterion 11).
- **Frage 281 (ENDPOINT-MEANING): A.** Customer unsure; **revisitable at the demo**. Open path: choice of which end the cut starts at; closed path: the end is where the overcut ends (criterion 13).

## Still open before Ready

- `adrs.md` (architect: the laser data of operation and item, arc-length tab positions, `format_version`).
- UX notes (Tabs block and path options in the unfolded row of `0111`; handles on the path).

## UX notes

(ux-engineer, before Ready.) Tabs block and path options inside the unfolded row of `0111`; handles on the path.

## Links

Requirements: R-MFG-LASER-004 (tabs), R-MFG-LASER-005 (path options) (new)
Depends on: `0101`, `0111`
Related: `0112`, `0029` (output), `0057` (override rule), `0068`
PR: none yet
