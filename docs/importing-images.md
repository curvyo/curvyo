# Importing images

The **Import** button of the tool rail (the card under the Offset tool) and **Ctrl+I** (Cmd+I on
macOS) open one file dialog for drawings and pictures: SVG, PNG, JPEG and WebP, up to 20 files at
once. Files dropped on the canvas are imported where they are dropped; Ctrl+V pastes a picture
from the clipboard ("Paste and copy" below). Each file is read by its
content, never by its name: a PNG called `photo.jpg` imports as a PNG. An SVG becomes a new layer
(see `docs/svg-import.md`); a picture becomes an **image object** on top of the active layer,
selected. One import is one step in the History ("Import image", "Import images", or "Import"
for drawings and pictures together); Ctrl+Z removes it. Ctrl+I does nothing while you type in a
text field or edit a text on the canvas. Open… still opens an SVG as a project; it does not open
pictures. Pictures embedded in an SVG come in with the drawing as image objects, at the size the
SVG gives them (`docs/svg-import.md`, "Images"). (`specs/0058-bitmap-import`)

## The preview in the dialog

On **Linux** the Import dialog shows the highlighted picture on its right with its pixel size
under it ("3000 × 2000 px"), read from the file's header. "Enable preview" at the bottom left is on
by default; switched off, the pane is hidden. A file over the size limits (see "Size and place")
shows its name and "Too large" and is never decoded; a damaged or unreadable file or any other
file shows its name only; a folder shows nothing. An **SVG** shows its page as Import SVG would place
it, on white, with the page size in millimetres under it ("210 × 297 mm", the page-size rules of
`docs/svg-import.md`): fills and solid strokes, hairlines at least 1 px wide, and texts in the
fonts Import SVG would give them (Curvyo fonts, then the included fonts, then the fonts of this
computer; a missing font in the included sans, as on import), straight or on their path; embedded
pictures, dashes and texts inside masks or clips are left out of the preview. The fonts are listed
once per open dialog, and only the faces a file needs are read. An SVG over 50 MiB shows "Too large"; one Import SVG
refuses (compressed, entities, not SVG) or one that draws nothing shows its name only. The preview
reads only the highlighted file, with the same decoder or SVG reader and limits as the import, and
never writes. On **Windows** the dialog's own
preview pane is used (switched on once with Alt+P, then Windows remembers it); on **macOS** the
dialog's own Quick Look preview in column and gallery view. In the **browser** the browser's own
file picker is used, which has no preview. (`specs/0087-image-clipboard-and-drop` criteria 26 to 28)

## Formats

PNG (all bit depths, palettes, transparency, interlaced), JPEG (baseline, progressive, grey,
CMYK) and WebP (lossy, lossless, transparency; of an animation only the first frame). Every other
file, BMP, GIF and TIFF included, is refused.

## Size and place

- **Size.** A PNG's `pHYs` density or a JPEG's JFIF density gives the size; a file without one,
  or with a density below 10 or above 2,400 dpi, comes in at 96 dpi. An EXIF resolution is not
  read. Horizontal and vertical densities are used separately.
- **Fit.** A picture larger than the document is scaled down so that it fits; it is never
  enlarged. The document size never changes.
- **Place.** The picture's centre lands on the centre of the view and is moved the least needed
  to lie inside the document; dropped files land with their centre at the drop point. Several
  files step 10 mm right and down.
- **Orientation.** A photo turned by its camera (EXIF orientation) is shown upright.
- **Working resolution.** A picture of more than 16 megapixels, or more than 8192 pixels on a
  side, is shown and traced at a reduced resolution; the file in the project keeps the original.

## Colour

Every pixel is taken as sRGB. Colour profiles (ICC), gamma and other colour data in the file are
ignored and nothing is converted, so a photo in Adobe RGB or Display P3 looks slightly flat.
16-bit channels are rounded to 8 bits. Transparency is kept: what is behind shows through.

## In the project

The file's bytes are stored once in the project, unchanged, also when the same picture is
imported twice or an image is duplicated. They include the photo's metadata, such as a GPS
position. A picture stays in the project file while the History can bring its object back; Wipe
history removes pictures nothing uses any more. A project holds at most 512 MB of pictures.

## On the canvas

An image is selected by a click anywhere inside its frame, transparent parts included. It moves,
scales, rotates, skews and flips like a rectangle; a stretch or skew distorts the picture and it
never becomes a path. It is not part of any cut, score or engrave path. Fade a reference picture
with the Opacity field of the Layers tab; hide or lock it there. Zoomed in to 8 screen pixels per
image pixel and more, the pixels show as squares. Boolean operations, Combine, Split, Break
apart, Cut, Fracture, Flatten, Weld, Offset and Stroke to path refuse a selection with an image.

