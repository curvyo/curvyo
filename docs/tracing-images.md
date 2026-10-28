# Tracing an image

The **Trace** tool turns an image object into smooth, closed paths you can cut, engrave, plot or
edit. Select one or several images and press **Trace**, the last button of the Select bar (it is
there only while nothing but images is selected), or press **I**. Ctrl+I stays Import. The bar
above the canvas becomes the Trace bar; the right panel gets one more tab, **About Trace**, which
opens only when you ask for it (see "About Trace" below).
(`specs/0025-raster-trace`, slices 1 to 3: black and white, presets and Advanced settings, the
View pill, large images.)

## The preview

The image is washed out (a white veil over it) and the outline the trace would make is drawn over it as a thin blue line on
a white casing, holes included. It is an overlay, not part of the drawing: it is not in the
History, the Layers tab, a saved file or any output, and it goes when you leave the tool with
Escape, **S** or another tool. Pan and zoom redraw it without tracing again.

The trace runs in the background, so the editor stays responsive. While a value is dragged, a
reduced copy of the picture (longest side 1000 pixels) is traced; on release the full picture
follows. After 400 ms a small progress pill appears after Ignore below, with the percentage (for
example "42 %") and an x, "Stop (Esc)", and the old outline is drawn at half strength; a quick
trace shows nothing. The x (or a first Escape) stops the trace: the pill says "Stopped" (its
tooltip: "Preview is out of date"), the old outline stays at half strength, and a **Retry** button
beside it traces again, as do Enter, a double-click on the image or changing a value.

For an image over 4 megapixels the preview is always the reduced copy, and a muted line says
"Preview from a reduced copy"; the counts are those of the copy. Create paths then traces the
full picture with the progress pill at once; the drawing is not changed until it finishes.

If a trace fails, the pill turns red and says "Failed", with a **Retry** button (Enter on the
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
paths afterwards. A pick sets Smooth, Ignore below, Corners, Upscale and Denoise, never the
Threshold or the toggles. After any change of those five values the button reads
"Custom"; picking a preset again restores its values. The menu opens with Enter or Down on the
button; Up and Down move, Enter picks, Escape closes.

| Preset | For | Smooth | Ignore below | Corners | Upscale | Denoise |
|---|---|---|---|---|---|---|
| **Logo / Line art** ("Sharp corners, clean curves.") | logos, line art, area laser cutting, plotting, embroidery; the first use | 80 | 0.5 mm | 70 | Off | Off |
| **Photo** ("Many shapes. Photos trace roughly.") | photographs, black and white | 50 | 0.1 mm | 30 | Off | Off |
| **Silhouette for cutting/carving** ("Closed, simple, few nodes.") | cutting plotters, milling, carving: few nodes, nothing smaller than 2 mm | 100 | 2.0 mm | 70 | Off | Low |
| **Engraving** ("Keeps fine lines.") | engraving fine detail and thin lines | 80 | 0.1 mm | 70 | Auto | Off |

The choice (the preset, or Custom with its values) is remembered for the next image and applies to
every selected image.

## Settings

- **Threshold** (0 to 100 %, brightness): pixels darker than this become shapes. It starts at the
  image's automatic value (Otsu's method on the brightness of the picture), shown as a whole
  percent; transparent pixels count as white. Brightness is `0.2126 R + 0.7152 G + 0.0722 B` of
  the stored colours (0 to 255). The tracer works with that brightness value: a percent becomes
  `round(p x 255 / 100)`, so 50 % is 128; the automatic value is used exactly until you change
  the field.
- **Smooth** (0 to 100, default 80): higher gives fewer nodes and smoother curves. Sharp corners
  of the picture stay sharp. Above 80, Smooth also merges nodes while the outline stays within
  (Smooth - 80) x 0.01 mm of the trace, so 0.2 mm at 100; up to 80 nothing is merged.
