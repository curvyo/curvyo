# ADR 0077: Stored AI results as derived PNG assets in the project file: `assets/derived/`, the result record on the step, input hash, limits and pruning at save

**Status:** Proposed (architect, 2026-11-19), `needs-customer` (file format and persistence, `CLAUDE.md` §3). The customer decided the storage (Frage 275 A) and asked for generous limits (64 MB per result, 512 MB per document, "it may be a generous limit"); confirmed here with the rules below. Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 18, 23, 24, 48, 59, 66. Extends [ADR 0017](0017-image-nodes-and-embedded-assets.md) §2 (the asset store and container); the step register is [ADR 0072](0072-image-adjustments-on-the-image-object.md) §1; the models [ADR 0075](0075-on-device-image-models-runtime-and-crate.md).

**In plain words (for the customer).** What an AI tool produced is saved in the project as a picture next to the original. Burning reads that saved picture, so the dots stay the same on every computer and offline. If you change an earlier step, the result is marked out of date but still used until you run it again. Results nothing refers to any more are removed when you save.

## Context

The model output differs slightly between CPUs (ADR 0075 §8); the dots must not (spec criterion 17). So the output is stored once and treated as data. It is reproducible by running the model again, which original pictures are not, so it may be pruned more eagerly than ADR 0017 prunes originals (only at a document wipe).

## Options

