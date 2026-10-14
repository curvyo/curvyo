# Raster trace: UX proposal

Author: ux-engineer. Date: 2026-10-14. Status: proposal for the customer; nothing here is accepted.
Mockup: `ux-mockup.html` in this folder (open it in a browser). Input: `research.md` (PO), the
customer request of 2026-10-14, `docs/design-system.md`, and the Offset tool of `0038`
(`specs/0038-path-offset/ux-rework.md`) as the model for how a tool with values behaves.

Spec 0058 (bitmap import) is not on `origin/main` yet. This proposal assumes an **image node**
that has a placed size in mm, a name, an optional lock and a visibility flag, and that appears in
Layers and can be selected like any object.

## 0. Summary of the decisions

1. **Trace is a tool**, a new one-button card **"Image tools"** in rail column A under "Path
   tools". Key `I` (default pending customer).
2. **Its settings live in a bar at the top middle**, in the overlay row where the Node bar and
   the shape bars sit. The right panel gets nothing: no section, no tab, no row.
3. **Choosing the tool with an image selected shows the bar and a preview at once**; the bar is
   the permission. Without an image the bar shows one line and one button, no preview.
4. **Preview is a blue outline over the dimmed image**, computed on a reduced copy while a value
   is being changed and at full size when it settles. A three-state view control and a hold-key
   compare trace and original.
5. **Commit: one labelled button "Create paths", plus Enter and a double-click on the image.**
   There is no Apply/Cancel pair. Escape cancels. The reasoning is in section 6.
6. **Result:** one group above the image, the image hidden, the tool returns to Select with the
   group selected, one undo step.
7. **Units are mm relative to the placed size**, with a pixel hint. No dpi question.

## 1. Where the tool lives in the rail

Today (`docs/design-system.md`, "Tool rail architecture"): column A holds Tools (7 buttons,
312 px), Group (92 px) and Path tools (Offset, 48 px, ending 480 px below the viewport top);
column B holds Path and Boolean.

**Proposal: a fourth card of column A, "Image tools", with one button, the Trace tool**, 8 px
under Path tools. It is 48 px high and ends 536 px below the viewport top; at 800 x 600 that
leaves about 36 px above the status bar (to be re-measured at build, as for `0038`). Same card
surface, 40 x 40 px button, 20 px glyph, `role="toolbar"` named "Image tools", one Tab stop, the
rail keyboard model. Tab order: Tools, Group, Path tools, Image tools, Path, Boolean.

Why not the Path tools card: Offset works on shapes that are already vectors; Trace turns an
image into vectors. The two cards will grow in different directions (Path tools: more
geometry; Image tools: crop, adjust, later). One card per kind of operand keeps a button from
ever moving. The cost is 52 px of rail height; column A is then at 536 of roughly 560 px and
**the next tool must go to column B or to a third column** (an architecture note for the
rail, not for this slice).

Why not column B: B holds commands that act at once on the selection. Trace needs a tool state
with a bar, so it belongs with the tools (the Offset reasoning).

**Glyph (final art at build, checked at 1x, 1.25x, 2x in greyscale):** 16 x 16, 1.5 px
absolute stroke. Left: a 3 x 3 grid of small squares (a bitmap). Right: a smooth curve crossing
the grid. Two shapes, no modifier mark. Lucide `ScanLine` is the fallback.

**Tooltip:** "Trace tool (I)", muted second line "Turn an image into paths. Enter creates them,
Esc cancels."

**The tool is a tool in every sense:** it takes the active-tool look, shares the
one-active-tool rule with Tools and Path tools, and `I` is a tool letter in every state (not
M, K, H, V, which have other jobs). Customer to confirm `I`; `T` would collide with a future
Text tool.

## 2. What happens when the tool is chosen

Three states, all with the bar in the overlay row (one bar at a time; the Select bar is
replaced in place, no animation).

