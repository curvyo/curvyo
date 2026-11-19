# ADR 0076: The model download: a separate static Curvyo model host, pinned hashes, a device cache, and the one network use of the app

**Status:** Proposed (architect, 2026-11-19), `needs-customer` (network use and data leaving the machine, hosting and its cost: `CLAUDE.md` §3; spec criterion 63). The customer chose "on the device, downloaded once from a separate Curvyo model host, if possible" (Frage 274 B). Open for the customer: **Frage 280** (which host; default: bundling until a host exists, decision 6), `specs/0135-image-optimisation/adrs.md`. Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 63 to 65. Models: [ADR 0075](0075-on-device-image-models-runtime-and-crate.md). Amends `0029` criterion 35 (no network) as the spec states.

**In plain words (for the customer).** The first time you run an AI tool, Curvyo asks once whether it may download that model (with its size). The download is a plain file from a Curvyo address; your picture never goes anywhere. The file is checked against a fingerprint built into the app and kept on your computer. Without the internet everything else, and every saved AI result, keeps working.

## Context

Curvyo has no network use today apart from the maker's own sync relay and machine connector, which talk only to places the maker set up. A model download is the first request to a server the maker did not choose, so it must carry nothing about the maker or the picture, must be checkable, and must fail visibly. The browser build needs a host that answers cross-origin requests (CORS); the desktop webview fetches the same way.

## Options

**Who downloads**

| Option | Why it won or lost |
|---|---|
| **A. The frontend's `fetch` in both builds (chosen)** | One path for browser and desktop, as the relay WebSocket (ADR 0031); no Rust HTTP client and TLS stack added; the webview's CSP names the one host. |
| B. A Rust HTTP client in `curvyo-app` for the desktop | A new dependency tree for a single GET, and the browser still needs A. |

**Where the file is cached:** the desktop's data directory `models/<sha256>.onnx`, written by `curvyo-storage-io` through an app command (atomic temp file and rename), and IndexedDB in the browser (chosen; the split already used for material photos, ADR 0058, and fonts) against the browser's Cache Storage in both (evictable in the webview, a second storage rule).

**Host** (Frage 280; all serve the same pinned files, so integrity never depends on the host)

| Option | For | Against |
|---|---|---|
| A. **Own static host** `models.<curvyo domain>`: a public object-storage bucket with CORS behind the domain (Proposal: Cloudflare R2, whose egress is free) | What Frage 274 B asked for; immutable URLs; cost near zero at about 82 MB per full set per device | Needs a domain and a bucket the customer owns; uploading files at a release is an ops step |
| B. GitHub Releases of a `curvyo/models` repository | Free, no ops | Release downloads send no CORS headers: the browser build cannot fetch them. Lost. |
| C. A Hugging Face organisation `curvyo` | Free, CORS, common for models | A third party's servers and terms, not "a Curvyo host"; the maker's IP reaches them |
| D. **Bundling** (the earlier Frage 274 A): the files beside the app, fetched lazily same-origin in the web app and embedded in the desktop installer (the ADR 0028 §5 path) | No network at all on the desktop, no host, no cost | Installer and web deployment about +82 MB (+13 MB with FFDNet instead of SCUNet, ADR 0075) |

## Decision

1. **The request.** A plain `GET https://<host>/<model name>/<version>/<sha256>.onnx`, issued only after the consent line (spec criterion 63) for a model not on the device, with `credentials: "omit"`, `referrerPolicy: "no-referrer"`, `cache: "no-store"`, `redirect: "error"`, no query string, no custom header and no body (test: the captured request has a fixed URL and no body or cookie). Nothing is sent after it. It is the **only** request of the application to a server the maker did not configure; CSP `connect-src` (Tauri config and the web app's headers) gains exactly this host. The host, path prefix and the pinned list are constants in Rust (`MODELS`, ADR 0075 §2), read by the frontend through the wasm; the frontend holds no second list.
2. **Integrity.** The download is read with a cap of the pinned size; the bytes go to `curvyo-imageai-core::verify` (size and SHA-256) before they are stored, and again on every load. A mismatch stores nothing, removes any cached file of that hash and shows "The downloaded model did not match its checksum and was removed." with Try again (criterion 64). A partial or interrupted download is never stored; Try again restarts it (no resume, Proposal). TLS protects privacy in transit; the pinned hash, not the host, protects the bytes, so a compromised host can at most deny service.
3. **States** (criterion 65), owned by the frontend's model module and shown in the step row: not on the device ("Model not downloaded yet (<size> MB)."), downloading (progress, Cancel), host not reachable ("Model host not reachable. Everything else keeps working and stored results still burn." with Try again), checksum failure (decision 2), ready. No state blocks the UI or another step; stored results burn without the model (ADR 0077).
4. **Consent** is asked per model before its first download and remembered per device in the per-user preferences, never in the project (Proposal; criterion 63). A model removed from the cache asks again. A settings line lists downloaded models with their size and a Remove button (UX notes).
5. **Not synced.** Models are device-local: not in the project, not in the workspace sync, not in the ADR 0062 library stream. A second device burns stored results without them and downloads only to run a step itself.
6. **Host choice (Frage 280; its options A, B, C are this table's A, C, D; B lost).** Recommendation: **A**, as the customer asked. **Default if unanswered: D (bundling)**: the same code with the host constant set to the app's own origin, nothing on the network, no ops, no cost; the consent line then reads "Loads the <model name> model (<size> MB)." and no download state appears on the desktop. Moving from D to A later is the host constant, the CSP line and the consent text; no file format changes.

## Consequences

- With A: the customer owns a domain, a bucket and an upload step per model release; the host sees each downloader's IP address and the time of the download, nothing else. Recorded in `docs/technical-debt.md` as an operating duty, not a debt.
- With D: installer and web app grow by the model sizes; the desktop never touches the network for models.
- `0029` criterion 35 gains its one exception (spec "Changes to accepted specs"); a future request of any other kind needs its own ADR.
