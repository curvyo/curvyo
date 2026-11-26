//! The library browser's Sets view (spec `0152` criteria 26 and 27, UX notes 0.2): the cards by
//! group, searched by name, category and author, narrowed by Sources (the groups), Colour and
//! Category, with their counts.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use curvyo_library_core::{
    Scratch, SearchIndex, WEIGHT_CATEGORY, WEIGHT_NAME, WEIGHT_SOURCE, fold,
};

use crate::library_browser::{BrowseList, CardKind};
use crate::library_browser_query::{ColourFilter, FacetRow, id};
use crate::library_browser_sets_text::{group_heading, no_set_matches_text, sets_subject};
use crate::library_browser_text::UNCATEGORISED;
use crate::workspace_rows::natural_cmp;

/// What the maker chose in the Sets view; each view keeps its own (H10).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SetsState {
    /// The search field's text in this view.
    pub text: String,
    /// The selected groups by key; none is all.
    pub sources: Vec<String>,
    /// The selected set categories by key; none is all.
    pub categories: Vec<String>,
    /// The Colour strip (a set's palette flag).
    pub colour: ColourFilter,
}

/// One group as shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SetsGroup {
    /// The group key.
    pub key: String,
    /// "api.iconify.design · 280 sets · 4 loaded".
    pub heading: String,
    /// The group's name alone, so the Lock can follow it (`specs/0157` criterion 4).
    pub name: String,
    /// The source has an access token: the Lock mark after the name.
    pub private: bool,
    /// Its cards shown, in order.
    pub cards: Vec<u32>,
}

/// What the Sets view shows for one state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SetsAnswer {
    /// The groups with cards shown, in order.
    pub groups: Vec<SetsGroup>,
    /// All cards.
    pub total: usize,
    /// The cards shown.
    pub count: usize,
    /// The Sources rows (the groups).
    pub sources: Vec<FacetRow>,
    /// The Category rows; empty while no card has a category.
    pub categories: Vec<FacetRow>,
    /// Whether a card has a category.
    pub has_categories: bool,
    /// Whether a card has a palette flag.
    pub has_colour: bool,
    /// "196 sets · 4 loaded", "12 of 196 sets".
    pub subject: String,
    /// 'No set matches "hrat".'; empty while something is shown.
    pub empty: String,
    /// The active filters.
    pub active: usize,
}

/// The Sets view's tables, built once per list.
#[derive(Default)]
pub(crate) struct SetsTables {
    index: SearchIndex,
    /// `(key, label)`: A to Z, "Uncategorised" (key "") last.
    categories: Vec<(String, String)>,
    category_of: Vec<u16>,
    /// The cards A to Z.
    order: Vec<u32>,
    group_of: Vec<usize>,
}

