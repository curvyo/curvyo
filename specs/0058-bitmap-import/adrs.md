# ADRs for Bitmap import

Architect review 2026-10-14, written without the product owner's access to the code and checked
against `base/` at `0c23fff`; rechecked against the code on 2026-10-26 (format 24, after the SVG
import #205), see the dated notes. The spec needs **one new ADR, [0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md)**
(Accepted 2026-10-16 by the customer for the `image` crate and the node/asset design; the spike of
2026-10-26 confirmed the numbers with three corrections, ADR 0017 "Spike result").

## Depends on

- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md): the image leaf (`shape = "image"`,
  `image_asset`, `image_placement` over the unit square); bytes in `assets/<sha256>.<ext>`, only the
  hash in the document; `curvyo-image-core` on `image` =0.25.10 (features `png`, `jpeg`, `webp`, no default features, no `kamadak-exif`, no TIFF); premultiplied texture
  upload and the `ImageQuad` list in `DrawList`; the 512 MiB project cap; the format bump to 25.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1 (`assets/`), §9 (refuse a
  newer file): the container still writes only three members (`pack(&Document, app_version)`), so
  `pack` and `unpack` are extended. Fonts are not embedded yet; images are the first blob kind.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §9 and §11, Q4: wipe prunes the asset
  store; object wipe does not; undo of an import is a node delete and revival.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2: `visible`, `locked`,
  `opacity` resolve through ancestors for an image like any node. §3/§4: an image is not a clone
  origin.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3, §4, §5: decoding runs in wasm
  (also on desktop), textures belong to `editor-wasm`, `render-core` stays pure; bytes cross the
  boundary once, as a typed array.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §10 as amended by
  [ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md) §5 (no passthrough bag, no loss
  report) and §8 (an `<image>` with a `data:` URI maps to this node in a later slice): the built SVG
  import ignores `<image>`; this slice does not change `svg-core`.
- [ADR 0022](../../docs/adr/0022-texts-on-carriers-and-text-clones.md) and
  [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md): an image is neither a text
  carrier nor a guide; both commands refuse it by kind.
- [ADR 0006](../../docs/adr/0006-license.md) and `deny.toml`: every new dependency is on the allow-list;
  the new crate gets the AGPL exception entry.
