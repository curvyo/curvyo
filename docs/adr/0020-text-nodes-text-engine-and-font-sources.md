# ADR 0020: Text nodes, the text engine and font sources

**Status:** Accepted 2026-10-20 by the customer (Frage 5 TEXT-ADR, answer A); proposed by the
architect 2026-10-19. `needs-customer` (document model: a new node kind; file format:
`format_version` bump; one new crate and new dependencies). The spike in "Before the build" ran
(see "Spike results (2026-10-19)"); its one failed point, the wasm size, is settled by the
customer's raised budget (Frage 6 TEXT-WASMSIZE, answer A). Extends
[ADR 0002](0002-document-model-units-and-svg-round-trip.md) §7 (text was named, never modelled),
[ADR 0009](0009-concurrent-editing-semantics.md) §3 and §5, [ADR 0004](0004-persistence-and-cross-machine-sync.md)
§12 (font collection), [ADR 0006](0006-license.md) item 5 (shipped fonts), [ADR 0001](0001-ui-framework-and-canvas-rendering.md)
§4 and §5 (text-input overlay, bytes across the boundary), [ADR 0011](0011-workspace-and-crate-layout.md)
(one new crate). Asked by `specs/0074-text-objects`. Next free number: 0018 is taken by the SVG
reader ADR of `0024`.

## Context

`0074` puts the first text into the document. Three things are expensive to change later: the
shape of the text node in the file, the library that turns hostile font bytes into outlines inside
a wasm module that cannot catch a panic (ADR 0017 §Context), and where fonts come from on each
target. The editor core runs as wasm in the webview on the desktop too (ADR 0001 §3), so the wasm
figure is the shipping figure. Today: `format_version` is 16 on `main`, no font code, no shaping
library, and the Tauri host already reads the maker's formats file from `app_data_dir()`.

## Decision

### 1. A text is a leaf with its own affine map

A node with `shape = "text"` and four registers, plus the ordinary `name`, `visible`, `locked`,
`opacity` (ADR 0015 §2) and style keys (fill, stroke; no markers):

