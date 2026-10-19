# Spike: text engine for ADR 0020

Branch `spike/text-engine`, never merged. Date 2026-10-19. Checks the four points of ADR 0020
"Before the build" for option A (`harfrust` + `skrifa`) and, where A fails a point, option B
(`rustybuzz` + `ttf-parser`).

Machine: VM `impa` (Intel Xeon SierraForest, 8 vCPU, 19 GB RAM, virtio disk with btrfs), Linux
7.2.9, rustc 1.99.0, wasm-bindgen 0.2.129, node 26.11.1. Release profile of the workspace
(`opt-level = 3`, `lto = true`, `codegen-units = 1`), the same profile and the same
`wasm-bindgen` step (no `wasm-opt`) as `frontend/scripts/build-wasm.sh`.

Versions, pinned exactly in `spike-text/Cargo.toml`: `harfrust =0.14.0`, `skrifa =0.48.0`
(`default-features = false, features = ["std"]`, no autohinter), which share `read-fonts 0.45.1`
and `font-types 0.12.7`; `fontdb =0.24.0`; fallback `rustybuzz =0.20.1`, `ttf-parser =0.25.1`.

Fonts: Source Sans 3 3.052R and Source Serif 4 4.005R (SIL OFL 1.1, official GitHub releases,
downloaded into `target/spike-fonts/`), TTF and OTF (CFF) of each; Liberation Sans, FreeSerif and
Adwaita Sans from the system for the hostile run.

## Summary

| Point | Target | A: harfrust + skrifa | B: rustybuzz + ttf-parser |
|---|---|---|---|
| 1a. builds for `wasm32-unknown-unknown` | yes | yes | yes |
| 1b. `cargo deny check` | passes | **passes** | **fails**: both crates are flagged unmaintained (RUSTSEC-2026-0206, RUSTSEC-2026-0192) |
| 1c. added release wasm size | 700 KiB | **1,247 KiB** in the editor wasm (1,274 KiB standalone): **fails** | 686 KiB in the editor wasm (749 KiB standalone): passes, by 14 KiB |
| 2. shape + outline 500 characters | < 10 ms | native 0.12 to 0.45 ms; wasm (node) 0.21 to 0.78 ms | native 0.21 to 0.29 ms; wasm (node) 0.44 to 0.52 ms |
| 3. fontdb lists 2,000 faces | < 2 s | warm 22 ms for 2,030 faces; cold 8.2 s on this VM, I/O-bound (see 3) | same (fontdb is independent of A/B) |
| 4. hostile input: no panic, no hang | none | **0 panics, 0 hangs** in 7,235 fixtures and 91,169 random mutations (600 s) | **33 panics** (3 fixtures, 30 mutations), 0 hangs |

**Verdict:** A holds on speed, licences, `cargo deny` and hostile input, and fails only the wasm
size budget, by 547 KiB. B is not a usable fallback: it fails `cargo deny` (both crates are now
officially unmaintained; the advisory names `harfrust` as the replacement), it panics on mutated
fonts (a panic aborts the wasm module), and it saves only 561 KiB.
The size budget needs a decision by the architect and customer, not a library switch (options
under "What the architect has to decide").

## 1. Build, licences, `cargo deny`, wasm size

**Build.** `cargo build --release --target wasm32-unknown-unknown -p spike-text` builds without
warnings for A and B. Neither needs `getrandom`, threads or a C compiler.

**Licences (A and fontdb):**

| Crate | Version | Licence | Used by |
|---|---|---|---|
| harfrust | 0.14.0 | MIT (only MIT, not MIT OR Apache-2.0 as the ADR draft says) | core |
| skrifa | 0.48.0 | MIT OR Apache-2.0 | core |
| read-fonts | 0.45.1 | MIT OR Apache-2.0 | core |
| font-types | 0.12.7 | MIT OR Apache-2.0 | core |
| bytemuck | 1.25.2 (already in the workspace) | Zlib OR Apache-2.0 OR MIT | core |
| smallvec | 1.16.2 (already in the workspace) | MIT OR Apache-2.0 | core |
| bitflags | 2.x (already in the workspace) | MIT OR Apache-2.0 | core |
| fontdb | 0.24.0 | MIT | `-io` |
| fontconfig-parser | 0.5.8 | MIT | `-io` (Linux) |
| roxmltree | 0.20.0 | MIT OR Apache-2.0 | `-io` (Linux) |
| memmap2 | 0.9.11 | MIT OR Apache-2.0 | `-io` |
| slotmap | 1.1.1 | Zlib | `-io` |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT | `-io` |

