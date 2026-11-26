# Library token: reach a Git catalogue in a private repository with an access token

Status: Done (PR #501)
Priority: Should
Origin: Customer, 2026-11-26 (translated): "continue with 376 (token for private repos)". This is Frage 376 (LIB-TOKEN) of `0152`, recommendation A, taken as the customer's choice: a token per Git catalogue in the operating system's key store, desktop only, its own story. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: `0138` (Online section, Git catalogue, Add source, Remove), `0152` (library browser; criteria 32 and 33, UX notes 0.7 and H13), ADR 0085 (sources file, request guard, desktop-only Online section).

## User value

As a maker who keeps my own templates and parts in a private Git repository (a studio's shared library, paid designs I must not publish), I want to give Curvyo a read-only access token for that catalogue once, so that it appears in the Library tab and the browser like any public one, without my token ever being written to a file I might share, shown again or sent anywhere but to that repository's host.

## Field reference

| Tool | How a private source is reached |
|---|---|
| Inkscape (extension manager, palettes) | Public URLs only; no credential store |
| LightBurn library, Ink/Stitch | Files from disk or public download; no private catalogue |
| Git clients, VS Code, GitHub Desktop | Token or login kept in the OS key store (Keychain, Credential Manager, Secret Service) |

**What we do better.** The token lives where the maker's other secrets live (the OS key store), is shown once, is bound to one catalogue and one host, and every failure says what to do ("The token was refused.") instead of an empty list.

## Terms

- **Token:** a read-only personal access token the maker creates at the Git host (for GitHub: a fine-grained token with Contents: read on that one repository). Curvyo never needs more.
- **Key store:** the operating system's credential store as the desktop build reaches it (ADR in `adrs.md` names the mechanism).
- **Source settings:** the `KeyRound` action of a user-added Git catalogue row in the Library tab ("Change token…" or "Add token…") and the inline form it opens (UX notes section 4; replaces the sketch of `0152` UX notes 0.7). Remove stays the armed `X` of `0138`. This story adds no other setting.

## Acceptance criteria

1. Given the Add source form with kind Git catalogue (desktop build), then it has an optional **Access token** row under Address: a masked field, no Show button, no autofill, with the statement "The token is sent only to this address and kept in your system's key store."; kinds other than Git catalogue (Iconify) have no such row.
2. Given an address and a token, when the maker presses Add, then the validating request of `0138` (one GET of the index) is sent **with** the token, and only on success the source is added and the token is saved in the key store under that source; on failure nothing is saved, the form stays, and the token field keeps the typed text for a retry within the form only.
3. Given the refusals during Add, then each is shown under the field it is about and focus moves there: under the token field "The server refused the token." (401), "This token has no read access to the repository." (403, or a 404 of the index, with a token), "The server sent the request elsewhere. The token was not sent there." (redirect to another host with a token), "Enter a token of at most 1024 characters, without spaces." (bad token text, refused before any request; outer white space of a pasted token is dropped first) and the key store line of criterion 11; a wrong address, an offline machine and an invalid index keep the messages of `0138`. Without a token, a Git catalogue answering 404 of the index reads "No Git catalogue at this address. For a private repository, add an access token." and 401 or 403 reads "The server refused the request. For a private repository, add an access token." (the hint appears in the Add form only, not in the tree's failure line or a card).
4. Given a source that has a token, then the Library tab row and the browser's source group show a `Lock` mark after the name, tooltip line "Private, with a token"; it appears from the first paint, without a request.
5. Given source settings of a source with a token, when the maker presses **Change token…**, enters a new token and confirms, then it is validated as in criterion 2 (one GET of the index) and, on success only, replaces the stored one; on failure the old token stays in the key store and the old state of the source is unchanged (the key store line ends "Nothing was changed." here instead of "The source was not added."). The field is empty on opening (the old token is never shown or prefilled).
6. Given source settings of a source with a token, then **Remove token** forgets the token (key store entry deleted) and keeps the source as a public one; its next request is sent without a token (and fails with the source's failure line if the repository is private). A source without a token offers **Add token…** with the same flow as Change token.
7. Given a source with a token and **Remove** (the armed press of `0138`), then the entry in `online-sources.toml` and the key store entry are both deleted, in that order; if the key store delete fails the source is still removed and a persistent, selectable line at the top of the Online group says "The saved token could not be deleted from the key store; remove it there by hand: git-token:<address> in curvyo/<slot>." (the entry's account and service, ADR 0090 decision 2).
8. Given any time after saving, then the token is **never shown again**: not in the form, source settings, tooltips, the Credits block, an error line, the clipboard, the hosting guide or the accessibility tree; the masked field has no value attribute after Add.
9. Given `online-sources.toml`, a saved document, a project export, the sync service, a log, a crash report or a diagnostic copy, then none contains the token (nor part of it). The sources file stores only that a key store entry exists by the source's id **(Proposal)**, never the secret; a test writes a source with a sentinel token and searches every file the app wrote and every log line of the run for the sentinel (ADR 0085 decision 1 "holds no credential" stays true).
10. Given any request that carries the token, then it is sent **only** to the host of that catalogue's own address in `online-sources.toml` (index, pictures, template files), over `https`; a redirect to another host is refused and the token is not sent on it; the host command builds the header from the key store entry of the source id, never from a value the page passes in, and no other source (Iconify, the Curvyo catalogue, another Git catalogue) receives it; a token is never put in a URL or a query.
11. Given the key store is unavailable (no Secret Service on Linux, locked or denied, an error from the OS), then Add with a token is refused with "Your system's key store is not available, so the token cannot be kept. The source was not added." and nothing is stored in a file as a fallback (no plain-text mode); a source without a token is added as before.
12. Given an already added source whose key store entry is missing or cannot be read (key store locked, entry deleted outside Curvyo), then the source stays in the list, shows the Lock mark with the failure line "The saved token could not be read from the key store." and its requests are not sent without it (no silent downgrade to an anonymous request); Change token… and Remove still work.
13. Given a source with a token and a 401, a 403, or a 404 of the index on a later Refresh, expand or load (a 404 of one picture or template file keeps the `0138` load line), then the source's failure line says "The token was refused." (its card's row 3 in the browser), the cached items already fetched stay visible, and Change token… is offered next to Refresh.
14. Given the web build (browser build), then the Online section does not exist (ADR 0085 decision 2 "Builds"), so there is no token field, no Lock, no key store access and no mention of tokens in the Library tab; the browser's hosting guide (criterion 32 of `0152`) is shown in the desktop build only where the Add source form is.
15. Given `0152` criterion 33, then its text changes (as part of this story's PR): "Given a Git catalogue in a private repository, then the Add source form offers an optional Access token (criteria 1 to 13 of `0157`); without a token, a private repository is refused at Add with 'No Git catalogue at this address. For a private repository, add an access token.' (404, as GitHub answers) or 'The server refused the request. For a private repository, add an access token.' (401 or 403); the tree's failure line stays without the hint." The line "The repository must be public." is removed from the form and from the guide.
16. Given the in-app hosting guide (`0152` criterion 32; H11, UX notes 0.6 item 6), then its **Private repositories** step explains, in plain steps, how to create a read-only token for the repository (for GitHub: fine-grained token, one repository, Contents: Read-only, an expiry), where to paste it (Add source, Access token), that it is kept in the system's key store and sent only to the catalogue's own host, that it is never in `online-sources.toml`, and how to remove or change it; the final text is the product owner's and ships with the app (no request).
17. Given the ready Curvyo catalogue (`raw.githubusercontent.com/curvyo/templates`), then it is public, has no token row and never sends one (ADR 0085 decision 1).
18. Given all of the above, then no `format_version` of a document changes; `online-sources.toml` changes only if the architect needs one additive field (ADR 0085's `format = 1` rules; an older app ignores the field and sees the source without a token).

## Out of scope

- A token in plain text in `online-sources.toml` or in any file (Frage 376 C, rejected), a plain-text fallback when the key store is missing.
- Login flows (OAuth, device flow), refreshing tokens, one token for several sources, importing tokens from `git` credentials or `gh`.
- Writing to a catalogue (push, upload), private catalogues on a Curvyo server, accounts or paid content.
- Tokens for Iconify or any source other than a Git catalogue; sending a token to a host other than the source's own.
- The web build (no Online section there; a separate decision if it ever gets one).
- Syncing a token between machines (the sources file is machine-local, ADR 0085; each machine adds its own token).
- Checking a token's scope or expiry beyond the refusals of criteria 3 and 13.

## Open questions (customer)

Frage 386 (LIB-TOKEN-FALLBACK): What happens when the key store is unavailable (criterion 11)?
- A: Refuse Add with a token and say why; no token is kept anywhere else. Recommended.
- B: Keep the token for this session only, in memory, and ask again after a restart.
- C: Offer a plain-text file after an explicit warning.
- Recommendation: A. Default if unanswered: A.

Frage 387 (LIB-TOKEN-HOSTS): Which Git hosts does the token work with?
- A: Any `https` address of the catalogue with a standard `Authorization: Bearer <token>` header, to that address's host only; the hosting guide gives the GitHub steps (raw files of a private repository need the token). Recommended.
- B: GitHub only (`raw.githubusercontent.com` and `api.github.com`), other hosts refused with a token.
- Recommendation: A. Default if unanswered: A (the architect fixes the exact header per host in the ADR; a host that needs another scheme is a follow-up).

Frage 388 (LIB-TOKEN-LOCK): Is the Lock mark enough, or should a private source also be listed apart?
- A: Lock mark and tooltip only; the source sits in the list like any other. Recommended.
- B: A "Private" group in the Sets view and the Library tab.
- Recommendation: A. Default if unanswered: A.

(Numbers 386 to 388 are the next free ones in `specs/README.md` at drafting; the lead moves the counter.)

## UX notes

`ux-notes.md` (ux-engineer, 2026-11-26; T1 to T14, replaces the sketch of `0152` UX notes 0.7). Its section 10 texts for criteria 3, 5, 7, 13 and 15 and the "Source settings" term are adopted above. Architect notes of `adrs.md` (redirect refused, 404 hint) are adopted in criteria 3, 10 and 15.

## Links

Requirements: R-ASSET-004, R-SYS-007 (nothing leaves the machine unasked), R-LIB-* of `0137`/`0138`. Origin: Frage 376 (LIB-TOKEN) in `specs/0152-library-browser/specification.md`. ADR: `adrs.md` (architect, key store and header decisions, new dependency with licence, needs-customer not expected: it only stores a secret on the maker's machine and sends it to the host the maker named). PR: (none yet)
