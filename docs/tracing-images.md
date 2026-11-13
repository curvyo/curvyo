# Tracing an image

The **Trace** tool turns an image object into smooth, closed paths you can cut, engrave, plot or
edit. Select one or several images and press **Trace**, the last button of the Select bar (it is
there while the selection holds an image or a linked clone of an image; anything else selected
with them is passed over), or press **I**. Ctrl+I stays Import. The bar
above the canvas becomes the Trace bar; the right panel gets one more tab, **About Trace**, which
opens only when you ask for it (see "About Trace" below).
(`specs/0025-raster-trace`, slices 1 to 3: black and white, presets and Advanced settings, large
images; `specs/0089-trace-quality-and-colour` slices D, C, E and M: the geometry, the methods
Brightness, Remove background and Pick colours, and Multicolour; the bar as tidied on 2026-11-06.)

## The preview

The image is washed out (a white veil over it) and the outline the trace would make is drawn over it as a thin blue line on
a white casing, holes included. It is an overlay, not part of the drawing: it is not in the
History, the Layers tab, a saved file or any output, and it goes when you leave the tool with
Escape, **S** or another tool. Pan and zoom redraw it without tracing again. This is the one way
the tool shows a trace; the outline over the image shows what the tracer found.

The trace runs in the background, so the editor stays responsive. While a value is dragged, a
reduced copy of the picture (longest side 1000 pixels) is traced; on release the full picture
follows. At the far right of the bar, after the expander, sits the blue **Trace** button; it does
what Enter does and creates the paths. When a trace has run for 400 ms the same blue button shows
its percentage (for example "42 %") and an x, "Stop (Esc)", and the old outline is drawn at half
strength; once shown, the percentage stays at least 300 ms, so a quick trace never flashes it. The
x (or a first Escape) stops the trace: the button says "Stopped" (its tooltip: "Preview is out of
date"), the old outline stays at half strength, and its **Retry** traces again, as do Enter, a
double-click on the image or changing a value.

For an image over 4 megapixels the preview is always the reduced copy (a screen reader hears
"Preview from a reduced copy"); the counts are those of the copy. Create paths then traces the
full picture with the progress in the button at once; the drawing is not changed until it finishes.

If a trace fails, the button says "Failed", with a **Retry** button (Enter on the
canvas retries too); Retry and Analyse always run on a working trace worker, a fresh one after a failure; its tooltip, also read out by a screen reader, gives the reason, for example
"Trace failed: out of memory at 16 Mpx. Your drawing is unchanged." The values stay; "Failed"
goes with the next successful trace, a changed value, another image or leaving the tool.

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

The **Method** button after Preset chooses how pixels become shapes; only the method's own fields
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
  or a press on its button. The picked colours sit beside the pipette as small swatches, in the
  order you picked them; pointing at one shows its colour code ("#3366CC") and an x, and a click
  takes the colour out (Delete, Backspace, Enter or Space on the focused swatch do the same; the
  arrows move between them). Picked colours that touch become one shape. The picked colours belong
  to this use of the tool and are not remembered. With none nothing is traced, and the Trace button
  is dimmed; its tooltip gives the reason, "0 paths: no colour picked."

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

An automatic analysis runs once per image, on your computer, when the image is first shown in the
tool, and applies its values at once. It sets only values that depend on the image: the method
(Brightness, or Remove background when the image has pale or light colours that Brightness would
miss), the background and Colour tolerance of Remove background (half way between the background
and the palest colour), and the Threshold (the automatic one, or a lower cut when a photographed
sheet has a wide shadow). It never changes the preset, Smooth, Tolerance, Ignore below, the
Advanced values, Output or picked colours; with Pick colours it keeps the method and sets only the
Threshold.

What it found is not printed; a screen reader hears it, for example "Line drawing · scan ·
automatic Threshold", "Logo · pale colours kept" or "Graphic · photo of paper · shadow left out":
the image type (Graphic, Logo, Line drawing, Sketch), how the image was made when it is a scan or
a photo of paper, then what was set. When the type is unclear (a photo without a sheet of paper)
it is "Type unclear · Brightness with automatic Threshold" and sets just that. Once you change the
method, the Threshold, the Colour tolerance or the background ("Custom · Line drawing"), the
analysis never changes those values again for this image; the blue **Analyse** button, last in the
Advanced block, sets them back to the analysis in one go (it is no History step; it is always
available, and with nothing changed it sets the same values again). If you change any setting
before the analysis has answered, its answer is not applied to this image; Analyse applies it on
request. If the analysis fails, the image keeps its automatic Threshold (a screen reader hears
"Analysis failed · automatic Threshold used") until the next Analyse or image; Analyse asks a fresh
worker, so it always answers.
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
  on the placed image in millimetres. Dark pixels that touch only at a corner count as one shape, so a thin diagonal line stays
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

1. **Output** (Single colour, Multicolour) and, with Multicolour, **Colours**, with Brightness
   the palette's pipette, and the palette's swatches; see "Multicolour" below. Four icon buttons
   follow (pressed means on, the tooltip names each).
2. The options: Denoise, Upscale and Corners.
3. **Parts**, the numbered chips of the parts with **Add part** and the trash (see "Parts"), and
   behind them the **Mask** group (see "Mask"), then at the far right the blue **Analyse** button
   (see Analysis).

The block is one flowing line: on a wide window everything stands on one line, and with the block
open the bar is as wide as that line needs (at most the window). When the width does not suffice,
whole items wrap; the Parts and Mask controls wrap as one unit first, then the Mask group goes to a
line of its own, then Add part and the trash go below the chips. (`specs/0114-image-trace-mask`
UX notes §2.)

- **Invert**: traces what the method leaves out (with Brightness the light parts: pixels at or
  above the Threshold); transparent pixels stay empty.
- **Drop border shapes**: drops the shapes that touch the border of the picture.
- **Recognise text** (off by default, remembered): reads the text first; see "Recognise text"
  below.
- **Recognise shapes** (off by default, remembered): sets rectangles, circles, ellipses, polygons
  and stars as shapes; see "Recognise shapes" below.

- **Denoise** (Off, Low, High): a median filter on the brightness before the Threshold; Low takes
  3 x 3 pixels, High 5 x 5. It removes scan noise and JPEG ringing.
- **Upscale** (Auto, Off): traces a copy at twice the size; Auto does so for images under 1
  megapixel. Small or low-resolution pictures keep their shape better.
- **Corners** (0 to 100, default 70): higher makes a bend a sharp corner more readily (70: bends
  sharper than 60 degrees). Corners are found on the pixel outline before it is calmed and placed
  where the two straight sides meet.

Invert and Drop border shapes start off with each start of the app.

## Mask

To trace only a piece of a picture, draw that piece on the picture itself. The **Mask** group in
the Parts row of Advanced holds **Rectangle**, **Lasso**, **Subtract**, **Inside | Outside** and
**Clear mask**. The regions you draw belong to the image: they are saved with it, come back when
you trace it again (also after reopening the file or on another device), move, scale, rotate and
mirror with it, go with a copy of it, and are deleted with it (one undo brings both back).
(`specs/0114-image-trace-mask`, ADR 0061.)

- **Rectangle**: while it is pressed, a drag on the canvas draws a rectangle on the picture; its
  corners land on whole pixels of the picture, Shift draws it from the centre, Ctrl a square, a
  drag under 2 px draws nothing. A chip by the pointer shows its size in mm while you drag.
- **Lasso**: while it is pressed, each click sets a corner on a whole pixel, and a line follows the
  pointer to the next one. A right click, a double click (its second click adds no corner), Enter
  or a click on the first corner finishes it, closed by a straight side from the last corner to the
  first. Backspace removes the last corner; Escape discards the lasso (nothing is added; a second
  Escape releases Lasso). Pressing Rectangle, picking another part, Add part, Remove part, the
  pipette, the Trace button, choosing another image in Layers or leaving the tool closes an
  unfinished lasso the same way. A lasso needs 3 corners not on one line; with fewer it is
  dropped. A lasso that crosses itself covers every area it winds around (non-zero winding). There
  is no context menu while you draw, and Shift, Ctrl and Alt mean nothing while drawing.
- A tool stays pressed until you press it again, press Escape or leave the tool; Rectangle and
  Lasso exclude each other and the pipette.
- **Subtract**: the regions finished while it is pressed are taken out of the mask; they are
  drawn dashed, and a minus badge by the pointer shows while you draw one. The mask is the
  regions drawn without Subtract minus those drawn with it, whatever the order; overlaps count
  once.
- With no tool pressed, a click on a region of the active part selects it (a box around it shows
  under the pointer first). A drag moves it; a rectangle's 8 handles resize it (Shift about the
  centre, Ctrl keeps the proportions at a corner); a lasso shows its corners, and a drag on one
  moves that corner alone. The arrow keys move the selected region 1 px (Shift 10 px), Delete or
  Backspace removes it. Escape cancels a drag, then discards a lasso in progress, stops a running
  trace, disarms the pipette, releases the pressed tool, drops the selected region and last leaves
  the tool, one step per press.
