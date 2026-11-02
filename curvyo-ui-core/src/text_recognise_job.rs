//! Searches the face tiers of Recognise text line by line, one line and tier per step
//! (`specs/0088-trace-recognition` criteria 8 and 9; `adrs.md` decisions 3a and 5).
//!
//! The tiers in the customer's order: faces used in the document, favourites, the fonts the
//! maker installed (Curvyo fonts folder), the included fonts, the other installed fonts. A face
//! counts in its earliest tier; a tier holds outline faces only, at most [`TIER_CAP`]. A line
//! stops at the first tier whose best face has 80 % of its letters sure; when no tier does, the
//! best face over all tiers wins if half of its letters are sure. Faces that are not loaded are
//! asked for and the step waits for them, so the system fonts are read only for lines the earlier
//! tiers could not read.

use std::sync::Arc;

use curvyo_document_core::{FontKind, FontRef};
use curvyo_text_core::{Face, LineMatch, Mm, RecognisedRun, Recogniser};
use kurbo::BezPath;

use crate::family_key::family_key;
use crate::font_match::{FaceMatch, FontBlock, ListedFace, match_face};

/// The most faces one tier searches (Proposal, `adrs.md` decision 5).
pub const TIER_CAP: usize = 64;

/// One line of a job: its outline set, its line there, and the face that came nearest on it
/// (listing index, deviation in mm).
pub type LineNearest = (usize, usize, Option<(usize, f64)>);

/// The tiers, searched in this order (Frage 102 B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaceTier {
    /// Faces already used by texts of the document.
    Document,
    /// The maker's favourite families (`0075`).
    Favourites,
    /// The fonts the maker installed (the Curvyo fonts folder).
    Installed,
    /// The fonts shipped with the app.
    Included,
    /// The other fonts installed on the computer.
    System,
}

impl FaceTier {
    /// The tiers in search order.
    pub const ALL: [Self; 5] = [
        Self::Document,
        Self::Favourites,
        Self::Installed,
        Self::Included,
        Self::System,
    ];

    /// The code the host words.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Favourites => "favourites",
            Self::Installed => "installed",
            Self::Included => "included",
            Self::System => "system",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Document => 0,
            Self::Favourites => 1,
            Self::Installed => 2,
            Self::Included => 3,
            Self::System => 4,
        }
    }
}

/// The listed faces of each tier (indices into the listing) and how many each tier had before
/// the cap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tiers {
    /// Per tier, the faces searched, in listing order.
    pub faces: [Vec<usize>; 5],
    /// Per tier, the faces it had before the cap.
    pub before_cap: [usize; 5],
}

impl Tiers {
    /// How many tiers have a face to search (the readout's "5 face sources").
    #[must_use]
    pub fn sources(&self) -> usize {
        self.faces.iter().filter(|f| !f.is_empty()).count()
    }
}

/// The listed faces the document's texts use (tier 1); outline faces only.
fn document_faces(listed: &[ListedFace], document_fonts: &[FontRef]) -> Vec<usize> {
    document_fonts
        .iter()
        .filter(|f| f.kind == FontKind::Outline)
        .filter_map(|font| match match_face(font, listed) {
            FaceMatch::Face(key) => listed.iter().position(|face| face.key == Some(key)),
            FaceMatch::NeedsLoad(index) => Some(index),
            FaceMatch::Missing => None,
        })
        .collect()
}

/// The tier of a listed outline face the document does not use: favourites, then its block.
fn later_tier(face: &ListedFace, favourite_keys: &[String]) -> FaceTier {
    if favourite_keys.contains(&family_key(&face.family)) {
        return FaceTier::Favourites;
    }
    match face.block {
        FontBlock::Curvyo => FaceTier::Installed,
        FontBlock::Included => FaceTier::Included,
        FontBlock::System => FaceTier::System,
    }
}

