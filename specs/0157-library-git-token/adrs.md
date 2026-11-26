# ADRs for library Git token

Architect, 2026-11-26. One new ADR, not `needs-customer` (the customer chose Frage 376 A; no new
crate, no new dependency, no document format change; one additive field in `online-sources.toml`).
Fragen 386 A and 387 A are taken at their defaults; Frage 388 is UX only and changes no decision.

- [ADR 0090](../../docs/adr/0090-git-catalogue-access-token-in-the-os-key-store.md) (Proposed):
  - owner: `curvyo-storage-io::catalogue_token` beside the existing `credentials` module;
    `AccessToken` (zeroized, redacted `Debug`, 1 to 1024 visible ASCII characters); `keyring`
    3.6.3 already locked, no new dependency (decision 1; criteria 9, 10);
  - identity: service `curvyo/<slot>`, account `git-token:<address>` (decision 2; criterion 7);
  - keychain only, never the passphrase key file; a public source never touches the key store
    (decision 3; criterion 11, Frage 386 A);
  - `token = true` on a Git `[[source]]`, `format` stays 1; core crates see only this bool
    (decision 4; criteria 4, 9, 17, 18);
  - `Authorization: Bearer <token>` through an `Authorized` transport wrapper, only to the source's
    own origin, host pinned so another host's redirect is refused; flagged source without a
    readable token sends nothing, `Failure::TokenUnreadable` (decision 5; criteria 2, 10, 12,
    Frage 387 A);
  - refusals worded by `ui-core` from the existing statuses and the `token` bool (decision 6;
    criteria 3, 13, 15);
  - host commands `online_add` (optional token), `online_set_token`, `online_forget_token`,
    `online_remove` order; none returns a token (decision 7; criteria 2, 5, 6, 7, 8);
  - hygiene and the webview field (decision 8; criteria 8, 9); desktop only (decision 9;
    criterion 14); tests and the one recorded live check (decision 10).
- [ADR 0085](../../docs/adr/0085-online-template-sources-read-only.md): decisions 1 (sources file,
  ready entry public) and 2 (request shape, guard, desktop-only Online section) amended by 0090
  decisions 4 and 5.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md): §3 (machine
  local, never synced) and §4 (hygiene) apply unchanged; §1's passphrase fallback is not used for
  tokens (0090 decision 3).
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md) decision 12: the
  credential store and its slot, reused.

## Notes for the product owner and UX

- Criterion 10 and decision 5 agree: a token-bearing source refuses a redirect to another host
  (not followed without the token).
- GitHub answers 404 for a private repository without a valid token; criterion 3's "403 or 404
  with a token" covers it. Criterion 15's line ("The server refused the request...") will not
  appear on GitHub without a token: that is a 404, which reads "No Git catalogue at this address."
  (`0138`). The product owner should name the status in criterion 15, or add the hint "For a
  private repository, add an access token." to the 404 line of a Git source without a token.
- GitLab private catalogues may not work (ADR 0090 Consequences); the guide names only forges the
  PR checked live.
