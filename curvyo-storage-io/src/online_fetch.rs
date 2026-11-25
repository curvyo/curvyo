//! The answers of an online source (ADR 0085 decisions 3 to 6): each function sends the request
//! `curvyo-library-core` built through a [`Transport`], checks and parses the answer, and names a
//! failure by the table of decision 6. No state is kept; the caller holds the session's lists.

use curvyo_library_core::{
    Catalogue, Failure, IconBody, IconSet, MAX_ICONIFY_BYTES, MAX_INDEX_BYTES, MAX_PICTURE_BYTES,
    MAX_TEMPLATE_BYTES, SearchAnswer, TemplateEntry, collection_request, collections_request,
    icons_request, parse_collection, parse_collections, parse_icons, parse_search, search_request,
    verify,
};

use crate::http::Transport;

const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The checked index of the Git catalogue at `base`.
///
/// # Errors
/// [`Failure`] by decision 6.
pub fn fetch_catalogue(transport: &dyn Transport, base: &str) -> Result<Catalogue, Failure> {
    let bytes = transport.get(&Catalogue::request(base), MAX_INDEX_BYTES)?;
    Ok(Catalogue::parse(&bytes)?)
}

/// The icon sets of the Iconify API at `base`.
///
/// # Errors
/// [`Failure`] by decision 6.
pub fn fetch_collections(transport: &dyn Transport, base: &str) -> Result<Vec<IconSet>, Failure> {
    let bytes = transport.get(&collections_request(base), MAX_ICONIFY_BYTES)?;
    Ok(parse_collections(&bytes)?)
}

/// The icon names of set `prefix`.
///
/// # Errors
/// [`Failure`] by decision 6; a prefix that is not a plain name is `Invalid`.
pub fn fetch_collection(
    transport: &dyn Transport,
    base: &str,
    prefix: &str,
) -> Result<Vec<String>, Failure> {
    let request = collection_request(base, prefix).ok_or(Failure::Invalid)?;
    Ok(parse_collection(
        &transport.get(&request, MAX_ICONIFY_BYTES)?,
    )?)
}

/// The bodies of `names` of set `prefix`, one request for at most 100 names.
///
/// # Errors
/// [`Failure`] by decision 6; no usable name is `Invalid`.
pub fn fetch_icons(
    transport: &dyn Transport,
    base: &str,
    prefix: &str,
    names: &[String],
) -> Result<Vec<IconBody>, Failure> {
    let request = icons_request(base, prefix, names).ok_or(Failure::Invalid)?;
    Ok(parse_icons(&transport.get(&request, MAX_ICONIFY_BYTES)?)?)
}

/// The search of the Iconify API at `base` for `words` (already [`curvyo_library_core::search_words`]).
///
/// # Errors
/// [`Failure`] by decision 6.
pub fn fetch_search(
    transport: &dyn Transport,
    base: &str,
    words: &str,
) -> Result<SearchAnswer, Failure> {
    let bytes = transport.get(&search_request(base, words), MAX_ICONIFY_BYTES)?;
    Ok(parse_search(&bytes)?)
}

/// The template file of `entry`, checked against its size and SHA-256 before it is returned
/// (criterion 8); a file that is longer or different than the entry is a `Mismatch`.
///
/// # Errors
/// [`Failure`] by decision 6; an entry over the size limit is `TooLarge`.
pub fn fetch_template(
    transport: &dyn Transport,
    base: &str,
    entry: &TemplateEntry,
) -> Result<Vec<u8>, Failure> {
    if entry.size > MAX_TEMPLATE_BYTES {
        return Err(Failure::TooLarge);
    }
    let max = usize::try_from(entry.size).map_err(|_| Failure::TooLarge)?;
    let bytes = transport
        .get(&entry.template_request(base), max)
        .map_err(|error| match Failure::from(error) {
            Failure::TooLarge => Failure::Mismatch,
            other => other,
        })?;
    verify(&bytes, Some(entry.size), &entry.sha256)?;
    Ok(bytes)
}

