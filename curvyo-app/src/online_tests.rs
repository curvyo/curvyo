//! The host's online logic against a recording fake transport: what is sent, to whom and when,
//! and what the rows show after a failure (`specs/0138` criteria 1, 2, 5, 8 and 10).

#![allow(clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use curvyo_document_core::Document;
use curvyo_library_core::{Request, sha256_hex};
use curvyo_storage_io::HttpError;

use super::*;
use curvyo_library_core::{Millis, READY_ADDRESS};
use curvyo_ui_core::library_online::READY_NAME;

/// The fake `Transport`: canned answers by URL, every URL asked recorded.
#[derive(Default)]
struct Fake {
    answers: HashMap<String, Result<Vec<u8>, HttpError>>,
    asked: Mutex<Vec<String>>,
    /// The URLs asked with a token (the token itself cannot be read back, by design).
    bearer: Mutex<Vec<String>>,
}

impl Fake {
    fn with(mut self, url: &str, answer: Result<Vec<u8>, HttpError>) -> Self {
        self.answers.insert(url.to_owned(), answer);
        self
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl Transport for Fake {
    fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError> {
        self.asked.lock().unwrap().push(request.url().to_owned());
        let answer = self
            .answers
            .get(request.url())
            .cloned()
            .unwrap_or(Err(HttpError::Connection))?;
        if answer.len() > max_bytes {
            return Err(HttpError::TooLarge);
        }
        Ok(answer)
    }
}

impl Fake {
    fn bearer_asked(&self) -> Vec<String> {
        self.bearer.lock().unwrap().clone()
    }
}

/// The same double also records the requests that carry a token.
struct Bearing(Fake);

impl Transport for Bearing {
    fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError> {
        self.0.get(request, max_bytes)
    }

    fn get_bearer(
        &self,
        request: &Request,
        max_bytes: usize,
        _token: &curvyo_storage_io::AccessToken,
    ) -> Result<Vec<u8>, HttpError> {
        self.0.bearer.lock().unwrap().push(request.url().to_owned());
        self.0.get(request, max_bytes)
    }
}

/// Enter in the search as the commands run it: mark, then one request per Iconify source.
fn search_all(state: &OnlineState, fake: &Fake, sources: &Sources, text: &str) -> Vec<String> {
    let Some((words, keys)) = state.search_begin(sources, text) else {
        return Vec::new();
    };
    for key in &keys {
        let entry = sources.find(key).unwrap();
        state.search_one(fake, entry, &words);
    }
    keys
}

const GIT: &str = "https://example.org/cat/";
const ICONS: &str = "https://api.iconify.design";

fn git() -> SourceEntry {
    SourceEntry {
        kind: SourceKind::Git,
        address: GIT.into(),
        name: "Cat".into(),
        added: Millis(1),
        token: false,
    }
}

fn iconify() -> SourceEntry {
    SourceEntry {
        kind: SourceKind::Iconify,
        address: ICONS.into(),
        name: "api.iconify.design".into(),
        added: Millis(2),
        token: false,
    }
}

fn sources(entries: &[SourceEntry]) -> Sources {
    let mut sources = Sources::default();
    for e in entries {
        sources.add(e.kind, &e.address, &e.name, e.added).unwrap();
    }
    sources
}

fn index(template: &[u8]) -> Vec<u8> {
    format!(
        r#"{{"catalogue_format":1,"name":"Cat","templates":[{{"path":"a/box","name":"Box","author":"Jane","licence":{{"spdx":"CC-BY-4.0","name":"CC BY 4.0","url":"https://l.example/by"}},"credit":"Box by Jane, CC BY 4.0","size":{},"sha256":"{}"}}]}}"#,
        template.len(),
        sha256_hex(template)
    )
    .into_bytes()
}

fn container() -> Vec<u8> {
    curvyo_document_core::pack(&Document::new(1), "t").unwrap()
}

#[test]
fn nothing_is_asked_while_rows_are_built_or_before_the_maker_acts() {
    let fake = Fake::default();
    let state = OnlineState::default();
    let rows = state.rows(&sources(&[git(), iconify()]), &HashSet::new(), "");
    assert!(rows.iter().any(|r| r.name == "Cat"));
    let typed = state.rows(&sources(&[git(), iconify()]), &HashSet::new(), "home");
    assert!(typed.len() > 1);
    assert_eq!(fake.asked(), Vec::<String>::new());
}

#[test]
fn a_failed_source_is_offline_and_the_other_keeps_its_state() {
    let template = container();
    let fake = Fake::default().with(
        "https://example.org/cat/curvyo-catalogue.json",
        Ok(index(&template)),
    );
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    state.fetch_list(&fake, &iconify());
    let mut open = HashSet::new();
    open.insert("s:online:https://api.iconify.design\u{1f}".to_owned());
    let rows = state.rows(&sources(&[git(), iconify()]), &open, "");
    let cat = rows.iter().find(|r| r.name == "Cat").unwrap();
    assert_eq!(cat.online.as_ref().unwrap().state, "1");
    let icons = rows
        .iter()
        .find(|r| r.name == "api.iconify.design")
        .unwrap();
    assert_eq!(icons.online.as_ref().unwrap().state, "Offline");
    assert!(rows.iter().any(|r| r.name == "Offline"));
    // A later success clears it.
    let later = Fake::default().with("https://api.iconify.design/collections", Ok(b"{}".to_vec()));
    state.fetch_list(&later, &iconify());
    let rows = state.rows(&sources(&[git(), iconify()]), &HashSet::new(), "");
    assert_eq!(
        rows.iter()
            .find(|r| r.name == "api.iconify.design")
            .unwrap()
            .online
            .as_ref()
            .unwrap()
            .state,
        "0"
    );
}

#[test]
fn enter_sends_the_words_only_to_the_added_iconify_sources() {
    let answer = br#"{"icons":["mdi:home"],"collections":{}}"#.to_vec();
    let fake = Fake::default().with(
        "https://api.iconify.design/search?query=home%20icon&limit=64",
        Ok(answer),
    );
    let state = OnlineState::default();
    let ready = SourceEntry {
        kind: SourceKind::Git,
        address: READY_ADDRESS.into(),
        name: READY_NAME.into(),
        added: Millis(3),
        token: false,
    };
    let all = sources(&[git(), ready, iconify()]);
    let asked = search_all(&state, &fake, &all, "  home   icon ");
    assert_eq!(asked, vec!["online:https://api.iconify.design".to_owned()]);
    assert_eq!(
        fake.asked(),
        vec!["https://api.iconify.design/search?query=home%20icon&limit=64".to_owned()]
    );
    // An empty text, or no Iconify source, sends nothing.
    let none = Fake::default();
    assert_eq!(search_all(&state, &none, &all, "   "), Vec::<String>::new());
    assert_eq!(
        search_all(&state, &none, &sources(&[git()]), "home"),
        Vec::<String>::new()
    );
    assert_eq!(none.asked(), Vec::<String>::new());
}

#[test]
fn a_search_failure_shows_on_its_own_source_only() {
    let state = OnlineState::default();
    let fake = Fake::default();
    search_all(&state, &fake, &sources(&[iconify()]), "home");
    let mut open = HashSet::new();
    open.insert("s:online:https://api.iconify.design\u{1f}".to_owned());
    let rows = state.rows(&sources(&[iconify()]), &open, "");
    assert!(
        rows.iter().any(
            |r| r.name == "Offline" && r.online.as_ref().is_some_and(|o| o.status == "offline")
        )
    );
}

#[test]
fn a_template_is_kept_only_when_it_matches_its_entry() {
    let genuine = container();
    let fake = Fake::default()
        .with(
            "https://example.org/cat/curvyo-catalogue.json",
            Ok(index(&genuine)),
        )
        .with(
            "https://example.org/cat/a/box/template.curvyo",
            Ok(genuine.clone()),
        );
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    let bytes = state.template(&fake, &git(), "a/box").unwrap();
    assert_eq!(*bytes, genuine);
    assert!(
        state
            .held_file("online:https://example.org/cat/", "a/box")
            .is_some()
    );
    let origin = state.template_origin(&git(), "a/box").unwrap();
    assert_eq!(
        origin.origin.credit.as_deref(),
        Some("Box by Jane, CC BY 4.0")
    );
    assert_eq!(
        origin.origin.address,
        "https://example.org/cat/a/box/template.curvyo"
    );
    // The page rows of the downloaded template are listed.
    let mut open = HashSet::new();
    open.insert("s:online:https://example.org/cat/\u{1f}".to_owned());
    open.insert("f:online:https://example.org/cat/\u{1f}a".to_owned());
    open.insert("t:online:https://example.org/cat/\u{1f}a\u{1f}box".to_owned());
    let rows = state.rows(&sources(&[git()]), &open, "");
    assert!(
        rows.iter()
            .any(|r| r.kind == curvyo_ui_core::library_tree::RowKind::Page),
        "{rows:?}"
    );
}

#[test]
fn a_changed_template_is_refused_and_not_kept() {
    let genuine = container();
    let mut changed = genuine.clone();
    changed[10] ^= 0xFF;
    let fake = Fake::default()
        .with(
            "https://example.org/cat/curvyo-catalogue.json",
            Ok(index(&genuine)),
        )
        .with("https://example.org/cat/a/box/template.curvyo", Ok(changed));
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    assert_eq!(
        state.template(&fake, &git(), "a/box").unwrap_err(),
        Failure::Mismatch
    );
    assert!(
        state
            .held_file("online:https://example.org/cat/", "a/box")
            .is_none()
    );
    let rows = state.rows(
        &sources(&[git()]),
        &{
            let mut open = HashSet::new();
            open.insert("s:online:https://example.org/cat/\u{1f}".to_owned());
            open.insert("f:online:https://example.org/cat/\u{1f}a".to_owned());
            open
        },
        "",
    );
    let row = rows.iter().find(|r| r.name == "Box").unwrap();
    assert_eq!(row.line, "Cannot be loaded: does not match the catalogue.");
}

#[test]
fn a_file_that_is_no_document_is_refused_even_with_a_matching_hash() {
    let junk = b"not a curvyo file".to_vec();
    let fake = Fake::default()
        .with(
            "https://example.org/cat/curvyo-catalogue.json",
            Ok(index(&junk)),
        )
        .with("https://example.org/cat/a/box/template.curvyo", Ok(junk));
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    assert_eq!(
        state.template(&fake, &git(), "a/box").unwrap_err(),
        Failure::Invalid
    );
}

#[test]
fn refresh_forgets_what_was_downloaded() {
    let genuine = container();
    let fake = Fake::default()
        .with(
            "https://example.org/cat/curvyo-catalogue.json",
            Ok(index(&genuine)),
        )
        .with("https://example.org/cat/a/box/template.curvyo", Ok(genuine));
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    state.template(&fake, &git(), "a/box").unwrap();
    state.fetch_list(&fake, &git());
    assert!(
        state
            .held_file("online:https://example.org/cat/", "a/box")
            .is_none()
    );
}

#[test]
fn an_icon_import_carries_the_sets_licence_and_a_credit_when_it_needs_one() {
    let collections = br#"{"mdi":{"name":"Material","total":2,"author":{"name":"Pictogrammers","url":""},"license":{"title":"CC BY 4.0","spdx":"CC-BY-4.0","url":"https://l.example/by"}}}"#.to_vec();
    let icons = br#"{"prefix":"mdi","icons":{"home":{"body":"<path d=\"M0 0\"/>"}},"width":24,"height":24}"#.to_vec();
    let fake = Fake::default()
        .with("https://api.iconify.design/collections", Ok(collections))
        .with("https://api.iconify.design/mdi.json?icons=home", Ok(icons));
    let state = OnlineState::default();
    state.fetch_list(&fake, &iconify());
    let found = state.icons(&fake, &iconify(), "mdi", &["home".to_owned()]);
    assert_eq!(found.len(), 1);
    // A second ask for the same icon sends nothing.
    state.icons(&fake, &iconify(), "mdi", &["home".to_owned()]);
    assert_eq!(
        fake.asked()
            .iter()
            .filter(|u| u.contains("mdi.json"))
            .count(),
        1
    );
    let import = state.icon_import(&iconify(), "mdi", "home").unwrap();
    assert!(import.svg.contains("width=\"24mm\""), "{}", import.svg);
    assert_eq!(import.origin.address, "https://api.iconify.design#mdi:home");
    assert_eq!(
        import.origin.credit.as_deref(),
        Some("home from Material by Pictogrammers, CC BY 4.0")
    );
    assert!(state.knows_licence_url("https://l.example/by"));
    assert!(!state.knows_licence_url("https://elsewhere.example/"));
}

#[test]
fn a_thumbnail_without_the_png_magic_or_hash_is_dropped() {
    let genuine = container();
    let png = b"\x89PNG\r\n\x1a\nxx".to_vec();
    let json = format!(
        r#"{{"catalogue_format":1,"name":"Cat","templates":[{{"path":"a/box","name":"Box","size":{},"sha256":"{}","thumbnail_sha256":"{}"}}]}}"#,
        genuine.len(),
        sha256_hex(&genuine),
        sha256_hex(&png)
    );
    let fake = Fake::default()
        .with(
            "https://example.org/cat/curvyo-catalogue.json",
            Ok(json.into_bytes()),
        )
        .with(
            "https://example.org/cat/a/box/thumbnail.png",
            Ok(png.clone()),
        );
    let state = OnlineState::default();
    state.fetch_list(&fake, &git());
    assert_eq!(*state.thumbnail(&fake, &git(), "a/box").unwrap(), png);
    state.thumbnail(&fake, &git(), "a/box");
    assert_eq!(
        fake.asked()
            .iter()
            .filter(|u| u.contains("thumbnail"))
            .count(),
        1
    );
}

#[test]
fn add_sends_exactly_one_request_and_the_ready_entry_none() {
    let json = br#"{"catalogue_format":1,"name":"Their catalogue","templates":[]}"#.to_vec();
    let fake = Fake::default().with("https://example.org/cat/curvyo-catalogue.json", Ok(json));
    let state = OnlineState::default();
    let mut list = Sources::default();
    // A Git address without the closing slash gets it; one GET, of the index.
    let key = state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            "https://example.org/cat",
            None,
            Millis(5),
            &|| true,
        )
        .unwrap();
    assert_eq!(key, "online:https://example.org/cat/");
    assert_eq!(
        fake.asked(),
        vec!["https://example.org/cat/curvyo-catalogue.json".to_owned()]
    );
    assert_eq!(list.entries()[0].name, "Their catalogue");
    // The same address again, a plain http address: refused, nothing sent.
    assert!(
        state
            .add(
                &fake,
                &mut list,
                SourceKind::Git,
                "https://example.org/cat/",
                None,
                Millis(6),
                &|| true
            )
            .is_err()
    );
    assert!(
        state
            .add(
                &fake,
                &mut list,
                SourceKind::Git,
                "http://example.org/x/",
                None,
                Millis(6),
                &|| true
            )
            .is_err()
    );
    // The ready Curvyo catalogue joins the list without a request.
    let none = Fake::default();
    state
        .add(
            &none,
            &mut list,
            SourceKind::Git,
            READY_ADDRESS,
            None,
            Millis(7),
            &|| true,
        )
        .unwrap();
    assert_eq!(none.asked(), Vec::<String>::new());
    assert_eq!(list.entries()[1].name, READY_NAME);
    assert_eq!(fake.asked().len(), 1);
}

#[test]
fn a_source_that_does_not_answer_is_not_added_and_leaves_nothing() {
    let state = OnlineState::default();
    let mut list = Sources::default();
    let fake = Fake::default();
    let text = state
        .add(
            &fake,
            &mut list,
            SourceKind::Iconify,
            "https://api.example.org",
            None,
            Millis(1),
            &|| true,
        )
        .unwrap_err();
    assert_eq!(
        text,
        AddStop::Refused(Refusal {
            field: RefusalField::Address,
            text: "Offline. Nothing was sent.".to_owned()
        })
    );
    assert_eq!(list.entries().len(), 0);
    assert_eq!(
        fake.asked(),
        vec!["https://api.example.org/collections".to_owned()]
    );
    assert!(
        state
            .rows(&list, &HashSet::new(), "")
            .iter()
            .all(|r| r.kind != curvyo_ui_core::library_tree::RowKind::Source)
    );
}

#[test]
fn a_search_shows_searching_until_each_source_answers() {
    let state = OnlineState::default();
    let list = sources(&[iconify()]);
    let (words, keys) = state.search_begin(&list, "home").unwrap();
    let mut open = HashSet::new();
    open.insert("s:online:https://api.iconify.design\u{1f}".to_owned());
    let rows = state.rows(&list, &open, "home");
    assert!(rows.iter().any(|r| r.name == "Searching \"home\"..."));
    let answer = br#"{"icons":["mdi:home"],"collections":{}}"#.to_vec();
    let fake = Fake::default().with(
        "https://api.iconify.design/search?query=home&limit=64",
        Ok(answer),
    );
    let said = state.search_one(&fake, list.find(&keys[0]).unwrap(), &words);
    assert_eq!(said, "api.iconify.design: 1 icons for home");
    let rows = state.rows(&list, &open, "home");
    assert!(rows.iter().all(|r| !r.name.starts_with("Searching")));
    // A source that does not answer says Offline, on its row and in its announcement.
    let down = Fake::default();
    let said = state.search_one(&down, list.find(&keys[0]).unwrap(), &words);
    assert_eq!(said, "api.iconify.design: Offline");
    let rows = state.rows(&list, &open, "home");
    assert_eq!(
        rows.iter()
            .find(|r| r.kind == curvyo_ui_core::library_tree::RowKind::Source)
            .unwrap()
            .online
            .as_ref()
            .unwrap()
            .state,
        "Offline"
    );
}

#[test]
fn a_cancelled_add_discards_the_check_and_lists_nothing() {
    let json = br#"{"catalogue_format":1,"name":"C","templates":[]}"#.to_vec();
    let fake = Fake::default().with("https://example.org/cat/curvyo-catalogue.json", Ok(json));
    let state = OnlineState::default();
    let mut list = Sources::default();
    let stop = state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            "https://example.org/cat/",
            None,
            Millis(1),
            &|| false,
        )
        .unwrap_err();
    assert_eq!(stop, AddStop::Cancelled);
    assert_eq!(list.entries().len(), 0);
    assert!(
        state.held.lock().unwrap().is_empty(),
        "what the check fetched is forgotten"
    );
    let before = state.add_generation();
    state.cancel_add();
    assert_eq!(state.add_generation(), before + 1);
}

