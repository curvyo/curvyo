//! The rows an Iconify source shows: icon sets, icon cells and search results.

use curvyo_library_core::{Failure, IconSet};

use crate::library_online::{
    CELLS_PER_ROW, IconCell, OnlineExtra, OnlineSourceView, OnlineView, failed, loading, status_row,
};
use crate::library_online_text::{
    LICENCE_UNKNOWN, failure_text, icons_text, licence_line, searching_row_text,
};
use crate::library_tree::{LibraryRow, RowKind, matched, plain_row, row_key};

fn cell(set: Option<&IconSet>, prefix: &str, name: &str) -> IconCell {
    let (set_name, licence) = set.map_or_else(
        || (prefix.to_owned(), LICENCE_UNKNOWN.to_owned()),
        |s| (s.name.clone(), licence_line(s.licence.as_ref()).0),
    );
    IconCell {
        prefix: prefix.to_owned(),
        name: name.to_owned(),
        label: format!("{name}, icon, {set_name}, licence {licence}"),
        tooltip: format!("{name}\n{set_name} · {licence}"),
    }
}

fn icon_rows(
    rows: &mut Vec<LibraryRow>,
    source: &OnlineSourceView<'_>,
    level: usize,
    tag: &str,
    cells: &[IconCell],
) {
    for (n, chunk) in cells.chunks(CELLS_PER_ROW).enumerate() {
        rows.push(LibraryRow {
            label: format!("{} icons", chunk.len()),
            online: Some(OnlineExtra {
                cells: chunk.to_vec(),
                ..OnlineExtra::default()
            }),
            ..plain_row(
                format!("i:{}#{tag}#{n}", source.key),
                RowKind::Icons,
                level,
                "",
                &source.key,
            )
        });
    }
}

/// The set `prefix` from the source's list, else from the info a search answer carried.
fn set_of<'a>(source: &'a OnlineSourceView<'_>, prefix: &str) -> Option<&'a IconSet> {
    source
        .sets
        .unwrap_or_default()
        .iter()
        .find(|s| s.prefix == prefix)
        .or_else(|| {
            source
                .search
                .and_then(|s| s.sets.iter().find(|x| x.prefix == prefix))
        })
}

/// The results row of the last Enter search and its icon cells (UX notes 7.2).
fn push_search(rows: &mut Vec<LibraryRow>, source: &OnlineSourceView<'_>, query: &str) {
    // The results leave with the search text; they stay held for the session (UX notes 7.2).
    let Some(search) = source.search.filter(|_| !query.trim().is_empty()) else {
        return;
    };
    let (text, status) = match &search.outcome {
        _ if search.running => (searching_row_text(&search.words), "loading"),
        Ok(icons) if icons.is_empty() => (format!("\"{}\": no icons", search.words), "search"),
        Ok(icons) => (
            format!("\"{}\" · {} icons", search.words, icons.len()),
            "search",
        ),
        Err(Failure::Offline) => ("Offline".to_owned(), "offline"),
        Err(failure) => (failure_text(source.kind, *failure), "failed"),
    };
    let key = format!("x:{}#search", source.key);
    rows.push(status_row(&source.key, 1, key, &text, status));
    if let Ok(icons) = &search.outcome {
        let cells: Vec<IconCell> = icons
            .iter()
            .filter(|i| query.trim().is_empty() || matched(&i.name, query).is_some())
            .map(|i| cell(set_of(source, &i.prefix), &i.prefix, &i.name))
            .collect();
        icon_rows(rows, source, 1, "search", &cells);
    }
}

fn set_row(
    set: &IconSet,
    source: &OnlineSourceView<'_>,
    key: String,
    expanded: bool,
    query: &str,
) -> LibraryRow {
    let (licence, licence_url) = licence_line(set.licence.as_ref());
    LibraryRow {
        line: icons_text(set.total),
        expandable: true,
        expanded,
        path: vec![set.prefix.clone()],
        matched: matched(&set.name, query),
        label: format!(
            "{}, icon set, licence {licence}, {}",
            set.name,
            icons_text(set.total)
        ),
        online: Some(OnlineExtra {
            glyph: "iconify".to_owned(),
            author: set.author.clone(),
            licence,
            licence_url,
            size: icons_text(set.total),
            samples: set
                .samples
                .iter()
                .map(|n| cell(Some(set), &set.prefix, n))
                .collect(),
            ..OnlineExtra::default()
        }),
        ..plain_row(key, RowKind::Set, 1, &set.name, &source.key)
    }
}

/// Whether a typed query keeps the set: its name, prefix or an icon of it matches.
fn set_kept(set: &IconSet, icons: Option<&Vec<String>>, query: &str) -> bool {
    query.trim().is_empty()
        || matched(&set.name, query).is_some()
        || matched(&set.prefix, query).is_some()
        || icons.is_some_and(|names| names.iter().any(|n| matched(n, query).is_some()))
}

fn push_set(
    rows: &mut Vec<LibraryRow>,
    set: &IconSet,
    source: &OnlineSourceView<'_>,
    view: &OnlineView<'_>,
) {
    let query = view.query;
    let searching = !query.trim().is_empty();
    let icons = source.set_icons.get(&set.prefix).map(|l| &l.names);
    if !set_kept(set, icons, query) {
        return;
    }
    let key = row_key(
        RowKind::Set,
        &source.key,
        std::slice::from_ref(&set.prefix),
        None,
    );
    let expanded = view.expanded.contains(&key) || (searching && icons.is_some());
    rows.push(set_row(set, source, key, expanded, query));
    if !expanded {
        return;
    }
    let Some(names) = icons else {
        rows.push(loading(source, 2));
        return;
    };
    let cells: Vec<IconCell> = names
        .iter()
        .filter(|n| {
            !searching || matched(n, query).is_some() || matched(&set.name, query).is_some()
        })
        .map(|n| cell(Some(set), &set.prefix, n))
        .collect();
    icon_rows(rows, source, 2, &set.prefix, &cells);
}

/// The rows of an Iconify source's list: the search results on top, then the sets.
pub(crate) fn push_iconify(
    rows: &mut Vec<LibraryRow>,
    source: &OnlineSourceView<'_>,
    view: &OnlineView<'_>,
) {
    push_search(rows, source, view.query);
    let Some(listed) = source.sets else {
        if source.search.is_none() || source.failure.is_some() {
            rows.push(
                source
                    .failure
                    .map_or_else(|| loading(source, 1), |f| failed(source, 1, f)),
            );
        }
        return;
    };
    for set in listed {
        push_set(rows, set, source, view);
    }
}
