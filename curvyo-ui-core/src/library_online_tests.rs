use super::*;
use curvyo_library_core::{SetListing, parse_collections, parse_search};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../curvyo-library-core/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

struct Held {
    catalogue: Option<Catalogue>,
    sets: Option<Vec<IconSet>>,
    set_icons: HashMap<String, SetListing>,
    set_failures: HashMap<String, Failure>,
    search: Option<SearchView>,
    templates: HashMap<String, IndexedDocument>,
    refused: HashMap<String, Failure>,
}

impl Held {
    fn git() -> Self {
        Self {
            catalogue: Some(Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap()),
            sets: None,
            set_icons: HashMap::new(),
            set_failures: HashMap::new(),
            search: None,
            templates: HashMap::new(),
            refused: HashMap::new(),
        }
    }

    fn iconify() -> Self {
        Self {
            catalogue: None,
            sets: Some(parse_collections(&fixture("iconify_collections.json")).unwrap()),
            ..Self::git()
        }
        .without_catalogue()
    }

    fn without_catalogue(mut self) -> Self {
        self.catalogue = None;
        self
    }

    fn view<'a>(
        &'a self,
        kind: SourceKind,
        failure: Option<Failure>,
        expanded: &'a HashSet<String>,
        query: &'a str,
        ready_offered: bool,
    ) -> OnlineView<'a> {
        let (name, address) = match kind {
            SourceKind::Git => ("Curvyo catalogue", "https://example.org/cat/"),
            SourceKind::Iconify => ("api.iconify.design", "https://api.iconify.design"),
        };
        OnlineView {
            sources: vec![OnlineSourceView {
                key: format!("online:{address}"),
                name,
                address,
                kind,
                failure,
                catalogue: self.catalogue.as_ref(),
                sets: self.sets.as_deref(),
                set_icons: &self.set_icons,
                set_failures: &self.set_failures,
                search: self.search.as_ref(),
                templates: &self.templates,
                refused: &self.refused,
            }],
            ready_offered,
            expanded,
            query,
        }
    }
}

fn open(key: &str) -> HashSet<String> {
    std::iter::once(key.to_owned()).collect()
}

#[test]
fn with_no_source_only_the_ready_entry_is_offered() {
    let held = Held::git();
    let none = HashSet::new();
    let view = OnlineView {
        sources: Vec::new(),
        ready_offered: true,
        expanded: &none,
        query: "",
    };
    let rows = online_rows(&view);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind, RowKind::Heading);
    assert_eq!(rows[0].name, "Online");
    assert_eq!(rows[1].kind, RowKind::Ready);
    assert_eq!(
        rows[1].online.as_ref().unwrap().host,
        "raw.githubusercontent.com"
    );
    drop(held);
    let nothing = OnlineView {
        ready_offered: false,
        ..view
    };
    assert!(
        online_rows(&nothing).is_empty(),
        "no heading without anything to list"
    );
}

#[test]
fn a_collapsed_source_row_states_its_count_and_label() {
    let held = Held::git();
    let none = HashSet::new();
    let rows = online_rows(&held.view(SourceKind::Git, None, &none, "", false));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].online.as_ref().unwrap().state, "4");
    assert_eq!(
        rows[1].label,
        "Curvyo catalogue, Git catalogue, 4 templates"
    );
    assert!(!rows[1].expanded);
}

