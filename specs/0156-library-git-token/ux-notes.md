# 0156 Library token: UX notes

ux-engineer, 2026-11-26. Written from `specification.md` (criteria 1 to 18) at the defaults of
Fragen 386 A (no key store: refuse, nothing kept), 387 A (`Authorization: Bearer` to the source's own
origin) and 388 A (Lock mark only), and from `adrs.md` / ADR 0090 (decisions 3 to 8). Checked against
`specs/0138-template-library/ux-notes.md` (sections 3, 4, 8, 9), `specs/0152-library-browser/ux-notes.md`
(0.2 Sets groups and cards, 0.6 hosting guide, 0.7 the sketch these notes replace), `docs/design-system.md`
("Library tab", "Library browser", "Armed action", "Action notice", "Hidden, not disabled", "No notice
after a success", "Connector group" for `KeyRound`) and the code: `panel/AddSourceForm.tsx`,
`panel/OnlineRows.tsx` (`OnlineSourceCells`), `panel/LibraryTab.tsx`,
`curvyo-ui-core/src/library_online_text.rs`, `library_browser_guide.rs`. **(Proposal)** items are
vetoable at the demo and listed as T1 to T14 (section 9); text changes for the product owner are in
section 10. "Muted" is `--panel-muted-fg`. No new colour token, no new component, no popup, no
operating tip: every new line names a state, a refusal or what leaves the machine.

## 1. Principle

The token is typed once, in a field that never shows it, and afterwards exists in the UI only as a
fact: a `Lock` after the source's name. Everything about it is managed in the Library tab (the place
to manage sources, `0152` section 1); the browser only shows the Lock and the failure line.

## 2. Add source form (criteria 1, 2, 3, 11, 15)

Rows of the `0138` form, kind **Git catalogue**, top to bottom:

| Row | Control |
|---|---|
| Kind | unchanged |
| Address | unchanged: label, field, its refusals under it, the "What is sent" line "Add sends one request to this address." |
| Access token | label "Access token (optional)" 12 px above; field full width, 28 px, 14 px, `type="password"`, `autocomplete="off"`, `autocapitalize="off"`, `autocorrect="off"`, `spellcheck="false"`, `data-1p-ignore`, `data-lpignore="true"`; no Show button, no placeholder |
| Statement | 12 px muted, under the field: "The token is sent only to this address and kept in your system's key store." The field is described by it (`aria-describedby`) |
| Token refusal | under the statement, `role="alert"`, `--field-invalid`, the field `aria-invalid` and described by it |
| Guide | **Host your own library…** unchanged |
| Buttons | Cancel, Add unchanged |

- The line "The repository must be public." leaves the form (criterion 15).
- Kind **Iconify**: no Access token row, no statement; switching to Iconify **clears** the token
  field, switching back shows it empty (T2).
- **Empty is no token**: an empty field (or only spaces) adds a public source exactly as `0138`.
  Leading and trailing white space of a pasted token is dropped before the check (T3); anything else
  that `AccessToken::parse` refuses is refused before any request: "Enter a token of at most 1024
  characters, without spaces." (under the token field; focus stays in it).
