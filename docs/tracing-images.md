# Tracing an image

The **Trace** tool turns an image object into smooth, closed paths you can cut, engrave, plot or
edit. Select one or several images and press **Trace**, the last button of the Select bar (it is
there only while nothing but images is selected), or press **I**. Ctrl+I stays Import. The bar
above the canvas becomes the Trace bar; the right panel gets one more tab, **About Trace**, which
opens only when you ask for it (see "About Trace" below).
(`specs/0025-raster-trace`, slices 1 to 3: black and white, presets and Advanced settings, the
View pill, large images; `specs/0089-trace-quality-and-colour` slices D and C: the geometry, and
the methods Brightness, Remove background and Pick colours.)

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
washed-out image under the outline, where the tool starts every time), **Trace** (the result
filled black as Create paths would make it, the image hidden) and **Image** (the original at full
strength, no outline). Hold **\\** to see the image while it is held; **Shift+\\** steps through the
three views. Neither changes the drawing; zoom and pan work in every view.

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
every selected image.

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
  or a press on its button. A click on a swatch selects it and shows its Remove button; Delete
  removes the focused swatch. Picked colours that touch become one shape. The picked colours
  belong to this use of the tool and are not remembered. With none the status says "0 paths: no
  colour picked."

**Colour tolerance** (0 to 100; 40 for Remove background, 15 for Pick colours at first use): how far
a colour may be from the background or a picked colour and still count as it; it grows with the
square of the value (100 is half the colour cube's edge), so the low end separates pale colours.
Colours are compared as their stored values (no colour management).

The pipette samples the image itself (what the tracer reads, not the screen), the middle value of
the 3 x 3 pixels under the pointer; the chip beside the pointer shows the colour or "No colour
here". Esc first stops a running trace, then disarms the pipette, then leaves the tool.

Transparent pixels are never shapes, with Invert neither, whatever colour they store. A partly
transparent pixel counts by its opacity once: a black pixel at 50 % opacity is just a shape; the
image is never laid over white first.

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

The double-arrow button at the far right, before the Trace button ("More trace options"), opens a block under the values
(open or closed is remembered). Its first row starts with two icon buttons (pressed means on, the
tooltip names each):

- **Invert**: traces what the method leaves out (with Brightness the light parts: pixels at or
  above the Threshold); transparent pixels stay empty.
- **Drop border shapes**: drops the shapes that touch the border of the picture.

Then:

- **Denoise** (Off, Low, High): a median filter on the brightness before the Threshold; Low takes
  3 x 3 pixels, High 5 x 5. It removes scan noise and JPEG ringing.
- **Upscale** (Auto, Off): traces a copy at twice the size; Auto does so for images under 1
  megapixel. Small or low-resolution pictures keep their shape better.
- **Corners** (0 to 100, default 70): higher makes a bend a sharp corner more readily (70: bends
  sharper than 60 degrees). Corners are found on the pixel outline before it is calmed and placed
  where the two straight sides meet.

The last row of the block gives the counts of the result (see Status). Invert and Drop border
shapes start off with each start of the app.

## Status

The counts of a trace, for example "7 paths, 412 nodes, 0.4 s" (paths are outlines, nodes their
anchor points, the time is that of the last full trace) and, muted, "38 specks ignored" when
specks were dropped, are the last row of the Advanced block; with the block closed they are not
shown. While a trace runs or after a stop they stay, at half strength. Results you should read
show in a muted row under the values whether the Advanced block is open or not:

- "0 paths at this Threshold: too low." (or "too high."): the Trace button creates nothing. With
  Remove background "0 paths at this Colour tolerance: too high.", with Pick colours "0 paths at
  this Colour tolerance: too low." or "0 paths: no colour picked."
- "1 path covering the whole image.": the whole picture is darker than the Threshold; Create
  paths works.
- "Too much detail: raise Specks.": one shape has 65,000 holes or more; raise Ignore below.
- "412,000 nodes. Slow to edit.": more than 100,000 nodes; Create paths works, and the result is
  still one group with one compound path, one row in Layers.

## About Trace

While the Trace tool is active the Properties panel has one more tab, last in its strip:
**About Trace** (the "i" glyph). It shows a short description of the tool; it is the only
description of this kind in the app and never opens by itself. The "i" button at the start of the
bar opens the same tab and is pressed while it shows; pressing it
again, or leaving the tool, returns the panel to the tab it showed before (hidden again if it was
hidden). Any other tab leaves it too. The text:

Trace turns the selected image into paths. Dark parts become shapes, light parts stay empty. Press Enter to create the paths.

- Preset: a named set of values for what you do with the paths.
- Logo / Line art: sharp corners, clean curves.
- Photo: many shapes, photos trace roughly.
- Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.
- Engraving: keeps fine lines, drops shapes below 0.1 mm.
- View: the image with the outline, the result alone, or the image alone. Hold \ to peek at the image.
- Method: how pixels become shapes.
- Brightness: pixels darker than the Threshold.
- Remove background: everything unlike the background colour; Auto takes it from the image border, the pipette from a click.
- Pick colours: pixels like the colours picked with the pipette; Shift+click adds a colour.
- Colour tolerance: how far a colour may be from the background or a picked colour and still count as it.
- Threshold: pixels darker than this brightness (0 to 100 %) become shapes.
- Smooth: calms the pixel steps of the outline before the fit; higher is calmer.
- Tolerance: how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.
- Ignore below: drops shapes and holes smaller than this size in mm.
- Denoise: smooths out noise in the picture before tracing.
- Upscale: doubles small images before tracing; Auto does it below 1 Mpx.
- Corners: higher turns more bends into sharp corners.
- Invert: traces what the method leaves out; transparent pixels never become shapes.
- Drop border shapes: drops shapes that touch the image border.

## Create paths

The blue **Trace** button at the far right of the bar, Enter on the canvas, Ctrl+Enter in a field,
or a double-click on the image adds one group "Trace of <image name>" directly above each image, with one compound
path inside, filled black, without stroke, holes as holes. The image stays visible and unchanged
under it; hide or move it as usual. The new groups are selected and the Select tool is active. The
notice says "Traced: 7 paths, 412 nodes." or, for several images, "Traced 3 images: 21 paths,
1,204 nodes." The tooltip of the group's row in the Layers tab repeats it (until the app is
closed). It is one step "Trace image" in the History: Ctrl+Z removes every group of it. With
several images selected the bar says "Tracing 1 of 3 images" and the preview shows the image you
clicked last; every image is traced with the same settings. If only the reduced copy has been
traced, Create paths waits for the full picture; Escape stops before anything is added.

The result is ordinary paths: they move, scale, rotate and edit like any group, take part in
every output, and do not follow the image if it is moved later.

## Privacy and rights

Tracing happens on your computer; nothing is sent anywhere. You are responsible for having the
right to trace an image.
