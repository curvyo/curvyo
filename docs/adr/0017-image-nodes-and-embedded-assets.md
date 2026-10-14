# ADR 0017: Image nodes, embedded assets and bitmap decoding

**Status:** Proposed 2026-10-14, `needs-customer` (document model: a new node kind; file format:
`assets/` in the container; new crate and dependencies). Extends [ADR 0002](0002-document-model-units-and-svg-round-trip.md)
§5 and §10 (images were named in the owned SVG subset, never modelled), [ADR 0004](0004-persistence-and-cross-machine-sync.md)
§1 (`assets/` in the container) and §9 (format refusal), [ADR 0014](0014-history-undo-and-branches.md)
§9 (wipe), [ADR 0011](0011-workspace-and-crate-layout.md) (one new crate). Asked by
`specs/0058-bitmap-import`. The decoder choice below is a design until the spike in "Before
Accepted" has run.

## Context

The customer wants a PNG, JPEG or BMP in the drawing, first as the input of the Trace tool
(`0025`). Three things are hard to change later: the shape of the node in the document, where the
picture bytes live in the file, and which code decodes hostile bytes inside a wasm module that
cannot catch a panic. Today the code has none of the three. The container writes `manifest.json`,
`document.loro` and `document.json` only, and `pack` takes just a `Document`. `ObjectSnapshot` has
`Path`, `Primitive` and `Clone`. No image decoder is in the lock file; `png` and `sha2` are, as
transitive dependencies. The editor core runs as wasm in the webview on desktop too (ADR 0001 §3),
so a wasm figure is the shipping figure.

## Decision

### 1. An image is a leaf with its own affine map (as a clone is)

A node with `shape = "image"` and two registers: `image_asset` (the 64-character lowercase hex
SHA-256 of the file's bytes, written once) and `image_placement` (six finite numbers, the
`Affine` that maps the **unit square** (0,0)-(1,1), y down, to document millimetres; the
square's top-left is the picture's top-left corner). `name`, `visible`, `locked`, `opacity` are
the registers of ADR 0015 §2. No style keys, no outline. Read model: `ObjectSnapshot::Image(ImageSnapshot
{ id, asset, placement })`. The unit square is independent of the pixel size, so a working copy
at another resolution (§4) and a later re-decode never change the stored frame. The pixel-to-mm
map of the trace hand-off is `placement` followed by a scale of `1/w, 1/h` of the working image.
Validation on open: placement entries finite and within ±1e7 (as the clone matrix); an unreadable
`image_asset` is damage; a singular placement is allowed (a flip through zero, `0056`) and draws
nothing.

An image has no `PathSnapshot`. Everything that needs a path (Boolean, Combine, Cut, Offset, to
path, any machine output) must match on the kind, so the compiler forces a decision at each of the
roughly 120 places that already match `ObjectSnapshot::Clone`; the bounding box and hit test use
a `frame_outline()` (the mapped square) that no output code can reach by accident.

| Option | Why it lost or won |
|---|---|
| **A. New leaf kind with a full affine (chosen)** | Skew, rotation, flip and group transforms compose into the matrix with no decomposition; precedent is the clone (ADR 0015). The cost is an arm in every exhaustive match. |
| B. A rectangle primitive with an "image" attribute | Hit, bounds and nudge come free, but a skew converts a primitive to a path (`0047`), and Boolean, Combine and every output would accept the picture frame as geometry: a laser would cut the border of the photo. Unsafe by default for a manufacturing tool. |
| C. A fourth `NodeKind` beside `Leaf` | Selection, transform, group box, Ctrl+A and ordering all iterate leaves; they would need the same arms with no shared type. |

### 2. Bytes live in the container, the document holds the hash

`Document` owns an `AssetStore` (`BTreeMap<AssetHash, Arc<[u8]>>`, sorted, so a save is
deterministic). `pack(&Document)` keeps its signature and writes each asset as
`assets/<hash>.<ext>` after the three existing members, in hash order, **Stored** for PNG, JPEG,
GIF and WebP and **Deflated** for BMP (BMP is not compressed). `unpack` reads every `assets/*`
member whose name is `<64 hex>.<ext>`, each read bounded by the file ceiling of `0058` (a bomb in
the zip is cut off), and does not hash them on open. An entry that is missing, oversized, unreadable
or whose bytes do not hash to its name is a **missing asset**, not an open failure: the object
keeps its frame and `0058` criterion 30 draws it. The hash is checked when the picture is decoded.
The store is shared (`Arc`), never copied, by Duplicate, copy and paste, undo and redo, and the
history preview.