- `specs/0049-linked-clone` (built, #121): `make_clone` refuses an image; `0056` flip and `0019`/`0047`
  transforms compose into `image_placement`; `0050` layers: an image is a `Leaf`.

## Feature-local decisions

1. **Disagreements with the specification, fixed in it (2026-10-14).**
   - "The container already has room": it does not in code. `pack(&Document)` writes `manifest.json`,
     `document.loro`, `document.json` only; there is no `assets/` and no `thumbnail.png`. ADR 0004
     §1 only names the place. Reworded; the API change is ADR 0017 §2.
   - Criterion 23: PNG, JPEG, WebP are Stored (BMP and GIF were dropped by the customer 2026-10-16).
   - Criterion 19: `catch_unwind` does not work on `wasm32-unknown-unknown` (panic aborts the
     module) and a core crate has no clock to enforce 10 s. The panic clause is dropped; the time is
     a test bound; a time-boxed fuzz run becomes part of the PR.
   - Criteria 18 and 44: "byte-identical on every platform" holds for PNG and for our
     scaling step; a JPEG or lossy WebP decode may differ by 1 per channel between SIMD and scalar
     paths or wasm and native. Reworded (PNG is exact; BMP and GIF are gone).
   - Criterion 25: the asset store is pruned by the document wipe only (ADR 0017 §2); "saving never
     keeps an asset that nothing refers to" would need the oplog scanned on every save.
   - Criterion 38: Linked clone (0049) is built, so the refusal is part of this slice, not of "the
     later of the two".
   - Criterion 46: the desktop app runs the same wasm in the webview, so the native figures are a
     regression guard and not the shipping time; the wasm times in the webview are measured too and
     written in the PR.
   - D5: WebGL2 guarantees only 2048 for the texture side and `gpu.rs` takes the adapter's limits, so
     8192 is a working-image limit, not a texture limit (ADR 0017 §5). "32-bit WASM heap can hold"
     was loose: wasm32 addresses 4 GiB, the browser's limit is lower.
   - Criterion 16/17: a project total of 512 MiB (customer decision IMG-TOTAL = A).
   - Criterion 7, CMYK JPEG: supported only if the pinned decoder converts it; the spike decides the
     fixture's expectation.
2. **Placement is stored over the unit square, not over pixels.** The working copy's size, the
   orientation and a decoder upgrade then never change a saved frame. "Original size" (34) re-reads
   the pixel size and density from the asset with `probe`; with a missing asset it shows a notice.
3. **No pixel size in the document.** Derived from the asset; the Style tab (35) shows it after the
   decode.
4. **Where the placement rules live** (natural size, fit, view centre, clamp, offsets of several
   files): a pure function in `curvyo-ui-core` taking numbers (pixels, density, document size, view
   centre) and returning an `Affine`; the import command in `editor-wasm` calls `probe`, that
   function and `Document::add_image`.
5. **Order of work (milestones on one branch):** `image-core` with fixtures and the spike result;
   `document-core` (node kind, store, container, format, golden); `ui-core` and `render-core`;
   `editor-wasm` and `frontend`. The format and the model are the milestones to review on the
   branch diff.
6. **Open for the plan:** how the clipboard holds objects today (a paste after Open must still find
   the bytes, ADR 0017 §6); whether `export_json` prints image nodes without the hash path
   inventing a new field.

## Dated notes

- 2026-10-16: customer decisions applied. `kamadak-exif` dropped: `image` 0.25 reports orientation for JPEG, WebP, TIFF and (via the `eXIf` chunk) PNG, so criterion 8 now covers all three formats; the density comes from our own `pHYs`/JFIF reads and the EXIF `XResolution` fallback is dropped (criterion 9). The image goes on top of the active layer (criterion 14), so no "insert at bottom" operation is needed in `document-core`; `add_image` appends like every new object. TIFF is in only if the spike finds `image`'s `tiff` feature free; default out.

- 2026-10-14: ADR number 0017 is the next free one on `main`; a branch that takes it first makes
  the other renumber (rule of the ADR index). Format number: 14 if this merges before `0057`, else 15.

- 2026-10-26: **spike done** (ADR 0017 "Spike result"). `image` =0.25.10 on wasm32: +671 KB
  (+221 KB gzip) on a 13.2 MB `editor-wasm`; wasm decode 180 ms for a 12 Mpx JPEG, 120 ms for a
  16 Mpx PNG, 642 ms for a 50 Mpx JPEG; `cargo deny` passes; no C, no new licence. TIFF is
  **out** (+406 KB, five more crates); criteria 1, 6, 23 and 26 drop their TIFF clauses. CMYK JPEG
  is supported (the fixture expects RGB). Three corrections for the plan and the spec:
  1. `max_alloc` is not enforced on the `into_decoder` path: `decode_working` checks
     `total_bytes()` itself before allocating (a 900 Mpx PNG bomb aborted the wasm module in the
     spike). The 50 Mpx header ceiling of criterion 17 stays the first check.
  2. No full-size RGBA8: the box downscale reads the decoder's own layout. Criterion 19's memory
     cap becomes "decoder buffer in its own layout + working image + 64 MiB" (a 50 Mpx RGBA16 PNG
     needs 400 MB of decoder buffer and is valid).
  3. A truncated JPEG decodes to a partial picture, grey below the cut, without an error. PNG and
     WebP fail as criterion 20 says; for JPEG see the customer question JPEG-TRUNC below.
- 2026-10-26: **today's code** (replaces the format numbers of 2026-10-14).
  - Format: `CURRENT_FORMAT_VERSION` is 24; this slice takes the next free number at merge, 25
    unless another merge takes it first. Golden `image_v25.curvyo` and its damaged variants.
  - Image node: `ObjectSnapshot` has `Path`, `Primitive`, `Clone`, `Text`; `Image(ImageSnapshot)`
    is the fifth arm, unboxed (a hash and six numbers are no larger than the others).
  - Asset store: as ADR 0017 §2; `pack` writes `assets/<hash>.<ext>` Stored after the three
    members. There is **no clipboard yet** (no copy and paste in the code), so open item 6 is moot:
    Duplicate shares the asset; the clipboard slice carries the `Arc` when it comes.
  - History wipe: `Document::wipe_history` (`history/wipe.rs`) prunes the store to the hashes alive
    in the tree after its re-import; `wipe_object_history` does not prune. Undo of an import is the
    existing delete and revival.
  - Render: `DrawList` is cut into artwork layers painted with one depth value each (each object's
    fill and stroke, `glyphs.rs`). An image is **one artwork layer of its own**; `ImageQuad` names
    that layer's index instead of `at_vertex`, and the host draws it with the textured pipeline at
    that depth, in list order (a translucent shape above an image needs the image drawn first).
    `render-core` stays pixel-free.
  - Export: no SVG or machine export exists in the code, so nothing exports an image in this
    slice. Every later output that matches `ObjectSnapshot` decides the `Image` arm; machine
    output skips it (it has no path).
- 2026-10-26: **one Import tool (customer change of 2026-10-27).** An "Import" tool in the tool
  rail replaces "Import SVG…" and "Import image…" in the File tab; it opens one picker (SVG and
  images) and dispatches by content. In the code: `ui-core`'s `FileKind` (`svg_import_text.rs`)
  gains `Image` (PNG, JPEG, WebP magic bytes), and `editor-wasm` gets one entry
  `import_file(bytes, name)` that calls the SVG import or the image import; each stays its own
  one-commit path with its own History name. A project file picked there keeps the 0024 refusal.
  Criteria 1 and 3 to 5 (button, Ctrl+I, drop, several files) go back to the product owner and UX
  for the tool; the placement, limit and model criteria are unchanged.
- 2026-10-26: **what the later SVG `<image>`/`<text>` slice needs from this one** (ADR 0018 §8).
  - From `0058`: `image-core::probe` (format by content, ceilings); an asset insert on `Document`
    that can run inside `insert_drawing`'s one commit (dedup, 512 MiB total); the image arm of
    the snapshot tree that `insert_drawing` takes. `add_image` is built on that insert, not beside it.
  - `svg-core` emits an image leaf: decoded bytes, `x`, `y`, `width`, `height`,
    `preserveAspectRatio`, matrix. `ui-core` maps it to `placement`; `meet` needs the pixel size
    from `probe`; `slice` needs a crop the node does not have (decided there; default: as `meet`).
  - Only `data:image/png|jpeg|webp;base64,` URIs are read (content wins over the MIME type);
    base64 needs `base64` 0.22 (already in `Cargo.lock`) or about 40 lines of our own. External
    hrefs (paths, `http:`) are never fetched (ADR 0018 §6) and are ignored silently (§5 (b)), so
    an Inkscape file with linked images imports without the picture: customer question for that
    slice, default: ignore.
  - `MAX_ATTRIBUTE_BYTES` (16 MiB) skips a `data:` URI of more than about 12 MiB of image; the
    50 MiB file limit of the SVG import stays. Raising the limit for `href` alone is that slice's
    choice.
  - Image decoding counts against the SVG import's CPU-time budget (`d5e3a0be`); an image over a
    ceiling inside an SVG is skipped silently, not refused with a notice.
  - `<text>` needs nothing from this slice.
- 2026-10-26: **customer question JPEG-TRUNC.** A JPEG cut off in transfer: A) import the part
  that decodes, grey below the cut (as browsers show it; no extra code); B) refuse it as damaged
  (our own end-marker check, about 30 lines, which still misses some cases). Recommendation and
  default: **A**; criterion 20 is reworded to it.