- **Enter** in the token field adds, as in the Address field. Paste works; copy and cut out of the
  field do nothing (the password field's own behaviour).
- **Order of checks** (each refusal ends the press, nothing is sent): address refusals of `0138`, then
  the token text, then the key store (T4: with a token the host command opens the key store
  **before** the validating request, so a missing key store sends nothing), then the one GET.
- While checking: the token field is read only like Address; Cancel stops the check. A source the
  host had already added when the cancel landed is removed with its token (`online_remove`, so the
  key store entry goes too).
- **Success**: the form closes as in `0138`, the new row shows the Lock from its first frame; no
  notice.
- **Failure**: the form stays, the token field keeps the typed text (criterion 2), the refusal goes
  under the field it is about, focus moves to that field:

| Case | Under | Text |
|---|---|---|
| 401 with a token | token | "The server refused the token." |
| 403, or 404 of the index, with a token | token | "This token has no read access to the repository." |
| Redirect to another host, with a token | token | "The server sent the request elsewhere. The token was not sent there." |
| Key store unavailable (no Secret Service, locked, denied, an OS error) | token | "Your system's key store is not available, so the token cannot be kept. The source was not added." |
| Key store save failed after a good GET (the entry is taken out again, ADR 0090 decision 7) | token | the same key store line |
| 404 of the index **without** a token, Git catalogue | address | "No Git catalogue at this address. For a private repository, add an access token." |
| 401 or 403 **without** a token, Git catalogue | address | "The server refused the request. For a private repository, add an access token." |
| Wrong address, offline, invalid index, other statuses | address | the `0138` texts, unchanged |

- **The 404 hint (settles the architect's note, T5)**: GitHub answers 404 for a private repository
  without a token, so the hint "For a private repository, add an access token." is appended to the
  token-less 404 **and** 401/403 refusals of the Add form only, where the Access token field is in
  view. It is not appended to the tree's failure line or a card (section 5): there the shown
  `KeyRound` action is the pointer, and a 28 px line has no room for a second sentence.
- **Field hygiene (criterion 8, ADR 0090 decision 8)**: the token field is **uncontrolled** (read
  through a ref at Add, `value = ""` set on success, Cancel, kind switch and unmount); its text is
  never in React state, a store, `localStorage`, the undo history or a `title`. Reason: a React
  controlled input syncs its `value` **attribute** to the typed text, which criterion 8 forbids. A
  test asserts `getAttribute("value") === null` after typing and after Add. The webview must not
  offer to save it as a password (checked on the three desktop webviews in the PR).

## 3. Lock mark (criterion 4, Frage 388 A)

- **Library tab source row**: `Lock` 12 px, 1.5 px stroke, muted, 4 px after the name, `flex: none`;
  the name is cut before it, never the Lock (T6). Read from the `token` flag of `online-sources.toml`,
  so it is there from the first paint, without a request.
- **Browser, Sets view group header**: the same `Lock` 12 px after the group name, before " · 120
  templates". Items view tiles, cards and the Sources facet rows carry none (Frage 388 A: no own
  group, no further mark).
- Tooltip (row and header): line 1 the name, line 2 muted the host (as today), line 3 muted "Private,
  with a token".
- Accessible name: ", private" after the kind: "Studio parts, Git catalogue, private, 120 templates".
  The glyph is `aria-hidden`.
- The Lock stays while the token is refused or unreadable (criteria 12, 13): it says "a token is
  configured", the failure line says what is wrong.

## 4. Token settings of a source (criteria 5, 6, 12, 13)

"Source settings" of the spec are one action and one inline form (T7):

- **Action**: user-added Git catalogue rows get a third action in their slot, **`KeyRound`** 24 x 24
  between Refresh and Remove (slot 80 px: 3 x 24 + 2 x 4); same rules as Refresh and Remove (shown on
  hover, focus within; not a Tab stop; Right from the name cell reaches Refresh, Token, Remove).
  Tooltip and accessible name by state: "Change token…" (source has a token) / "Add token…" (none).
  It is **always shown** (`library-action-shown`) while the source's failure line is a token or
  access failure (section 5), so it sits next to Refresh (criterion 13). The Curvyo catalogue ready
  entry and Iconify rows have no such action and keep the 56 px slot (criterion 17).
- **Form**: the press opens the inline form in the Add source form's place above the footer (the
  `0045` look, `role="group"` named "Access token of Studio parts", `--toolbar-bg`, 8 px padding).
  One inline form at a time: opening it closes an open Add source form (its text dropped) and the
  reverse; **Add source...** is absent while either is open (`0138` Q3) (T8).

