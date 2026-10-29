# Plan for Image clipboard, drag and drop, and a preview in the import dialog

Slices 1 and 2 are planned on `story/image-clipboard`; this file holds slice 3 only on its own branch.

## Slice 3: a preview in the import dialog (criteria 26 to 29)

Implementer, 2026-10-28, on `story/import-preview` from `main`. Design: `adrs.md` decision 6.

### Affected crates/modules

- **`curvyo-image-core`:** `thumbnail(bytes, longest) -> Result<WorkingImage, ImageError>` in a new
  module `thumbnail`: the header and ceilings of `probe` first, then the bounded `decode_working`,
  then `reduced_copy` to the longest side (criteria 26 and 28).
- **`curvyo-app`:**
  - `import_preview` (pure, every OS): what the pane shows for one highlighted path, from the file's
    length and a byte reader: a picture with the size line "W × H px", "Too large" (over the file or
    pixel ceilings, never decoded), the name only (unreadable, not an image), or nothing (a folder).
  - `import_dialog` (`cfg(target_os = "linux")`): the GTK3 `FileChooserDialog` of the Import button
    with the same filters as today, multi-select, the preview pane on the right and an "Enable
    preview" check (on by default) as extra widget; decodes off the GTK thread (`gio::spawn_blocking`),
    drops stale results; runs on the main thread through `run_on_main_thread` and answers through a
    channel. `pick_import_files` uses it on Linux and keeps `tauri-plugin-dialog` elsewhere.
  - `Cargo.toml`: `gtk = "0.18"` (Linux only) and `curvyo-image-core`.
  - SVG preview (customer request on PR #217, lead decision 2026-10-29): `svg_preview` (pure: bytes
    in, page picture and page size in mm out, through `curvyo-svg-core::read_svg` and its limits)
    and `raster` (an anti-aliased scanline fill of `kurbo` paths, edge budget, off-picture segments
    dropped or replaced by their chord); strokes through `kurbo::stroke`. `import_preview` sends an
    `.svg` there with the SVG importer's 50 MiB ceiling. Linux-only deps `curvyo-svg-core` and
    `kurbo` (both already in the workspace). No new crate: `tiny-skia` would add unsafe SIMD code and
    several crates for one fill; `resvg`/`usvg` would be a second SVG parser outside our limits
    (`0024` criterion 73).
  - SVG texts (customer comment on PR #217, lead decision 2026-10-29): `preview_fonts` lists the
    three font blocks of `0074` once per dialog (Curvyo folder and system fonts through
    `curvyo-storage-io`, the included fonts from the bundled page's `fonts/index.json` through
    Tauri's asset resolver) when the first file with text is previewed, and reads a face's bytes the
    first time a file needs it; `svg_preview` draws each text in z-order through
    `curvyo-ui-core::svg_text_glyphs` (new: the pieces of `svg_import`'s text conversion laid out by
    the engine as Object to path does, on their carrier for a `<textPath>`), the glyph segments
    charged to the work budget. Linux-only deps `curvyo-ui-core`, `curvyo-text-core`,
    `curvyo-document-core` (workspace crates). Listing the system fonts runs in the decode thread
    (measured: 30 faces, 174 ms cold, 7 ms warm), never on the GTK thread.
- **Docs:** `docs/importing-images.md` (the preview per platform, the browser deviation),
  `docs/technical-debt.md` (GTK4 move; a Flatpak/portal build loses the pane), `adrs.md` (crit. 27
  deviation is already recorded in decision 6).

### Tasks

- [x] 1. Test first: `thumbnail` of a 3000 x 2000 px PNG has longest side 256, aspect within 1 px;
  the iCCP bomb, the 60000 x 60000 header and a file over 100 MiB are refused before a decode
  (fulfils AC 26, 28).
- [x] 2. `curvyo-image-core::thumbnail` (fulfils AC 26, 28).
- [x] 3. Test first: `import_preview` for a PNG ("3000 × 2000 px" and a 256 px picture), a JPEG, an
  over-ceiling file ("Too large", the reader is never called), an unreadable file and a text file
  (name only), a folder (nothing) (fulfils AC 26, 28).
- [x] 4. `curvyo-app::import_preview` (fulfils AC 26, 28).
- [x] 5. `curvyo-app::import_dialog`: GTK3 dialog with the pane and the check; wired into
  `pick_import_files` on Linux; Open/Save and fonts unchanged; cancel returns an empty list, so the
  page keeps its focus rule (fulfils AC 26, 29).
- [x] 6. Docs: user page, technical debt (fulfils AC 27).
- [x] 7. Manual check under Xvfb with a screenshot; full gate and `cargo deny check`.
- [x] 8. Test first: `svg_preview` for an A4 page (181 x 256 px, "210 × 297 mm", pixels), `0024`'s
  page-size rules (viewBox only, content box), a hairline stroke (1 px), hidden objects, a clone,
  the importer's refusals and an empty file; `raster` for coverage, even-odd, far-off and budget;
  `import_preview` for an `.svg` (picture and mm line, over 50 MiB never read, refused: name only)
  (fulfils AC 26, 28).
- [x] 9. `curvyo-app::raster`, `curvyo-app::svg_preview`, `.svg` routing in `import_preview`;
  spec criterion 26, user page; manual check under Xvfb with an SVG (fulfils AC 26, 28).
- [x] 10. Test first: `svg_preview` with the included fonts: a Source Sans 3 text covers its
  letters' box and nothing else, the listing and the one face are read once per dialog; a missing
  face draws as the substitute, no font source leaves the text out. Then `preview_fonts`,
  `svg_text_glyphs`, texts in `svg_preview`'s walk; spec criterion 26, user page; manual check
  under Xvfb with the Inkscape text fixture of `0024` (fulfils AC 26, 28).

### Validation

- Unit tests for `thumbnail` and `import_preview` (criteria 26 and 28), with the `0058` hostile
  fixtures.
- Manual: the built app under Xvfb, Import, a 3000 x 2000 px PNG highlighted, screenshot; the same
  with an Inkscape A4 SVG ("210 × 297 mm"), the Wikimedia symbols of the `0024` fixtures and
  `0024`'s Inkscape text fixture (`svg_real/inkscape_text/text_ink.svg`: faces, lines, text on path).
- Windows and macOS keep the OS dialog (recorded deviation, `adrs.md` decision 6); their manual check
  with screenshots belongs to the PR (nightly runners have no desktop session).
