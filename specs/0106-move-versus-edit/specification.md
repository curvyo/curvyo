# Move versus edit: everything moves anywhere, editing happens in the object's own page context

Status: Done (PR #299; Fragen 164 to 166 answered A by default with the acceptance)
Priority: Must
Origin: Customer (2026-11-05, translated): "Objects can be moved anywhere: pasteboard, over a laser page, over a sketch page. Moving always works; editing only in the context of an active vector page or a matching manufacturing page." Customer direction 2026-11-05 for this slice: moving always works, anywhere (outside a page, over laser pages, over sketch pages); editing only in the context of an active vector page or the matching manufacturing page; Text, Layers, Snapping, Grid and Share tabs exist on laser pages. Frage 156 A (PASTEBOARD-EDIT, answered 2026-11-05: pasteboard objects are editable while a drawing page is active; with a laser or sketch page active they only move; a double-click switches to the last drawing page). Slice 3, part 2 of `specs/0063-manufacturing-workbenches/` (concept A, `concept-output-pages.md` §2.7, sketch i). Split from `0100` on 2026-11-09. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

## User value

As a maker laying out a laser job I want to grab, move, turn and scale any object from anywhere (a part on another drawing page, a leftover on the pasteboard, a clone on the bed), and to change what an object is only where it belongs, with one double-click taking me there, so that I never rewrite a drawing or a bed by accident while I arrange work, and never have to read a message to find out why a tool does nothing.

**What other tools do.** Inkscape edits every object from everywhere (layers lock, but pages are only a frame): moving the wrong thing and node-editing the wrong thing are the same gesture. LightBurn has one bed and no second context, so a part on the bed is always fully editable and a fix means editing the copy. Fusion 360 separates workspaces (Design, Manufacture) by an explicit switch and a part cannot be moved from the wrong one. Affinity and Illustrator lock by layer, not by place. **We do better:** moving never needs a mode; an edit tool simply shows nothing on an object that belongs elsewhere (no nodes, no style fields); one double-click on that object makes its page active and the same object editable, so the fix is one gesture and not a dialog.

## Words (shown in the UI)

- **Move:** change where a whole object is, not what it is.
- **Edit:** change what an object is.
- **Context:** the active page's kind decides what is edited (the table in criterion 1). An object **in context** can be edited; an object **out of context** can only be selected and moved.

## Acceptance criteria

"Member" is the membership read of ADR 0043 §1 (by position for drawing and laser pages, by ownership for a sketch and for a placed page). "Laser page" means a page of kind Laser whose extension is on; a "Laser (off)" or "Unknown kind" page (`0098` criterion 18) counts as a drawing page here, as it does for the tools. "Own item" is as `0098` criterion 11.

### What is in context

1. Given any active page, then an object is in context exactly as this table says, read by the one membership function of ADR 0043 §1 (no second rule), and everything else is out of context:

   | Active page | In context (can be edited) | Out of context (select and move only) |
   |---|---|---|
   | Drawing | its members; pasteboard objects (members of no page, a drawing object lying over a sketch page included; Frage 156 A) | members of any other drawing page, own items, clones and placed pages of any laser page |
   | Laser | its own items; its clones as clones (criterion 11 keeps what `0049` and `0057` give a clone: Placement row, own style of a path root, Unlink clone); its placed pages by placement only (criterion 17 of `0100`) | pasteboard objects; members of any drawing page; items of another laser page |
   | Sketch | its sketch geometry, by the sketch tools | every drawing object, wherever it lies, pasteboard and over the sketch page included |

   An object overlapping a drawing page and a laser page is a member of the laser page (`0100` criterion 5). Test: a project with drawing pages A and B, laser page L, a sketch page S and a rectangle on each, one pasteboard rectangle, one rectangle over S and one rectangle over both B and L; activating A, B, L and S in turn gives for each rectangle the in-context result of the table (for example with A active: the A rectangle, the pasteboard rectangle and the rectangle over S are in context, the other four are not).
2. Given an active page and a press with Select on an object that is out of context, then the object is selected, the active page does not change (no tab, tool rail or panel change, no history step), and the object can be moved at once. A press on an object that is in context behaves as built. A press on empty canvas inside another page's frame still activates that page (`0098` criterion 8). Amends `0098` criterion 8 and generalises `0099` criterion 2, which keeps the drawing page active for a drawing object over a sketch page. **Frage 164 default A.**
3. Given Select, then Shift+press adds or removes an object of any context; a marquee selects every selectable object it touches or encloses in any context (a sketch is never an object, `0099` criterion 4); Select all (Ctrl+A) selects the objects in context of the active page only (Proposal: Select all is the "edit everything here" gesture). While a sketch page is active, a press selects an out-of-context object only where it hits no sketch geometry (sketch geometry wins, as `0099` criterion 3 reads today), and a marquee and Ctrl+A there stay on sketch geometry (Proposal); sketch geometry and drawing objects are never selected together: pressing the other kind replaces the selection, with or without Shift (Proposal). Amends `0099` criterion 3 (its closing remark names this slice).
4. Given a selection that holds at least one out-of-context object, then the whole selection is move-only: every edit command of criterion 9 is refused for all of it and every edit tool shows nothing on it (criteria 9 to 12). No edit command writes to the in-context part alone. **Frage 165 default A.** Test: a rectangle on A and a rectangle on B, A active, both selected, Fill set to red: nothing changes, and with only the A rectangle selected the fill changes.

### Moving always works

5. Given an out-of-context object, then these commands work exactly as they do in context, with the same selection box, the same handles of `0019` and the same snapping (`0060`, `0061`): drag, arrow keys, the typed X, Y, W, H and angle, scale, rotate, skew, Flip (`0056`), align and distribute, **Arrange in a grid** (`0061` criteria 19 to 21; checked in `curvyo-ui-core/src/grid_arrange.rs` and `session/align.rs`: it writes one translation per selected unit and creates or deletes nothing; Proposal: a move command only while it stays so, any variant that creates or deletes objects is an edit), Delete, Duplicate, Copy, Paste and Cut, the Ctrl copy of a move, **Linked clone**, **Place on…** and the cloning drag onto a laser page (`0100` criteria 1 to 6 and 25 hold wherever the selection came from). Each is one undo step with its label and changes only what the same command changes in context (position, matrix, Delete's removal), plus the conversions `0047` already makes on skew. Test: Laser 1 active, an ellipse primitive on drawing page B moved 40 mm right with the arrow keys, rotated by 30 degrees from the box handle and deleted: after each step the outline is where it would be in context within 1e-9 mm, and the ellipse is still a primitive after the rotation; undo restores each state. A move does not change which page is active.
6. Given any selection, then the Layers tab's eye and lock toggles (view state, not shape) and the page list's rename, reorder and delete work as built and are unaffected by context.
7. Given a sketch page is active and a linked clone of a sketch lies on the pasteboard (the placement of `0099` criterion 8), then its selection box has the handles of `0019` like every box (amends `0099` criterion 8 and its Out of scope "until `0100`"): it moves, scales, rotates, skews and flips by handle, drag, arrow keys and typed X, Y, W, H and angle, aligns and distributes, one step each, writing only the clone's own matrix (own placement on, `0057`), and the sketch geometry and the sketch page are never written. The clone stays out of context, so no edit of criterion 9 is offered on it. Test: rotate it by 90 degrees and scale it to 50 %: the drawn outline equals the sketch outline under that matrix within 0.01 mm and the sketch's solved geometry is byte-identical.
8. Given a project with a sketch page and drawing objects lying over it, then the sketch geometry keeps its fixed place (`0099` criterion 4: no box, no handles) and a drawing object over the page is selected and moved as an out-of-context object by criteria 2 and 5. Moving it never moves, adds or removes sketch geometry and a drawing object over a sketch page is still never part of the sketch or its output (`0099` criterion 1).

### Editing shows nothing outside the context

9. Given a selection that is entirely out of context, then none of these commands writes anything, and each is refused as a whole with no history step; a key or chord that reaches a hidden command shows the notice "Not editable here: on <page>." (page name cut at 24 characters; Proposal, the `0100` Ctrl+G notice is the precedent): Node tool changes, nodes and segments of any kind; path operations (Boolean, Combine, Offset, Stroke to path, Object to path, Trace, text carrier, Tiling apply); Style changes of any section (Fill, Stroke, Markers, Spread colours, opacity, Guide flag, the Placement row and Style row of a clone); text content, font, size and the text-on-path commands; Unlink clone; Group, Ungroup, entering a group and drawing into one; Z-order commands (Home, End, Page Up and Down; Proposal: stacking is a change within a page); and sketch geometry, constraints and dimensions. A command is a move command only if criterion 5 lists it; any command not listed is an edit command and is guarded (Proposal: unknown future commands default to guarded). Test: a guard test in the editor session enumerates every command id and asserts it is classified; for each edit command, an out-of-context selection leaves the document byte-identical and the history unchanged.
10. Given an out-of-context object, then edit tools show nothing on it: the Node tool draws no nodes, handles or segment hover and a press on its outline selects it as Select does; the Text tool treats a press on an out-of-context text as a press on empty canvas (it starts a new text where the maker pressed, in the active page's context; Proposal); the Pen, shape and Pencil tools keep creating objects in the active page's context and never join, extend or snap-edit an out-of-context object's nodes (node snapping to it stays, `0060`). No tool is switched away.
11. Given a selection that is entirely out of context, then the Style tab shows no Fill, Stroke, Markers, Spread colours or Placement fields (hidden, not disabled) and one muted line naming where the object lies and the gesture, "On Drawing 2. Double-click to edit." (Proposal; at most 70 characters; the page name cut at 24 characters; "Pasteboard", "Laser 1", "Sketch 1" read the same; wording by UX notes), and the Select bar offers the move commands of criterion 5 and none of the edit buttons of criterion 9 (hidden, not disabled). Given a selection of in-context objects only, the tab and bar are as built.
12. Given a laser page active, then its in-context objects (own items, clones, placed pages) get exactly the commands `0049`, `0057`, `0098` criterion 11 and `0100` criteria 8 to 10 and 17 to 19 give them, and nothing new: a clone's shape is its source's and is never node-edited there, a placed page is moved, turned, scaled, duplicated and deleted only.

### A double-click activates the context

13. Given an out-of-context object on a drawing page D, when the maker double-clicks it with Select (or with an edit tool active), then D becomes active in that press, the object stays selected and is now in context, the tool rail and tabs follow the kind and the panel tab follows `0098` criterion 9, and nothing else happens: no entering of a group, no text editing, no node edit on that double-click. The activation is not a history step. A further double-click then behaves as built (a group is entered, a text is edited, the Node tool shows nodes). Test: A active, a group on B double-clicked: B active, the group selected, not entered; the second double-click enters it.
14. Given a pasteboard object and a laser or sketch page active (Frage 156 A), when the maker double-clicks it, then the **last active drawing page** becomes active and the object stays selected (editable there as pasteboard object, criterion 1). The last active drawing page is the drawing page most recently active in this session; if there is none or it was deleted, the first drawing page in list order; it is kept in the session and not saved. Exception (Proposal): a linked clone of a sketch on the pasteboard is opened by `0099` criterion 13, which is more specific: a double-click opens its sketch page and this criterion does not apply to it. If the project has no drawing page, nothing is activated and a notice reads "There is no drawing page to edit this on." (Proposal, 70 characters at most). With a drawing page already active the pasteboard object is in context and a double-click is as built.
15. Given an own item or a clone of laser page L, or a placed page, while any other page is active, when the maker double-clicks it, then L becomes active and the object stays selected. While L is active a double-click on its own item or clone is as built, including the clone double-click of `0049`/`0066` (Select original) and `0099` criterion 13 (a clone of a sketch opens its sketch page). A placed page of a drawing page D double-clicked while L is active activates D and selects the topmost source object under the pointer (as the clone's Select original selects its original, with the same view change; an empty frame area activates D and selects nothing); this replaces the notice of `0093` criterion 7 and `0100` criterion 19.
16. Given a drawing object over a sketch page with that sketch page active, when the maker double-clicks it, then the object's page (criterion 1: the drawing page it is a member of, else the last active drawing page as criterion 14) becomes active. Given the sketch geometry of an active sketch page, a double-click is as built (`0066`).

### The laser page's own Layers tree

17. Given a laser page is active, then the Layers tab shows a tree rooted at the page (name, the Laser glyph and word of `0098` criterion 6) in place of the document layer tree: **Placed pages and own items in the order the canvas draws them, topmost first, then the Fixtures row** (the engraved grid of `0093`; Proposal: an eye that is the same state as the eye of the Fixtures block in the Document tab, `0098` criterion 13; no selection or editing of fixtures until `0102`). Drawing and sketch pages keep the Layers tab of `0050` as built; on a drawing page a row of an object lying on a laser page is listed and selects it as an out-of-context object (criterion 2). **Frage 166 default A.**
18. Given the laser page's tree, then each placed page is one row "Placed page of <name>" (glyph, name cut at 40 characters), expandable to the members of its source page shown live and read-only (dimmed, no eye, no lock, no drag; pressing an inner row selects the placed page; Proposal: the inner rows exist so that `0101` can assign operations at any depth); each own item and group is a row as in `0050` (eye, lock, rename, expand for groups, drag to reorder inside the page, Delete); each clone is a row with the `Link2` glyph, not expandable (`0100` UX notes); pressing a row selects the object on the canvas and scrolls it into view; a selection on the canvas marks its rows and expands to them. A row shows no layer name and no document layer; the eye and lock of the document layer an object lies on still hide and lock it, shown on its row as inherited state as `0050` shows a hidden group's members (Proposal).
19. Given a placed page row, then Delete removes the reference only (`0098` criterion 12), the eye and lock are not shown in this slice (the placed page's Show comes with `0101`; no new register is written), and the row cannot be dragged to another laser page or reordered against own items (Proposal: the order is as `0100` draws it).
20. Given a laser page with 1,000 own items and 20 placed pages of 50 objects each, then pressing the Layers tab shows the first rows within 100 ms, the list holds at most 60 row elements at any time, and the scrollbar is exact from the first frame (the budgets of `0050` criterion 48), also with every placed page expanded. The tree keeps `aria-level`, `aria-posinset`, `aria-setsize`, `aria-rowcount` and the roving focus of `0050` (UX notes).

### Files, budgets, no I/O

21. Given any project saved by the build of `0100`, then it opens unchanged and is not rewritten on open; this slice writes no new register and does not change `format_version` (38; the last active drawing page, the context of a selection and the tree are session state or derived). A project saved with a selection or an active page of any kind reopens as any project does. The golden `bring_content_in_v38.curvyo` still passes.
22. Given a document of 1,000 objects on 10 pages, then (a) classifying a selection of 1,000 objects as in or out of context, and (b) each press, marquee step and double-click of this slice, takes within 16 ms of thread CPU time in `curvyo-editor-wasm` (test under `curvyo-editor-wasm/tests/cpu_time/`; budgets confirmed in `adrs.md`), and moving 1,000 selected out-of-context objects by drag costs per pointer move what moving them in context costs (within 10 %).
23. Given the whole slice, then there is no network access, no device I/O, no G-code and no telemetry (`R-SYS-002`, `R-SYS-007`).

## Changes to accepted specs

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0098` | 8 | a press inside a page's frame activates it | a press on an out-of-context object selects it and keeps the active page; an empty press, the row and the page keys activate as built (2, 13) |
| `0098` | 11 | an object made on a laser page is selected, moved, styled as on a drawing page | unchanged for the active laser page; seen from any other page it is out of context (1, 5, 9) |
| `0099` | 2, 3 | a drawing object over a sketch page is node-edited and styled as a pasteboard object while a drawing page is active; sketch tools never pick a drawing object; move-only selection comes with `0100` | unchanged for the first; sketch Select presses select a drawing object move-only where no sketch geometry is hit (3) |
| `0099` | 8, Out of scope | the pasteboard clone shows a box without handles until `0100` | handles (7) |
| `0100` | 19, Out of scope | double-click inside a placed page keeps the `0093` notice until `0106`; the laser page's Layers tree is `0106` | double-click activates the source page (15); the tree (17 to 19) |
| `0093` | 7 | the inside of a placed page is read-only with a notice on double-click | read-only inside, double-click activates the source page (15) |
| `0050` | Layers tab | one document layer tree on every page | a page tree on laser pages; unchanged elsewhere (17) |

## Out of scope

- Operations, assignment and Show/Output on the tree's rows, and an eye or lock on placed page rows (`0101`).
- Fixtures as selectable objects, their editing and their tree rows (`0102`); fixture jobs (`0103`).
- Editing inside a placed page, node-editing a clone's shape on the bed, a placed page moved to another laser page (criterion 18 of `0100` stands).
- Locking a page's content against moving; a "lock" for out-of-context objects other than the existing lock.
- A general kind change of a page (`0104`); Paste as linked clone (needs the object clipboard).
- Auto-pan, a mode switch, a modal message or a toast when a tool shows nothing: the double-click is the answer. The only texts are criterion 11's muted line and the notice of criterion 9.
- Marquee that skips out-of-context objects (Select all does; see criterion 3, vetoable).

## Open customer questions

Defaults apply until the customer answers; none blocks the build.

**Frage 164 (PRESS-KEEPS-PAGE): Does a press on an object of another page switch to that page?** (Changes accepted behaviour: `0098` criterion 8 activates a page by a press inside its frame.)
- A: no. The press selects the object and moves it; the active page stays; a double-click switches (criteria 2, 13). Without this the move-versus-edit split cannot exist for objects over other drawing pages.
- B: yes, as today; only pasteboard objects and objects over laser and sketch pages are move-only while another kind is active.
- Recommendation: A (it is what "moving always works, editing in the context" means, and what Frage 156 A and the double-click assume).
- Default if unanswered: A.

**Frage 165 (MIXED-SELECTION): What happens to a selection of in-context and out-of-context objects together?**
- A: the whole selection is move-only; edit tools show nothing (criterion 4). One predictable rule.
- B: edits apply to the in-context objects only; the rest only move with them.
- C: Shift+press cannot add an out-of-context object to an in-context selection (and the reverse).
- Recommendation: A (B writes to a subset the maker may not see; C blocks the marquee over page borders).
- Default if unanswered: A.

**Frage 166 (LASER-LAYERS): What is "the laser page's own Layers tree"?**
- A: on a laser page the Layers tab shows a tree rooted at the page: placed pages (expandable, live, read-only inside), own items and clones in canvas order, then Fixtures (criteria 17 to 19). Document layers are not shown there.
- B: the document layer tree of `0050` stays, with the page's placed pages and a Fixtures row added at the top.
- Recommendation: A (the customer's wording is "its own tree", and `0101` hangs operations on exactly these rows; B hides placed pages inside a layer list that holds objects of every page).
- Default if unanswered: A.

## UX notes

`ux-notes.md` (ux-engineer, 2026-11-09): the selected out-of-context object, the Style line, double-click and Enter to switch context, the laser page's own Layers tree, edit tools on foreign objects and the refusal text. Proposals in it are vetoable at the demo.

## Links

Requirements: R-EDIT-038, R-MFG-008, R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 3, part 2; concept `concept-output-pages.md` §2.7, sketch i; Frage 156 A)
Depends on: `0098`, `0099`, `0100` (all Done), `0050`, `0049`, `0057`, `0060`, `0061`, `0019`, `0047`
Next: `0101` `laser-operations` (hangs operations on the rows of criteria 17 to 19); `0102` `laser-fixtures`
Changes: `specs/0098-page-kinds/`, `specs/0099-sketch-pages/`, `specs/0100-bring-content-in/`, `specs/0093-laser-workbench-shell/`, `specs/0050-layers-and-objects-panel/`
ADRs: `adrs.md` (architect, to do): the single context classification (one function beside the ADR 0043 §1 membership read, a guard test that every command id is move or edit), the page tree for laser pages and its virtualisation, and where the "last active drawing page" lives (session state, not the file).
Format: no `format_version` change expected (criterion 21).
PR: -
