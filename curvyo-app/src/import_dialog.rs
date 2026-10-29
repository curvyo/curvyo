//! The Import button's file dialog on Linux (`specs/0087-image-clipboard-and-drop` criterion 26,
//! `adrs.md` decision 6): GTK3's file chooser with our own preview pane, as GIMP and Inkscape show it.
//!
//! The dialog lives on the GTK main thread; the command waits for its answer on a channel. The
//! highlighted file is decoded off that thread by [`preview_of`], one at a time
//! ([`PreviewSlot`]); a result for a file that is no longer highlighted is dropped. The fonts an
//! SVG's texts need are listed and read in that decode, once per dialog ([`PreviewFonts`]).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};

use gtk::prelude::*;
use gtk::{gdk_pixbuf, gio, glib};
use tauri::{AppHandle, Manager};

use crate::fonts::fonts_dir;
use crate::import_preview::{PREVIEW_SIDE, Preview, TOO_LARGE, preview_of};
use crate::preview_fonts::{FontSources, PreviewFonts};
use crate::preview_slot::PreviewSlot;

/// The main window's label, matching `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";
/// The space around the preview's picture, in pixels.
const PANE_PADDING: i32 = 6;

/// Shows the Import dialog with `filters` (name and extensions, in order) and waits for it: the
/// chosen files, or none when the maker cancelled or the dialog could not be shown.
pub(crate) fn pick_files(app: &AppHandle, filters: &[(&str, &[&str])]) -> Vec<PathBuf> {
    let (answer, chosen) = mpsc::channel();
    let window = app.get_webview_window(MAIN_WINDOW);
    let filters: Vec<(String, Vec<String>)> = filters
        .iter()
        .map(|(name, extensions)| {
            (
                (*name).to_string(),
                extensions.iter().map(|e| (*e).to_string()).collect(),
            )
        })
        .collect();
    let fonts = PreviewFonts::new(font_sources(app));
    let shown = app.run_on_main_thread(move || {
        let parent = window.and_then(|window| window.gtk_window().ok());
        show(parent.as_ref(), &filters, fonts, answer);
    });
    if shown.is_err() {
        return Vec::new();
    }
    chosen.recv().unwrap_or_default()
}

/// The font sources the page lists: the Curvyo folder, the included fonts of the bundled page,
/// the installed fonts.
fn font_sources(app: &AppHandle) -> FontSources {
    let assets = app.clone();
    FontSources {
        curvyo_dir: fonts_dir(app).ok(),
        included: Box::new(move |file| {
            assets
                .asset_resolver()
                .get(format!("fonts/{file}"))
                .map(|asset| asset.bytes)
        }),
        system: true,
    }
}

fn show(
    parent: Option<&gtk::ApplicationWindow>,
    filters: &[(String, Vec<String>)],
    fonts: PreviewFonts,
    answer: mpsc::Sender<Vec<PathBuf>>,
) {
    let dialog = gtk::FileChooserDialog::with_buttons(
        Some("Import"),
        parent,
        gtk::FileChooserAction::Open,
        &[
            ("_Cancel", gtk::ResponseType::Cancel),
            ("_Import", gtk::ResponseType::Accept),
        ],
    );
    dialog.set_modal(true);
    dialog.set_select_multiple(true);
    dialog.set_default_response(gtk::ResponseType::Accept);
    add_filters(&dialog, filters);
    wire_preview(&dialog, fonts);

    // `close` also emits a response (delete event); only the first one answers.
    let answer = RefCell::new(Some(answer));
    dialog.connect_response(move |dialog, response| {
        let Some(answer) = answer.borrow_mut().take() else {
            return;
        };
        let chosen = if response == gtk::ResponseType::Accept {
            dialog.filenames()
        } else {
            Vec::new()
        };
        let _ = answer.send(chosen);
        dialog.close();
    });
    dialog.show();
}

fn add_filters(dialog: &gtk::FileChooserDialog, filters: &[(String, Vec<String>)]) {
    for (name, extensions) in filters {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(name));
        for extension in extensions {
            filter.add_pattern(&format!("*.{extension}"));
        }
        dialog.add_filter(filter);
    }
}