// --- `specs/0157` library Git token -----------------------------------------------------------

use curvyo_storage_io::{AccessToken, CredentialStore, load_token};

/// The sentinel every file and line of a run is searched for (criterion 9).
const SENTINEL: &str = "SENTINEL-tok_0157-xyz";

/// A state whose key store is the encrypted key file in `dir`: the test double of the system one.
fn keyed(dir: &std::path::Path) -> OnlineState {
    let state = OnlineState::default();
    let dir = dir.to_path_buf();
    state.use_keys(crate::online_token::Keys::new(
        "slot".to_owned(),
        Box::new(move || CredentialStore::file(&dir, "pw")),
    ));
    state
}

fn tok(text: &str) -> AccessToken {
    AccessToken::parse(text).unwrap()
}

fn index_json() -> Vec<u8> {
    br#"{"catalogue_format":1,"name":"Private","templates":[]}"#.to_vec()
}

const INDEX_URL: &str = "https://example.org/cat/curvyo-catalogue.json";

fn stored(dir: &std::path::Path) -> bool {
    let store = CredentialStore::file(dir, "pw").unwrap();
    load_token(&store, GIT).unwrap().is_some()
}

#[test]
fn add_with_a_token_sends_it_on_the_one_request_and_keeps_it_only_on_success() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let mut list = Sources::default();
    let fake = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let token = tok(SENTINEL);
    let key = state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            GIT,
            Some(&token),
            Millis(1),
            &|| true,
        )
        .unwrap();
    assert_eq!(fake.0.asked(), vec![INDEX_URL.to_owned()]);
    assert_eq!(
        fake.0.bearer_asked(),
        vec![INDEX_URL.to_owned()],
        "with the token"
    );
    assert!(list.find(&key).unwrap().token, "the flag, not the secret");
    assert!(stored(dir.path()));
    assert!(!list.to_text().contains(SENTINEL));
}