- **Dedup and determinism** follow from content addressing. Import of the same bytes adds nothing.
- **History cost:** the oplog holds 64 characters and 6 numbers per image step, never the picture.
  Undo of an import deletes the node (ADR 0014 §11 revival works unchanged); the bytes stay in the
  store, so redo and "undo of the delete" decode nothing from disk.
- **Pruning:** only the document wipe (ADR 0014 §9) prunes: after the shallow re-import the store
  keeps the hashes of nodes alive in the tree. Object wipe does not (ADR 0014 Q4: the data stays).
  Save never prunes. So a file may hold bytes that only history refers to, until a wipe;
  `0058` criterion 25 is worded to that.
- **Total size:** the whole store is in memory (wasm32, no threads) and in one file. A project
  holds at most **512 MiB** of assets (constant `MAX_PROJECT_ASSET_BYTES`); the import that would
  pass it is refused with a notice. Customer question 2.
- **Sync (not built):** the Loro updates carry only the hash, so a peer without the bytes already
  has a defined state (missing asset). Blobs would travel by hash; for ADR 0008 the server-visible
  blob id must then be a keyed value, not the plain hash, which would let a server confirm a known
  file. Nothing here blocks that.

| Option | Why it lost or won |
|---|---|
| **A. Bytes in the container, hash in Loro (chosen)** | Small oplog and snapshots, wipe and sync stay cheap, dedup free. Needs a second place to save and a GC rule (above). |
| B. Bytes as a Loro binary value | One place, but every shallow snapshot, relay update and history step carries up to 100 MB, and a deleted image can never leave the oplog short of a wipe. |
| C. Link to a file on disk | Rejected by `0058` D3 (browser, move, encryption). |

### 3. Format

