# Spike report: machine connector (story 0064)

Date: 2026-11-03. Branch `story/0064-machine-connector`. Code: `spike/` (own Cargo workspaces,
never merged; how to rerun: `spike/README.md`). Host: Arch Linux x86_64, Rust 1.99.0 (repo
toolchain), WebKitGTK 2.54.1, Node 26, wasm-bindgen 0.2.129. Answers ADR 0032 Decision 7/8/10
and `adrs.md` feature-local decision 9.

## Summary

| Question | Result | Effect on ADR 0032 |
|---|---|---|
| 1. `snow` + `spake2` on wasm32, caller randomness | **Yes.** No `getrandom` in the wasm graph; handshake + pairing tests pass native and in wasm (Node) | Decision 8 holds; pin `snow =0.10.0`, `spake2 =0.4.0`, `rand_chacha =0.3.1` (exact features below) |
| 2. `ws://<LAN-IP>` from the Tauri webview | **Linux: yes, after one CSP change** (tested in a real Tauri 2.12 window). Windows/macOS: by docs, likely yes; not run | Decision 7: direct WebSocket from the page stays; byte pipe in `curvyo-app` not needed on Linux, kept as the documented fallback. Add the CSP line; macOS needs an Info.plist key |
| 3. Static `aarch64-unknown-linux-musl` with `serialport` + `tungstenite` | **Yes, on this host with no C toolchain** (`rust-lld` + Rust's self-contained musl). Not run on ARM (no qemu, no Pi here) | Decision 10 holds; CI needs only `rustup target add` and the linker env var. Use `tungstenite =0.28.0`, not 0.30 |
| 4. `cargo deny` | **advisories ok, bans ok, licenses ok, sources ok** (repo `deny.toml` + musl targets) | No new licence; one new old `nix 0.26` via `serialport` |

## 1. `snow` and `spake2` on wasm32 with caller randomness

Dependencies (`spike/channel/Cargo.toml`):

```toml
snow = { version = "=0.10.0", default-features = false, features = ["use-curve25519", "use-chacha20poly1305", "use-blake2"] }
spake2 = { version = "=0.4.0", default-features = false }      # default feature = getrandom
rand_chacha = { version = "=0.3.1", default-features = false } # rand_core 0.6 line, as spake2 0.4
rand_core = { version = "0.6", default-features = false }
```

- `snow` 0.10: with `use-getrandom` off, `DefaultResolver::resolve_rng()` returns `None`. A
  20-line `CryptoResolver` whose `resolve_rng` returns a `ChaCha20Rng::from_seed(caller_seed)`,
  wrapped in `FallbackResolver::new(seed_resolver, DefaultResolver)`, supplies the RNG; DH,
  cipher and hash come from the default resolver. `Builder::with_resolver(...)` for keypair
  generation, initiator and responder. `std` feature off, works (alloc only).
- `spake2` 0.4.0 (latest stable; 0.5 is only `0.5.0-pre.0`): `start_a_with_rng` /
  `start_b_with_rng` take any `CryptoRng + RngCore` (rand_core 0.6). `Ed25519Group`.
- Identity binding: both sides use `"app:" ‖ h` and `"con:" ‖ h` with `h` = Noise handshake
  hash; the SPAKE2 messages travel inside Noise transport messages.
- `cargo tree --target wasm32-unknown-unknown -i getrandom` → "did not match any packages".
- Tests (`spike/channel/src/lib.rs`): XX completes, both hashes equal, initiator sees the
  responder's static key after message 2, JSON `hello` round trip, a flipped ciphertext bit is
  rejected, right code → equal PAKE keys, wrong code → different keys, same seed → same keypair.
  - `cargo test -p spike-0064-channel` → 3 passed.
  - `cargo test --target wasm32-unknown-unknown -p spike-0064-channel` with
    `wasm-bindgen-test-runner` (Node) → 3 passed.
- Size added to a wasm (post-`wasm-bindgen`, `+simd128` from the repo config, no `wasm-opt`;
  full run minus a baseline with the same export doing nothing):
  editor profile (`opt-level=3`, LTO, cgu 1): **+149 KB raw, +52 KB gzip**;
  `opt-level="z"`: +108 KB raw, +41 KB gzip. Upper bound: the editor wasm already contains
  `sha2`, so the real delta is somewhat smaller. Goes to `docs/technical-debt.md` with the story
  (no limit set).
- Note for the plan: `snow::types::Random` is `Send + Sync`, so the seeded RNG is owned by the
  resolver box; one seed per handshake (32 bytes from `crypto.getRandomValues` in the frontend,
  OS RNG in the connector). The `channel` module stays a pure step function as Decision 1 says.

## 2. `ws://` to a LAN address from the Tauri webview

**Linux, tested.** `spike/webview`: Tauri 2.12.1 / wry 0.57 with exactly `curvyo-app`'s tauri
features and `custom-protocol` (production origin), page script opens
`new WebSocket("ws://10.0.2.2:7979", ["curvyo-connector.1"])` against the spike echo server,
run under Xvfb (`spike/run-webview.sh`), screenshots in `spike/evidence/`.

- Page origin `tauri://localhost`, `isSecureContext = true`.
- With `curvyo-app`'s current CSP (`connect-src 'self' ipc: http://ipc.localhost`): the
  constructor throws `SecurityError: The operation is insecure.`, a `securitypolicyviolation`
  for `connect-src ws://10.0.2.2:7979/` fires, the server sees nothing
  (`evidence/webkitgtk-current-csp.png`, `evidence/echo-current-csp.log`).
- With `ws:` added to `connect-src`: upgrade arrives with `Origin: tauri://localhost` and the
  subprotocol, 3 binary bytes echoed, the page reports the echo back
  (`evidence/webkitgtk-csp-with-ws.png`, `evidence/echo-csp-with-ws.log`). WebKitGTK applies no
  mixed-content block to `ws:` from the `tauri:` scheme. No Tauri capability/permission is
  involved: capabilities only gate IPC commands, not the page's network requests.

Lines needed in `curvyo-app/tauri.conf.json` (both `csp` and `devCsp`):
`connect-src 'self' ipc: http://ipc.localhost ws:` (dev: plus the existing Vite entries). CSP
cannot express "private IP ranges only"; `ws:` allows any host on the `ws` scheme. Acceptable
because only our own script runs (`script-src 'self'`), and every frame is Noise.

**Windows (WebView2), from docs, not run.** Tauri 2 serves the app as `http://tauri.localhost`
by default (`app.windows[].useHttpsScheme` false); keep it false, because from
`https://tauri.localhost` Chromium blocks `ws:` as mixed content. Chromium's Local Network Access
(LNA) gates WebSocket connections to private IPs behind a `local-network` permission prompt;
the page's origin (`*.localhost`) is loopback address space, which is more private than the
LAN, so by the LNA rules no prompt is expected, but whether WebView2 shows, auto-denies or
auto-grants it inside an embedded host is **unverified**. Check on the nightly Windows runner or
a Windows machine before the UI slice ships; the fallback below covers a "no".

**macOS (WKWebView), from docs, not run.** Same `tauri://localhost` scheme and WebKit engine as
Linux, so the CSP line is the same. macOS 15+ asks the user for Local Network access the first
time an app connects to a LAN address; the bundle needs `NSLocalNetworkUsageDescription` in
`Info.plist` (Tauri: `bundle.macOS` Info.plist merge), otherwise the connection fails silently.
ATS does not apply to connections by IP address; a `.local` hostname would need
`NSAllowsLocalNetworking`. **Unverified** until run on a Mac.

**Fallback (if WebView2 or WKWebView refuses):** a byte pipe in `curvyo-app` — two commands
(`connector_open(addr) -> id`, `connector_send(id, bytes)`) and one event
(`connector_frame { id, bytes }`), a `tungstenite` client thread in Rust; Noise and protocol stay
in the editor wasm, so the host sees only ciphertext. Cost: a sync WebSocket client in
`curvyo-app` (same crate as the connector uses) and three capability entries. Not needed for
Linux.

Finding for the ADR: any web page the user visits can open a WebSocket to the connector (the
browser sends its `Origin`, the connector may not trust it). Noise stops it from doing anything,
but an unpaired client may start pairing: a hostile page can burn the 5 attempts of a code
(denial of pairing, not a takeover). Proposal: while a code is shown, refuse pairing from an
`Origin` that is neither `tauri://localhost`, `http://tauri.localhost` nor the configured
browser-build origin, and keep the per-code limit. Chromium's LNA prompt additionally blocks
public sites by default.

## 3. Static `aarch64-unknown-linux-musl` build

`spike/connector`: `serialport =4.10.1` (default features off: no `libudev`) and
`tungstenite =0.28.0` (default features off, `handshake`).

```text
rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  cargo build --release --target aarch64-unknown-linux-musl -p spike-0064-connector
→ ELF 64-bit LSB executable, ARM aarch64, statically linked, stripped, 530 KB
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  cargo build --release --target x86_64-unknown-linux-musl -p spike-0064-connector
→ ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped, 599 KB
```

- No cross C toolchain on this host (no `aarch64-linux-gnu-gcc`, `musl-gcc`, `zig`, `cross`)
  and none needed: the dependency set has no C code, Rust's musl targets ship `crt*.o` and
  `libc.a` (self-contained), and `rust-lld` ships with the toolchain. Release profile used:
  `opt-level="z"`, LTO, `panic=abort`, `strip`.
- The x86_64 musl binary runs here: `ports` lists 32 `/dev/ttyS*` (`Unknown` type), `open`
  on a missing device returns a clean error. The aarch64 binary was **not run** (no qemu-user,
  no Pi); a run on a Pi 4 is a manual check before the story is demoed.
- Without `libudev`, `serialport` scans `/sys/class/tty` and still reads USB VID/PID, product,
  manufacturer and serial from sysfs, so the connector can name a CH340/Arduino port. Caveat: it
  `expect`s `/sys/class/tty` to exist (panics in a container without sysfs); the connector
  should treat "no ports" as an error path, not call it blind.
- CI needs: the two `rustup target add`s and the two linker env vars (or a
  `.cargo/config.toml` `[target.*-musl] linker = "rust-lld"` entry); no apt packages, no Docker.
- `tungstenite` 0.30 pulls `rand 0.10`, `getrandom 0.4`, `sha1/digest 0.11` (all new to the
  lockfile); 0.28 uses `rand 0.9`, `getrandom 0.3.4`, `sha1 0.10`/`digest 0.10`, which the
  workspace already locks. Same API for the spike's use. Pin `=0.28.0` (Proposal).

## 4. `cargo deny`

Run in `spike/` with a copy of the repository `deny.toml` plus the two musl targets and AGPL
exceptions for the spike crates (`spike/deny.toml`):

```text
cargo deny check   →   advisories ok, bans ok, licenses ok, sources ok
```

- Licences of the new crates: `snow` Apache-2.0 OR MIT, `spake2` MIT OR Apache-2.0,
  `curve25519-dalek` BSD-3-Clause, `chacha20poly1305`/`blake2`/`hkdf` MIT OR Apache-2.0,
  `serialport` MPL-2.0, `nix` MIT, `tungstenite` MIT OR Apache-2.0 — all on the allow-list.
- Duplicate warnings (bans: `multiple-versions = "warn"`): `rand_core` 0.6/0.9,
  `rand_chacha` 0.3/0.9, `bitflags` 1/2, `syn` — all already duplicated in the repo lock. New to
  the repo: `nix 0.26.4` next to the existing `nix 0.31.3` (via `serialport`, Linux io only).
- No advisory hits. (`snow` < 0.9.5 had RUSTSEC-2024-0011; 0.10.0 is past it.)

## Changes to ADR 0032 (for the architect)

1. Decision 8: replace "RustCrypto resolver features: X25519, ChaChaPoly, BLAKE2s" with the
   exact feature list above; `spake2 =0.4.0` default features off; `rand_chacha =0.3.1`
   (rand_core 0.6, not the workspace's 0.9 line); `tungstenite =0.28.0`; `serialport =4.10.1`.
   Remove the "if not, the primitive crates replace them" branch: confirmed.
2. Decision 7: open point closed for Linux (direct WebSocket, CSP `connect-src ... ws:`); add
   `useHttpsScheme: false` (default) as a requirement and `NSLocalNetworkUsageDescription` for
   macOS; Windows and macOS stay "verify before the UI slice", byte pipe as fallback.
3. Decision 3 (Proposal): pairing refused from foreign `Origin`s, see §2.
4. Decision 10: CI builds the musl binaries with `rust-lld`, no extra toolchain; the aarch64
   binary is run once by hand on a Pi 4 before the demo.
