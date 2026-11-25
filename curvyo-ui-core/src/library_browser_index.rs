//! Builds the browser's search index from the list and, once it has arrived, the keyword file
//! (spec `0152` criterion 19, spec `0153` criteria 1, 2 and 9, ADR 0087 decisions 4 and 5).

use curvyo_library_core::{
    Keywords, SearchIndex, WEIGHT_ALIAS, WEIGHT_CATEGORY, WEIGHT_KEYWORD, WEIGHT_NAME,
    WEIGHT_SOURCE, WEIGHT_TAG,
};

use crate::library_browser::{BrowseList, ItemKind};

/// The index of `list`: per item its name, aliases, tags, categories, source and page names, and
/// for an Iconify icon the keyword and category words `keywords` has for its prefix and name.
pub(crate) fn build_index(
    list: &BrowseList,
    categories: &[(String, String)],
    item_categories: &[Vec<u16>],
    keywords: Option<&Keywords>,
) -> SearchIndex {
    let mut index = SearchIndex::new();
    for (n, item) in list.items.iter().enumerate() {
        let source = &list.sources[item.source as usize];
        let mut fields: Vec<(&str, u8)> = vec![(&item.name, WEIGHT_NAME)];
        fields.extend(item.aliases.iter().map(|a| (a.as_str(), WEIGHT_ALIAS)));
        fields.extend(item.tags.iter().map(|t| (t.as_str(), WEIGHT_TAG)));
        fields.extend(
            item_categories[n]
                .iter()
                .filter_map(|c| categories.get(usize::from(*c)))
                .filter(|(key, _)| !key.is_empty())
                .map(|(_, label)| (label.as_str(), WEIGHT_CATEGORY)),
        );
        let published = keywords
            .filter(|_| item.kind == ItemKind::Icon && !source.prefix.is_empty())
            .and_then(|k| k.of(&source.prefix, &item.name));
        if let Some(words) = published {
            fields.extend(words.keywords().map(|w| (w, WEIGHT_KEYWORD)));
            fields.extend(words.categories().map(|w| (w, WEIGHT_CATEGORY)));
        }
        fields.push((&source.label, WEIGHT_SOURCE));
        fields.extend(item.page_names.iter().map(|p| (p.as_str(), WEIGHT_SOURCE)));
        if let Some(parent) = item.parent.and_then(|p| list.items.get(p as usize)) {
            fields.push((&parent.name, WEIGHT_SOURCE));
        }
        index.push(&fields);
    }
    if let Some(keywords) = keywords {
        index.set_related(keywords);
    }
    index
}
