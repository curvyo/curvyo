//! The file lists `WebKitGTK` hides from the page, read on the GTK side (`specs/0087-image-clipboard-and-drop`
//! criteria 6 and 21, lead decision 2026-10-29): `WebKitGTK` names `text/uri-list` in a paste or a
//! drop of files from a file manager but gives the page an empty string and no File objects. The
//! host reads the clipboard's file list itself and keeps the list of the drag that was dropped on
//! the webview, so [`crate::local_files`] can read the files.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;
use tauri::{AppHandle, Manager};

use crate::local_files::Source;

/// The main window's label, matching `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";

/// The clipboard formats a file manager puts file names in, read in this order.
const CLIPBOARD_TARGETS: [&str; 2] = ["x-special/gnome-copied-files", "text/uri-list"];

/// How long after a drop the page may take the dropped list.
const DROP_WINDOW: Duration = Duration::from_secs(5);

/// The lists of the drag over the webview: kept while it runs, given out once after it was
/// dropped, forgotten when the next drag starts.
#[derive(Debug, Default)]
struct DragList {
    /// Which drag the lists belong to.
    drag: u64,
    uris: Option<String>,
    text: Option<String>,
    dropped_at: Option<Instant>,
}

impl DragList {
    /// Drag `drag` is over the webview: the lists of any other drag are forgotten.
    fn enter(&mut self, drag: u64) {
        if drag != self.drag {
            *self = Self {
                drag,
                ..Self::default()
            };
        }
    }

    /// The drag's uri list or text arrived.
    fn received(&mut self, uris: Option<String>, text: Option<String>) {
        if uris.is_some() {
            self.uris = uris;
        }
        if text.is_some() {
            self.text = text;
        }
    }

    /// The drag was dropped on the webview.
    fn dropped(&mut self, now: Instant) {
        self.dropped_at = Some(now);
    }

    /// The lists of a drag dropped at most [`DROP_WINDOW`] ago, uri list first, once; nothing for
    /// a drag that was not dropped here.
    fn take(&mut self, now: Instant) -> Vec<String> {
        let fresh = self
            .dropped_at
            .is_some_and(|at| now.saturating_duration_since(at) <= DROP_WINDOW);
        let lists = [self.uris.take(), self.text.take()];
        self.dropped_at = None;
        if fresh {
            lists.into_iter().flatten().collect()
        } else {
            Vec::new()
        }
    }
}

/// The lists of the drag over the main webview, shared with the command.
#[derive(Default)]
pub(crate) struct DraggedLists(Mutex<DragList>);

/// Which drag a GTK drag context is: a new number for each context the webview sees.
#[derive(Default)]
struct DragCounter {
    current: glib::WeakRef<gtk::gdk::DragContext>,
    number: u64,
}

impl DragCounter {
    fn of(&mut self, context: &gtk::gdk::DragContext) -> u64 {
        if self.current.upgrade().as_ref() != Some(context) {
            self.current.set(Some(context));
            self.number += 1;
        }
        self.number
    }
}

/// Keeps the uri list and text of each drag over the main webview, marks its drop and forgets it
/// when the next drag starts; `WebKitGTK` still handles every drag as before.
pub(crate) fn watch_drags(app: &AppHandle) {
    app.manage(DraggedLists::default());
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let handle = app.clone();
    let _ = window.with_webview(move |webview| {
        let view = webview.inner();
        let counter = Rc::new(RefCell::new(DragCounter::default()));
        let with_list = move |context: &gtk::gdk::DragContext, step: &dyn Fn(&mut DragList)| {
            let drag = counter.borrow_mut().of(context);
            if let Ok(mut list) = handle.state::<DraggedLists>().0.lock() {
                list.enter(drag);
                step(&mut list);
            }
        };
        let with_list = Rc::new(with_list);
        let on_motion = Rc::clone(&with_list);
        view.connect_drag_motion(move |_, context, _, _, _| {
            on_motion(context, &|_| {});
            false
        });
        let on_data = Rc::clone(&with_list);
        view.connect_drag_data_received(move |_, context, _, _, data, _, _| {
            let uris = data.uris();
            let uris = (!uris.is_empty()).then(|| {
                uris.iter().fold(String::new(), |mut list, uri| {
                    list.push_str(uri);
                    list.push_str("\r\n");
                    list
                })
            });
            let text = data.text().map(|text| text.to_string());
            on_data(context, &|list| list.received(uris.clone(), text.clone()));
        });
        view.connect_drag_drop(move |_, context, _, _, _| {
            with_list(context, &|list| list.dropped(Instant::now()));
            false
        });
    });
}

/// The texts that may name local files: the clipboard's file lists and text for a paste, the
/// lists of the drag just dropped (taken) for a drop. Blocks until the GTK thread answered.
pub(crate) fn texts(app: &AppHandle, from: Source) -> Vec<String> {
    match from {
        Source::Paste => clipboard_texts(app),
        Source::Drop => app
            .try_state::<DraggedLists>()
            .and_then(|lists| {
                lists
                    .0
                    .lock()
                    .ok()
                    .map(|mut list| list.take(Instant::now()))
            })
            .unwrap_or_default(),
    }
}

/// The clipboard's file lists and text, read on the GTK main thread.
fn clipboard_texts(app: &AppHandle) -> Vec<String> {
    let (answer, read) = mpsc::channel();
    let asked = app.run_on_main_thread(move || {
        let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
        let mut texts: Vec<String> = CLIPBOARD_TARGETS
            .iter()
            .filter_map(|target| clipboard.wait_for_contents(&gtk::gdk::Atom::intern(target)))
            .map(|data| String::from_utf8_lossy(&data.data()).into_owned())
            .collect();
        texts.extend(clipboard.wait_for_text().map(|text| text.to_string()));
        let _ = answer.send(texts);
    });
    if asked.is_err() {
        return Vec::new();
    }
    read.recv().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unnecessary_wraps)] // the shape `received` takes
    fn list(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    #[test]
    fn a_dropped_drag_gives_its_lists_once() {
        let now = Instant::now();
        let mut drags = DragList::default();
        drags.enter(1);
        drags.received(list("file:///tmp/a.png\r\n"), None);
        drags.received(None, list("/tmp/a.png"));
        drags.dropped(now);
        assert_eq!(drags.take(now), ["file:///tmp/a.png\r\n", "/tmp/a.png"]);
        assert_eq!(drags.take(now), [] as [String; 0], "taken once");
    }

    #[test]
    fn a_drag_that_was_not_dropped_here_gives_nothing() {
        let now = Instant::now();
        let mut drags = DragList::default();
        drags.enter(1);
        drags.received(list("file:///tmp/a.png\r\n"), None);
        assert_eq!(drags.take(now), [] as [String; 0]);
    }

    #[test]
    fn the_next_drag_forgets_the_last_ones_lists() {
        let now = Instant::now();
        let mut drags = DragList::default();
        drags.enter(1);
        drags.received(list("file:///home/me/secret.png\r\n"), None);
        drags.dropped(now);
        // A second drag, of text only, is dropped later: the first drag's files are gone.
        drags.enter(2);
        drags.received(None, list("hello"));
        drags.dropped(now);
        assert_eq!(drags.take(now), ["hello"]);
    }

    #[test]
    fn a_drop_taken_too_late_gives_nothing() {
        let now = Instant::now();
        let mut drags = DragList::default();
        drags.enter(1);
        drags.received(list("file:///tmp/a.png\r\n"), None);
        drags.dropped(now);
        assert_eq!(
            drags.take(now + DROP_WINDOW + Duration::from_millis(1)),
            [] as [String; 0]
        );
    }
}
