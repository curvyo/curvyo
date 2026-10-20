# ADRs for Text objects

Architect draft 2026-10-19, written against `main` at `e108a1b` (`format_version` 16) without
editing the repository. The spec needs **one new ADR, 0020**
([`docs/adr/0020-text-nodes-text-engine-and-font-sources.md`](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md)),
`needs-customer`: a new node kind, a format bump, a new crate and new dependencies. Accepted by the
customer on 2026-10-20.

## Depends on

- **ADR 0020 (new)**: §1 the text leaf (`shape = "text"`, `text_string` as `LoroText`, `text_font`
  {family, style, face_hash, face_index}, `text_size` = cap height in mm, `text_placement` affine;
  no measured layout stored; uniform scale goes into the size); the matching rule for criteria 29
  and 30; the format bump. §2 `curvyo-text-core` on `harfrust` + `skrifa`, pinned, with `FaceStore`
  and glyph cache. §3 `fontdb` in `curvyo-storage-io`, `font_bytes` Tauri command, included fonts as
  frontend assets. §4 licence record per family (`font.toml` + licence text, checked by a test). §5
  hidden-textarea input sink, edit session in `ui-core`, one commit at exit.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §7: font reference,
  outlines never stored, glyphs as paths that may be open; §9: the edit session is one commit; §11:
  versioned format.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3 (string merges per character),
  §5 (missing face: substitute marked, no layout written, no Object to path); §2 (the edit session
  is ephemeral state).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9 (older builds refuse the
  new version), §12 (referenced font collection; embedding is later).
- [ADR 0006](../../docs/adr/0006-license.md) item 5: each shipped font carries its own licence;
  `deny.toml` gets the allow entries for the new crate (AGPL) and the new dependencies.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §4 (DOM overlay for text
  input only), §5 (face bytes cross once per face per session as a typed array), §6 (the Tauri host
  owns font discovery).
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): `curvyo-text-core` joins the crate
  table; `render-core` and `ui-core` depend on it; it depends on no other workspace crate.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §11: undo of "Add text" is a node
  delete and revival, as for any leaf.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2: `visible`,
  `locked`, `opacity` resolve through ancestors; a text is not a clone origin (spec out of scope).
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md): pattern for a leaf with its own
  affine, SHA-256 content hash, wasm cannot catch a panic, fuzz run in the PR.

## Feature-local decisions

1. **Disagreements with the specification, to fix in it (2026-10-19, for the product owner).**
   - Criterion 19: the Curvyo folder is `app_data_dir()/fonts/` through Tauri's path resolver (the
     base the formats file of `0045` uses), not the `directories` crate.
   - Criterion 28: "the layout values" are not stored; the file holds string, font reference, size,
     placement and style only (ADR 0020 §1). The golden-file list stays.
   - Criterion 16: the browser shows Included only (no Curvyo block until `0062` criterion 9).
   - Criterion 30 (Proposal) is accepted as ADR 0020 §1 matching step (b).
2. **The edit session commits once** (criteria 7, 8, 9): typing does not write to Loro; at exit
   the difference to the stored string is applied as Loro text edits in one commit; empty means no
   node and no commit. Collaborators see the in-flight text through awareness only (ADR 0009 §2).
3. **Caret positions** are the shaper's cluster boundaries, so a ligature or a combining mark is
   never split; no `unicode-segmentation` dependency.
4. **Tabs** in pasted text (criterion 3) are replaced by four spaces in the string itself, so the
   stored string needs no tab rule.
5. **Hostile files** (criterion 32): limits are named constants in `curvyo-text-core`
   (`MAX_FACE_BYTES` 64 MiB, `MAX_GLYPH_POINTS` 65,535); a fuzz target and a time-boxed run are
   part of the PR.

## Dated notes

- 2026-10-19: draft; ADR number 0020 because 0018 is held by the SVG reader ADR of `0024`.
- 2026-10-20: ADR 0020 accepted by the customer, with the wasm budget for the text engine raised
  to 1.3 MiB uncompressed (spike: 1,247 KiB). `harfrust` is MIT only; `fontdb` 0.24 reads names
  with its own reader, not `ttf-parser`.
