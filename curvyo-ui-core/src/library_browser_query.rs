//! The browser's query (spec `0152` criteria 11 to 20, ADR 0086 decisions 2 and 4): the fuzzy
//! search, the filters by AND with their facet counts, the sorts and the second level, over one
//! list held for the session.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use curvyo_library_core::{Keywords, Scratch, SearchIndex, fold};

use crate::library_browser::{BrowseList, CardKind, ItemKind};
use crate::library_browser_index::build_index;
use crate::library_browser_sets::{SetsAnswer, SetsState, SetsTables};
use crate::library_browser_sets_text::{CardWords, card_words};
use crate::library_browser_tables::{categories_of, orders_of, source_rows};
use crate::library_browser_text::{
    TileWords, browse_subject, narrowed_subject, nothing_matches_text, tile_words,
};
use crate::library_browser_why::{WhyWords, why_words};

/// The Sort control (criterion 17).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    /// By score, only with search text; ties keep the own sort.
    Best,
    /// Name A to Z, case-insensitive, digits as numbers.
    #[default]
    Name,
    /// Source, then name.
    Source,
    /// Local pages by their document's change time first, the rest after by name.
    Newest,
}

/// The Colour filter (criterion 14).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColourFilter {
    /// Everything.
    #[default]
    All,
    /// Icons of monochrome sets (and the refined ones).
    Mono,
    /// Icons with their own colours.
    Colour,
}

/// What the maker chose: search text, sort, filters and the open second level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowseState {
    /// The search field's text.
    pub text: String,
    /// The chosen sort.
    pub sort: Sort,
    /// The sort chosen last without text: Best match's tie order.
    pub own_sort: Sort,
    /// The checked Source rows by key; none is all.
    pub sources: Vec<String>,
    /// The checked Category rows by key; none is all.
    pub categories: Vec<String>,
    /// The Colour strip.
    pub colour: ColourFilter,
    /// The Customizable switch.
    pub customizable: bool,
    /// The item whose pages are shown (the second level).
    pub level: Option<u32>,
    /// The Sources selection is a set, catalogue or folder opened from the Sets view (criterion
    /// 27): the subject line names it; a selection made in the list does not.
    pub narrowed: bool,
}

/// One row of a check list with its count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FacetRow {
    /// The key the state names it by.
    pub key: String,
    /// The row's name.
    pub label: String,
    /// The items that pass the other filters.
    pub count: usize,
    /// A workspace or folder row under "My documents".
    pub indent: bool,
}

/// What the grid, the facets and the subject line show for one state.
// The page reads each flag on its own (a disabled strip, a muted line, the Show sets button).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Answer {
    /// The items at this level.
    pub total: usize,
    /// The items shown.
    pub count: usize,
    /// The items shown, in order (crosses to the page as a `Uint32Array`).
    #[serde(skip)]
    pub order: Vec<u32>,
    /// The Source rows.
    pub sources: Vec<FacetRow>,
    /// The Category rows: A to Z, "Uncategorised" last.
    pub categories: Vec<FacetRow>,
    /// Whether a fetched item has a category (else a muted state line).
    pub has_categories: bool,
    /// Whether a fetched item has colour data (else Monochrome and Colour are disabled).
    pub has_colour: bool,
    /// Whether a fetched item is customizable (else the switch is disabled).
    pub has_customizable: bool,
    /// "132 of 4,210 items", "Box job › Lid · 3 pages".
    pub subject: String,
    /// 'Nothing matches "hrat" in Monochrome.'; empty while something is shown.
    pub empty: String,
    /// The active filters (the Filters button's count).
    pub active: usize,
    /// The empty Items view offers Show sets: there are sets or sources to open (H14).
    pub show_sets: bool,
}

