# UX notes: centre lines

Status: Revised after the third demo (2026-11-18) for `0117`. Every choice here is **(Proposal)**, vetoable at the demo, unless it names a customer decision. Patterns come from `docs/design-system.md`: "Select bar layout" (pill, 36 px rows, dividers), "Value field" with "Signed bar", "Trace value field", "Trace info tab and page", "Trace progress slot", "Trace preview", "A preview is never a commit", "Switch", "Action notice" and "Width editing".

Customer decisions of 2026-11-18 behind this revision: the operation lives in the **Node tool** (not the Select bar); the options are the app's usual **sliders**; a **live preview** while a slider moves; an **i** button opens an info page on the right, as in Trace; the options sit in the top bar, never in a popup; **Thick parts** (mm) replaces the blob ratio.

## 1. Entry points

- **Node bar:** a group of its own, `role="group"` "Centre lines", after the Close path group and before Edit width (Edit width stays the last control, row "Edit width"). It holds one text button **"Find centre lines…"** with the Lucide `Route` glyph (16 px, 1.5 px stroke) before the label, 28 px high, 14 px type, in the border look of Close path's Sharp and Smooth. A 1 px 20 px divider at 25 % sits before it, as before Close path. The group wraps as a whole.
- **Not in the Select bar** any more (customer 2026-11-18). The Select bar's button and its group are removed.
- **When it is in the tree:** whenever the Node tool is active and something is selected, so nothing moves in the Node bar when the selection changes. It is also shown in the Node bar's clone-only state: after the existing `cloneOnly` text ("Linked clones have no nodes. Edit the original or unlink the clone.") the bar shows the divider and the group, so criterion 16 can be reached.
- **Context menus:**
  - Node tool: the item "Find centre lines…" with the `Route` glyph, last in the menu after a separator. It does what the button does.
  - Select tool **(Proposal)**: the menu of round 5 keeps its one item. A press switches to the Node tool with the same selection and opens the Centre lines bar there, so a right click is still a short way in.
  - The existing exceptions apply: no menu while drawing, with the eyedropper, or while picking a text-on-path carrier.
- **Keyboard:** the button is a Tab stop of the Node bar; Enter or Space opens the bar with focus on Overlap. Shift+F10 or the Menu key opens the context menu at the centre of the selection box. No letter shortcut.
- **No popups:** the options are a bar in the top bar (§3), never a popover or floating chip.

## 2. Selection, enabled state and refusals

- **What counts:** the objects the Node tool shows, that is the selection (its editing set), at any depth in a selected group. Primitives and linked clones count although their nodes are not editable here. **Selected nodes do not matter**: the operation works on whole objects, never on a node subset. A subject is as in the spec's Terms (filled, closed, visible, unlocked).
- **Enabled** when the selection holds at least one subject.
- **Dimmed** otherwise, in the look of the Node bar's Join: `aria-disabled` at 40 %, Tab stop and tooltip kept, no hover fill. Unlike Join, a press on the dimmed button answers with the refusal (criterion 4). It makes no history entry and opens no bar. In the menu the item is dimmed the same way but stays selectable.
- **Refusal texts**, first reason in this order:
  - "Convert text to path first." when the first non-subject is a text.
  - "Trace the image first." when it is an image.
  - "Select filled shapes." for everything else (open paths, stroke-only shapes, guides, hidden or locked objects, nothing selected).