/// The thumbnail of `entry` as PNG bytes, or `None` when the index states no hash for it (an
/// unchecked picture is never read) or the bytes are not the PNG the index names.
///
/// # Errors
/// [`Failure`] by decision 6 when the request fails.
pub fn fetch_thumbnail(
    transport: &dyn Transport,
    base: &str,
    entry: &TemplateEntry,
) -> Result<Option<Vec<u8>>, Failure> {
    let Some(hash) = &entry.thumbnail_sha256 else {
        return Ok(None);
    };
    let max = usize::try_from(MAX_PICTURE_BYTES).unwrap_or(usize::MAX);
    let bytes = transport.get(&entry.thumbnail_request(base), max)?;
    let fine = bytes.starts_with(PNG_MAGIC) && verify(&bytes, None, hash).is_ok();
    Ok(fine.then_some(bytes))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use curvyo_library_core::{Request, sha256_hex};

    use super::*;
    use crate::http::HttpError;

    /// The fake `Transport` (a second implementation next to `UreqTransport`): canned answers by
    /// URL, and the list of the URLs asked.
    #[derive(Default)]
    pub struct Scripted {
        answers: HashMap<String, Result<Vec<u8>, HttpError>>,
        pub asked: RefCell<Vec<String>>,
    }

    impl Scripted {
        fn with(mut self, url: &str, answer: Result<Vec<u8>, HttpError>) -> Self {
            self.answers.insert(url.to_owned(), answer);
            self
        }
    }

    impl Transport for Scripted {
        fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError> {
            self.asked.borrow_mut().push(request.url().to_owned());
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

    const BASE: &str = "https://example.org/cat/";

    fn entry(bytes: &[u8]) -> TemplateEntry {
        let json = format!(
            r#"{{"catalogue_format":1,"name":"C","templates":[{{"path":"a/b","name":"B","size":{},"sha256":"{}","thumbnail_sha256":"{}"}}]}}"#,
            bytes.len(),
            sha256_hex(bytes),
            sha256_hex(b"\x89PNG\r\n\x1a\nxx"),
        );
        Catalogue::parse(json.as_bytes())
            .unwrap()
            .templates
            .remove(0)
    }

    #[test]
    fn a_catalogue_is_one_get_of_its_index() {
        let json = br#"{"catalogue_format":1,"name":"C","templates":[]}"#.to_vec();
        let transport =
            Scripted::default().with("https://example.org/cat/curvyo-catalogue.json", Ok(json));
        assert_eq!(fetch_catalogue(&transport, BASE).unwrap().name, "C");
        assert_eq!(transport.asked.borrow().len(), 1);
    }

    #[test]
    fn failures_are_named_by_the_table() {
        let url = "https://example.org/cat/curvyo-catalogue.json";
        let missing = Scripted::default().with(url, Err(HttpError::Status(404)));
        assert_eq!(
            fetch_catalogue(&missing, BASE).unwrap_err(),
            Failure::NotFound
        );
        let down = Scripted::default();
        assert_eq!(fetch_catalogue(&down, BASE).unwrap_err(), Failure::Offline);
        let html = Scripted::default().with(url, Ok(b"<html>".to_vec()));
        assert_eq!(fetch_catalogue(&html, BASE).unwrap_err(), Failure::Invalid);
        let busy = Scripted::default().with(url, Err(HttpError::Status(503)));
        assert_eq!(
            fetch_catalogue(&busy, BASE).unwrap_err(),
            Failure::Status(503)
        );
    }

    #[test]
    fn a_template_is_returned_only_when_it_matches_its_entry() {
        let url = "https://example.org/cat/a/b/template.curvyo";
        let entry = entry(b"genuine");
        let good = Scripted::default().with(url, Ok(b"genuine".to_vec()));
        assert_eq!(fetch_template(&good, BASE, &entry).unwrap(), b"genuine");
        let changed = Scripted::default().with(url, Ok(b"changed".to_vec()));
        assert_eq!(
            fetch_template(&changed, BASE, &entry).unwrap_err(),
            Failure::Mismatch
        );
        let longer = Scripted::default().with(url, Ok(b"genuine and more".to_vec()));
        assert_eq!(
            fetch_template(&longer, BASE, &entry).unwrap_err(),
            Failure::Mismatch
        );
    }

    #[test]
    fn a_thumbnail_needs_its_hash_and_the_png_magic() {
        let url = "https://example.org/cat/a/b/thumbnail.png";
        let entry = entry(b"genuine");
        let good = Scripted::default().with(url, Ok(b"\x89PNG\r\n\x1a\nxx".to_vec()));
        assert!(fetch_thumbnail(&good, BASE, &entry).unwrap().is_some());
        let other = Scripted::default().with(url, Ok(b"\x89PNG\r\n\x1a\nyy".to_vec()));
        assert!(fetch_thumbnail(&other, BASE, &entry).unwrap().is_none());
        let mut bare = entry.clone();
        bare.thumbnail_sha256 = None;
        let none = Scripted::default();
        assert!(fetch_thumbnail(&none, BASE, &bare).unwrap().is_none());
        assert_eq!(
            *none.asked.borrow(),
            Vec::<String>::new(),
            "no request without a hash"
        );
    }

    #[test]
    fn iconify_requests_carry_the_protocols_path_only() {
        let base = "https://api.iconify.design";
        let transport = Scripted::default()
            .with(
                "https://api.iconify.design/search?query=home%20icon&limit=64",
                Ok(br#"{"icons":["mdi:home"],"collections":{}}"#.to_vec()),
            )
            .with("https://api.iconify.design/collections", Ok(b"{}".to_vec()));
        assert_eq!(
            fetch_search(&transport, base, "home icon")
                .unwrap()
                .icons
                .len(),
            1
        );
        assert_eq!(fetch_collections(&transport, base).unwrap(), Vec::new());
        assert_eq!(
            fetch_collection(&transport, base, "../x").unwrap_err(),
            Failure::Invalid
        );
        assert_eq!(
            transport.asked.borrow().len(),
            2,
            "a bad prefix sends nothing"
        );
    }
}
