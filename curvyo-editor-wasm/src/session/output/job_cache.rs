//! The active laser page's job, built lazily at most once per change (ADR 0064 §9; `specs/0029`
//! criteria 3, 12, 36): keyed by the document version, the drawing read, the material library's
//! generation and the extension switches; every operation's plan is reused through one `PlanCache`.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock};

use curvyo_document_core::{DocumentVersion, NodeId};
use curvyo_laser_core::job::{Built, Gate, JobInput, PlanCache, Timeline, build_job, gate};

use super::super::Session;
use super::super::object_cache::DocumentRead;
use super::input::PageInput;
use super::view_json::TabFacts;

/// One built job of a laser page with what every view of it reads.
#[derive(Debug)]
pub(in crate::session) struct PageJob {
    /// The laser page.
    pub manufacturing: NodeId,
    /// A number that changes with every rebuild: the draw cache and Play compare it.
    pub generation: u64,
    /// The input it was built from.
    pub input: JobInput,
    /// The job and the numbers of each block.
    pub built: Built,
    /// Its lengths and times.
    pub timeline: Timeline,
    /// Its gate.
    pub gate: Gate,
    /// What the Operations tab reads of it beyond the gate, built once per job (criterion 36 d).
    pub tab: OnceLock<TabFacts>,
}

/// What a job was built from: equal keys give an equal job.
#[derive(Debug, Clone, PartialEq)]
struct JobKey {
    version: DocumentVersion,
    manufacturing: Option<NodeId>,
    library: u64,
    extensions: curvyo_ui_core::ExtensionSettings,
}

/// The job held with its key and the drawing read it was built from (held, so its address is
/// never reused while compared).
type Held = (JobKey, Arc<DocumentRead>, Option<Arc<PageJob>>);

/// The job cache of the session.
#[derive(Debug, Default)]
pub(in crate::session) struct JobCache {
    held: RefCell<Option<Held>>,
    plans: RefCell<PlanCache>,
    generation: Cell<u64>,
    /// How often a job was built, for the tests.
    #[cfg(test)]
    pub(in crate::session) builds: Cell<usize>,
}

impl Session {
    /// The key of the active laser page's job now.
    fn job_key(&self) -> JobKey {
        JobKey {
            version: self.document.version(),
            manufacturing: self.active_manufacturing().map(|m| m.id),
            library: self.materials.library_generation(),
            extensions: self.manufacturing.extensions.clone(),
        }
    }

    /// The active laser page's job, built on a miss; `None` on another page or with its
    /// extension off.
    pub(in crate::session) fn laser_job(&self) -> Option<Arc<PageJob>> {
        let read = self.document_read();
        let key = self.job_key();
        let cache = &self.output.job;
        if let Some((held, held_read, job)) = cache.held.borrow().as_ref()
            && *held == key
            && Arc::ptr_eq(held_read, &read)
        {
            return job.clone();
        }
        let job = self
            .page_input()
            .map(|page| Arc::new(self.build_page_job(page)));
        *cache.held.borrow_mut() = Some((key, read, job.clone()));
        job
    }

    fn build_page_job(&self, page: PageInput) -> PageJob {
        let cache = &self.output.job;
        #[cfg(test)]
        cache.builds.set(cache.builds.get() + 1);
        let generation = cache.generation.get().wrapping_add(1);
        cache.generation.set(generation);
        let built = build_job(
            &page.input,
            self.materials().reads(),
            &mut cache.plans.borrow_mut(),
        );
        let timeline = Timeline::of(&built.job);
        let gate = gate(&page.input, &built, &timeline, &page.facts);
        PageJob {
            manufacturing: page.manufacturing,
            generation,
            input: page.input,
            built,
            timeline,
            gate,
            tab: OnceLock::new(),
        }
    }
}