- **Ignore below** (specks, 0 to 10 mm, default 0.5 mm): shapes and holes smaller than a square of
  this side are dropped (a speck becomes background, a small hole is filled). The size is measured
  on the placed image in millimetres; the muted readout gives it in image pixels ("0.50 mm  ~ 2
  px").

The fields work like every value field of the app: a click sets the value at the pointer, a drag
changes it (the reduced copy is traced while you drag), a double-click, Enter or F2 opens number
entry. Up and Down change a value by one step (Threshold 1 %, Specks 0.1 mm), Shift by ten
(Threshold 10 %, Specks 1 mm), Ctrl by the finest step (Specks 0.01 mm), Home and End go to the
limits. A typed value is set with Enter and kept within the range; Enter then returns to the
canvas. Smooth and Specks are remembered between
uses as a setting of the app, not of the project; the Threshold starts at each image's automatic
value.

## Advanced

The double-arrow button after Ignore below ("More trace options") opens a block under the values
(open or closed is remembered). Its first row starts with two icon buttons (pressed means on, the
tooltip names each):

- **Invert**: traces the light parts: pixels at or above the Threshold become shapes.
- **Ignore background**: drops the shapes that touch the border of the picture.

Then:

- **Denoise** (Off, Low, High): a median filter on the brightness before the Threshold; Low takes
  3 x 3 pixels, High 5 x 5. It removes scan noise and JPEG ringing.
- **Upscale** (Auto, Off): traces a copy at twice the size; Auto does so for images under 1
  megapixel. Small or low-resolution pictures keep their shape better.
- **Corners** (0 to 100, default 70): higher makes a bend a sharp corner more readily.

The last row of the block gives the counts of the result (see Status). Invert and Ignore
background start off with each start of the app.

## Status

The counts of a trace, for example "7 paths, 412 nodes, 0.4 s" (paths are outlines, nodes their
anchor points, the time is that of the last full trace) and, muted, "38 specks ignored" when
specks were dropped, are the last row of the Advanced block; with the block closed they are not
shown. While a trace runs or after a stop they stay, at half strength. Results you should read
show in a muted row under the values whether the Advanced block is open or not:

- "0 paths at this Threshold: too low." (or "too high."): Create paths is not available.
- "1 path covering the whole image.": the whole picture is darker than the Threshold; Create
  paths works.
- "Too much detail: raise Specks.": one shape has 65,000 holes or more; raise Ignore below.
- "412,000 nodes. Slow to edit.": more than 100,000 nodes; Create paths works, and the result is
  still one group with one compound path, one row in Layers.

## About Trace

While the Trace tool is active the Properties panel has one more tab, last in its strip:
**About Trace** (the "i" glyph). It shows a short description of the tool; it is the only
description of this kind in the app and never opens by itself. The "i" button after the word
"Trace" at the start of the bar opens the same tab and is pressed while it shows; pressing it
again, or leaving the tool, returns the panel to the tab it showed before (hidden again if it was
hidden). Any other tab leaves it too. The text:

Trace turns the selected image into paths. Dark parts become shapes, light parts stay empty. Press Enter to create the paths.

- Preset: a named set of values for what you do with the paths.
- Logo / Line art: sharp corners, clean curves.
- Photo: many shapes, photos trace roughly.
- Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.
- Engraving: keeps fine lines, drops shapes below 0.1 mm.
- View: the image with the outline, the result alone, or the image alone. Hold \ to peek at the image.
- Threshold: pixels darker than this brightness (0 to 100 %) become shapes.
- Smooth: higher gives fewer nodes and a looser fit, up to 0.2 mm.
- Ignore below: drops shapes and holes smaller than this size in mm.
- Denoise: smooths out noise in the picture before tracing.
- Upscale: doubles small images before tracing; Auto does it below 1 Mpx.
- Corners: higher turns more bends into sharp corners.
- Invert: traces the light parts instead of the dark ones.
- Ignore background: drops shapes that touch the image border.

## Create paths

Creating the paths needs no button: Enter on the canvas, Ctrl+Enter in a field, or a double-click on
the image adds one group "Trace of <image name>" directly above each image, with one compound
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