impl SetsTables {
    pub(crate) fn new(list: &BrowseList) -> Self {
        let mut index = SearchIndex::new();
        for card in &list.cards {
            let mut fields: Vec<(&str, u8)> = vec![(&card.name, WEIGHT_NAME)];
            if let Some(category) = &card.category {
                fields.push((category, WEIGHT_CATEGORY));
            }
            fields.push((&card.author, WEIGHT_SOURCE));
            fields.push((&card.prefix, WEIGHT_SOURCE));
            index.push(&fields);
        }
        let mut labels: HashMap<String, String> = HashMap::new();
        for category in list.cards.iter().filter_map(|c| c.category.as_ref()) {
            labels
                .entry(fold(category))
                .or_insert_with(|| category.clone());
        }
        let mut categories: Vec<(String, String)> = labels.into_iter().collect();
        categories.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| a.0.cmp(&b.0)));
        categories.push((String::new(), UNCATEGORISED.to_owned()));
        let at: HashMap<&str, u16> = categories
            .iter()
            .enumerate()
            .filter_map(|(n, (key, _))| u16::try_from(n).ok().map(|n| (key.as_str(), n)))
            .collect();
        let none = u16::try_from(categories.len() - 1).unwrap_or(u16::MAX);
        let category_of = list
            .cards
            .iter()
            .map(|c| {
                c.category
                    .as_ref()
                    .and_then(|k| at.get(fold(k).as_str()).copied())
                    .unwrap_or(none)
            })
            .collect();
        let mut order: Vec<u32> = (0..list.cards.len()).map(id).collect();
        order.sort_by(|a, b| {
            natural_cmp(&list.cards[*a as usize].name, &list.cards[*b as usize].name)
        });
        let group_of = list
            .cards
            .iter()
            .map(|c| {
                list.groups
                    .iter()
                    .position(|g| g.key == c.group)
                    .unwrap_or(usize::MAX)
            })
            .collect();
        Self {
            index,
            categories,
            category_of,
            order,
            group_of,
        }
    }

    /// The groups that hold a card of `order`, each with its cards in that order.
    fn groups(&self, list: &BrowseList, order: &[u32]) -> Vec<SetsGroup> {
        let mut shown = Vec::new();
        for (g, group) in list.groups.iter().enumerate() {
            let cards: Vec<u32> = order
                .iter()
                .copied()
                .filter(|i| self.group_of[*i as usize] == g)
                .collect();
            if !cards.is_empty() {
                let all: Vec<_> = list.cards.iter().filter(|c| c.group == group.key).collect();
                shown.push(SetsGroup {
                    key: group.key.clone(),
                    heading: group_heading(&group.label, &all),
                    name: group.label.clone(),
                    private: group.private,
                    cards,
                });
            }
        }
        shown
    }

    /// The answer for `state`.
    pub(crate) fn query(
        &self,
        list: &BrowseList,
        state: &SetsState,
        scratch: &mut Scratch,
    ) -> SetsAnswer {
        let scores = self.index.scores(&state.text, scratch);
        let groups: HashSet<&str> = state.sources.iter().map(String::as_str).collect();
        let keys: HashSet<String> = state.categories.iter().map(|k| fold(k)).collect();
        let mut group_counts = vec![0usize; list.groups.len()];
        let mut category_counts = vec![0usize; self.categories.len()];
        let mut shown = vec![false; list.cards.len()];
        for (n, card) in list.cards.iter().enumerate() {
            let text = scores.is_none_or(|s| s[n] > 0);
            let src = groups.is_empty() || groups.contains(card.group.as_str());
            let category = &self.categories[usize::from(self.category_of[n])].0;
            let cat = keys.is_empty() || keys.contains(category);
            let col = match state.colour {
                ColourFilter::All => true,
                ColourFilter::Mono => card.palette == Some(false),
                ColourFilter::Colour => card.palette == Some(true),
            };
            if text
                && cat
                && col
                && let Some(count) = group_counts.get_mut(self.group_of[n])
            {
                *count += 1;
            }
            if text && src && col {
                category_counts[usize::from(self.category_of[n])] += 1;
            }
            shown[n] = text && src && cat && col;
        }
        let mut order: Vec<u32> = self
            .order
            .iter()
            .copied()
            .filter(|i| shown[*i as usize])
            .collect();
        if let Some(scores) = scores {
            order.sort_by_key(|i| std::cmp::Reverse(scores[*i as usize]));
        }
        let shown_groups = self.groups(list, &order);
        let has_categories = self.categories.len() > 1;
        // The subject counts sets only; "loaded" as the cards' Loaded state (sets and catalogues).
        let loaded = list
            .cards
            .iter()
            .filter(|c| c.loaded && c.kind != CardKind::Folder)
            .count();
        let sets_total = list
            .cards
            .iter()
            .filter(|c| c.kind == CardKind::Set)
            .count();
        let sets_shown = order
            .iter()
            .filter(|i| list.cards[**i as usize].kind == CardKind::Set)
            .count();
        let count = order.len();
        let active = usize::from(!state.sources.is_empty())
            + usize::from(!state.categories.is_empty())
            + usize::from(state.colour != ColourFilter::All);
        SetsAnswer {
            groups: shown_groups,
            total: list.cards.len(),
            count,
            sources: list
                .groups
                .iter()
                .zip(group_counts)
                .map(|(g, count)| FacetRow {
                    key: g.key.clone(),
                    label: g.label.clone(),
                    count,
                    indent: false,
                })
                .collect(),
            categories: self
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
                .collect(),
            has_categories,
            has_colour: list.cards.iter().any(|c| c.palette.is_some()),
            subject: sets_subject(sets_shown, sets_total, loaded),
            empty: if count > 0 {
                String::new()
            } else if list.cards.is_empty() {
                "Nothing in the library yet.".to_owned()
            } else {
                no_set_matches_text(state.text.trim())
            },
            active,
        }
    }
}