| Row | Control |
|---|---|
| Heading | the source name 12 px semibold muted, with its Lock if it has a token |
| Field | label "New access token" (has a token) / "Access token" (none) 12 px above; the field of section 2 (same attributes, uncontrolled, **empty on opening**, criterion 5) |
| Statement | the same 12 px muted line as in section 2 |
| Refusal | under the statement, as in section 2 |
| Remove token | only with a token: a 28 px text button "Remove token", left-aligned, 12 px `--toolbar-icon`; **Armed action**: first press arms ("Press again to remove the token", `--destructive` ground, the `0137` 4.4 timing), the second removes |
| Buttons | **Cancel** and **Save**, the `0045` pair |

- Focus on opening: the field. Tab order: field, Remove token, Cancel, Save. Enter in the field
  saves; Escape or Cancel closes, field cleared, focus back to the source row.
- **Save** (Change token… or Add token…): an empty field or bad text gives the section 2 token text
  refusal, nothing sent; else one GET of the index with the new token (criterion 5); busy as Add
  (Save shows `LoaderCircle`, field read only, Cancel stops). Success: the form closes, the Lock is
  shown, the source's failure line leaves, focus returns to the source row; if the source was
  expanded, that one GET's list is shown (no second request). No notice. Failure: the refusals of
  section 2 (key store line ends "Nothing was changed." here instead of "The source was not added.",
  T9), the old token and the old state stay.
- **Remove token**: the key store entry is deleted and the flag cleared; the form closes, the Lock
  leaves, focus back to the source row; nothing is fetched (the next Refresh or expand goes without a
  token, criterion 6). If the delete fails the flag is still cleared and the key store line of
  section 6 shows (T10).

## 5. Failure lines after Add (criteria 6, 12, 13)

Shown where `0138` and `0152` show a source failure: the Library tab's "Fetch failed" child row (28 px,
`CircleAlert` 14 px `--destructive`, reason 12 px muted, cut with the whole text in the tooltip and the
source row's description) and the browser card's row 3 (`CircleAlert` and the reason). Items already
fetched this session stay below the line and in the grid (criterion 13).

| Source | Answer | Line | `KeyRound` shown |
|---|---|---|---|
| with a token | 401, 403, or 404 of the index | "The token was refused." | yes |
| with a token | entry missing or unreadable; **no request sent** | "The saved token could not be read from the key store." | yes |
| with a token | redirect to another host | "The server sent the request elsewhere." (existing) | no |
| without a token, Git | 401 or 403 | "The server refused the request." (was "The server answered 401.") | yes |
| without a token, Git | 404 of the index | "No Git catalogue at this address." (unchanged) | yes |

- A 404 of one picture or template file of a source with a token keeps the `0138` load line
  ("cannot be loaded: damaged."): the index answered, so the token works (T11).
- A token source whose key store entry cannot be read never falls back to an anonymous request
  (criterion 12); Change token… and Remove keep working.
- Offline stays "Offline" (`0138` section 8), with or without a token.

## 6. Removing a source with a token (criterion 7)

- The armed `X` of `0138` is unchanged ("Press again to remove Studio parts from the library"); the
  text does not mention the token (it goes with the source, as its list does).
- If the key store delete fails, the source is still removed and a line shows at the top of the
  Online group: `CircleAlert` 14 px `--destructive`, 12 px text, wraps, selectable, `role="alert"`:
  "The saved token could not be deleted from the key store; remove it there by hand:
  git-token:<address> in curvyo/<slot>." It stays until the Library tab is left or another source is
  added or removed (T12); not an Action notice, because the label must stay readable and copyable.
  The label is ADR 0090 decision 2's account and service, the strings the OS key store lists.

## 7. Builds, guide, ready entry (criteria 14, 16, 17)

- **Browser build**: no Online section (ADR 0085 decision 2), so no field, no Lock, no `KeyRound`, no
  key store, and no word "token" in the Library tab or the browser; where the hosting guide is shown
  in the browser build, its **Private repositories** step is left out (T13).
