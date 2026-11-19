//! The active laser page's job, built lazily at most once per change (ADR 0064 §9; `specs/0029`
//! criteria 3, 12, 36): keyed by the document version, the drawing read, the material library's
//! generation and the extension switches; every operation's plan is reused through one `PlanCache`.
//! The views settle it within a frame's share of dots and else read the last job as stale while
//! the host works the rest in ticks (ADR 0069 §12; `specs/0123` criteria 12, 37 d).

use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock};

use curvyo_document_core::{DocumentVersion, NodeId};
use curvyo_laser_core::job::{
    Built, Gate, JobInput, PlanCache, Timeline, build_job, gate, prepare_job,
};

use super::super::Session;
use super::super::object_cache::DocumentRead;
use super::input::PageInput;
use super::tab_facts::TabFacts;

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

/// The raster dots one tick of the host works: about 10 ms of a Floyd-Steinberg dither in a
/// release build (ADR 0069 §12, `specs/0123` criterion 37 d).
pub(in crate::session) const DOTS_PER_TICK: u64 = 300_000;

/// The raster dots a view's first read after a change works before it falls back to the last
/// job: a third of a tick, so the frame of the edit itself (which also reads the page and draws
/// the preview) keeps its budget, and a small job is still ready at once (ADR 0069 §12).
pub(in crate::session) const FIRST_READ_DOTS: u64 = 100_000;

/// The input of a job not built yet, with the key it was read for: its raster sweeps are worked
/// in ticks.
type Pending = (JobKey, Arc<DocumentRead>, PageInput);

/// The job cache of the session.
#[derive(Debug, Default)]
pub(in crate::session) struct JobCache {
    held: RefCell<Option<Held>>,
    pending: RefCell<Option<Pending>>,
    /// Whether the pending job's sweeps are all held: the next tick builds it, so no tick both
    /// works dots and builds.
    prepared: Cell<bool>,
    /// How many pending inputs were read: a tick that reads a new one has worked its share.
    pendings: Cell<u64>,
    /// Whether the views show a stale job in the stale words: the host asks for them once the job
    /// has been pending for 300 ms, until then the last numbers stay (UX notes P35).
    stale_words: Cell<bool>,
    plans: RefCell<PlanCache>,
    generation: Cell<u64>,
    /// How often a job was built, for the tests.
    #[cfg(test)]
    pub(in crate::session) builds: Cell<usize>,
}

impl JobCache {
    /// An empty plan cache sharing the job's grey pyramids, for the Operations view's preview.
    pub(super) fn sharing_pyramids(&self) -> PlanCache {
        self.plans.borrow().sharing_pyramids()
    }
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
        let pending = cache.pending.borrow_mut().take();
        let page = match pending {
            Some((held, held_read, page)) if held == key && Arc::ptr_eq(&held_read, &read) => {
                Some(page)
            }
            _ => self.page_input(),
        };
        let job = page.map(|page| Arc::new(self.build_page_job(page)));
        *cache.held.borrow_mut() = Some((key, read, job.clone()));
        cache.stale_words.set(false);
        job
    }

    /// The job the views read: the current one when it is built or its raster sweeps take at most
    /// [`FIRST_READ_DOTS`] dots more (worked now, on the first read after a change), else the
    /// last job of the same page with `true` (stale) while [`Session::advance_job`] works the rest.
    /// With the Toolpath view pressed the canvas draws the full job, so the views read it too.
    pub(in crate::session) fn settled_job(&self) -> Option<(Arc<PageJob>, bool)> {
        self.active_manufacturing()?;
        if self.toolpath_view_on() {
            return self.laser_job().map(|job| (job, false));
        }
        let Some(stale) = self.stale_job() else {
            return self.laser_job().map(|job| (job, false));
        };
        let key = self.job_key();
        let read = self.document_read();
        let cache = &self.output.job;
        let tried = cache
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|(held, held_read, _)| *held == key && Arc::ptr_eq(held_read, &read));
        if tried {
            return Some((stale, true));
        }
        let Some(page) = self.page_input() else {
            return self.laser_job().map(|job| (job, false));
        };
        let ready = prepare_job(
            &page.input,
            self.materials().reads(),
            &mut cache.plans.borrow_mut(),
            FIRST_READ_DOTS,
        );
        *cache.pending.borrow_mut() = Some((key, read, page));
        cache.prepared.set(false);
        cache.pendings.set(cache.pendings.get().wrapping_add(1));
        if ready {
            return self.laser_job().map(|job| (job, false));
        }
        Some((stale, true))
    }

    /// The last job of the active laser page while it is not the current one; `None` when the
    /// current job is built or there is no last job of the same page to show meanwhile.
    fn stale_job(&self) -> Option<Arc<PageJob>> {
        let key = self.job_key();
        let read = self.document_read();
        let held = self.output.job.held.borrow();
        match held.as_ref() {
            Some((held_key, held_read, job))
                if !(*held_key == key && Arc::ptr_eq(held_read, &read))
                    && held_key.manufacturing == key.manufacturing
                    && held_key.extensions == key.extensions =>
            {
                job.clone()
            }
            _ => None,
        }
    }

    /// Whether the views read a stale job: the host calls [`Session::advance_job`] in ticks.
    #[must_use]
    pub fn job_pending(&self) -> bool {
        self.settled_job().is_some_and(|(_, stale)| stale)
    }

    /// The host's 300 ms mark of a pending job (UX notes P35): from now until the job is built,
    /// the views show the stale job's times and lengths as "…".
    pub fn show_stale_words(&self) {
        self.output.job.stale_words.set(self.job_pending());
    }

    /// Whether the views write a stale job's times and lengths as "…".
    pub(in crate::session) fn stale_words_shown(&self) -> bool {
        self.output.job.stale_words.get()
    }

    /// One tick of the host (a `requestAnimationFrame`): works up to 300,000 dots of the
    /// pending job, or builds it in the tick after its sweeps are all held. Whether the job was
    /// built by this call, so the host reads the views again.
    pub fn advance_job(&self) -> bool {
        let cache = &self.output.job;
        let pendings = cache.pendings.get();
        if !self.job_pending() || cache.pendings.get() != pendings {
            return false;
        }
        if cache.prepared.get() {
            let _ = self.laser_job();
            return true;
        }
        let ready = self
            .output
            .job
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|(_, _, page)| {
                prepare_job(
                    &page.input,
                    self.materials().reads(),
                    &mut self.output.job.plans.borrow_mut(),
                    DOTS_PER_TICK,
                )
            });
        cache.prepared.set(ready);
        false
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
