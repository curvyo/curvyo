# ADR 0090: Git catalogue access token: kept in the operating system's key store by `curvyo-storage-io`, sent as a Bearer header only to the catalogue's own origin

**Status:** Proposed 2026-11-26 by the architect. Not `needs-customer`: the customer chose Frage 376 A (a token per Git catalogue in the OS key store, desktop only); no new crate, no new dependency, no document `format_version` change; one additive field in `online-sources.toml` (`format` stays 1). Spec: [`0157`](../../specs/0157-library-git-token/specification.md) criteria 1 to 18; Fragen 386 A and 387 A at their defaults (decisions 3 and 5). Amends [ADR 0085](0085-online-template-sources-read-only.md) decisions 1 and 2 and [ADR 0007](0007-asset-library-connector-and-credential-storage.md) §1 for this path. Builds on [ADR 0031](0031-sync-engine-relay-protocol-and-transport.md) decision 12 (the credential store module) and [ADR 0071](0071-connector-api-ctl-and-maintenance-mode.md) (a keychain-only credential row).

**In plain words (for the customer).** The token goes into the same system key store Curvyo already uses for its sharing keys (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux). `online-sources.toml` only notes "this source has a token", never the token. The token is sent in one standard header and only to the exact address the catalogue lives at; if that server points elsewhere, Curvyo stops. Without a key store, a token cannot be added, and nothing falls back to a file. The browser version has no online sources, so no token either.

## Context

ADR 0085 reads a Git catalogue as files over HTTPS: `curvyo-library-core` builds a `Request` (URL plus `pins_host`), `curvyo-storage-io::http::UreqTransport` sends it with redirects followed by hand (`max_redirects(0)`, at most 3 hops), `storage-io::online_fetch` checks and parses, and `curvyo-app::online_state` holds the session data. The host commands take a source key (`online:<address>`), never a URL. The credential store already exists: `curvyo-storage-io::credentials::CredentialStore` (ADR 0007 §1, ADR 0031 decision 12) on `keyring` 3.6.3 (MIT OR Apache-2.0; features `sync-secret-service` over libdbus, `apple-native`, `windows-native`; already locked and passing `cargo deny`), with an encrypted key-file variant used as the test double. `CredentialStore::keychain(slot)` probes once and answers `NoKeychain` when no Secret Service answers (no D-Bus session, no daemon, a refused unlock).

## Options

**Which crate holds the key store access**

| Option | Why it won or lost |
|---|---|
| **A. The existing `curvyo-storage-io::credentials` module (chosen)** | ADR 0007 §5 put credentials there; `keyring` is already its dependency; no ADR-0011 change. |
| B. A new `curvyo-keystore-io` crate | A second wrapper around the same `keyring` entries; a crate for one function. |
| C. `curvyo-app` calls `keyring` itself | The app is glue; the store's tests and the hygiene rules would live in Tauri code. |

**No key store on this computer (Frage 386)**

| Option | Why it won or lost |
|---|---|
| **A. Keychain only; Add with a token is refused (chosen, Frage 386 A)** | The spec's criterion 11; the connector pairing does the same (ADR 0071). |
| B. ADR 0007 §1's passphrase key file | A second passphrase prompt in the Add form; criterion 11 forbids a file. Stays for sharing keys. |
| C. A plain file | Frage 376 C, rejected by the customer. |

**The header (Frage 387)**

| Option | Why it won or lost |
|---|---|
| **A. `Authorization: Bearer <token>` for every Git catalogue (chosen, Frage 387 A)** | GitHub accepts `Bearer` for classic and fine-grained tokens; Gitea and Forgejo accept `bearer` for their tokens; one rule, no host list. |
| B. Per forge (`token` for GitHub and Gitea, `PRIVATE-TOKEN` for GitLab) keyed by host name | Self-hosted forges have any host name; a guessed table that is wrong for every unknown host. |
| C. Token in the URL (`?private_token=`, `user:token@`) | ADR 0007 §4 forbids it: URLs end up in logs, errors and redirects. |

**How the token reaches the request**

| Option | Why it won or lost |
|---|---|
| **A. A `storage-io::http::Authorized` transport that wraps another `Transport` and adds the token (chosen)** | `online_fetch` and `library-core` stay unchanged; the scripted test doubles see whether a token came. |
| B. A token field on `library-core::Request` | Core would hold the secret; `Request` is `Clone`, `Debug` and `PartialEq`. |
| C. A token parameter on every `online_fetch` function | Seven signatures change for one source kind. |

## Decision

