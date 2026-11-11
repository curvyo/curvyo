# Assets

`assets/` holds what the tests read. Plain git files, no LFS.

## `assets/trace-references/`: reference images, CC0 1.0

Public domain dedication (CC0 1.0). Sources: openclipart.org, the maintainer's own photos,
scans and AI-generated logos (each sidecar in `tests/fixtures/trace/reference/customer/*.json`
says `CC0 1.0` and its origin), and other CC0 or public-domain images (Wikimedia Commons and others) as listed per image in
`tests/fixtures/trace/reference/SOURCES.md`. The sidecars name their image as
`tests/assets/trace-references/<file>`; the tests read it from here.

## `assets/models/paddleocr-latin/`: OCR weights, Apache-2.0

The PaddleOCR Latin (mobile) detector and recogniser as ONNX, unmodified, with the alphabet.
Licence: Apache-2.0 (`LICENSE` in that folder). `model.toml` records each file's source URL and
SHA-256; `tests/model_record.rs` checks the record against the files and the hashes the crate
expects.