#[test]
fn a_refused_token_is_not_kept_and_the_refusal_is_under_the_token_field() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let mut list = Sources::default();
    let token = tok(SENTINEL);
    for (answer, text) in [
        (HttpError::Status(401), "The server refused the token."),
        (
            HttpError::Status(403),
            "This token has no read access to the repository.",
        ),
        (
            HttpError::Status(404),
            "This token has no read access to the repository.",
        ),
        (
            HttpError::Redirect,
            "The server sent the request elsewhere. The token was not sent there.",
        ),
    ] {
        let fake = Bearing(Fake::default().with(INDEX_URL, Err(answer)));
        let stop = state
            .add(
                &fake,
                &mut list,
                SourceKind::Git,
                GIT,
                Some(&token),
                Millis(1),
                &|| true,
            )
            .unwrap_err();
        assert_eq!(
            stop,
            AddStop::Refused(Refusal {
                field: RefusalField::Token,
                text: text.to_owned()
            })
        );
    }
    assert_eq!(list.entries().len(), 0);
    assert!(!stored(dir.path()), "nothing saved on a failure");
    assert!(state.held.lock().unwrap().is_empty());
}

#[test]
fn without_a_key_store_add_with_a_token_is_refused_before_anything_is_sent() {
    let state = OnlineState::default();
    let mut list = Sources::default();
    let fake = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let stop = state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            GIT,
            Some(&tok("t")),
            Millis(1),
            &|| true,
        )
        .unwrap_err();
    assert_eq!(
        stop,
        AddStop::Refused(Refusal {
            field: RefusalField::Token,
            text: "Your system's key store is not available, so the token cannot be kept. The source was not added."
                .to_owned()
        })
    );
    assert_eq!(fake.0.asked(), Vec::<String>::new());
    assert_eq!(list.entries().len(), 0);
    // A source without a token is added as before.
    state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            GIT,
            None,
            Millis(1),
            &|| true,
        )
        .unwrap();
    assert!(!list.entries()[0].token);
}

