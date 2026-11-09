# ADR 0047: Document fonts

**Status:** Accepted 2026-11-09 (the customer asked for the behaviour; decisions below are inside it). `needs-customer` only for FONT-LICENCE (default A, proposal; embedding permission of a font file). Extends [ADR 0017](0017-image-nodes-and-embedded-assets.md) §2 (the `assets/` members and the asset store), [ADR 0020](0020-text-nodes-text-engine-and-font-sources.md)'s `face_hash`, [ADR 0031](0031-sync-engine-relay-protocol-and-transport.md) decision 3 (asset records). Asked by `specs/0107-document-fonts`.

## Context

A text stores a `FontRef` with the SHA-256 of the font file (`face_hash`), but not the file. On another device the text is "missing" and draws in the fallback face; in a shared workspace this is what the other maker sees. Pictures already solve the same problem: bytes in the container, hash in the document, an asset record in sync.

## Decision

1. **Fonts are assets.** The bytes of a font file a text uses are inserted into the document's `AssetStore` under `AssetHash::of(bytes)`, which equals `FaceHash::of(bytes)` (both SHA-256 of the file). The container writes them as `assets/<hash>.bin` (the existing writer and reader, `bin` is a known extension). No new member kind, no new crate, no new dependency. `format_version` does not change: a build without this ADR reads the members as undecodable assets and drops them on its next save.
2. **What is embedded:** every font file that a text node of the tree uses, deduplicated by hash, by the session before every save, before every pack for a room and on every sync push that follows a font change. Only bytes the session holds (the face store) can be embedded; a font not loaded in the session cannot be used by a text it lays out, so the set is complete for everything drawn. Fonts used only by history are not embedded and are not removed once embedded (like pictures); the document wipe keeps exactly the hashes that a text or an image in the tree names (`prune_assets` extended).
3. **Budget:** fonts share `MAX_PROJECT_ASSET_BYTES` (512 MiB) and `MAX_ASSET_BYTES` with pictures; an insert that would pass the budget is refused with a notice and the text behaves as today. No size limit of our own. Compression (fonts compress well) is a follow-up, listed in technical debt.
4. **Resolution on open:** the session lists the document's fonts as a fourth face block `FontBlock::Document`, with host id `doc:<hash>`, family, style, index and kind taken from the `FontRef` and the bytes from the asset store (loaded at once, parsed once). `match_face` order: exact bytes in any block (installed copies first, so counts and the Remove prompt keep working), then a same-name Document face (FONT-SAME-NAME A: the file looks as authored), then the same-name faces of Curvyo, Included, This computer as before, else Missing. `FontBlock::Document` sorts last, so the three old blocks keep their order.
5. **"Installed"** for the Fonts tab: a face of the same family and style exists in Curvyo fonts, Included or This computer. A Document row is an installed row when so, else "not installed" (star and tags greyed, install icon instead of the trash). Install hands the embedded bytes to Add font's path as a dropped file, so its checks and archive handling run; no new host command.
6. **Sync:** `referenced_assets` returns image and font hashes; asset records, `insert_asset` and the background-stream asset path carry fonts unchanged (ADR 0031 decision 3, 0037). The receiving session lists the Document block again when the asset total or the used fonts change.
7. **Licence (FONT-LICENCE A, Proposal):** a font whose `fsType` is "restricted licence" (bit 1 = value 2) is not embedded; every other value is. Read by `curvyo_text_core::font_info`.
8. **Cost:** listing the document fonts walks the text nodes; it runs on a font change, an open and an asset arrival, never per frame (frame budget of `0092` criterion 13).

## Consequences

The container grows by the used font files (a few hundred KB each; CJK fonts are large). Files opened in an older build look as before. A text now draws in the embedded bytes instead of a same-named installed font with other bytes. Unused embedded fonts stay until a wipe (debt entry).
