# Plan for Bitmap import

Implementer, 2026-10-26, on `story/bitmap-import` from `main` at `d5e3a0be` (format 24, SVG import
of `0024` merged). Design: ADR 0017 (with the spike result) and `adrs.md`.

## Affected crates/modules

- **`curvyo-image-core` (new, ADR 0017 §4):** `probe` (format by content, header size, density,
  ceilings), `decode_working` (decoded-size check, own-layout box downscale, orientation, truncated
  JPEG), `WorkingImage::mip_level`, the limits as constants. Depends on `image` =0.25.10 (png,
  jpeg, webp) only. Fixtures in `curvyo-image-core/tests/fixtures/images/` with the script that
  made them.
- **`curvyo-document-core`:** `AssetHash`, `AssetStore` (in `Document`, shared `Arc` bytes,
  512 MiB cap), `ImageSnapshot` as the fifth `ObjectSnapshot` arm, `image_codec` (`shape =
  "image"`, `image_asset`, `image_placement`), `Document::add_images` (one commit), container
  `assets/<hash>.<ext>` Stored, `document.json`, `format_version` 25, wipe prunes the store,
  transforms/flip/duplicate on the image arm, clone refusal, golden `image_v25.curvyo`.
- **`curvyo-ui-core`:** placement rules (natural size, fit, view centre, clamp, drop point,
  stagger) as a pure function; `FileKind::Image`; the notice texts; the image arm in hit test,
  bounds, select box, transforms, kind refusals of the path commands, Layers row kind.
- **`curvyo-render-core`:** `ImageQuad` list in `DrawList`, an image is one artwork layer of its
  own; node opacity and visibility as for every leaf.
- **`curvyo-editor-wasm`:** `Session::import_files` (routing by content to the SVG import or the
  image import, one commit per batch), the working-image cache keyed by hash, the textured GPU
  pipeline (premultiplied mips, sampler by zoom), the failed-asset frame.
- **`frontend`:** the Import card in the rail, Ctrl+I outside text edit, drop on the canvas, the
  File tab rows removed, notices.

## Tasks

Milestone 1, image-core:
- [ ] 1. Crate, `deny.toml` exception, workspace pin `image` =0.25.10 (ADR 0017 §4).
- [ ] 2. Fixtures and their script; golden expectations (AC 6, 7, 8, 9, 20; Fixtures list).
- [ ] 3. `probe`: format by content, header size, density from `pHYs`/JFIF, 96 dpi rule, header
      and file ceilings (AC 2, 6, 9, 16, 17).
- [ ] 4. `decode_working`: decoded-size cap before decode, own-layout integer box downscale,
      16-bit rounding, orientation after the downscale, animated first frame, truncated JPEG
      flag; hostile files fail fast (AC 7, 8, 18, 19, 20, 21, 42).
- [ ] 5. `mip_level` (2x2 integer box) for the GPU upload (AC 28).

Milestone 2, document-core (format 25):
- [ ] 6. `AssetHash`, `AssetStore` with dedup and the 512 MiB cap (AC 16, 17, 23).
- [ ] 7. `ImageSnapshot`, `image_codec`, the fifth arm everywhere the compiler asks (AC 31, 32,
      36, 37, 38, 40, 41).
- [ ] 8. `Document::add_images`: active layer / Initial Layer, on top, one commit "Import image" /
      "Import images" (AC 2, 5, 13, 14, 15, 45).
- [ ] 9. Container: `assets/` Stored, bounded read, missing asset is not an open failure; JSON;
      format 25; golden and damaged variants (AC 23, 44).
- [ ] 10. Wipe prunes the store (AC 25); duplicate shares the asset; `make_clone` refuses (AC 38).

Milestone 3, ui-core and render-core:
- [ ] 11. Placement function: natural size, fit, view centre, clamp, drop point, stagger (AC 9 to
      12, 4, 5).
- [ ] 12. `FileKind::Image`; notice texts incl. truncated JPEG (AC 2, 10, 17, 18, 21, 24, 26).
- [ ] 13. Image arm in hit test (whole quad), bounds, transforms, refusals of outline commands with
      "Images can't be used with this command. Nothing was changed." (AC 31, 32, 37).
- [ ] 14. `ImageQuad` and the image artwork layer; opacity and hidden (AC 27, 30).

Milestone 4, editor-wasm and frontend:
- [ ] 15. `Session::import_files` routing by content; one commit per batch; selection; notices
      (AC 1 to 5, 10, 18, 24, 26).
- [ ] 16. Working-image cache, textured pipeline with premultiplied mips and the sampler rule;
      failed-asset frame with a cross and the open notice (AC 27, 28, 30).
- [ ] 17. Rail Import card, Ctrl+I outside text edit, drop at the drop point, File tab rows gone
      (AC 1, 3, 4).
- [ ] 18. User page `docs/importing-images.md` and the string test (AC 47); design-system rows.

## Validation

- Golden decoder tests over every fixture (size, sample pixels, notice or failure); hostile files
  with a time and allocation bound; determinism (two decodes equal).
- Container golden `image_v25.curvyo` and its four damaged variants; byte-identical second save.
- Property-style test of the box downscale (constant input stays constant, sums preserved).
- `#[ignore]` budget benches (AC 46), run once in release, numbers in the PR.
- Browser check in Chromium: import PNG/JPEG/WebP, move/rotate/flip, opacity, hide, undo, save and
  reopen, drop; screenshots.
- Full gate of `CLAUDE.md` §7 once at the end.

## Deliberately not in this pass (reported to the lead)

- AC 34 "Original size" and AC 35 Style tab Image block: still **(Proposal)**, not accepted.
- AC 46 eviction of off-screen textures after 30 s (needs a clock from the host): textures are
  dropped when the document no longer shows the image.
- The fuzz run of 10 minutes per format: a proptest mutation smoke over the fixtures stands in
  locally; the timed run is for the PR.
