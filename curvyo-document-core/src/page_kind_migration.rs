//! The open-time conversion of a format-36 file to page kinds (ADR 0042 §9).
//!
//! One commit with the reserved label `migrate_format`, before anything reads membership:
//! output pages get a place in the list and the plane, and the sketch of each drawing page moves
//! to its own sketch page, a linked clone with own placement keeping the drawing as it looked.

use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::page_frame::{Frame, write_frame};
use crate::pages::{Page, key, next_page_origin, write};
use crate::{PageKind, Vec2};

/// The first format with page kinds; an older file is converted when it is opened.
pub(crate) const PAGE_KINDS_FORMAT: u32 = 37;

impl Document {
    /// Converts a document read from a file of format `found` (ADR 0042 §9). A file that needs
    /// nothing gets no commit; nothing here is an undo step.
    pub(crate) fn migrate_page_kinds(&self, found: u32) {
        if found >= PAGE_KINDS_FORMAT {
            return;
        }
        self.commit_migration(|doc| {
            let pages = doc.pages();
            let mut placed: Vec<Page> = pages
                .iter()
                .filter(|p| !matches!(p.kind, PageKind::Output { .. }))
                .cloned()
                .collect();
            let root = doc.loro().get_map(ROOT_MAP);
            let mut order = placed
                .iter()
                .filter_map(|p| match root.get(&key(&p.id, "order"))?.get_deep_value() {
                    loro::LoroValue::Double(n) if n.is_finite() => Some(n),
                    _ => None,
                })
                .fold(0.0_f64, f64::max);
            // Output pages, in tree order of their manufacturings (all sat at (0, 0)).
            for manufacturing in doc.manufacturings() {
                let Some(mut page) = manufacturing.page else {
                    continue;
                };
                page.origin = next_page_origin(&placed, page.size);
                order += 1.0;
                write(&root, &page.id, "order", order);
                write_frame(
                    &root,
                    &page.id,
                    Frame {
                        origin: page.origin,
                        size: page.size,
                    },
                );
                placed.push(page);
            }
            // Each drawing page's sketch moves to a new sketch page.
            for page in &pages {
                if matches!(page.kind, PageKind::Output { .. }) {
                    continue;
                }
                let Some(sketch) = doc.claimed_sketch(&page.id) else {
                    continue;
                };
                order += 1.0;
                let target = doc.write_sketch_page_for(page, &placed, order);
                doc.move_sketch_to(sketch, page, &target);
                placed.push(target);
            }
        });
        // The moved clones keep their own placement as part of the conversion, not as a user
        // transform: no "stayed in place" notice may follow the open (`0098` criterion 19).
        let _ = self.take_clone_hold();
    }

    /// Writes the sketch page that takes over `page`'s sketch: id `<page id>-sketch` (the same on
    /// every peer), name "<page name> sketch", the same size, at the Add page position.
    fn write_sketch_page_for(&self, page: &Page, placed: &[Page], order: f64) -> Page {
        let root = self.loro().get_map(ROOT_MAP);
        let target = Page {
            id: format!("{}-sketch", page.id),
            name: format!("{} sketch", page.name),
            origin: next_page_origin(placed, page.size),
            size: page.size,
            grid: crate::PageGrid::DEFAULT,
            kind: PageKind::Sketch,
        };
        write(&root, &target.id, "name", target.name.clone());
        write(&root, &target.id, "order", order);
        write(
            &root,
            &target.id,
            crate::page_kind::KIND_FIELD,
            crate::page_kind::SKETCH,
        );
        write_frame(
            &root,
            &target.id,
            Frame {
                origin: target.origin,
                size: target.size,
            },
        );
        target
    }

    /// Leaves a linked clone with own placement of `sketch` at its place on `from`, moves the
    /// sketch by the origin difference with every clone of it held back, and moves the claim.
    fn move_sketch_to(&self, sketch: crate::NodeId, from: &Page, to: &Page) {
        let clone = self.create_clone_above(sketch);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if let Ok(meta) = tree.get_meta(crate::paths::tree_id_of(clone)) {
            crate::clone_codec::write_flag(&meta, crate::clone_codec::KEY_OWN_PLACEMENT, true);
        }
        let delta = Vec2::new(to.origin.x - from.origin.x, to.origin.y - from.origin.y);
        // A refusal (a held clone past the matrix limit) leaves the sketch where it was; the
        // claim still moves, so the file opens and the sketch is edited on its page.
        let _ = self.translate_page_objects(&[sketch], delta);
        let root = self.loro().get_map(ROOT_MAP);
        let owner = crate::sketch_page::OWNER_FIELD;
        // invariant: the attached root map takes plain registers; the old claim was just read.
        #[allow(clippy::unwrap_used)]
        {
            root.delete(&key(&from.id, owner)).unwrap();
            crate::sketch_codec::write(&root, &key(&to.id, owner), &sketch).unwrap();
        }
        if let Ok(meta) = self.sketch_meta(sketch) {
            let _ = crate::sketch_codec::write(&meta, crate::sketch_page::PAGE_KEY, &to.id);
        }
    }
}
