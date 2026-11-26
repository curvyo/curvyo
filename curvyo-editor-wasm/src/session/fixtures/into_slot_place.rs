//! A whole page put into a slot as a live placed page (`specs/0154-library-into-slots` criterion
//! 3, slice 2; ADR 0088 decision 7): the slot fit becomes the placement's matrix.

use curvyo_document_core::{
    Affine, CameFrom, FragmentTarget, ImportError, NodeId, Page, PageKind, Point, Vec2,
};
use curvyo_laser_core::{FitMode, SlotFit, slot_fit_matrix};
use curvyo_ui_core::library_index_text::clones_became_paths_text;
use curvyo_ui_core::slot_drop_text::{fit_notice, slot_filled_text};

use super::super::{Session, Tool};
use super::into_slot::{SlotTarget, UnitBox};
use super::into_slot_put::{
    PLACE_PAGE_INTO_SLOT_LABEL, REPLACE_SLOT_CONTENT_LABEL, SlotDrop, SlotSource,
};

/// Where the page that is placed comes from.
enum PageOrigin<'a> {
    /// A page of the open document.
    Open(&'a str),
    /// A page of a read-only library source, added as a new page first (`0137` criterion 4).
    Library(&'a SlotSource<'a>),
}

/// The refusal of a page that cannot be a placed page: only a drawing page is placed (`0093`).
const ONLY_DRAWING_TEXT: &str = "Only a drawing page can be placed live.";

/// The refusal of a source that is no whole page: elements are never placed live.
const ONLY_PAGE_TEXT: &str = "Only a whole page can be placed live.";

/// The page-local matrix of a placement whose global map is `fit` (ADR 0088 decision 7):
/// `T(-output origin) o fit o T(source origin)`.
fn local_matrix(output: &Page, source: &Page, fit: Affine) -> Affine {
    Affine::translation(Vec2::new(-output.origin.x, -output.origin.y))
        .compose(fit)
        .compose(Affine::translation(Vec2::new(
            source.origin.x,
            source.origin.y,
        )))
}

/// The frame of `page` as a unit box.
fn frame_box(page: &Page) -> UnitBox {
    let min = page.origin;
    UnitBox::between(
        min,
        Point::new(
            min.x + page.size.width.as_mm(),
            min.y + page.size.height.as_mm(),
        ),
    )
}

impl Session {
    /// The target slot of the active laser page under the document point `at`, the one drawn last.
    pub(in crate::session) fn slot_at(&self, at: Point) -> Option<NodeId> {
        self.slot_targets()
            .into_iter()
            .rev()
            .find(|t| t.holds(at, curvyo_laser_core::FIT_TOLERANCE))
            .map(|t| t.id)
    }

    /// Places the drawing page `page` of the open document into the slot `slot` as a live placed
    /// page, centred and fitted by `mode`, as one step "Place page into slot" (criterion 3). A
    /// filled slot refuses unless `replace` is set: then its content goes in the same step.
    ///
    /// # Errors
    /// The sentence to show when nothing was written for another reason.
    pub fn place_page_into_slot(
        &mut self,
        page: &str,
        slot: NodeId,
        mode: FitMode,
        replace: bool,
    ) -> Result<SlotDrop, String> {
        self.place_into_slot(&PageOrigin::Open(page), slot, mode, replace)
    }

    /// Adds the whole library page `from` as a new page and places it into the slot `slot` as a
    /// live placed page, in one step "Place page into slot" (criterion 3). Picks are refused.
    ///
    /// # Errors
    /// The sentence to show when nothing was written.
    pub fn place_library_page_into_slot(
        &mut self,
        from: &SlotSource<'_>,
        slot: NodeId,
        mode: FitMode,
        replace: bool,
    ) -> Result<SlotDrop, String> {
        if from.page.is_none() {
            return Err(ONLY_PAGE_TEXT.to_owned());
        }
        self.place_into_slot(&PageOrigin::Library(from), slot, mode, replace)
    }