| State | Canvas | Bar |
|---|---|---|
| **An image is selected** (or several: section 8) | The image is dimmed (see 4.1) and the preview of the remembered settings is computed and drawn. | Full bar (section 3). |
| **Nothing selected** | Untouched. Pointer over an image: hover box as in the Select tool. A click on an image selects it and starts the preview. | One row: text "Click an image to trace it." and a text button **"Import image..."** (the 0058 command, so the maker is never stuck with no image in the document). |
| **Something else is selected** (a path, a text, a group without images) | Untouched. | One row: "The selection is not an image. Click an image to trace it." Paths are already vectors; nothing is refused, the tool just waits. |

Nothing computes before the maker chose the tool, and nothing is drawn in the document. Choosing
another tool, Escape, or selecting a non-image removes the preview and the bar's full state.
The settings are remembered (app setting like the Offset distance, not in the file).

## 3. The bar

A pill in the overlay row, centred like the Node bar. Rows of 36 px with 28 px controls,
12 px text (`text-sm` for fields), 1 px dividers at 25 % between groups, wrapping **by whole
groups** (the Select bar rule). First row never moves. Controls are the design system's: the
value field with `--value-fill`, the two-segment pill of the Move chip, the 28 px icon buttons.

### 3.1 Groups

| Group | Contents | Notes |
|---|---|---|
| **Look** (row 1) | **Preset** menu button (132 px; shows "Logo / Line art" with a chevron; "Custom" once a value was changed). **Mode** pill "B/W \| Colours" (104 px). **View** pill of three icon segments (84 px), section 4.4. | Preset first: it is what a maker changes most. |
| **Tone** (row 2) | **Threshold** (B/W) or **Colours** (Colours mode), a value field 168 px, label inside, `--value-fill` bar. **Smooth** value field 168 px, 0 to 100. | Threshold and Colours share one slot and one width, so switching Mode moves nothing. Threshold 0 to 255, starts at the image's automatic value (Otsu) and shows it; Colours 2 to 16. |
| **Detail** (row 3) | **Specks** field "Ignore below" with `mm` suffix and a muted pixel hint, e.g. "0.50 mm  ~ 6 px". **Advanced** button (text and chevron, `aria-expanded`). | Specks is the length of the side of a square of the same area, in mm; see Frage TRACE-UX-UNIT. The hint updates when the image is scaled on the canvas. |
| **Result** (row 4) | Status text: "7 paths, 412 nodes, 0.4 s" (tabular figures). **Create paths** button (accent fill, white text, 112 px). | The status is the only readout. During compute it shows "Tracing..." and a thin 2 px progress line under the row. |

### 3.2 Width at 800 px

The overlay row at 800 x 600 with the properties panel open and two rail columns is **about
356 px** (measured for `0035`). The groups above are sized to fit that: row 1 = 132 + 8 + 104 +
8 + 84 = 336; row 2 = 168 + 8 + 168 = 344; row 3 about 212 + 8 + 112 = 332; row 4 about 220.
The bar is **4 rows, 156 px tall** (144 + 12 padding) at 800 px with the panel open. With the
panel collapsed (row of about 628 px) the groups pair up and the bar is 2 rows (84 px). The
bar never runs under the right panel. This is the dense end and the mockup shows it. If four
rows turn out to cover too much canvas at the narrowest window, the fallback is to move Specks
into Advanced; I do not ask the customer, it is a build-time call.

### 3.3 Advanced

A button that is **always present** in row 3 (text "Advanced" and a chevron, never hidden,
never shown disabled). It opens a block **below row 4**, inside the same pill, so the four
rows above do not move and Create paths stays where it is. Open or closed is remembered.
Contents, two columns of 168 px, rows of 36 px:

- **Denoise**: pill "Off | Low | High" (JPEG noise, scan grain).
- **Upscale**: pill "Auto | Off" (2x before tracing; Auto means for images under 1 MP).
- **Corners**: value field 0 to 100 (how readily a bend becomes a sharp corner).
- **Node limit**: number field, empty means no limit (raises the fit tolerance until it fits).
- **Invert**: switch (trace the light parts).
- **Ignore background**: switch (drop the lightest region; on in Logo and Silhouette).
- **Full-size preview**: switch (off by default; the preview normally uses a reduced copy).