| Register | Content |
|---|---|
| `text_string` | a `LoroText`: per-character merge (ADR 0009 §3). At most 10,000 Unicode scalar values (`MAX_TEXT_CHARS`). |
| `text_font` | one atomic map: `family`, `style` (the face's style name), `face_hash` (64 hex, SHA-256 of the font **file** bytes, as ADR 0017's asset hash), `face_index` (index in a `.ttc`, else 0). Written only by a maker's pick. |
| `text_size` | `Length` in mm: the height of the capital H (spec Question 3). Range 0.1 to 2,000 mm. |
| `text_placement` | six finite numbers, the `Affine` from text space (mm, origin at the first baseline's left end, y down) to document mm. Its translation is the anchor. |

No measured layout is stored: no advances, no line positions, no outlines. Line height, cap
height, kerning and ligatures are derived from the face each time. This makes ADR 0009 §5 hold by
construction: a peer drawing a substitute has nothing measured it could write back.

**Transforms.** A transform `T` composes into `text_placement`, except that its uniform scale goes
into the size: if the linear part of `T` is `s` times an orthogonal matrix (within 1e-9 relative),
then `text_size *= s` and the placement takes `T` with `s` divided out; otherwise `text_placement
= T * text_placement` and the size is kept (spec criterion 26). A size clamped at 2,000 mm keeps
the rest of `s` in the matrix, so nothing jumps.

**Name.** An empty `name` reads as the first 24 characters of the string (Layers row, history
label); a rename writes `name`. No extra register.

**Read model.** `ObjectSnapshot::Text(TextSnapshot { id, string, font, size, placement, style })`.
As with images, every exhaustive match gets an arm; path-only commands convert first (spec
criterion 24), output code never sees a text.

**Font matching** (spec criteria 29, 30), in order, over the faces the host listed: (a) same
`face_hash` and `face_index`; (b) same family and style, case-insensitive, preferring Curvyo fonts,
then Included, then This computer; (c) missing: draw with the included sans face, mark it, refuse
Object to path, write nothing. (b) uses the face without a notice. Hashes are computed lazily, only
for the few faces that pass the family/style filter, never for the whole system list.

**File format.** The node is a new on-disk shape: next free `format_version` at merge (17 if
nothing else lands first); older builds refuse the file (ADR 0004 §9). Golden file
`text_v<N>.curvyo`.

| Option | Why it lost or won |
|---|---|
| **A. Leaf with string, font ref, cap-height size, affine (chosen)** | Same pattern as the clone and image leaves; skew and rotation need no decomposition; nothing measured is stored. |
| B. Store shaped glyph positions (advances) with the string | Faster open, but every peer with another font version writes different numbers; violates the spirit of ADR 0009 §5 and bloats the log on every keystroke. |
| C. Size as em size plus a separate anchor and rotation | Mirrors SVG `font-size`, but cap height is what the maker sets (Question 3); rotation and skew as separate fields need a decomposition rule for every transform. |

### 2. Shaping and outlines: `harfrust` + `skrifa` in a new `curvyo-text-core`

New crate `curvyo-text-core`, one responsibility: "turns a face's bytes, a string and a size into
positioned glyph outlines and line metrics." It depends on `kurbo` (workspace) and on `harfrust`
(shaping: GSUB, GPOS, `liga`, `calt`, kerning) and `skrifa`/`read-fonts` (tables, metrics,
outlines; TrueType quadratics are delivered as `quad_to`, so the exact degree elevation of
criterion 23 is ours). `harfrust` is MIT only; `skrifa` and `read-fonts` are MIT OR Apache-2.0. All `no_std`-capable, wasm32-clean. Pinned exactly, as
`i_overlay` is: their outlines become cuts. `#![forbid(unsafe_code)]`, no I/O.

It owns: a `FaceStore` (`BTreeMap<FaceKey, Arc<[u8]>>`, filled by the editor session once per
face per session, ADR 0001 §5), a per-face glyph outline cache in em units, line layout (left
aligned, line height from hhea or OS/2 typo with `USE_TYPO_METRICS`), cap height (measured H, else
OS/2 `sCapHeight`, else 0.7 em), cluster boundaries for the caret and hit box. Callers:
`curvyo-render-core` (draw), `curvyo-ui-core` (Object to path, hit box, edit session). It does not
depend on `curvyo-document-core`; `ui-core` maps its `BezPath` output to document paths.

**Hostile files.** The reader returns `Result` everywhere. Limits: a face file at most 64 MiB,
at most 65,535 points per glyph, composite depth as `skrifa` enforces (spike confirms), shaping
input capped by `MAX_TEXT_CHARS`. `cargo-fuzz` target on `FaceStore::add` and `shape_and_outline`
with the fixtures of `tests/fixtures/fonts/`, time-boxed run in the PR (as ADR 0017).

| Option | Why it lost or won |
|---|---|
| **A. `harfrust` + `skrifa` (chosen)** | Current HarfBuzz port and the font reader Chrome ships for untrusted fonts (fuzzed upstream); one table parser (`read-fonts`) for shaping and outlines. Newer, 0.x versions: hence the exact pin and the spike. |
| B. `rustybuzz` + `ttf-parser` | Mature and equally small, but `rustybuzz` is superseded by `harfrust` (same maintainers moved); choosing it now means a migration later. Fallback if the spike fails A. |
| C. `swash` | Shaping plus scaling in one crate, but its shaper covers less of OpenType and it is slowly maintained. |
| D. `cosmic-text` | A full text system: brings `fontdb` (filesystem) and rich-text layout into core; most of it we do not need and part of it is forbidden in `*-core`. |
| E. Put it into `curvyo-geometry-core` | No new crate, but a hostile-input parser and two large dependencies inside the geometry kernel; render-core would pull both. |

### 3. Font sources: `fontdb` in `curvyo-storage-io`, bytes passed into core

- **Desktop list.** A module `fonts` in `curvyo-storage-io` (ADR 0007's precedent: a module, not a
  crate) uses `fontdb` (MIT) to list the Curvyo folder (`app_data_dir()/fonts/`, the base the
  formats file already uses; non-recursive, every face of a TTC) and the system fonts (its own
  fontconfig reader on Linux, no C library; the OS font folders on Windows and macOS). System
  fonts are listed on first use of the Text tool or Fonts tab, off the UI thread. Each entry:
  `source`, `family`, `style`, `path` (host side only), `face_index`, an opaque `id`.
- **Bytes.** A Tauri command `font_bytes(id)` returns raw bytes (`tauri::ipc::Response`, not
  JSON); the frontend hands them to wasm once per face per session.
- **Included fonts.** Static frontend assets, `frontend/public/fonts/<family>/`, fetched on first
  use on both targets, never compiled into the wasm. The browser lists Included only; its Curvyo block
  stays empty (not shown) until adding fonts in the browser (`0062` criterion 9) exists.
- The spec's `directories` crate is not needed: Tauri's path resolver is already a dependency.

| Option | Why it lost or won |
|---|---|
| **A. `fontdb` in `-io`, bytes into core (chosen)** | Pure Rust, no C dependency, knows fontconfig rules; core stays file-free (`CLAUDE.md` §6). `fontdb` 0.24 reads the font name tables with its own reader; it brings no `ttf-parser`. |
| B. `font-kit` | Uses fontconfig, CoreText, DirectWrite through C/FFI: three native code paths, `unsafe` in the dependency tree we must vet. |
| C. Own scan of OS font folders with `skrifa` | No new dependency, but we would reimplement fontconfig's include and alias rules. |

### 4. Licence record for shipped fonts

Each family sits in `frontend/public/fonts/<family>/` with its font files, its licence text
(`OFL.txt` or `LICENSE`) and a `font.toml` (`family`, `licence = "OFL-1.1" | "Apache-2.0"`,
`source_url`, `version`). The app's licence list reads these. `cargo deny` and the npm check do not
see them; a test in `curvyo-text-core/tests/included_fonts.rs` does: every font file has a
`font.toml` with an allowed licence and a licence text that matches it, and every face parses.
Fonts are shipped **unmodified**: no subsetting or renaming, because a modified OFL font may not use
a Reserved Font Name (Source Sans 3 has one). ADR 0006 item 5 governs; nothing here changes it.

### 5. Text input: a hidden textarea as the input sink, state in `ui-core`

While a text is in edit mode, one off-screen-transparent `<textarea>` (1 px, positioned at the
caret so the IME candidate window appears there) holds focus. It is only an input sink: its
`beforeinput`, `compositionstart/update/end`, `copy`, `cut`, `paste` and key events become small
declared messages (`Insert`, `Delete`, `MoveCaret`, `Select`, `Preedit`) to the edit session in
`curvyo-ui-core`, which owns string, caret, selection and the in-session undo (criterion 9). The
string, caret, selection highlight and the IME preedit are drawn by the canvas through the engine
(what you see is what is cut). The session is ephemeral (ADR 0009 §2) and writes **one** commit
at exit: the difference to the stored string as Loro text edits (keeps per-character merge).
Clipboard works through the native events, no permission prompt.

| Option | Why it lost or won |
|---|---|
| **A. Hidden textarea sink, canvas draws (chosen)** | IME, dead keys and clipboard from the browser; layout from our engine only. |
| B. Visible `contenteditable` styled with the font over the canvas | The browser's shaping and metrics differ from ours: the caret drifts from the cut outlines; rotated or skewed text cannot be overlaid. |
| C. Key events on the canvas only | No IME or dead keys; ADR 0001 §4 permits the overlay for exactly this case. |

## Before the build (spike `spike/text-engine`, never merged)

1. `harfrust` and `skrifa` versions build for `wasm32-unknown-unknown`, pass `cargo deny`; added
   release wasm size measured (budget 700 KiB uncompressed; over it, option B is measured too).
   The customer raised this budget to 1.3 MiB uncompressed on 2026-10-20 (Frage 6 TEXT-WASMSIZE, A).
2. Shaping plus outlines for 500 characters inside 10 ms in the webview (criterion 3's 50 ms).
3. `fontdb` lists 2,000 faces in under 2 s on the plan's machine.
4. Hostile fixtures and a 10-minute fuzz run: no panic, no hang.

## Spike results (2026-10-19)

Branch `spike/text-engine` (never merged), report `docs/spikes/text-engine.md` on that branch.

- **Wasm size:** option A adds 1,247 KiB to the release editor wasm (433 KiB gzip) against the
  700 KiB budget: failed. The customer raised the budget for the text engine to 1.3 MiB
  uncompressed on 2026-10-20 (Frage 6 TEXT-WASMSIZE, A); 1,247 KiB is inside it.
- **Speed:** shaping plus outlines of 500 characters take 0.2 to 0.8 ms in wasm (target 10 ms).
- **Hostile input:** 0 panics and 0 hangs in about 98k hostile cases (7,235 fixtures and 91,169
  random mutations, 600 s).
- **Fallback B:** `rustybuzz` + `ttf-parser` fails `cargo deny`: both crates are flagged
  unmaintained (RUSTSEC-2026-0206, RUSTSEC-2026-0192). B is no fallback.
- **Facts corrected from the draft:** `harfrust` is MIT only (not MIT OR Apache-2.0), and
  `fontdb` 0.24 has its own name-table reader rather than `ttf-parser`.

## Customer decisions (2026-10-20)

All answered A: TEXT-FONTSOURCE (Curvyo folder, then Included, then system fonts on the desktop;
the browser has the first two only), TEXT-LICENCE (ship Source Sans 3, Source Serif 4 and Great
Vibes, OFL 1.1 or Apache-2.0 only), TEXT-SIZE (Size is the cap-H height in mm, exact),
TEXT-TOPATH (a group with one path per letter), TEXT-ADR (this ADR accepted), TEXT-WASMSIZE (wasm
budget for the text engine 1.3 MiB uncompressed). Details in `specs/0074-text-objects/`.

## Dated notes

- 2026-10-20, customer: the wasm budget for the text engine is raised from 1.3 MiB to **1.4 MiB**
  uncompressed. The finished slice of `0074` adds 1,426 KiB (501 KiB gzip) to the release editor
  wasm: the engine about 1,290 KiB, the slice's own code about 135 KiB (`docs/technical-debt.md`).

- 2026-10-20, single-line fonts (`specs/0076-single-line-fonts/adrs.md`): §1 `text_font` gains the
  optional key `kind = "single_line"` (absent: outline), matching compares faces of the same kind
  and substitutes EMS Readability for a missing single-line face; `format_version` 18. §2
  `curvyo-text-core` also reads SVG fonts (`roxmltree` `=0.20.0`, own path-data reader, limits in
  `limits`), `Face` has two sources and `Face::kind()`. §3 `curvyo-storage-io` lists `.svg` in the
  Curvyo folder through `curvyo-text-core`'s SVG-only reader entry point (amended 2026-10-23,
  `specs/0076-single-line-fonts/adrs.md` decision 5). §4 the allowed licences include the Hershey notice
  (`licence = "Hershey"`, `NOTICE.txt`; customer, Frage 18 SL-LICENCE A). Customer, Frage 20
  SL-WASM: the wasm budget for the text engine is raised from 1.4 MiB to **2 MiB** uncompressed
  (`docs/technical-debt.md`).

- 2026-10-26, text layout controls slice 2 (`specs/0079-text-layout-controls/adrs.md` decisions
  7 to 10): §1 one more optional register, `text_taper` (a fraction, −0.5 to 0.5, absent 0);
  `format_version` 24. §2 `LayoutOptions.taper` maps every glyph in its own frame before the turn;
  horizontal and vertical lines stay lines, other segments are refitted with `kurbo`'s curve
  fitter within 0.01 mm (no new dependency).

- 2026-10-24, text layout controls slice 3 (`specs/0079-text-layout-controls/adrs.md` decisions
  11 to 13 and the spike note): §1 two more optional registers, `text_writing_mode` and
  `text_glyph_orientation`; `format_version` 20. §2 `layout` lays every line out in line space
  and maps it by the writing mode's frame (a quarter turn for columns); upright runs are shaped
  top to bottom by `harfrust` (`vmtx`/`VORG`, `vert`), sideways runs left to right and turned;
  Auto reads a generated UAX #50 table (`tools/unicode-data/vertical_orientation.py`, Unicode
  17.0, 99 ranges). No new dependency.

