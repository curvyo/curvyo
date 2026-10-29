# ADRs for Image clipboard, drag and drop, and a preview in the import dialog

Architect, 2026-10-28, with the customer answers of the same day (Frage 65 A, Frage 67 B, Frage 68 A,
Frage 69 B: "if that is possible in the browser, else a deviation for native"). **No new ADR, no new
crate, no new package in `Cargo.lock`, no `format_version` change, no trait, no generic.** Spikes of
2026-10-28 (throwaway crate in the scratchpad, nothing committed; one `Cargo.toml` edit for `cargo
deny`, reverted) answer the open platform questions below. Nothing here is `needs-customer` beyond
Frage 69, which the customer has answered; decision 6 is how that answer is built.

## Depends on

- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §1, §2, §6: the image leaf, the
  asset store, `WorkingImage`; §"Clipboard" (carry the `Arc`) applies to the future object clipboard,
  not to this story (decision 7).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3, §6: the editor runs as wasm
  in the webview on both builds; the host stays a narrow command interface.
- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) §5: the one Web Worker that
  instantiates `curvyo-editor-wasm`; the copy's PNG encode runs there (decision 2).
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): `curvyo-app → curvyo-image-core` is an
  app-to-core edge, allowed; no new crate. The SVG preview adds the Linux-only app-to-core edges
  `curvyo-app → curvyo-svg-core`, `curvyo-ui-core`, `curvyo-text-core` and `curvyo-document-core`
  (reading, font matching and text layout as Import SVG does them), allowed likewise.
- `specs/0058-bitmap-import/adrs.md` (2026-10-26 notes): `editor-wasm`'s one `import_file(bytes, name)`
  door, `FileKind::Image` by magic bytes, `probe` before decode, placement in `ui-core`.

## Spike results (2026-10-28, Linux build host, Xvfb, X11)

| Question | Result |
|---|---|
| WebKitGTK 2.54 (`webkit2gtk-4.1`, what Tauri links): `tauri://` origin with `register_uri_scheme_as_secure` (as `wry` 0.57 does) | `isSecureContext` true; `ClipboardItem`, `navigator.clipboard.write` and `read` exist |
| Ctrl+V with a GTK clipboard image, focus on a non-editable `<canvas tabindex=0>` | `paste` fires on the canvas; `clipboardData.files` = one `image/png` File; no prompt |
| Same with a hidden `<textarea>` focused in `keydown` (fallback technique) | also works (`paste` on the textarea, same File) |
| `navigator.clipboard.write([new ClipboardItem({"image/png": promise})])` from a click | ok; GTK sees target `image/png`, `wait_for_image` gives the 40 x 20 px picture |
| Tauri's `enable_clipboard_access` (`javascript_can_access_clipboard`) | not needed for any of the above (same results off and on) |
| `rfd` 0.16 (under `tauri-plugin-dialog` 2.8.1, feature `gtk3`, the default we use) | no preview hook anywhere in its source |
| GTK3 `FileChooserDialog` built with `gtk` 0.18 + `set_preview_widget` / `connect_update_preview` | works: preview pane on the right of the native dialog (as GIMP and Inkscape do) |
| gdk-pixbuf as the preview decoder | rejected: it decoded the `0058` iCCP bomb fixture, i.e. it bypasses our ceilings (crit. 28) |
| `cargo deny check` with `gtk = "0.18"` (Linux only) and `curvyo-image-core` added to `curvyo-app` | advisories, bans, licences, sources ok; `Cargo.lock` gains 2 edge lines, **no new package** |

Not testable here: Wayland, WebView2, WKWebView, real file-manager drops. They are the manual
per-platform checks the spec already asks for in the PRs (criteria 6, 10, 11, 19).

## Feature-local decisions