/// The browser's session memory: the list, its search index, categories and sort orders.
pub struct Browser {
    list: BrowseList,
    index: SearchIndex,
    sets: SetsTables,
    scratch: Scratch,
    /// `(key, label)`: folded key, A to Z, "Uncategorised" (key "") last.
    categories: Vec<(String, String)>,
    item_categories: Vec<Vec<u16>>,
    palette: Vec<Option<bool>>,
    pending: Vec<u32>,
    last: Option<BrowseState>,
    orders: [Vec<u32>; 3],
    /// Per source row: key, label, whether indented under "My documents".
    rows: Vec<(String, String, bool)>,
    /// Per list source: its row, and the "My documents" row for a local one.
    row_of_source: Vec<(usize, Option<usize>)>,
    /// The keyword file, kept for the session (spec `0153` criterion 4).
    keywords: Option<Arc<Keywords>>,
}

pub(crate) fn id(index: usize) -> u32 {
    // invariant: a list holds far fewer than 2^32 items
    u32::try_from(index).unwrap_or(u32::MAX)
}

impl Browser {
    /// The browser over `list`: the index, categories and orders are built once.
    #[must_use]
    pub fn new(list: BrowseList) -> Self {
        Self::with_keywords(list, None)
    }

    /// The browser over `list` with the session's keyword file, if it has arrived.
    #[must_use]
    pub fn with_keywords(list: BrowseList, keywords: Option<Arc<Keywords>>) -> Self {
        let (categories, item_categories) = categories_of(&list);
        let index = build_index(&list, &categories, &item_categories, keywords.as_deref());
        let sets = SetsTables::new(&list);
        let palette = list
            .items
            .iter()
            .map(|item| {
                (item.kind == ItemKind::Icon)
                    .then(|| list.sources[item.source as usize].palette)
                    .flatten()
            })
            .collect();
        let (rows, row_of_source) = source_rows(&list);
        let orders = orders_of(&list, &row_of_source);
        Self {
            list,
            index,
            sets,
            scratch: Scratch::default(),
            categories,
            item_categories,
            palette,
            pending: Vec::new(),
            last: None,
            orders,
            rows,
            row_of_source,
            keywords,
        }
    }

    /// The keyword file arrived: kept for the session, the index rebuilt once (ADR 0087
    /// decision 5); the next query uses it.
    pub fn set_keywords(&mut self, keywords: Arc<Keywords>) {
        self.index = build_index(
            &self.list,
            &self.categories,
            &self.item_categories,
            Some(&keywords),
        );
        self.keywords = Some(keywords);
    }

    /// The keyword file, once it has arrived: a new list keeps it (ADR 0087 decision 5).
    #[must_use]
    pub fn keywords(&self) -> Option<&Arc<Keywords>> {
        self.keywords.as_ref()
    }

    /// Why item `index` matched the search `text`, when a typed word matched it only through a
    /// keyword, a category or a related word (spec `0153` criterion 14); asked for the one item
    /// the detail bar shows, outside the keystroke pass.
    pub fn why(&mut self, index: u32, text: &str) -> Option<WhyWords> {
        let reasons = self.index.explain(text, index, &mut self.scratch);
        why_words(&reasons)
    }

    /// The list.
    #[must_use]
    pub fn list(&self) -> &BrowseList {
        &self.list
    }

    /// A body arrived for icon `item`: a monochrome set's icon with fixed colours counts as Colour
    /// from the next change of the filters or the search (criterion 14, Proposal).
    pub fn note_body(&mut self, item: u32, fixed_colour: bool) {
        if fixed_colour && self.palette.get(item as usize) == Some(&Some(false)) {
            self.pending.push(item);
        }
    }

    fn customizable(&self, item: usize) -> bool {
        let it = &self.list.items[item];
        it.tagged_customizable
            || (it.kind == ItemKind::Icon
                && self.list.sources[it.source as usize].palette == Some(false))
    }

    fn colour_passes(colour: ColourFilter, palette: Option<bool>) -> bool {
        match colour {
            ColourFilter::All => true,
            ColourFilter::Mono => palette == Some(false),
            ColourFilter::Colour => palette == Some(true),
        }
    }