- **Hosting guide, step 6 "Private repositories"** (`0152` 0.6), desktop: the product owner's text in
  this structure, each one plain sentence, numbered: (1) create a read-only token for that one
  repository (GitHub: fine-grained token, one repository, Contents: Read-only, an expiry); (2) Library
  tab, Add source…, Git catalogue, paste it into Access token; (3) it is kept in your system's key
  store and sent only to the catalogue's own address, never written to `online-sources.toml`; (4) to
  change or remove it, the `KeyRound` action of the source; (5) a token that expired reads "The token
  was refused."; make a new one and use Change token…. It names only forges the PR checked live
  (ADR 0090 decision 10). No link to the forge's token page beyond the example repository (T14).
- **Curvyo catalogue** ready entry: no token row, no `KeyRound`, never a Lock (criterion 17).

## 8. Keyboard and screen readers

- Tab order of the Library tab unchanged except the inline form: search, tree, (form: Kind,
  Address, Access token, Host your own library…, Cancel, Add), Add folder..., Add source....
- Tree: Right from a Git catalogue's name cell reaches Refresh, Token, Remove; Delete still arms
  Remove. No new shortcut.
- Announcements: refusals are `role="alert"` text changes; success is silent (the Lock and the row's
  name, ", private", change); the token text is never part of a name, description or live region.

## 9. Proposals

| # | Proposal |
|---|---|
| T1 | Token refusals go under the token field, address refusals under Address; focus moves to the field the refusal is about |
| T2 | Switching the kind to Iconify clears the token field |
| T3 | Outer white space of a pasted token is dropped before the check; the text refusal "Enter a token of at most 1024 characters, without spaces." |
| T4 | With a token, the key store is opened before the validating GET, so a missing key store sends nothing |
| T5 | The hint "For a private repository, add an access token." only on the token-less 401/403/404 refusals of the Add form |
| T6 | Lock 12 px muted after the name, never cut; ", private" in the accessible name; tooltip line 3 "Private, with a token" |
| T7 | Source settings = a `KeyRound` action ("Change token…" / "Add token…") on user Git catalogue rows, slot 80 px, plus one inline form |
| T8 | One inline form at a time in the Library tab (Add source or Access token), in the same place above the footer |
| T9 | Key store unavailable on Change/Add token: "Your system's key store is not available, so the token cannot be kept. Nothing was changed." |
| T10 | Remove token whose key store delete fails still clears the flag and shows the section 6 line |
| T11 | On a token source, 401, 403 and a 404 of the index read "The token was refused."; a 404 of one file keeps the load line |
| T12 | The failed-delete line is a persistent, selectable line at the top of the Online group, not a transient notice |
| T13 | The browser build's hosting guide has no Private repositories step |
| T14 | Guide step 6 in five numbered sentences; Change and Remove name the `KeyRound` action |

## 10. For the product owner (texts to adopt)

- Criterion 3: add "The server sent the request elsewhere. The token was not sent there." (redirect
  with a token) and "Enter a token of at most 1024 characters, without spaces." (bad token text); 404
  with a token is "a 404 of the index".
- Criterion 15, replace the quoted line by: "without a token, a private repository is refused at Add
  with 'No Git catalogue at this address. For a private repository, add an access token.' (404, as
  GitHub answers) or 'The server refused the request. For a private repository, add an access
  token.' (401 or 403)". The tree's failure line stays without the hint (section 5).
- Criterion 13: "a 401, a 403, or a 404 of the index" on a source with a token.
- Criterion 5: the key store line on Change ends "Nothing was changed." (T9).
- Criterion 7: the line ends "remove it there by hand: git-token:<address> in curvyo/<slot>."
- "Source settings" in Terms: the `KeyRound` action and its inline form (section 4), not expanded
  actions of the row.
