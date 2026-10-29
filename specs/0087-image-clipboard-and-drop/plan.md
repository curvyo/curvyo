# Plan for Image clipboard, drag and drop, and a preview in the import dialog

# Slice 1: Ctrl+V and Ctrl+C for images (criteria 1 to 18)

Implementer, 2026-10-28, slice 1 (Ctrl+V and Ctrl+C for images, criteria 1 to 18) on
`story/image-clipboard` from `main`. Design: `adrs.md` decisions 1, 2, 3, 5, 7. Slices 2 and 3 get
their own plan sections when they are built.

## Affected crates/modules

- **`curvyo-image-core`:** `encode_png(&WorkingImage) -> Result<Vec<u8>, ImageError>` in a new
  module `encode`: RGBA8 PNG, fast compression, adaptive filter, no metadata chunks; refuses a
  picture over the working limits (`TooManyPixels`) and a pixel buffer that does not match its size
  (`Damaged`), so the encoder never panics. Golden file `tests/fixtures/encoded/`.
- **`curvyo-document-core`:** the commit label `paste_image` next to the import labels.
- **`curvyo-ui-core`:** the history name "Paste image"; the clipboard notices
  (`image_clipboard_text`); `paste_rank` (which clipboard file a paste takes: image content first,
  then an image MIME type that the import refuses with its notice, never SVG or text).
- **`curvyo-editor-wasm`:** `ImportOrigin` (picker, drop, paste) on `Session::import_begin_from`:
  History name and default object name only; the Trace tool stays active for an import of images
  only; `Session::image_copy` (the topmost selected image's working pixels and the count of
  selected images); `Session::clipboard_allowed` (the chord gate); the wasm surface (`import_begin`
  with an origin, `image_copy`, `encode_png`, `paste_rank`, `clipboard_notice`).
- **`frontend`:** `useImageClipboard` (document `paste` listener, Ctrl+C keydown, the last pointer
  position over the canvas, the busy and copy notices), the `png` job of the trace worker, the paste
  origin of `importFiles`.
- **Docs:** `docs/design-system.md` (shortcut table, notices), `docs/importing-images.md` (paste and
  copy rules, notices).

## Tasks

Milestone 1, core (tests first):
- [x] 1. `encode_png`: round trip through `decode_working` keeps every RGBA byte, translucent pixels
      included; only IHDR, IDAT and IEND chunks; the working limits and a bad buffer are refused;
      golden file of a 40 x 20 px marker picture (AC 11, 12, 14).
- [x] 2. Label `paste_image` and the name "Paste image" (AC 1).
- [x] 3. Clipboard notices and `paste_rank` (AC 4, 5, 7, 13, 14, 15).

Milestone 2, session:
- [x] 4. `ImportOrigin` and `import_begin_from`: paste is one step "Paste image", default name
      "Pasted image", placement under the point or at the view centre with the `0058` clamp;
      refused data changes nothing (AC 1, 2, 5, 7, 8).
- [x] 5. Trace tool kept for an import of images only, any origin (AC 9).
- [x] 6. `image_copy`: the topmost selected image's stored pixels, upright, without the object's
      transform; the count of selected images; nothing for a selection without an image (AC 12, 13).
- [x] 7. `clipboard_allowed`: no drag, entry chip, long operation or text edit (AC 3, 16).
- [x] 8. Budget: `image_copy` of a 16 Mpx image within 50 ms of thread CPU time (release); the
      encode times of a 2000 x 2000 px photo-like picture and of 16 Mpx printed for the PR (AC 14).

Milestone 3, page:
- [x] 9. Wasm surface and the `png` job of the trace worker (AC 11, 14).
- [x] 10. `useImageClipboard`: paste from the `paste` event (items, then files), never in a text
      field or the text editor; under the last pointer position, else the view centre; Ctrl+C writes
      one `ClipboardItem` with a `Promise<Blob>` created inside the key press; "Copying..." after
      400 ms; the failure notices (AC 1 to 16).
- [x] 11. Docs and the notice test against `docs/importing-images.md` (AC 18).
- [x] 12. Browser check in Chromium (synthetic paste event, Ctrl+V from the clipboard, Ctrl+C read
      back, undo) and the WebKitGTK harness of the architect's spike (AC 10, 11).

## Validation

- `cargo nextest run -p curvyo-image-core -p curvyo-ui-core -p curvyo-editor-wasm`; the budget test
  in release.
- `frontend/tests/acceptance_0087_browser.test.ts` with `CURVYO_E2E_BROWSER=/usr/bin/chromium`.
- Full gate of `CLAUDE.md` §7 once at the end.

## Not covered by an automated test

- AC 6 (a file copied in the file manager) and the desktop checks of AC 10 and 11 on Windows and
  macOS: manual, reported in the PR per platform.
