# Library into slots: drop a finished page or picked elements onto a fixture slot of a laser page

Status: Draft
Priority: Should
Origin: Customer, 2026-11-25 (translated): "We still need to add that from workspaces you can insert single objects of pages, or whole pages, into laser placeholders. That's partly the idea behind the fixture placeholders: that you can simply insert a finished page there." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: `0102` (holders, slots, fit), `0103` (fixture jobs, unchanged), `0113` (fixture sets), `0100` (placed pages, clones, slot snap for placed pages), `0137` (copy import, Came from), `0152` (browser pane, drag, double-click)

## User value

As a maker who lays out a jig (a board with 15 coaster-sized slots) I want to drop a finished page from my library, or a few elements of one, into a slot and have it sit centred and fitted in that slot, or fill all empty slots with it, so that I do not draw, size and centre the same piece by hand fifteen times.

## Field reference

| Tool | What it does |
|---|---|
| LightBurn | Array and Grid Array of a selection; no slots as targets; no library page as source. |
| Inkscape | Clones, Tiled Clones, align to a rectangle by hand; no slot concept. |
| Figma | Drop into a frame or component slot; the content fits the slot's rule. |
| Ours today | `0102` fits a moved item to a slot by centre (0.1 mm size match, turn by 90 degrees, never scales); `0100` places a page of the open document as a live placed page, also snapped to a slot that it fits; `0137` copies a library page or elements into the open document. None of them take a library item onto a slot. |

**What we do better.** The slot is the drop target: it lights, takes the item, centres and fits it, and one gesture can fill every empty slot of a holder. The result is ordinary content the Operations of `0101` already know.

## Terms

- **Library source**: a page or picked elements from the Library tab (side panel), the page preview pane (`0137` criterion 5) or the library browser (`0152`).
- **Slot**: a slot of an Active holder of the active laser page (`0102` criterion 19). A hidden holder (eye off) still counts as Active (`0102` criterion 6); an inactive one is not a target.
- **Filled**: a slot is filled when a member of the laser page (an own item, a group, a clone or a placed page, no fixture) has its box centre inside the slot's outline. It is **computed**, never stored (Proposal; no new register).
- **Fit** (criterion 6): the matrix that places the content in the slot.

## Acceptance criteria

