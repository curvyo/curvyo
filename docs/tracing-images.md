# Tracing an image

The **Trace** tool turns an image object into smooth, closed paths you can cut, engrave, plot or
edit. Select one or several images and press **Trace**, the last button of the Select bar (it is
there only while nothing but images is selected), or press **I**. Ctrl+I stays Import. The bar
above the canvas becomes the Trace bar; nothing appears in the right panel.
(`specs/0025-raster-trace`, slices 1 and 2: black and white, presets and Advanced settings.)

## The preview

The image is washed out (a white veil over it) and the outline the trace would make is drawn over it as a thin blue line on
a white casing, holes included. It is an overlay, not part of the drawing: it is not in the
History, the Layers tab, a saved file or any output, and it goes when you leave the tool with
Escape, **S** or another tool. Pan and zoom redraw it without tracing again.

The trace runs in the background, so the editor stays responsive. While a value is dragged, a
reduced copy of the picture (longest side 1000 pixels) is traced; on release the full picture
follows. After 400 ms the status says "Tracing..." with a thin progress line; after 1.5 s a
**Stop** button appears. A stopped trace leaves the old outline at half strength with the line
"Preview is out of date"; changing a value traces again.

With nothing selected the bar says "Click an image to trace it." and offers "Import image...".
With something else selected it says "The selection is not an image. Click an image to trace
it." A click on an image selects it and starts its preview.

## Presets

The **Preset** button at the start of the bar picks a set of values for what you do with the
paths afterwards. A pick sets Smooth, Ignore below, Corners, Upscale and Denoise, never the
Threshold, the Node limit or the switches. After any change of those five values the button reads
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

- **Threshold** (0 to 255): pixels darker than this become shapes. It starts at the image's
  automatic value (Otsu's method on the brightness of the picture); transparent pixels count as
  white. Brightness is `0.2126 R + 0.7152 G + 0.0722 B` of the stored colours.
- **Smooth** (0 to 100, default 80): higher gives fewer nodes and smoother curves. Sharp corners
  of the picture stay sharp. Above 80, Smooth also merges nodes while the outline stays within
  (Smooth - 80) x 0.01 mm of the trace, so 0.2 mm at 100; up to 80 nothing is merged.
- **Ignore below** (specks, 0 to 10 mm, default 0.5 mm): shapes and holes smaller than a square of
  this side are dropped (a speck becomes background, a small hole is filled). The size is measured
  on the placed image in millimetres; the muted readout gives it in image pixels ("0.50 mm  ~ 2
  px").

Up and Down change a value by one step (Specks 0.1 mm), Shift by ten (Specks 1 mm), Ctrl by the
finest step (Specks 0.01 mm), Home and End go to the limits. A typed value is set with Enter and
kept within the range; Enter then returns to the canvas. Smooth and Specks are remembered between
uses as a setting of the app, not of the project; the Threshold starts at each image's automatic
value.

## Advanced

The **Advanced** button beside Ignore below opens a block under the status row (open or closed is
remembered):

- **Denoise** (Off, Low, High): a median filter on the brightness before the Threshold; Low takes
  3 x 3 pixels, High 5 x 5. It removes scan noise and JPEG ringing.
- **Upscale** (Auto, Off): traces a copy at twice the size; Auto does so for images under 1
  megapixel. Small or low-resolution pictures keep their shape better.
- **Corners** (0 to 100, default 70): higher makes a bend a sharp corner more readily.
- **Node limit** (empty for none): when the result has more nodes, the curves are fitted more
  loosely until it fits. The status then reads "Node limit reached: 20,000 nodes, tolerance
  raised." or, when even the loosest fit has more, "Node limit could not be met: 31,400 nodes."
- **Invert**: traces the light parts: pixels at or above the Threshold become shapes.
- **Ignore background**: drops the shapes that touch the border of the picture.
- **Full-size preview** (off by default): the preview traces the whole picture at once instead of
  a reduced copy first. Over 4 megapixels it is not available ("Over 4 Mpx").

The Node limit, Invert, Ignore background and Full-size preview start off with each start of the
app.

## Status

After a trace the status line gives the result, for example "7 paths, 412 nodes, 0.4 s" (paths are
outlines, nodes their anchor points, the time is that of the last full trace) and, muted, "38
specks ignored" when specks were dropped. Other states:

- "0 paths at this Threshold: too low." (or "too high."): Create paths is not available.
- "1 path covering the whole image.": the whole picture is darker than the Threshold; Create
  paths works.
- "Too much detail: raise Specks.": one shape has 65,000 holes or more; raise Ignore below.

## Create paths

**Create paths** (the button, Enter on the canvas, Ctrl+Enter in a field, or a double-click on
the image) adds one group "Trace of <image name>" directly above each image, with one compound
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
