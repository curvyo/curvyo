# vectrace

A raster tracer and text reader in pure Rust. An RGBA8 raster and trace settings go in, closed
outlines in millimetres come out. Colour tracing, an analysis that proposes settings and an
OCR text reader (PaddleOCR through `rten`) are included. No filesystem, network, clock or
threads in the tracer; it builds for `wasm32-unknown-unknown` and gives the same result there.

## API

```rust
let raster = Raster { width, height, rgba: &rgba };                     // RGBA8, straight alpha
let report = analyse(raster)?;                                          // type, capture, proposal
let map = PixelMap::scale(25.4 / 96.0, 25.4 / 96.0);                    // pixels to millimetres
let params = TraceParams::with_threshold(128);                          // or a preset from PRESETS
let result = trace(raster, map, &params, &mut |_| ControlFlow::Continue(()))?;
let colours = trace_colours(raster, map, &ColourParams::new(params), &mut poll)?;
let model = vectrace::ocr::OcrModel::load(&det_onnx, &rec_onnx, &dict_txt)?;  // bytes are passed in
let lines = vectrace::ocr::read_lines(&model, width, height, &rgba, &mut poll)?;
```

`poll` is called while a trace runs; return `ControlFlow::Break(())` to stop it. See the crate
documentation (`cargo doc --open`) for the full API.

## Licence

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.

## Test assets

The OCR weights and the reference images used by the tests are in `assets/`; their licences
and sources are in [ASSETS.md](ASSETS.md). `VECTRACE_ASSETS` points the tests at another
directory with the same layout; a missing file skips the test with a message unless
`VECTRACE_REQUIRE_ASSETS` is set, then it fails.
