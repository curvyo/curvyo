# ADR 0085: Online template sources: a Git catalogue as files over HTTPS and an Iconify-compatible API, read only, fetched by the desktop host on demand

**Status:** Proposed 2026-11-24 by the architect, `needs-customer` (network: requests leave the machine; a new per-user file; a new crate and a new dependency; amends [ADR 0007](0007-asset-library-connector-and-credential-storage.md) §5, §6, §8 and §14 for the read path). Frage 334 (TPL-HOST) answered 2026-11-24: A, GitHub raw files of `curvyo/templates` (decision 1); Frage 340 (TPL-SEARCH) answered 2026-11-24: B, Enter also searches the added Iconify sources (decision 5). Spec: [`0138`](../../specs/0138-template-library/specification.md) criteria 1 to 10, Fragen 271 B, 272, 273 A. Builds on [ADR 0084](0084-document-library-index-read-only-sources-and-copy-import.md) (read-only source, fragment, Came from), [ADR 0076](0076-model-download-host-and-the-one-network-use.md) (request hygiene, failed request as the offline signal), [ADR 0018](0018-svg-import-crate-parser-and-limits.md) (hostile SVG guard).

**In plain words (for the customer).** You add an online source by its address: a template catalogue (a plain Git repository of folders, read as files) or an icon server that speaks the Iconify API. Curvyo asks it for anything only when you add it, open it, press Refresh, load something or press Enter in the search, never in the background or while you type, and the request carries nothing about you or your project. Enter sends only the words you typed, and only to the Iconify sources you added. The ready Curvyo catalogue is read from GitHub's raw files of the `curvyo/templates` repository, so GitHub sees those requests. Each template's licence and author are shown before you load it. A template is checked against the fingerprint in the catalogue before it is read, and it can never run code. Without the internet the online part says "Offline" and everything else works. Nothing is kept on disk except the list of sources you added; no passwords or tokens exist in this slice.

## Context

ADR 0007 (accepted) put the pure protocols in `curvyo-library-core` and HTTP in `curvyo-storage-io` with `reqwest`, planned two traits (`AssetSource`, `AssetSink`), a disk cache, and a `gitoxide` clone for Git because Git was also the push target. This slice is narrower. It only reads, it has no credentials (both kinds are public; paid catalogues are Later), its requests run only in the desktop build (the browser has the Library tab since Frage 341 C, ADR 0084 decision 10, but not its Online section in this slice, decision 2), it fetches only on the maker's action (Frage 273 A; Enter in the search counts as one, Frage 340 B), and Curvyo hosts no upload server (Frage 272). The Tauri CSP lists exactly one foreign origin, `models.curvyo.org` (ADR 0076 decision 4). The Iconify docs (read 2026-11-24) state licence and author per icon set (`/collections`: `license {title, spdx, url}`, `author {name, url}`). They state no terms, rate limits or CORS policy for the public API. Its `/search` endpoint takes `query` and `limit` (and optional filters this slice does not send) and answers icon names as `prefix:name` with the matching sets' info.

## Options

**Reading a Git catalogue**

