# Reproduce the isolated experiment

This is an experiment, not a workspace member or production API. Assertions and
numeric literals deliberately use independently known coordinates in millimetres.
Copy this directory to `target/0066-solver-spike` from the repository root, then:

```sh
python3 target/0066-solver-spike/generate.py
cargo test --locked --release --manifest-path target/0066-solver-spike/Cargo.toml
cargo run --locked --release --manifest-path target/0066-solver-spike/Cargo.toml > target/0066-solver-spike/evidence/native-results.json 2> target/0066-solver-spike/evidence/cpu.log
cargo build --locked --release --lib --target wasm32-unknown-unknown --manifest-path target/0066-solver-spike/Cargo.toml
wasm-bindgen --target nodejs --out-dir target/0066-solver-spike/pkg target/0066-solver-spike/target/wasm32-unknown-unknown/release/curvyo_0066_solver_spike.wasm
cd target/0066-solver-spike
node compare.cjs
```

Use wasm-bindgen CLI 0.2.129. Add `--offline` after downloading the pinned packages.
CPU measurements use Linux `CLOCK_THREAD_CPUTIME_ID` via rustix. The portable
WASM corpus excludes clocks; its geometry matches the native corpus. Four records
are intentional API rejection checks, including two unsupported native ellipse
pair types. Those two types are separately exercised through exact auxiliary
common-contact geometry. Passing an unsupported-API rejection is not support.

The generator, corpus, native runner, WASM runner and raw evidence are retained.
No product robustness or cross-platform proof beyond the named executions is implied.