All are in the `deny.toml` allow-list. With A and fontdb, `cargo deny check` gives
"advisories ok, bans ok, licenses ok, sources ok"; the count of duplicate-version warnings stays at
31 (no new duplicate). `fontdb 0.24` does **not** pull `ttf-parser`: it vendors a minimal name-table
reader (`src/ttf_parser.rs`), so the ADR's line "Adds `ttf-parser` (MIT/Apache) to the host binary"
is out of date. `memmap2` has `unsafe` (mmap) in the `-io` dependency tree; fontdb can be built
without its `memmap` feature if the architect wants to avoid it.

With B added, `cargo deny check` fails: `error[unmaintained]` for `rustybuzz` (RUSTSEC-2026-0206,
"the crate is unmaintained and will not receive further fixes", alternative: `harfrust`) and
`ttf-parser` (RUSTSEC-2026-0192). Licences of B: rustybuzz MIT, ttf-parser MIT OR Apache-2.0, plus
`unicode-bidi-mirroring`, `unicode-ccc`, `unicode-properties`, `unicode-script`, `core_maths`
(MIT and/or Apache-2.0).

**Wasm size.** Two methods, both after `wasm-bindgen`, no `wasm-opt`:

1. Standalone: `spike-text` as a `cdylib` with three `#[wasm_bindgen]` exports (parse a face from
   bytes, shape and outline a string), built with and without the engine feature. The baseline
   (wasm-bindgen, allocator, the same exports doing a byte sum) is 21,222 bytes.
2. In the editor: `curvyo-editor-wasm` of `origin/main` with one extra export that calls the
   engine, against the unchanged editor (10,320,412 bytes). This is the shipping number: code that
   the editor already contains (core, alloc, fmt) is not counted twice. The change was measured and
   reverted; it is not on the branch.

| Build | Total bytes | Added | Added gzip -9 |
|---|---|---|---|
| A standalone | 1,325,582 | 1,304,360 (1,274 KiB) | 410,013 |
| A in the editor | 11,597,585 | **1,277,173 (1,247 KiB)** | 442,887 |
| B standalone | 769,900 | 748,678 (731 KiB) | 266,435 |
| B in the editor | 11,023,190 | **702,778 (686 KiB)** | 259,894 |
| harfrust only (standalone) | 1,074,702 | 1,053,480 | |
| skrifa only (standalone) | 349,650 | 328,428 | |

Where A's bytes go (code section by crate, standalone, from the wasm name section): harfrust
457 KiB, read-fonts 265 KiB, skrifa 148 KiB, data section 132 KiB (Unicode and shaping tables),
the rest is core, alloc and the allocator. harfrust's code is about twice rustybuzz's (246 KiB);
it uses `#[inline(always)]` widely.

Optimising for size makes it worse: `opt-level = "s"` gives 1,441,277 bytes for A standalone,
`"z"` 1,527,310 (per-package for the font crates only) and 1,471,712 / 1,564,367 for the whole
build. Removing the wasm name section (`wasm-bindgen --remove-name-section`) gives A standalone
1,158,404 bytes against a 16,107-byte baseline, still 1,115 KiB added. `wasm-opt` is not installed here and not part of the
editor build; it was not measured.

## 2. Shaping plus outlines, 500 characters

Text: 500 characters of Latin, "AVATAR Toffee: the office waffle flavour fits five fine fjords.
Typewriters, LAVA, Yacht, We fly off. fi fl ff ffi ffl! " repeated. One call shapes the whole
string (default features, so `kern`/GPOS, `liga`, `calt` on) and draws **every** glyph through an
`OutlinePen` into a `Vec<f32>` (no glyph cache, so this is the worst case of the ADR's cached
design). "Cold" also parses the face from the bytes (and, in wasm, copies the bytes across the
boundary, about 0.3 to 1.2 MB per call); "warm" reuses a parsed face. 300 runs each, after one
warm-up call.

Shaping checks: in Source Serif 4, "AV" shapes to 1,219 font units against 1,338 for the nominal
advances (kerning of -119 units applied), and "fi", "fl", "ff", "ffi", "ffl" each become one
ligature glyph (gid 417 for "fi"). Source Sans 3 kerns "AV" by -14 units and has an "ff" ligature
but, by design, no "fi" ligature, so Source Serif 4 is the better fixture for criterion 5. harfrust
and rustybuzz return identical glyph ids and advances for all cases.

