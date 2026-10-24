# Editor wasm size

Release `curvyo_editor_wasm_bg.wasm` after wasm-bindgen, no wasm-opt, as built by
`frontend/scripts/build-wasm.sh`. Recorded by the CI job `wasm size` on every push to
`main`; rows whose ref is not `main` were measured on a branch by hand.

![Editor wasm size over time](wasm-size.svg)

| Date (UTC) | Commit | PR | Ref | Bytes | gzip -9 | Bytes change |
|---|---|---|---|---|---|---|
| 2026-10-24 | `3082c82` |  | story/text-on-path (branch) | 12,866,733 | 4,268,422 | +196,438 |
| 2026-10-24 | `5cb7a0a` |  | story/text-kerning (branch) | 12,763,446 | 4,227,451 | +93,151 |
| 2026-10-24 | `3ff03cf` |  | main | 12,670,295 | 4,202,645 | +2,296,304 |
| 2026-10-20 | `4ece1b1` |  | main | 10,373,991 | 3,405,489 |  |