#[test]
fn an_expanded_catalogue_lists_folders_and_templates_with_their_licence_line() {
    let held = Held::git();
    let expanded = open("s:online:https://example.org/cat/\u{1f}");
    let rows = online_rows(&held.view(SourceKind::Git, None, &expanded, "", false));
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Online",
            "Curvyo catalogue",
            "big",
            "boxes",
            "misc",
            "signs"
        ]
    );
    let mut all = expanded.clone();
    for r in &rows {
        if r.kind == RowKind::Folder {
            all.insert(r.key.clone());
        }
    }
    let rows = online_rows(&held.view(SourceKind::Git, None, &all, "", false));
    let finger = rows.iter().find(|r| r.name == "Finger box").unwrap();
    assert_eq!(finger.line, "Jane Doe · CC0 1.0 · 3 B");
    assert_eq!(
        finger.label,
        "Finger box, template by Jane Doe, licence CC0 1.0, 3 bytes"
    );
    let extra = finger.online.as_ref().unwrap();
    assert_eq!(
        extra.licence_url,
        "https://creativecommons.org/publicdomain/zero/1.0/"
    );
    assert!(extra.loadable && extra.thumbnail);
    let plate = rows.iter().find(|r| r.name == "Name plate").unwrap();
    assert_eq!(
        plate.online.as_ref().unwrap().font_note,
        "Font not included"
    );
    assert!(plate.label.ends_with("font not included"));
    assert!(plate.unreadable, "format 999 is newer than this build");
    assert_eq!(plate.line, "Cannot be loaded: made by a newer Curvyo.");
    let none = rows.iter().find(|r| r.name == "No licence stated").unwrap();
    assert_eq!(none.online.as_ref().unwrap().licence, "Licence unknown");
    assert_eq!(none.online.as_ref().unwrap().licence_url, "");
    let huge = rows.iter().find(|r| r.name == "Huge").unwrap();
    assert_eq!(huge.line, "Cannot be loaded: too large.");
}

#[test]
fn an_unfetched_source_shows_loading_a_failed_one_its_reason() {
    let mut held = Held::git();
    held.catalogue = None;
    let expanded = open("s:online:https://example.org/cat/\u{1f}");
    let rows = online_rows(&held.view(SourceKind::Git, None, &expanded, "", false));
    assert_eq!(rows[2].name, "Loading...");
    assert_eq!(rows[2].online.as_ref().unwrap().status, "loading");
    let rows = online_rows(&held.view(
        SourceKind::Git,
        Some(Failure::Offline),
        &expanded,
        "",
        false,
    ));
    assert_eq!(rows[1].online.as_ref().unwrap().state, "Offline");
    assert_eq!(rows[2].name, "Offline");
    assert_eq!(rows[2].online.as_ref().unwrap().status, "offline");
    let rows = online_rows(&held.view(
        SourceKind::Git,
        Some(Failure::NotFound),
        &expanded,
        "",
        false,
    ));
    assert_eq!(rows[2].name, "No Git catalogue at this address.");
    let rows = online_rows(&held.view(
        SourceKind::Git,
        Some(Failure::Status(500)),
        &expanded,
        "",
        false,
    ));
    assert_eq!(rows[2].name, "The server answered 500.");
}

#[test]
fn a_refused_template_does_not_expand_and_says_why() {
    let mut held = Held::git();
    held.refused
        .insert("boxes/finger-box".into(), Failure::Mismatch);
    let mut expanded = open("s:online:https://example.org/cat/\u{1f}");
    expanded.insert("f:online:https://example.org/cat/\u{1f}boxes".into());
    let rows = online_rows(&held.view(SourceKind::Git, None, &expanded, "", false));
    let finger = rows.iter().find(|r| r.name == "Finger box").unwrap();
    assert_eq!(
        finger.line,
        "Cannot be loaded: does not match the catalogue."
    );
    assert!(!finger.expandable);
}

#[test]
fn iconify_sets_open_into_a_grid_of_five() {
    let mut held = Held::iconify();
    held.set_icons.insert(
        "mdi".into(),
        SetListing {
            names: (0..7).map(|n| format!("icon-{n}")).collect(),
            ..SetListing::default()
        },
    );
    let mut expanded = open("s:online:https://api.iconify.design\u{1f}");
    expanded.insert("e:online:https://api.iconify.design\u{1f}mdi".into());
    let rows = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "", false));
    let mdi = rows
        .iter()
        .find(|r| r.name == "Material Design Icons")
        .unwrap();
    assert_eq!(mdi.line, "7,447 icons");
    assert_eq!(mdi.online.as_ref().unwrap().licence, "Apache 2.0");
    let grids: Vec<&LibraryRow> = rows.iter().filter(|r| r.kind == RowKind::Icons).collect();
    assert_eq!(grids.len(), 2);
    assert_eq!(grids[0].online.as_ref().unwrap().cells.len(), 5);
    assert_eq!(grids[1].online.as_ref().unwrap().cells.len(), 2);
    assert_eq!(
        grids[0].online.as_ref().unwrap().cells[0].label,
        "icon-0, icon, Material Design Icons, licence Apache 2.0"
    );
}

