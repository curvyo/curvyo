# UX notes: image trace mask (0114), slices 2 to 5

Status: Draft (ux-engineer, 2026-11-13). Written against `specification.md`, `adrs.md`, ADR 0061,
`plan.md` and the branch `story/image-trace-mask-2` (slice 1 built). Model: Tracyo
`~/tracyo/specs/0002-trace-parts/ux-notes.md` and `specification.md` on its branch
`story/mask-in-parts-row` (PR #16), read-only; Tracyo section numbers are written `T§n`. Rule: **where
Curvyo has the same element, it looks and behaves as Tracyo draws it** (Tracyo copied most of it from
Curvyo; Curvyo has the real cursors and badges Tracyo had to approximate in egui). Own choices are
marked **(Proposal)** and vetoable at the demo. No hint or tip text anywhere in the UI; labels that
name a control or a state stay. Sizes are CSS px at scale 1, measured in Chromium (section 0).

Sources in Curvyo: `frontend/src/components/TraceBar.tsx` (`MaskGroup`, `MaskToggle`, `MaskPill`,
`ProgressSlot`, `TraceBar`), `TraceAdvanced.tsx`, `TraceParts.tsx`, `MoveBadges.tsx`,
`SelectToolbar.tsx` (`Divider`, `useRowStarts`), `hooks/useDeleteArm.ts`, `hooks/useTrace.ts`
(`keyDown`), `hooks/useEditorSession.ts` (secondary press), `components/Canvas.tsx`
(`onContextMenu`), `lib/traceText.ts`, `lib/traceParts.ts`, `lib/cursors.ts`;
`curvyo-render-core/src/trace_mask_overlay.rs`, `pen_preview.rs`, `pen_cue.rs`, `theme.rs`;
`curvyo-editor-wasm/src/session/trace_mask_input.rs`, `trace_mask_tool.rs`, `trace.rs`, `pen.rs`.

## Customer decisions of 2026-11-13 (override the spec and Tracyo's notes)

1. The Lasso is **finished by a right click**; it then closes automatically with a straight side
   from the last corner to the first (3 or more corners; fewer: dropped, nothing stored).
2. **Esc aborts**: it cancels the lasso in progress and discards the shape (adds nothing).
3. The other ways to finish stay: a click on the first corner, a double click, Enter.
4. Ending the tool otherwise (another region tool, another rail tool, switching the part, leaving
   the Trace tool) closes an unfinished lasso automatically.
5. No context menu on the canvas while a lasso is in progress.
6. The Advanced block is **one flowing line** and wraps only when the width does not suffice;
   the first thing to wrap is the Parts group as a whole (label, chips, Add part, trash and the Mask
   group behind them); no divider at a wrap; on a wide window everything stays on one line.
7. Lasso fill rule: non-zero winding.
8. The Mask group leaves the first row of the Trace bar for the Parts row; Pick mask paths,
   Shift+click and "image selected with paths" go (slice 4).

These make the Lasso behave exactly as Curvyo's Pen already does (right click and Enter finish,
Esc discards: `0072` criterion 19a, customer 2026-10-19 and 2026-11-11). Tracyo's T§6 and T§7
("Esc finishes the lasso and releases Lasso") need the same change in Tracyo (for the lead).

## 0. Reference pictures (`reference/`)

Made with `reference/capture.ts` and `reference/capture-canvas.ts` (the DevTools pattern of the
slice 1 check and `frontend/tests/*_browser.test.ts`: Vite dev server, headless Chromium,
`--force-device-scale-factor=1`, run from `frontend/` with
`node --experimental-strip-types <script> <out-dir> <frontend-dir> <picture>`), picture
`~/tracyo/specs/0001-trace-and-save/reference/input-logo.png` (640 x 420), branch at `482ec371`
with the wasm built from it, Method set to Brightness, Single colour, one part. "before" is
Curvyo as it is; "target" is the same page with a **DOM patch** (not React): the Mask group moved
behind the trash in one wrapping unit with a 1 x 20 divider, Pick mask paths removed, Rectangle
drawn with `Square`, an inert copy of it with the `Lasso` glyph. The target pictures were for
geometry; the implementer re-captured 06 to 10 from the real build in slice 4 (2026-11-13,
`reference/capture-real.ts`, same picture, Brightness, Single colour; criterion 25).

| File | What |
|---|---|
| `00-before-1280-advanced-closed` | 1280 x 800, panel open (bar 804 px): Mask group in row 1 at x 184..440 |
| `01-before-1280-advanced-open` | the same with Advanced open |
| `02-before-1280-rectangle-subtract-pressed` | slice 1's pressed Rectangle and Subtract in row 1 |
| `03-before-1280-panel-closed-advanced-open` | 1280 x 800, panel closed (bar max 1084 px) |
| `04-before-1920-advanced-open` | 1920 x 1080, panel open (bar 1412 px) |
| `05-before-800-advanced-open` | 800 x 600, panel open (bar 324 px) |
| `06-target-1280-advanced-open` | the real build at 1280, panel open (bar 804 px, 157 px high) |
| `07-target-1280-panel-closed-advanced-open` | the real build at 1280, panel closed (bar 1084 px) |
| `08-target-1920-advanced-open-grown` | the real build at 1920 with section 2.5's width rule: one line each, bar 1330 px |
| `09-target-1920-advanced-open` | the real build at 1920 with eight parts (was the DOM patch without the width rule): the unit has wrapped as a whole to line 2, Add part dimmed |
| `10-target-800-advanced-open` | the real build at 800 x 600: the Parts group and the Mask group each on their own line, no divider, Analyse on its own line (bar 445 px) |
| `11-before-canvas-rectangle-inside` | slice 1 canvas: rectangle region Inside, 12 % wash outside, black 1 px line |
| `12-before-canvas-subtract-drawing` | slice 1: a subtracted rectangle while drawn (black dashed, no Remove badge, no size chip) |
| `13-before-canvas-rectangle-selected` | slice 1: the selected rectangle (accent 1.5 px outline, square handles) |