**Native (release), milliseconds:**

| Face | A cold median (p99) | A warm median (p99) | B warm median (p99) |
|---|---|---|---|
| Source Sans 3 TTF | 0.155 (0.237) | 0.124 (0.143) | 0.221 (0.300) |
| Source Sans 3 OTF (CFF) | 0.453 (0.590) | 0.416 (0.528) | 0.266 (0.305) |
| Source Serif 4 TTF | 0.132 (0.267) | 0.124 (0.215) | 0.212 (0.280) |
| Source Serif 4 OTF (CFF) | 0.442 (0.575) | 0.423 (0.724) | 0.289 (0.342) |

**Wasm in node 26 (V8), `wasm-bindgen --target nodejs`, milliseconds:**

| Face | first call | A cold median (p99) | A warm median (p99) | B cold median (p99) |
|---|---|---|---|---|
| Source Sans 3 TTF | 7.5 (includes compiling the module) | 0.315 (0.742) | 0.224 (0.289) | 0.471 (0.953) |
| Source Sans 3 OTF | 3.2 | 0.762 (1.677) | 0.681 (0.921) | 0.499 (0.938) |
| Source Serif 4 TTF | 0.6 | 0.250 (0.291) | 0.211 (0.237) | 0.436 (0.500) |
| Source Serif 4 OTF | 0.8 | 0.780 (0.868) | 0.728 (0.998) | 0.517 (0.690) |

Every case is more than 10 times under the 10 ms target. Caveat: the desktop webview on Linux is
WebKitGTK (JavaScriptCore), not V8; this spike did not run it there. With a factor of 10 to spare,
a slower wasm engine does not change the verdict. CFF outlines are about three times slower than
TrueType in skrifa (charstring interpretation); a per-face glyph cache (ADR 0020 §2) removes that
cost after the first draw of a glyph.

## 3. fontdb listing system faces

`fontdb::Database::load_system_fonts()` (its own fontconfig reader, no C library), release:

- This machine has **30 system faces** (8 families). First call 139 ms (fontconfig files and font
  headers not yet in the page cache), then 2.5 to 3.7 ms.
- 2,000 faces: a folder with 2,000 font files (copies of 182 different TTF/OTF files, 636 MB),
  added to the fontconfig configuration through `FONTCONFIG_FILE` and also loaded with
  `load_fonts_dir`. With the files in the page cache: **2,030 faces in 22 to 23 ms**, 15
  families. That is 0.011 ms per face, so 2,000 faces is 90 times under the 2 s target.
- Not in the page cache (fresh reflink copies on btrfs): **8.2 s** for 2,000 faces, of which
  0.05 s is CPU time. The I/O floor of this VM disk alone, opening each file and reading its first
  4 KiB from Python, is 7.4 s (3.7 ms per file). fontdb's own cost is the difference.