    fn place_into_slot(
        &mut self,
        origin: &PageOrigin<'_>,
        slot: NodeId,
        mode: FitMode,
        replace: bool,
    ) -> Result<SlotDrop, String> {
        let targets = self.slot_targets();
        let target = targets
            .iter()
            .find(|t| t.id == slot)
            .ok_or_else(|| "the slot is gone".to_owned())?;
        let content = self
            .slot_contents(std::slice::from_ref(target))
            .pop()
            .unwrap_or_default();
        if !content.is_empty() && !replace {
            return Ok(SlotDrop {
                refused: Some(slot_filled_text(&target.name)),
                notice: None,
            });
        }
        let source = self.page_to_place(origin)?;
        let unit = frame_box(&source);
        let fit = target.fit(unit, mode);
        let laser = self.active_page();
        let slot_at = target.board.apply_point(target.slot.centre);
        if !laser.contains(slot_at) {
            return Err("the slot lies outside its page".to_owned());
        }
        let prepared = self.prepare_import(origin)?;
        let label = if content.is_empty() {
            PLACE_PAGE_INTO_SLOT_LABEL
        } else {
            REPLACE_SLOT_CONTENT_LABEL
        };
        let placed = self.recording_unguarded(|this| {
            this.document.batch(label, |document| {
                let (page, report) = match &prepared {
                    Some((fragment, anchors, came_from)) => {
                        let report = document
                            .insert_fragment(fragment, FragmentTarget::Page, came_from, anchors)
                            .map_err(|error| match error {
                                ImportError::SketchPage => ONLY_DRAWING_TEXT.to_owned(),
                                other => other.to_string(),
                            })?;
                        let page = report
                            .page
                            .clone()
                            .ok_or_else(|| "the page was not added".to_owned())?;
                        (page, Some(report))
                    }
                    None => (source.clone(), None),
                };
                let matrix = this.placement_matrix(&laser, &page, fit, target);
                let id = document
                    .place_page(&laser.id, &page.id, matrix)
                    .map_err(|error| error.to_string())?;
                if !content.is_empty() {
                    document.delete_objects(&content).map_err(|error| {
                        format!("the slot's content could not be removed: {error}")
                    })?;
                }
                Ok::<_, String>((id, report))
            })
        })?;
        let (id, report) = placed;
        self.tool = Tool::Select;
        self.selection.set(&[id]);
        let real = mode == FitMode::RealSize;
        let clones = report
            .as_ref()
            .and_then(|r| clones_became_paths_text(r.clones_became_paths));
        let notice = match (fit_notice(&target.name, &fit, real), clones) {
            (Some(a), Some(b)) => Some(format!("{a} {b}")),
            (a, b) => a.or(b),
        };
        Ok(SlotDrop {
            refused: None,
            notice,
        })
    }

    /// The page that goes into the slot: a drawing page of the open document or of the library
    /// source, in the coordinates of its own document.
    fn page_to_place(&self, origin: &PageOrigin<'_>) -> Result<Page, String> {
        let (pages, id) = match origin {
            PageOrigin::Open(id) => (self.document.pages(), (*id).to_owned()),
            PageOrigin::Library(from) => (
                from.source.document.pages(),
                from.page.unwrap_or_default().to_owned(),
            ),
        };
        let page = pages
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| "the page is gone".to_owned())?;
        if page.kind == PageKind::Drawing {
            Ok(page)
        } else {
            Err(ONLY_DRAWING_TEXT.to_owned())
        }
    }

    /// The library page's private copy with its minted anchors and Came from, for the batch.
    #[allow(clippy::type_complexity)]
    fn prepare_import(
        &mut self,
        origin: &PageOrigin<'_>,
    ) -> Result<
        Option<(
            curvyo_document_core::Fragment,
            Vec<curvyo_document_core::AnchorId>,
            CameFrom,
        )>,
        String,
    > {
        let PageOrigin::Library(from) = origin else {
            return Ok(None);
        };
        let page = from.page.unwrap_or_default();
        let roots = from.source.page_top_level(page);
        let fragment = from
            .source
            .document
            .extract_fragment_placing(&roots, Some(page), &from.source.library_placed_members())
            .map_err(|error| error.to_string())?;
        let anchors = (0..fragment.anchor_count())
            .map(|_| self.minter.mint())
            .collect();
        Ok(Some((fragment, anchors, from.came_from.clone())))
    }

    /// The placement matrix of `page` into `target`, read after the page was added (an added page
    /// may not move the laser page, but the slot is read again to be sure).
    fn placement_matrix(
        &self,
        laser: &Page,
        page: &Page,
        fit: SlotFit,
        target: &SlotTarget,
    ) -> Affine {
        let output = self
            .document
            .pages()
            .into_iter()
            .find(|p| p.id == laser.id)
            .unwrap_or_else(|| laser.clone());
        let board = self
            .slot_targets()
            .into_iter()
            .find(|t| t.id == target.id)
            .map_or(target.board, |t| t.board);
        let unit = frame_box(page);
        let fit_map = slot_fit_matrix(unit.centre, fit, target.slot.centre, board);
        local_matrix(&output, page, fit_map)
    }
}