Layering (stacked or no gaps) joins the block when the second option exists
(`research.md` Frage 6). Until then a control with one choice is not drawn.

### 3.4 Presets

A menu (`role="menu"`, opens below the button, arrow keys, Enter picks, Escape closes) with
four items, each two lines: name and what it is for.

| Preset | Sets | Line 2 in the menu |
|---|---|---|
| **Logo / Line art** | B/W, Smooth 60, Specks 0.5 mm, Upscale Auto, Corners 70 | Sharp corners, clean curves. |
| **Photo** | Colours 8, Smooth 70, Specks 1 mm, Denoise Low, Node limit 20 000 | Many shapes. Photos trace roughly. |
| **Silhouette for cutting** | B/W, Smooth 75, Specks 1 mm, Ignore background, Corners 50 | Closed, simple, few nodes. |
| **Engraving** | B/W, Smooth 30, Specks 0.1 mm, Corners 50 | Keeps fine lines. |

A preset sets every value, including Advanced ones, and marks itself. Editing any value turns
the button's text into "Custom" (the preset is not lost; re-picking restores it). With a
machine profile active, Silhouette for cutting takes its smallest feature from the machine (a
later link; the field value is always what you see). Saving own presets is out of scope.

## 4. Live preview

### 4.1 What is drawn

- The **image is dimmed** to 35 % (a white wash over it on the light theme; on the dark theme
  the same value over the dark canvas), so the line reads on any picture, dark or light.
- The **trace is a hollow outline**, 1.5 px screen-space, `--preview-new` (the "new" half of
  blue-new/black-old, as for every preview), over a 1 px `--selection-casing` white casing so
  it holds against dark regions. Holes are outlined too. No fills in this view.
- The bar is the permission for the preview; the document is not touched. The preview is an
  overlay layer, never a document node, and is dropped when the tool is left.

### 4.2 When it is computed

- **Debounce 150 ms** after the last change of any value.
- **While a slider is dragged or a key is held:** trace a reduced copy (longest side 1000 px).
  On release or when the value is committed: trace at full size. A stale job is cancelled when
  a newer value arrives.
- **While computing**, the previous outline **stays** (no flicker, no blank canvas). After 400
  ms the status row shows "Tracing..." and the 2 px progress line; if a job runs longer than
  1.5 s the status row adds a text button **"Stop"**. The old outline is then drawn at 50 %
  and a muted line says "Preview is out of date" so a stale outline is never mistaken for the
  current result.
- **Escape while a job runs** stops the job (first press); the next press leaves the tool.
- The first preview after choosing the tool uses the reduced copy, so the maker sees a first
  answer quickly; the full-size one follows.

### 4.3 Statistics

"7 paths, 412 nodes, 0.4 s" after each settled trace. Beyond the count: if specks were
dropped, an extra muted line "38 specks ignored" (tooltip: what they are). The node count has
a tooltip "Fewer nodes edit and cut faster". Announced to screen readers once per settled
result (`role="status"`, `aria-live="polite"`), not on every slider tick. Over 100 000 nodes
the line reads "412 000 nodes. Slow to edit; raise Smooth or Specks." and Create paths still
works (permit, with the cost named).

### 4.4 How the maker compares

The **View** pill in row 1 has three icon segments, each with a tooltip: **Overlay** (default:
dimmed image + outline), **Trace** (the result as it would be created, with its fills, image
hidden), **Image** (the original at full strength, no outline). Beyond the pill:

