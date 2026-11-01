# Tracing an image

The **Trace** tool turns an image object into smooth, closed paths you can cut, engrave, plot or
edit. Select one or several images and press **Trace**, the last button of the Select bar (it is
there only while nothing but images is selected), or press **I**. Ctrl+I stays Import. The bar
above the canvas becomes the Trace bar; the right panel gets one more tab, **About Trace**, which
opens only when you ask for it (see "About Trace" below).
(`specs/0025-raster-trace`, slices 1 to 3: black and white, presets and Advanced settings, the
View pill, large images; `specs/0089-trace-quality-and-colour` slices D, C, E and M: the geometry,
the methods Brightness, Remove background and Pick colours, the Mask view and Multicolour.)

## The preview

The image is washed out (a white veil over it) and the outline the trace would make is drawn over it as a thin blue line on
a white casing, holes included. It is an overlay, not part of the drawing: it is not in the
History, the Layers tab, a saved file or any output, and it goes when you leave the tool with
Escape, **S** or another tool. Pan and zoom redraw it without tracing again.

The trace runs in the background, so the editor stays responsive. While a value is dragged, a
reduced copy of the picture (longest side 1000 pixels) is traced; on release the full picture
follows. At the far right of the bar, after the expander, sits the blue **Trace** button; it does
what Enter does and creates the paths. When a trace has run for 400 ms the same blue button shows
its percentage (for example "42 %") and an x, "Stop (Esc)", and the old outline is drawn at half
strength; once shown, the percentage stays at least 300 ms, so a quick trace never flashes it. The
x (or a first Escape) stops the trace: the button says "Stopped" (its tooltip: "Preview is out of
date"), the old outline stays at half strength, and its **Retry** traces again, as do Enter, a
double-click on the image or changing a value.

For an image over 4 megapixels the preview is always the reduced copy, and a muted line says
"Preview from a reduced copy"; the counts are those of the copy. Create paths then traces the
full picture with the progress in the button at once; the drawing is not changed until it finishes.

If a trace fails, the button says "Failed" with a red ring, with a **Retry** button (Enter on the
canvas retries too); its tooltip, also read out by a screen reader, gives the reason, for example
"Trace failed: out of memory at 16 Mpx. Your drawing is unchanged." The values stay; "Failed"
goes with the next successful trace, a changed value, another image or leaving the tool.

## View

The **View** pill beside the Preset button switches what the canvas shows: **Overlay** (the
washed-out image under the outline, where the tool starts every time), **Mask** (exactly the
pixels the tracer turns into shapes: content black, everything else white, after Ignore below and
Drop border shapes and at the resolution traced, so twice the image's pixels with Upscale; zoomed
in, each pixel stays a sharp square), **Trace** (the result filled black as Create paths would make
it, the image hidden) and **Image** (the original at full strength, no outline). Transparent pixels
are white in the Mask, since they never become shapes; over 4 megapixels it is the reduced copy's
mask, and after a stop it is drawn at half strength like the outline. Hold **\\** to see the image
while it is held; **Shift+\\** steps through the four views. Neither changes the drawing; zoom
and pan work in every view, and the pipette works in the Mask view too.

With nothing selected the bar says "Click an image to trace it." and offers "Import image...".
With something else selected it says "The selection is not an image. Click an image to trace
it." A click on an image selects it and starts its preview.

## Presets

The **Preset** button at the start of the bar picks a set of values for what you do with the
paths afterwards. A pick sets Smooth, Tolerance, Ignore below, Corners, Upscale and Denoise, never
the method, the Threshold or the toggles. After any change of those six values the button reads
"Custom"; picking a preset again restores its values. The menu opens with Enter or Down on the
button; Up and Down move, Enter picks, Escape closes.

| Preset | For | Smooth | Tolerance | Ignore below | Corners | Upscale | Denoise |
|---|---|---|---|---|---|---|---|
| **Logo / Line art** ("Sharp corners, clean curves.") | logos, line art, area laser cutting, plotting, embroidery; the first use | 80 | 0.5 px | 0.5 mm | 70 | Off | Off |
| **Photo** ("Many shapes. Photos trace roughly.") | photographs, black and white | 50 | 0.5 px | 0.1 mm | 30 | Off | Off |
| **Silhouette for cutting/carving** ("Closed, simple, few nodes.") | cutting plotters, milling, carving: few nodes, nothing smaller than 2 mm | 100 | 1.5 px | 2.0 mm | 70 | Off | Low |
| **Engraving** ("Keeps fine lines.") | engraving fine detail and thin lines | 80 | 0.35 px | 0.1 mm | 70 | Auto | Off |

The choice (the preset, or Custom with its values) is remembered for the next image and applies to
every selected image. A choice remembered by an earlier version keeps its meaning: Smooth keeps
its number, a preset value it did not hold yet (Tolerance) takes that preset's value, and the
method starts as Brightness.

## Method

The **Method** button after View chooses how pixels become shapes; only the method's own fields
change with it (it is remembered, like each method's Colour tolerance):

- **Brightness** ("Dark and light pixels by a cut-off."): pixels darker than the Threshold.
- **Remove background** ("Everything unlike the background."): every pixel farther than the
  **Colour tolerance** from the background colour. Beside the field: a chip with the colour, its
  hex ("none" for a transparent background), **Auto** (pressed: the colour is the most frequent
  colour of the image border) and the pipette; a click with the pipette on the image takes the
  colour there and releases Auto.
- **Pick colours** ("Pixels like the colours you picked."): pixels made of the picked colours,
  including the soft edge toward the background, so pale colours and near-greys can be traced on
  their own. The pipette is armed on choosing the method; a click on the image picks its colour (it
  replaces the picked ones), **Shift+click** adds one (up to 8); the pipette stays armed until Esc
  or a press on its button. Beside the pipette the bar says how many colours are picked ("3 picked",
  "None picked"); the swatches themselves are shown once, in the Advanced block, where a click
  selects one and its Remove button takes it out; Delete removes the focused swatch. Picked colours
  that touch become one shape. The picked colours belong to this use of the tool and are not
  remembered. With none the Trace button is dimmed and its tooltip gives the reason, "0 paths: no
  colour picked."

**Colour tolerance** (0 to 100; 40 for Remove background, 19 for Pick colours at first use): how far
a colour may be from the background or a picked colour and still count as it; it grows with the
square of the value (100 is half the colour cube's edge), so the low end separates pale colours.
Colours are compared as their stored values (no colour management).

The pipette samples the image itself (what the tracer reads, not the screen), the middle value of
the 3 x 3 pixels under the pointer; the chip beside the pointer shows the colour or "No colour
here". Esc first stops a running trace, then disarms the pipette, then leaves the tool.

Transparent pixels are never shapes, with Invert neither, whatever colour they store. A partly
transparent pixel counts by its opacity once: a black pixel at 50 % opacity is just a shape; the
image is never laid over white first.

## Analysis

The first row of the Advanced block holds a line that says what the automatic analysis found in the
image the preview shows, and what it set, and the blue **Analyse** button. It runs once per image, on your computer, when the image is first shown in
the tool ("Analysing..." if that takes longer than 0.4 s), and applies its values at once. It sets
only values that depend on the image: the method (Brightness, or Remove background when the image
has pale or light colours that Brightness would miss), the background and Colour tolerance of
Remove background (half way between the background and the palest colour), and the Threshold (the
automatic one, or a lower cut when a photographed sheet has a wide shadow). It never changes the
preset, Smooth, Tolerance, Ignore below, the Advanced values, Output or picked colours; with Pick
colours it keeps the method and sets only the Threshold.

The line reads, for example, "Line drawing · scan · automatic Threshold", "Logo · pale colours
kept" or "Graphic · photo of paper · shadow left out": the image type (Graphic, Logo, Line drawing,
Sketch), how the image was made when it is a scan or a photo of paper, then what was set. When the
type is unclear (a photo without a sheet of paper) it reads "Type unclear · Brightness with
automatic Threshold" and sets just that. Once you change the method, the Threshold, the Colour
tolerance or the background, the line reads "Custom · Line drawing" and the analysis never changes
those values again for this image; **Analyse** sets them back to the analysis in one go (it is no
History step; it is always available, and with nothing changed it sets the same values again). If
you change any setting before the analysis has answered, its answer is not applied to this image:
the line reads Custom and Analyse applies it on request. If the analysis fails the line reads
"Analysis failed · automatic Threshold used" and the image keeps its automatic Threshold. A long
line wraps to a second row. With the Advanced block closed a screen reader still hears the line.
Several selected images use the shown image's analysis.

## Settings

- **Threshold** (0 to 100 %, brightness; Brightness only): pixels darker than this become shapes.
  It starts at the image's automatic value (Otsu's method on the brightness of the picture), shown
  as a whole percent; pixels under 50 % opacity count as the light class. Brightness is `0.2126 R + 0.7152 G + 0.0722 B` of
  the stored colours (0 to 255). The tracer works with that brightness value: a percent becomes
  `round(p x 255 / 100)`, so 50 % is 128; the automatic value is used exactly until you change
  the field.
- **Smooth** (0 to 100, default 80): calms the pixel steps of the outline before the fit (0.03
  pixels of smoothing per step, 2.4 at 80). Sharp corners of the picture are found before and stay
  where they are.
- **Tolerance** (0.1 to 3 px, default 0.5 px): the largest distance, in pixels of the image, of
  the paths from the calmed outline. Higher gives fewer nodes. Corners do not move when it
  changes; long straight edges become single lines.
- **Ignore below** (specks, 0 to 10 mm, default 0.5 mm): shapes and holes smaller than a square of
  this side are dropped (a speck becomes background, a small hole is filled). The size is measured
  on the placed image in millimetres; the muted readout gives it in image pixels ("0.50 mm  ~ 2
  px"). Dark pixels that touch only at a corner count as one shape, so a thin diagonal line stays
  one path.

The fields work like every value field of the app: a click sets the value at the pointer, a drag
changes it (the reduced copy is traced while you drag), a double-click, Enter or F2 opens number
entry. Up and Down change a value by one step (Threshold 1 %, Tolerance 0.05 px, Specks 0.1 mm),
Shift by a big step (Threshold 10 %, Tolerance 0.5 px, Specks 1 mm), Ctrl by the finest step
(Tolerance 0.01 px, Specks 0.01 mm), Home and End go to the limits. A typed value is set with
Enter and kept within the range; Enter then returns to the canvas. Smooth, Tolerance and Specks
are remembered between
uses as a setting of the app, not of the project; the Threshold starts at each image's automatic
value.

## Advanced

The bar holds what a straightforward trace needs: choose, set the one main value, trace. The
double-arrow button at the far right, before the Trace button ("More trace options"), opens a block
under the values with everything else (open or closed is remembered). Its rows, top to bottom, each
shown only when it has something to say:

1. The analysis line and the **Analyse** button (see Analysis).
2. The status line: the counts of the result (see Status).
3. **Output** (Single colour, Multicolour) and, with Multicolour, **Colours**, and with Brightness
   the palette's pipette; see "Multicolour" below.
4. The colour strip, the one place swatches are shown, with the actions of the selected swatch.
5. The options: three icon buttons (pressed means on, the tooltip names each), then three values.

- **Invert**: traces what the method leaves out (with Brightness the light parts: pixels at or
  above the Threshold); transparent pixels stay empty.
- **Drop border shapes**: drops the shapes that touch the border of the picture.
- **Recognise text** (on by default, remembered): reads the text first; see "Recognise text"
  below.

- **Denoise** (Off, Low, High): a median filter on the brightness before the Threshold; Low takes
  3 x 3 pixels, High 5 x 5. It removes scan noise and JPEG ringing.
- **Upscale** (Auto, Off): traces a copy at twice the size; Auto does so for images under 1
  megapixel. Small or low-resolution pictures keep their shape better.
- **Corners** (0 to 100, default 70): higher makes a bend a sharp corner more readily (70: bends
  sharper than 60 degrees). Corners are found on the pixel outline before it is calmed and placed
  where the two straight sides meet.

Invert and Drop border shapes start off with each start of the app.

## Recognise text

With **Recognise text** on (Advanced, on by default, a setting of the app) Trace reads the text in
a black and white trace before it traces the rest (`specs/0091-trace-text-recognition`):

- The first preview of a picture reads its text once, offline, with the model shipped in the app
  (PaddleOCR Latin, Apache-2.0, about 10 MB; nothing is downloaded and nothing leaves the
  machine). The Trace button shows the progress as "Reading text" and can stop it; later previews
  of the same picture in the session reuse the lines.
- Each line the reader is sure of (mean confidence 0.6 and a letter or digit) gets its own
  Threshold and polarity (white text on a dark box counts as text), is traced on its own, and its
  pixels are left out of the main trace and filled from their surroundings, so a shape under the
  text stays whole. The Mask view shows the left-out pixels in grey; the Advanced status line
  counts "3 text lines set".
- Create paths fits each line to the fonts as Recognise text does (fonts in the document,
  favourites, your fonts, the included fonts, the other installed fonts), with the read string as
  a hint, and writes one text per line, black, above the "Trace of" group, in the same step. A
  line no font fits within the tolerance is set in the nearest font anyway, every letter guessed.
  Guessed letters are named in one warning and highlighted in the Glyph table and the Text tool
  until the first edit; when all letters are sure, nothing is announced.
- With Recognise text off, or with Multicolour (for now), Trace works exactly as before and the
  model is not loaded.

## Multicolour

**Output** in the Advanced block switches between **Single colour** (one black shape, as before)
and **Multicolour**: the content the method selects is split into its colours, and every colour
becomes one filled shape. Neighbouring colours share one boundary: the same curve, node for node,
so there is no gap and no overlap between them, also where three colours meet. Output and Colours
are remembered as settings of the app.

**Colours** (2 to 16, "Auto" at the lowest stop) sets how many colours are found. Auto finds the
count in the image (the field shows it muted, for example "Auto  6"); where colours flow into
each other, as in a photo, Auto takes 6 steps. Colours closer than 3 % of the colour range are
one colour; pale colours and small accents are kept. Only areas of flat colour vote, so the
blended pixels along edges and JPEG fringes never become colours of their own. Areas smaller than
Ignore below join the neighbouring colour they share the longest border with ("2 regions joined"
in the status line). With Brightness the pipette after the field adds a colour you pick to the
palette (it stays armed; Shift+click adds more, Esc ends it); a colour that is already there is
selected instead ("Same as #3366CC"). With Pick colours the palette is exactly the picked colours
and the field shows their number.

The **palette** is the colour strip in the Advanced block: one swatch per colour, largest area
first, picked colours first with a small square in their corner (with Pick colours and Single
colour it holds the picked colours, and Remove is its one action). Arrows move through it; the selected swatch
shows its hex value and name ("#FFF2A8 · Pale yellow") and five buttons:

- **Isolate**: the other colours are drawn dimmed in every view ("Isolating #FFF2A8"); Enter
  still creates every colour.
- **Recolour**: a colour picker under the row changes the colour's fill, not its shape; nothing is
  traced again. The row reads "#FFE000 (was #FFF2A8)"; **Reset** returns to the found colour.
- **Merge into**: choose another swatch (Esc cancels; "Merging #FFF2A8 into..."): its pixels are
  traced with that colour, as one shape. The swatch stays, at half strength with the target's dot;
  pressing Merge into again undoes it.
- **Exclude**: the colour becomes background and is not traced; the swatch shows a crossed circle
  ("2 colours excluded"). Pressing it again brings the colour back.
- **Remove** (picked colours only): takes the colour out of the palette. Delete on the focused
  swatch does the same.

In the Mask view Multicolour shows each colour as it was found, the background white.

## Create paths

## Status

The counts of a trace, for example "7 paths, 412 nodes, 0.4 s" (paths are outlines, nodes their
anchor points, the time is that of the last full trace) and, muted, "38 specks ignored" when
specks were dropped, are stated once, in the status line of the Advanced block; with the block
closed they are not drawn (a screen reader still hears them). While a trace runs or after a stop
they stay, at half strength. With Multicolour the counts name the colours first, for example "3
colours, 7 paths, 412 nodes, 0.4 s". The status line also says what a colour action did
("Isolating #FFF2A8", "Merging #FFF2A8 into...", "2 colours excluded") and, instead of the counts,
"No colours found." (Multicolour found no colour in the content; the Trace button creates nothing)
and "0 paths: no colour picked."

Results whose fix is a control of the bar show in a muted row under the values whether the
Advanced block is open or not; the counts are then not printed again:

- "0 paths at this Threshold: too low." (or "too high."): the Trace button is dimmed and creates
  nothing; its tooltip gives the same reason. With Remove background "0 paths at this Colour
  tolerance: too high.", with Pick colours "0 paths at this Colour tolerance: too low."
- "1 path covering the whole image.": the whole picture is darker than the Threshold; Create
  paths works.
- "Too much detail: raise Ignore below.": one shape has 65,000 holes or more.
- "412,000 nodes. Slow to edit.": more than 100,000 nodes; Create paths works, and the result is
  still one group with one compound path, one row in Layers.
- "Same as #3366CC": a pipette pick matched a colour that is already there; it goes after 3
  seconds.

## About Trace

While the Trace tool is active the Properties panel has one more tab, last in its strip:
**About Trace** (the "i" glyph). It shows a short description of the tool; it is the only
description of this kind in the app and never opens by itself. The "i" button at the start of the
bar opens the same tab and is pressed while it shows; pressing it
again, or leaving the tool, returns the panel to the tab it showed before (hidden again if it was
hidden). Any other tab leaves it too. The text:

Trace turns the selected image into paths. The method decides which parts become shapes. Press Enter or the Trace button to create the paths.

- Preset: a named set of values for what you do with the paths.
- Logo / Line art: sharp corners, clean curves.
- Photo: many shapes, photos trace roughly.
- Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.
- Engraving: keeps fine lines, drops shapes below 0.1 mm.
- View: the image with the outline, the mask, the result alone, or the image alone. Hold \ to peek at the image; Shift+\ steps through the views.
- Mask: what the tracer turns into shapes, black on white (in Multicolour each colour as found), after Ignore below and Drop border shapes.
- Method: how pixels become shapes.
- Brightness: pixels darker than the Threshold.
- Remove background: everything unlike the background colour.
- Background: the colour Remove background leaves out; Auto takes it from the image border.
- Pick colours: pixels like the colours picked with the pipette; Shift+click adds a colour, up to 8. The picked colours are shown and edited in Advanced.
- Pipette: takes a colour from the image under a click; Esc ends it.
- Colour tolerance: how far a colour may be from the background or a picked colour and still count as it.
- Threshold: pixels darker than this brightness (0 to 100 %) become shapes.
- Smooth: calms the pixel steps of the outline before the fit; higher is calmer.
- Tolerance: how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.
- Ignore below: drops shapes and holes smaller than this size in mm.
- Analysis: names the image type and what it set for a new image: the method, its background and Colour tolerance, and the Threshold; never the preset or other values. Custom after you change one of them; Analyse sets them back.
- Status: the result in numbers (paths, nodes and time, specks ignored, colours) and what a colour action did.
- Output: Single colour traces the content as one black shape; Multicolour gives one filled shape per colour, neighbours sharing their edges.
- Colours: how many colours Multicolour finds, 2 to 16; Auto counts them in the image. With Pick colours the picked colours are the palette.
- Palette: the colours of a Multicolour trace, largest area first; picked colours carry a corner mark and come first.
- Isolate: shows one colour and dims the others; Enter still creates every colour.
- Recolour: gives a colour another fill without tracing again.
- Merge into: traces a colour as another one, as one shape.
- Exclude: makes a colour background; it is not traced.
- Remove: takes a picked colour out of the palette.
- Invert: traces what the method leaves out; transparent pixels never become shapes.
- Drop border shapes: drops shapes that touch the image border.
- Recognise text: reads the text in the picture first, sets each line as text in the nearest font and leaves it out of the paths.
- Denoise: smooths out noise in the picture before tracing.
- Upscale: doubles small images before tracing; Auto does it below 1 Mpx.
- Corners: higher turns more bends into sharp corners.

## Create paths

The blue **Trace** button at the far right of the bar, Enter on the canvas, Ctrl+Enter in a field,
or a double-click on the image adds one group "Trace of <image name>" directly above each image, with one compound
path inside, filled black, without stroke, holes as holes. With Multicolour the group holds one
compound path per colour, filled with that colour (or its recolour), named by its hex value,
largest area at the bottom. The image stays visible and unchanged
under it; hide or move it as usual. The new groups are selected and the Select tool is active; no
notice appears, the selected group is the feedback. The tooltip of the group's row in the Layers
tab says "Traced: 7 paths, 412 nodes." (until the app is closed). It is one step "Trace image" in the History: Ctrl+Z removes every group of it. With
several images selected the bar says "Tracing 1 of 3 images" and the preview shows the image you
clicked last; every image is traced with the same settings. If only the reduced copy has been
traced, Create paths waits for the full picture; Escape stops before anything is added.

The result is ordinary paths: they move, scale, rotate and edit like any group, take part in
every output, and do not follow the image if it is moved later.

## Privacy and rights

Tracing happens on your computer; nothing is sent anywhere. You are responsible for having the
right to trace an image.