/// The preview pane on the right and the "Enable preview" check (on) at the bottom left.
fn wire_preview(dialog: &gtk::FileChooserDialog, fonts: PreviewFonts) {
    let previewer = Rc::new(Previewer {
        pane: Pane::new(),
        slot: RefCell::new(PreviewSlot::default()),
        fonts: Arc::new(Mutex::new(fonts)),
    });
    dialog.set_preview_widget(&previewer.pane.root);
    dialog.set_use_preview_label(false);
    let enable = gtk::CheckButton::with_mnemonic("Enable _preview");
    enable.set_active(true);
    dialog.set_extra_widget(&enable);

    let update = {
        // Weak: the check's own handler holds this closure.
        let enable = enable.downgrade();
        move |chooser: &gtk::FileChooserDialog| {
            let enabled = enable.upgrade().is_none_or(|check| check.is_active());
            chooser.set_preview_widget_active(enabled);
            let path = if enabled {
                chooser.preview_filename()
            } else {
                None
            };
            previewer.request(path);
        }
    };
    let update = Rc::new(update);
    dialog.connect_update_preview({
        let update = Rc::clone(&update);
        move |chooser| update(chooser)
    });
    let weak = dialog.downgrade();
    enable.connect_toggled(move |_| {
        if let Some(dialog) = weak.upgrade() {
            update(&dialog);
        }
    });
}

/// The pane and its single decode: one file decodes off the GTK thread at a time, the newest
/// highlight waits, and a result for a file that is no longer highlighted is dropped.
struct Previewer {
    pane: Pane,
    slot: RefCell<PreviewSlot>,
    /// Used by the one running decode only.
    fonts: Arc<Mutex<PreviewFonts>>,
}

impl Previewer {
    /// A new highlight (`None`: nothing to preview). The pane empties at once.
    fn request(self: &Rc<Self>, path: Option<PathBuf>) {
        self.pane.show(&Preview::Nothing);
        let start = self.slot.borrow_mut().request(path);
        if let Some(path) = start {
            Rc::clone(self).decode(path);
        }
    }

    fn decode(self: Rc<Self>, path: PathBuf) {
        glib::MainContext::default().spawn_local(async move {
            let fonts = Arc::clone(&self.fonts);
            let preview = gio::spawn_blocking(move || {
                let mut fonts = fonts
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                preview_of(&path, &mut fonts)
            })
            .await;
            let after = self.slot.borrow_mut().finished();
            if after.show
                && let Ok(preview) = preview
            {
                self.pane.show(&preview);
            }
            if let Some(next) = after.next {
                self.decode(next);
            }
        });
    }
}

/// The preview pane: the picture over one or two lines of text.
struct Pane {
    root: gtk::Box,
    image: gtk::Image,
    text: gtk::Label,
}

impl Pane {
    fn new() -> Self {
        let side = i32::try_from(PREVIEW_SIDE).unwrap_or(i32::MAX);
        let root = gtk::Box::new(gtk::Orientation::Vertical, PANE_PADDING);
        root.set_border_width(u32::try_from(PANE_PADDING).unwrap_or_default());
        let image = gtk::Image::new();
        image.set_size_request(side, side);
        let text = gtk::Label::new(None);
        text.set_justify(gtk::Justification::Center);
        text.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        text.set_max_width_chars(28);
        root.pack_start(&image, false, false, 0);
        root.pack_start(&text, false, false, 0);
        root.show_all();
        Self { root, image, text }
    }

    fn show(&self, preview: &Preview) {
        self.image.clear();
        match preview {
            Preview::Nothing => self.text.set_text(""),
            Preview::NameOnly { name } => self.text.set_text(name),
            Preview::TooLarge { name } => self.text.set_text(&format!("{name}\n{TOO_LARGE}")),
            Preview::Picture { picture, size_line } => {
                if let Some(pixbuf) = pixbuf_of(picture) {
                    self.image.set_from_pixbuf(Some(&pixbuf));
                }
                self.text.set_text(size_line);
            }
        }
    }
}

/// The picture as a pixbuf that only holds its pixels (gdk-pixbuf decodes nothing here).
fn pixbuf_of(picture: &curvyo_image_core::WorkingImage) -> Option<gdk_pixbuf::Pixbuf> {
    let width = i32::try_from(picture.width).ok()?;
    let height = i32::try_from(picture.height).ok()?;
    let stride = width.checked_mul(4)?;
    Some(gdk_pixbuf::Pixbuf::from_mut_slice(
        picture.pixels.clone(),
        gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        width,
        height,
        stride,
    ))
}
