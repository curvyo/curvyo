# Plan for library Git token

Implementer, 2026-11-26. Defaults: Frage 386 A (no key store: refuse), 387 A (`Authorization:
Bearer` to the source's own origin), 388 A (Lock mark only). ADR 0090 decisions 1 to 10. No new
dependency (`keyring`, `zeroize`, `ureq` are locked), no `format_version` change; `online-sources.toml`
gains the additive `token` flag (format 1).

## Affected crates/modules

- `curvyo-library-core`: `SourceEntry.token`, `Sources::set_token`, `Failure::TokenUnreadable`.
- `curvyo-storage-io`: `catalogue_token` (`AccessToken`, `load/save/forget_token`, `token_label`);
  `http` (`Transport::get_bearer`, `Authorized`, origin-pinned redirects for a token request).
- `curvyo-ui-core`: `library_token_text` (refusals by field, failure lines, fixed texts),
  `OnlineExtra`/`OnlineSourceView`/`BrowseGroup` token flags, the guide's Private repositories step;
  `curvyo-editor-wasm`: `library_token_words`.
- `curvyo-app`: `online_token` (key store opener, `sent`, `set_token`, `delete_token`), `online_state`
  (`add` with a token, token-aware list/template/thumbnail), commands `online_add` (optional token),
  `online_set_token`, `online_forget_token`, `online_remove` (file first, then key store).
- Frontend: `AddSourceForm` (uncontrolled token field), `TokenForm`, `OnlineRows` (Lock, `KeyRound`),
  `LibraryTab` (one inline form at a time, delete-failed line), browser group header Lock, `GuideView`.

## Tasks

- [x] 1. library-core: the `token` flag round-trips, defaults false, refused for Iconify and the ready
  entry; `Failure::TokenUnreadable` (AC 9, 17, 18)
- [x] 2. storage-io `catalogue_token`: parse bounds, redacted `Debug`, save/load/forget on the key-file
  double (AC 8, 9, 11)
- [x] 3. storage-io `http`: `Authorized`, header per request on the source's origin only, redirect to
  another origin refused, local-server tests record the header (AC 10)
- [x] 4. ui-core words: refusals under their field, the 404 hint in Add only, failure lines by the
  `token` bool, Lock and action flags, items stay under the failure line (AC 3, 4, 13, 15)
- [x] 5. guide: Private repositories step, "must be public" removed (AC 15, 16)
- [x] 6. app: Add with a token (key store first, GET, then save), Change/Add/Remove token, Remove order,
  flagged source without a readable token sends nothing, sentinel search (AC 2, 5, 6, 7, 9, 10, 11, 12)
- [x] 7. frontend: token field (no `value` attribute), Lock, `KeyRound` action and inline form, delete
  failed line, browser Lock, guide step, web build shows none of it (AC 1, 4 to 8, 14, 16)
- [x] 8. docs: `0152` criterion 33 text, design-system rows already in, spec Status, README row

## Validation

Rust tests above; frontend vitest for the field hygiene and the rows; the full gate of `CLAUDE.md`
section 7. Not verifiable here: the real OS key store and a live private repository (ADR 0090
decision 10: one manual check in the PR).
