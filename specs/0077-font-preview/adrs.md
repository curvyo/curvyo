# ADRs for the font preview

Architect draft 2026-10-23, written against `story/single-line-fonts` at `67dd251` (`0074`, `0075`
and `0076` on one branch) without running code. Written for the defaults of Frage 25 to 28 (all
A: tiles only, the split built in this story, no hover preview, the sample follows the selected
text). If the customer answers Frage 27 B, decision 2 already gives the hover preview its drawing
path; nothing here changes.

**No new ADR, no new crate, no new external dependency, no project `format_version` change, no
change to the font library file.** Every decision below is reversible in one PR: the only state
written is a per-user `localStorage` key (decision 4), which is not a file format we protect.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1: the preview is React
  widget code; the rules it applies (missing characters, sample choice) are Rust. §4: the
  preview is a panel beside the canvas, not an overlay on it, so the "DOM overlays only for text
  input and handles" rule is untouched. §5: face bytes cross once per face as typed arrays; per
  keystroke only the sample string and the path strings of visible tiles cross. §6: the host
  gains no command (it already hands out font bytes).
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §2: `curvyo-text-core`
  draws the tiles; its responsibility sentence ("turns a face's bytes, a string and a size into
  positioned glyph outlines and line metrics") covers laying out a sample line and reporting
  the characters a face lacks. 64 MiB face limit and `Result` on hostile bytes apply unchanged.
  §3: bytes come from the host's `font_bytes` (desktop) or IndexedDB (browser), as for rows.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: `ui-core` → `text-core`
  already exists; `editor-wasm` stays a binding layer.
- [`0074` adrs.md](../0074-text-objects/adrs.md): the face pick of a row press ("Change font"),
  `.notdef` drawing (criterion 6), the Text bar state read after each change.
- [`0075` adrs.md](../0075-font-library/adrs.md) decisions 1 and 8: the font library file keeps
  `deny_unknown_fields` and its `format`; this story adds nothing to it (decision 4).
- [`0076` adrs.md](../0076-single-line-fonts/adrs.md) decisions 3 and 6: `Face::kind()` and the
  engine-drawn `<svg>` path for single-line names; tiles reuse that drawing for both kinds.
- [`0051`](../0051-sidebar-and-no-menu/) criterion 10 (`frontend/src/lib/panelCollapsed.ts`): the
  pattern for per-user view state in `localStorage`, guarded, no host command.
- [`0007`](../0007-stroke-and-fill-styling/) criterion 39: `keep_view_origin_for_width_change`,
  the resize that keeps the view's top-left origin when the panel opens or closes.

## Feature-local decisions (2026-10-23)

1. **Canvas area split (criteria 2 to 5, 23): a dated note, no ADR.** One React component
   `CanvasArea` in `frontend/src/components/` wraps today's rulers, bars and `<Canvas>` and takes
   one optional side pane `{ mode: "split" | "full", content }`. Split: a two-column layout, the
   pane 35 % of the canvas area or one tile column if wider (2026-10-23, was `max(320px, 50 %)`,
   then 30 %),
   a 1px border; when a split would leave the drawing under 360px the pane opens in full. Before the width changes (open, close, split to full and back) the frontend calls the
   existing `keep_view_origin_for_width_change(delta)`, so the drawing neither moves nor re-zooms
   (criteria 2 and 5). Full: the pane covers the whole canvas area; the canvas stays mounted at
   its last size (never resized to 0, which the `wgpu` surface rejects), rulers and bars are not
   rendered, and the canvas requests no frames while covered. The pane re-renders only on its own
   inputs (sample, size, listed families, Text bar state), never on canvas frames or pointer
   moves (criterion 23). `0063` reuses `CanvasArea` by passing its own pane; no pane registry, no
   divider, no second slot until a story needs one (§5). Rejected: an ADR now (nothing here is
   hard to reverse: one component, no format, no public Rust API); building `0063`'s workbench
   shell first (Frage 26 B, pulls manufacturing work forward); a separate window (out of scope).

2. **Drawing tiles (criteria 11, 13, 15, 17, 18): every tile from the engine's outlines, both
   kinds.** A tile is an inline `<svg>` with one `<path>`: filled with `currentColor` for an
   outline face, `fill="none"` and a 1 screen-px non-scaling stroke for a single-line face (as
   `0076` decision 6 rows). Scale = size px / the face's cap height, so the slider sets the cap
   height on screen for every tile alike (criterion 12).
   - `curvyo-text-core::preview_path(face, text)` stays as the rows use it: the whole sample as
     one line, however wide; the tile clips it at its edge (customer decision 2026-10-23, see the
     dated note; the wrap into up to 3 lines with an ellipsis is gone). Tests: one line however
     long, an empty string.
   - `curvyo-text-core::Face::missing_chars(&self, text) -> Vec<char>`: the distinct characters
     (whitespace and controls excluded, first-seen order) that map to glyph 0 (OpenType charmap;
     SVG `glyph_of` = `<missing-glyph>`). Pure, no cache needed (a charmap lookup per character).
   - `curvyo-ui-core::preview_faces` ("keeps the faces the font preview draws within a byte
     budget, the least recently drawn dropped first"): `add(id, bytes, index)`, `get(id)` (marks
     use), eviction above **256 MiB** of face bytes, a use counter instead of a clock. The
     `editor-wasm` binding exports `preview_tile(id, text) -> {d, box, cap, missing, single_line}`
     (JSON, binding only) and `add_preview_face` / `has_preview_face`.
   - Bytes load through the existing `fontFaceLoader` fetch (one read per face, rows and tiles
     share it), after 100 ms in view, visible tiles plus one screen (criterion 17). Grid
     virtualisation follows `useFontTreeScroll`; no new npm dependency.
   - Kerning question: none left. Tiles use the engine's shaping (`harfrust`, or the SVG `hkern`
     rule), the same as the cut; `.notdef` and `<missing-glyph>` are drawn as the face draws them.
   - Budget: one `preview_tile` per visible tile per keystroke (30 calls, glyph outlines cached
     per face by ADR 0020 §2). The implementer measures this in the plan; if 100 ms is missed,
     coordinates go to 2 decimals before any other change (dated note here).
   - Column width default `max(240px, 4 × size px)`, capped at the grid width; UX notes may tune.
   Rejected: outline tiles as CSS `FontFace` text, as the rows do. The browser falls back to other
   fonts for characters the face lacks, so the tile would show an "ü" the cut will not have, which
   breaks criterion 18 and the story's user value (umlauts); WebKit (macOS Tauri) and Chromium
   shape differently from the engine; and the missing check needs the bytes in wasm anyway, so
   CSS would be a second copy and a second drawing path. Rejected: a `<canvas>` per tile (`0076`
   decision 6's reasons). Rejected: reusing the session's `TextFonts` store (it never drops a
   face; scrolling 2,000 system families would keep every file in wasm memory).

3. **Missing-glyph check (criterion 18).** `Face::missing_chars` of decision 2, returned with the
   tile's path in the same call, so the marker and the drawing come from the same face and sample.
   The frontend keeps the last result per tile (id + sample) and shows at most 10 characters. No
   coverage table per family: a tile shows one face (Regular or first), so the check is per face.
   Test: the spec's "Jürgen" fixture without "ü" and Source Sans 3.

4. **Where sample, size and split/full live (criterion 6): webview `localStorage`, key
   `curvyo.font-preview`, JSON `{ "sample": string, "size": number, "full": bool }`**, read and
   written with the guarded pattern of `panelCollapsed.ts`; unreadable or out-of-range values mean
   the defaults ("Anna & Jonas 2026", 32, split). Same key in the browser build. Not synced
   between computers (accepted: view state). Rejected: keys in the font library file. It has
   `deny_unknown_fields`, so a `[preview]` table forces `format = 3`, and a build of `0075`/`0076`
   would report the file broken and rename it at its next change, risking the maker's tags and
   favourites for a view setting; it would also sync a sample text that is per-session in spirit.
   Rejected: a new per-user view settings file (a new file format and two host commands for three
   values).

5. **Which session state the preview follows (criteria 8, 9).** The Text bar state the frontend
   already reads after each change (`text_bar()`, `useText.ts`) gains two fields: `sample_node`
   (the node id when exactly one text object is selected or being edited, else empty) and
   `sample` (that text's first line, at most 200 characters, cut at a char boundary). Computed in
   `curvyo-ui-core`'s text bar state, pure and unit-tested (none, one, two texts, edit mode, a
   newline, 201 characters). The preview overwrites its field when `(sample_node, sample)`
   differs from the last value it saw, and only then, so a typed sample stays until the selection
   or that text's first line changes (criterion 10). No extra document read per keystroke, no
   new event. Rejected: a separate `selected_text_first_line()` call (a second read per change);
   comparing the sample only (selecting another text with the same first line would not reset a
   typed sample).

## For the product owner

- Criterion 6: "per user" means per browser profile and per computer (`localStorage`, decision 4).
- Criterion 15: outline tiles are drawn from the engine's glyphs, not as browser text (decision
  2); what the maker sees is what the cut gets, including `.notdef`.

## Open for the customer

None. Frage 25 to 28 are product questions; their defaults (A) need nothing further here.
Frage 27 B would reuse decision 2's `preview_tile` for the hover drawing and needs a note then.

## Dated notes

- 2026-10-23: draft. No ADR: decision 1 is one React component, decisions 2, 3 and 5 are pure
  functions in existing crates, decision 4 is a `localStorage` key, not a protected format.
- 2026-10-23, customer feedback after the demo (decisions 1 and 2): the split default is 35 %;
  a tile shows one line, cut off hard at its right edge (no wrap, no ellipsis), so
  `preview_path` lost its wrap parameter and the tile call its width and line count. The engine
  lays out the whole line rather than dropping glyphs past the width: simpler, and a width change
  (divider drag, window resize) no longer redraws any tile. A divider drag applies at most one
  width per animation frame and resizes and redraws the canvas in the commit that changes its
  width (`CanvasArea` calls `fitCanvas`), not in a later resize-observer step.