- AC 17: no test by design (rule recorded in `adrs.md` decision 7).

## Notes for the lead

- `encode_png` returns a `Result` (the ADR wrote `-> Vec<u8>`): `image`'s encoder panics on a
  buffer that does not match the size, and a working image over the limits must not be encoded.
- Named clipboard files: browsers give a clipboard bitmap the placeholder name "image.png"
  (Chromium, WebKitGTK); the page treats `image.png`/`.jpeg`/`.jpg`/`.webp` as no name, so such a
  paste is "Pasted image" and any other file name is kept (criterion 6). (Proposal)
- The Trace-tool rule of criterion 9 applies to every import of images only (picker, drop,
  paste); an import with an SVG still switches to the Select tool.
- The copy uses a second instance of the trace worker (same script and module), so stopping a
  trace never cancels a copy.

## Results (2026-10-28)

- Times (native release, `tests/image_copy_budget.rs`): main thread `image_copy` of 16 Mpx 6.7 ms;
  `encode_png` 2000 x 2000 px photo-like 25.5 ms (10.4 MB PNG), 4000 x 4000 px 102 ms. The wasm
  worker is slower than native; it runs off the main thread.
- Chromium 154 headless (`acceptance_0087_browser.test.ts`): synthetic paste, History "Paste
  image", undo/redo, nothing to paste, text field keeps its paste, Ctrl+C read back (40 x 20 px,
  marker top-left), Ctrl+V from the written clipboard, Ctrl+C with nothing selected.
- WebKitGTK 2.54 (spike harness, Xvfb, X11, `tauri://` secure scheme, the built `dist`):
  synthetic paste imports; Ctrl+C writes ("Image copied."); a real Ctrl+V from the GTK clipboard
  fires `paste` on the canvas with `image.png` and imports it. `navigator.clipboard.read()` outside
  a gesture is refused there (not used by the app).

# Slice 2: dropping image data (criteria 19 to 25)

Implementer, 2026-10-28, same branch (clipboard and drop ship as one PR). Design: `adrs.md`
decision 4.

## Affected crates/modules

- **`curvyo-app`:** `dragDropEnabled: false` in `tauri.conf.json`; the `DragDrop` branch of
  `on_window_event` is deleted; new command `add_dropped_fonts(files: [{name, bytes}])` stages the
  bytes in a fresh temporary folder (one subfolder per file, the base name cleaned) and runs the
  same checks and archive handling as `add_font_files`; `GrantedFontPaths` holds dialog picks only.
- **`curvyo-ui-core`:** the link notice "That drop is a link, not an image. Nothing was downloaded."
- **`curvyo-editor-wasm`:** the drop origin names a nameless file "Dropped image".
- **`frontend`:** `lib/dropData.ts` (what a drag carries: files, a link only, or nothing; never a
  fetch); `useFileDrop` takes link drags (outline, link notice) and refuses them elsewhere;
  `useFontDrop` uses the page's drop events in both builds; Add font sends dropped files' bytes to
  `add_dropped_fonts` on the desktop.

## Tasks

- [x] 13. Session: a dropped file without a name is "Dropped image", one step "Import image"
      (AC 20); the Trace rule of AC 9 holds for a drop (AC 24).
- [x] 14. Link notice in ui-core, quoted in the user page (AC 21, 25).
- [x] 15. `dropData`: files win; `text/uri-list` or `text/html` without files is a link and is
      never fetched (fetch stub); plain text is nothing (AC 20, 21, 23).
- [x] 16. `useFileDrop` with links; window-level refusal for link drags too (AC 21, 23).
- [x] 17. Host: `dragDropEnabled: false`, `add_dropped_fonts` with a test of the staging, the
      `DragDrop` branch removed; `useFontDrop` and Add font on bytes (AC 19, `0075` crit. 24).
- [x] 18. Docs: drop rules (files, image data, links) and the desktop/browser differences (AC 25).
- [x] 19. Chromium: synthetic drops (image file, nameless image, link only with no request,
      mixed); WebKitGTK under Xvfb with an xdotool drag if feasible; font drop still works.

## Not covered by an automated test

- AC 19 on Windows (WebView2) and macOS (WKWebView), X11 against Wayland: manual, in the PR.

## Slice 2 results (2026-10-28)