#[test]
fn a_token_for_iconify_or_the_ready_entry_is_never_kept_or_sent() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let mut list = Sources::default();
    let fake = Bearing(Fake::default());
    state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            READY_ADDRESS,
            Some(&tok(SENTINEL)),
            Millis(1),
            &|| true,
        )
        .unwrap();
    assert!(!list.entries()[0].token);
    let icons =
        Bearing(Fake::default().with("https://api.iconify.design/collections", Ok(b"{}".to_vec())));
    state
        .add(
            &icons,
            &mut list,
            SourceKind::Iconify,
            ICONS,
            Some(&tok(SENTINEL)),
            Millis(2),
            &|| true,
        )
        .unwrap();
    assert!(!list.entries()[1].token);
    assert_eq!(icons.0.bearer_asked(), Vec::<String>::new());
    assert_eq!(fake.0.bearer_asked(), Vec::<String>::new());
}

#[test]
fn a_source_with_a_token_sends_it_to_its_own_requests_and_no_other_source_does() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let fake = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let mut list = Sources::default();
    state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            GIT,
            Some(&tok(SENTINEL)),
            Millis(1),
            &|| true,
        )
        .unwrap();
    list.add(
        SourceKind::Git,
        "https://other.example/c/",
        "Other",
        Millis(2),
    )
    .unwrap();
    let before = fake.0.bearer_asked().len();
    let private = list.entries()[0].clone();
    state.fetch_list(&fake, &private);
    assert_eq!(fake.0.bearer_asked().len(), before + 1, "Refresh sends it");
    let other = list.entries()[1].clone();
    state.fetch_list(&fake, &other);
    assert_eq!(
        fake.0.bearer_asked().len(),
        before + 1,
        "another catalogue gets none"
    );
}