## Paste and copy

- **Paste.** **Ctrl+V** (Cmd+V on macOS) on the canvas pastes the picture on the clipboard, such
  as a screenshot or an image copied in a browser or another program. It is imported by the rules
  of this page (formats by content, size and fit, limits and notices) as one step "Paste image" in
  the History; Ctrl+Z removes it. Its centre lands under the pointer while the pointer is over the
  canvas, as for a drop (not moved into the document); else on the centre of the view, moved the
  least needed to lie inside the document. It is
  named "Pasted image". When the clipboard holds a picture and text, the picture is pasted. Text,
  an SVG and other files are not pasted.
- **Files copied in the file manager.** In the desktop app on Linux, picture and SVG files copied
  in the file manager are pasted with Ctrl+V even when the window receives only their names (a
  list of `file://` addresses or paths, as GNOME's Files and WebKitGTK do): the app reads the list
  from the clipboard and the files from the disk and imports them by the rules of this page, under the pointer or in the view
  centre, each named by its file, in one step ("Paste image" for one picture). Only ordinary
  files on this computer are read (a link to a file is followed; folders, devices and network
  addresses are not), and a file over 100 MB is refused before it is read. On Windows and macOS
  the window receives the copied files themselves. The browser cannot read files by their names:
  there a copied file is pasted only where the browser hands over the file itself. While you type in a
  text field or edit a text on the canvas, Ctrl+V pastes text there as before. With the Trace tool
  active the pasted picture is traced and the tool stays.
- **Copy.** **Ctrl+C** (Cmd+C on macOS) with an image selected puts its picture on the clipboard as
  PNG, to paste into other programs. It is the picture as stored, upright and with its
  transparency, without the object's rotation, flip, size or opacity, and without the file's
  metadata; a picture of more than 16 megapixels is copied at its reduced working resolution. With
  several images selected the topmost is copied. A JPEG or WebP picture copied and pasted back
  comes in as a new PNG, so the project stores it a second time. With no image selected, Ctrl+C
  does nothing; there is no copy and paste of paths and texts yet.
- Nothing is downloaded: a pasted link to a web address is not fetched.

## Drop

- **Files.** Picture and SVG files dropped from the file manager on the canvas are imported with
  their centre at the drop point, in one step ("Import image", "Import images" or "Import"). Of a
  drop that mixes pictures and other files, the pictures come in and one notice names the rest. A
  dropped `.curvyo` project opens nothing.
- **Image data.** A picture dragged from a browser or another program comes in the same way; it
  takes the file name the program gives, else "Dropped image".
- **File names.** In the desktop app on Linux, files dragged from a file manager that hands over
  only their names (`file://` addresses or paths) are read from the disk and imported like dropped files, by
  the same rules as files copied in the file manager (see Paste and copy).
- **Links.** A drop that carries only a link to a web address (or a picture of a web page that
  the browser hands over as its address) is not downloaded; the notice says so. The browser
  cannot read files by their names, so there a dropped file name is a link too.
- **Elsewhere.** On the panel, the rails, the rulers and the bars nothing is dropped. While a
  drag is over the canvas a dashed outline shows. With the Trace tool active a dropped picture
  is traced and the tool stays.
- **Desktop and browser.** Both use the same drop. In the desktop app the window hands every
  drop to the page, so dropping fonts on the Fonts tab works as in the browser: their bytes are
  checked and added like files chosen in Add font. Which programs hand over image data or only a
  link differs between browsers and systems.

## Notices

- Scaled to 20 % to fit the document.
- Shown and traced at reduced resolution (16 MP). Original is kept.
- This image adds 24.0 MB to the project file.
- Animated image: only the first frame was used.
- photo.jpg is incomplete; the part that could be read was imported.
- Imported 20 of 31 images. The rest were skipped.
- 2 files could not be imported.
- This file type isn't supported. Use SVG, PNG, JPEG or WebP.
- The image is damaged and could not be read.
- Image too large: 12,000 x 9,000 px. The limit is 50 megapixels.
- File too large: 140 MB. The limit is 100 MB.
- Project too large: this image adds 140 MB. The limit is 512 MB.
- Not enough memory to open this image.
- An image in this project could not be read.
- Images can't be used with this command. Nothing was changed.
- Nothing to paste: the clipboard has no image.
- Pasting image...
- Copying...
- Could not copy the image: <short reason>.
- Copying images needs a newer browser.
- That drop is a link, not an image. Nothing was downloaded.

A file of 50 megapixels can keep the window busy for some seconds while it is read; there is no
cancel.
