//! Acceptance tests of `specs/0138-template-library` for the host fetch (`curvyo-storage-io`),
//! through a recording fake `Transport`: no network is used. Written before the implementation
//! was read (black box, public API).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::cell::RefCell;

use curvyo_library_core::{Catalogue, Failure, Request, sha256_hex};
use curvyo_storage_io::{HttpError, Transport, fetch_catalogue, fetch_template, fetch_thumbnail};

const BASE: &str = "https://raw.githubusercontent.com/curvyo/templates/main/";
const BODY: &[u8] = b"abc";

/// Answers by URL suffix and records every request it was asked for.
struct Fake {
    answers: Vec<(String, Result<Vec<u8>, HttpError>)>,
    seen: RefCell<Vec<String>>,
}

impl Fake {
    fn new(answers: Vec<(&str, Result<Vec<u8>, HttpError>)>) -> Self {
        Self {
            answers: answers
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl Transport for Fake {
    fn get(&self, request: &Request, _max_bytes: usize) -> Result<Vec<u8>, HttpError> {
        self.seen.borrow_mut().push(request.url().to_owned());
        for (suffix, answer) in &self.answers {
            if request.url().ends_with(suffix.as_str()) {
                return answer.clone();
            }
        }
        Err(HttpError::Status(404))
    }
}

fn index(hash: &str) -> Vec<u8> {
    format!(
        r#"{{"catalogue_format": 1, "name": "Test", "templates": [{{
            "path": "boxes/finger-box", "name": "Finger box", "author": "Jane Doe",
            "tags": [], "licence": {{"spdx": "CC0-1.0", "name": "CC0 1.0",
            "url": "https://creativecommons.org/publicdomain/zero/1.0/"}}, "fonts": [],
            "size": 3, "sha256": "{hash}", "format_version": 58}}]}}"#
    )
    .into_bytes()
}

// Criterion 2: the catalogue is one GET of the index file under the ready address, nothing else.
#[test]
fn ac2_catalogue_fetch_is_one_get_of_the_index() {
    let fake = Fake::new(vec![(
        "curvyo-catalogue.json",
        Ok(index(&sha256_hex(BODY))),
    )]);
    let catalogue = fetch_catalogue(&fake, BASE).expect("the index is read");
    assert_eq!(catalogue.templates.len(), 1);
    let seen = fake.seen.borrow();
    assert_eq!(seen.len(), 1, "one request for the list: {seen:?}");
    assert!(seen[0].starts_with(BASE), "{seen:?}");
}

// Criterion 8: a template whose bytes match its entry is returned.
#[test]
fn ac8_template_with_matching_hash_is_returned() {
    let hash = sha256_hex(BODY);
    let catalogue = Catalogue::parse(&index(&hash)).unwrap();
    let entry = catalogue.entry("boxes/finger-box").unwrap().clone();
    let template_url = entry.template_request(BASE).url().to_owned();
    let fake = Fake::new(vec![(template_url.as_str(), Ok(BODY.to_vec()))]);
    let bytes = fetch_template(&fake, BASE, &entry).expect("matching bytes are returned");
    assert_eq!(bytes, BODY);
}

// Criterion 8: a changed template on the moving branch fails with the "does not match its catalogue
// entry" kind (Mismatch) and no bytes are returned.
#[test]
fn ac8_changed_template_fails_with_mismatch_and_returns_nothing() {
    let hash = sha256_hex(BODY);
    let catalogue = Catalogue::parse(&index(&hash)).unwrap();
    let entry = catalogue.entry("boxes/finger-box").unwrap().clone();
    let template_url = entry.template_request(BASE).url().to_owned();
    let fake = Fake::new(vec![(template_url.as_str(), Ok(b"abX".to_vec()))]);
    let result = fetch_template(&fake, BASE, &entry);
    assert_eq!(result, Err(Failure::Mismatch));
}