#[test]
fn a_flagged_source_whose_token_cannot_be_read_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let fake = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let mut entry = git();
    entry.token = true;
    state.fetch_list(&fake, &entry);
    assert_eq!(
        fake.0.asked(),
        Vec::<String>::new(),
        "no silent anonymous request"
    );
    assert_eq!(
        state.with(&entry.key(), |h| h.failure),
        Some(Failure::TokenUnreadable)
    );
    // No key store at all reads the same.
    let none = OnlineState::default();
    none.fetch_list(&fake, &entry);
    assert_eq!(fake.0.asked(), Vec::<String>::new());
    let mut flagged = sources(&[git()]);
    flagged.set_token(&entry.key(), true).unwrap();
    let probe = none.rows(&flagged, &HashSet::new(), "");
    let opened: HashSet<String> = std::iter::once(
        probe
            .iter()
            .find(|r| r.kind == curvyo_ui_core::library_tree::RowKind::Source)
            .unwrap()
            .key
            .clone(),
    )
    .collect();
    let rows = none.rows(&flagged, &opened, "");
    assert!(
        rows.iter()
            .any(|r| r.name == "The saved token could not be read from the key store.")
    );
}

fn open_all() -> HashSet<String> {
    std::iter::once("s:online:https://example.org/cat/".to_owned()).collect()
}