- **Hold `\` to show the original** while the key is held, then it returns (a peek, no state).
- `Shift+\` cycles the three views.

No split view in the first slice (Frage TRACE-UX-COMPARE): a divider is a drag target that
competes with panning and a press on the canvas; the toggle plus the peek key compares in place
at the same zoom, which is what the eye needs. Zoom and pan work as always while the tool is
active (Space, wheel, the status bar field).

## 5. Large images

- The image is never decoded or traced on the UI thread (platform side: a worker; the browser
  uses a Web Worker). The bar stays responsive and the preview comes in when it is ready.
- **Over 4 MP** the preview is always the reduced copy, and the status shows "Preview from a
  reduced copy". Create paths traces at full size with progress and **Stop**; the document is
  untouched until it finishes.
- **Over 25 MP** the trace itself runs on a half-size copy, stated in the status row ("Traced
  at half size, 36 MP image") with a pointer to Advanced "Full-size trace". Never a refusal
  and never an out-of-memory.
- The outline overlay is a cached path per result: pan and zoom redraw it culled to the
  viewport, never re-trace. A result of 100 000 nodes still pans smoothly (build check).
- Apply on a result of over 20 000 nodes inserts it as one group in one step; the
  Layers panel shows the group as a single row.

## 6. Applying, cancelling, and why one labelled button

### 6.1 Routes

| Route | Action |
|---|---|
| Button **Create paths** in the bar | Applies. |
| **Enter** with focus on the canvas or on a bar button | Applies. In a bar field, Enter commits the typed value (the preview follows) and returns focus to the canvas; the next Enter applies. |
| **Double-click on the image** | Applies (the same as Enter). |
| **Escape** | Stops a running job; else leaves the tool and drops the preview. No dialog (nothing was changed). |
| Choosing another tool | Same as Escape. |

### 6.2 Is there an Apply button?

The customer disliked the Apply button for Offset. The cases differ:

- Offset has a canvas handle and a value the maker drags; releasing the drag is the commit, so
  a button was a second route. **Trace has no handle on the canvas and nothing to drag**, so
  without a button the only commit is a key or a double-click that nothing on screen advertises.
  The customer's rule "operations have a visible trigger" then fails.
- The result is large and not free to undo mentally (hundreds of nodes, the image hidden).
  A deliberate, visible act is right before it.

So the bar has **one** button, named for its result: **"Create paths"**, not "Apply", in the
last row where the eye ends. There is **no Cancel button**: Escape and choosing another tool
cancel, and nothing is lost by cancelling because nothing was changed. The tooltip of the
button names the keys: "Create paths (Enter)". If the customer prefers no button, the
alternative is in Frage TRACE-UX-COMMIT.

### 6.3 What the maker gets

- One group named **"Trace of <image name>"**, **directly above the image** in the same layer
  and at the same place, with the image's transform baked in (the paths sit on the picture
  pixel for pixel). Colours as traced; in B/W mode one compound path with holes.
- **The image is hidden** (not deleted, not locked), see Frage TRACE-UX-SOURCE. A **success
  notice** (the Action notice) next to the rail: "Traced: 7 paths, 412 nodes. Source image
  hidden." for 3 s, with the same text in the Layers row tooltip. If the image is locked it
  stays visible and the notice says so.
- The tool **returns to Select** with the group selected, so the next move is editing
  (Frage TRACE-UX-AFTER). The bar of the Select tool is back.
- **One undo step** "Trace image" in History: Ctrl+Z removes the group and shows the image again;
  redo brings both back. Changing a value is not a history step (it is tool state), and
  Ctrl+Z in the tool undoes the last document change, never a preview.

## 7. Keyboard route and accessibility

- `I` selects the tool. With the canvas focused and an image selected, **Tab** enters the bar
  (the standing order: rail, canvas, bar). Tab order in the bar: Preset, Mode, View, Threshold
  or Colours, Smooth, Specks, Advanced, the Advanced controls when open, Create paths.
- Value fields: Up/Down 1, Shift 10, Ctrl 0.1 on Specks, Home/End to the limits, typing and
  Enter. Pills and the View control: arrows move and select, as the Offset chip's pills.
  Preset menu: Enter or Down opens, arrows, Enter picks, Escape closes only the menu.
- Enter creates; Escape cancels; `\` peeks. `Ctrl+Enter` creates from inside a text field
  without the extra step.
- Hit targets 28 x 28 px or larger (design-system minimum for bar controls), focus ring 2 px
  `--editor-accent`. Contrast: the outline holds 3:1 against the dimmed image through the white
  casing; text uses `--toolbar-icon` on `--toolbar-bg`; the stale state is stated in words, not
  only by 50 % opacity.
- Every state change that is not visible on the canvas is also text: the status row (live,
  polite), "Preview is out of date", the empty and error lines. The 2 px progress line has
  `role="progressbar"` and `aria-busy` on the bar while a job runs.
- No motion is used (instant in and out like every bar), so reduced-motion needs no case.
- The dark theme: the dim wash is the canvas colour at 65 %, not white; the casing stays white
  (the existing casing rule).

## 8. Several images selected (default pending customer)

The tool traces **the image the maker clicked last** and the bar says "Tracing 1 of 3 images".
Create paths traces **all selected images with the same settings** in one step (a group per
image, one undo step); the preview shows only the active one. Simple, and a batch is possible
without a second UI. Alternative in Frage TRACE-UX-MULTI.

## 9. Empty and error states (permit over refuse)

| Situation | What the maker sees | Can they continue? |
|---|---|---|
| No image selected | "Click an image to trace it." and **Import image...** | Yes, either action. |
| A non-image selected | "The selection is not an image. Click an image to trace it." | Yes. |
| Image still loading | Bar with the values; status "Loading image..."; Create paths waits (same row). | Values can be set meanwhile. |
| Result has **0 paths** at these settings | Status "0 paths. Try moving Threshold toward <lighter or darker>." (direction computed). Create paths is `aria-disabled` and the status says why. | Change a value. |
| **Everything is one shape** (a full black or white page) | "1 path covering the whole image. Check Threshold or Invert." Create paths works. | Yes. |
| **Over 100 000 nodes** | Warning line in the status (4.3). Create paths works. | Yes. |
| **Photo preset** | Muted line under the status: "Photos trace into many shapes. Fewer colours and a larger Specks value give lighter results." | Yes. |
| Locked or hidden image | Traced as usual; the image stays as it is, the notice says "Source image is locked and stays visible." | Yes. |
| **Trace fails** (out of memory, internal error) | `--field-invalid` message card under the bar, as the Move chip's: "Trace failed: <reason>. Your drawing is unchanged." with a **Retry** button. The bar values stay. | Retry or change values. |
| A job is stopped | Outline at 50 % with "Preview is out of date". | Change a value or press Enter to trace again. |

Nothing is refused on size, format, or an unsuitable image; the tool says what will happen and
lets the maker go on.

## 10. Comparison

| | Inkscape Trace Bitmap | Illustrator Image Trace | Curvyo (this proposal) |
|---|---|---|---|
| Where | Docked dialog on the right, two tabs (Single, Multiple scans) | Panel (or Control bar) at the side | A tool; settings in the bar at the top of the canvas; nothing on the right |
| Preview | A small thumbnail in the dialog; "Update" button | Live "tracing object" on the canvas with a Preview tick | Live outline on the canvas at real size, over the dimmed image; reduced copy while adjusting |
| Units | Pixels (speckles, scans) | Pixels (Noise, Paths) | mm of the placed image, with a pixel hint |
| Commit | OK / Apply in a dialog, result replaces or sits beside | "Expand" turns the live object into paths | One "Create paths" button, Enter, or a double-click; one undo step |
| Presets | None | A long list named after looks | Four named after jobs: Logo, Photo, Silhouette for cutting, Engraving |
| Advanced | Everything on the dialog | A collapsed Advanced section | One always-visible Advanced expander, short list |
| Source image | Stays | Stays under the tracing object | Hidden by default, toggled in Layers |
| Comparing | Toggle the checkbox, eyeball | Preview tick on and off | Three views plus a peek key, same zoom |
| Result facts | None | None | Paths, nodes and time, and a warning at 100 000 |

What we do differently and why: the maker is working on a laser job, so sizes are in millimetres
and the presets are named after the job; the settings sit where the eye already is (above the
canvas) and not in a window that hides the picture; there is no live object to expand (a trace is
plain paths, so the file stays simple and undo is one step); and the node and path counts are
shown because they decide how long the job takes on the machine.

## 11. UX notes for the specification (to carry over at Ready)

1. The Trace tool is a one-button card "Image tools" in rail column A; key `I`.
2. Its settings are in a bar in the overlay row; the right panel is unchanged.
3. Tool choice with an image shows the bar and the preview; without an image one line and
   "Import image...".
4. Preview is overlay only; the document changes on Create paths, Enter, or a double-click.
5. Settings: Preset, Mode, Threshold or Colours, Smooth, Specks (mm, pixel hint), Advanced.
6. Result: a group above the image, image hidden, tool back to Select, one undo step.
7. Stats are shown; large and failed cases never refuse, they say what happens.
8. Keyboard: `I`, Tab into the bar, arrows, Enter, Escape, `\` peek.

## 12. Open questions for the customer

Customer language is German; these are written in English because they live in the repo and
the lead translates them for the message.

**Frage 1 (TRACE-UX-RAIL): Where does the Trace tool sit in the rail?**
Options: (a) a new card "Image tools" in column A under Path tools (recommended); (b) a second
button in the Path tools card; (c) column B with the commands.
Recommendation: (a). Default: (a), key `I`.

**Frage 2 (TRACE-UX-KEY): Which key selects the tool?**
Options: (a) `I` (image); (b) `T` (trace; blocks Text); (c) Shift+Alt+B as in Inkscape (long).
Recommendation: (a). Default: (a).

**Frage 3 (TRACE-UX-COMMIT): How does the maker create the paths?**
Options: (a) one button "Create paths" plus Enter and double-click; Escape cancels (recommended);
(b) no button, only Enter and double-click (the Offset way; nothing on screen shows how to
finish); (c) Apply and Cancel buttons (what you disliked for Offset).
Recommendation: (a), because a trace has no handle to release and a visible trigger is your own
rule. Default: (a).

**Frage 4 (TRACE-UX-AUTOPREVIEW): Does the preview start when the tool is chosen?**
Options: (a) yes, for the remembered settings, on a reduced copy first (recommended); (b) only
after the first change of a value; (c) a Preview switch in the bar, off by default.
Recommendation: (a), because the maker chose Trace to see a trace; Offset's "no standing
preview" was about a chip the maker had not opened. Default: (a); the Advanced "Full-size
preview" switch covers slow machines.

**Frage 5 (TRACE-UX-COMPARE): How does the maker compare trace and original?**
Options: (a) a three-view pill (Overlay, Trace, Image) plus a hold-key peek (recommended);
(b) a draggable split view on the canvas; (c) only the Preview on/off switch.
Recommendation: (a); (b) can follow if (a) is not enough. Default: (a).

**Frage 6 (TRACE-UX-SOURCE): What happens to the source image after Create paths?** (This is
research Frage 8, VEC-SOURCE, answered with a mechanism.)
Options: (a) hidden, shown again with the eye in Layers (recommended); (b) kept visible under
the paths; (c) the maker chooses in Advanced ("Hide source image" switch, on).
Recommendation: (a): otherwise the maker cannot see the result without moving it. Default: (a).

**Frage 7 (TRACE-UX-AFTER): Which tool after Create paths?**
Options: (a) Select, with the new group selected (recommended); (b) stay in the Trace tool,
bar shows the result.
Recommendation: (a), so a second Trace needs a deliberate `I`. Default: (a).

**Frage 8 (TRACE-UX-UNIT): What does "Specks" measure?**
Options: (a) the side of an equal-area square in mm, with a pixel hint (recommended); (b) area
in mm2 (as in `research.md`); (c) pixels.
Recommendation: (a): a length compares directly to a feature or a kerf. Default: (a).

**Frage 9 (TRACE-UX-MULTI): Several images selected?**
Options: (a) preview the last clicked image, Create paths traces all with the same settings
(recommended); (b) trace only the last clicked and ignore the rest; (c) the tool needs exactly
one image and says so.
Recommendation: (a). Default: (a).

**Frage 10 (TRACE-UX-PRESET): Four presets in the first slice?**
Options: (a) Logo / Line art, Photo, Silhouette for cutting, Engraving (recommended);
(b) only the first two until the machine link exists.
Recommendation: (a). The "Silhouette" value for Specks is a fixed 1 mm until a machine profile
gives a kerf. Default: (a).
