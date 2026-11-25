# Template library over the network: browse and load templates from sources on the net that I added

Status: Done (PR #478; Ready 2026-11-24; ADR 0085 accepted with PR #478; Fragen 334 A and 340 B answered 2026-11-24)
Priority: Could
Origin: Customer (2026-11-19, translated): a template library like the document library, but loadable over the network; (first wish: "we must also build offering templates over the network", moved out of scope on 2026-11-19, see Out of scope); maybe connect svgrepo.com, fontrepo.com, 1001fonts etc. legally: "check licences, terms and APIs". Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker who starts a new job I want to browse ready-made templates (a box, a sign, a font specimen, an icon set) from sources on the internet next to my own documents, load one with a click, so that I do not start from a blank page, and so that the sources I use are ones I chose.

## What exists and where this fits

| Existing | Relation |
|---|---|
| `R-ASSET-001` to `-003` | save a template, browse it, add more local libraries. Not built yet. This entry's browse surface is the one of `0137`. |
| `R-ASSET-004` | connect to a third-party asset service with credentials the maker supplies; first target Iconify; **we run no accounts, payments or subscriptions**. Every source below is a connector under this rule. |
| `R-ASSET-005` | upload to a service the maker already has write access to; first target a git forge. Offering is out of scope for now (Frage 272, see Out of scope); the catalogue format below is the one a Git catalogue is read in. |
| `0137` document library | the same surface for local documents; a template is a copy import with Came from info. |
| `0075`, `0107`, R-FONT note ("for fonts we must still build something") | the font source is an open research item; the findings below are its first data. |
| `0096`, `0105`, `0116` | sync, sharing and library sync run between the maker's **own** devices or invited people; this entry is public or semi-public content. No overlap, no shared server contract. |
| `R-SYS-007` | no project data leaves the machine without an active choice; fetching a template sends nothing about the project; the maker adds each source on purpose. |

## Research: the named sites (2026-11-19, web fetch and search; no assumption beyond what is quoted)

| Source | Finding | Consequence |
|---|---|---|
| svgrepo.com | Licence page: "Unless an individual icon page indicates otherwise, all icons and vectors distributed on this website are subject to this license" (the SVG Repo License); under it "You don't need to give attribution to SVG Repo or any author"; use "as part of a commercial project" is allowed, reselling as they are is not. Per-icon pages can instead carry CC0, MIT, GPL, Apache, MPL, SIL OFL, CC BY, CC BY-SA, CC BY-NC or a "Logo License" limited by the copyright holder; CC BY needs credit, link and change notice; MIT, Apache etc. need the licence text kept. The licence page describes **no API** (the footer's "Icon API" points to svgapi.com, a separate site; its terms were not read). It states **no policy on bulk or automated download** and warns against redistributing "in a way similar to the SVG Repo website". Its Terms page returned HTTP 429 and was not read. | Per-item licence data exists but is only on each icon page. A connector would have to scrape pages: allowed or not is unknown. **Do not connect** without asking the operator in writing; embedded in-app browsing of svgrepo.com in a web view (the maker's own browser use) is outside this product. |
| 1001fonts.com | Licensing is per font: "Free For Commercial Use" (FFC) and "Free For Personal Use" (FFP, commercial use needs the author's prior written permission), plus others. The site's Terms: "You may not modify, reproduce, republish or distribute in any way the source code of this website"; "You may not link to any embedded files like images and zip-files found on this website within referring web-sites" (and "other software applications (illegal hotlinking)"); "Commercial use of the character map images and waterfall images is prohibited". The Terms do not mention bots or scraping. **No API found.** The "general font usage terms" document was not read. | Hotlinking from an app is forbidden by the ToS text; a connector cannot legally fetch files from their server inside the app without permission. A maker may download a font in his own browser and add it with Add font (`0075`). **Do not connect.** |
| fontrepo.com | Home page: 37,065 fonts, previews, TTF downloads, "License terms vary by font; check the author's terms before personal or commercial use"; filter by licence type (100% free, personal use, public domain / GPL / OFL); links "Licensing, Sources, Contact"; icons point to SVGRepo. Its Licensing page URL returned 404; **no API found**, no operator, terms or source list read. | Cannot be assessed: no terms on ownership or automation were found. **Do not connect.** |
| fontsource.org | "A collection of open-source fonts" self-hosted as npm packages; the introduction page names no licences and no API. | Licence data per package lives in the package (not read). Candidate only after the architect reads the package licences (no assumption: Google Fonts' licences are OFL or Apache, but this was not verified here). |
| Iconify (existing R-ASSET-004 target) | Chosen by the customer earlier; not re-checked in this research. | Unchanged. |
| General | Font and icon sites license per item. A connector has to carry the licence text and the author per item and show it before use (R-FONT-008 already asks the same of shipped fonts). | Criterion 6. |

What could not be verified: the full ToS of svgrepo.com, the "general font usage terms" of 1001fonts, any terms of fontrepo.com, any operator reply. **None of these three should be connected until the operator permits it in writing or exposes a documented API with terms.**

## Acceptance criteria (as far as clear)

1. Given the library surface of `0137`, then it has a section "Online" listing the sources the maker has added; with none added it lists nothing and sends nothing (R-SYS-007). A source is added by its address on purpose, never by default (adding sends one request to check the address, and the form says so; F6, **Proposal**): any **Iconify-compatible API** or any **Git catalogue** (criterion 6) the maker knows. The Curvyo catalogue (criterion 7), read from GitHub raw files of `curvyo/templates` (Frage 334 A), appears as a ready entry with an Add button, not as an added source **(Proposal)**.
2. Given a source, then it is one connector of `R-ASSET-004`: a read-only catalogue (names, tags, thumbnails, licence per item, size) fetched only when the maker opens that source's list or presses Refresh, over TLS, with no project data and no identifier of the maker in the request (Frage 273 A). Nothing is fetched in the background. Fetched lists and downloaded templates live for the session only; nothing is written but the sources list (`online-sources.toml`), there is no offline cache (F2, **Proposal**, ADR 0085). No credential field exists in this slice (F1; no source needs one).
3. Given a template, then its row shows its name, author, licence (name and link) and size before loading; Load copies every page of the template into the open document as new pages, a plain copy with Came from info (`0137` criteria 4 to 6) where the Came from record carries the address and licence instead of a document name; one undo step "Load template" (F3, **Proposal**); a template also expands to its pages like a local document. An Iconify icon is imported as a path through the SVG reader (`0024`, ADR 0018), 24 mm wide, at the drop point or centred, one undo step "Import icon" (F5, **Proposal**).
4. Given a licence that needs credit (CC BY and similar), then the credit text is kept in the Came from record of the loaded objects and the document's "Credits" list (in the File tab) is **derived from the objects that carry it** (F4, **Proposal**, ADR 0085): while an object with a credit is in the document its credit is listed, and the app never removes a credit from an object; deleting the object removes its credit from the list. No separate credit list is stored (that would need its own format).
5. Given no network, then an "Online" source whose own request failed to connect shows "Offline" and nothing else is affected (the state is per source, after a connection failure of that source's request; there is no app-wide online state; F7, **Proposal**, ADR 0085, which replaces the UX note F7 wording); local libraries, drawing and saving work (R-SYS-002, R-COLLAB-005); templates already loaded stay in the document.
6. Given a **Git catalogue**, then it is a plain repository of folders, each holding a `.curvyo` file, a thumbnail and a `template.toml` (name, author, tags, licence), plus an index file that lists every template with its hash. Anyone may host one; Curvyo reads it and writes nothing to it.
7. Given the first slice, then it ships two kinds of source (Frage 271 B): **a Curvyo catalogue** (the Git repository `curvyo/templates` with permissively licensed templates, the format of criterion 6, read from its GitHub raw files, Frage 334 A) and **Iconify** (the connector of `R-ASSET-004`, any Iconify-compatible API address); the three sites of the research (svgrepo.com, fontrepo.com, 1001fonts.com) stay unconnected.
8. Given a downloaded template, then it is a `.curvyo` file opened with the same reader as any file (damaged or newer-version files are refused as in `0001` criterion 7); its SHA-256 is checked against the catalogue's index before it is read (a changed template on a moving branch fails with "does not match its catalogue entry"; Refresh fixes it); nothing executes (no scripts, no extension code from a template).
9. Given a template with fonts, then fonts that are not embeddable or whose licence is unknown are not fetched; the text falls back as in `0107` and the row says "Font not included" as its third line (F8, **Proposal**).
10. Given a search in the library (Frage 340 B, answered 2026-11-24), then typing filters live only what was already fetched and sends nothing; **Enter** in the search field also sends the typed words to every **Iconify source the maker added** (only those; never to a Git catalogue or the Curvyo catalogue), once per Enter, over TLS, with no project data and no identifier of the maker; results join the icon grid of that source. It is a maker action like opening a list (Frage 273 A): nothing is sent while typing, in the background or on a source the maker has not added; the form of an Iconify source says that Enter sends the words to it.

## Out of scope

- Accounts, ratings, comments, payments, a moderated marketplace of our own (`R-ASSET-004`: we operate none; no extension marketplace, `0065`).
- Syncing the online catalogue offline for every source; background refresh (Frage 273 A: nothing in the background).
- **Offering templates over the network (customer, 2026-11-19, Frage 272):** Curvyo offers nothing itself for now: no publish button, no catalogue or server of ours that others upload to, no accounts. Curvyo is free software; whoever wants to offer a catalogue hosts their own server service (a Git repository or an Iconify-compatible API) and anyone may run their own library. Makers add such sources by address (criterion 1). The customer may become a provider himself later.
- Connecting the three sites above (see research), scraping any site, mirroring any site's files.
- Machine profiles and material records as templates (`0116`, `0028`).

## Decisions

Customer, 2026-11-19:

- **Frage 271 (TPL-SOURCES): B.** The first slice connects our own Curvyo catalogue (Git) and Iconify. svgrepo.com, fontrepo.com and 1001fonts.com stay unconnected (research above).
- **Frage 272 (TPL-OFFER): A, or an Iconify-compatible API, but Curvyo offers nothing itself for now.** Offering is out of scope (Out of scope); sources are any Iconify-compatible API or Git catalogue the maker adds. The licence display and `R-SYS-007` rules stay (criteria 1 to 4).
- **Frage 273 (TPL-FETCH): A.** Only when the maker opens a source's list or presses Refresh (criterion 2).

Customer, answered 2026-11-24:

- **Frage 334 (TPL-HOST): A, answered 2026-11-24.** The ready Curvyo catalogue is read from GitHub raw files of the `curvyo/templates` repository (criteria 1 and 7); no `templates.curvyo.org` host. GitHub sees those requests.
- **Frage 340 (TPL-SEARCH): B, answered 2026-11-24.** Enter in the library search also sends the words to the Iconify sources the maker added, only on Enter (criterion 10); consistent with Frage 273 A.

## Open, with defaults (customer may veto at the demo)

- **ADR 0085** (network, new crate `curvyo-library-core`, new dependency `ureq` 3) is Proposed, `needs-customer`; accepted with the slice PR.
- The public Iconify API states no terms; the source line says so (**Proposal**). fontsource is not part of this slice.

## Spec deltas of 2026-11-24

UX notes F1 to F8 and ADR 0085's notes are taken into the criteria above as **(Proposal)**; none changes a decision of 2026-11-19 (Fragen 271 to 273). F6 (one request on Add) stays inside Frage 273 A: the maker's own action, to a source the maker named. No `format_version` change of its own: the online keys are added to the format-58 record of `0137`.

## Later

- Paid catalogues may come later; details later, no design now.

## UX notes

See `ux-notes.md` (ux-engineer, 2026-11-24), on top of `../0137-document-library/ux-notes.md`: the "Online" section of the Library tab, the Curvyo catalogue ready entry, Add source, fetch on expand and Refresh, Git template rows with the licence line, Iconify sets and icon grid, Load, offline state, Came from and the Credits block; proposals Q1 to Q18, spec deltas F1 to F8 (accepted above), Frage 340 (TPL-SEARCH, answered B). Draft: `docs/design-drafts/2026-11-library/03-online-section.html`.

## Links

Requirements: R-ASSET-007, R-ASSET-008 (offering, now Later); R-ASSET-004, R-SYS-007, R-FONT-008
ADRs: `adrs.md`, ADR 0085
Depends on: `0137`; the asset-connector slice (architect's ADR, credentials)
Related: `0075`, `0107`, `0096`, `0105`, `0116`
PR: none yet
