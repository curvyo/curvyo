//! Copy import from a read-only library source (`specs/0137-document-library` criteria 4 to 6,
//! ADR 0084 §5, §6): a page as a new page that becomes active, or picked elements onto the active
//! page centred on a drop point, each one step, the copies selected.

use curvyo_document_core::{
    CameFrom, Document, FragmentReport, FragmentTarget, ImportError, NodeId, Point, Vec2,
};
use curvyo_ui_core::library_index_text::{
    SKETCH_PAGE_REFUSAL_TEXT, clones_became_paths_text, drawing_fallback_text,
    fallback_reason_text, texts_released_text,
};

use super::{Session, Tool};

/// What an import did, for the page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LibraryImport {
    /// The imported page, for a page import.
    pub page: Option<String>,
    /// The status notice to show (5 s), when there is one.
    pub notice: Option<String>,
}

/// What an online import adds to an import (`specs/0138`): the step's own label and whether the
/// fonts of the source stay out (criterion 9).
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOptions<'a> {
    /// The step's label, instead of the library import's own (`import_icon`).
    pub label: Option<&'a str>,
    /// Leave the source's embedded font files behind.
    pub without_fonts: bool,
}

/// What a read-only source session offers the preview pane.
impl Session {
    /// The top-level objects of `page` in paint order: the page's members by the membership read
    /// (ADR 0051 §4), each taken at its top-level object (a child of a layer or the root).
    #[must_use]
    pub fn page_top_level(&self, page: &str) -> Vec<NodeId> {
        let Some(contents) = self.page_contents() else {
            return Vec::new();
        };
        let leaves = contents.get(page).cloned().unwrap_or_default();
        let mut tops: Vec<NodeId> = Vec::new();
        for leaf in leaves {
            let top = top_level(&self.document, leaf);
            if !tops.contains(&top) {
                tops.push(top);
            }
        }
        tops
    }

    /// The source document, read only.
    #[must_use]
    pub const fn source_document(&self) -> &Document {
        &self.document
    }

    /// Imports `roots` (or, with `page`, that whole page) of the read-only `source` as plain
    /// copies with `came_from`. Elements land on the active page centred on `drop` (canvas
    /// pixels), or on the visible canvas without one. A placed page among them comes in as copies
    /// of its drawing page's members (ADR 0084 decision 6).
    ///
    /// # Errors
    /// The sentence to show when nothing was imported.
    pub fn import_from_library(
        &mut self,
        source: &Self,
        roots: &[NodeId],
        page: Option<&str>,
        came_from: &CameFrom,
        drop: Option<(f64, f64)>,
    ) -> Result<LibraryImport, String> {
        self.import_with(
            source,
            roots,
            page,
            came_from,
            drop,
            ImportOptions::default(),
        )
    }

    /// [`Session::import_from_library`] with the options of an online item.
    ///
    /// # Errors
    /// The sentence to show when nothing was imported.
    pub fn import_with(
        &mut self,
        source: &Self,
        roots: &[NodeId],
        page: Option<&str>,
        came_from: &CameFrom,
        drop: Option<(f64, f64)>,
        options: ImportOptions<'_>,
    ) -> Result<LibraryImport, String> {
        let drop = drop.map(|(x, y)| self.view().screen_to_document(x, y));
        let roots = page.map_or_else(|| roots.to_vec(), |page| source.page_top_level(page));
        let mut fragment = source
            .document
            .extract_fragment_placing(&roots, page, &source.library_placed_members())
            .map_err(|error| error.to_string())?;
        if options.without_fonts {
            fragment = fragment.without_fonts();
        }
        let anchors: Vec<_> = (0..fragment.anchor_count())
            .map(|_| self.minter.mint())
            .collect();
        let active = self.active_page();
        let target = if page.is_some() {
            FragmentTarget::Page
        } else {
            let (min, max) = joint_box(fragment.source(), &fragment.roots());
            let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
            let at = drop.unwrap_or_else(|| self.viewport_centre());
            self.document.set_draw_parent(self.draw_parent(None));
            FragmentTarget::Elements {
                page: &active.id,
                offset: Vec2::new(at.x - centre.x, at.y - centre.y),
            }
        };
        let insert =
            |document: &Document| document.insert_fragment(&fragment, target, came_from, &anchors);
        // An online icon is its own step by name; nested, the outer label is the step's.
        let report = match options.label {
            Some(label) => self.document.batch(label, insert),
            None => insert(&self.document),
        }
        .map_err(|error| match error {
            ImportError::SketchPage => SKETCH_PAGE_REFUSAL_TEXT.to_owned(),
            other => other.to_string(),
        })?;
        Ok(self.after_import(&report))
    }

