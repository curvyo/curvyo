//! What this session holds of each online source, in memory only (nothing is written to disk).

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use curvyo_document_core::{OnlineOrigin, UnixMillis};
use curvyo_library_core::{
    Catalogue, Failure, IconBody, IconSet, Licence, Millimetres, Millis, SetListing, SourceEntry,
    SourceKind, Sources, base_address, host_of, icon_svg_mm, is_ready, needs_credit, search_words,
};
use curvyo_storage_io::{
    AccessToken, Authorized, Transport, UreqTransport, fetch_catalogue, fetch_collection,
    fetch_collections, fetch_icons, fetch_search, fetch_template, fetch_thumbnail, save_token,
};
use curvyo_ui_core::library_index::IndexedDocument;
use curvyo_ui_core::library_online::{
    OnlineSourceView, OnlineView, READY_NAME, Refusal, RefusalField, SearchView, add_address_text,
    add_duplicate_text, add_refusal, icon_credit_text, key_store_text, online_rows,
    search_done_text,
};
use curvyo_ui_core::library_tree::LibraryRow;

/// An icon import is this wide (UX notes 6, Proposal).
const ICON_WIDTH: Millimetres = Millimetres(24.0);

/// What this session holds of one source; nothing of it is written to disk (UX F2).
#[derive(Default)]
pub(crate) struct Held {
    pub(crate) failure: Option<Failure>,
    pub(crate) catalogue: Option<Catalogue>,
    pub(crate) sets: Option<Vec<IconSet>>,
    pub(crate) set_icons: HashMap<String, SetListing>,
    pub(crate) set_failures: HashMap<String, Failure>,
    pub(crate) bodies: HashMap<(String, String), IconBody>,
    pub(crate) search: Option<SearchView>,
    pub(crate) files: HashMap<String, Arc<Vec<u8>>>,
    pub(crate) documents: HashMap<String, IndexedDocument>,
    pub(crate) refused: HashMap<String, Failure>,
    pub(crate) thumbnails: HashMap<String, Option<Arc<Vec<u8>>>>,
}

/// The session's memory of the online sources and the one transport.
#[derive(Default)]
pub(crate) struct OnlineState {
    pub(crate) transport: UreqTransport,
    /// The key store of the access tokens (ADR 0090); none until the app installs it.
    pub(crate) keys: std::sync::OnceLock<crate::online_token::Keys>,
    /// Counts the Cancels of the Add source form; a check that started before one is discarded.
    add_cancels: std::sync::atomic::AtomicU64,
    held: Mutex<HashMap<String, Held>>,
}