/// The tiers of `listed` (criterion 8): `document_fonts` are the fonts of the document's texts,
/// `favourites` the family names the maker starred. A face counts in its earliest tier; every
/// tier but the document's is capped at [`TIER_CAP`].
#[must_use]
pub fn recognise_tiers(
    listed: &[ListedFace],
    document_fonts: &[FontRef],
    favourites: &[String],
) -> Tiers {
    let favourite_keys: Vec<String> = favourites.iter().map(|f| family_key(f)).collect();
    let document = document_faces(listed, document_fonts);
    let mut tiers = Tiers::default();
    for (index, face) in listed.iter().enumerate() {
        let tier = if document.contains(&index) {
            FaceTier::Document
        } else if face.kind == FontKind::Outline {
            later_tier(face, &favourite_keys)
        } else {
            continue;
        };
        let at = tier.index();
        tiers.before_cap[at] += 1;
        if tier == FaceTier::Document || tiers.faces[at].len() < TIER_CAP {
            tiers.faces[at].push(index);
        }
    }
    tiers
}

/// Whether a listed face can be searched now.
#[derive(Debug, Clone)]
pub enum FaceState {
    /// Loaded.
    Ready(Arc<Face>),
    /// Asked for; the step waits.
    Loading,
    /// Unreadable or gone: the tier goes on without it.
    Unavailable,
}

/// What one step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStep {
    /// These listed faces must be loaded before the job can go on; nothing was searched.
    NeedFaces(Vec<usize>),
    /// One line was searched in one tier, or one face of it was made ready; `done` of `total`
    /// lines are decided.
    Running {
        /// Lines decided.
        done: usize,
        /// Lines found.
        total: usize,
    },
    /// Every line is decided.
    Finished,
}

/// A line that became text.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedLine {
    /// The outline set it is from (0 for a selection; the OCR line for Trace, `specs/0091`).
    pub source: usize,
    /// The tier its face is from.
    pub tier: FaceTier,
    /// Its face, an index into the listing.
    pub face: usize,
    /// The text.
    pub run: RecognisedRun,
}

/// The best match of a line so far.
#[derive(Debug, Clone)]
struct Best {
    tier: FaceTier,
    face: usize,
    found: LineMatch,
}

/// The face that came nearest on some line: its listing index, its largest deviation, and how
/// telling the line is (over the tolerance, several glyphs).
#[derive(Debug, Clone)]
struct Near {
    face: usize,
    deviation: f64,
    rank: (bool, bool),
}

/// One Recognise text run over a selection: the lines, the tiers and the decisions so far.
#[derive(Debug, Clone)]
pub struct RecogniseJob {
    /// One per outline set: a selection has one, Trace one per OCR line (`specs/0091`).
    recognisers: Vec<Recogniser>,
    /// Every line to decide: its recogniser and its line there.
    order: Vec<(usize, usize)>,
    /// Per line of `order`: the face that came nearest on it (listing index, deviation).
    line_nearest: Vec<Option<(usize, f64)>>,
    tiers: Tiers,
    line: usize,
    tier: usize,
    best: Option<Best>,
    accepted: Vec<AcceptedLine>,
    /// The nearest face so far.
    nearest: Option<Near>,
    /// The tolerance, mm.
    tolerance: f64,
    last_tier: Option<FaceTier>,
}

impl RecogniseJob {
    /// A job over `outlines` (one closed subpath each, document millimetres).
    #[must_use]
    pub fn new(outlines: &[BezPath], tolerance: Mm, tiers: Tiers) -> Self {
        Self::of(vec![Recogniser::new(outlines, tolerance)], tolerance, tiers)
    }

    /// A job over Trace's OCR lines (`specs/0091` `adrs.md` decision 4): per line its outlines
    /// (one closed subpath each, document millimetres), the string the OCR read (the hint) and
    /// the line's own tolerance (criterion 26); `tolerance` ranks the nearest face.
    #[must_use]
    pub fn with_hints(lines: &[(Vec<BezPath>, String, Mm)], tolerance: Mm, tiers: Tiers) -> Self {
        let recognisers = lines
            .iter()
            .map(|(outlines, text, own)| Recogniser::with_hint(outlines, *own, text))
            .collect();
        Self::of(recognisers, tolerance, tiers)
    }