// Criterion 5: a connection failure of this source's request is the Offline kind; the fetch of
// another source is not affected because the state is per request.
#[test]
fn ac5_connection_failure_is_offline_for_that_request_only() {
    let offline = Fake::new(vec![("curvyo-catalogue.json", Err(HttpError::Connection))]);
    assert_eq!(
        fetch_catalogue(&offline, BASE).unwrap_err(),
        Failure::Offline
    );
    let fine = Fake::new(vec![(
        "curvyo-catalogue.json",
        Ok(index(&sha256_hex(BODY))),
    )]);
    assert!(fetch_catalogue(&fine, BASE).is_ok());
}

// Criterion 2 (decision 6): a 404 on the index is NotFound, a 500 another status, never a crash.
#[test]
fn ac2_http_status_maps_to_failure_kinds() {
    let missing = Fake::new(vec![("curvyo-catalogue.json", Err(HttpError::Status(404)))]);
    assert_eq!(
        fetch_catalogue(&missing, BASE).unwrap_err(),
        Failure::NotFound
    );
    let broken = Fake::new(vec![("curvyo-catalogue.json", Err(HttpError::Status(500)))]);
    assert_eq!(
        fetch_catalogue(&broken, BASE).unwrap_err(),
        Failure::Status(500)
    );
}

// Criterion 2: an answer over the size limit is refused as TooLarge.
#[test]
fn ac2_oversized_answer_is_refused() {
    let big = Fake::new(vec![("curvyo-catalogue.json", Err(HttpError::TooLarge))]);
    assert_eq!(fetch_catalogue(&big, BASE).unwrap_err(), Failure::TooLarge);
}

// Criterion 2 and 8: a redirect the guard refuses is Redirect; nothing is followed to another host.
#[test]
fn ac2_refused_redirect_is_reported() {
    let redirect = Fake::new(vec![("curvyo-catalogue.json", Err(HttpError::Redirect))]);
    assert_eq!(
        fetch_catalogue(&redirect, BASE).unwrap_err(),
        Failure::Redirect
    );
}

// Criterion 3 and 8: the thumbnail is kept only when its hash matches the index; with no hash the
// picture is never read.
#[test]
fn ac3_thumbnail_without_index_hash_is_not_read() {
    let hash = sha256_hex(BODY);
    let catalogue = Catalogue::parse(&index(&hash)).unwrap();
    let entry = catalogue.entry("boxes/finger-box").unwrap().clone();
    assert!(entry.thumbnail_sha256.is_none());
    let fake = Fake::new(vec![(
        "thumbnail.png",
        Ok(b"\x89PNG\r\n\x1a\nxxx".to_vec()),
    )]);
    let picture = fetch_thumbnail(&fake, BASE, &entry).expect("no failure");
    assert!(picture.is_none(), "an unchecked picture is never read");
}

// Criterion 2 (no identifier and nothing of the project in the request): every request the host
// makes for a list or a template is under the given address and carries no query or user data.
#[test]
fn ac2_requests_carry_no_project_or_user_data() {
    let hash = sha256_hex(BODY);
    let catalogue_fake = Fake::new(vec![("curvyo-catalogue.json", Ok(index(&hash)))]);
    let catalogue = fetch_catalogue(&catalogue_fake, BASE).unwrap();
    let entry = catalogue.entry("boxes/finger-box").unwrap().clone();
    let template_url = entry.template_request(BASE).url().to_owned();
    let fake = Fake::new(vec![
        ("curvyo-catalogue.json", Ok(index(&hash))),
        (template_url.as_str(), Ok(BODY.to_vec())),
    ]);
    let _ = fetch_catalogue(&fake, BASE).unwrap();
    let _ = fetch_template(&fake, BASE, &entry).unwrap();
    for url in fake.seen.borrow().iter() {
        assert!(url.starts_with(BASE), "{url}");
        assert!(!url.contains('?'), "no query carries data: {url}");
        assert!(!url.contains('@'), "no user identifier: {url}");
    }
}
