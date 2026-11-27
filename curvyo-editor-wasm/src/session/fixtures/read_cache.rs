//! Holds the fixtures read from the document until the document, the page choice or the
//! extensions change (the `0113` debt entry): a pan, a zoom, a hover or a selection on a laser
//! page reads no manufacturing again, however often a frame asks for its fixtures.

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{DocumentVersion, ManufacturingSnapshot};

use super::super::{Session, Workbench};
use super::placed::PageFixtures;

/// Everything the reads depend on: the document (fixtures and sets are its nodes), the page
/// choice and the extensions switched on.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ReadKey {
    version: DocumentVersion,
    workbench: Workbench,
    extensions: curvyo_ui_core::ExtensionSettings,
}

/// The active laser page's manufacturing with its fixtures as read.
pub(super) type ActiveRead = Option<Arc<(ManufacturingSnapshot, PageFixtures)>>;

/// The last reads and the key they were made for.
#[derive(Debug, Default)]
pub(super) struct ReadCache {
    active: RefCell<Option<(ReadKey, ActiveRead)>>,
    available: RefCell<Option<(ReadKey, Arc<[PageFixtures]>)>>,
    /// How often the document was read, for the tests.
    #[cfg(test)]
    pub(super) reads: std::cell::Cell<usize>,
}

impl Session {
    pub(super) fn read_key(&self) -> ReadKey {
        ReadKey {
            version: self.document.version(),
            workbench: self.workbench(),
            extensions: self.manufacturing.extensions.clone(),
        }
    }

    /// The active page's manufacturing and fixtures, while it is a laser page with its extension
    /// on, before the read-only marks of [`Session::active_fixtures`].
    pub(super) fn active_read(&self) -> ActiveRead {
        let key = self.read_key();
        let cache = &self.fixtures.read_cache;
        if let Some((held, read)) = cache.active.borrow().as_ref()
            && *held == key
        {
            #[cfg(test)]
            assert_eq!(
                *read,
                self.fresh_active(),
                "an input is missing from the key"
            );
            return read.clone();
        }
        #[cfg(test)]
        self.count_read();
        let read = self.fresh_active();
        *cache.active.borrow_mut() = Some((key, read.clone()));
        read
    }

    fn fresh_active(&self) -> ActiveRead {
        let manufacturing = self.active_manufacturing()?;
        if !self.process_enabled(&manufacturing.process) {
            return None;
        }
        let fixtures = PageFixtures::read(&manufacturing)?;
        Some(Arc::new((manufacturing, fixtures)))
    }

    /// The fixtures of every laser page whose extension is on, in document order.
    pub(super) fn available_read(&self) -> Arc<[PageFixtures]> {
        let key = self.read_key();
        let cache = &self.fixtures.read_cache;
        if let Some((held, read)) = cache.available.borrow().as_ref()
            && *held == key
        {
            #[cfg(test)]
            assert_eq!(
                *read,
                self.fresh_available(),
                "an input is missing from the key"
            );
            return Arc::clone(read);
        }
        #[cfg(test)]
        self.count_read();
        let read = self.fresh_available();
        *cache.available.borrow_mut() = Some((key, Arc::clone(&read)));
        read
    }

    fn fresh_available(&self) -> Arc<[PageFixtures]> {
        self.document
            .manufacturings()
            .iter()
            .filter(|m| self.process_enabled(&m.process))
            // Only laser pages hold laser fixtures; a plotter page draws its mat itself (`0159`).
            .filter(|m| m.process == curvyo_laser_core::PROCESS_ID)
            .filter_map(PageFixtures::read)
            .collect()
    }

    #[cfg(test)]
    fn count_read(&self) {
        let reads = &self.fixtures.read_cache.reads;
        reads.set(reads.get() + 1);
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;

    use super::super::tests::laser;
    use super::*;

    fn reads(s: &Session) -> usize {
        s.fixtures.read_cache.reads.get()
    }

    /// A job pin placed at `x, y` on the page, left selected.
    fn pin(s: &mut Session, x: f64, y: f64) -> curvyo_document_core::NodeId {
        let o = s.active_page().origin;
        assert!(s.arm_job_fixture("pin"));
        s.pointer_down(Point::new(o.x + x, o.y + y), false);
        s.pointer_up(Point::new(o.x + x, o.y + y), false, false);
        s.end_fixture_placement();
        s.selected_fixture().expect("the placed pin")
    }

    #[test]
    fn a_pan_a_zoom_and_a_hover_read_no_fixtures_again() {
        let mut s = laser();
        let _ = pin(&mut s, 50.0, 50.0);
        let first = s.frame_draw_list();
        let before = reads(&s);
        let _ = s.frame_draw_list();
        assert_eq!(reads(&s), before, "an unchanged frame reads nothing");
        s.wheel(40.0, 0.0, 100.0, 100.0, false, false);
        let _ = s.frame_draw_list();
        s.wheel(0.0, -120.0, 100.0, 100.0, false, true);
        let _ = s.frame_draw_list();
        s.pointer_hover(Point::new(30.0, 30.0), false, false);
        let _ = s.frame_draw_list();
        assert_eq!(reads(&s), before, "the view and a hover read nothing");
        assert_ne!(s.frame_draw_list(), first, "the view did move");
    }

    #[test]
    fn a_fixture_edit_and_a_page_change_read_again() {
        let mut s = laser();
        let pin = pin(&mut s, 50.0, 50.0);
        let _ = s.frame_draw_list();
        let before = reads(&s);
        assert!(s.set_fixture_grid_value(pin, "columns", 3.0));
        let _ = s.frame_draw_list();
        let after_edit = reads(&s);
        assert!(after_edit > before, "an edit of a fixture reads again");
        s.add_page_of_kind("curvyo.laser", None).unwrap();
        let _ = s.frame_draw_list();
        assert!(reads(&s) > after_edit, "another page reads again");
        assert!(
            s.active_read()
                .is_some_and(|read| read.1.fixtures.is_empty())
        );
    }
}
