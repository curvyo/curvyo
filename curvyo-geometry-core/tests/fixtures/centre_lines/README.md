# Centre lines fixtures

Golden fixtures of `specs/0117-centre-lines` (criteria 19 and 20), read by
`curvyo-geometry-core/tests/centre_lines_golden.rs` and the unit tests of
`curvyo-geometry-core/src/centre_line/`.

| File | What | Provenance |
|---|---|---|
| `whale_traced.txt` | A whale-like line drawing, traced once by `vectrace` (threshold 128, smooth 80, no speck removal) | Drawn for Curvyo by `curvyo-editor-wasm/src/session/centre_lines_fixture_tests.rs` (own work, CC0) |
| `whale_expected.txt` | Its golden centre lines | Written by the golden test under `CURVYO_BLESS=1` |
| `dolphin.png` | A dolphin over thick wave bands, grayscale | The customer's own drawing; its check-in as a test fixture was approved on 2026-11-18. Licence terms: not yet recorded (asked of the customer) |
| `dolphin_traced.txt` | `dolphin.png` traced once by `vectrace` at the app's default settings | `bless_traced_dolphin` in the same file as the whale |
| `dolphin_expected.txt` | Its golden centre lines | Written by the golden test under `CURVYO_BLESS=1` |

Regenerate the traced inputs only on purpose (a `vectrace` bump never moves them):
`CURVYO_BLESS=1 cargo test -p curvyo-editor-wasm bless_traced -- --ignored`. Regenerate the
goldens with `CURVYO_BLESS=1 cargo test -p curvyo-geometry-core --test centre_lines_golden`;
both are reviewed in the PR.

The orca picture of the demo of 2026-11-18 is licensed to the customer and is not checked in;
its numbers in `specs/0117-centre-lines/adrs.md` were measured locally
(`CURVYO_TRACE_PNG`/`CURVYO_TRACE_OUT` and `CURVYO_CENTRE_INPUT`, see `measure_tests.rs`).
