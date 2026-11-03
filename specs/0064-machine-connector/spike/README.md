# Spike 0064: connector channel, webview `ws://`, musl build

Throwaway code for `specs/0064-machine-connector/spike-report.md`. Never merged into production
crates. Its own Cargo workspace (`[workspace]` here, `webview/` is a second one), so the
repository workspace and its gate never see it.

- `channel/` — Noise XX (`snow`) + SPAKE2 (`spake2`) with caller-supplied seed bytes; native and
  wasm32 tests.
- `connector/` — `serialport` (no libudev) + sync `tungstenite`: `ports`, `open <dev>`,
  `echo <addr>` (WebSocket echo server requiring subprotocol `curvyo-connector.1`).
- `webview/` — minimal Tauri 2 app, same tauri features as `curvyo-app`; its page opens
  `ws://10.0.2.2:7979` (the spike host's LAN address; edit `webview/dist/probe.js`).
- `run-webview.sh` — echo server + probe under Xvfb, screenshot of the window.
- `evidence/` — screenshots and echo-server logs.

Commands (from this directory):

```text
cargo test -p spike-0064-channel
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test --target wasm32-unknown-unknown -p spike-0064-channel
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  cargo build --release --target aarch64-unknown-linux-musl -p spike-0064-connector
cargo build --release -p spike-0064-connector
(cd webview && CARGO_TARGET_DIR=../../../../target cargo build) && ./run-webview.sh ws-csp
cargo deny check
```