    fn category_ids(&self, keys: &[String]) -> HashSet<u16> {
        let keys: HashSet<String> = keys.iter().map(|k| fold(k)).collect();
        self.categories
            .iter()
            .enumerate()
            .filter(|(_, (key, _))| keys.contains(key))
            .filter_map(|(n, _)| u16::try_from(n).ok())
            .collect()
    }

    /// The answer for `state`.
    pub fn query(&mut self, state: &BrowseState) -> Answer {
        if self.last.as_ref() != Some(state) {
            for item in std::mem::take(&mut self.pending) {
                self.palette[item as usize] = Some(true);
            }
            self.last = Some(state.clone());
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        let scores = self.index.scores(&state.text, &mut scratch);
        let picked: HashSet<&str> = state.sources.iter().map(String::as_str).collect();
        let all_local = picked.contains("local");
        let cats = self.category_ids(&state.categories);
        let mut source_counts = vec![0usize; self.rows.len()];
        let mut category_counts = vec![0usize; self.categories.len()];
        let mut shown = vec![false; self.list.items.len()];
        let mut total = 0;
        for (n, item) in self.list.items.iter().enumerate() {
            if item.parent != state.level {
                continue;
            }
            total += 1;
            let source = &self.list.sources[item.source as usize];
            let text = scores.is_none_or(|s| s[n] > 0);
            let src = picked.is_empty()
                || picked.contains(source.key.as_str())
                || (source.local && all_local);
            let cat = cats.is_empty() || self.item_categories[n].iter().any(|c| cats.contains(c));
            let col = Self::colour_passes(state.colour, self.palette[n]);
            let cust = !state.customizable || self.customizable(n);
            if text && cat && col && cust {
                let (row, parent) = self.row_of_source[item.source as usize];
                source_counts[row] += 1;
                if let Some(parent) = parent {
                    source_counts[parent] += 1;
                }
            }
            if text && src && col && cust {
                for c in &self.item_categories[n] {
                    category_counts[usize::from(*c)] += 1;
                }
            }
            shown[n] = text && src && cat && col && cust;
        }
        let best = state.sort == Sort::Best && scores.is_some();
        let own = if state.sort == Sort::Best {
            state.own_sort
        } else {
            state.sort
        };
        let mut order: Vec<u32> = self
            .order(own)
            .iter()
            .copied()
            .filter(|i| shown[*i as usize])
            .collect();
        if let (true, Some(scores)) = (best, scores) {
            order.sort_by_key(|i| std::cmp::Reverse(scores[*i as usize]));
        }
        self.scratch = scratch;
        self.answer(state, total, order, source_counts, category_counts)
    }

    fn order(&self, sort: Sort) -> &[u32] {
        match sort {
            Sort::Best | Sort::Name => &self.orders[0],
            Sort::Source => &self.orders[1],
            Sort::Newest => &self.orders[2],
        }
    }

    /// The Sets view's answer for `state` (criterion 26).
    pub fn query_sets(&mut self, state: &SetsState) -> SetsAnswer {
        let mut scratch = std::mem::take(&mut self.scratch);
        let answer = self.sets.query(&self.list, state, &mut scratch);
        self.scratch = scratch;
        answer
    }

    fn answer(
        &self,
        state: &BrowseState,
        total: usize,
        order: Vec<u32>,
        source_counts: Vec<usize>,
        category_counts: Vec<usize>,
    ) -> Answer {
        let sources: Vec<FacetRow> = self
            .rows
            .iter()
            .zip(source_counts)
            .map(|((key, label, indent), count)| FacetRow {
                key: key.clone(),
                label: label.clone(),
                count,
                indent: *indent,
            })
            .collect();
        let has_categories = self.categories.iter().any(|(key, _)| !key.is_empty());
        // Without a category in the fetched items the filter is one muted line, no rows (G5).
        let categories: Vec<FacetRow> = self
            .categories
            .iter()
            .filter(|_| has_categories)
            .zip(category_counts)
            .map(|((key, label), count)| FacetRow {
                key: key.clone(),
                label: label.clone(),
                count,
                indent: false,
            })
            .collect();
        let colour_word = match state.colour {
            ColourFilter::All => None,
            ColourFilter::Mono => Some("Monochrome".to_owned()),
            ColourFilter::Colour => Some("Colour".to_owned()),
        };
        let picked = |rows: &[FacetRow], keys: &[String]| -> Vec<String> {
            let keys: HashSet<String> = keys.iter().map(|k| fold(k)).collect();
            rows.iter()
                .filter(|r| keys.contains(&fold(&r.key)))
                .map(|r| r.label.clone())
                .collect()
        };
        let mut values = picked(&sources, &state.sources);
        values.extend(colour_word);
        if state.customizable {
            values.push("Customizable".to_owned());
        }
        values.extend(picked(&categories, &state.categories));
        let count = order.len();
        let searching = !state.text.trim().is_empty();
        let show_sets = self.list.cards.iter().any(|c| c.kind != CardKind::Folder);
        let empty = if count > 0 {
            String::new()
        } else if total == 0 && !searching && values.is_empty() && state.level.is_none() {
            // Sets or sources to open: "No items yet." with Show sets (H14); else the library
            // holds nothing at all (the browser build without documents, UX notes 10).
            if show_sets {
                "No items yet.".to_owned()
            } else {
                "Nothing in the library yet.".to_owned()
            }
        } else {
            nothing_matches_text(state.text.trim(), &values)
        };
        let level = state
            .level
            .and_then(|p| self.list.items.get(p as usize))
            .map(|item| (item, &self.list.sources[item.source as usize]));
        let narrowed = self.narrowed_to(state, &sources);
        let mut subject = match (level, narrowed) {
            (None, Some((label, in_source))) => narrowed_subject(&label, count, in_source),
            _ => browse_subject(count, total, level),
        };
        if searching && !self.list.search_status.is_empty() && level.is_none() {
            subject = format!("{subject} · {}", self.list.search_status);
        }
        Answer {
            total,
            count,
            order,
            show_sets,
            has_categories,
            has_colour: self.palette.iter().any(Option::is_some),
            has_customizable: (0..self.list.items.len()).any(|n| self.customizable(n)),
            sources,
            categories,
            subject,
            empty,
            active: usize::from(!state.sources.is_empty())
                + usize::from(!state.categories.is_empty())
                + usize::from(state.colour != ColourFilter::All)
                + usize::from(state.customizable),
        }
    }

    /// The one Source the Items view is narrowed to (a set, catalogue or folder opened from Sets),
    /// with its items at this level: its name for the subject line.
    fn narrowed_to(&self, state: &BrowseState, rows: &[FacetRow]) -> Option<(String, usize)> {
        let [key] = state.sources.as_slice() else {
            return None;
        };
        if !state.narrowed {
            return None;
        }
        let label = rows.iter().find(|r| &r.key == key)?.label.clone();
        let in_source = self
            .list
            .items
            .iter()
            .filter(|item| item.parent == state.level)
            .filter(|item| {
                let source = &self.list.sources[item.source as usize];
                &source.key == key || (key == "local" && source.local)
            })
            .count();
        Some((label, in_source))
    }

    /// The words of item `index` for its tile, tooltip and the detail bar.
    #[must_use]
    pub fn tile(&self, index: u32) -> Option<TileWords> {
        let item = self.list.items.get(index as usize)?;
        Some(tile_words(
            &self.list,
            index,
            item,
            self.palette[index as usize],
        ))
    }

    /// The words of card `index`.
    #[must_use]
    pub fn card(&self, index: u32) -> Option<CardWords> {
        self.list
            .cards
            .get(index as usize)
            .map(|card| card_words(index, card))
    }
}