| Question | Chosen | Rejected and why |
|---|---|---|
| Where the bytes go | **The existing `AssetStore`, content-addressed, as a second class `Derived`, written to `assets/derived/<sha256>.png`** | Inside the register (a Loro binary value): every snapshot, relay update and history step would carry megabytes (ADR 0017 option B). A sidecar file: the browser and the workspace sync know only the container. |
| What is stored | **The raw model output** (the mask at model resolution, the upscaled or denoised pixels), the step's other fields applied afterwards in integers | The finished, blended result: every Strength or Threshold change would need the model again (criterion 48). |
| Pruning | **At save: keep what the current tree or this session referred to** | Never prune (ADR 0017's rule): every re-run would stay in the file until a wipe and fill the 512 MB. Prune by the whole history: the history keeps every result ever written, the same as never. |

## Decision

1. **Asset class.** `AssetStore` entries gain a class `Original | Derived`; the hash is the SHA-256 of the PNG bytes. `pack` writes derived entries as `assets/derived/<hash>.png`, **Stored** (PNG is compressed), in hash order after the originals, so a save is deterministic and an unchanged result is never recompressed (criterion 55). `unpack` reads them with the ADR 0017 §2 rules (bounded read, a missing, oversized or mismatching entry is a missing result, not an open failure). The PNG is written by `curvyo-image-core::encode_png` (fixed settings, no metadata, same bytes for the same pixels): grey 8-bit for a mask, RGBA or grey 8-bit as the input was for Upscale and AI denoise.
2. **Result record on the step** (`image_adjustments` entry, ADR 0072 §1): `result = { asset: <64 hex>, model: text, model_version: text, input_hash: <64 hex>, width_px: integer, height_px: integer }`. **Input hash** (`curvyo-image-core::adjust_hash`, new edge `image-core → sha2`, workspace pin, already locked): SHA-256 over a version tag, the step kind, model name and version, the model fields (Upscale `factor`; none for the other two), and the step's input pixels at full resolution with their width, height and channel count. It is computed once per upstream `AdjustKey` (ADR 0073 §7) and cached.
3. **Applying a result** (`adjust_mask.rs`, integer, no model): Upscale replaces the picture by the stored pixels (size must equal `width_px` x `height_px`); AI denoise gives `in + (ai − in) · strength / 100`, rounded half away from zero; Remove background resizes the stored mask to the picture by the integer area or bilinear rule of ADR 0073 §4, maps it by Threshold `T` and Edge softness `w = max(1, softness · 128 / 100)` to `m′ = 0` below `T − w`, 255 above `T + w`, linear between, and applies it last: grey `255 − (255 − g) · m′ / 255`, or on RGBA `a · m′ / 255` (masked out composites to white and burns nothing, criterion 47).
4. **Out of date and missing.** A step whose current input hash differs from the stored one is **Out of date**: the stored result is still applied (the dots do not change by themselves), the check warns "<n> stored AI results of <image name> are out of date." and Run refreshes it (criterion 48). A step whose result is absent, damaged or whose asset is missing (removed, pruned, not yet synced) **does not burn the picture**: the raster item is skipped with the check warning of unreadable images, so the job never burns something the maker did not see (Proposal; spec criterion 66 "not burnable until Run").
5. **Pruning at save.** A save keeps a derived asset if a node of the current tree refers to it (also as a clone's own adjustments) or this session wrote or referred to it (the session's undo stack can bring it back); all others are left out of the container and the store. Undo or a history preview that reaches a pruned result shows it as missing (decision 4) until Run. Originals keep ADR 0017's rule.
6. **Limits** (criterion 66, **confirmed**): `MAX_DERIVED_ASSET_BYTES` 64 MiB per result (also within the working limit of ADR 0017 §4 in pixels) and `MAX_DERIVED_PROJECT_BYTES` 512 MiB per document, counted separately from `MAX_PROJECT_ASSET_BYTES` (512 MiB of pictures and fonts, ADR 0047 §3). A Run that would pass either is refused before it starts when the size is known (Upscale) and before the write otherwise, with the message naming the limit and "Remove stored result" per step; the group warns from 384 MiB. **Memory:** the whole store is in wasm memory (ADR 0017 §2), so the worst case is about 1 GiB of asset bytes plus working images within the 4 GiB of wasm32; recorded in `docs/technical-debt.md` with the trigger "a measured out-of-memory", whose fix is to keep derived bytes outside the wasm heap until a step reads them.
7. **Sync and history.** Derived assets travel as asset records by hash exactly as pictures and fonts (`referenced_assets` returns them; ADR 0047 §6, ADR 0031 decision 3); a peer that has the register before the bytes sees decision 4's missing state. The history holds only the register (64-character hashes and a few numbers), never pixels (criterion 24). Library sync (ADR 0062) carries none of it.
8. **Expected size** (stated per PR as criterion 66 asks; estimates until measured): a mask at 320 x 320 about 30 to 60 KB; AI denoise of 1 Mpx 1.5 to 3 MB; Upscale x4 of 1 Mpx (16 Mpx RGBA) 25 to 45 MB; x2 of 1 Mpx 6 to 12 MB.
9. **Format.** Part of slice 3's number (ADR 0072 §8): an older build would neither read `assets/derived/` nor apply the result. Golden `image_adjust_ai_v<N>.curvyo` with one stored result per kind (stand-in PNGs `mask_stored`, `upscale_stored_2x`, `denoise_ai_stored`, criterion 54) and a damaged variant (wrong hash, missing member, oversized entry, malformed `result`).

## Consequences

- A project with stored results burns byte-identical G-code with no model and no network (criterion 48's test).
- Saving can shrink a file (pruned results); a history state from an earlier session may need a Run again.
- Files with upscaled pictures grow by tens of megabytes; the budget line in the group shows it.
- Amends ADR 0017 §2 by a note on acceptance (a second asset class with its own pruning and limits).

## Note 2026-11-23: what slice 3 built (architect review; for the customer's acceptance with the PR)

**In plain words (for the customer).** Stored AI results are saved in the project file exactly as
described, with the 64 MB and 512 MB limits, and removed at save when nothing refers to them; undo
after a save still finds a result removed in the same session. One part is not built yet: in a
shared workspace the result picture is not sent to the other people, so on their side the step
shows its result as missing (the picture is not burned there until someone runs the step again).

What changes against the text above:
- Decision 1: the store keeps originals and derived results in two maps of `AssetStore`
  (`curvyo-document-core/src/assets.rs`), not a class field; the container side is
  `container_assets.rs`, the API `derived_assets.rs` (`insert_derived_asset`,
  `derived_asset_bytes`, `referenced_derived_assets`). A derived member is hashed on read; a
  mismatch, an oversized member or one past 512 MiB is a missing result.
- Decision 3: the integer application is `curvyo-image-core/src/adjust_result.rs` (not
  `adjust_mask.rs`), the input hash `adjust_hash.rs`.
- Decision 5: the results the tree refers to when a file is opened count as this session's
  (`mark_referenced_derived_assets`), so a save after their removal keeps them for undo.
- Decision 6: the limit and the budget line count every derived result the store holds, also
  re-runs of this session that nothing refers to any more (kept for undo until the project is
  opened again).
- Decision 7: not built. The workspace sync, the background writer and adopting a document push
  only the pictures' and fonts' assets (`image_assets`, `received_assets` in
  `curvyo-editor-wasm`); a peer sees decision 4's missing state. Debt in
  `docs/technical-debt.md`.
