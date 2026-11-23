# ADR 0076: The model download: on demand from a Curvyo origin behind a CDN, a pinned manifest in the app, a device cache, and the one network use of the app

**Status:** Proposed (architect, 2026-11-19), `needs-customer` (network use and data leaving the machine, hosting and its cost: `CLAUDE.md` §3; spec criterion 63). Customer decisions: Frage 274 B (on the device, downloaded from a separate Curvyo model host) and **Frage 280 A** (2026-11-19: "like digiKam: the model is downloaded on demand when it is needed; the app does not ship or preload it; put it on some CDN"). The bundling default is withdrawn. Open for the customer: **Frage 282** (which CDN provider; `specs/0135-image-optimisation/adrs.md`), needed before slice 3 ships, not before it is built. Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 63 to 65. Models: [ADR 0075](0075-on-device-image-models-runtime-and-crate.md). Amends `0029` criterion 35 (no network) as the spec states.

**In plain words (for the customer).** As in digiKam: Curvyo contains no AI model. The first time you run an AI tool, Curvyo asks whether it may download that one model (with its size), from `models.curvyo.org`. The download shows its progress, can be cancelled and picks up where it stopped. Your picture never goes anywhere. The file is checked against a fingerprint built into the app and kept on your computer. Without the internet everything else, and every saved AI result, keeps working.

## Context

Curvyo's only network use today goes to places the maker set up (sync relay, machine connector). A model download is the first request to a server the maker did not choose: it must carry nothing about the maker or the picture, be checkable, fail visibly, and cost nothing when the AI tools are not used. The browser build needs cross-origin headers (CORS) and survives browser storage eviction; the desktop webview fetches the same way but has a real data folder. Three files total about 82 MB (ADR 0075); SCUNet alone is 72 MB, so an interrupted download must resume.

## Options

**Host** (Frage 280; every option serves the same pinned files, so integrity never depends on the host)

| Option | Why it won or lost |
|---|---|
| **A. A Curvyo-owned origin bucket behind a CDN at `models.curvyo.org` (chosen, Frage 280 A)** | The customer's choice; `curvyo.org` is already registered (ADR 0013); immutable files suit CDN caching; a few cents a month. Costs: an account, a DNS record, an upload per model version (Consequences). |
| B. Hugging Face organisation `curvyo` | A third party's servers and terms; the maker's IP reaches them. Rejected by the customer. |
| C. GitHub Releases | No CORS headers on release downloads: the browser cannot fetch them. |
| D. Bundling with the app (the earlier default) | Rejected by the customer: +82 MB for every maker, loaded whether used or not. |

**Browser cache**

| Option | Why it won or lost |
|---|---|
| **A. IndexedDB, the file as 4 MiB chunk records (chosen)** | The store browser fonts (`browserFonts.ts`) and material photos (ADR 0058) already use, under the same `navigator.storage.persist()` request; chunks give resume without holding the file in memory while it downloads. |
| B. Origin Private File System | Fits large files, but writing needs a worker's sync access handle in Safari, and it is a second browser storage rule. Same quota and eviction as A, so no gain. |
| C. Cache Storage | `cache.put` refuses a 206 partial response, so no resume. |

**Desktop cache:** the app data folder, `models/<sha256>.onnx` (chosen), written by `curvyo-storage-io` through app commands, against IndexedDB in the webview (evictable and quota-bound on a machine that has a real disk).

## Decision

