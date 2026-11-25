# ADRs for the template library over the network

Architect, 2026-11-24. One new ADR; Fragen 271 B, 272 and 273 A are fixed; Frage 340
(TPL-SEARCH) answered 2026-11-24: B; Frage 334 (TPL-HOST) answered 2026-11-24: A. fontsource is
not part of this slice.

- [ADR 0085](../../docs/adr/0085-online-template-sources-read-only.md) (Proposed,
  `needs-customer`: network, a new crate and a new dependency):
  - `online-sources.toml` and the ready Curvyo entry at
    `https://raw.githubusercontent.com/curvyo/templates/main/` (criteria 1, 7; Frage 334 A,
    answered 2026-11-24);
  - GET-only requests from the desktop host on the maker's action only, to added sources only, with the CSP unchanged: no `raw.githubusercontent.com` or Iconify host in `connect-src`, the guard pins the ready entry to `raw.githubusercontent.com` (2);
  - the Online section in the desktop build only in this slice **(Proposal)**;
  - the Git catalogue format `curvyo-catalogue.json` with SHA-256 per file (6);
  - the hash check before reading, nothing executes, the font rule (3, 8, 9);
  - the Iconify endpoints, with licence per set (2, 7), and `/search` on Enter with the words
    only, to added Iconify sources only (10; Frage 340 B, answered 2026-11-24);
  - offline and errors per source (5);
  - Came from keys and derived credits (3, 4);
  - the new crate `curvyo-library-core`, the dependency `ureq` 3, and the ADR 0007 amendments.
- [ADR 0084](../../docs/adr/0084-document-library-index-read-only-sources-and-copy-import.md):
  a downloaded template is a read-only source; its import is the fragment of decision 6.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md): §1 to §4
  (credentials) unused in this slice (no source needs one); §9 licence per item kept.
- [ADR 0076](../../docs/adr/0076-model-download-host-and-the-one-network-use.md): request hygiene and
  "a failed request is the offline signal" reused; its one foreign origin stays the only one in the
  CSP, because these requests run in the host.
- [ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md): an Iconify icon goes
  through the SVG reader and its limits.
- `format_version`: none; the online keys are added to the format-58 record, which keeps unknown
  keys.

## Answered 2026-11-24

- **Frage 334 (TPL-HOST): A, answered 2026-11-24** (the architect had recommended B). The ready
  Curvyo catalogue is read from GitHub raw files of `curvyo/templates`; GitHub sees those
  requests. ADR 0085 decision 1 and its option table.
- **Frage 340 (TPL-SEARCH): B, answered 2026-11-24.** Enter in the library search sends the words
  to each added Iconify source's `/search`; nothing else is sent. ADR 0085 decisions 2 and 5.

## Notes for the PO and UX (ADR 0085 Consequences)

1. F1: no credential field in this slice.
2. F2: fetched lists and templates live for the session only; nothing is written but the sources
   list.
3. F4: credits are derived from the objects; criterion 4's "never removes it" would need a stored
   list and a format number. The PO decides the wording.
4. F6: Add sends one request to check the address; F7: "Offline" is per source, after a
   connection failure of that source's own request; there is no app-wide online state.
5. A changed template on a moving branch fails its hash check with "does not match its catalogue
   entry"; Refresh fixes it.
6. The public Iconify API states no terms; the source line says so **(Proposal)**.
7. Criterion 10: the search text is trimmed and cut at 100 characters **(Proposal)**; an empty
   text sends nothing; results replace the source's previous ones for the session.
8. The `0137` spec says the Online section comes to both builds; ADR 0085 keeps it desktop only in
   this slice **(Proposal)**, since the browser would need `fetch`, CORS and a web-build CSP. The
   PO aligns the wording or raises a question.
9. The `curvyo/templates` repository (files, index script, CI check) must exist before the ready
   entry works; that is lead work outside this checkout.

## Build notes (implementer, 2026-11-24)

Where the build differs from or settles a point of ADR 0085; the architect confirms or amends.

1. **Crate edges.** `curvyo-ui-core → curvyo-library-core` (the Online rows format catalogue
   entries, sets and failures) and `curvyo-storage-io → curvyo-library-core`; `curvyo-editor-wasm`
   needs no edge (it reads the rows' words through `curvyo-ui-core`).
2. **Hash check in the host.** `library-core::verify` runs in `curvyo-storage-io` (`fetch_template`)
   before the bytes are returned or read; the wasm never sees an unchecked template. The result is
   the same as decision 4, one step earlier.
3. **Fonts.** The rule is the index's: a template whose index lists a font with a licence outside
   the allow list of `library-core::font_licence_is_known` (or none) leaves all embedded font files
   behind (`Fragment::without_fonts`); the `fsType` check of ADR 0047 §7 is not repeated here.
4. **Roots.** `ureq` 3 runs on `rustls-no-provider` (the locked `ring`) and is given the operating
   system's roots from `rustls-native-certs` as `RootCerts::Specific`, read on the first request; no
   bundled root list, no `deny.toml` exception.
5. **Came from words.** `document` holds the source's name and `page` the template's name (or
   "Set › icon"), so the existing line "From {document} › {page}" reads "From Curvyo catalogue › Box";
   the licence and author are the extra keys. Open source is not offered for an online item.
6. **Ready entry.** Add on the ready Curvyo catalogue sends nothing (UX notes 2: it joins the list
   and is not opened); the first request is the expand. Adding any other address sends the one
   validating request.
7. **Browser.** The Online group shows one line "Not available in the browser. Use the desktop app."
   and sends nothing (`invoke` has no `online_*` command there).
8. **Steps.** "Load template" is one step (`load_online_template`; `load_template` already names
   the document template of `0019`), "Import icon" one step (`import_icon`) through the SVG reader.

**Architect review, 2026-11-24.** Build notes 1 to 6 are accepted and written into ADR 0085
(decisions 1, 2, 4, 7 and 8); note 3 relies on the catalogue's index script refusing a template
that embeds a font it does not list, which joins the `curvyo/templates` task of note 9 above.
