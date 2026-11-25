//! The template rows of a Git catalogue in the Online group.

use std::collections::BTreeMap;

use curvyo_document_core::CURRENT_FORMAT_VERSION;
use curvyo_library_core::{Failure, TemplateEntry};

use crate::library_online::{
    OnlineExtra, OnlineSourceView, OnlineView, failed, kind_glyph, loading, status_row,
};
use crate::library_online_text::{counted, licence_line, refused_line, size_text, size_words};
use crate::library_tree::{LibraryRow, RowKind, matched, plain_row, push_pages, row_key};

/// The templates of one catalogue folder and its sub-folders.
#[derive(Default)]
struct Level<'a> {
    folders: BTreeMap<String, Level<'a>>,
    templates: Vec<&'a TemplateEntry>,
}

impl<'a> Level<'a> {
    fn count(&self) -> usize {
        self.templates.len() + self.folders.values().map(Level::count).sum::<usize>()
    }

    /// Files `entry` under the folders of its path.
    fn insert(&mut self, entry: &'a TemplateEntry) {
        let parts: Vec<&str> = entry.path.split('/').collect();
        let mut at = self;
        for folder in &parts[..parts.len() - 1] {
            at = at.folders.entry((*folder).to_owned()).or_default();
        }
        at.templates.push(entry);
    }
}

/// Whether a typed query keeps the template: its name or one of its tags matches.
fn kept(entry: &TemplateEntry, query: &str) -> bool {
    query.trim().is_empty()
        || matched(&entry.name, query).is_some()
        || entry.tags.iter().any(|t| matched(t, query).is_some())
}

/// The rows of a Git source's list: its folders and templates, or why there are none.
pub(crate) fn push_templates(
    rows: &mut Vec<LibraryRow>,
    source: &OnlineSourceView<'_>,
    view: &OnlineView<'_>,
) {
    let Some(catalogue) = source.catalogue else {
        rows.push(
            source
                .failure
                .map_or_else(|| loading(source, 1), |f| failed(source, 1, f)),
        );
        return;
    };
    let mut root = Level::default();
    for entry in catalogue.templates.iter().filter(|e| kept(e, view.query)) {
        root.insert(entry);
    }
    walk(rows, &root, source, view, &mut Vec::new(), 1);
    if rows.is_empty() && view.query.trim().is_empty() {
        rows.push(status_row(
            &source.key,
            1,
            format!("x:{}#empty", source.key),
            "No templates.",
            "failed",
        ));
    }
}

fn walk(
    rows: &mut Vec<LibraryRow>,
    level: &Level<'_>,
    source: &OnlineSourceView<'_>,
    view: &OnlineView<'_>,
    at: &mut Vec<String>,
    depth: usize,
) {
    let searching = !view.query.trim().is_empty();
    for (name, inner) in &level.folders {
        at.push(name.clone());
        let key = row_key(RowKind::Folder, &source.key, at, None);
        let expanded = searching || view.expanded.contains(&key);
        rows.push(LibraryRow {
            line: inner.count().to_string(),
            expandable: true,
            expanded,
            path: at.clone(),
            label: format!("{name}, folder, {}", counted(inner.count(), "template")),
            ..plain_row(key, RowKind::Folder, depth, name, &source.key)
        });
        if expanded {
            walk(rows, inner, source, view, at, depth + 1);
        }
        at.pop();
    }
    for entry in &level.templates {
        push_template(rows, source, view, entry, depth);
    }
}

/// Why a template cannot be loaded: its own refusal this session, a newer file or too large.
fn refusal(source: &OnlineSourceView<'_>, entry: &TemplateEntry) -> Option<Failure> {
    let newer = entry
        .format_version
        .is_some_and(|v| v > CURRENT_FORMAT_VERSION);
    source
        .refused
        .get(&entry.path)
        .copied()
        .or_else(|| newer.then_some(Failure::Newer))
        .or_else(|| entry.too_large().then_some(Failure::TooLarge))
}

/// The second line and the accessible name of a template row.
fn lines(
    entry: &TemplateEntry,
    refused: Option<Failure>,
    licence: &str,
    size: &str,
) -> (String, String) {
    let author = entry.author.as_str();
    let mut parts: Vec<&str> = Vec::new();
    if !author.is_empty() {
        parts.push(author);
    }
    parts.push(licence);
    parts.push(size);
    let line = refused.map_or_else(|| parts.join(" · "), refused_line);
    let by = if author.is_empty() {
        String::new()
    } else {
        format!(" by {author}")
    };
    let mut label = format!(
        "{}, template{by}, licence {licence}, {}",
        entry.name,
        size_words(entry.size)
    );
    if entry.fonts_withheld() {
        label.push_str(", font not included");
    }
    (line, label)
}

fn push_template(
    rows: &mut Vec<LibraryRow>,
    source: &OnlineSourceView<'_>,
    view: &OnlineView<'_>,
    entry: &TemplateEntry,
    level: usize,
) {
    let path: Vec<String> = entry.path.split('/').map(str::to_owned).collect();
    let key = row_key(RowKind::Template, &source.key, &path, None);
    let (licence, licence_url) = licence_line(entry.licence.as_ref());
    let refused = refusal(source, entry);
    let size = size_text(entry.size);
    let (line, label) = lines(entry, refused, &licence, &size);
    let expandable = refused.is_none();
    let expanded = expandable && view.expanded.contains(&key);
    rows.push(LibraryRow {
        line,
        expandable,
        expanded,
        unreadable: refused.is_some(),
        path,
        picture: entry.thumbnail_sha256.is_some() && refused.is_none(),
        matched: matched(&entry.name, view.query),
        label,
        online: Some(OnlineExtra {
            glyph: kind_glyph(source.kind).to_owned(),
            author: entry.author.clone(),
            licence,
            licence_url,
            size,
            font_note: if entry.fonts_withheld() {
                "Font not included".to_owned()
            } else {
                String::new()
            },
            thumbnail: entry.thumbnail_sha256.is_some(),
            loadable: refused.is_none(),
            ..OnlineExtra::default()
        }),
        ..plain_row(key, RowKind::Template, level, &entry.name, &source.key)
    });
    if expanded {
        match source.templates.get(&entry.path) {
            Some(document) => push_pages(rows, document, None, level + 1, ""),
            None => rows.push(loading(source, level + 1)),
        }
    }
}
