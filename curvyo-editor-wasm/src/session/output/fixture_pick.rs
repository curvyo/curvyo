//! The output part of the Fixture jobs section (`specs/0029` criterion 2, Frage 221 B; UX notes
//! §6.2, P17, P32): the fixture material pick, one run line per kind and the header's time.

use curvyo_laser_core::job::{BlockSource, Ring, RunFacts, duration_text, read_output_settings};
use curvyo_laser_core::library::MaterialRead;
use curvyo_laser_core::settings::{SetKind, main_set};
use serde::Serialize;

use super::super::Session;
use super::view_json::{NumbersLineView, numbers_view, time_text};

/// The output part of the Fixture jobs section, while a fixture job runs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureOutputView {
    /// The pick "Fixture jobs run with".
    pub pick: FixturePickView,
    /// One run line per kind that runs, in run order.
    pub runs: Vec<FixtureRunView>,
    /// The fixture runs' total time for the collapsed header ("1 min 4 s"; "–" while no run has a
    /// time).
    pub time: String,
}

/// The fixture jobs' material pick (Frage 221 B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixturePickView {
    /// The picked material's id, `None` without a pick.
    pub material: Option<String>,
    /// Its name, `None` without a pick or for a material not loaded here.
    pub name: Option<String>,
    /// The library's materials in their order.
    pub options: Vec<FixtureOptionView>,
}

/// One material of the pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureOptionView {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Whether it has a set of every kind the fixture jobs run for this laser.
    pub pickable: bool,
    /// "No Score set" when not pickable.
    pub note: Option<String>,
}

/// One fixture run line: the fixture jobs of one kind, ringed with the fixture glyph.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureRunView {
    /// The kind (the glyph and the word).
    pub kind: super::super::operations::KindView,
    /// The run's time ("–" without a pick or a set).
    pub time: String,
    /// The numbers of the material's set; empty for "No settings".
    pub numbers: Vec<NumbersLineView>,
}

impl Session {
    /// The output part of the active laser page's Fixture jobs section; `None` while no fixture
    /// job runs (the pick hides, a stored pick stays).
    pub(in crate::session) fn fixture_output_view(&self) -> Option<FixtureOutputView> {
        let job = self.laser_job()?;
        let runs: Vec<&RunFacts> = job
            .gate
            .runs
            .iter()
            .filter(|run| run.ring == Ring::Fixture)
            .collect();
        let kinds: Vec<_> = job
            .input
            .blocks
            .iter()
            .filter(|b| matches!(b.source, BlockSource::Fixture { .. }))
            .filter_map(|b| b.kind.known())
            .map(main_set)
            .collect();
        if kinds.is_empty() {
            return None;
        }
        let manufacturing = self.document.manufacturing(job.manufacturing)?;
        let picked = read_output_settings(&manufacturing).fixture_material;
        let options = self.fixture_options(&kinds, &job.input.machine.profile_id);
        let name = picked
            .as_deref()
            .and_then(|id| self.materials().find(id))
            .map(|m| m.name.clone());
        // "–" as each run's own time when no run has one (no pick, no set), never "0 s".
        let times: Vec<f64> = runs.iter().filter_map(|run| run.time_s).collect();
        let time = if times.is_empty() {
            "–".to_owned()
        } else {
            duration_text(times.iter().sum())
        };
        Some(FixtureOutputView {
            pick: FixturePickView {
                material: picked,
                name,
                options,
            },
            runs: runs
                .iter()
                .map(|run| FixtureRunView {
                    kind: super::super::operations::kind_view_of(run.kind),
                    time: time_text(run),
                    numbers: numbers_view(&run.numbers),
                })
                .collect(),
            time,
        })
    }

    /// The pick's options: the library's loaded materials, each pickable when it has a set of
    /// every kind in `kinds` for the laser `profile`.
    fn fixture_options(&self, kinds: &[SetKind], profile: &str) -> Vec<FixtureOptionView> {
        self.materials()
            .listed()
            .into_iter()
            .filter_map(|read| match read {
                MaterialRead::Loaded(material) => {
                    let missing = kinds
                        .iter()
                        .find(|&&kind| material.set_for(profile, kind).is_none());
                    Some(FixtureOptionView {
                        id: material.id.clone(),
                        name: material.name.clone(),
                        pickable: missing.is_none(),
                        note: missing.map(|kind| format!("No {} set", kind.label())),
                    })
                }
                _ => None,
            })
            .collect()
    }
}