- **Inside** traces only what lies inside the mask, **Outside** only what lies outside it. Without
  regions the whole image is traced either way. On an untouched picture the choice is not
  remembered (Inside on every entry); once the picture has a region or a second part, its mode is
  stored with it.
- **Clear mask** (the x) deletes the regions of the active part in one step; the part and its
  settings stay. Without regions it does nothing.

Every finished edit is one History step: "Draw rectangle", "Draw lasso", "Move region", "Resize
region", "Move lasso corner", "Delete region", "Clear mask", "Mask: Inside" or "Mask: Outside"; a
drag is one step. Undo and redo work while the tool is open. The regions are no objects: they are
not in Layers, not exported and not snapped to; every part's regions are drawn as thin black lines
above the drawing while the Trace tool shows the image, only the active part's can be clicked.

The trace ends exactly on the edge of the mask: a shape that crosses it is cut there, so Inside and
Outside of the same mask fit together without a gap or an overlap. A cut shape comes back as
straight segments along the cut; shapes away from the edge keep their curves. The automatic
Threshold, the suggested background and the analysis then read only the pixels inside the mask.
With Recognise text, a line becomes text when the centre of its box lies inside the mask (the text
is kept whole); a line outside it is neither text nor traced.

In the preview the image outside the mask is washed further (to 12 %), and the outline is drawn
only inside it. A screen reader hears "Inside 2 paths" or "Outside 1 path" (the count is of the
regions). Regions that cover none of the image give "0 paths: the mask is outside the image." and
nothing to create; regions that cannot be combined into one area give "0 paths: the mask paths
could not be combined."

