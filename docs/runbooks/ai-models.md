# Runbook: the AI model files and their host (`specs/0135` slice 3)

The customer's operating duty (Frage 280 A, Frage 282 default A; ADR 0075 §4, §7, ADR 0076 §2,
§9, §12). Curvyo ships **no model**: the app downloads each file on demand from
`https://models.curvyo.org` and checks it against a SHA-256 compiled into the app. The three
files below are converted and pinned (2026-11-21, §2 and §4); if they are not uploaded at the
paths of §3, a download ends with "Model host not reachable." or "This model is no longer
available." Stored results and every other step work regardless.

## 1. The three files to obtain

| Step | Model (name, version in the app) | Upstream source | Upstream file | Licence of code and weights | Training data |
|---|---|---|---|---|---|
| Remove background | `u2netp`, `1` | <https://github.com/xuebinqin/U-2-Net> | `u2netp.pth` (4.7 MB; the repository README links it, Google Drive) | Apache-2.0, "Copyright (c) 2020 Xuebin Qin" | DUTS-TR (research use) |
| Upscale | `realesr-animevideov3`, `1` | <https://github.com/xinntao/Real-ESRGAN> | release `v0.2.5.0`, asset `realesr-animevideov3.pth` (2,504,012 bytes, SHA-256 `b8a8376811077954d82ca3fcf476f1ac3da3e8a68a4f4d71363008000a18b75d`) | BSD-3-Clause, "Copyright (c) 2021, Xintao Wang" | anime video frames with synthetic degradations |
| AI denoise | `ffdnet-color`, `1` | <https://github.com/cszn/KAIR> (`models/network_ffdnet.py`); weights release `v1.0` | `ffdnet_color.pth` (3,411,551 bytes, SHA-256 `286f04f7e3a20254c067ba95efb5d3339f4fc8a09a365a93cd469184d4cdb279`) | MIT, "Copyright (c) 2019 Kai Zhang" | BSD400, ImageNet validation, Waterloo Exploration (Gaussian noise) |

Rejected on 2026-11-21 (too slow in the browser, ADR 0075 note): `realesr-general-x4v3` (143 s on 1
Mpx in Chromium) and `scunet-color-real-psnr` (470 s). `tools/models/export_onnx.py
WORK_DIR realesr-general-x4v3 scunet-color-real-psnr` still exports them.

Record the SHA-256 of each downloaded `.pth` (`sha256sum`); it goes into `models.toml` as a
comment beside the entry (the upstream file hash of ADR 0075 §4). Customer decision Frage 281 A
accepts weights trained on research datasets.

## 2. Convert each to ONNX (ADR 0075 §4)

On a machine with Python 3.11, the upstream repository checked out at the release above, and
`torch` and `onnx` pinned in a virtual environment:

- opset 17, **static input shapes**, weights inline (no external data), no custom operators;
- `u2netp`: input `1 x 3 x 320 x 320` (RGB, ImageNet mean and deviation), first output `d0`
  (`1 x 1 x 320 x 320`, after its sigmoid);
- `realesr-animevideov3` (SRVGGNetCompact, `num_feat=64, num_conv=16, upscale=4`): input
  `1 x 3 x 256 x 256` (RGB 0 to 1), output `1 x 3 x 1024 x 1024`;
- `ffdnet-color` (FFDNet, `in_nc=3, nc=96, nb=12`, the noise level fixed at 15/255 inside the
  file): input `1 x 3 x 256 x 256` (RGB 0 to 1), output the same.

`tools/models/export_onnx.py WORK_DIR` does all three (pinned torch 2.9.1+cpu, onnx 1.23.2,
onnxruntime 1.32.0) and checks each against torch; `curvyo-imageai-core/tests/real_models.rs`
(ignored, `CURVYO_MODELS_DIR`) times them on `rten` and `tools/models/compare_ort.py` compares the
outputs with ONNX Runtime. The local conversion of 2026-11-21 gave:

| File | Bytes | SHA-256 |
|---|---|---|
| `u2netp-1/<sha>.onnx` | 4,611,299 | `47bb7b9d53f1eebde658cfb63aa5e110e6fcab39b50779eb2e5522e5dfe182ed` |
| `realesr-animevideov3-1/<sha>.onnx` | 2,492,873 | `b5dc4e5c20f404247a91ee0cc1014c79f2940274b2586e67b129d8369cb7c1a8` |
| `ffdnet-color-1/<sha>.onnx` | 3,423,405 | `91328fb58c0184b31c8ecfbf934df889a47f29ffd13ea0a1b7c40a706ed880a0` |

A re-conversion with other wheel versions can give other bytes: pin what was uploaded.

## 3. Names and paths to upload (ADR 0076 §2)

For each converted file with SHA-256 `<sha>` (64 lowercase hex):

```text
https://models.curvyo.org/u2netp/1/<sha>.onnx
https://models.curvyo.org/u2netp/1/LICENSE.txt
https://models.curvyo.org/realesr-animevideov3/1/<sha>.onnx
https://models.curvyo.org/realesr-animevideov3/1/LICENSE.txt
https://models.curvyo.org/ffdnet-color/1/<sha>.onnx
https://models.curvyo.org/ffdnet-color/1/LICENSE.txt
```

`LICENSE.txt` is the upstream licence text (and any upstream `NOTICE`) followed by the line
"Converted to ONNX by Curvyo, weights unchanged." A published path is never overwritten: a new
conversion is a new `version` and a new hash.

## 4. Where the hashes go

**Done 2026-11-21 (lead):** the three files of the table in §2 are pinned in `models.toml` and
`src/models.rs`. The upload was verified on 2026-11-23 by a GET of the three URLs (pinned bytes, `Range`, CORS);
the CI release gate no longer has `continue-on-error`. For a later model version:

In **one commit**, for each model:

1. `curvyo-imageai-core/models.toml`: `sha256` (the ONNX file's), `bytes` (its exact size) and
   `placeholder = false`;
2. `curvyo-imageai-core/src/models.rs`: the same `sha256` as 32 bytes (`[0x.., ...]`), `bytes`
   and `placeholder: false`.

Then run `CURVYO_RELEASE_MODELS=1 cargo test -p curvyo-imageai-core --test model_record`: it
fails while a placeholder remains, and `the_compiled_list_is_models_toml` fails when the two files
disagree. In `.github/workflows/ci.yml` (job `imageai-boundary`), remove `continue-on-error: true`
from the release gate step in the same commit. Before an app release that changes `MODELS`, fetch
each pinned URL and compare its SHA-256 (ADR 0076 §12).

## 5. The host (Frage 282, default A: Cloudflare R2)

- an R2 bucket on the customer's account with the custom domain `models.curvyo.org` (the
  `curvyo.org` DNS on Cloudflare); public read of objects, no listing, writes only from the
  maintainer's account;
- CORS: `Access-Control-Allow-Origin: *`, methods `GET, HEAD`, allowed request header `Range`,
  exposed headers `Content-Range, Content-Length, ETag`, `Access-Control-Max-Age: 86400`;
- on `.onnx` and `LICENSE.txt`: `Cache-Control: public, max-age=31536000, immutable`,
  `Content-Type: application/octet-stream`, byte ranges on, no compression or transformation, no
  redirect;
- access logs off or kept at most 30 days;
- a takedown deletes the file and purges the cache; the next release drops it from `MODELS`.

The app's only change for another provider is none: it knows the URL alone (`MODEL_HOST`).