1. **Paste reads the page's `paste` event in both builds; no clipboard plugin.** A document-level
   `paste` listener, active only when the canvas (or nothing editable) has focus, takes the first
   `clipboardData` file item whose bytes `FileKind` identifies as an image (content wins over the MIME
   label; image wins over text, crit. 4; SVG and others give crit. 5). Chromium/WebView2, Firefox and
   WebKitGTK deliver screenshots as `image/png` Files there (spike for WebKitGTK); no permission
   prompt anywhere. `navigator.clipboard.read()` is not used (Safari and WKWebView show a paste
   prompt for foreign content). If the macOS check (crit. 10) shows WKWebView does not fire `paste`
   with a non-editable focus, the fix is the spike-proven hidden-textarea catcher, in the same slice.
   (Proposal)
   - Rejected: `tauri-plugin-clipboard-manager` / `arboard`. A second code path for desktop only, RGBA
     over IPC (64 MB at 16 Mpx), its own X11/Wayland clipboard owner beside GTK's, and a new
     dependency tree, for something the webview already does.
2. **Copy writes `image/png` with `navigator.clipboard.write` in both builds.** The keydown handler
   creates the `ClipboardItem` synchronously inside the gesture with a `Promise<Blob>` value (Safari's
   rule; Chromium and Firefox 127+ accept it), and the promise resolves when the PNG is ready. The PNG
   is encoded by a new `curvyo-image-core::encode_png(&WorkingImage) -> Vec<u8>` (`image`'s `png`
   feature already pinned has the encoder; fast compression, no metadata chunks: crit. 12) inside the
   existing trace worker (ADR 0023 §5), so the main thread only copies the RGBA out (crit. 14). Not
   `OffscreenCanvas.convertToBlob`: premultiplied canvas storage changes the RGB of translucent pixels,
   so the copy would not be the stored pixels. The browser without `ClipboardItem` gets crit. 15's
   notice; the secure-context rule holds on `tauri://localhost` (spike), `http://tauri.localhost`
   (Chromium treats `*.localhost` as secure) and our HTTPS browser build. (Proposal)
3. **One import door, three origins.** `import_file(bytes, name)` gains an origin argument (`picker`,
   `drop`, `paste`) that only selects the History name and the default object name ("Pasted image",
   "Dropped image"); decoder, limits, notices and the one commit are the `0058` path unchanged. No new
   command. **Placement (Frage 67 B, crit. 2):** the canvas hook that already receives pointer moves keeps
   the last position in canvas CSS pixels and clears it on `pointerleave`. A paste with a position goes
   through the drop placement that exists (`import_begin` with a point: the same screen-to-document
   mapping as a drop, centre at the point, clamp of `0058` crit. 12); with no position (pointer over a
   panel or outside, or never over the canvas) `import_begin` without a point (view centre). Nothing
   new in `ui-core`.
4. **Drop: `dragDropEnabled: false`; HTML5 drops in both builds.** Reading `wry` 0.57: Tauri's drop
   handler always returns `true`, which on WebKitGTK swallows file (`text/uri-list`) drops, on WKWebView
   consumes every drop, and on WebView2 sets `SetAllowExternalDrop(false)`. So in today's desktop build
   `useFileDrop` (`0058` crit. 4) most likely never receives a file drop, and image data without a
   file (crit. 20) cannot reach the page on Windows or macOS. With the handler off, all three webviews
   deliver HTML5 drops with Files and bytes, and `useFileDrop` is the one code path. Cost: the font drop
   (`useFontDrop`, `0075` crit. 24) loses the host's `DragDrop` paths, so it moves to HTML5 Files as in
   the browser build, and the host gets `add_dropped_fonts(files: Vec<{name, bytes}>)` that runs the
   same checks and archive handling as `add_font_files`; the `GrantedFontPaths` list then holds dialog
   picks only, and the `on_window_event` drop branch is deleted. Links are never fetched (Frage 68 A).
   (Proposal)
   - Rejected: keep the host handler and route its paths to the importer (as `useFontDrop` does).
     Two code paths, a new path grant for import, and crit. 20 impossible on Windows and macOS.
5. **The copy writes the working image's pixels** (Frage 65 A): upright, straight alpha, sRGB, no
   transform. A pasted-back JPEG or WebP asset is a new PNG asset (the bytes differ); noted for the user
   page, not deduplicated.