    /// Loads every page of the read-only `source` as new pages after the last page, one step
    /// "Load template" (`specs/0138` criterion 3); the first loaded page becomes active.
    ///
    /// # Errors
    /// The sentence to show when nothing was imported.
    pub fn load_online_template(
        &mut self,
        source: &Self,
        came_from: &CameFrom,
        without_fonts: bool,
    ) -> Result<LibraryImport, String> {
        let mut fragments = Vec::new();
        for page in source.document.pages() {
            let roots = source.page_top_level(&page.id);
            let mut fragment = source
                .document
                .extract_fragment_placing(&roots, Some(&page.id), &source.library_placed_members())
                .map_err(|error| error.to_string())?;
            if without_fonts {
                fragment = fragment.without_fonts();
            }
            let anchors: Vec<_> = (0..fragment.anchor_count())
                .map(|_| self.minter.mint())
                .collect();
            fragments.push((fragment, anchors));
        }
        let pages: Vec<(
            &curvyo_document_core::Fragment,
            &[curvyo_document_core::AnchorId],
        )> = fragments.iter().map(|(f, a)| (f, a.as_slice())).collect();
        let reports = self
            .document
            .insert_fragment_pages(&pages, came_from)
            .map_err(|error| error.to_string())?;
        let mut done = LibraryImport {
            page: None,
            notice: None,
        };
        let mut notices = Vec::new();
        for report in reports.iter().rev() {
            // The first loaded page is the one left active (UX F3), so it is activated last.
            let one = self.after_import(report);
            notices.extend(one.notice);
            done.page = one.page;
        }
        done.notice = (!notices.is_empty()).then(|| notices.join(" "));
        Ok(done)
    }

    /// Selects the copies, activates an imported page and words the notice.
    fn after_import(&mut self, report: &FragmentReport) -> LibraryImport {
        let lines: Vec<String> = [
            clones_became_paths_text(report.clones_became_paths),
            texts_released_text(report.texts_released),
        ]
        .into_iter()
        .flatten()
        .collect();
        let mut notice = (!lines.is_empty()).then(|| lines.join(" "));
        if let (Some(page), Some(fallback)) = (&report.page, &report.fallback) {
            notice = Some(drawing_fallback_text(
                &page.name,
                &fallback_reason_text(fallback),
            ));
        }
        if let Some(page) = &report.page {
            self.activate_page(&page.id);
        }
        self.tool = Tool::Select;
        self.selection.set(&report.roots);
        LibraryImport {
            page: report.page.as_ref().map(|p| p.id.clone()),
            notice,
        }
    }

    /// Each placed page of this document with the top-level members of its drawing page by the
    /// membership read: what a placement becomes when it is imported.
    fn library_placed_members(&self) -> Vec<(NodeId, Vec<NodeId>)> {
        self.document
            .placed_pages()
            .into_iter()
            .filter_map(|placed| Some((placed.id, self.page_top_level(&placed.source?.id))))
            .collect()
    }

    /// Sets the page's clock for the change times of the Came from lines.
    pub fn set_came_from_clock(&self, now: i64, offset_minutes: i32) {
        self.clock.set((now, offset_minutes));
    }
}

impl Session {
    /// The Came from line of the selection (UX notes section 8.2), as JSON: `{"text",
    /// "tooltip", "open", "record"}` or the empty string when no selected object has one. `now`
    /// and `offset_minutes` give the local time of the tooltip.
    #[must_use]
    pub fn came_from_view(&self, now: i64, offset_minutes: i32) -> String {
        let records: Vec<Option<CameFrom>> = self
            .selection
            .ids()
            .iter()
            .map(|&id| self.document.came_from(id))
            .collect();
        came_from_json(&records, now, offset_minutes)
    }

    /// The Came from line of the page `id`, as [`Session::came_from_view`].
    #[must_use]
    pub fn page_came_from_view(&self, id: &str, now: i64, offset_minutes: i32) -> String {
        came_from_json(&[self.document.page_came_from(id)], now, offset_minutes)
    }
}