1. **Owner.** `curvyo-storage-io` gets `catalogue_token.rs` (one responsibility: the access token of a Git catalogue): `AccessToken` (a `Zeroizing<String>`; `Debug` prints `AccessToken(<redacted>)`; no `Display`, `Serialize`, `Clone` or accessor other than the header value inside `http.rs`); `AccessToken::parse(text)`: 1 to 1024 visible ASCII characters (`0x21..=0x7E`), so no header injection and it fits Windows' 2560-byte blob; and `load`, `save`, `forget` on a `&CredentialStore`. No new dependency.
2. **Identity.** Key store service `curvyo/<slot>` (the installation's slot, ADR 0031 decision 12), account `git-token:<address>` where `<address>` is the normalised base address of `online-sources.toml` (ADR 0085 decision 1; at most 500 characters). The address is not secret and appears in the OS key store's list, so the maker can find and delete it there (criterion 7's label).
3. **Store.** Always `CredentialStore::keychain(slot)`, opened by the token commands themselves; never the key file, even if the sharing flow opened it this run (criterion 11). `NoKeychain` refuses Add or Change with the criterion-11 line; a source without a token never touches the key store, so a public user never sees a Secret Service or Keychain prompt.
4. **Sources file.** `SourceEntry` gains `token: bool` (`token = true` in TOML, default `false`, written only when true); allowed only for `kind = "git"` and never for the ready entry. `format` stays 1: an older build ignores the field (it then sees a public source). The file still holds no credential (ADR 0085 decision 1). `library-core` and `ui-core` only see this bool; the row's Lock mark (criterion 4) reads it without a request.
5. **Sending.** `Authorized { inner: &dyn Transport, token: &AccessToken }` implements `Transport`. `UreqTransport` sends `Authorization: Bearer <token>` per request (never on the agent's config), only on a hop whose scheme, host and port equal the source address's; a hop elsewhere gets no header. A source with `token = true` also pins its host (`Request::pins_host`, as for the ready entry), so a redirect to another host is refused (criterion 10, `Failure::Redirect`). Iconify sources, the ready entry and other Git catalogues are never wrapped. The token is read once per host command from the key store, held for that command's requests, and dropped when it returns. If `token = true` and the key store gives none or fails, no request is sent: new `Failure::TokenUnreadable` (no data) in `library-core` (criterion 12; no silent anonymous request).
6. **Refusals.** 401, 403 and 404 stay `Failure::Status`/`NotFound`; `ui-core` words them by the row's `token` bool (criteria 3, 13, 15). GitHub answers 404, not 401, for a private repository without a valid token.
7. **Commands.** `online_add` takes an optional token (its validating GET goes through `Authorized`; on success the entry is written, then the token saved; if the save fails the entry is taken out again and nothing is kept). `online_set_token(key, token)` validates the same way and replaces only on success; `online_forget_token(key)` deletes the entry and clears the flag; `online_remove` writes the file first, then forgets the token, and reports a failed delete (criterion 7). No command returns a token or any part of it.
8. **Hygiene (ADR 0007 §4, enforced in review).** The token never enters `library-core`, `ui-core`, `editor-wasm`, a URL, an error string, a log, `eprintln!`, a panic message or a fixture. `HttpError` carries no strings; `ureq` errors are dropped as today. The webview sees the token only while it is typed: a `type="password"` field with `autocomplete="off"`, cleared after the call, never kept in a store, `localStorage` or the undo history.
9. **Builds.** Only `curvyo-app` calls these commands. The browser build has no Online section (ADR 0085 decision 2), so it has no token field and no key store code; no `cfg` in core crates; all `*-core` crates stay wasm32-buildable.
10. **Tests.** `storage-io`: `AccessToken::parse` bounds and refusals (empty, space, CR/LF, non-ASCII, 1025 characters); `Debug` redaction; save, load and forget on `CredentialStore::file(tempdir)` (the store's existing test double, so no store trait); the local-server tests of `http.rs` record the header: present on the first hop and on a same-origin redirect, absent on a redirect to another origin (`127.0.0.1` to `localhost` or another port) and absent without a token. `library-core`: the `token` field round-trips, a missing field reads false, the ready entry refuses it. `curvyo-app::online_tests` with the scripted transport: Add with a token sends it only on validation success, Iconify never receives it, a flagged source with no stored token sends nothing (`TokenUnreadable`), Remove deletes both. The PR runs one manual live check against a private GitHub repository with a fine-grained token (Contents: Read-only) and records it; the hosting guide names only the forges checked this way.

## Consequences

- Sources file: one additive field, no `format` bump. No document format change.
- A GitLab private catalogue may not work: GitLab's `/-/raw/` web route does not document token headers, and its files API does not fit the `<base><path>` shape of ADR 0085 decision 3. If the live check fails there, the guide says so; a GitLab path is a follow-up.
- Moving to another machine means adding the token again (ADR 0007 §3).
- A token deleted in the OS key store by hand shows criterion 12's line; Change token fixes it.
- ADR 0007 §1's passphrase fallback stays for sharing keys only; tokens are keychain-only (decision 3).