#[test]
fn change_token_replaces_only_on_success_and_a_failure_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let ok = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let mut list = Sources::default();
    state
        .add(
            &ok,
            &mut list,
            SourceKind::Git,
            GIT,
            Some(&tok("old-token")),
            Millis(1),
            &|| true,
        )
        .unwrap();
    let key = list.entries()[0].key();
    let refused = Bearing(Fake::default().with(INDEX_URL, Err(HttpError::Status(401))));
    let held_before = state.with(&key, |h| h.catalogue.is_some());
    let err = state
        .set_token(&refused, &mut list, &key, "new-token")
        .unwrap_err();
    assert_eq!(err.field, RefusalField::Token);
    assert_eq!(err.text, "The server refused the token.");
    assert!(list.entries()[0].token, "old state unchanged");
    assert!(stored(dir.path()), "the old token stays");
    assert_eq!(state.with(&key, |h| h.catalogue.is_some()), held_before);
    assert_eq!(state.with(&key, |h| h.failure), None);
    // A bad text is refused before anything is sent.
    let before = ok.0.asked().len();
    let err = state
        .set_token(&ok, &mut list, &key, "has space")
        .unwrap_err();
    assert_eq!(
        err.text,
        "Enter a token of at most 1024 characters, without spaces."
    );
    assert_eq!(ok.0.asked().len(), before);
    // Success: one GET with the new token.
    state
        .set_token(&ok, &mut list, &key, "  new-token \n")
        .unwrap();
    assert_eq!(ok.0.asked().len(), before + 1);
    assert_eq!(
        ok.0.bearer_asked().last().map(String::as_str),
        Some(INDEX_URL)
    );
}

