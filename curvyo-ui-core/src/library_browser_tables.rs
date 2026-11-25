//! The tables the browser builds once per list (spec `0152` criteria 12, 13 and 17): the
//! category of every item and card, the Source rows, and the Name, Source and Newest orders.

use std::cmp::Ordering;
use std::collections::HashMap;

use curvyo_library_core::fold;

use crate::library_browser::{BrowseItem, BrowseList, ItemKind};
use crate::library_browser_query::id;
use crate::library_browser_text::UNCATEGORISED;
use crate::workspace_rows::natural_cmp;

pub(crate) type Categories = (Vec<(String, String)>, Vec<Vec<u16>>);

/// The category table and each item's categories (criterion 13): an icon's set
/// category and groups, a local item's folder, a template's tags equal to one of those, merged by
/// folded name; an item with none is "Uncategorised".
pub(crate) fn categories_of(list: &BrowseList) -> Categories {
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut named: Vec<Vec<String>> = Vec::with_capacity(list.items.len());
    for item in &list.items {
        let source = &list.sources[item.source as usize];
        let mut own: Vec<String> = Vec::new();
        match item.kind {
            ItemKind::Icon => {
                own.extend(source.category.clone());
                own.extend(item.groups.iter().cloned());
            }
            ItemKind::Page | ItemKind::Document if source.local => own.push(source.label.clone()),
            _ => {}
        }
        for name in &own {
            labels.entry(fold(name)).or_insert_with(|| name.clone());
        }
        named.push(own);
    }
    for (n, item) in list.items.iter().enumerate() {
        if item.kind == ItemKind::Template || (item.kind == ItemKind::Page && !item.tags.is_empty())
        {
            named[n].extend(
                item.tags
                    .iter()
                    .filter(|t| labels.contains_key(&fold(t)))
                    .cloned(),
            );
        }
    }
    let mut table: Vec<(String, String)> = labels.into_iter().collect();
    table.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| a.0.cmp(&b.0)));
    table.push((String::new(), UNCATEGORISED.to_owned()));
    let at: HashMap<&str, u16> = table
        .iter()
        .enumerate()
        .filter_map(|(n, (key, _))| u16::try_from(n).ok().map(|n| (key.as_str(), n)))
        .collect();
    let none = u16::try_from(table.len() - 1).unwrap_or(u16::MAX);
    let ids = |names: &[String]| -> Vec<u16> {
        let mut out: Vec<u16> = names
            .iter()
            .filter_map(|name| at.get(fold(name).as_str()).copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        if out.is_empty() {
            out.push(none);
        }
        out
    };
    let items = named.iter().map(|n| ids(n)).collect();
    (table, items)
}

pub(crate) type SourceRows = (Vec<(String, String, bool)>, Vec<(usize, Option<usize>)>);

/// The Source rows (criterion 12): "My documents" with a row per workspace or folder when there
/// are several, then each catalogue, each loaded set, and "Search results".
pub(crate) fn source_rows(list: &BrowseList) -> SourceRows {
    let locals = list.sources.iter().filter(|s| s.local).count();
    let mut rows: Vec<(String, String, bool)> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    let parent = (locals > 0).then(|| {
        rows.push(("local".to_owned(), "My documents".to_owned(), false));
        0
    });
    let mut of = Vec::with_capacity(list.sources.len());
    for source in &list.sources {
        if source.local && locals == 1 {
            of.push((parent.unwrap_or(0), None));
            continue;
        }
        let label = if source.key == "search" {
            "Search results".to_owned()
        } else {
            source.label.clone()
        };
        let row = *at.entry(source.key.clone()).or_insert_with(|| {
            rows.push((source.key.clone(), label, source.local));
            rows.len() - 1
        });
        of.push((row, parent.filter(|_| source.local)));
    }
    // "Search results" last.
    if let Some(search_row) = rows.iter().position(|r| r.0 == "search") {
        let moved = rows.remove(search_row);
        rows.push(moved);
        let end = rows.len() - 1;
        for entry in &mut of {
            if entry.0 == search_row {
                entry.0 = end;
            } else if entry.0 > search_row {
                entry.0 -= 1;
            }
        }
    }
    (rows, of)
}

/// A key that sorts as [`natural_cmp`] over folded text: a character by its code, a run of digits
/// by its value, placed where its first digit's code would sort.
pub(crate) fn natural_key(text: &str) -> Vec<u128> {
    let mut key = Vec::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut run = String::from(c);
            while let Some(d) = chars.next_if(char::is_ascii_digit) {
                run.push(d);
            }
            let trimmed = run.trim_start_matches('0');
            let digits = &trimmed[..trimmed.len().min(24)];
            let value: u128 = digits.parse().unwrap_or(0);
            let len = u128::try_from(trimmed.len().min(0xff)).unwrap_or(0xff);
            key.push((u128::from('0') << 100) | (len << 80) | value);
        } else {
            key.push(u128::from(u32::from(c)) << 100);
        }
    }
    key
}

/// The Name, Source and Newest orders (criterion 17), each stable.
pub(crate) fn orders_of(
    list: &BrowseList,
    row_of_source: &[(usize, Option<usize>)],
) -> [Vec<u32>; 3] {
    let folded: Vec<String> = list.items.iter().map(|i| fold(&i.name)).collect();
    let mut name: Vec<u32> = (0..list.items.len()).map(id).collect();
    name.sort_by_cached_key(|i| natural_key(&folded[*i as usize]));
    let mut source = name.clone();
    source.sort_by_key(|i| row_of_source[list.items[*i as usize].source as usize].0);
    let mut newest = name.clone();
    newest.sort_by(|a, b| {
        let (a, b) = (&list.items[*a as usize], &list.items[*b as usize]);
        let local = |i: &BrowseItem| list.sources[i.source as usize].local;
        match (local(a), local(b)) {
            (true, true) => b.modified.cmp(&a.modified),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => Ordering::Equal,
        }
    });
    [name, source, newest]
}
