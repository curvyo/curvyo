# Importing images

The **Import** button of the tool rail (the card under the Offset tool) and **Ctrl+I** (Cmd+I on
macOS) open one file dialog for drawings and pictures: SVG, PNG, JPEG and WebP, up to 20 files at
once. Files dropped on the canvas are imported where they are dropped. Each file is read by its
content, never by its name: a PNG called `photo.jpg` imports as a PNG. An SVG becomes a new layer
(see `docs/svg-import.md`); a picture becomes an **image object** on top of the active layer,
selected. One import is one step in the History ("Import image", "Import images", or "Import"
for drawings and pictures together); Ctrl+Z removes it. Ctrl+I does nothing while you type in a
text field or edit a text on the canvas. Open… still opens an SVG as a project; it does not open
pictures. Pictures embedded in an SVG come in with the drawing as image objects, at the size the
SVG gives them (`docs/svg-import.md`, "Images"). (`specs/0058-bitmap-import`)

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

A file of 50 megapixels can keep the window busy for some seconds while it is read; there is no
cancel.