#[test]
fn search_results_sit_on_top_with_their_words_and_the_sets_licence() {
    let mut held = Held::iconify();
    let answer = parse_search(&fixture("iconify_search.json")).unwrap();
    held.search = Some(SearchView {
        words: "home".into(),
        outcome: Ok(answer.icons),
        running: false,
        sets: answer.sets,
    });
    let expanded = open("s:online:https://api.iconify.design\u{1f}");
    let rows = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "home", false));
    assert_eq!(rows[2].name, "\"home\" · 3 icons");
    assert_eq!(rows[2].online.as_ref().unwrap().status, "search");
    let grid = rows.iter().find(|r| r.kind == RowKind::Icons).unwrap();
    let cells = &grid.online.as_ref().unwrap().cells;
    assert_eq!(
        cells[0].label,
        "home, icon, Material Design Icons, licence Apache 2.0"
    );
    assert_eq!(
        cells[1].label,
        "home, icon, mdi-light, licence Licence unknown"
    );
}

#[test]
fn a_failed_or_empty_search_shows_on_its_own_source() {
    let mut held = Held::iconify();
    held.search = Some(SearchView {
        words: "zzz".into(),
        outcome: Ok(Vec::new()),
        running: false,
        sets: Vec::new(),
    });
    let expanded = open("s:online:https://api.iconify.design\u{1f}");
    let rows = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "home", false));
    assert_eq!(rows[2].name, "\"zzz\": no icons");
    held.search = Some(SearchView {
        words: "home".into(),
        outcome: Err(Failure::Offline),
        running: false,
        sets: Vec::new(),
    });
    let rows = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "home", false));
    assert_eq!(rows[2].name, "Offline");
    assert_eq!(rows[2].online.as_ref().unwrap().status, "offline");
}

#[test]
fn typing_filters_only_what_was_fetched() {
    let held = Held::git();
    let none = HashSet::new();
    let rows = online_rows(&held.view(SourceKind::Git, None, &none, "finger", false));
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Online", "Curvyo catalogue", "boxes", "Finger box"]);
    let rows = online_rows(&held.view(SourceKind::Git, None, &none, "nothing like it", false));
    assert_eq!(
        rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["Online", "Curvyo catalogue"]
    );
    // The ready entry is not offered while searching (nothing is fetched on its behalf).
    let offered = Held::git();
    assert_eq!(
        online_rows(&offered.view(SourceKind::Git, None, &none, "x", true))
            .iter()
            .filter(|r| r.kind == RowKind::Ready)
            .count(),
        0
    );
}

#[test]
fn words_of_failures_and_sizes() {
    assert_eq!(
        failure_text(SourceKind::Iconify, Failure::Invalid),
        "No Iconify API at this address."
    );
    assert_eq!(
        add_failure_text(SourceKind::Git, Failure::Offline),
        "Offline. Nothing was sent."
    );
    assert_eq!(
        add_failure_text(SourceKind::Git, Failure::Status(500)),
        "The server did not answer."
    );
    assert_eq!(
        load_failure_text("Box", Failure::Mismatch),
        "Box cannot be loaded: does not match the catalogue. Nothing was changed."
    );
    assert_eq!(size_text(24 * 1024), "24 KB");
    assert_eq!(size_text(1536 * 1024), "1.5 MB");
    assert_eq!(size_text(0), "0 B");
}