    fn of(recognisers: Vec<Recogniser>, tolerance: Mm, tiers: Tiers) -> Self {
        let order: Vec<(usize, usize)> = recognisers
            .iter()
            .enumerate()
            .flat_map(|(r, rec)| (0..rec.line_count()).map(move |l| (r, l)))
            .collect();
        Self {
            line_nearest: vec![None; order.len()],
            recognisers,
            order,
            tiers,
            line: 0,
            tier: 0,
            best: None,
            accepted: Vec::new(),
            nearest: None,
            tolerance: tolerance.get(),
            last_tier: None,
        }
    }

    /// The number of lines found.
    #[must_use]
    pub fn lines(&self) -> usize {
        self.order.len()
    }

    /// The outline sets' recognisers, in order.
    #[must_use]
    pub fn recognisers(&self) -> &[Recogniser] {
        &self.recognisers
    }

    /// Per line: its outline set, its line there, and the face that came nearest on it
    /// (listing index, deviation in mm).
    #[must_use]
    pub fn line_nearest(&self) -> Vec<LineNearest> {
        self.order
            .iter()
            .zip(&self.line_nearest)
            .map(|(&(r, l), near)| (r, l, *near))
            .collect()
    }

    /// The lines that became text, in line order.
    #[must_use]
    pub fn accepted(&self) -> &[AcceptedLine] {
        &self.accepted
    }

    /// The face that came nearest over every search (listing index, deviation in mm).
    #[must_use]
    pub fn nearest(&self) -> Option<(usize, f64)> {
        self.nearest.as_ref().map(|n| (n.face, n.deviation))
    }

    /// The last tier searched.
    #[must_use]
    pub const fn last_tier(&self) -> Option<FaceTier> {
        self.last_tier
    }

    /// The tiers.
    #[must_use]
    pub const fn tiers(&self) -> &Tiers {
        &self.tiers
    }

    fn next_line(&mut self) {
        if let Some(best) = self.best.take()
            && let Some(run) = best.found.run
        {
            self.accepted.push(AcceptedLine {
                source: self.order[self.line].0,
                tier: best.tier,
                face: best.face,
                run,
            });
        }
        self.line += 1;
        self.tier = 0;
    }

    /// Takes the match of the current line in `tier` (`listed`: the listing index of each face
    /// searched): the nearest face, the best match so far, and the next tier or line.
    fn record(&mut self, tier: FaceTier, listed: &[usize], found: LineMatch) {
        self.last_tier = Some(tier);
        if let Some(near) = found.nearest {
            // A line of one dot fits any face, and a line of dots fits within the tolerance
            // without a margin: lines of several glyphs judged over the tolerance name the
            // nearest face when there are any (the notice's deviation is then over it).
            let deviation = near.max_deviation.get();
            let own = &mut self.line_nearest[self.line];
            if own.is_none_or(|(_, d)| deviation < d) {
                *own = Some((listed[near.face], deviation));
            }
            let candidate = Near {
                face: listed[near.face],
                deviation,
                rank: (deviation > self.tolerance, found.letters >= 2),
            };
            let better = self.nearest.as_ref().is_none_or(|n| {
                candidate.rank > n.rank || (candidate.rank == n.rank && deviation < n.deviation)
            });
            if better {
                self.nearest = Some(candidate);
            }
        }
        let stops = found
            .run
            .as_ref()
            .is_some_and(|run| 5 * run.sure >= 4 * run.letters);
        let face = found.run.as_ref().map(|run| listed[run.face]);
        if let Some(face) = face
            && self.best.as_ref().is_none_or(|b| found.beats(&b.found))
        {
            self.best = Some(Best { tier, face, found });
        }
        if stops {
            self.next_line();
        } else {
            self.tier += 1;
        }
    }

