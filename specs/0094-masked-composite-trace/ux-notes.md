# UX notes: masked and composite trace (0094)

Written 2026-11-04 by the ux-engineer against the Draft `specification.md`. Every item marked
**(Proposal)** is mine and vetoable at the demo. It assumes the spec's defaults for Fragen 131 to
134 (all A); Frage 130 was answered C by the customer on 2026-11-05 (§11); sections that depend on an answer say so. Widths are arithmetic from the merged Trace
bar (inner 308 px at 800 x 600 with the panel open, 588 px closed; 36 px rows; `0089` UX notes
§22.1); the PR re-measures. No hint texts and no success notices: every string below names a
control or states a state. Icons are Lucide, `lucide-react` ^1.50.0 (installed 1.50.0), 16 px,
1.5 px stroke, `currentColor`.

## 1. The Mask control in the bar (slice 1)

**Where (Proposal, differs from the spec's "Mask row sits in Advanced").** Inside or Outside is the
one decision that changes what a masked trace is, so it belongs to the straightforward path of the
bar (§22 principle: choose, set the main value, trace), not to Advanced. It is drawn **only while
the active part has at least one mask path**, so the bar of an unmasked trace is exactly today's.
Fallback if vetoed: the same control as the first item of the Output row in Advanced; nothing else
below changes.

**Position:** one whole item in the flow directly after the View pill (before Method). Item order:
About Trace, Preset, View, **Mask**, Method, its main field, method group, Smooth, Tolerance, Ignore
below, expander, Trace.

**Look:** the segmented look of the View pill, `role="radiogroup"` named "Mask", two **text**
segments "Inside" and "Outside" (28 px high, 64 px wide each, 14 px; selected =
`--toolbar-icon-active-bg` / `--toolbar-icon-active-fg`), led by a non-interactive 16 px
`SquareDashed` glyph (`aria-hidden`, 28 px slot, 4 px gap) that marks the group as the mask. Item
width 28 + 4 + 128 = 160 px. Text, not icon-only segments: Inside and Outside have no pair of
Lucide icons that reads unambiguously (`BoxSelect` / `Frame` both read as "a frame"), and a word
costs 40 px more than an icon. Default Inside on every entry into the tool; not remembered, not in
the document, not in presets (spec criterion 17). One Tab stop; Left/Right/Up/Down move and select
(no wrap); no shortcut. Changing it is not a history step.

**States:**
- No mask path in the active part: the item is not in the DOM (nothing selected besides the image,
  or the first part is "whole image").
- Mask paths outside the image: the control stays; the Trace button is `aria-disabled`, tooltip and
  status line "0 paths: the mask is outside the image." (spec criterion 7).
- Selection of a text, a second image or a sketch with the image: the Mask item is absent; the
  muted row "Use Object to path for text; one image at a time." and the dimmed Trace button
  (spec criterion 14), as the existing "The selection is not an image." row.
- Shift+click on a closed path in the tool adds it to or removes it from the mask paths (spec
  criterion 13): the Mask item appears with the first path and disappears with the last. Hover
  while Shift is held uses the existing `--hover-box`; no new cursor.

*Frage 130 is answered C (2026-11-05):* the selection route and Shift+click stay, and the pick
button of §11 is added. It replaces the non-interactive `SquareDashed` lead glyph of this group and
is present before any mask exists; everything else in this section holds.

## 2. How the mask reads on the canvas

The region is the area of all mask paths together, clipped to the image. The mask paths themselves
stay ordinary objects above the image, drawn as artwork with their selection boxes (Frage 131 A:
nothing marks them as guides). No new colour, no region fill, no extra outline (the stencil edge
of the result coincides with the mask path, which is already visible; a second line would hide
whether the edge is exact).

| View | Inside | Outside |
|---|---|---|
| **Overlay** (default) | The traced images are washed to 35 % as today; **the part of the image outside the region is washed further to 12 %** (a white quad at 88 % clipped to the complement of the region, same layer as the existing wash, under objects above it). The traced outline (1.5 px `--preview-new` over its white casing) exists only inside the region, so it ends on the mask path. | The same, with inside and outside swapped: the image inside the region at 12 %, the outline only outside. |
| **Mask** | Pixels outside the region `#FFF` (spec criterion 10), pixels inside as today (content `#000`, rest `#FFF`). No region outline: the view shows "exactly the mask the tracer reads". A reader tells the region from white paper by the mask path, which is drawn above it as artwork. | Swapped. |
| **Trace** | The result filled black as Create paths makes it; nothing is drawn outside the region (the image is hidden as in this view today), so the area outside is the empty canvas. | Swapped. |
| **Image** | The original, no wash, no outline, no region cue (unchanged); the mask paths are visible as artwork. | Same. |

12 % keeps the discarded area visible as context (the maker sees what he leaves out) and is
distinct from the 35 % wash by 23 points; both are information carried by lightness, not colour.
With several parts (§3) the extra wash is computed for the **active** part only; the other parts'
traced outlines stay in the preview at full strength, so the joined result reads as one drawing.
A stale or stopped preview keeps the 50 % rule of "Trace preview".

## 3. The Parts row (slice 2)

**Where:** the first row of the Advanced block, above the analysis row (the analysis, status and
every control below belong to the active part, so the part is chosen first). **The row is always in the block (Proposal)**, so
that "Add part" is findable; with one unmasked part it shows chip "1" and Add part (the spec's
criterion 18).

**Row (36 px, wraps as whole items, 4 px gaps):**
1. **Chips**, one `role="radiogroup"` "Parts", roving tabindex, one Tab stop.
2. **Add part**, a 28 px icon button, `Plus`, tooltip and name "Add part"; `aria-disabled` and 50 %
   at 8 parts (tooltip "8 parts at most"; a limit, so a reason is allowed).
3. **Remove part**, a 28 px `Trash2` icon button in the row, always visible, not a Tab stop, the
   Inline delete confirm of `0081` (§5).

**Chip:** 28 x 28 px, 4 px radius, the segmented look (`role="radio"`, selected =
`--toolbar-icon-active-bg`); the part's number (14 px tabular, creation order) with a 6 px mark at
the lower right that carries the mode in shape, not colour: a solid square for Inside, an outlined
square for Outside, no mark for "whole image". A chip without mask paths (not the first) is **muted**:
number at 50 %, dashed 1 px border; it contributes nothing. Accessible name per spec criterion 32:
"Part 2 of 3, inside, 2 paths, active" ("whole image", "no paths" where they apply). Tooltip: the
name without "active".

**Pick a chip:** click, Space, Enter, or an arrow (arrows select as they move, like the palette
strip): the part becomes active, its settings load into the bar and Advanced, and the selection
becomes its mask paths plus the image (spec criterion 19). Selecting the image plus paths while a
chip is active assigns them to that part. A mouse press returns focus to the canvas like the other
bar buttons. The bar shows a muted 12 px state label **"Part 2 of 3"** (`--panel-muted-fg`, not a
Tab stop, after the Mask item) whenever there are two or more parts, so that the bar's values are
never mistaken for a global setting while Advanced is closed. The expander also carries the 6 px
accent dot with a 1 px `--toolbar-bg` ring while Advanced is closed and there are two or more parts
(the pattern of the Text bar's row 2); its description names it: "3 parts".

**Add part:** appends chip N+1, active, a copy of the settings of the part that was active, no mask
paths, Inside; it is muted until paths are selected. The selection is cleared to the image alone so
that the next click on a path assigns it (spec criterion 19). Focus stays on the Add button.

**Order:** left to right is creation order and also stacking order, bottom to top: the later part
wins inside its region (Frage 132 A). The order cannot be changed (spec). The tooltip of Add part
needs no explanation of this; the Parts text of `docs/tracing-images.md` and the About Trace page
state it (the one place hint text lives).

**Join:** there is **no Join button.** Create paths, the blue Trace button, is the join (spec
slice 2, one Create paths). With two or more parts its tooltip reads "Create paths (Enter), 3 parts
joined"; its `aria-disabled` rule is "0 paths: no part has anything to trace." when every part is
muted or empty.

*Depends on Frage 132.* A (assumed) is as above. C (no resolution, separate groups) would change
the same button's tooltip to "Create paths (Enter), 3 groups" and §6. B changes nothing in the UI.
*Depends on Frage 133.* A (assumed): the row is empty of history on every entry. B (stored parts)
would restore the chips on entry and needs a way to drop them: a text button "Clear parts" in the
row, not decided here.

## 4. Tab order and keyboard

Tab order in the bar: About Trace, Preset, View, **Pick mask paths**, **Mask** and **Clear mask** (when drawn), Method, main field, method
group, Smooth, Tolerance, Ignore below, Advanced, the Advanced controls, Stop or Trace again,
Create paths, Retry (existing; the pick button, Mask and Clear mask are the new bar stops).
Inside the Advanced block, top to bottom: **Parts** (one stop; Add part; Remove part is no stop),
Analyse, the Output row, then as today.

| Key | Where | Does |
|---|---|---|
| Arrows | Mask group | Move and select Inside / Outside, no wrap |
| Arrows, Home, End | Parts group | Move and make the part active (loads settings and selection) |
| Delete | Focused chip | Arms the Remove part trash for that part; Delete or Enter on the armed chip removes it (§5) |
| Enter / Space | Add part | Adds a part |
| Escape | Armed trash | Disarms only; the next Escape follows the existing rules (control in the block returns focus to the canvas, then leave the tool) |
| Space / Enter | Pick mask paths | Arms or disarms the pick mode (§11) |
| Escape | Canvas or bar, pick mode armed | Disarms the pick mode only (§11); then the existing cascade |
| Shift+click | Canvas, Trace tool | Adds or removes a closed path as mask path of the active part |
| Enter | Tool | Create paths (joins the parts) |

No new letter shortcut, no menu item. The Mask and Parts controls are not reachable by key from
the canvas except by Tab; that matches Preset and View. Every other key of the Trace tool is
unchanged.

## 5. Remove part (Inline delete confirm, `0081`)

The `Trash2` button sits in the Parts row. First press (or Delete on the focused chip) **arms**:
the row takes `--destructive-soft`, the trash becomes the filled `--destructive` square with a white
glyph, no size change; the chip of the part to be removed keeps its number. The second press after
400 ms removes it. Disarms after 4 s, on Escape, on a pointer press elsewhere, when focus leaves
the row, and when the list changes (a chip is picked or a part added). Name and tooltip say what
the next press does: "Remove part 2" then "Press again to remove part 2"; the polite status region
repeats it. No dialog, no notice after the removal (spec criterion 28).

After removal chips renumber, the previous part (or the new first one) becomes active and its
selection loads. Removing the **last** remaining part is not offered: with one part the trash is
`aria-disabled` and 50 %, tooltip "Keep at least one part". Removing part 1 while others exist
makes the next part the first one; if it has mask paths it stops covering the whole image (it keeps
its region); this is stated by its chip losing nothing, so nothing is announced.
A deleted mask path in the document mutes its chip without notice (spec criterion 27).

## 6. What Create paths and Join make

- **One group "Trace of <image name>"**, with one or several parts alike, one History step, selected
  afterwards, no notice (spec criteria 12 and 24; the names follow `0089`, the same group name as
  an unmasked trace).
- **Children:** exactly what an unmasked trace of the same Output makes. Single colour: the
  compound path(s) the single-colour trace makes today, named as today. Multicolour: one filled
  compound path per distinct hex over all parts, named by its hex, largest area at the bottom,
  stroke off (spec criterion 23). Part membership is not visible in the result: no subgroup per
  part, no "Part 2" in any name, because the join closes the seams and a part name would claim a
  boundary that no longer exists. Recognised text lines and shapes sit in the same group as ordinary
  text and primitives, as in `0091` and `0088`.
- **The mask paths are untouched** (Frage 131 A): same ids, nodes, style, stacking and layer, not
  moved into the group, not selected afterwards (the new group is).
- The Layers row tooltip of the group: "Traced: 7 paths, 412 nodes." and, for a join, "Traced: 7
  paths, 412 nodes, 3 parts." Nothing else is added.

*Depends on Frage 131.* B (guides) would turn the mask paths green dashed after Create paths and
make the Layers tooltip add "2 mask paths made guides"; C (deleted) would remove them in the same
History step. Neither changes the bar.
*Depends on Frage 132 C:* the result would be one group per part named "Trace of <image name>, part 2".

## 7. Status line segments

The single status line of the Advanced block (`role="status"`, polite, tabular, 50 % while stale),
segments joined by " · ", in this order; a segment that has nothing to say is absent:
1. Mask (muted), only with a mask: "Inside 2 paths" / "Outside 1 path". With two or more parts this
   segment is replaced by "3 parts" (muted).
2. The counts, as today: "7 paths, 412 nodes, 0.4 s" (Multicolour: "3 colours, 7 paths, ...").
   With several parts these are the counts of the **joined** result.
3. Muted "38 specks ignored", muted "2 regions joined" (the Multicolour segment; joined parts do
   not add a word, the "3 parts" segment already says it).
4. "3 text lines set" / "12 shapes set" as `0091` and `0088`.
5. For a non-first part with nothing to trace, when it is the active part: muted "Part 3 has no
   paths" (a state, no instruction).

Replacing messages (stated once, an empty result hides the counts): "0 paths: the mask is outside
the image." and "0 paths: no part has anything to trace." Notices in the bar's notice row stay
only those §22.2 keeps; nothing about parts or masks is added there.

## 8. Narrow window (800 x 600, panel open, inner 308 px)

- The bar is already full at this width; the Mask item (160 px) and the "Part 2 of 3" label
  (about 72 px) are whole flow items and wrap to the next line like Method and Smooth; dividers
  hidden at row starts. No control shrinks, truncates or turns into an icon.
- The Parts row: chips 28 px with 4 px gaps. Up to 5 chips plus Add and Remove fit one line
  (5 x 28 + 4 x 4 + 8 + 28 + 4 + 28 = 224 px). With 6 to 8 parts the Add and Remove buttons wrap
  to a second 36 px line (8 chips = 252 px); the Parts group stays one Tab stop. No horizontal
  scroll anywhere. The block keeps `min(60vh, 360px)` and its vertical scroll.
- Status line wraps to two lines, never an ellipsis.
- With the panel closed (588 px) Mask and the label fit on the bar's second line as at 308 px;
  nothing else changes.

## 9. Review points for the implementer

Hit targets 28 px minimum for every new control. Contrast: chip number `--toolbar-icon` on
`--toolbar-bg` 8.2:1; muted chip 50 % is informational (its state is also in the dashed border and
the accessible name). Large documents: the extra wash is one clipped quad, tessellated with the
preview, re-done 150 ms after the view rests; with a 500-node mask the Mask view and the preview
stay at 50 % while a new result is computed. Reduced motion: nothing animates.
Undo: Create paths is one step; part and mask changes are not steps (tool state).

## 10. Proposed design-system row text (the implementer applies it)

Add to "Trace tool (`0025`, ...)" in `docs/design-system.md`, after "Trace View pill":

| Trace Mask and Parts (`0094`, 2026-11-04) | **Mask** (slice 1): one flow item after the View pill, drawn only while the active part has a mask path: `role="radiogroup"` "Mask", a 16 px `SquareDashed` glyph (`aria-hidden`) and two 64 x 28px text segments "Inside" and "Outside" in the View pill's segmented look, default Inside on every entry, one Tab stop, arrows move and select, not remembered, not in presets, not a history step. **On the canvas:** the mask paths stay ordinary artwork; Overlay washes the image outside the region (Outside: inside it) to 12 % on top of the 35 % wash and draws the traced outline only in the region; Mask view white outside the region, no outline; Trace view draws nothing outside the region; Image view shows no cue. **Parts** (slice 2): first row of the Advanced block, `role="radiogroup"` "Parts" of 28 x 28px chips (number, 6 px mark: solid square Inside, outlined square Outside, none for the whole image; muted with a dashed border when it has no paths), roving tabindex, arrows select, picking loads the part's settings and its mask paths with the image; `Plus` "Add part" (disabled at 8), `Trash2` with the Inline delete confirm (disabled with one part). With two or more parts the bar shows a muted 12 px "Part 2 of 3" after the Mask item and the expander carries the accent dot while Advanced is closed. Left to right is creation and stacking order, bottom to top; not reorderable. There is no Join button: Create paths joins. Result: one group "Trace of <name>", the children of an unmasked trace, no subgroup or name per part, mask paths untouched; tooltip counts add ", 3 parts". Status line segments in this order: muted "Inside 2 paths" (or "3 parts"), the counts, specks, joined regions, text, shapes, muted "Part 3 has no paths". Narrow window: whole items wrap, 6 to 8 chips move Add and Remove to a second line | `0094` criteria 1, 7, 10, 12, 13, 17 to 19, 24, 27, 28, 32, UX notes §1 to §8. Frage 130 A, 131 A, 132 A, 133 A, 134 A assumed |

Also: the Select bar Trace button row and the "Trace bar" row get the new muted messages ("0
paths: the mask is outside the image.", "Use Object to path for text; one image at a time."), the
item order gains Mask after View, and the Tab order sentence gains Mask and Parts. The
Interaction-conventions rows are unchanged.

## 11. Pick mask paths (slice 1b, Frage 130 C, customer 2026-11-05)

Customer: select the mask paths after entering the Trace tool, with a button in the Trace bar. The
selection route (Select image plus paths) and Shift+click (criterion 13) stay; the button is a
third way to build the same mask set. Items marked **(Proposal)** are vetoable at the demo.

**Where.** The first element of the Mask group, a 28 px toggle that **replaces the lead glyph** of
§1: the group is now always in the bar, one flow item directly after the View pill. Before a mask
exists it is the button alone (28 px); with mask paths it is button, the Inside / Outside radiogroup
and Clear mask (§11.5), 28 + 4 + 128 + 4 + 28 = 192 px. Item order: About Trace, Preset, View,
**Pick mask paths, Mask, Clear mask**, Method, ... Icon: Lucide `SquareDashedMousePointer` (a dashed
region with a pointer; present in the installed lucide-react 1.50.0, as are `MousePointerClick`,
`Crosshair` and `SquareMousePointer`), 16 px, 1.5 px stroke. Name and tooltip "Pick mask paths"
(no shortcut text, no second line). One Tab stop, first of the group.

**The mode.**
1. **Armed:** `aria-pressed="true"`, `--toolbar-icon-active-bg` / `--toolbar-icon-active-fg`;
   disarmed is the transparent bar-button look. State by fill and the attribute, not colour alone.
   The canvas cursor becomes the "Pick carrier cursor" look of `0080` with this glyph: the Select
   arrow with the `SquareDashedMousePointer` glyph at 10 px at its lower right (`cursor_hint`
   "pick-mask"), `crosshair` where custom images are ignored; the same over every point of the canvas
   while armed (the cursor tells the mode, not the target).
2. **Pick:** a click on a path-like object or a group adds it to the mask paths of the active part;
   a click on a picked one removes it. The accepted set is exactly the Select route's (open and
   closed paths, compound paths, rectangles, ellipses, polygons and stars; a group counts through
   its path-like members). The hit rule is its own (2026-11-05, criteria 35 and 38): the traced image is skipped, the topmost mask-eligible object under the pointer wins whether it lies above or below the image, a closed path counts by its interior even without a fill, an open path by its stroke, and a group as the group. The
   mode **stays armed** after every pick.
3. **Refused:** text, a second image and a sketch are not added. The bar's notice row shows the
   existing muted string "Use Object to path for text; one image at a time." for those clicks and
   clears with the next accepted pick or when the mode ends **(Proposal)**. The existing row
   *replaces the controls* when such an object is selected; here it would take the button away
   under the pointer and end the mode, so the object is simply not selected. A click on the traced
   image or on empty canvas does nothing (no selection change, mode stays); double-click on the
   image does not create paths while armed (Enter still does).
4. **How a picked path reads.** Nothing new: it is added to the selection, so it carries the
   ordinary selection box (§2: mask paths are ordinary artwork). While armed, the candidate under
   the pointer gets the existing `--hover-box` outline; no badge, no fill, no new colour. The Mask
   segments appear with the first pick (Inside selected, or the choice already made in this entry)
   and the preview re-traces.
5. **Clear mask (Proposal).** A 28 px `X` icon button, name and tooltip "Clear mask", last in the
   group, drawn only with mask paths, one Tab stop after the radiogroup. It removes all mask paths
   of the active part from the set in one press and leaves the image selected; the paths stay in
   the document untouched. It is not a delete, so no inline confirm; not a history step; no
   notice. The group shrinks back to the button alone. The mode state is unchanged.
6. **Ending.** Pressing the button again; **Escape** (before this story Escape stopped a running
   trace, then left the tool; now, while armed, the first Escape only disarms, with the selection,
   mask and preview kept; the next Escape follows the existing cascade); leaving the tool by `S`,
   another rail tool or Create paths. Never remembered: every entry starts disarmed.

**Keyboard.** Space or Enter on the focused button arms it; no letter or chord (`M` and the
others are not assigned here: the Trace tool has no letter shortcuts for its bar, §4). Picking is
pointer-only, like every canvas pick in the app (`0080` pick mode); the keyboard route to the same
result is the selection route, which stays. A mouse press returns focus to the canvas as for the
other bar buttons; the key press keeps it on the button. Escape from the button disarms and
returns focus to the canvas.

**Status and notices.** `aria-pressed` carries the mode for screen readers; the polite status
region says "Pick mask paths on" / "Pick mask paths off" (not visible). The status line segment
"Inside 2 paths" updates with each pick (§7). No text appears in the bar while armed, no
instruction, no success notice after a pick, a removal, a clear or leaving. A mask outside the
image gives the existing "0 paths: the mask is outside the image.".

**Narrow window (800 x 600, panel open, inner 308 px).** The group is a whole flow item: 28 px
unmasked, 192 px masked; it wraps with its neighbours exactly as §8 says, dividers hidden at row
starts, nothing shrinks. With the "Part 2 of 3" label (about 72 px) after it, the line still breaks
only between items. The PR re-measures.

**Parts (slice 2).** A pick adds to or removes from the **active** part's mask paths; the chip
becomes unmuted with its first path. The mode stays armed when a chip is picked or a part added
(Add part, then picks, is the intended flow); Remove part disarms nothing. Clear mask empties the
active part only. A path may belong to several parts (Frage 132 A resolves the overlap).

**Changes to §1:** the `SquareDashed` lead glyph is gone (its role, naming the group, passes to the
pick button); the Mask item is "not in the DOM" only for its radiogroup, the group is always there.

### Design-system row (the implementer applies it, after "Trace Mask and Parts")

| Trace pick mask paths (`0094` slice 1b, 2026-11-05) | First element of the Trace bar's Mask group, one flow item after the View pill, present before any mask exists: a 28 px toggle, Lucide `SquareDashedMousePointer` 16 px, name and tooltip "Pick mask paths", no shortcut, `aria-pressed`, `--toolbar-icon-active-bg` / `-fg` when armed; replaces the `SquareDashed` lead glyph. Armed: cursor "pick-mask" (Select arrow with the glyph at 10 px, `crosshair` fallback), a click adds or removes the path-like object or group under the pointer to the active part's mask paths (the Select route's set; the hit rule skips the traced image and takes the topmost eligible object under the pointer, a closed path by its interior), the mode stays armed, the candidate gets `--hover-box`, a picked path is an ordinary selected object. Text, a second image and a sketch are refused with the muted notice row "Use Object to path for text; one image at a time."; empty canvas and the image do nothing. Ends by the button, by Escape (first, before stopping a run or leaving), by leaving the tool or Create paths; never remembered. The Mask radiogroup and a 28 px `X` "Clear mask" (removes all mask paths of the active part from the set; not a delete, no confirm, no history step) appear with the first pick. Pointer-only picking; the Select route and Shift+click remain. No hint text, no notice on success | `0094` criteria 34 to 43, UX notes §11. Frage 130 C |

## Changes to the spec (for the product owner)
- Slice 1b and criteria 34 to 43 (§11); Frage 130 answered C.
- "The Mask row sits in the Advanced block" becomes "the Mask control sits in the bar after the
  View pill and is drawn only with mask paths" (§1; Proposal, fallback: Advanced).
- Criterion 18 "Parts row with chip 1 and Add part" holds with the row always present; criterion
  28 adds "with one part the trash is disabled".
- Criterion 32 is kept as written; the Mask segments are named "Inside" and "Outside" (visible text).
