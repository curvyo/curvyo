# Plan for "Template library over the network: browse and load templates from sources on the net that I added"

One slice, built to ADR 0085 (decisions 1 to 9) and `ux-notes.md`, on top of `0137` (merged, PR #470).
Customer answers 2026-11-24: Frage 334 A (Curvyo catalogue = GitHub raw files of `curvyo/templates`),
340 B (Enter also searches the added Iconify sources), 341 C (the browser build has the Library tab;
ADR 0085 decision 2 keeps the Online section desktop only, so the browser shows the Online group as
"Not available in the browser" and sends nothing). No `format_version` change: the online keys are
added to the format-58 Came from record. New crate `curvyo-library-core`, new dependency `ureq` 3.

## Affected crates/modules

- `curvyo-library-core` (new, pure, `forbid(unsafe_code)`, wasm32): `sources.rs` (the
  `online-sources.toml` text), `request.rs` (a guarded GET request value, the ready entry),
  `catalogue.rs` (the index parse and checks), `verify.rs` (size and SHA-256), `iconify.rs`
  (request builders, parsers, `icon_svg`, the search words), `error.rs`; fixtures in `tests/fixtures/`.
- `curvyo-storage-io`: `http.rs` (new: `Transport` trait, `UreqTransport`, error kinds).
- `curvyo-document-core`: `came_from.rs` (optional online keys: `address`, `licence`, `author`,
  `credit`), `credits.rs` (new: the credits derived from the objects).
- `curvyo-ui-core`: `library_online.rs` (new: the Online rows and every word of them).
- `curvyo-app`: `online.rs` (new: the commands, the session memory), `library.rs` (the online
  source keys read through the same commands).
- `curvyo-editor-wasm`: the record with the online keys, `import_icon_svg`, the credits list, texts.
- `frontend`: the Library tab rows of the Online group, Add source form, Load and icon import, the
  File tab's Credits block.

## Tasks

M1 `curvyo-library-core` (tests first, fixtures are recorded response shapes, no network):
- [ ] 1. Catalogue index parse and checks: format, limits, path rule, licence per item, credit
  required for attribution licences, newer format (fulfils AC 6, 7, 3)
- [ ] 2. `verify`: size and SHA-256 against the entry before any read (fulfils AC 8)
- [ ] 3. Iconify: request builders, `/collections`, `/collection`, `/<prefix>.json`, `/search`
  parsers, `icon_svg`, search words trimmed, collapsed, cut at 100 (fulfils AC 2, 7, 10)
- [ ] 4. Guarded request: https only, no query beyond the protocol, ready entry only at
  `raw.githubusercontent.com` (fulfils AC 1, 2)
- [ ] 5. `online-sources.toml` text: parse, add (duplicates, https), remove, order (fulfils AC 1)
- [ ] 6. Font rule: "Font not included" when a font's licence is unknown (fulfils AC 9)

M2 host fetch (`curvyo-storage-io::http`, `curvyo-app`):
- [ ] 7. `Transport` with `UreqTransport` (GET only, 3 redirects, https, timeouts, size limit, no
  cookies or Referer, `User-Agent: Curvyo`) and a recording fake; the ureq transport is tested
  against a local server (fulfils AC 2)
- [ ] 8. Host commands: add (one request), expand, refresh, template load with hash check, thumbnail,
  icon bodies, search on Enter to Iconify sources only; per-source offline and error kinds;
  session-only memory (fulfils AC 1, 2, 5, 8, 10)

M3 document and rows:
- [ ] 9. Came from online keys; Load imports every page, one step "Load template"; icon import 24 mm,
  one step "Import icon" (fulfils AC 3)
- [ ] 10. Credits derived from the objects (fulfils AC 4)
- [ ] 11. `curvyo-ui-core::library_online` rows and texts (fulfils AC 1, 3, 5, 9, 10)

M4 UI:
- [ ] 12. Online group, ready entry, Add source form, expand and Refresh, Load, icon grid, Enter
  search and the Search online button, Offline per source, browser "not available" (fulfils AC 1 to 5, 10)
- [ ] 13. Credits block in the File tab (fulfils AC 4)
- [ ] 14. Docs: design-system row, spec status, ADR 0007 amendments noted

## Validation

Golden fixtures for the catalogue and the Iconify answers; a fake transport for the host logic; a local
server for the ureq transport; `cargo deny check` for the crate and `ureq`; the full gate of
CLAUDE.md section 7. At most one manual live request to Iconify and to GitHub raw, reported in the PR.