impl OnlineState {
    /// The number of Cancels so far.
    pub(crate) fn add_generation(&self) -> u64 {
        self.add_cancels.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The Add source form was cancelled: the check that runs is discarded.
    pub(crate) fn cancel_add(&self) {
        self.add_cancels
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub(crate) fn with<R>(&self, key: &str, read: impl FnOnce(&mut Held) -> R) -> R {
        let mut all = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        read(all.entry(key.to_owned()).or_default())
    }

    /// Add source (criterion 1, UX F6): the address is checked with one request, except the ready
    /// Curvyo catalogue, which sends nothing until it is opened. With a `token` the key store is
    /// opened first, the request carries the token, and only after it succeeded the token is kept
    /// (`specs/0156` criteria 2, 3, 11). `Err` is the refusal and the field it is shown under;
    /// `Ok` the key of the source now in `sources`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add(
        &self,
        transport: &dyn Transport,
        sources: &mut Sources,
        kind: SourceKind,
        address: &str,
        token: Option<&AccessToken>,
        now: Millis,
        wanted: &dyn Fn() -> bool,
    ) -> Result<String, AddStop> {
        let under_address = |text: &str| {
            AddStop::Refused(Refusal {
                field: RefusalField::Address,
                text: text.to_owned(),
            })
        };
        let base = base_address(kind, address).map_err(|_| under_address(add_address_text()))?;
        if sources.entries().iter().any(|e| e.address == base) {
            return Err(under_address(add_duplicate_text()));
        }
        // A token is only for a Git catalogue of the maker's own, never the ready entry.
        let token = token.filter(|_| kind == SourceKind::Git && !is_ready(&base));
        let store = token
            .map(|_| {
                self.open_keys().map_err(|_| {
                    AddStop::Refused(crate::online_token::under_token(key_store_text(false)))
                })
            })
            .transpose()?;
        let probe = SourceEntry {
            kind,
            address: base.clone(),
            name: String::new(),
            added: Millis(0),
            token: false,
        };
        let name = if is_ready(&base) {
            READY_NAME.to_owned()
        } else {
            let authorized = token.map(|token| Authorized::new(transport, token));
            let sender: &dyn Transport = authorized
                .as_ref()
                .map_or(transport, |a| a as &dyn Transport);
            self.fetch_list(sender, &probe);
            let key = probe.key();
            // Cancel while the check ran: what it fetched is discarded, nothing is listed.
            if !wanted() {
                self.forget(&key);
                return Err(AddStop::Cancelled);
            }
            if let Some(failure) = self.with(&key, |held| held.failure) {
                self.forget(&key);
                return Err(AddStop::Refused(add_refusal(
                    kind,
                    token.is_some(),
                    failure,
                )));
            }
            let given = self.with(&key, |held| held.catalogue.as_ref().map(|c| c.name.clone()));
            given
                .filter(|name| !name.trim().is_empty())
                .or_else(|| host_of(&base))
                .unwrap_or_else(|| base.clone())
        };
        if let (Some(token), Some(store)) = (token, &store)
            && save_token(store, &base, token).is_err()
        {
            self.forget(&probe.key());
            return Err(AddStop::Refused(crate::online_token::under_token(
                key_store_text(false),
            )));
        }
        let key = sources
            .add(kind, &base, &name, now)
            .map_err(|_| under_address(add_duplicate_text()))?;
        if token.is_some() {
            // invariant: the source was just added and is a Git catalogue of the maker's own.
            let _ = sources.set_token(&key, true);
        }
        Ok(key)
    }

    /// Fetches the list of `entry` (the index or the icon sets) and keeps it or its failure.
    /// Refresh also forgets the downloaded templates and refusals, so a moved branch is read again.
    pub(crate) fn fetch_list(&self, transport: &dyn Transport, entry: &SourceEntry) {
        let key = entry.key();
        let outcome = match entry.kind {
            SourceKind::Git => self
                .sent(transport, entry, |t| fetch_catalogue(t, &entry.address))
                .map(|c| (Some(c), None)),
            SourceKind::Iconify => {
                fetch_collections(transport, &entry.address).map(|s| (None, Some(s)))
            }
        };
        self.with(&key, |held| match outcome {
            Ok((catalogue, sets)) => {
                held.failure = None;
                if catalogue.is_some() {
                    held.catalogue = catalogue;
                    held.files.clear();
                    held.documents.clear();
                    held.refused.clear();
                    held.thumbnails.clear();
                }
                if sets.is_some() {
                    held.sets = sets;
                }
            }
            Err(failure) => held.failure = Some(failure),
        });
    }

    /// The list of one set (names, groups, aliases), fetched once per session.
    pub(crate) fn open_set(&self, transport: &dyn Transport, entry: &SourceEntry, prefix: &str) {
        let key = entry.key();
        if self.with(&key, |held| held.set_icons.contains_key(prefix)) {
            return;
        }
        match fetch_collection(transport, &entry.address, prefix) {
            Ok(listing) => self.with(&key, |held| {
                held.failure = None;
                held.set_failures.remove(prefix);
                held.set_icons.insert(prefix.to_owned(), listing);
            }),
            Err(failure) => self.with(&key, |held| {
                held.failure = Some(failure);
                held.set_failures.insert(prefix.to_owned(), failure);
            }),
        }
    }

    /// The bytes of the template at `path`: downloaded, checked against the index and read once
    /// per session. A refusal is kept and shown on the row.
    pub(crate) fn template(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        path: &str,
    ) -> Result<Arc<Vec<u8>>, Failure> {
        let key = entry.key();
        if let Some(bytes) = self.with(&key, |held| held.files.get(path).cloned()) {
            return Ok(bytes);
        }
        let template = self
            .with(&key, |held| {
                held.catalogue.as_ref().and_then(|c| c.entry(path).cloned())
            })
            .ok_or(Failure::Invalid)?;
        let fetched = self.sent(transport, entry, |t| {
            fetch_template(t, &entry.address, &template)
        });
        let result = fetched.and_then(|bytes| {
            let members = curvyo_document_core::read_library_members(Cursor::new(&bytes));
            let segments: Vec<String> = path.split('/').map(str::to_owned).collect();
            let size = bytes.len() as u64;
            let document = curvyo_ui_core::library_index::record_of_members(
                &key,
                segments,
                (size, UnixMillis::new(0)),
                members,
                || Some(bytes.clone()),
            );
            match document.state {
                curvyo_ui_core::library_index::DocumentState::Readable => Ok((bytes, document)),
                curvyo_ui_core::library_index::DocumentState::Newer => Err(Failure::Newer),
                _ => Err(Failure::Invalid),
            }
        });
        match result {
            Ok((bytes, document)) => {
                let bytes = Arc::new(bytes);
                self.with(&key, |held| {
                    held.failure = None;
                    held.refused.remove(path);
                    held.files.insert(path.to_owned(), bytes.clone());
                    held.documents.insert(path.to_owned(), document);
                });
                Ok(bytes)
            }
            Err(failure) => {
                self.with(&key, |held| {
                    if matches!(failure, Failure::Offline | Failure::TokenUnreadable) {
                        held.failure = Some(failure);
                    } else {
                        held.refused.insert(path.to_owned(), failure);
                    }
                });
                Err(failure)
            }
        }
    }

    /// The thumbnail of the template at `path`, checked by its hash; once per session.
    pub(crate) fn thumbnail(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        path: &str,
    ) -> Option<Arc<Vec<u8>>> {
        let key = entry.key();
        if let Some(done) = self.with(&key, |held| held.thumbnails.get(path).cloned()) {
            return done;
        }
        let template = self.with(&key, |held| {
            held.catalogue.as_ref().and_then(|c| c.entry(path).cloned())
        })?;
        let png = self
            .sent(transport, entry, |t| {
                fetch_thumbnail(t, &entry.address, &template)
            })
            .ok()
            .flatten()
            .map(Arc::new);
        self.with(&key, |held| {
            held.thumbnails.insert(path.to_owned(), png.clone())
        });
        png
    }

    /// The bodies of `names` of `prefix`, the ones not held fetched in one request; the icons held.
    pub(crate) fn icons(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        prefix: &str,
        names: &[String],
    ) -> Vec<IconBody> {
        let key = entry.key();
        let missing: Vec<String> = self.with(&key, |held| {
            names
                .iter()
                .filter(|n| !held.bodies.contains_key(&(prefix.to_owned(), (*n).clone())))
                .cloned()
                .collect()
        });
        if !missing.is_empty()
            && let Ok(found) = fetch_icons(transport, &entry.address, prefix, &missing)
        {
            self.with(&key, |held| {
                for icon in found {
                    held.bodies
                        .insert((prefix.to_owned(), icon.name.clone()), icon);
                }
            });
        }
        self.with(&key, |held| {
            names
                .iter()
                .filter_map(|n| held.bodies.get(&(prefix.to_owned(), n.clone())).cloned())
                .collect()
        })
    }

    /// Enter in the search, first step (criterion 10): marks every Iconify source of `sources` as
    /// searching (its results row says so) and returns the words as they go out and the keys; an
    /// empty text sends nothing. Git catalogues and the ready entry are never in the list.
    pub(crate) fn search_begin(
        &self,
        sources: &Sources,
        text: &str,
    ) -> Option<(String, Vec<String>)> {
        let words = search_words(text)?;
        let keys: Vec<String> = sources.iconify().map(SourceEntry::key).collect();
        for key in &keys {
            self.with(key, |held| {
                held.search = Some(SearchView {
                    words: words.clone(),
                    outcome: Ok(Vec::new()),
                    running: true,
                    sets: Vec::new(),
                });
            });
        }
        Some((words, keys))
    }

    /// Enter in the search, second step: the one request to `entry`; its results row and state are
    /// set when it ends. The announcement of that source (UX notes 7.2).
    pub(crate) fn search_one(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        words: &str,
    ) -> String {
        let outcome = fetch_search(transport, &entry.address, words);
        let counted = outcome.as_ref().map(|a| a.icons.len()).map_err(|f| *f);
        self.with(&entry.key(), |held| {
            held.failure = outcome
                .as_ref()
                .err()
                .copied()
                .filter(|f| *f == Failure::Offline);
            held.search = Some(match outcome {
                Ok(answer) => SearchView {
                    words: words.to_owned(),
                    outcome: Ok(answer.icons),
                    running: false,
                    sets: answer.sets,
                },
                Err(failure) => SearchView {
                    words: words.to_owned(),
                    outcome: Err(failure),
                    running: false,
                    sets: Vec::new(),
                },
            });
        });
        search_done_text(&entry.name, words, counted)
    }

    /// Runs `read` over the views of `sources` with what this session fetched of them.
    pub(crate) fn with_views<R>(
        &self,
        sources: &Sources,
        read: impl FnOnce(Vec<OnlineSourceView<'_>>) -> R,
    ) -> R {
        let all = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let empty = Held::default();
        let views = sources
            .entries()
            .iter()
            .map(|entry| {
                let held = all.get(&entry.key()).unwrap_or(&empty);
                OnlineSourceView {
                    key: entry.key(),
                    name: &entry.name,
                    address: &entry.address,
                    kind: entry.kind,
                    token: entry.token,
                    failure: held.failure,
                    catalogue: held.catalogue.as_ref(),
                    sets: held.sets.as_deref(),
                    set_icons: &held.set_icons,
                    set_failures: &held.set_failures,
                    search: held.search.as_ref(),
                    templates: &held.documents,
                    refused: &held.refused,
                }
            })
            .collect();
        read(views)
    }

    /// The Online rows for the sources, what was fetched and the tree's state.
    pub(crate) fn rows(
        &self,
        sources: &Sources,
        expanded: &std::collections::HashSet<String>,
        query: &str,
    ) -> Vec<LibraryRow> {
        let ready_added = sources.entries().iter().any(|e| is_ready(&e.address));
        self.with_views(sources, |views| {
            online_rows(&OnlineView {
                sources: views,
                ready_offered: !ready_added,
                expanded,
                query,
            })
        })
    }

    /// The Came from origin of the template at `path`, with its name and whether its fonts stay out.
    pub(crate) fn template_origin(
        &self,
        entry: &SourceEntry,
        path: &str,
    ) -> Option<TemplateOrigin> {
        let template = self.with(&entry.key(), |held| {
            held.catalogue.as_ref().and_then(|c| c.entry(path).cloned())
        })?;
        let licence = template.licence.clone().unwrap_or(Licence {
            spdx: String::new(),
            name: String::new(),
            url: String::new(),
        });
        Some(TemplateOrigin {
            name: template.name.clone(),
            without_fonts: template.fonts_withheld(),
            origin: OnlineOrigin {
                address: template.template_request(&entry.address).url().to_owned(),
                spdx: licence.spdx,
                licence: licence.name,
                licence_url: licence.url,
                author: template.author.clone(),
                credit: template.credit.clone(),
            },
        })
    }

    /// The Came from origin and the 24 mm SVG of icon `name` of `prefix`; the body must be held.
    pub(crate) fn icon_import(
        &self,
        entry: &SourceEntry,
        prefix: &str,
        name: &str,
    ) -> Option<IconImport> {
        self.with(&entry.key(), |held| {
            let icon = held.bodies.get(&(prefix.to_owned(), name.to_owned()))?;
            let set = held
                .sets
                .as_ref()
                .and_then(|sets| sets.iter().find(|s| s.prefix == prefix))
                .or_else(|| {
                    held.search
                        .as_ref()
                        .and_then(|s| s.sets.iter().find(|s| s.prefix == prefix))
                });
            let (set_name, author, licence) = set.map_or_else(
                || (prefix.to_owned(), String::new(), None),
                |s| (s.name.clone(), s.author.clone(), s.licence.clone()),
            );
            let licence = licence.unwrap_or(Licence {
                spdx: String::new(),
                name: String::new(),
                url: String::new(),
            });
            let credit = needs_credit(&licence.spdx)
                .then(|| icon_credit_text(name, &set_name, &author, &licence.name));
            Some(IconImport {
                svg: icon_svg_mm(icon, ICON_WIDTH),
                name: name.to_owned(),
                set: set_name,
                origin: OnlineOrigin {
                    address: format!("{}#{prefix}:{name}", entry.address),
                    spdx: licence.spdx,
                    licence: licence.name,
                    licence_url: licence.url,
                    author,
                    credit,
                },
            })
        })
    }

    /// Whether `url` is a licence address of something this session fetched (the only addresses
    /// the licence link opens).
    pub(crate) fn knows_licence_url(&self, url: &str) -> bool {
        let all = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        all.values().any(|held| {
            held.catalogue
                .iter()
                .flat_map(|c| &c.templates)
                .any(|t| t.licence.as_ref().is_some_and(|l| l.url == url))
                || held
                    .sets
                    .iter()
                    .flatten()
                    .chain(held.search.iter().flat_map(|s| &s.sets))
                    .any(|s| s.licence.as_ref().is_some_and(|l| l.url == url))
        })
    }

    /// Replaces what the session holds of the list of `key` with a freshly fetched `catalogue`.
    pub(crate) fn install_catalogue(&self, key: &str, catalogue: Catalogue) {
        self.with(key, |held| {
            held.failure = None;
            held.catalogue = Some(catalogue);
            held.files.clear();
            held.documents.clear();
            held.refused.clear();
            held.thumbnails.clear();
        });
    }

    pub(crate) fn forget(&self, key: &str) {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(key);
    }

    /// The downloaded template bytes for the preview and the import, when held.
    pub(crate) fn held_file(&self, key: &str, path: &str) -> Option<Arc<Vec<u8>>> {
        self.with(key, |held| held.files.get(path).cloned())
    }
}

/// Why Add did not add a source.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AddStop {
    /// Refused, with the sentence and the field it is shown under.
    Refused(Refusal),
    /// Cancelled while the check ran.
    Cancelled,
}

/// What Load needs of a template.
#[derive(Debug, Serialize)]
pub(crate) struct TemplateOrigin {
    pub(crate) name: String,
    without_fonts: bool,
    origin: OnlineOrigin,
}

/// What an icon import needs.
#[derive(Debug, Serialize)]
pub(crate) struct IconImport {
    svg: String,
    name: String,
    set: String,
    origin: OnlineOrigin,
}

#[cfg(test)]
#[path = "online_tests.rs"]
mod tests;