| Option | Why it won or lost |
|---|---|
| **A. Files over HTTPS at a base address (the repository's raw files or any static host) (chosen)** | A few GETs, no Git library, no clone; any forge or a bucket serves it; the index's hashes make the host untrusted. |
| B. A `gitoxide` clone (ADR 0007 §14) | Downloads the history to read a few files; a working copy per source on disk; the young part of `gitoxide` was only needed for push. Kept for the push target later. |
| C. A forge's REST API | One vendor, and GitHub allows 60 unauthenticated requests an hour. |

**Where the requests run**

| Option | Why it won or lost |
|---|---|
| **A. The desktop host, `curvyo-storage-io::http` (chosen)** | The CSP stays exact; catalogue hosts need no CORS headers; a later credential never enters page code (ADR 0007 §4); the host can refuse any address that is not an added source. |
| B. The webview's `fetch` | Needs `connect-src https:` (every origin open to page code) and CORS on every catalogue host. |

**Dispatch between the two kinds**

| Option | Why it won or lost |
|---|---|
| **A. A two-variant `SourceKind` enum with pure request builders and response parsers (chosen)** | Sans-I/O functions need no trait object; one `match`. |
| B. ADR 0007 §6's `AssetSource` trait | Its point was the pairing with `AssetSink` and a local-folder library, neither of which exists yet; it returns with them. |

**Keeping fetched data**

| Option | Why it won or lost |
|---|---|
| **A. In memory for the session only (chosen, UX F2)** | Nothing about browsing is written to disk; the spec puts offline catalogues out of scope. |
| B. The disk cache of ADR 0007 §8 | Offline browsing nobody asked for, plus cache eviction and licence questions. |

**Crate**

| Option | Why it won or lost |
|---|---|
| **A. Create `curvyo-library-core` in ADR 0011's slot ("asset catalogs … the asset-provider protocols as pure request/response functions") (chosen)** | Exactly the slot's text; wasm-buildable; testable with recorded responses. |
| B. Modules in `curvyo-ui-core` | Protocol parsing and hash checks are not view logic. |

**Host of the ready Curvyo catalogue (Frage 334, answered 2026-11-24)**

| Option | Why it won or lost |
|---|---|
| **A. GitHub raw files of `curvyo/templates` (chosen by the customer)** | Free, no publish step; the catalogue format needs only static files, and the hashes make the host untrusted. Cost: GitHub sees the maker's requests. |
| B. `templates.curvyo.org` on the bucket and CDN of ADR 0076 | Only our host sees requests; a few cents a month and one publish step. The architect's recommendation; not chosen. |
| C. No ready entry | Makers would have to know an address. |

**Search over the network (Frage 340, answered 2026-11-24)**

| Option | Why it won or lost |
|---|---|
| **B. Enter sends the typed words to the Iconify sources the maker added, `/search` (chosen by the customer)** | Finds icons in sets nobody has expanded; one request per source per Enter, a maker action as Frage 273 A asks. Cost: the source sees the words. |
| A. Search filters only what was fetched | Sends nothing, but an icon is found only after its set was opened. |

## Decision

1. **Sources file.** `online-sources.toml` in the app data folder, `format = 1` (the `recent-workspaces.toml` rules). Each `[[source]]` holds `kind` (`git` or `iconify`), `address` (an `https` URL; a Git address ends in `/`), `name` and `added`. It is machine-local and not synced **(Proposal)**, and holds no credential. **Add** sends one validating GET (the index, or `/collections`) as part of the maker's action (UX F6); **Remove** deletes the entry. The Curvyo catalogue's address is a constant in the app, shown as a ready entry, and nothing is sent until Add.
   - **Ready entry (Frage 334 A, answered 2026-11-24):** kind `git`, address `https://raw.githubusercontent.com/curvyo/templates/main/` (branch `main`, **Proposal**). The `curvyo/templates` repository holds the files of decision 3; its own CI runs the index script and refuses a commit whose index does not match its files, so `main` is always consistent. GitHub's raw CDN may serve the index and a file from two commits for a few minutes after a push; that is the hash mismatch of decision 4, and Refresh fixes it.
   - **Allowed host:** the guard of decision 2 accepts the ready entry only at host `raw.githubusercontent.com`, and a redirect from it to another host is refused **(Proposal)**. The repository is public, so no token is sent.
2. **Requests.**
   - **Shape:** GET only, `https` only, no cookies, no `Referer`, no query beyond the protocol's, and `User-Agent: Curvyo` without a version **(Proposal)**. At most 3 redirects, each `https`. Timeouts are 10 s to connect and 60 s in total.
   - **When:** only on Add, on expanding a source or an icon set, on Refresh, on Load, for the pictures of visible rows of an expanded source, and on Enter in the library search to the added Iconify sources (decision 5, Frage 340 B). Never at start, while typing, on hover or tab change, and nothing in the background (criterion 2).
   - **Guard:** the host command takes a source id and a request built by `library-core`, never a free URL, and refuses an address that is not in `online-sources.toml` (or the ready entry being added). The CSP does not change: the requests run in the Rust host, not in the webview, so `connect-src` keeps `models.curvyo.org` as its one foreign origin and neither `raw.githubusercontent.com` nor any Iconify host is added.
   - **Builds:** the Online section exists only in the desktop build in this slice **(Proposal)**. In the browser build the Library tab (ADR 0084 decision 10) has no Online section, because its requests would need the page's `fetch`, the hosts' CORS headers and a `connect-src` for the web build: a separate decision.
   - **Dependency:** `ureq` 3 (MIT OR Apache-2.0), with `rustls` on the already locked `ring` provider and the operating system's trust roots (`rustls-platform-verifier`). It replaces ADR 0007 §5's `reqwest`: blocking GETs from blocking host commands need no async stack. The PR checks it with `cargo deny`.
3. **Git catalogue format** (criterion 6), `catalogue_format = 1`:
   - Files: `curvyo-catalogue.json` at the base address, and per template `<path>/template.curvyo`, `<path>/thumbnail.png` and `<path>/template.toml`. `template.toml` is the authors' source; Curvyo never reads it. A script in the catalogue's own repository builds the index from it, and Curvyo writes nothing to a catalogue.
   - Index fields: `catalogue_format`, `name`, `templates[]` with `path`, `name`, `author`, `tags`, `licence {spdx, name, url}`, `credit` (required when the licence needs attribution; the script refuses CC-BY-* without it), `fonts [{family, style, spdx}]`, `size`, `sha256`, `thumbnail_sha256` and `format_version`.
   - Hashes: SHA-256 of the exact file bytes, lower-case hex.
   - `path`: 1 to 4 segments of `[a-z0-9][a-z0-9._-]*`, no `..`, at most 200 characters.
   - Limits: index 4 MiB and 10,000 templates, picture 1 MiB PNG, template 64 MiB **(Proposal)**.
   - A newer `catalogue_format` shows "Made for a newer Curvyo". A template's newer `format_version` is shown before any download. A missing licence shows "Licence unknown", never as free (ADR 0007 §9).
4. **Loading a Git template.**
   - The template's bytes are checked for size and SHA-256 against the index in the wasm (`library-core::verify`) before anything parses them. On a mismatch: "The template on the source does not match its catalogue entry. Nothing was loaded." (Refresh fetches the index again; a moving branch can cause this.)
   - The checked bytes open as a read-only source (ADR 0084 decision 5), so pages, element picks and the import are those of the document library. Load imports every page, one step "Load template" (UX F3).
   - **Nothing executes** (criterion 8): a template is document data; extensions are compiled in (ADR 0034), and generated objects are drawn from their stored outline (ADR 0080).
   - **Fonts** (criterion 9): an embedded font is kept only when the index lists its family with a known licence and its `fsType` allows embedding (ADR 0047 §7). Otherwise it is dropped from the fragment before insert, the text falls back by `0107`, and the row says "Font not included".
5. **Iconify-compatible API** at a base address.
   - **Endpoints used:** `/collections` (sets with name, total, author, licence, samples), `/collection?prefix=<p>` (icon names) and `/<prefix>.json?icons=<a,b,…>` (icon bodies, batched per screen of visible icons, at most 100 per request). The SVG is built locally (`library-core::iconify_svg`).
   - **Search (Frage 340 B, answered 2026-11-24; criterion 10):** Enter in the library search sends `GET /search?query=<words>&limit=64` to every Iconify source in `online-sources.toml`, once per Enter, in parallel, and to nothing else: never to a Git catalogue, the ready Curvyo entry or a source not added. **What is sent:** the words only: the search text trimmed, inner whitespace collapsed to one space, at most 100 characters **(Proposal)**, percent-encoded, plus the fixed `limit`. No prefix, locale, page or document data, and no identifier beyond the request shape of decision 2. An empty text sends nothing. Typing sends nothing and filters only what is already fetched, as before. The request is built by `library-core::iconify_search(words)` and parsed by its parser (`icons` as `prefix:name`, at most the response limit of this decision). The bodies of the visible results then come by the `/<prefix>.json?icons=` request above. A result's licence and author are those of its set, from the response's set info or the fetched `/collections`. Without either, the result reads "Licence unknown". Results replace that source's previous results, live for the session only, and a failure shows on that source by decision 6.
   - **Licence per item** is its set's licence and author, as Iconify states them per set.
   - **Import:** an icon goes in through the SVG reader of `0024` with ADR 0018's limits, as a plain copy with Came from (UX F5). There is no hash: TLS and the SVG guard are the protection. Responses are at most 8 MiB.
   - **Terms:** the public API states none, so the source's terms line reads "No terms stated by this source" **(Proposal)**. Adding it remains the maker's choice.
6. **Offline and errors.** A failed request is the signal (ADR 0076 decision 7); nothing probes. Each failure is shown only on the source it hit:

   | Failure | Shown on that source |
   |---|---|
   | connection level (DNS, connect, timeout, TLS) | "Offline" |
   | `404` on the index | "No Git catalogue at this address." |
   | another HTTP status | "The server answered <code>." |
   | unparsable index | "No Git catalogue at this address." |
   | newer format, hash mismatch, too large | its own line |

   Other sources keep their state until their own request (UX F7: the app has no global online state). Local sources, drawing and saving are never affected, and loaded templates stay in the document (criterion 5).
7. **Came from for online items** adds keys to ADR 0084 decision 7's map: `address` (the template URL, or `<api base>#<prefix>:<name>`), `licence {spdx, name, url}`, `author` and `credit`. `document` holds the template or set name, `page` the page or icon name. The File tab's Credits block is derived from the records in the tree (UX F4); no root list is stored. **No `format_version` change:** format-57 builds keep unknown keys of the record (ADR 0084 decision 7). If the product owner keeps criterion 4 literal (a credit outlives its object), a stored credits list needs the next free number.
8. **`curvyo-library-core`** (new crate; `#![forbid(unsafe_code)]`; built for `wasm32-unknown-unknown`) holds: the sources file text, the catalogue index parse and checks, the Iconify request builders and parsers, `iconify_svg`, `verify` and the error kinds. Dependencies `serde`, `serde_json`, `toml`, `sha2` and `thiserror`, all locked. The edges `storage-io → library-core` (ADR 0011) and `editor-wasm → library-core` (new: the wasm parses the bytes the host returns). Golden tests use recorded responses in `tests/fixtures/` and no network.
9. **ADR 0007 for the read path:** §5 `ureq` instead of `reqwest`; §6 the traits wait for `AssetSink` or a local library; §8 no disk cache; §14 Git is read as files, and a clone stays the push design. §1 to §4 (credentials), §9 (licence per item) and §10 to §16 (upload) are unchanged; upload stays out of scope (Frage 272).

## Consequences

- A source's host sees the maker's IP address, the time and which index, sets, pictures and templates were fetched; that is inherent in adding it. For the ready Curvyo catalogue that host is GitHub (Frage 334 A). An Iconify source also sees the search words of each Enter (Frage 340 B).
- GitHub states no limit for unauthenticated raw reads that we rely on; a refusal shows as "The server answered <code>." on that source. Moving the catalogue to an own host later changes only the constant and the guard's host, not the format.
- The `curvyo/templates` repository with its index script and CI check must exist before the ready entry can work; that is a task for the lead, outside this checkout.
- A catalogue author needs a static host and our index script; any Git forge's raw files work, because the desktop host needs no CORS.
- Without credentials, private catalogues cannot be read yet; ADR 0007 §1 to §4 apply unchanged when a story asks.
- The public Iconify API's terms and limits are unknown to us; the PR checks it with one live request before release, and its endpoints are pinned by recorded fixtures.
- **For the UX notes:** no credential field (F1); session-only data (F2); "Offline" is per source on a connection failure (F7); credits are derived (F4); adding sends one request (F6); the form of an Iconify source says that Enter sends the words to it; the browser's Library tab has no Online section in this slice.