A **linked clone** of an image shows the original's regions and parts on the clone, and what you
draw there changes the original's (one mask per picture); deleting the clone keeps them, and
**Unlink** gives the clone a copy. **Ctrl+C** and **Ctrl+V** carry the regions to the pasted
picture within one Curvyo window (also into another document there); from another program only
the picture arrives. On a locked image the regions are shown and traced, but the Mask group, Add
part and the trash are unavailable ("Locked"). With two or more images selected the Parts and Mask
controls are not shown; Create paths traces each image with its own stored parts and regions (an
image without any whole, with the bar's settings).

Create paths makes the same group "Trace of ..." as without a mask, in one History step; the
regions stay on the image.

## Parts

A part is a piece of the picture with its own mask (its regions, Inside or Outside) and its own
settings (Method, Threshold, Smooth and everything else). The Parts row of Advanced holds the muted
label "Parts", a chip per part, numbered 1, 2, 3 in the order they were made, then **Add part** (+)
and the trash, and behind them the Mask group. Part 1 starts as the whole image; with regions it
is limited to them like any other part. (`specs/0094-masked-composite-trace` slice 2,
`specs/0114-image-trace-mask`.)

- **Add part** makes the next part active: it starts with a copy of the settings of the part
  before (without its picked colours), no regions and Inside. Until it has regions its chip is
  muted (dashed) and it adds nothing. At most 8 parts.
- A **click on a chip** (or the arrows, Home and End on the focused chips) makes that part active:
  its settings load into the bar and its regions can be clicked. Everything the bar and Advanced
  show then belongs to the active part (the active chip is the only mark of it). Drawing, Clear
  mask and Inside / Outside work on the active part only; the pressed tool and Subtract stay.
- The **trash** removes the active part with its regions and settings (Delete on a chip arms it
  too): the first press turns the Parts group soft red, the second press after a short pause
  removes it; the parts renumber and the part before becomes active. With one part the trash is
  dimmed.
- **Order**: left to right is also bottom to top. Where parts overlap, the later one wins: inside
  its mask only its shapes exist, and the earlier parts are cut exactly at its edge, so no area is
  traced twice.
- **Create paths** (the Trace button) joins all parts into one group "Trace of ...", in one History
  step: shapes that cross from one part into another are joined into one path along the seam, and
  same colours merge in Multicolour. The preview shows the joined result; changing a setting traces
  only the active part again (with a mask change the parts below it follow, since their area
  changes).
- Recognise text and Recognise shapes are settings of each part: a text line or a shape belongs to
  the part that holds its centre.
- The parts are stored with the picture, each with its settings: Add part, the trash and a settings
  change of a stored part are History steps ("Add part", "Remove part", "Change trace settings";
  a slider drag is one step at its end), and undo and redo show the values again. Leaving the tool
  and Create paths keep the parts; on the next entry the same parts are there with part 1 active
  and the bar shows its settings. The automatic values (an Auto Threshold, Remove background's
  Auto) are read from the picture again, so the preview and Create paths give what they gave
  before. A picture with no region and one part stores nothing: its bar shows the remembered
  settings, as before.

With two or more parts a screen reader hears "3 parts", and for an active later part without
regions "Part 3 has no paths". When no part has anything to trace, the bar says "0 paths: no part
has anything to trace." A part inside the picture is traced from its own cut-out (with a margin, so
the result is the same) unless it reads text, drops border shapes, uses Denoise or is upscaled, so
several parts cost about as much as one trace of the whole picture. Each part reads its automatic
Threshold, background and analysis from its own area; what you set for a part stays with it.

