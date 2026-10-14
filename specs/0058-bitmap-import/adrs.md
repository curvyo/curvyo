# ADRs for Bitmap import

Architect review 2026-10-14, written without the product owner's access to the code and checked
against `base/` at `0c23fff`. The spec needs **one new ADR, [0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md)**
(Proposed, `needs-customer`: new node kind, assets in the file format, one new crate, new
dependencies). It must be Accepted, after its spike, before this spec is Ready.

## Depends on

- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md): the image leaf (`shape = "image"`,
  `image_asset`, `image_placement` over the unit square); bytes in `assets/<sha256>.<ext>`, only the
  hash in the document; `curvyo-image-core` on the pinned `image` crate; premultiplied texture
  upload and the `ImageQuad` list in `DrawList`; the 512 MiB project cap; the format bump.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1 (`assets/`), §9 (refuse a
  newer file): the container writes only three members today, so `pack` and `unpack` are extended.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §9 and §11, Q4: wipe prunes the asset
  store; object wipe does not; undo of an import is a node delete and revival.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2: `visible`, `locked`,
  `opacity` resolve through ancestors for an image like any node. §3/§4: an image is not a clone
  origin.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3, §4, §5: decoding runs in wasm
  (also on desktop), textures belong to `editor-wasm`, `render-core` stays pure; bytes cross the
  boundary once, as a typed array.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §10: images were in the owned
  SVG subset; `0024` takes the requirements of "Coordination with 0024".
- [ADR 0006](../../docs/adr/0006-license.md) and `deny.toml`: every new dependency is on the allow-list;
  the new crate gets the AGPL exception entry.
- `specs/0049-linked-clone` (built, #121): `make_clone` refuses an image; `0056` flip and `0019`/`0047`
  transforms compose into `image_placement`; `0050` layers: an image is a `Leaf`.

## Feature-local decisions

1. **Disagreements with the specification, fixed in it (2026-10-14).**
   - "The container already has room": it does not in code. `pack(&Document)` writes `manifest.json`,
     `document.loro`, `document.json` only; there is no `assets/` and no `thumbnail.png`. ADR 0004
     §1 only names the place. Reworded; the API change is ADR 0017 §2.
   - Criterion 23: BMP is uncompressed, so its entry is Deflated; only PNG, JPEG, GIF, WebP are Stored.
   - Criterion 19: `catch_unwind` does not work on `wasm32-unknown-unknown` (panic aborts the
     module) and a core crate has no clock to enforce 10 s. The panic clause is dropped; the time is
     a test bound; a time-boxed fuzz run becomes part of the PR.
   - Criteria 18 and 44: "byte-identical on every platform" holds for PNG, BMP, GIF and for our
     scaling step; a JPEG or lossy WebP decode may differ by 1 per channel between SIMD and scalar
     paths or wasm and native. Reworded.
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
   - Criterion 16/17: a project total of 512 MiB is added as a Proposal (customer question 2).
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

- 2026-10-14: ADR number 0017 is the next free one on `main`; a branch that takes it first makes
  the other renumber (rule of the ADR index). Format number: 14 if this merges before `0057`, else 15.