1. Given a laser page that is active with at least one Active holder, and a library source being dragged (from the Library tab row, the page preview pane or a browser tile), when the pointer is over a slot, then the slot lights in the accent colour of `0102`, a badge beside the pointer reads "Into slot <name>" (glyph and word, never colour alone), and a release puts the source into that slot (criterion 5). Over no slot the drag behaves as built (`0137` criteria 4 and 5: a page is added as a new page, elements land at the drop point). Escape during the drag cancels, nothing is written.
2. Given a library source of **elements** (picked in the page preview pane with Select click, Shift+click or marquee, `0137` criterion 5), then they are dropped as one unit: the copies are wrapped in one group named after the source page (Proposal), so they fit, move and scale together; a single picked object is not wrapped. Given a **whole page**, then its members, in their stacking order, are the unit, wrapped in the same way, with the page's frame as the unit's box (so the page's margins count as part of the piece).
3. Given the choice between copy and live link (Frage 377), then, at the default (C, slice 1 copy only): the dropped unit is an **independent copy** exactly as `0137` makes it (Came from on the top-level group, resources embedded, nothing merged with an earlier drop), landing as an own item of the laser page, **Not assigned** (`0100` criterion 3, 155 A). Given the later slice (a placed page): a page of the **open** document is placed as a live placed page of `0100` criterion 13, centred and fitted in the slot (criterion 6); a library page of another document is first added as a new page by `0137` criterion 4 and then placed, in one undo step; elements are never placed live (only a whole page of the open document can be a placed page, `0093`).
4. Given a drop into a slot, then the operation is **one undo step** named "Put into slot" (with the fill of criterion 8 named "Fill slots"); undo removes the copies, a created page and a created placement together and restores everything else exactly, including the selection (Proposal: the new unit is selected after the drop).
5. Given the drop of criterion 1, then the unit is placed by the **fit** of criterion 6 and shows afterwards, in a notice of at most 70 characters, what happened when it was not a plain centre ("Scaled to 74 % and turned to fit <slot>.", "Does not fit <slot>: centred at full size."). A notice is shown, not a dialog (permit over refuse, `0102` criterion 20).
6. Given the fit (Frage 379), then at the default (A, "Fit"): (a) the unit's box is turned by 90 degrees when that makes its box fit the slot's box with the larger scale, never otherwise (a square in a square is never turned, `0102` criterion 25); (b) it is scaled **uniformly, aspect kept**, so its box is contained in the slot's box (for a circle slot, in the slot's inscribed square) with a 0 mm margin, and it may be scaled up as well as down (Proposal); a unit whose size matches the slot within 0.1 mm in either orientation is **not scaled** (the 0.1 mm rule of `0102` criterion 25); (c) its box centre is placed on the slot's centre; (d) a held **Alt** during the drop keeps the real size (centre only, no scaling, turn only for the 0.1 mm match) and says so in the badge. Stroke widths, dashes and text sizes follow the matrix as for any scaled group of `0010`; output-relevant values (power, speed) are not touched. Test: a 60 x 40 mm page into a 30 x 30 mm slot is scaled to 50 % and its box is 30 x 20 mm centred, a 40 x 60 mm page into a 60 x 40 slot is turned and scaled to 66.7 %, a 25 x 25 mm square into a 25.05 x 25 mm slot is only centred.
7. Given a slot that is **already filled** (Frage 380), then at the default (C): the drop is refused for that slot with the notice "<slot> is filled." and nothing is written, and the notice offers one armed button **Replace** (first press arms, second press after 400 ms confirms, as `0081`) that removes the members whose centre lies in the slot (one undo step together with the drop, "Replace slot content") and then puts the unit in. Members of fixtures are never replaced; a placed page that lies in the slot is replaced as a reference (its source stays, `0098` criterion 12).
8. Given a holder (Frage 378 B, **fill all empty slots** in the first slice), then a **Fill empty slots** command, reached from the context line of the dropped unit's drop (a press on the holder's name chip while dragging, Proposal), from the detail bar of the library browser and from the keyboard list of criterion 10, puts one copy of the source into **every empty slot** of that holder, each fitted by criterion 6 to its own slot, as one undo step "Fill slots". Filled slots are skipped and counted in the notice ("Filled 11 slots, 4 were already filled."). With no empty slot it writes nothing and says "No empty slot in <holder>." The order of the copies in the stacking order is the slot order (creation order, `0102` criterion 19). A fill of up to 50 slots with a page of 200 objects takes within 100 ms of thread CPU time in `curvyo-editor-wasm` (test under `tests/cpu_time/`, `CLAUDE.md` §5).
9. Given a drag over a slot that the unit cannot be put in (the slot is damaged and skipped, `0102` criterion 33; the fixture is inactive; the page is not a laser page; no slot), then no slot lights, the badge says the reason in at most 70 characters ("Fixture is not on the bed.") and a release acts as if there were no slot (criterion 1). A drop over a slot of a laser page that is not the active page activates that page as `0098` criterion 8 does and does not drop into the slot (Proposal, one rule for all drops).
10. Given the keyboard, then with a library tile, a page row or picked elements selected, an **Into slot…** button (detail bar of the browser, the Library tab's row actions, the page preview pane's Import elements bar) opens an inline list, no popup: first the Active holders of the active laser page with a **Fill empty slots** row, then their slots as rows with name, shape, size in mm and "filled" when so; arrows move, Enter takes, Escape closes and returns focus to the button (the pattern of `0100` criterion 15). The button is disabled with the reason as its description when the active page is not a laser page or has no slot. Enter on the slot row is the keyboard drop and writes the same undo step as criterion 4.
11. Given what the **job and the Operations** see, then the dropped unit is an ordinary own item (or group) of the laser page, Not assigned until the maker assigns it as `0101` says (Frage 155 A stays); a placed page is a member of the page and is assigned at any depth as `0101` says; a slot, a holder and a fixture job do not change: fixture jobs take fixtures only and the dropped content is a work item (`0103` criterion 10), so it never runs inside a fixture job. Nothing is output by this story (`0029` unchanged). Moving or deleting the holder afterwards does not move the content (the content is not a child of the holder, Proposal; moving both together is the maker's selection).
12. Given a laser page whose machine profile has a fixture set (`0113`), then the content dropped into a slot is **not** part of the profile's set (a set holds fixtures, not work); **Save as profile** and **Update profile** ignore it exactly as they ignore the page's own items.
13. Given the **browser build**, then criteria 1 to 12 hold with the sources of that build (`0152` criterion 24).
14. Given the format, then no `format_version` change at the default: a copy writes what `0137` writes (Came from) and existing kinds; a placed page writes the placement of `0093`; a slot stores nothing about its content (computed "filled"). Should the customer choose a stored slot-content link (not offered), that is a new question.
15. Given the whole slice, then no network access beyond what the library source already needs (`0138` rules unchanged), no device I/O, no G-code, no telemetry (`R-SYS-002`, `R-SYS-007`).
16. Given the tests, then: golden tests in `tests/fixtures/` of a laser page with a holder of 4 slots (rectangle 30 x 30, rectangle 60 x 40, circle 25, a damaged one) and a library page of 60 x 40 mm: single drop, fill of the empty slots, drop onto a filled slot (refused, then Replace), turned fit, circle fit, Alt real size, undo of each, save and reopen; the fit function is a pure function in a `-core` crate (`curvyo-editor-core` or `curvyo-ui-core`, architect) with a table-driven unit test of the fit cases of criterion 6.

## Out of scope

- Slots on other pages than laser pages; screws, clamps, stops and grids as targets (as `0102` criterion 40).
- Dragging a drawing page row of the open document onto a slot **without** the library (it is `0100` criterion 13, already snapped to a slot that it fits); this story only says how the live placed page of Frage 377 B or C is fitted, and does not change `0100`.
- A stored slot-to-content link, so that "replace the content of every slot with another page" or "update all copies" works later (a separate story, needs a format decision); the live link of a placed page is the only kind of link here.
- Arrays by count and spacing, Grid array, nesting; auto-filling a holder with several different pages; choosing a different page per slot (a list of pages for N slots).
- Fitting to the slot's outline shape (a heart in a circle by contour, not by box); a margin or inset field; a slot-wide scale value the maker types.
- Output, machine, or job changes; the content's operations, materials or power.
- Dropping from a library of **another** document's laser page with its own fixtures (the unit is the page's members; fixtures of the source page are never copied, `0102` criterion 7).