`format_version` is the next free number at merge (14 if this merges before `0057`, else 15; the
rule of the constant's doc comment). Older builds refuse the file by version ("saved by a newer
version"): they would show nothing where the picture is. No migration: a file without images is
unchanged and not rewritten. `document.json` lists image nodes (hash, placement) and no bytes.
Golden: `image_v<N>.curvyo` and its four damaged variants (`0058` criterion 44).

### 4. Decoding: one new crate, `curvyo-image-core`

Pure, `#![forbid(unsafe_code)]`, wasm32, bytes in and pixels out, depends on none of our crates.
`curvyo-ui-core` and `curvyo-editor-wasm` depend on it; so will `0025`. `curvyo-document-core`
does not (it needs only `sha2` for the hash and the store). It offers:

- `probe(bytes) -> ImageInfo`: format by content, pixel size, density as a `PixelDensity` newtype
  (PNG `pHYs`, JFIF, EXIF, BMP header; about 150 lines of fixed-offset reads plus `kamadak-exif`),
  EXIF orientation. Header only, no pixel allocation; the size ceilings are checked here, before
  any decoder runs.
- `decode_working(bytes, &Limits) -> WorkingImage`: RGBA8, **straight** alpha, orientation applied,
  sRGB assumed, ICC and gamma ignored, area-averaging integer box downscale of our own (about 40
  lines; `fast_image_resize` was rejected: SIMD and float paths, `unsafe`, results that differ by CPU).
- `WorkingImage::mip_level(n)`: the same 2x2 integer box, for the GPU upload (§5).

**Crate set (chosen): `image` 0.25.x, `default-features = false`, features `png`, `jpeg`, `bmp`
(plus `gif`, `webp` if customer question 1 of `0058` stays at A), pinned with `=`** as `loro` and
`i_overlay` are, because a decoder upgrade can change lossy output. It brings `png`, `fdeflate`,
`zune-jpeg`, `image-webp`, `gif`, `weezl` and the BMP decoder in the same tree, one `Limits`
(`max_image_width/height`, `max_alloc`) that every decoder honours before allocating, and one
fuzz target. Plus `kamadak-exif` (BSD-2-Clause) and `sha2` (document-core). Licences, from
memory, to be confirmed by `cargo deny check`: all MIT OR Apache-2.0, `zune-*` MIT OR Apache-2.0
OR Zlib, `kamadak-exif` BSD-2-Clause; every one is on the `deny.toml` allow-list. `deny.toml` gets
the AGPL exception for the new crate, as for the others.

**Rejected:** separate `png` + `zune-jpeg` + `image-webp` + `gif` + our own BMP: six APIs and six
limit mechanisms, and a hand-written BMP RLE and V4/V5 decoder (about 300 lines of hostile-input
code we would own; `tinybmp` targets embedded-graphics and does not read RLE). A C library
(libjpeg-turbo, libpng): no C in the build (wasm, ADR 0006 audits).

**`unsafe` in dependencies:** our crates forbid it (CLAUDE.md §5); a dependency may contain it, as
`i_overlay` does (`docs/technical-debt.md`). The spike counts it with `cargo geiger` for the
resolved tree and records the number; a build script that compiles C is a stop. **Determinism:**
PNG, BMP and GIF decode exactly everywhere; JPEG and lossy WebP may differ by 1 per channel
between a SIMD and a scalar path or between wasm and native. Our scaling and mip steps are exact
for equal input. `0058` criteria 18 and 44 are worded to that.

**CMYK/YCCK JPEG:** supported only if the pinned decoder converts it; otherwise the file is
"not supported". No own conversion in this slice.

**Hostile input on wasm:** a panic in wasm aborts the module, so `catch_unwind` is not a defence
(`0058` criterion 19 is amended). The defences are the pre-checks of `probe`, `Limits`, fallible
allocation of our own buffers, and fuzzing: a time-boxed run (10 minutes per format) of the
decoder boundary is a precondition of the PR and its result is written in the PR description.
There is no runtime timeout (no clock, no threads in a core crate); the 10 s of `0058` is a test
bound.

### 5. Working copy, texture and render

The working image is a cache in the editor session, keyed by hash, not in the `Document`.
`editor-wasm` decodes at import and on first draw, one image per task so the UI can breathe, and
releases it by the eviction rule of `0058` criterion 46. Decoding blocks the webview's main
thread; accepted, recorded in `docs/technical-debt.md`.

`render-core` stays pixel-free. `DrawList` gets `images: Vec<ImageQuad>`; a quad carries the asset
hash, four view-space corners, the effective opacity, the screen pixels per image pixel (for the
sampler choice of `0058` criterion 28) and `at_vertex`, the index in the triangle list where it
is drawn, so images interleave with shapes in z-order. `editor-wasm` (ADR 0001 §4: GPU
submission) owns textures. It uploads the CPU mip chain (wgpu does not generate mips) as
**premultiplied** alpha, because filtering straight alpha leaves dark fringes at transparent
edges; the trace hand-off stays straight. The device's `max_texture_dimension_2d` can be below the
8192 of the working limit (`gpu.rs` takes the downlevel adapter's limits): the host then skips the
top mip levels. The working image does not depend on the device.

### 6. Interplay

- **Clones (0049, built; 0057):** `make_clone` returns a refusal for an image and the UI does not
  offer it; a `clone_link` whose origin is an image reads as an orphan by the existing wrong-kind
  rule. Duplicate shares the asset.
- **Layers, groups, order (0050, 0023):** an image is a `Leaf`; all node registers apply.
- **Transforms (0019, 0047, 0056):** the writers compose the group or flip map into `placement`;
  there is nothing to hold back.
- **SVG (0024):** images are now modelled, so they leave the passthrough bag. Export writes the
  asset bytes as a data URI and `placement` as the transform; a path or URL `href` is a listed loss.
- **Trace hand-off (0025):** `WorkingImage` plus `ImageSnapshot::pixel_map(&WorkingImage) -> Affine`
  (`placement` then scale `1/w, 1/h`) plus the hash. `0025` depends on `image-core` and
  `document-core`, not on the editor.
- **Clipboard:** copy and paste carries the asset's `Arc`, not the hash alone, or a paste after
  Open would show a missing asset. The plan checks how the clipboard is held today.

## Before Accepted

A throwaway spike on `spike/image-decoder` answers, and this ADR records, with numbers: wasm32
build with the chosen features and the size added to the module; decode time of a 12 Mpx JPEG and
a 16 Mpx PNG in the webview (not native); peak memory for a 50 Mpx JPEG; the `cargo geiger` count;
what the pinned decoder does with CMYK, BMP RLE and V5, and a PNG bomb; `cargo deny check`.

## Consequences

- One new crate, one new node kind, one new container member family, three new direct
  dependencies (`image`, `kamadak-exif`, `sha2`, plus their trees).
- Every exhaustive match on `ObjectSnapshot` grows an arm; the compiler finds them.
- A saved project can hold up to 512 MiB of pictures and loads them all into memory on open.
- The first slice that adds a blob kind (fonts, ADR 0004 §1) reuses the store and the container
  rule unchanged.

## Questions for the customer

1. **Where do the picture bytes live?** *A (default, recommended):* in the container beside the
   document, hash in the document (above). *B:* inside the document as binary values: simpler
   save, but history, wipe and later sync carry every picture.
2. **How much may one project hold in pictures?** *A (default, recommended):* 512 MiB in total,
   further imports refused with a notice. *B:* no cap (a project of several GB would fail to
   open in the browser). *C:* 256 MiB.