- Chromium 154 (`acceptance_0087_browser.test.ts`, second test, synthetic `DataTransfer`): named
  image file, nameless image data ("Dropped image"), link only (notice, no request), plain text
  not taken, a link over the rail refused, a mixed drop (image in, "1 file could not be
  imported."), a font dropped on the Fonts tab added.
- WebKitGTK 2.54 (spike harness without Tauri's drop handler, which is what `dragDropEnabled:
  false` gives): synthetic named file, nameless data and link drop behave as in Chromium. A real
  xdotool drag needs a drag source program under Xvfb; none is installed, so it was not run.
- The Fonts tab's drop handlers stop the event, so the page's refusal of drops outside the canvas
  leaves them alone; the refusal also skips a drag a target already took.
- `add_dropped_fonts` takes the bytes as JSON numbers (the ADR's `Vec<{name, bytes}>`); large
  archives cost IPC time. Desktop drops on Windows, macOS and Wayland: manual, in the PR.

## Local file names in a paste or a drop (2026-10-29)

Customer on PR #218: copy and paste and drag and drop from a file manager reach the desktop page
only as file names (`text/uri-list`, `x-special/gnome-copied-files`, `file://` URIs or paths) and
are refused as links. Lead decision 2026-10-29 (Proposal): the desktop host resolves and reads
them; criteria 6 and 21 are amended.

### Affected crates/modules

- **`curvyo-app`:** `file_uris` (pure: the local paths a text names; percent-decoding, GNOME's
  `copy`/`cut` line, CRLF, `#` comments, Windows drive paths; another scheme or host skipped,
  prose names none); `local_files` (command `read_local_files(texts, from)`: regular files only,
  symlinks resolved, `curvyo-image-core::MAX_FILE_BYTES` checked before a read, 20 files read, the
  rest named); `native_file_lists` (Linux: the GTK clipboard's file lists for a paste, the uri list
  of the drag over the webview for a drop, because WebKitGTK hands the page an empty
  `text/uri-list` and no File). `curvyo-image-core` becomes a dependency on every OS (no new
  package).
- **`curvyo-editor-wasm`:** origin `PastedFiles` ("paste-files": paste placement, "Paste image" for
  one image, SVG imported); `import_add_too_large` (the limit's notice from the length alone).
- **`frontend`:** `lib/localFiles.ts` (the texts handed over, the host's answer as import files),
  `platform/localFiles.ts` (the command, none in the browser); `useImageClipboard` asks the host when
  a paste brings no image; `useFileDrop` asks it for a link drop (and, on the desktop, plain text);
  `importFiles` takes a file over the limit by its length.

### Tasks

- [x] 20. Test first: `file_uris` parsing (AC 6, 21).
- [x] 21. Test first: `local_files` checks (folder, device, missing, limit before a read, symlinks,
      20 files) (AC 6, 21).
- [x] 22. Session: `PastedFiles` and `import_add_too_large`, tests (AC 6, 8).
- [x] 23. Page: paste and drop ask the host; unit tests of the texts and the answer (AC 6, 21).
- [x] 24. `native_file_lists` for WebKitGTK (AC 6, 21).
- [x] 25. Chromium with a faked host; the real app under Xvfb with WebKitGTK (AC 6, 21).
- [x] 26. Spec criteria 6 and 21, user page.
- [x] 27. Architect review 2026-10-29: the command takes only `from` (paste or drop), never a
      text or path from the page; the host reads the GTK clipboard and the dropped drag's list
      (given once, within 5 s of the drop, forgotten when the next drag starts); the file is
      checked on the opened handle; the reads run in `spawn_blocking`. Windows and macOS read no
      text-only file names (their webviews hand over files). JSON byte arrays noted as debt.

### Results (2026-10-29)

- WebKitGTK 2.54 (spike harness, Xvfb, X11): a paste of files put on the GTK clipboard as
  `x-special/gnome-copied-files`, `text/uri-list` and text gives the page the type
  `text/uri-list` with an empty string and no File; a real XDND drag of a `text/uri-list` gives
  the page `text/uri-list` (empty) and `text/html` (an `<a>` with the URI as text), no File.
- The built desktop app under Xvfb (WebKitGTK): Ctrl+V of two copied files imports both, named by
  their files; a clipboard with only `text/uri-list` imports the file; a clipboard with only a web
  address says "Nothing to paste"; a real GTK drag of a `text/uri-list` onto the canvas imports the
  file at the drop point.
- Chromium 154 with a faked desktop host (`acceptance_0087_browser.test.ts`, third test): a paste
  of a uri list and GNOME's list (two files, one step), a paste with no readable text (host reads
  its clipboard), a plain path pasted ("Paste image"), a web address ("Nothing to paste"), a
  dropped `file://` list and a dropped plain path, a file over the limit ("File too large: 140
  MB."), a web link dropped (link notice, no request).
- Windows (WebView2) and macOS (WKWebView): not tested here; their webviews are expected to hand
  over File objects for file-manager copies and drops, and the text route covers what they expose
  as text. Reported in the PR.

# Slice 3: a preview in the import dialog (criteria 26 to 29)

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