1. **The manifest is compiled into the app.** `MODELS` (ADR 0075 §2, generated from `curvyo-imageai-core/models.toml`) holds per model: name, version, size in bytes, SHA-256, licence, attribution line and **one URL**. It is part of the app's code, so it is as trustworthy as the app (installer signature, the web app's origin); no remote manifest, no signature key, no update check (a remote list would allow a silent model change, which criterion 64 forbids). One URL, not a list: the customer ruled out mirrors; a second origin is a code change. The frontend reads the list through the wasm and holds no second copy.
2. **URL scheme.** `https://models.curvyo.org/<name>/<version>/<sha256>.onnx`, for example `.../u2netp/1/<64 hex>.onnx`, plus `.../<name>/<version>/LICENSE.txt` beside it (decision 9). A published path is **immutable**: never overwritten, never re-used; a new conversion is a new version and a new hash. The host constant is `https://models.curvyo.org` (Proposal; ADR 0013's registered domain; UX draft 04 shows `models.curvyo.app` and follows this).
3. **On demand only.** No model request, cache read of model bytes, `HEAD` or update check happens at app start, at project open, or for any non-AI step (criterion 63). Showing an AI step checks only whether the file is present (a name lookup). Bytes are read only when Run starts, into the dedicated worker (ADR 0075 §3).
4. **The request.** `GET <url>` with `credentials: "omit"`, `referrerPolicy: "no-referrer"`, `cache: "no-store"` (we keep our own copy), `redirect: "error"`, no query, no body, and only the `Range` header on a resume. It is the **only** request of the application to a server the maker did not configure; `connect-src` in the Tauri config and the web app's CSP gains exactly this origin (test: the captured request has the fixed URL, no body, no cookie, at most a `Range` header).
5. **Resume and integrity.** The download streams into the partial file (desktop `models/<sha256>.part`, browser chunk records) with a cap at the pinned size. Cancel or a broken connection keeps the partial file; Run or Try again sends `Range: bytes=<stored>-` and appends a `206` answer whose `Content-Range` starts at `<stored>`; a `200` or any other start discards the partial file and starts at zero. When the size is reached, `curvyo-imageai-core::verify` checks size and SHA-256; only then is the file committed (desktop: rename; browser: a `complete` record). A mismatch deletes it and shows "The downloaded model did not match its checksum and was removed." with Try again (criterion 64). A partial file is never parsed. `Model::load` verifies again on every load. TLS gives privacy in transit; the pinned hash protects the bytes, so a compromised host or CDN can at most deny service.
6. **Consent** (criterion 63) is the consent line before a download, shown when a Run needs a model with neither a complete nor a partial file on the device. A partial file counts as consent given, so a resume does not ask again; a removed or evicted model asks again. No separate consent preference is stored (Proposal; simpler than remembering consent next to a file that may be gone).
7. **States** (criterion 65), owned by the frontend's model module, shown in the step row, never blocking the UI or another step:

   | State | Shown |
   |---|---|
   | not on the device | "Model not downloaded yet (<size> MB)." |
   | downloading | progress (bytes of total), Cancel |
   | partial (cancelled or interrupted) | "Download paused at <n> of <size> MB." with Resume (Proposal) |
   | offline, host unreachable, `5xx`, `403` | "Model host not reachable. Everything else keeps working and stored results still burn." with Try again |
   | `404` or `410` (withdrawn, decision 10) | "This model is no longer available. Stored results still burn." (Proposal) |
   | browser storage too small | "Not enough browser storage for the <name> model (<size> MB)." (Proposal) |
   | checksum failure | decision 5 |
   | ready | the Storage line with size and Remove |

   Offline is not detected separately: `navigator.onLine` is unreliable, a failed request is the signal.
8. **Desktop and browser.**
   - *Desktop (Tauri):* the webview's `fetch` downloads (same code as the browser); app commands `model_append(sha256, offset, bytes)` (refuses an offset that is not the current length), `model_commit(sha256)`, `model_read(sha256)` and `model_remove(sha256)` in `curvyo-storage-io` (atomic rename, ADR 0058 pattern). No quota, never evicted; Remove is the only deletion.
   - *Browser:* before the first chunk, `navigator.storage.estimate()`; if the free quota is below the size plus 16 MiB, the storage state of decision 7 shows and nothing is fetched. At the first Download the page calls `navigator.storage.persist()` (the font request, `fontStorageKeep.ts`, generalised); granted or refused, the download goes on. An evicted model reads as "not on the device" and asks again; stored results are in the project and are not affected.
9. **Licences and attribution.** The About dialog's licence list (desktop and web) shows each model from `models.toml` whether downloaded or not: name, version, upstream repository, copyright line, licence, "converted to ONNX by Curvyo, weights unchanged" (the Apache-2.0 §4 change notice) and the training data note (ADR 0075 §9). The host serves the upstream licence text, and any upstream `NOTICE`, as `LICENSE.txt` beside each file.
10. **Versioning and takedown.** A new model version is a new app release with a new pinned hash (criterion 64); old files stay online for older releases. A takedown (upstream licence change, legal request) deletes the file from the origin and purges the CDN; the next release drops it from `MODELS`; older apps show the `404` state; stored results keep burning (ADR 0077).
11. **Not synced.** Models are device-local: not in the project, the workspace sync or the ADR 0062 library stream.
12. **Host settings** (the runbook in `docs/technical-debt.md` lists them as the customer's operating duty):
    - origin: a bucket with public read of objects, no listing, writes only from the maintainer's account;
    - CORS: `Access-Control-Allow-Origin: *` (no credentials are sent), methods `GET, HEAD`, allowed request header `Range`, exposed headers `Content-Range, Content-Length, ETag`, `Access-Control-Max-Age: 86400`; this covers the web app and both Tauri origins (`tauri://localhost`, `http://tauri.localhost`);
    - `Cache-Control: public, max-age=31536000, immutable` on `.onnx` and `LICENSE.txt`, `Content-Type: application/octet-stream`, byte ranges on (`Accept-Ranges: bytes`), no compression or other transformation, no redirect;
    - access logs off or kept at most 30 days (the host sees IP address and time, nothing else);
    - a release-checklist step fetches each pinned URL and compares its SHA-256 before an app release that changes `MODELS`.

## Consequences

- No model is in the repository, the installer, the web app or the wasm; a maker who never uses an AI tool never contacts the host.
- The customer owns the account, DNS record, bucket, CDN setting and an upload per model version (operating duty, `docs/technical-debt.md`). Cost: storage of about 82 MB per version kept online and download traffic of up to 82 MB per device for the full set; on the recommended provider (Frage 282) traffic is not charged.
- If the host is lost, the AI tools cannot download; stored results, every other step and the job keep working. The fallback is a new release with a new host constant, not a bundled model.
- `0029` criterion 35 gains its one exception (spec "Changes to accepted specs"); a request of any other kind needs its own ADR.
- **For the PO and UX:** decision 7's paused, withdrawn and storage lines are new texts (Proposal); decision 6 removes the stored consent preference; draft 04's host name becomes `models.curvyo.org`.

## Note 2026-11-23: what slice 3 built (architect review; for the customer's acceptance with the PR)

**In plain words (for the customer).** The download works as described, from
`models.curvyo.org`, with no cookie, no referrer and nothing about your picture. Two parts are
not built yet: the desktop app keeps the models in its webview storage like the browser does (so
the system may clear them and they download again), and there is no About dialog yet listing the
models' licences. The three files are now about 10.5 MB in total, not 82 MB (ADR 0075 note
2026-11-23).

What changes against the text above:
- Context and Consequences: the three pinned files total 10,527,577 bytes; storage and traffic
  per device are about 10.5 MB, not 82 MB.
- Decision 4: the desktop CSP (`curvyo-app/tauri.conf.json`, `csp` and `devCsp`) gains
  `https://models.curvyo.org` in `connect-src`. The web app sets no CSP today (GitHub Pages sends
  none), so there is nothing to amend there; a future web CSP lists this origin.
- Decision 8 (desktop): not built. Both builds use the IndexedDB chunk store
  (`frontend/src/lib/modelStore.ts`); the four `curvyo-storage-io` commands are a debt
  (`docs/technical-debt.md`, "The desktop app caches models in the webview's IndexedDB").
- Decision 9: the data is served by the wasm (`ai_models_json`) and beside each file on the host
  (`LICENSE.txt`); the About dialog is a debt ("No About licence list exists yet").
- Decision 12: the host settings and upload steps are the runbook `docs/runbooks/ai-models.md`
  (Frage 282 default A, Cloudflare R2), not `docs/technical-debt.md`.