Tracyo's `reference/curvyo-24` .. `curvyo-27` (Curvyo's Pen in progress, its close target, the Node
tool's corner nodes, the Remove badge) and `px-18` .. `px-23` (Lasso pressed, drawn, close target,
region, selected) are the canvas models of section 5; they are not copied here.

## 1. The Mask group's controls

Order inside the group: **Rectangle, Lasso, Subtract, Inside | Outside, Clear mask** (Pick mask
paths is gone). Each toggle is the existing `MaskToggle` (`ICON_BUTTON_CLASS`, 28 x 28, radius
8 px (`rounded-md`), glyph 16 px, stroke 1.5, `currentColor`).

| # | Control | Lucide (`lucide-react` 1.50.0) | Name and tooltip (verbatim) | `data-` | Screen reader on press |
|---|---|---|---|---|---|
| 1 | Rectangle | **`Square`** (replaces slice 1's `RectangleHorizontal`) | "Rectangle" | `data-trace-mask-rect` | "Rectangle on" / "Rectangle off" |
| 2 | Lasso | **`Lasso`** (`lasso.mjs`; not `LassoSelect`) | "Lasso" | `data-trace-mask-lasso` | "Lasso on" / "Lasso off" |
| 3 | Subtract | `SquareMinus` (keep) | "Subtract" | `data-trace-mask-subtract` | "Subtract on" / "Subtract off" |
| 4 | Inside \| Outside | `MaskPill` unchanged: `role="radiogroup"` "Mask", two 64 x 28 text segments, 1 px outline `--toolbar-icon` at 40 % | "Inside", "Outside" (no tooltip) | | `aria-checked` |
| 5 | Clear mask | `X` (keep) | "Clear mask" | `data-trace-clear-mask` | nothing |

**Icons.** `Square` because Tracyo uses it and Curvyo's own Rectangle tool on the rail uses it
(`ToolRail.tsx`); `RectangleHorizontal` is a third glyph for the same thing. `SquareMinus` stays (the
Rectangle glyph with a minus, Tracyo's choice). `Lasso` is the customer's name and a rope loop; it
does not read as the Select tool's lasso cursor (an arrow with a dashed squiggle, `cursors.ts`
`"lasso"`), and `LassoSelect` (a loop with a pointer) would.

**Look.** Rest: transparent ground, glyph `--toolbar-icon` `#3A3A3F`, hover `--editor-accent-hover`.
Pressed: `--toolbar-icon-active-bg` `#2F6FEE` ground, glyph `#FFFFFF`, `aria-pressed="true"` (the
class string of the pressed pipette and of slice 1's toggles, `02-before-...`). Focus: 2 px
`--editor-accent` ring outside a 1 px `--toolbar-bg` offset. Never dimmed, except on a locked image
(section 8).

**Behaviour.** Rectangle and Lasso are the two region tools: at most one is pressed; pressing one
releases the other (and closes a lasso in progress, section 6). A region tool stays pressed until
its own press, Esc (section 7) or leaving the tool. Subtract is independent: each press flips it,
with either tool or none; it changes no region already drawn and applies to the regions finished
while it is pressed (a lasso in progress follows it live). Pressing a region tool disarms an armed
pipette, and arming a pipette releases the region tool (closing a lasso in progress first)
**(Proposal: a press on the canvas means one thing, as Pick mask paths and the pipette excluded
each other)**. A mouse press hands the keyboard back to the canvas, a key keeps focus on the button
(as today). Pressing one region tool announces only the new one ("Lasso on"), T§4.

## 2. Layout: the Mask group moves into the Parts row

### 2.1 The first row loses the group

The flow of the first row becomes: About Trace, Preset, **Method**, its main field, its method
group, Smooth, Tolerance, Ignore below, then (far right) the expander and Trace. It is 264 px
narrower (the group's 256 px and one 8 px gap). One-line width with Brightness: **1140 px** at rest,
1148 with Advanced open (was 1404 and 1412). Positions (bar x): i 8..36, Preset 44..176, Method
184..360, Threshold 368..518, Smooth 526..676, Tolerance 684..834, Ignore below 842..1013, the
expander and Trace from the right edge.

| Window (Advanced open, Brightness) | Bar | First row before | First row after |
|---|---|---|---|
| 800 x 600, panel open | 324 | 6 lines, 216 px (i+Preset / Mask / Method / Threshold+Smooth / Tolerance / Ignore+Trace) | **5 lines, 180 px** (the Mask line goes) |
| 1280 x 800, panel open | 804 | 2 lines: i, Preset, Mask, Method, Threshold / Smooth, Tolerance, Ignore, expander, Trace | 2 lines: i, Preset, Method, Threshold, Smooth / Tolerance, Ignore, expander, Trace |
| 1280 x 800, panel closed | 1084 | 2 lines: ... Smooth / Tolerance, Ignore, expander, Trace | 2 lines: ... Ignore below (ends 1013) / expander and Trace alone at the right |
| 1920 x 1080, panel open | max 1444 | 1 line (1412) | 1 line; bar width by section 2.5 |

### 2.2 The Parts and Mask unit

`TraceAdvanced`'s `parts` slot becomes one flow item, the **Parts and Mask unit**: a wrapping row
(`flex flex-wrap items-center`, column gap 25 px, row gap 0) of two children, the existing
`TraceParts` (label "Parts", chips, Add part, trash) and the Mask group (`data-trace-mask`). The
Mask group stays **outside** `TraceParts`' arm zone (`ARM_ZONE`), so the armed trash colours only the
Parts group (T§3: label to trash, not the divider or the Mask group); moving focus from the trash to
Rectangle disarms (focus left the group). The Mask group carries the bar's `Divider` (1 x 20,
`--toolbar-icon` at 25 %, 13 px left of the group, y 8..28 of its line, no width of its own) and is
marked by `useRowStarts` on the unit, so the divider is **not drawn when the Mask group starts a line**.

Widths: Parts group **96 + 32 n** (n parts: 128 with one, 352 with eight); Mask group **256**
(Rectangle 28, Lasso 28, Subtract 28, segments 128, Clear mask 28, 4 px gaps; the same 256 as
today's group with Pick mask paths); unit on one line **377 + 32 n** (409 with one part, 633 with
eight). Positions from the unit's left edge u, one part (measured, `07-target-...` with u = 8,
identical to Tracyo's px numbers): label u+0..32, chip 1 u+36..64, Add part u+68..96, trash
u+100..128, divider at u+140, Rectangle u+153..181, Lasso u+185..213, Subtract u+217..245, Inside
u+249..313, Outside u+313..377, Clear mask u+381..409. Each further part moves everything after the
chips 32 px right. Controls sit 4 px below the line top (28 px in a 36 px line).

### 2.3 The Advanced block as one flowing line

Order unchanged otherwise: Output row (Output, Colours and palette with Multicolour, the four
toggles), Denoise, Upscale, Corners, **the unit**, Analyse (`margin-left: auto`, far right). Items
8 px apart, lines 36 px, the block's `max-h-[min(60vh,360px)]` and scroll unchanged.

**Wrap order (customer 6):** whole items wrap as today; the unit wraps **as a whole** to the next
line before anything inside it breaks; only when the unit alone is wider than the line does the
Mask group go to its own line (starting at the block's padding, no divider); only when the Parts
group alone is wider than the line do Add part and the trash go below the chips together (today's
rule, from 7 parts at 308 px). Analyse follows the last item and stays right-aligned on its line.

| Window (panel, bar) | Advanced lines (measured on the DOM patch) |
|---|---|
| 800 x 600 open, 324 | 7: Output / Denoise / Upscale / Corners / Parts group (8..136) / Mask group (8..264, no divider) / Analyse (237..312). Was 5 (Corners and Parts shared a line). Bar 445 px high (was 409); block 261 px, under its 360 px cap |
| 1280 x 800 open, 804 | 2: Output 8..309, Denoise 317..502, Upscale 510..660 / Corners 8..158, unit 166..575, Analyse 717..792. Bar height unchanged (157) |
| 1280 x 800 closed, 1084 | 2: Output, Denoise, Upscale, Corners (668..818) / unit 8..417, Analyse 997..1072 |
| 1920 x 1080 open, max 1444 | **1**: Output 8..309, Denoise 317..502, Upscale 510..660, Corners 668..818, unit 826..1235, Analyse at the far right (with one to four parts; from five parts the unit wraps) |
| 2560 wide | 1, up to eight parts |

### 2.4 Eight parts

Unit 633 px: chips u+36..288, Add part u+292..320 (`aria-disabled`, 50 %, tooltip "8 parts at
most"), trash u+324..352, divider u+364, Rectangle u+377..405, Lasso u+409..437, Subtract
u+441..469, Mask u+473..601, Clear mask u+605..633 (Tracyo's P18 with u = 8).

### 2.5 The bar's width with Advanced open

Today the Advanced block never widens the bar (`[contain:inline-size]`, customer 2026-10-31: the bar
is as wide as its controls). After the move, the first row needs only 1148 px while the Advanced
line needs **1330 px** (8 + Output 301 + Denoise 185 + Upscale 150 + Corners 150 + unit 409 + Analyse
75 + five 8 px gaps + 4 px right padding + 8), so at 1920 the unit would wrap in a window that has
room (`09-target-...`), which customer 6 rules out. **Rule (Proposal, follows from customer 6):**
with Advanced open the bar is as wide as the wider of its two one-line widths (first row, Advanced
line), at most the available width; the Advanced controls count as controls. Closed, nothing
changes. The Advanced line's one-line width grows 32 px per part and, with Multicolour, by the
Colours field, the pipette and the swatches. Measure it from the items (sum of item widths and
gaps), not from the block's max-content: removing `contain` alone grew the patched bar to the
1444 px cap (`08-target-...`, Analyse 122 px away from the unit). With the rule the bar is 1330 px
at 1920 (one part), and the first row's expander and Trace move to that right edge.

## 3. Tab order

First row: About Trace, Preset, Method, main field, method group, Smooth, Tolerance, Ignore below,
expander, Trace (Stop / Retry while running or stopped). Advanced block (DOM order after Trace):
Output, Colours, palette pipette, palette (Multicolour), Invert, Drop border shapes, Recognise text,
Recognise shapes, Denoise, Upscale, Corners, **Parts (one stop), Add part, Rectangle, Lasso,
Subtract, Mask (one stop), Clear mask**, Analyse. The trash is no Tab stop (as today). No Pick mask
paths stop anywhere. Disabled controls (Add part at 8 parts, a locked image) stay focusable with
`aria-disabled`, as today.

## 4. Tooltips, notices and screen-reader texts

No visible notice or status text in the bar (`0089` §27). Tooltips: Radix, 400 ms, bottom.

| Where | Text (verbatim) | Status |
|---|---|---|
| Rectangle, Lasso, Subtract, Clear mask | "Rectangle", "Lasso", "Subtract", "Clear mask" | Lasso new; others kept |
| Toggle press (hidden polite region of the group) | "Rectangle on" / "Rectangle off", "Lasso on" / "Lasso off", "Subtract on" / "Subtract off" | Lasso new (`maskToggleStatus`) |
| Chips | "Part 1 of 3, inside, 2 paths"; "Part 2 of 3, outside, 1 path"; "Part 3 of 3, no paths"; "Part 1 of 1, whole image"; name adds ", active" | kept; counts are regions (rectangles and lassos alike) |
| Add part | "Add part" / "8 parts at most" | kept |
| Trash | "Remove part 2" / "Press again to remove part 2" / "Keep at least one part"; polite "Press again to remove part 2" on arming | kept |
| Trace button, two or more parts | "Create paths (Enter), 3 parts joined" | kept |
| Dimmed Trace button and hidden region | "0 paths: the mask is outside the image."; "0 paths: the mask paths could not be combined."; "0 paths: no part has anything to trace." | kept; **`MASK_FAILED` loses its second sentence** "Convert unsupported editable curves to Bézier paths." (only mask paths could be editable curves; spec Behaviour 12 wording) |
| Hidden status region after a mask change | "Inside 2 paths" / "Outside 1 path"; "3 parts"; "Part 3 has no paths" | kept |
| Lasso in progress | nothing per corner (the Pen announces none); finishing announces the new mask as any region; a dropped or discarded lasso says nothing **(Proposal, T§4)** | new |
| Expander, Advanced closed, two or more parts | description "3 parts" first (as today) | kept |
| Nothing selected / something else | "Click an image to trace it." / "The selection is not an image. Click an image to trace it." | kept |
| Image with a text, path, sketch or second image | ~~"Use Object to path for text; one image at a time."~~ | **gone** (`TRACE_MIXED`, criterion 21): the bar is the normal bar for the image(s) |
| Pick mask paths | ~~"Pick mask paths"~~, ~~"Pick mask paths on/off"~~, ~~refused-pick notice~~, ~~cursor `pick-mask`~~ | **gone** |

The About Trace page (the one place for explanations) replaces its Mask entry, verbatim:
"Mask: the regions drawn on the picture that limit the trace; Rectangle drags one, Lasso sets its
corners one click at a time and closes them with a right click, a double click, Enter or a click on
the first corner (Esc discards it), Subtract takes the next regions out, Inside traces what lies
inside them, Outside what lies outside, Clear mask deletes the active part's regions." The Parts
entry stays. `traceText.test.ts` compares both with `docs/tracing-images.md`.

## 5. The regions on the canvas

The regions are no document objects: not in Layers, not selectable by the Select tool, not
exported, not snapped to. They are an overlay of the Trace tool (`trace_mask_overlay.rs`), drawn
only while the Trace tool shows the image.

**Drawing order**, bottom to top: the document up to the traced image; the 35 % veil and the 12 %
extra wash, directly above the image and **under the objects above it** (unchanged, `0094` §2);
the objects above the image; then the tool's decorations, **above every object**: region outlines,
the preview outline with its casing, the drag ghost, the hover box, selection box with handles or
corner squares, the drawing feedback (rubber band, close cue); last the DOM badges and chips. A
region is never hidden by an object above the image.

**Coordinates.** Regions live in the picture's frame. Outlines, the selection box, handles and corner
positions follow the image's placement: a turned image turns its regions and their dashed boxes, a
skewed one makes the box a parallelogram, a mirrored one mirrors them. Drawing and dragging map the
pointer into picture pixels (corners on whole pixels, slice 1).

| Item | Look | Zoom | Curvyo source |
|---|---|---|---|
| Extra wash | unchanged: where the active part does not trace, the picture shows at 12 % (LOGO blue `#2B4C9B`: `#B5C1DC` at 35 %, `#E6EAF3` at 12 %); none for a part without regions; only the active part's | | `trace_region.rs` |
| Region outline, added | `#000000`, solid, no fill, **0.25 mm in document units at the view's zoom, at least 1 px** (1 px at 100 %, 3.8 px at 400 %), butt ends, miter joins; a lasso's closing side like the others. **Every part's regions are drawn**; only the active part's are hit **(change: slice 1 draws the active part's only)** | width scales with zoom above 1 px; never thinner than 1 px | slice 1 `OUTLINE_MM` |
| Region outline, subtracted | the same line dashed 4 on, 3 off, screen px, phase restarting at each region's first corner | dash constant on screen | slice 1 `DASH_PX` |
| Hover box | no tool pressed, active part, no drag, pointer over a region's interior or within **8 px** of its outline: `--hover-box` (1 px accent at 65 % over the white casing at 65 %) on the region's box in the picture frame | 1 px | `HOVER_BOX` (change: slice 1 has none; its slop is 4 px) |
| Selected rectangle | **the dashed selection box** (1 px `#2F6FEE`, 4 on / 3 off) on the region's box over its black outline, and 8 resize handles 8 x 8, radius 2, white fill, 1 px `#2F6FEE`; a dragged handle filled `#2F6FEE`. Edge handles only while the shorter side on screen is 24 px or more. **(change: slice 1 draws a solid 1.5 px accent outline and square handles, `13-before-...`)** | constant screen px | `TRANSFORM_RESIZE_HANDLE_*`, `SELECTION_BOX_DASH_PX` |
| Selected lasso | the dashed box as a rectangle's, **no resize handles**; on every corner the Node tool's corner node: 14 x 14 square, `--node-fill` white, 1 px `--node-stroke` `#3A3A3F`; the corner under the pointer gets the 18 px hover ring (accent at 20 %); the selected corner (last pressed, also while dragged) is `#2F6FEE` edge to edge | constant | `NODE_SIZE_PX`, `NODE_STROKE`, `HOVER_RING_DIAMETER_PX` |
| Move, resize, corner drag | the region drawn as itself at the new geometry; its old outline stays as the **drag ghost** (1.5 px `--drag-ghost` over the 1.5 px white casing); box, handles and corners follow; the preview retraces after the release only | constant | row "Drag ghost" |
| Rectangle being drawn | the **live preview outline** (1.5 px `--preview-new`, hollow, over the white casing), dashed 4 / 3 with Subtract; the **Live transform readout** chip 12 px up and right of the pointer: "42.3 × 37.0 mm", the region's size in document mm along the picture's sides, in the display unit **(Proposal: Curvyo's unit; Tracyo shows picture px)**. **(change: slice 1 draws the final black look while dragging and no chip, `12-before-...`)** | constant | rows "Live preview outline", "Live transform readout" |
| Lasso being drawn | **the Pen in progress** (`pen_preview.rs`): the sides set so far `#2F6FEE`, solid, 0.25 mm at zoom and at least 1 px, no casing; every corner a hollow 14 x 14 square, 1 px `#2F6FEE`, filled as the Pen's; the last corner with the 18 px ring; the rubber band from the last corner to the pointer, 1 px `#2F6FEE`, solid; no size chip | sides scale above 1 px, the rest constant | `pen_preview.rs` |
| Lasso close target | 3 or more corners and the pointer within 16 px of the first corner: the first corner gets the 18 px ring; the rubber band is replaced by the closing side from the last corner to the first, 1 px `#2F6FEE`, dashed 6 on / 4 off from the last corner | constant | `pen_cue.rs` |
| Subtracted region being drawn | rectangle: the live outline dashed 4 / 3; lasso: its sides dashed 4 / 3 (rubber band and closing side unchanged); both with the **Remove badge** of `MoveBadges.tsx` (16 px `--accent` disc, 1 px white outer ring, white minus 8 px / 1.5 px, centre 16 px left of and 16 px below the pointer, mirrored right at the canvas edge as the existing badge) from the press (rectangle) or the first corner (lasso) until the region is finished, dropped or discarded, while Subtract is pressed | DOM, constant | row "Modifier badge" (criterion 8) |

**Hit testing**, no region tool pressed, in this order (the first that answers takes the press):
1. a running drag keeps the pointer;
2. the selected rectangle's handles (radius min(16, shorter side on screen / 3), at least 4, nearest
   centre wins) or the selected lasso's corners (16 px, nearest centre wins); corners beat the body;
3. a region of the **active** part: interior (a lasso by its non-zero winding area) or within 8 px of
   its outline; the topmost (last drawn) wins; regions of other parts are never hit;
4. otherwise the Trace tool's existing click (the object hit is selected, empty canvas clears).

With **Rectangle** pressed every left press on the canvas draws, over the picture, outside it or
over other objects; it never selects. With **Lasso** pressed every left click sets a corner. Space
+drag, middle drag and the wheel pan and zoom in every state, also with a lasso in progress.

**Cursors** (Curvyo's own, `lib/cursors.ts`; no new cursor):

| Where | `cursor_hint` |
|---|---|
| canvas with Rectangle or Lasso pressed, also during a drag and over the close target | `crosshair` (slice 1) |
| over a resize handle of the selected rectangle | `resize:<deg>`, the screen angle of that handle in the turned picture frame (the Select tool's rotated double arrow, built-in fallback) |
| over a region or a lasso corner, and during a move or corner drag | default arrow (the Select and Node tools keep it; measured `default` on slice 1) |
| pan, pipette | unchanged (grab / grabbing, eyedropper) |
| ~~`pick-mask`~~ | removed with its image and doc line |

## 6. Drawing and editing

**Rectangle** is unchanged from slice 1 (Shift from the centre, Ctrl a square, under 2 px nothing,
resize Shift about the centre, Ctrl keeps proportions at a corner, not below 1 px).

**Lasso** (spec Behaviour 3 with the customer decisions of 2026-11-13):

| Gesture (Lasso pressed) | Does |
|---|---|
| left click | sets a corner at the press, on the nearest whole picture pixel; the first click starts a lasso. A click on the pixel of the last corner adds nothing. Moving while pressed does nothing (no curves, no freehand) |
| click within 16 px of the first corner (3 or more corners) | finishes, closed |
| double click | finishes; its second click adds no corner (c1, c2, double click at c3 gives three corners, criterion 4) |
| **right click** | **finishes**: closed by a straight side from the last corner to the first; the right press adds no corner; consumed (`preventDefault` on the press and on `contextmenu`, as the Pen's `pen_secondary_press`); with no lasso in progress it does nothing and does not release Lasso |
| Enter (canvas) | finishes (consumed: not Create paths) |
| Backspace | removes the last corner; removing the only one discards the lasso |
| **Esc** | **discards** the lasso in progress; nothing is stored, no History step, nothing announced. **Lasso stays pressed (Proposal: as the Pen keeps its tool after Esc discards its path)**; the next Esc follows section 7 |
| Ctrl+Z | discards the lasso in progress and undoes nothing else (architect's Proposal, now the same result as Esc); redo or a History row click discards it, then acts **(Proposal)** |
| Rectangle or Lasso pressed, a chip picked, Add part, Remove part, the pipette armed, another rail tool or `S`, the Trace button, another image selected (Layers), leaving the Trace tool | **closed automatically** first (stored as finished, one step "Draw lasso" in the part that was active), then the action runs |
| Clear mask | the lasso in progress is dropped with the part's regions; one step "Clear mask" (T§6) |
| Advanced closed, pan, zoom, the window loses focus | nothing ends |
| Shift, Ctrl, Alt | no meaning while drawing or while a corner is dragged |
| Subtract pressed or released while drawing | the lasso follows it live (look, badge); the state at the finish decides |

"Finished" means: 3 or more corners and an area above zero become a region (added, or subtracted
while Subtract is pressed); fewer than 3 corners or all corners on one line: dropped, nothing
stored, no step. A self-crossing lasso covers its non-zero winding area (a figure 8 is filled
wherever covered; holes come only from Subtract). No limit on corners. A finished lasso is drawn at
once as a region (black line), the preview retraces, the status region announces the mask.

**Context menu.** No context menu while a lasso is in progress (customer 5). In the Trace tool the
canvas already suppresses the browser menu (`Canvas.tsx`, tool is not Node); the right press is
consumed before any other handler, so the desktop webview shows none either.

**Double click on the image** never creates paths while Rectangle or Lasso is pressed (as Pick mask
paths armed did); Enter still does when no lasso is in progress.

**Editing a lasso** (no tool pressed): a click selects it (section 5 hit rule); a drag inside or
within 8 px of a side moves it; a drag on a corner moves that corner (whole pixels, also making it
cross itself); arrows move it 1 picture px, Shift 10 px; Delete or Backspace removes the whole
lasso; Esc during a drag restores. No resize handles, no adding or deleting single corners.
History labels: "Draw lasso", "Move region", "Move lasso corner", "Delete region".

## 7. Keyboard

Canvas keys act when no text entry has focus. Ctrl is Cmd on macOS. The Trace tool's Esc cascade,
one step per press (`keys.rs` `escape`, `useTrace` `keyDown`):

1. a region drag running: cancel it (restores);
2. **a lasso in progress: discard it** (customer 2);
3. a running trace: stop it (existing);
4. the pipette armed: disarm it (existing);
5. a region tool pressed: release it;
6. a region selected: deselect it;
7. leave the tool for Select (existing).

Steps 5 and 6 follow Tracyo (release before deselect; the selection is older than a tool pressed
after it); slice 1 and its docs have them the other way round and change. An armed trash takes the
first Esc while focus is in the Parts group (disarm only).

| Key | Where | Does |
|---|---|---|
| Enter | canvas | finish a lasso in progress; else Create paths (existing) |
| Backspace | canvas, lasso in progress | remove its last corner |
| Delete, Backspace | canvas, a region selected, no lasso in progress | remove the region |
| Arrows | canvas, a region selected | move it 1 picture px, Shift 10 px |
| Space, Enter | focused Rectangle, Lasso, Subtract | press or release it |
| Arrows | Mask segments | move and select, no wrap |
| Arrows, Home, End | chips | make that part active, no wrap |
| Delete | focused chip | arm the trash; Delete or Enter again after 400 ms removes |
| Esc | focused chip, trash not armed | focus to the canvas |
| Enter, Space | Add part | add a part |

No letter shortcuts for the region tools (the Trace bar has none, `0094` §4). The keyboard map row
"Trace tool" in `docs/design-system.md` gains "Lasso: finish with Enter, a double click or a right
click; Escape discards it".

## 8. Bar states

- **Image with other objects selected** (a text, a path, a sketch, a second image): the full bar;
  the trace takes the images and image-root clones of the selection, nothing else (criterion 21).
  The Select bar's Trace button is shown while the selection holds at least one image or
  image-root clone **(Proposal; was "nothing but images, or one image with mask paths")**.
- **Two or more images**: the mask belongs to one picture; the region tools have none to draw on.
  See Frage 11 below; default: the Parts and Mask unit is not drawn, the preview image is traced with
  the bar's settings, and each image keeps its own stored mask for Create paths.
- **Locked image or linked clone of a locked root** (adrs 2026-11-12): regions drawn, not hit;
  Rectangle, Lasso, Subtract, Inside / Outside, Clear mask, Add part and the trash `aria-disabled`,
  50 %, tooltip "Locked" **(Proposal: the Layers state word)**; chips stay selectable (to see a part's
  settings), setting edits stay tool state; tracing and Create paths work.
- **Advanced closed**: the unit is not shown; a pressed region tool stays pressed and the canvas
  keeps drawing (T§2).

## 9. Life of the state

Entering the tool: stored parts, part 1 active, no region tool pressed, Subtract released, no region
selected. Region tools stay pressed after each region (as the Pen after each path); a chip pick or
Add part keeps the pressed tool and Subtract. Leaving the tool (or Create paths, which leaves it)
closes a lasso in progress, releases everything; parts and regions stay with the image. Undo and
redo while the tool is open reshow the regions and the active part's settings; a selected region
that an undo removed is deselected.

## 10. Chromium checks (the egui-free equivalents of Tracyo's pixel cases)

Pattern: `frontend/tests/acceptance_0114_*_browser.test.ts` like `acceptance_0089_bar_width_browser`
(DevTools over a Vite server, skipped without `BROWSER`), scale 1, picture `input-logo.png` (copy it
into `frontend/tests/fixtures/` or generate it), Brightness, Single colour, Advanced open. Bar cases
compare DOM rectangles (`getBoundingClientRect`, ± 1 px) and computed styles, not pixels; canvas cases
sample a `Page.captureScreenshot` (± the given tolerance). Regions in picture px: R1 (24, 25)-(324,
235), R2 (344, 315)-(619, 405), R3 (100, 450)-(300, 520) below the picture, L1 corners (60, 270),
(230, 290), (200, 400).

| Case | Tracyo | Setup | Check |
|---|---|---|---|
| B1 Mask not in row 1 | P1, criterion 25 | 1280 x 800, panel closed | no `[data-trace-mask]` inside `[data-trace-row]` outside `[role=group][aria-label=Advanced]`; no element named "Pick mask paths"; the test fails when the group is in row 1 |
| B2 Unit geometry | P1, P7 | as B1 | unit at x 8, line y 117..153; label 8..40, chip 44..72, Add 76..104, trash 108..136, divider x 148 (1 x 20, y 125..145), Rectangle 161..189, Lasso 193..221, Subtract 225..253, Inside 257..321, Outside 321..385, Clear mask 389..417; Analyse right-aligned |
| B3 Wide window, one line | customer 6 | 1920 x 1080, panel open | row 1 one line; every Advanced item on one line (equal top); bar width 1330 ± 2; Analyse's right edge = bar right − 12 |
| B4 Eight parts | P18 | B3 plus seven Add part | unit wrapped to line 2 at x 8 with section 2.4's positions; Add part `aria-disabled` |
| B5 Narrow window | | 800 x 600, panel open | Parts group and Mask group on two lines, both at x 8; the divider not visible (`visibility` or not rendered); Analyse on its own line, right-aligned; no horizontal scroll |
| B6 Pressed looks | P2, P16, P17 | press Rectangle, then Lasso, then Subtract | `aria-pressed` Rectangle false / Lasso true / Subtract true; computed background `rgb(47, 111, 238)` on the pressed, transparent on Rectangle; glyph colour white on pressed |
| B7 Focus | P6 | Shift+Tab from the Mask segments | focus on Subtract, its 2 px ring (box-shadow) present |
| B8 Tab order | | Tab from Corners | the names of section 3, in order, no "Pick mask paths" |
| B9 Tooltips and names | P5 | hover each control 900 ms | the texts of section 4 verbatim; the hidden region says "Lasso on" after pressing Lasso |
| B10 Trash armed | P12 | three parts, first press on the trash | `[data-trace-parts]` background `rgb(247, 211, 207)`; the Mask group and the divider transparent |
| B11 Chips | P8 to P10, P13 | part 1 R1+R2 Inside, part 2 a region Outside (active), part 3 none | chip looks of `0094` unchanged (mark, dashed muted chip), names verbatim |
| B12 Notices | P14, P15 | one part, R3 only, Inside; then Add part | the Trace button `aria-disabled` with "0 paths: the mask is outside the image." / "0 paths: no part has anything to trace." |
| C1 Wash Inside | C1 | R1, R2 Inside, zoom 100 % | inside R1 the LOGO blue `#B5C1DC` ± 3; outside (triangle yellow `#F2B705`) `#FDF6E1` ± 3 |
| C2 Wash Outside | C2 | R1, R2 Outside | inside R1 `#E6EAF3` ± 3; no preview outline inside R1 |
| C3 Region line | C3 | R1 at 100 %, then 400 % | a 1 px `#000000` (± 24) line along R1; 4 px (3.8 rounded) at 400 %; a subtracted region: runs of 4 ± 1 on / 3 ± 1 off |
| C4 Selected rectangle | C4 | R1, Rectangle released, click inside | the dashed accent box on R1's box, 8 handles 8 x 8 white inside an accent ring; no solid accent outline |
| C5 Lasso being drawn | C5 | Lasso, L1's three corners, pointer at (90, 380) | 14 x 14 corner squares with a 1 px `#2F6FEE` edge; sides and rubber band 1 px `#2F6FEE`; the ring round the last corner |
| C6 Close target | C6 | as C5, pointer at (61, 271) | ring round the first corner; dashed `#2F6FEE` closing side, runs of 6 ± 1 on / 4 ± 1 off; no line to the pointer |
| C7 Lasso region | C7 | L1 finished (each of: first corner, double click, Enter, right click), Inside | History gains "Draw lasso" once; 1 px black line on all three sides; inside 35 %, outside 12 %; the four finishes give equal screenshots of the region |
| C8 Selected lasso | C8 | L1 selected, corner 2 pressed | three 14 x 14 white squares with a `#3A3A3F` edge, corner 2 filled `#2F6FEE`; dashed box without handles |
| C9 Remove badge | | Subtract and Rectangle pressed, drag held; then Lasso with one corner | the minus badge's DOM node visible, centre 16 px left of and 16 px below the pointer; gone after the release or Esc |
| C10 Right click | customer 1, 5 | three corners, right press | one "Draw lasso"; a `contextmenu` listener sees `defaultPrevented`; no menu element in the DOM; Lasso still pressed; a right press with no lasso changes nothing |
| C11 Esc discards | customer 2 | three corners, Esc; Esc again | no new History row, no region; Lasso pressed after the first Esc, released after the second |
| C12 Auto close | customer 4 | three corners, then each of: Rectangle, chip 2, `S`, Layers row of another image | one "Draw lasso" each; the stored region equals the one closed by its first corner (wasm test compares the geometry, criterion 5) |
| C13 No create on double click | | Lasso pressed, double click on the image with no lasso in progress | no "Trace image" row; a lasso of one corner is not stored |
| C14 Cursors | | Rectangle pressed; then released over R1's body and over its corner handle | computed `cursor`: `crosshair`; `default`; a `url(...)` resize cursor |
| C15 Turned image | criterion 13 | image rotated 30 degrees, R1 | the black outline runs along the turned picture (samples on the turned edge), the dashed box turned with it |

## 11. What this changes in existing Curvyo docs and tests

- `docs/tracing-images.md`: intro ("or one image with mask paths" goes); "Method" ("after the Mask
  group" becomes "after Preset"); "Advanced" item 3 (the Parts row with the Mask group, the one-line
  rule); "Mask" rewritten for Rectangle, Lasso, Subtract, the regions with the image, no mask paths,
  no `TRACE_MIXED` sentence; "Parts" bullets ("Select (or pick) its mask paths", "its mask paths are
  selected with the image", "Pick mask paths, Shift+click", "mask paths are still a selection and are
  not stored", "the bar does not read them back yet"); "Create paths" (the regions stay); the About
  Trace Mask entry (section 4).
- `docs/design-system.md`: rows "Select bar Trace button" (visibility), "Trace bar" (order, no Mask
  group, no `TRACE_MIXED`), "Trace Advanced button and block" (the unit, the wrap order, the width
  rule), "Trace Mask and Parts (`0094`)" and "Trace pick mask paths (`0094` slice 1b)" replaced by
  section 12's row, "Trace info tab and page" (Mask entry), "Trace notices" (no refused-pick
  notice); keyboard map rows "Trace tool" and "Escape cascade" (section 7); the cursor list loses
  `pick-mask`.
- `specs/0094-masked-composite-trace/ux-notes.md` §1, §2 (mask paths as artwork), §4, §11 and the
  0094 spec: "Replaced by 0114" notes (plan slice 5).
- Frontend tests: delete `traceMaskPick.test.ts`, `acceptance_0094_pick_browser.test.ts`,
  `acceptance_0094_pick_tester.test.ts`; rewrite `traceBar.test.ts` (Tab order, group contents),
  `traceMaskRect.test.ts` (group now in Advanced, `Square` glyph), `acceptance_0094_tester.test.ts`
  (bar orders `BRIGHTNESS_BAR`, `MASKED_BAR`, the Shift+click and mixed-selection cases),
  `acceptance_0094_parts_browser.test.ts` (`PICK`), `acceptance_0089_cleanup_tester.test.ts` and
  `acceptance_0089_tidyup_tester.test.ts` (`MASK_STOPS`, bar orders, the three-control group check),
  `acceptance_0089_e_browser.test.ts` and `acceptance_0089_sliceE_tester.test.ts` (the "Mask group"
  in the bar list), `acceptance_0025_slice3_tester.test.ts` (bar order, `[data-trace-mask]` in row 1),
  `acceptance_0089_bar_width_browser.test.ts` (the bar now counts the Advanced line, section 2.5),
  `traceText.test.ts` (About text), and the `maskPick` fields of the fake APIs in
  `tracePickedColours.test.ts`, `traceBar.test.ts`, `traceMaskRect.test.ts`.
- Wasm tests (plan slice 4 names them): `session/trace_mask_pick_tests.rs`,
  `trace_mask_pick_hit_tests.rs`, the Shift+click and mixed cases in `tests/acceptance_0094_tester.rs`
  and `tests/acceptance_0094_parts_tester.rs`, `trace_region_tests.rs`, `trace_parts_tests.rs`.
- Reference screenshots: Curvyo has no committed screenshot of the Trace bar (no PNG under
  `specs/0089-*`, `specs/0094-*` or `frontend/tests`); criterion 25 is met by this folder's
  pictures, re-captured from the build in slice 4, and by B1 to B5. Tracyo's `px-*` are Tracyo's.

## 12. Differences from Tracyo, and changes to slice 1 as built

| # | Item | Curvyo 0114 | Tracyo | Slice 1 as built |
|---|---|---|---|---|
| 1 | Lasso finish and Esc | right click finishes; Esc discards, Lasso stays pressed | Esc finishes and releases (to change, customer 2026-11-13) | no Lasso |
| 2 | Parts row | the unit flows in the Advanced line after Corners, before Analyse; one line on a wide window | own last line below Corners and Analyse (to change, customer 6) | Parts after Corners, Mask group in row 1 |
| 3 | Bar width | the wider of row 1 and the Advanced line (section 2.5) | centred bar, cap 1256 | row 1 only |
| 4 | Rectangle icon | `Square` | `Square` | `RectangleHorizontal` |
| 5 | Regions of other parts | drawn, not hit | same | not drawn |
| 6 | Selected rectangle | dashed box, rounded white handles | same | solid 1.5 px accent outline, square handles |
| 7 | Rectangle being drawn | live outline with casing, size chip in mm | live outline, chip in picture px | final black look, no chip |
| 8 | Hover and hit slop | hover box, 8 px | same | no hover box, 4 px |
| 9 | Cursors | Curvyo's custom `resize:<deg>` over handles | egui system cursors | `crosshair` only |
| 10 | Esc steps 5 and 6 | release the tool, then deselect | same | deselect, then release |
| 11 | Regions above objects | above every object; wash under objects above the image | no objects | as Curvyo |
| 12 | Turned image | regions and boxes in the picture frame | no placement | as Curvyo |

## 13. Proposed design-system row (the implementer applies it; replaces rows "Trace Mask and Parts" and "Trace pick mask paths")

| Trace Mask in the Parts row (`0114`, 2026-11-13) | The Mask group (Rectangle `Square`, Lasso `Lasso`, Subtract `SquareMinus`: 28 px `aria-pressed` toggles, pressed `--toolbar-icon-active-bg` / `-fg`; Inside \| Outside 2 x 64 px; Clear mask `X`; 256 px) follows the trash of the Parts group in one wrapping unit (377 + 32 n px), 25 px after it behind the 1 x 20 bar divider, which is hidden when the group starts a line; the armed trash colours the Parts group only. The Advanced block is one flowing line: Output row, Denoise, Upscale, Corners, the unit, Analyse (far right); the unit wraps as a whole first, then the Mask group alone, then Add part and trash below the chips. With Advanced open the bar is as wide as the wider of its first row and its Advanced line, at most the available width. Row 1 has no Mask group. Rectangle and Lasso exclude each other and the pipette; Subtract is independent; tools stay pressed until pressed again, Esc or leaving the tool. **Canvas:** regions are a tool overlay above every object, in the picture's frame: black 0.25 mm (at least 1 px) outlines for every part, subtracted dashed 4 / 3; hover box within 8 px; selected rectangle: dashed box and 8 rounded handles; selected lasso: dashed box and 14 px Node-tool corner squares; drags leave the drag ghost; a rectangle is drawn with the live preview outline and a "W × H mm" readout; a lasso with the Pen-in-progress look and its 6 / 4 close cue; with Subtract dashed and with the Remove badge. Lasso: click per corner (whole pixels), finish by a right click, a double click, Enter or the first corner, closed by a straight side (3 corners or more, else dropped); Esc discards; another tool, part, image or leaving the tool closes it; no context menu; non-zero winding. Cursors: `crosshair` with a tool, `resize:<deg>` on handles, arrow on regions. Esc: drag, lasso, running trace, pipette, region tool, region selection, leave. No hint text | `0114` criteria 1 to 8, 23, 25, UX notes §1 to §9; customer 2026-11-13 |

## Changes to the spec (for the product owner)

- Behaviour 3 and criterion 5: Esc **discards** the lasso (customer 2026-11-13); remove Esc from the
  list of auto-closing ends; add "a right click finishes" and "no context menu while drawing"; add
  "picking a part, Add part, Remove part, the pipette and the Trace button close it too".
- Criterion 6: add "Esc discards a lasso of any length: nothing stored, no History step".
- Behaviour 4: the selection look is the dashed box (section 5), not the "box dashed" of slice 1's
  accent outline; corner squares are the Node tool's 14 px.
- Behaviour 12: "0 paths: the mask paths could not be combined." without the second sentence.
- Criterion 25: Curvyo has no committed Trace bar screenshots; the check is B1 to B5 plus the
  re-captured `reference/` pictures.
- Advanced block: one line on a wide window, the width rule of section 2.5.

## Open question

**Frage 11 (MULTI-IMAGE-MASK): Several images selected in the Trace tool.**
The mask belongs to one picture; with two or more images the region tools have no single picture to
draw on, and today every image is traced with the same settings.
- A: the Parts and Mask unit is not shown; Create paths traces each image with its own stored parts
  and regions (an image without a mask whole, with the bar's settings).
- B: the unit is not shown; the stored masks are ignored for a multi-image trace (every image whole).
- C: the unit edits the image clicked last (the previewed one); the others are traced with their own
  masks.

Recommendation A (the mask belongs to the image, so it applies wherever the image is traced).
Default if unanswered: A.
