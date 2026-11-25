# 0153 Library keywords: UX notes

ux-engineer, 2026-11-25. Written from `specification.md` (Draft, criteria 1 to 14), `adrs.md` and
ADR 0087 (decisions 4 to 6) on top of `specs/0152-library-browser/ux-notes.md` (sections 4, 5, 8,
10, 11) and the `docs/design-system.md` section "Library browser". No new control, no new token,
no new component: one detail-bar line, one credit line, one shipped-text place that reuses the
hosting guide's look. **(Proposal)** items are vetoable at the demo; section 6 lists them as K1 to
K9 (K, so they never collide with U, G or H of `0152`), section 7 the spec and ADR deltas.

## 1. Principle

Keywords make more icons match; they do not change how the browser looks or works. One ranked
list, no group or separator for "found by keyword" (K1): the ranking of criterion 3 already puts
name matches first, and the detail bar says why a tile is there when the name does not.

## 2. Ranking as the maker sees it (criterion 3)

- Best match orders by the weights of ADR 0087 decision 4; Name, Source and Newest ignore them
  as in `0152`. The subject line's count ("132 of 4,210 items") includes keyword and related
  matches; nothing counts them apart (K1).
- Ties keep the `0152` tie order. No badge, no colour marker on a tile (spec Out of scope:
  nothing on the tile).

## 3. Loading the keyword file (criterion 4, ADR 0087 decision 5)

- The fetch starts on the first non-empty search text in the Items view. Until it arrives, the
  search runs without keywords; when it arrives, the current query re-runs once, exactly like a
  keystroke (focus, scroll position and the current tile stay; the grid is at the top while
  typing anyway).
- **Nothing visible** while it loads, in both builds (K2). Only if it takes longer than **400 ms**
  (the threshold of "Copying..." in the design system) does the subject line add " · Loading
  keywords…" after the count (12 px muted, the Enter status slot of `0152` section 4; an Enter
  status, if any, wins the slot). It goes when the file is in or has failed. This is a state
  label, not a hint.
- **Failure** (offline in the browser build, a damaged file): no notice, no alert, the search
  works by names as before (ADR 0087 decision 5; `0138` "Offline is a state, never an alert").
  The fetch is tried again the next time the field goes from empty to text (K3; a maker action,
  so nothing refetches by itself). The desktop build has the file on disk; a failure there is a
  build defect the notices test catches.

## 4. Detail bar: why a tile matched (criterion 14)

- **When**: the bar shows a tile (hovered, else current) while the field holds text, and at least
  one typed word matched that tile **only** through a keyword, a category or a related word, not
  through its name or Iconify alias. Otherwise nothing new is shown.
- **Text** (K4), 12 px muted, ellipsis; the quoted word is the word in the data that the typed
  word hit (so a prefix or typo shows the real word: "magnif" or "magnifer" both show
  "magnifier"):
  - keyword: 'Matches “magnifier” (keyword)'
  - category: 'Matches “arrows” (category)'
  - related word: 'Matches “bin” (related to trash)', "bin" the head word the typed word hit,
    "trash" the icon's own word it led to.
  - Several such words: the first in typed order, then " and 1 more" (K5). The full list is in
    the preview box's tooltip.
- **Where** (K6): wide bar (113 px), line 4: the match text first, then " · " and the facts
  ("Monochrome · takes the object's colour"), the facts cut first by the ellipsis; the full facts
  stay in the preview box's tooltip. Narrow bar (89 px), a line 4 under the licence line appears
  only for the match text (a 20 px name line and three 16 px lines fit the 73 px inside the padding). Nothing moves in either layout:
  the bar's height is fixed.
- **Screen reader**: the same text is the `aria-description` of the current tile while it applies
  (K7), so a keyboard user hears why without hovering. No live announcement.
- **Cost**: the reason is computed for the one item the bar shows, when that item or the query
  changes, not in the per-keystroke pass (section 7, delta 2).

## 5. Credit line and licence texts (criterion 10)

- **Where** (K8): the last row of the facet column in the Items view (wide), and the last row of
  the filter band (narrow), 16 px below the Category section, not in the Sets view (keywords are
  not used there). One line, 12 px `--panel-muted-fg`, wraps inside the 192 px content width:

  > Search keywords: [Lucide](https://lucide.dev), [Tabler Icons](https://tabler.io/icons),
  > [Material Design Icons](https://pictogrammers.com/library/mdi/) · **Licences**

  Project names are links in the `0138` link-cell look (open in the system browser; the browser
  build a new tab). No versions and licence names on the line; they are in the licence texts
  (section 7, delta 1).
- **Licences** (12 px `--editor-accent` text button, the Host your own library… look) opens a
  place of the browser (`0152` 0.3, pushed on the history, Back "Back to all items", Escape and
  Alt+Left go back): the hosting guide's look (one scroll region, 16 px padding, text at most
  640 px), title "Search keyword licences" 14 px, then the shipped `keywords-LICENSES.txt` as
  preformatted 12 px text (name, version, licence and the licence text per source). It works
  offline in the desktop build. If the file cannot be read: "Cannot read the licence file." 14
  px muted, the `0152` empty-state look (K9).
- Tab order: the line's links after the Category section, before the grid (`0152` section 11).

## 6. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| K1 | One ranked list; no group, separator or count for keyword matches |
| K2 | No loading indicator; after 400 ms " · Loading keywords…" in the subject line until done or failed |
| K3 | A failed fetch is tried again when the field next goes from empty to text |
| K4 | Detail-bar texts 'Matches “magnifier” (keyword)', '(category)', 'Matches “bin” (related to trash)', quoting the data's word, not the typed text |
| K5 | Several such words: the first in typed order " and 1 more"; all in the preview box tooltip |
| K6 | Wide: the match text leads line 4 before the facts; narrow: a line 4 only for it |
| K7 | The match text is the current tile's `aria-description` while it applies |
| K8 | The credit line at the foot of the facet column or filter band, Items view only, project names as links, no versions |
| K9 | **Licences** opens a browser place with the shipped licence texts (hosting-guide look) |

## 7. Spec and ADR deltas (for the product owner and the architect)

1. **Criterion 10a**: the line sits in the library browser (section 5), as ADR 0087 decision 6
   says, not in the Credits area; it carries no versions (they are in the licence texts and the
   file header, 10b and 10c); MDI reads "(MIT)" per the architect, in the licence texts only.
2. **Criterion 14 needs an explain call**: the matcher has to say, for one item and the current
   query, which typed word matched which word and by which field (keyword, category, related and
   its head). Proposal for the architect: `Browser::explain(item)` on demand, outside the 100 ms
   keystroke budget; not in ADR 0087 today. "Category" joins the two texts of criterion 14.
3. **ADR 0087 decision 5**: "fetches once" becomes "once per success; after a failure again when
   the field next goes from empty to text" (K3). The 400 ms state label of K2 is the only visible
   trace of loading.