## Recognise text

With **Recognise text** on (Advanced, off by default, a setting of the app) Trace reads the text in
a black and white trace before it traces the rest (`specs/0091-trace-text-recognition`):

- The first preview of a picture reads its text once, offline, with the model shipped in the app
  (PaddleOCR Latin, Apache-2.0, about 10 MB; nothing is downloaded and nothing leaves the
  machine). The Trace button shows the progress as "Reading text" and can stop it; later previews
  of the same picture in the session reuse the lines.
- Each line the reader is sure of (mean confidence 0.6 and a letter or digit) gets its own
  Threshold and polarity (white text on a dark box counts as text), is traced on its own, and its
  pixels are left out of the main trace and filled from their surroundings, so a shape under the
  text stays whole. A screen reader hears "3 text lines set".
- Before the picture is traced, each line is fitted to the fonts as Recognise text does (fonts
  in the document, favourites, your fonts, the included fonts, the other installed fonts), with
  the read string as a hint and a tolerance of 1.5 picture pixels (3 % of the letter height on
  large text). Thin strokes are also compared by their centre line with the single-line fonts, so
  a line drawn in EMS Readability is set in EMS Readability. Then the picture is traced once, and
  the Trace button shows one percent from 0 to 100 for the whole job.
- Create paths writes one text per line, black, above the "Trace of" group, in the same step
  (a single-line text as a black stroke of the traced width). A line no font fits within the
  tolerance is set in Source Sans 3 anyway (the weight and lean of its strokes), every letter
  guessed: a wrong font is better than paths. Only a line that looks like no text stays paths: a single character, mostly marks, read
  with less than 0.8 confidence, or one short pattern repeated (a ring read as "O", a dot grid
  read as "TOTOTO"). Guessed letters are named in one warning and highlighted in the Glyph table
  and the Text tool until the first edit; when all letters are sure, nothing is announced.
- With Multicolour the same happens before the colour trace: the palette is the one of the
  picture as it is (a colour only the text has stays in it), the shapes under the text stay whole,
  and each text takes the palette colour nearest its letters, or the background colour (white
  text on a red box is white text above the whole box).
- Text turned upside down, or standing at about 90° either way, is read too: a line the reader
  is unsure of, or one steeper than 45°, is read once more turned round, the surer reading is
  kept, and its text is turned to match, so it reads the right way round on the canvas. Lines
  that read upright and sure are read once, as before.
- With Recognise text off, Trace works exactly as before and the model is not loaded.

## Recognise shapes

With **Recognise shapes** on (Advanced, off by default, a setting of the app, not part of the
presets) every closed traced outline that is a rectangle (with or without rounded corners), a
circle, an ellipse, a regular polygon or a star becomes that shape, in place, at its size and
turn (`specs/0088-trace-recognition` Part B). It works with Single colour and Multicolour:

- A shape is set only when every point of the traced outline lies within the tolerance of it:
  2 picture pixels, or 1 % of the square root of the outline's area on large shapes (0.2 mm at
  10 pixels per mm; a 50 mm circle may stray 0.44 mm). So the result does not depend on the dpi,
  and an outline that is only nearly a shape (a hand-drawn circle) stays a path, unmarked.
- The simplest shape wins: a circle before an ellipse, square corners before rounded ones, a
  rectangle before a polygon, a polygon (3 to 24 sides) before a star (3 to 24 points).
- The preview shows the shapes where the paths were, in the same fill. A screen reader hears
  "12 shapes set"; with none found nothing.
- Create paths puts the shapes into the "Trace of" group, after the path of their colour, in the
  same one step. They are ordinary shapes: rectangles, ellipses, polygons and stars with their
  handles.
- Bands: a ring, a frame or an outline drawn as a line of even width (a shape with one hole of
  the same kind, its sides the same distance apart all round) becomes that shape along the middle
  of the line, with a stroke as wide as the line and no fill: a circle, rectangle, polygon or star
  you can edit and whose line width you can change.
- Holes: any other shape with holes becomes one compound path of clean outlines, each hole that
  is a shape as that shape's outline and any other hole as traced (a line that gets wider or
  narrower stays like this). An outline that is no shape keeps its holes as traced paths, whatever
  they are.
- With Recognise text on too, the text is decided first: letters become text or stay paths,
  never shapes.
- With Recognise shapes off, Trace works exactly as before.

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
Ignore below join the neighbouring colour they share the longest border with (a screen reader
hears "2 regions joined"). With Brightness and with Remove background the pipette after the field
adds a colour you pick to the palette (it stays armed; Shift+click adds more, Esc ends it); a
colour that is already there adds nothing (a screen reader hears "Same as #3366CC"). With Pick
colours the palette is exactly the picked colours beside the pipette and the field shows their
number.

The **palette** is the row of swatches right after the palette pipette, one per colour of the
result, largest area first, colours picked with the palette pipette first. A picked colour takes
one of the Colours; the others go to the other colours of the image. A part that traces nothing
yet (no regions, or covered by another part) shows its own picked colours: the palette pipette
adds to them, the x removes one, and they apply when the part gets regions. The swatches are the picked colours of
Pick colours: 20 px squares 4 px apart, the row as wide as its colours, so the bar does not grow a
row for them (with Pick colours there is no palette row; the picked colours sit beside the pipette
in the bar). Pointing at a swatch fades it and shows an x; its tooltip gives its colour code and
name ("#4B392F · Brown") and, small, "Ctrl+click: exclude":

- **The x**: deletes the colour. The palette becomes the other colours, and every pixel is
  matched again among them, as if the deleted colour had never been found: each of its pixels
  goes to the colour nearest to it (to the background only where that colour is excluded). With
  Colours at Auto the field then shows the smaller count ("Auto  5"); a set count stays as it is.
  A screen reader hears "Removed #4B392F". Every colour can be deleted; with none left nothing is
  traced and the Trace button is dimmed (its tooltip: "0 paths: no colour in the palette."), and
  the palette pipette adds colours again. A new method, a new Colours value or another image
  starts from the found palette again.
- **Ctrl+click** (Cmd on a Mac): excludes the colour: it becomes background and is not traced; the swatch is
  drawn faded with a crossed circle (a screen reader hears "2 colours excluded"). A second
  Ctrl+click brings it back.
- A plain click elsewhere on the swatch does nothing.

On the keyboard the swatches are one Tab stop: the arrows, Home and End move between them, Enter
or Space excludes, Shift+Enter, Delete or Backspace deletes. A swatch's name gives its colour code,
its name and "excluded" when it is.

## Create paths

## Status

The bar does not print the counts of a trace. A screen reader hears them once the result settles,
for example "7 paths, 412 nodes, 0.4 s" (paths are outlines, nodes their anchor points, the time is
that of the last full trace) and "38 specks ignored" when specks were dropped; with Multicolour
the colours first, for example "3 colours, 7 paths, 412 nodes, 0.4 s". After Create paths the
Layers tooltip of the new group gives them ("Traced: 7 paths, 412 nodes.").

The bar shows no status or notice line at all. When a result cannot be created, the Trace button
is dimmed and its tooltip gives the reason; a screen reader hears it as well:

- "0 paths at this Threshold: too low." (or "too high."); with Remove background "0 paths at this
  Colour tolerance: too high.", with Pick colours "0 paths at this Colour tolerance: too low."
- "No colours found." (Multicolour found no colour in the content).
- "0 paths: no colour picked." (Pick colours with no colour picked) and "0 paths: no colour in
  the palette." (every palette colour deleted).
- "Too much detail: raise Ignore below.": one shape has 65,000 holes or more.

A screen reader also hears what used to be shown as notices: "1 path covering the whole image.",
"412,000 nodes. Slow to edit." (more than 100,000 nodes; Create paths works, and the result is
still one group with one compound path, one row in Layers), "Preview from a reduced copy",
"Same as #3366CC" and "Analysis failed · automatic Threshold used".

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
- Mask: the regions drawn on the picture that limit the trace; Rectangle drags one, Lasso sets its corners one click at a time and closes them with a right click, a double click, Enter or a click on the first corner (Esc discards it), Subtract takes the next regions out, Inside traces what lies inside them, Outside what lies outside, Clear mask deletes the active part's regions.
- Method: how pixels become shapes.
- Brightness: pixels darker than the Threshold.
- Remove background: everything unlike the background colour.
- Background: the colour Remove background leaves out; Auto takes it from the image border.
- Pick colours: pixels like the colours picked with the pipette; Shift+click adds a colour, up to 8. The picked colours sit beside the pipette; a click on one takes it out.
- Pipette: takes a colour from the image under a click; Esc ends it.
- Colour tolerance: how far a colour may be from the background or a picked colour and still count as it.
- Threshold: pixels darker than this brightness (0 to 100 %) become shapes.
- Smooth: calms the pixel steps of the outline before the fit; higher is calmer.
- Tolerance: how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.
- Ignore below: drops shapes and holes smaller than this size in mm.
- Output: Single colour traces the content as one black shape; Multicolour gives one filled shape per colour, neighbours sharing their edges.
- Colours: how many colours Multicolour finds, 2 to 16; Auto counts them in the image. With Pick colours the picked colours are the palette.
- Palette: the colours of a Multicolour trace beside the palette pipette, largest area first. The x on a colour deletes it: the others are matched again as if it had never been found; with no colour left nothing is traced. Ctrl+click excludes a colour: it becomes background and is not traced; a second Ctrl+click brings it back.
- Invert: traces what the method leaves out; transparent pixels never become shapes.
- Drop border shapes: drops shapes that touch the image border.
- Recognise text: reads the text in the picture first, sets each line as text in the nearest font and leaves it out of the paths.
- Recognise shapes: finds rectangles, circles, ellipses, polygons and stars in the traced outlines and sets them as shapes.
- Denoise: smooths out noise in the picture before tracing.
- Upscale: doubles small images before tracing; Auto does it below 1 Mpx.
- Corners: higher turns more bends into sharp corners.
- Parts: pieces of the picture, each with its own mask and settings; Add part makes the next one, the trash removes the active one, and Create paths joins them into one group.
- Analyse: sets the method, its background and Colour tolerance, and the Threshold back to what the analysis found in the image; never the preset or other values.

## Create paths

The blue **Trace** button at the far right of the bar, Enter on the canvas, Ctrl+Enter in a field,
or a double-click on the image adds one group "Trace of <image name>" directly above each image, with one compound
path inside, filled black, without stroke, holes as holes. With Multicolour the group holds one
compound path per colour, filled with that colour, named by its hex value,
largest area at the bottom. The image stays visible and unchanged
under it; hide or move it as usual. The new groups are selected and the Select tool is active; no
notice appears, the selected group is the feedback. The tooltip of the group's row in the Layers
tab says "Traced: 7 paths, 412 nodes." (until the app is closed). It is one step "Trace image" in the History: Ctrl+Z removes every group of it. With
several images selected the preview shows the image you clicked last (a screen reader hears
"Tracing 1 of 3 images"); every image is traced with the same settings, except that an image with
stored regions or parts is traced with its own (see "Mask"). If only the reduced copy has been
traced, Create paths waits for the full picture; Escape stops before anything is added.

The result is ordinary paths: they move, scale, rotate and edit like any group, take part in
every output, and do not follow the image if it is moved later.

## Privacy and rights

Tracing happens on your computer; nothing is sent anywhere. You are responsible for having the
right to trace an image.