So fontdb is fast enough; a cold scan of 2,000 files on a slow disk is not, whichever library
reads them. On a desktop the system fonts are usually in the page cache (the desktop itself uses
them); on a local SSD a cold random read is about 0.1 ms, which gives roughly 0.3 s. The ADR
already lists system fonts off the UI thread and on first use; the remaining risk is a cold first
listing on a slow disk (network home, HDD, VM), which a cached list (keyed by folder modification
times, as fontconfig's own cache does) would remove. This is a design note for the architect, not
a library question.

## 4. Hostile input

`spike-text/src/bin/hostile.rs`, release, stable toolchain. Each case runs on a worker thread
under `catch_unwind` with a 10 s timeout per case (a timeout is a hang). Per case: A parses the
face (`skrifa::FontRef` and `harfrust::Font`), shapes and outlines a 90-character Latin string,
then draws every glyph of the face; B parses with `rustybuzz::Face`, shapes and outlines the string
and draws every glyph with `ttf-parser`.

**Fixtures (7,235 cases):** empty input; zero-filled inputs of 1 B to 1 MiB, with and without an
sfnt header; 200 random byte strings (half with an `OTTO` or `00010000` header and a random table
count) and 50 random `ttcf` collections; and for each of seven real faces (Source Sans 3 TTF/OTF,
Source Serif 4 TTF/OTF, Liberation Sans, FreeSerif OTF, Adwaita Sans): truncation at every length up to 512
bytes and at 300 lengths across the file; for every table, its data zeroed, the file cut inside
it, its length set to 0xFFFFFFFF, its offset set past the end, and its content randomised;
1, 10, 100 and 1,000 random bytes overwritten (25 seeds each); and for the TrueType faces a
composite glyph that refers to itself, two composites referring to each other, all composites of
the face in one cycle (1,331 in Source Sans 3), and a simple glyph whose last
`endPtsOfContours` claims 65,536 points.

Result: **A: 0 panics, 0 hangs. B: 3 panics** (Adwaita Sans with 1,000 random bytes, seeds 12, 13
and 24), 0 hangs. The slowest fixture took 145 ms (A and B together, drawing every glyph of Adwaita
Sans with 1,000 random bytes). The composite cycles and the 65,536-point glyph each take
about 2 ms: `read-fonts` enforces `MAX_RECURSION_DEPTH = 64`, `MAX_COMPOSITE_EDGES = 2048` and
`MAX_OUTLINE_POINTS = 65,536` (`read-fonts/src/limits.rs`), so the ADR's point limit is already the
library's.

A difference worth knowing: A accepted 6,985 of the 7,235 inputs as a face (it parses tables
lazily and fails per glyph or per table), B only 3,798. So with A, "the face parses" does not mean
"the face is sound"; the ADR's per-face check (draw the H for the cap height, shape a sample) should
decide whether a listed file counts as a font (spec criteria 19 and 32).

**Random mutation run, 600 s:** 91,169 cases, each a random mutation of one of the seven faces
(1 to 64 random bytes anywhere, 1 to 16 random bytes inside one table, a random byte in a table
record's offset or length, a truncation, or a copied block). **A: 0 panics, 0 hangs. B: 30
panics**, 0 hangs. All 33 B panics (fixtures and run) are in `rustybuzz 0.20.1`,
`src/hb/ot_layout_gsubgpos.rs` lines 611, 616 and 621: `backtrack_coverages.get(index).unwrap()`
and the lookahead and input equivalents in chained-context lookups, which a damaged GSUB/GPOS
reaches. In wasm a panic aborts the module, so with B a damaged font would take the editor down.
The cases are saved in `target/spike-fonts/crashes/` (not committed) and reproduce with
`cargo run --release -p spike-text --bin repro -- <case>`.

The slowest case of the run took 1.17 s (a mutated Source Sans 3 OTF, A and B together, drawing
every glyph plus the string); it was not saved, so the spike cannot say which library spent
the time. It is below the 10 s hang limit, but a story-level fuzz target should report per-case
time for A alone; drawing a single text never draws every glyph of a face.

**Fuzzing:** `cargo-fuzz` is not installed and there is no nightly toolchain on this machine, so no
coverage-guided fuzz run was made (as instructed, nightly was not installed). The 10-minute random
mutation run above replaces it for this spike; it is not coverage-guided. Both libraries are fuzzed
upstream (fontations in OSS-Fuzz; harfrust runs HarfBuzz's fuzzing corpus). The ADR's planned
`cargo-fuzz` target stays necessary for the story.

## What the architect has to decide

A is the right library pair; only the 700 KiB budget does not hold. Options, none measured here
beyond what is above:

1. Raise the budget to about 1.3 MiB uncompressed for A (440 KiB more gzip). The editor wasm is
   10.3 MB today (wgpu dominates), so A adds 12 %. On the desktop the module is read from disk.
2. Load the text engine as a second wasm module on first need (a document with text, the Text tool
   or the Fonts tab): editor start stays as it is, but the render path must handle "engine not yet
   loaded".
3. Measure `wasm-opt -Oz` on the editor build (needs binaryen); typical savings are 10 to 20 %,
   which would still not reach 700 KiB for A.

B is not an option: it fails `cargo deny` today, and the saving (561 KiB) is less than half of A's
cost.

## How to reproduce

```text
cargo run --release -p spike-text --bin bench -- <font file>
cargo run --release -p spike-text --bin lig -- <font file>
cargo run --release -p spike-text --bin fontlist [-- <extra font dir>]
cargo run --release -p spike-text --bin hostile -- <seconds> <font files...>
cargo build --release --target wasm32-unknown-unknown -p spike-text   # add --no-default-features for the baseline
wasm-bindgen --target nodejs --out-dir target/spike-wasm/node --out-name spike \
  target/wasm32-unknown-unknown/release/spike_text.wasm
node spike-text/bench-wasm.js target/spike-wasm/node <font files...>
```