- **Tooltip:** enabled "Find centre lines"; dimmed "Find centre lines: " plus the reason in lower case ("Find centre lines: select filled shapes.").
- **Where a refusal shows:** an Action notice in the refusal look (`role="alert"`, 8 s), 4 px under the button, right-aligned (the Node bar's notice slot). From the menu: the same place. From Shift+F10 or the Menu key: the canvas notice slot.
- **Failed** **(Proposal, implementer 2026-11-19):** when the worker cannot answer (it crashed or ran out of memory), the bar closes and the refusal look says "Centre lines failed. Nothing changed."; the document is untouched.
- **Too large:** above 100,000 nodes the press is refused before the bar opens: "Too large: 123,456 nodes. The limit is 100,000." (criterion 18; the interim 20,000 is gone because preview and Apply run in the worker).

## 3. Centre lines bar

While open, the bar **replaces the Node bar in place** (left-aligned at `--rail-right` + 12, the Select bar pill, 36 px rows, 8 px between items, wrapping by whole items, dividers hidden at row starts), as the Trace bar replaces the Select bar. `role="toolbar"` "Find centre lines". The Node tool stays pressed in the rail. Leaving (Apply, Cancel, Escape) gives the Node bar back.

Order (also the Tab order): **i**, Overlap, Thick parts, Optimise, Keep inner areas, Offset (only while Keep inner areas is on), then at the far right (`margin-left: auto`) Cancel and Apply. No visible name label at the start, as in Trace (Frage 84).

### 3.1 The sliders

The four values are the app's usual slider: the shared `ValueField` through the `TraceField` adapter (row "Trace value field"), **150 x 28 px**, label inside at the left, value and unit at the right, the fill bar behind them, no reset slot. It is one control: there is no second number box beside it. A click or drag sets the value, a double-click, Enter or F2 opens the number entry, Home and End go to the track's ends, the tooltip is the two-line `SliderTip`. Fields that cross zero use the "Signed bar" with its zero mark. Arrow steps follow the value field's rule: Shift is ten times the step, Ctrl a tenth (never below the grid).

| Field (label) | Unit | Track | Default | Arrows / Shift / Ctrl | Shown as | Accessible name |
|---|---|---|---|---|---|---|
| Overlap | % | −50 to +50, linear, signed bar | +20 | 1 / 10 / 1 | integer, "−20 %" | "Overlap, percent of the line width" |
| Thick parts | mm | 0.5 to 20, log (`v = 0.5 · 40^p`); typed up to 100 | 6.0 (see §9) | 0.1 / 1 / 0.01 | one decimal up to 2 decimals typed, "6.0 mm" | "Thick parts, millimetres" |
| Optimise | % | 0 to 100, linear | 75 (CENTRE-DEFAULT A) | 1 / 10 / 1 | integer, "75 %" | "Optimise, percent" |
| Offset | mm | −5 to +5, linear, signed bar | 0.3 | 0.1 / 1 / 0.01 | up to 2 decimals, "0.3 mm" | "Inner area offset, millimetres" |

- **Thick parts range (Proposal), from the orca image (768 px wide):** placed at 96 dpi it is 203 mm wide (0.2646 mm per px), so the outline strokes (6 to 8 px) are 1.6 to 2.1 mm, the thick wave bands (8 to 20 px) 2.1 to 5.3 mm, and the pupil (about 12 px) about 3.2 mm. The log track spends half its length below 3.2 mm, where line art lives, and reaches 20 mm, about four times the orca's widest band, room for the same art placed at A3 or larger. A typed value up to 100 mm is the "maximum" of criterion 12 (nothing stays filled except what no centre line describes, a disc).
- **Keep inner areas:** the "Switch" (off by default), label "Keep inner areas". **Offset** appears beside it only while it is on (hidden, not disabled): the items after it move, which is acceptable because the move follows the maker's own press.
- **Values** are remembered for the session (criterion 15), and a value change is never a history step.
- **Typed input:** the value field's own rules: an invalid entry outlines the field in `--field-invalid` and keeps it open; a number outside the range is clamped. No status line.

### 3.2 The i button and the info page

- **Button:** the Trace bar's `InfoButton` look: 28 px icon toggle, Lucide `Info`, tooltip and name "About centre lines", `aria-pressed` while its page shows, `aria-controls` the page.
- **Page:** a tab of the Properties strip, Lucide `Info`, last, name and tooltip "About centre lines", in the tree **only while the Centre lines bar is open**. It behaves exactly like "About Trace" (`useTraceInfo` with this tab's id, not a copy): never switches by itself; a second press of i, or closing the bar, returns to the tab and the shown or hidden panel state stored when it was chosen. Body 244 px wide, its own scroll, 14 px: intro, then a definition list in bar order, terms `--toolbar-icon` semibold, texts `--panel-muted-fg`. Escape on the page returns the keyboard to the canvas and leaves the page open.
- **Pictures (new, Proposal):** under each entry's text a static before/after strip, 244 x 48 px: two 104 x 40 px frames with a 16 px `ArrowRight` (`--panel-muted-fg`) centred in the 36 px between them. Inline SVG, `aria-hidden`; ink is `--toolbar-icon` (so it follows the theme), the centre line 1 px `--editor-accent`. Customer exception to "no hint text", as About Trace (2026-11-18).
- **Draft texts** (exact strings go to one `centreLinesText.ts`; a test compares them with this list):

| Term | Text (at most two sentences) | Picture: before → after |
|---|---|---|
| Intro | "Find centre lines turns filled line art into paths along the middle of each line, with a width that follows the line. Press Enter or Apply to replace the selected shapes." | none |
| Overlap | "How far a side line reaches into the line it joins, in percent of that line's width. 0 % touches its edge; a negative value leaves a gap." | a filled T → its bar line and a stem line whose start dips into the bar |
| Thick parts | "A part wider than this stays filled as it is; the rest becomes lines. Raise it to turn broad bands into lines too." | a band with a fat blob → the blob filled at a low value, all lines at a high value (CENTRE-THICK-SCOPE B, Proposal) |
| Optimise | "Higher gives fewer nodes and a looser fit; lower follows the shape more closely. Corners keep their detail at every setting." | one wavy line with many node dots → the same line with few dots |
| Keep inner areas | "Also turns each hole inside a shape into a grey filled shape of its own. Off leaves the holes empty." | a filled ring → a closed line with a grey disc inside |
| Offset | "Moves the edge of each inner area inward, in mm. A negative value moves it outward, under the line." | grey disc with a white gap to the ring → grey disc reaching under the ring |

### 3.3 Cancel and Apply

- **Cancel:** a 28 px text button "Cancel" in the bar's button look. Same as Escape.
- **Apply:** the Trace finish button's look (`--editor-accent` fill, white 14 px text, 75 x 28 px), label "Apply", tooltip "Apply (Enter)" followed, once a preview exists, by its counts on a second line ("24 paths, 312 nodes, 2 kept filled").
- **Dimmed Apply** (`aria-disabled`, reason as tooltip and `aria-describedby`, never a notice): "No centre lines: every shape stays filled." when the preview found nothing to convert.

## 4. Live preview, commit, cancel

- **The press only opens the bar** and starts the first preview at the remembered values. **Apply** is the press of criterion 2: it writes **one undo step "Find centre lines"**, commits the result the preview already holds (no recompute on the main thread, criterion 15b), closes the bar and leaves the results selected in the Node tool.
- **When it recomputes** (criterion 15b): on every value change, **including each step of a drag** and each arrow step; typed values on Enter or leaving the entry; the switch at once. Runs go to the worker; a run whose values are no longer current is dropped, so the latest values always win.
- **Look** (customer: "the result outline over a dimmed source"; "A preview is never a commit"):
  - The subjects are washed to 35 % as in "Trace preview" (a quad in the canvas colour at 65 % over each subject, under the objects above it).
  - Over them, the result's painted outline as the "Live preview outline" (1.5 px hollow `--preview-new`, white casing, no fill), so the maker sees where the width profile differs from the source.
  - The centre line of each result path as a 1 px `--preview-new` line with its nodes as 3 px `--preview-new` dots, so Optimise is visible as fewer dots.
  - Kept-filled shapes (Thick parts) drawn at full strength, unwashed: what stays filled stands out.
  - Inner areas in their real look (`#D9D9D9`, no stroke) at full strength.
  - An overlay only: never in the document, History or Layers.
- **What the Node tool shows while the bar is open:** no node glyphs, handles, segment highlight, hover rings, marquee, selection box or Edit width diamonds. The cursor is the arrow. A press on an object does nothing; a press on empty canvas deselects, which cancels (spec 15b). Pan and zoom work as always.
- **Computing indicator:** when a run takes longer than 400 ms, the previous preview is drawn at 50 % (stale) and the Apply button shows Trace's running look (percentage, or "…" without a fraction, and an x "Stop (Esc)"), held at least 300 ms. A stale preview is never committed.
- **Apply while a run is pending:** the button keeps the running look until the run for the current values finishes, then commits; the fields are inert meanwhile. Escape or the x stops it with the document unchanged and the bar open.
- **Long Apply** (up to 15 s at 100,000 nodes, written in slices, criterion 18): the bar stays, fields inert, the Apply button shows the running look with the percentage of slices; the canvas keeps drawing; Escape or the x leaves the document unchanged.
- **Cancel** (Escape, Cancel, deselecting, another tool, Undo or Redo): drops the preview, writes nothing, gives the Node bar back with the same selection. Undo and Redo then do their usual step. Values that were valid stay remembered.
- **Announcements:** one hidden `role="status"` region (polite), always mounted, as in Trace §27: "Updating preview", then "24 paths, 312 nodes, 2 kept filled", or "No centre lines: every shape stays filled.". No visible status text in the bar.

## 5. Result

- **Centre paths** are ordinary paths: Fill off, stroke in the subject's fill colour, Round cap and Round join (main paths and rings); the width comes from the profile; closed rings are closed paths.
- **Branches** are their own paths with Butt caps on both ends; a free round end is a width taper to zero (CENTRE-CAPS A).
- **Inner areas** are filled shapes in `#D9D9D9` (CENTRE-INNERFILL default A), Stroke off.
- **Grouping:** several results of one subject form one group "Centre lines of <subject name>" ("Centre lines" for an unnamed subject); a single result is not grouped (CENTRE-GROUP default A). Order inside, bottom to top: inner areas, main path, branches in skeleton order. Layer names "Centre line", "Branch", "Inner area", "Filled" (a region kept filled).
- **After Apply** the Node tool stays active and shows the results' nodes, so the node count can be judged at once.
- **Notice after Apply:** none after a clean conversion (customer rule of 2026-10-31). Only when something was kept filled or skipped, or a clone was detached: a status Action notice under the Node bar's button, "Centre lines: 18 converted, 2 kept filled, 3 skipped." plus " Linked clones became ordinary paths." when so.

## 6. Editing the width afterwards

Edit width (`0115`) is the last control of the Node bar when **one** stroked path is selected: a single result is selected after Apply, so it is offered at once; in a group, select one path inside it first. Diamonds, wheel and typed width work as in `0115`.

## 7. Keyboard

| Key | Where | Does |
|---|---|---|
| Tab / Shift+Tab | bar | i, Overlap, Thick parts, Optimise, Keep inner areas, Offset, Cancel, Apply |
| Left/Right or Up/Down | a slider | one step and a new preview; held: repeats, the latest value previews |
| Shift / Ctrl + arrow | a slider | ten steps / a tenth step (never below the grid) |
| Home / End | a slider | track minimum / end |
| Enter, F2, a digit | a slider | opens the number entry; Enter in it commits the value and previews |
| Enter | canvas, the switch, i | Apply |
| Ctrl+Enter | anywhere in the bar, also in the entry | Apply |
| Space | the switch | toggles Keep inner areas |
| Escape | typing or dragging | ends that first (typed text or drag dropped) |
| Escape | anywhere else (bar or canvas) | Cancel; a second Escape is the Node tool's own |

A mouse press on i, the switch or a slider hands the keyboard back to the canvas afterwards, a key keeps it there (the Trace bar's rule). Opening from the button by mouse leaves focus on the canvas; by Enter or Space puts it on Overlap.

## 8. Layout

```text
1440 x 1000, panel open: one row, about 1000 px
+------------------------------------------------------------------------------------------------------------+
| (i) [Overlap      20 %] [Thick parts  6.0 mm] [Optimise     75 %] (o) Keep inner areas [Offset  0.3 mm]  Cancel [ Apply ] |
+------------------------------------------------------------------------------------------------------------+
     ^ signed bar, zero mark                                       ^ switch      ^ only while on

800 x 600, panel open: the bar area is 324 px, content 308 px, rows of 36 px
+--------------------------------------+
| (i) [Overlap      20 %]              |   row 1: 28 + 8 + 150
| [Thick parts 6.0 mm] [Optimise 75 %] |   row 2: 150 + 8 + 150 = 308
| (o) Keep inner areas [Offset 0.3 mm] |   row 3: about 140 + 8 + 150 (only while on)
|                     Cancel [ Apply ] |   row 4: at the right
+--------------------------------------+
Keep inner areas off: the switch, Cancel and Apply share row 3 (140 + 8 + 64 + 8 + 75 = 295): 3 rows.

Node bar with the group (800 x 600, wraps by whole groups):
| [kind] | Insert Delete Line Curve Join Split |
| Close path [Sharp] [Smooth] |
| [Route Find centre lines…] [Edit width] |
```

- All four sliders are 150 px so two fit a 308 px row. If "Thick parts" with "99.9 mm" does not keep 10 px between label and value at build, Thick parts takes 171 px (as Ignore below) and its row holds it alone.
- Never over the canvas beyond the top bar area; notices at most 320 px wide and wrap.
- Check both sizes in both themes, the info page at 244 px in both themes.

## 9. Questions

Frage 1 (CENTRE-THICK-SCOPE): Thick parts, whole shape or the part? A: a shape with any part wider than the value stays filled (spec Terms and criterion 12 today). B: only the too-wide part stays filled, the rest becomes lines. Recommendation B, because a Single-colour trace is one compound path and A lets one fat spot keep the whole orca filled. Default **B (Proposal, built since Round A of 2026-11-18)**: the spec's Terms and criterion 12 now say B as a Proposal; the customer decides at the demo.

Frage 2 (CENTRE-BLOBDEFAULT): Thick parts default. A: 6 mm, so the orca's 20 px wave bands (5.3 mm at 96 dpi) become lines. B: 3 mm as in the spec, which keeps every band over about 11 px filled and contradicts criterion 20. Recommendation and default A, built and marked **(Proposal)** in the spec's criterion 12. The pupil (about 3.2 mm) is narrower than the widest band, so no single value both keeps it filled and lines every band; a lone pupil is a disc and stays filled anyway.