## UX notes

(filled in by ux-engineer before Ready: the slot light and badge, the drop-time chip for Fill, the Into slot list, the notice texts of criteria 5, 7 and 8.)

## Open questions (customer)

Frage 377 (SLOT-KIND): Does a slot get an independent copy of the page, or a live placed page (`0100`), or both?
- A: A copy only. Works for every library source, including pages and elements of other documents; changing the source page later changes nothing in the slot.
- B: A live placed page only. Follows the source page on the next frame; only whole pages of the open document, a library page of another document is added as a page first, and picked elements cannot go into a slot.
- C: Both: the drop makes a copy by default, and a live placed page for whole pages of the open document (a switch in the Into slot list and Ctrl while dropping, Proposal). Built in two slices: slice 1 copy, slice 2 placed page.
- Recommendation: C, built in two slices (the copy first, it covers every source). Default if unanswered: C, slice 1 with the copy only.

Frage 378 (SLOT-FILL-ALL): Is "fill all empty slots of a holder with the same page" part of the first slice?
- A: No: slice 1 drops into one slot; Fill empty slots follows as slice 2.
- B: Yes: slice 1 has the single drop and Fill empty slots, as one undo step.
- Recommendation: B (one loop over the same operation and the same fit, and a jig with 15 slots is the reason for the story). Default if unanswered: B.

Frage 379 (SLOT-FIT): How is the content fitted into the slot?
- A: Fit: centred, scaled uniformly (aspect kept, up or down) to fit inside the slot, turned by 90 degrees when that fits better, an exact match (0.1 mm) is not scaled; Alt keeps the real size; the notice says the scale.
- B: Real size only: centred, turned by 90 degrees when that fits, never scaled; a piece larger than the slot overflows with a notice.
- C: Ask every time (a small chooser at the drop: Fit or Real size).
- Recommendation: A (the notice and Alt make the scale visible and reversible; B stays one key away). Default if unanswered: A.

Frage 380 (SLOT-FILLED): What happens when the slot is already filled?
- A: Replace without asking (the old content is removed, one undo step).
- B: Stack on top (the old content stays, the new unit is added over it).
- C: Refuse with the notice "<slot> is filled." and offer an armed Replace button; in a fill, filled slots are skipped and counted.
- Recommendation: C (nothing the maker drew is deleted by a drop; the replacement is one deliberate press, and undo covers it). Default if unanswered: C.

## Links

Requirements: R-MFG-001 to R-MFG-010 (epic `0063`), R-ASSET-001 to -003, R-ASSET-009 (proposed for `0152`)
Depends on: `0102`, `0103`, `0113`, `0100`, `0137`, `0152`
Related: `0141` (generators as a later source), `0153`
PR: none yet