6. **Import dialog preview (Frage 69 B: own preview where possible, platform deviation accepted).**
   - **Linux: our own GTK3 dialog with a preview pane** for Import only, like GIMP and Inkscape. A
     `#[cfg(target_os = "linux")]` module in `curvyo-app` builds `gtk::FileChooserDialog` (the same
     filters as today), sets an `Image` as preview widget and an "Enable preview" check as extra widget,
     and on `update-preview` decodes the highlighted file **off the GTK thread** with
     `curvyo-image-core::thumbnail(bytes, 256)` (new: `probe` and ceilings first, then the bounded
     decode, longest side 256 px, aspect kept; crit. 26 and 28), drops stale results, and shows
     "W x H px" or "Too large" or the name only. It runs on the GTK main thread through
     `run_on_main_thread` and answers the command through a channel. Direct dependency `gtk = "0.18"`,
     Linux only, MIT, already in the tree under Tauri (spike: `cargo deny` green, no new package); no
     `unsafe` (`close()`, not `destroy()`). `pick_project` and Add font keep `tauri-plugin-dialog`.
   - **Windows and macOS: the OS dialog as today, a recorded deviation from crit. 26.** `IFileOpenDialog`
     has the shell preview pane (switched on once with Alt+P, then remembered by Windows) and
     thumbnail views; `NSOpenPanel` shows Quick Look with the image in column and gallery view. Our own
     pane is possible only by FFI the plugin does not expose: on macOS an `accessoryView` with an
     `NSImageView` and a panel delegate (`objc2`), on Windows `IFileDialogCustomize` can add a text
     line for the pixel size but no image (`windows` COM). Both need `unsafe`, so an ADR (CLAUDE.md §5),
     and a second and third dialog code path, for platforms the customer does not work on. Proposal:
     deviation now; the macOS accessory view is the follow-up if the customer wants it there. The PR
     shows a screenshot of each (crit. 26).
   - **Browser: the browser's picker, no preview** (crit. 27). `<input type=file>` has no preview hook;
     `showOpenFilePicker` is Chromium-only and has none either; an own browser would need
     `showDirectoryPicker` (Chromium-only, a permission prompt per session, folder read access) plus
     a file-browser UI and would still read only Chromium. Recorded deviation (crit. 27): the
     browser's picker, no preview; the user page says so.
   - Rejected for all platforms: the xdg portal (`org.freedesktop.portal.FileChooser` has no preview
     option; GNOME's portal uses GTK4, which removed preview widgets); an own in-app file browser on the
     desktop (B in its full form: folder listing, capability for folder reads, a UI to maintain) when
     the native GTK3 dialog already takes our pane; a preview strip after the pick (C: does not help
     choosing). Consequence: the Linux dialog is bound to GTK3 and is rewritten when Tauri moves to
     GTK4/WebKitGTK 6; a Flatpak build would need the portal and lose the pane. Both go to
     `docs/technical-debt.md` with slice 3.
7. **Future object clipboard (crit. 17).** When it comes, it writes its own MIME type next to the
   `image/png` of a single image, carries the asset `Arc` (ADR 0017), and decides what Ctrl+V does when
   both are present; it is a document-model story with its own ADR. Nothing is prepared now.

## Spec criteria that should change (for the product owner)

- **10:** drop "else the host reads the clipboard through a command (architect: Tauri clipboard plugin,
  new dependency, ADR)": both builds use the `paste` event (decision 1); the fallback is the
  hidden-textarea catcher, not a plugin.
- **19:** replace the "for example" with the decision: slice 2 sets `dragDropEnabled: false` and moves
  the desktop font drop to bytes (`add_dropped_fonts`); the PR re-checks `0075` crit. 24 on the desktop.
- **26:** "on Linux, Windows and macOS ... needs no setting that the maker must switch on" holds on
  Linux only; add the Windows and macOS deviation of decision 6 (OS preview, Alt+P once on Windows, no
  pixel-size line guaranteed), or the customer asks for the macOS accessory view as a follow-up.
- 2, 27, 28: consistent with decisions 3 and 6; no edit.

## Format

None.

## Questions for the customer

None open. Frage 65 A, 67 B, 68 A and 69 B are applied above; the Windows/macOS deviation of decision 6
is shown at the slice 3 demo.