fn came_from_json(records: &[Option<CameFrom>], _now: i64, offset_minutes: i32) -> String {
    let Some((text, open)) = curvyo_ui_core::library_index_text::came_from_selection_text(records)
    else {
        return String::new();
    };
    let first = records.iter().flatten().next();
    serde_json::json!({
        "text": text,
        "tooltip": first.map(|r| format!(
            "{text}\n{}",
            curvyo_ui_core::library_index_text::imported_text(r.at.get(), offset_minutes)
        )),
        "open": open,
        "record": first,
        // The licence line of an online item (UX notes 9): "CC BY 4.0 · Jane Doe".
        "online": first.and_then(|r| r.online.as_ref()).map(|o| serde_json::json!({
            "licence": if o.licence.is_empty() { curvyo_ui_core::library_online::LICENCE_UNKNOWN } else { &o.licence },
            "licence_url": o.licence_url,
            "author": o.author,
        })),
    })
    .to_string()
}

impl Session {
    /// The credits of this document's objects as JSON (`specs/0138` criterion 4, UX notes 9):
    /// `[{"text", "licence", "licence_url", "address"}]`, identical ones once.
    #[must_use]
    pub fn credits_json(&self) -> String {
        serde_json::to_string(&self.document.credits()).unwrap_or_default()
    }
}

/// The top-level object `leaf` belongs to: its ancestor whose parent is a layer or the root.
pub(super) fn top_level(document: &Document, leaf: NodeId) -> NodeId {
    let mut current = leaf;
    while let Some(parent) = document.parent_node(current) {
        // A placed page is itself the top: its parent is its laser page's manufacturing.
        if document.is_layer(parent) || document.is_placement(current) {
            break;
        }
        current = parent;
    }
    current
}

/// The joint outline box of `ids` in `document`.
fn joint_box(document: &Document, ids: &[NodeId]) -> (Point, Point) {
    let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &id in ids {
        let leaves = if document.is_group(id) {
            document.leaves_of(id)
        } else {
            vec![id]
        };
        for leaf in leaves {
            if let Some(object) = document.object(leaf) {
                let (min, max) = curvyo_ui_core::object_outline_bounds(&object);
                lo = Point::new(lo.x.min(min.x), lo.y.min(min.y));
                hi = Point::new(hi.x.max(max.x), hi.y.max(max.y));
            }
        }
    }
    if lo.x.is_finite() {
        (lo, hi)
    } else {
        (Point::new(0.0, 0.0), Point::new(0.0, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::super::align_test_support::draw_rect;
    use super::*;
    use curvyo_document_core::UnixMillis;

    fn record() -> CameFrom {
        CameFrom {
            document: "Lid".into(),
            page: "Page 1".into(),
            page_id: "first".into(),
            item: None,
            at: UnixMillis::new(0),
            online: None,
        }
    }

    fn session(peer: u64) -> Session {
        let mut session = Session::new(peer);
        session.resize_viewport(1200.0, 800.0);
        session
    }

    #[test]
    fn elements_land_centred_on_the_drop_point_and_are_selected() {
        let mut source = session(1);
        let rect = draw_rect(&mut source, (0.0, 0.0), (20.0, 10.0));
        let bytes = source.pack("t").unwrap();
        let source = Session::open(3, &bytes).unwrap();
        let mut target = session(2);
        let screen = target.view().document_to_screen(Point::new(100.0, 100.0));
        let done = target
            .import_from_library(&source, &[rect], None, &record(), Some(screen))
            .unwrap();
        assert_eq!(done.page, None);
        let selected = target.selection.ids().to_vec();
        assert_eq!(selected.len(), 1);
        let object = target.document.object(selected[0]).unwrap();
        let (min, max) = curvyo_ui_core::object_outline_bounds(&object);
        assert!((f64::midpoint(min.x, max.x) - 100.0).abs() < 1e-6);
        assert!((f64::midpoint(min.y, max.y) - 100.0).abs() < 1e-6);
        assert_eq!(target.document.came_from(selected[0]), Some(record()));
    }

    #[test]
    fn a_page_comes_in_with_its_members_and_becomes_active() {
        let mut source = session(1);
        draw_rect(&mut source, (10.0, 10.0), (30.0, 20.0));
        let source = Session::open(3, &source.pack("t").unwrap()).unwrap();
        assert_eq!(source.page_top_level("first").len(), 1);
        let mut target = session(2);
        let done = target
            .import_from_library(&source, &[], Some("first"), &record(), None)
            .unwrap();
        let page = done.page.unwrap();
        assert_eq!(target.active_page().id, page);
        assert_eq!(target.document.pages().len(), 2);
        assert_eq!(target.selection.ids().len(), 1);
    }
}
