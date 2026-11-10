# 0108 slice 5: extraction rehearsal (2026-11-10)

Rehearsal: `vectrace/` copied to a scratch directory, its `Cargo.toml` made standalone (explicit
versions instead of `workspace = true`, the lints inline, the repo's `rust-toolchain.toml`, an own
`[workspace]`), the `curvyo-*` dev-dependencies removed.

| Check | Result |
|---|---|
| `cargo build --lib` (native) | passes |
| `cargo build --lib --target wasm32-unknown-unknown` | passes |
| `cargo clippy --lib -D warnings`, native and wasm32 | passes |
| `cargo test --lib` | **does not compile**: 2 blockers below |
| integration tests (`tests/*.rs`) | not run: blockers below |

So the library itself has no Curvyo dependency left. What keeps the test suite from standing alone:

1. `src/units.rs` test compares `PixelMap` bit for bit with `curvyo-document-core::Affine`.
   Fix before the move: freeze the expected bit patterns of the five fixed matrices as constants
   (generated once from `Affine`) and delete the dev-dependency.
2. `src/ocr/read.rs` test and `tests/{reference,ocr_reference,colour,quality,padding,turned,...}`
   decode PNG/JPEG with `curvyo-image-core::decode_working`. Fix: depend on `image =0.25.10`
   (same pin, `png`/`jpeg`, no default features) in dev-dependencies and call a 15-line local
   decode helper that does what `decode_working` does (check the reference tables stay identical
   first; a decoder difference can change pixels).
3. `tests/quality.rs` and `tests/common/quality/real.rs` read Inkscape baseline SVGs with
   `curvyo-svg-core`. Fix: parse the baselines in the helper with a small path reader, or move
   the quality harness to the new repo's `tools/` and keep the golden tables in the crate.
4. `tests/model_record.rs` reads `frontend/public/models/paddleocr-latin/model.toml`. Fix: copy
   `model.toml` into `vectrace/tests/fixtures/` and keep the weights in Curvyo (bytes are passed in);
   the record test then checks the file it is given.
5. Comments naming `curvyo-render-core` (`shapes/join.rs`) and Curvyo specs: reword when the repo
   exists; not a build matter.

## Steps for the move (next task, not done here)
1. Do fixes 1 to 4 on one branch in this repo; the full gate stays green.
2. `git subtree split -P vectrace -b vectrace-export`; push to the new repository's `main`
   (history of the crate is kept).
3. In the new repo: workspace `Cargo.toml` with `[workspace.lints]` as above, `rust-toolchain.toml`,
   `deny.toml` (MIT, Apache-2.0, BSD, Unicode; ban `rayon` outside `rten*`), `LICENSE-*`.
4. CI (ubuntu): `fmt --check`, `clippy -D warnings`, `nextest run`, wasm32 `build` and `clippy`,
   `deny check`, `doc -D warnings`, native vs wasm result check; nightly Windows and macOS test
   run; release by tag (`vX.Y.Z`) publishing to crates.io after the licence and name check.
5. In this repo: `vectrace = { version = "0.1", ... }` (git or registry) replaces the path
   dependency in `curvyo-ui-core` and `curvyo-editor-wasm`; delete `vectrace/`; ADR 0048 (Accepted
   by then) gets a note naming the repository.
6. Weights: `model.toml` and the ONNX files stay in Curvyo's `frontend/public/models/`; the
   standalone app and the SaaS ship their own copy.
