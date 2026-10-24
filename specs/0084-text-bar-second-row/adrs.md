# ADRs for the Text bar second row

ux-engineer draft 2026-10-24, for the architect to confirm. The story is frontend only: it moves
existing bar controls, changes the Font button's look and adds one per-user preference.

**No new ADR, no new crate, no new dependency, no `format_version` change, no change to the
session, the engine or the document.** The only state written is one `localStorage` key.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md): bar chrome is React
  widget code; nothing in this story crosses into wasm or redraws the canvas.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): no crate is touched.
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md): unchanged; the
  writing mode, glyph orientation and Taper registers keep their meaning, only their bar place
  moves.
- [`0051` adrs.md](../0051-sidebar-and-no-menu/adrs.md) decision 8: the pattern for per-user
  view state (webview `localStorage` in both builds, guarded, no host command).
- [`0074` adrs.md](../0074-text-objects/adrs.md): the Font button's command (show the panel,
  Fonts tab, focus the face row; Shift+Ctrl+T) is reused unchanged.

## Dated notes

- 2026-10-24: row 2 open or closed (criterion 14) is the key `curvyo.text-bar-row2`, the JSON
  boolean `true` when open. Every access in `try/catch`; a failed read or a missing key means
  closed. Not an undo step, not part of the project.