#[test]
fn add_token_to_a_public_source_marks_it_private_and_remove_token_clears_both() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let ok = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let mut list = sources(&[git()]);
    let key = list.entries()[0].key();
    state
        .set_token(&ok, &mut list, &key, "added-later")
        .unwrap();
    assert!(list.entries()[0].token && stored(dir.path()));
    assert_eq!(state.delete_token(GIT), None);
    assert!(!stored(dir.path()), "the key store entry is gone");
}

#[test]
fn a_key_store_that_will_not_delete_names_the_entry_by_hand() {
    let state = OnlineState::default();
    state.use_keys(crate::online_token::Keys::new(
        "abc".to_owned(),
        Box::new(|| Err(curvyo_storage_io::CredentialError::NoKeychain)),
    ));
    assert_eq!(
        state.delete_token(GIT).as_deref(),
        Some(
            "The saved token could not be deleted from the key store; remove it there by hand: git-token:https://example.org/cat/ in curvyo/abc."
        )
    );
}

#[test]
fn no_file_the_app_writes_and_no_line_it_shows_holds_the_token() {
    let dir = tempfile::tempdir().unwrap();
    let state = keyed(dir.path());
    let fake = Bearing(Fake::default().with(INDEX_URL, Ok(index_json())));
    let mut list = Sources::default();
    let token = tok(SENTINEL);
    let key = state
        .add(
            &fake,
            &mut list,
            SourceKind::Git,
            GIT,
            Some(&token),
            Millis(1),
            &|| true,
        )
        .unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    std::fs::write(app_dir.path().join("online-sources.toml"), list.to_text()).unwrap();
    let rows = state.rows(&list, &open_all(), "");
    let shown = format!("{rows:?}{}", serde_json::to_string(&rows).unwrap());
    let browse = serde_json::to_string(&state.browse_list(&list, None)).unwrap();
    let debug = format!("{token:?}{list:?}{:?}", state.with(&key, |h| h.failure));
    for text in [shown, browse, debug] {
        assert!(!text.contains(SENTINEL));
    }
    for entry in std::fs::read_dir(app_dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(SENTINEL));
    }
    // The key file is the key store itself and holds it only encrypted.
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(
            !String::from_utf8_lossy(&bytes).contains(SENTINEL),
            "sealed"
        );
    }
}
