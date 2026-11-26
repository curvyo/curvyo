//! The cards of the library browser's Sets view (spec `0152` criteria 26 to 28, UX notes 0.2):
//! one card per set, catalogue, folder or unfetched source, loaded or not, in groups by source.

use curvyo_library_core::{Failure, READY_HOST, SourceKind, host_of, is_ready};

use crate::library_browser::{BrowseCard, BrowseGroup, BrowseList, CardKind};
use crate::library_browser_items::LocalView;
use crate::library_index::DocumentState;
use crate::library_online::{OnlineSourceView, OnlineView, READY_NAME, kind_glyph};
use crate::library_token_text::source_failure_text;
use crate::workspace_rows::natural_cmp;

/// The local group's key.
pub const LOCAL_GROUP: &str = "local";
/// The ready Curvyo catalogue's group key while it is not added.
pub const READY_GROUP: &str = "ready";

fn card(kind: CardKind, name: &str, group: &str) -> BrowseCard {
    BrowseCard {
        kind,
        name: name.to_owned(),
        key: String::new(),
        host: String::new(),
        glyph: String::new(),
        prefix: String::new(),
        samples: Vec::new(),
        total: 0,
        category: None,
        palette: None,
        licence: None,
        author: String::new(),
        failure: String::new(),
        offline: false,
        loaded: false,
        group: group.to_owned(),
        narrow: String::new(),
        count: 0,
        pages: 0,
    }
}

fn failure_of(kind: SourceKind, token: bool, failure: Option<Failure>) -> (String, bool) {
    failure.map_or((String::new(), false), |f| {
        (source_failure_text(kind, token, f), f == Failure::Offline)
    })
}

fn push_folders(list: &mut BrowseList, local: LocalView<'_>) {
    let mut any = false;
    for source in local.sources.iter().filter(|s| s.error.is_none()) {
        any = true;
        let documents: Vec<_> = local
            .index
            .documents()
            .iter()
            .filter(|d| d.source == source.key && d.state == DocumentState::Readable)
            .collect();
        list.cards.push(BrowseCard {
            key: source.key.clone(),
            glyph: "folder".to_owned(),
            narrow: format!("local:{}", source.key),
            loaded: true,
            count: documents.len() as u64,
            pages: documents.iter().map(|d| d.pages.len() as u64).sum(),
            ..card(CardKind::Folder, &source.name, LOCAL_GROUP)
        });
    }
    if any {
        list.groups.push(BrowseGroup {
            key: LOCAL_GROUP.to_owned(),
            label: "My documents".to_owned(),
            rank: 0,
            private: false,
        });
    }
}

fn push_catalogue(list: &mut BrowseList, view: &OnlineSourceView<'_>) {
    let host = host_of(view.address).unwrap_or_default();
    let (failure, offline) = failure_of(view.kind, view.token, view.failure);
    let base = BrowseCard {
        key: view.key.clone(),
        host: host.clone(),
        glyph: "git".to_owned(),
        narrow: view.key.clone(),
        failure,
        offline,
        ..card(CardKind::Source, view.name, &view.key)
    };
    list.cards.push(match view.catalogue {
        Some(catalogue) => BrowseCard {
            kind: CardKind::Catalogue,
            loaded: true,
            count: catalogue.templates.len() as u64,
            ..base
        },
        None => base,
    });
}

fn push_iconify(list: &mut BrowseList, view: &OnlineSourceView<'_>) {
    let host = host_of(view.address).unwrap_or_default();
    let Some(sets) = view.sets else {
        let (failure, offline) = failure_of(view.kind, view.token, view.failure);
        list.cards.push(BrowseCard {
            key: view.key.clone(),
            host,
            glyph: kind_glyph(view.kind).to_owned(),
            failure,
            offline,
            ..card(CardKind::Source, view.name, &view.key)
        });
        return;
    };
    for set in sets {
        let (failure, offline) = failure_of(
            view.kind,
            view.token,
            view.set_failures.get(&set.prefix).copied(),
        );
        list.cards.push(BrowseCard {
            key: view.key.clone(),
            host: host.clone(),
            glyph: "iconify".to_owned(),
            prefix: set.prefix.clone(),
            samples: set.samples.clone(),
            total: set.total,
            category: set.category.clone(),
            palette: set.palette,
            licence: set.licence.clone(),
            author: set.author.clone(),
            failure,
            offline,
            loaded: view.set_icons.contains_key(&set.prefix),
            narrow: format!("set:{}#{}", view.address, set.prefix),
            ..card(CardKind::Set, &set.name, &view.key)
        });
    }
}

/// The cards and groups of the Sets view (criterion 26): My documents, the Curvyo catalogue, the
/// Git catalogues A to Z, the Iconify sources A to Z; inside a group A to Z.
pub(crate) fn push_cards(
    list: &mut BrowseList,
    local: Option<LocalView<'_>>,
    online: &OnlineView<'_>,
) {
    if let Some(local) = local {
        push_folders(list, local);
    }
    if online.ready_offered {
        list.groups.push(BrowseGroup {
            key: READY_GROUP.to_owned(),
            label: READY_NAME.to_owned(),
            rank: 1,
            private: false,
        });
        list.cards.push(BrowseCard {
            host: READY_HOST.to_owned(),
            glyph: "ready".to_owned(),
            ..card(CardKind::Ready, READY_NAME, READY_GROUP)
        });
    }
    for view in &online.sources {
        let (rank, label) = match view.kind {
            SourceKind::Git if is_ready(view.address) => (1, view.name.to_owned()),
            SourceKind::Git => (2, view.name.to_owned()),
            SourceKind::Iconify => (
                3,
                host_of(view.address).unwrap_or_else(|| view.name.to_owned()),
            ),
        };
        list.groups.push(BrowseGroup {
            key: view.key.clone(),
            label,
            rank,
            private: view.token,
        });
        match view.kind {
            SourceKind::Git => push_catalogue(list, view),
            SourceKind::Iconify => push_iconify(list, view),
        }
    }
    list.groups.sort_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then_with(|| natural_cmp(&a.label, &b.label))
    });
}