- 2026-10-24, text layout controls slice 1 (`specs/0079-text-layout-controls/adrs.md` decisions 1
  to 6): §1 the text node gains five optional registers, `text_align`, `text_letter_space`,
  `text_word_space`, `text_line_space` (factors of `text_size`) and `text_rotation` (degrees);
  absent is the default and a default is never written; `format_version` 19. Still no measured
  layout is stored. §2 `curvyo-text-core::layout` takes `LayoutOptions` (alignment, spacing in
  mm, line spacing, rotation): lines are aligned on their extent, justified to the widest line,
  letter spacing turns `liga`, `clig` and `calt` off, glyphs turn about their cluster's centre;
  drawing, the edit session, the hit box and Object to path share that one layout.

- 2026-10-24, customer: the wasm size budget for the text engine (2 MiB since 2026-10-20) is
  withdrawn; there is no hard budget. The size is tracked as the open entry "Editor wasm size" in
  `docs/technical-debt.md`, with `wasm-opt -Oz` and loading the engine on first use as the levers.

## Consequences

- One more leaf kind: an arm in every exhaustive match, as images took.
- A text drawn with a substitute stays visibly marked and cannot be cut until the font is there;
  embedding fonts (ADR 0004 §12) is the later fix.
- Two font parsers in the desktop product (`fontdb`'s own name-table reader in the host for names, `read-fonts` in
  wasm for drawing). Accepted: they never disagree about anything stored, only the list.
- Single-line fonts (ADR 0002 §7) later reuse `text_font` with a different source; the open-path
  output already fits the engine's `BezPath`.