    /// Searches the current line in the current tier, or asks for the faces it needs.
    pub fn step(&mut self, face: &dyn Fn(usize) -> FaceState) -> JobStep {
        loop {
            let total = self.lines();
            if self.line >= total {
                return JobStep::Finished;
            }
            if self.tier >= FaceTier::ALL.len() {
                self.next_line();
                continue;
            }
            let tier = FaceTier::ALL[self.tier];
            let listed = &self.tiers.faces[self.tier];
            let mut waiting = Vec::new();
            let mut ready: Vec<(usize, Arc<Face>)> = Vec::new();
            for &index in listed {
                match face(index) {
                    FaceState::Ready(found) => ready.push((index, found)),
                    FaceState::Loading => waiting.push(index),
                    FaceState::Unavailable => {}
                }
            }
            if !waiting.is_empty() {
                return JobStep::NeedFaces(waiting);
            }
            if ready.is_empty() {
                self.tier += 1;
                continue;
            }
            // A cold face is built on a step of its own, so the host paints and can stop the run
            // between faces (about 25 ms each, `docs/technical-debt.md`).
            if let Some((_, cold)) = ready.iter().find(|(_, f)| !f.recognition_ready()) {
                cold.prepare_recognition();
                return JobStep::Running {
                    done: self.line,
                    total,
                };
            }
            let faces: Vec<&Face> = ready.iter().map(|(_, f)| f.as_ref()).collect();
            let (r, l) = self.order[self.line];
            let found = self.recognisers[r].match_line(l, &faces);
            let listed: Vec<usize> = ready.iter().map(|(index, _)| *index).collect();
            self.record(tier, &listed, found);
            return JobStep::Running {
                done: self.line,
                total,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_text_core::{FaceHash, FaceKey};

    fn listed(block: FontBlock, family: &str, style: &str, kind: FontKind) -> ListedFace {
        ListedFace {
            block,
            family: family.to_owned(),
            style: style.to_owned(),
            host_id: format!("{family} {style}"),
            index: 0,
            key: Some(FaceKey {
                hash: FaceHash::of(format!("{family} {style}").as_bytes()),
                index: 0,
            }),
            kind,
        }
    }

    #[test]
    fn criterion_8_a_face_counts_in_its_earliest_tier() {
        let faces = vec![
            listed(FontBlock::Curvyo, "Mine", "Regular", FontKind::Outline),
            listed(
                FontBlock::Included,
                "Source Sans 3",
                "Regular",
                FontKind::Outline,
            ),
            listed(
                FontBlock::Included,
                "Source Sans 3",
                "Bold",
                FontKind::Outline,
            ),
            listed(
                FontBlock::Included,
                "EMS Readability",
                "Regular",
                FontKind::SingleLine,
            ),
            listed(FontBlock::System, "DejaVu Sans", "Book", FontKind::Outline),
            listed(
                FontBlock::System,
                "Liberation Serif",
                "Regular",
                FontKind::Outline,
            ),
        ];
        let used = FontRef {
            family: "Source Sans 3".to_owned(),
            style: "Bold".to_owned(),
            face_hash: faces[2].key.unwrap_or_else(|| unreachable!()).hash.to_hex(),
            face_index: 0,
            kind: FontKind::Outline,
        };
        let tiers = recognise_tiers(&faces, &[used], &["dejavu  sans".to_owned()]);
        assert_eq!(tiers.faces[0], vec![2]);
        assert_eq!(tiers.faces[1], vec![4]);
        assert_eq!(tiers.faces[2], vec![0]);
        assert_eq!(tiers.faces[3], vec![1]);
        assert_eq!(tiers.faces[4], vec![5]);
        assert_eq!(tiers.sources(), 5);
    }

    #[test]
    fn a_tier_is_capped_at_64_faces() {
        let faces: Vec<ListedFace> = (0..70)
            .map(|i| {
                listed(
                    FontBlock::System,
                    &format!("F{i}"),
                    "Regular",
                    FontKind::Outline,
                )
            })
            .collect();
        let tiers = recognise_tiers(&faces, &[], &[]);
        assert_eq!(tiers.faces[4].len(), TIER_CAP);
        assert_eq!(tiers.before_cap[4], 70);
    }
}