fn online_record(credit: Option<&str>) -> curvyo_document_core::CameFrom {
    curvyo_document_core::CameFrom {
        document: "Curvyo catalogue".into(),
        page: "Box".into(),
        page_id: "first".into(),
        item: None,
        at: curvyo_document_core::UnixMillis::new(1),
        online: Some(curvyo_document_core::OnlineOrigin {
            address: "https://example.org/cat/a/box/template.curvyo".into(),
            spdx: "CC-BY-4.0".into(),
            licence: "CC BY 4.0".into(),
            licence_url: "https://l.example/by".into(),
            author: "Jane Doe".into(),
            credit: credit.map(str::to_owned),
        }),
    }
}

#[test]
fn the_came_from_line_of_an_online_item_names_the_source_and_offers_no_open_source() {
    use crate::library_index_text::{came_from_selection_text, came_from_text};
    let record = online_record(None);
    assert_eq!(came_from_text(&record), "From Curvyo catalogue › Box");
    assert_eq!(
        came_from_selection_text(&[Some(record.clone())]),
        Some(("From Curvyo catalogue › Box".to_owned(), false))
    );
    let local = curvyo_document_core::CameFrom {
        online: None,
        ..record
    };
    assert_eq!(
        came_from_selection_text(&[Some(local)]).map(|(_, open)| open),
        Some(true)
    );
}

#[test]
fn the_credit_of_an_icon_and_the_notice_of_a_newer_list_are_worded_here() {
    assert_eq!(
        icon_credit_text("home", "Material", "Pictogrammers", "CC BY 4.0"),
        "home from Material by Pictogrammers, CC BY 4.0"
    );
    assert_eq!(
        icon_credit_text("home", "Material", "", "CC BY 4.0"),
        "home from Material, CC BY 4.0"
    );
    assert_eq!(
        sources_newer_text(),
        "The online sources list was made by a newer Curvyo. Nothing was changed."
    );
    assert_eq!(LICENCE_UNKNOWN, "Licence unknown");
}

#[test]
fn a_source_row_says_offline_and_its_failed_children_name_the_reason() {
    let held = Held::iconify();
    let expanded = open("s:online:https://api.iconify.design\u{1f}");
    let mut sourceless = Held::iconify();
    sourceless.sets = None;
    let rows = online_rows(&sourceless.view(
        SourceKind::Iconify,
        Some(Failure::Offline),
        &expanded,
        "",
        false,
    ));
    assert_eq!(rows[1].online.as_ref().unwrap().state, "Offline");
    assert_eq!(rows[1].label, "api.iconify.design, icon API, Offline");
    assert_eq!(rows[2].name, "Offline");
    drop(held);
}

#[test]
fn a_running_search_says_so_and_cleared_text_takes_the_results_away() {
    let mut held = Held::iconify();
    held.search = Some(SearchView {
        words: "home".into(),
        outcome: Ok(Vec::new()),
        running: true,
        sets: Vec::new(),
    });
    let expanded = open("s:online:https://api.iconify.design\u{1f}");
    let rows = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "home", false));
    assert_eq!(rows[2].name, "Searching \"home\"...");
    assert_eq!(rows[2].online.as_ref().unwrap().status, "loading");
    let cleared = online_rows(&held.view(SourceKind::Iconify, None, &expanded, "", false));
    assert!(
        cleared
            .iter()
            .all(|r| r.online.as_ref().is_none_or(|o| o.status != "loading"))
    );
}

#[test]
fn announcements_and_counts_read_singular_and_per_source() {
    assert_eq!(counted(1, "template"), "1 template");
    assert_eq!(counted(4, "template"), "4 templates");
    assert_eq!(
        search_done_text("api.iconify.design", "home", Ok(48)),
        "api.iconify.design: 48 icons for home"
    );
    assert_eq!(search_done_text("a", "zzz", Ok(0)), "a: no icons for zzz");
    assert_eq!(
        search_done_text("a", "home", Err(Failure::Offline)),
        "a: Offline"
    );
    assert_eq!(
        failure_text(SourceKind::Git, Failure::NotFound),
        "No Git catalogue at this address."
    );
}
