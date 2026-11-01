//! The trace worker's font fit (`specs/0091-trace-text-recognition` `adrs.md` decision 4): the
//! request and result JSON across the wasm boundary, and the worker's faces kept for its life.
//! Host-independent, so ordinary `cargo test` covers it; `wasm_trace.rs` holds one per worker.
//!
//! Request (made by the session): `{tolerance, faces: [[file, index]], tiers: [[face]] x 5,
//! lines: [{text, direction: [x, y], outlines: [svg]}]}`. A step answers `{kind: "need", faces:
//! [[file, index]]}` (the host hands them in with [`FitWorker::add_face`] and steps again) or
//! `{kind: "finished", lines: [{texts: [..], leftovers: [..]}]}`.

use std::sync::Arc;

use curvyo_text_core::{Face, FaceKey, FaceStore, Mm, RecognisedRun};
use curvyo_ui_core::{FaceState, FitLine, FitResult, JobStep, Tiers, TraceFit};
use kurbo::{Affine, BezPath, Vec2};
use serde_json::{Value, json};

/// The worker's faces and the fit in flight.
#[derive(Default)]
pub struct FitWorker {
    store: FaceStore,
    /// The faces handed in so far, by file and index.
    loaded: Vec<((String, u32), FaceKey)>,
    /// The request's faces, in its order.
    faces: Vec<(String, u32)>,
    /// Faces asked for once: still missing at the next step, they are unavailable.
    requested: Vec<usize>,
    fit: Option<TraceFit>,
}

impl std::fmt::Debug for FitWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FitWorker")
            .field("loaded", &self.loaded.len())
            .finish_non_exhaustive()
    }
}

/// `path` as SVG path data (shortest round-trip numbers: exact).
#[must_use]
pub fn path_svg(path: &BezPath) -> String {
    path.to_svg()
}

fn numbers(value: Option<&Value>) -> Vec<f64> {
    value
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn indices(value: Option<&Value>) -> Vec<usize> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()))
                .collect()
        })
        .unwrap_or_default()
}

fn paths(value: Option<&Value>) -> Vec<BezPath> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|v| BezPath::from_svg(v.as_str()?).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn file_index(value: &Value) -> Option<(String, u32)> {
    let pair = value.as_array()?;
    Some((
        pair.first()?.as_str()?.to_owned(),
        u32::try_from(pair.get(1)?.as_u64()?).ok()?,
    ))
}

/// A text of a finished fit as JSON (`face` indexes the request's faces; `hash` its file's).
fn text_json(text: &curvyo_ui_core::FitText, hash: &str) -> Value {
    let run = &text.run;
    json!({
        "text": run.text,
        "face": text.face,
        "hash": hash,
        "size": run.size.get(),
        "placement": run.placement.as_coeffs(),
        "letter_spacing": run.letter_spacing.get(),
        "word_spacing": run.word_spacing.get(),
        "max_deviation": run.max_deviation.get(),
        "guessed": run.guessed,
        "outlines": run.outlines,
        "sure": run.sure,
        "letters": run.letters,
        "nearest": text.nearest,
        "glyphs": text.glyphs.iter().map(path_svg).collect::<Vec<_>>(),
    })
}

/// One text of a finished fit read back.
#[derive(Debug, Clone, PartialEq)]
pub struct FitTextRead {
    /// The run.
    pub run: RecognisedRun,
    /// The request's face index.
    pub face: usize,
    /// The face file's hash, hex.
    pub hash: String,
    /// Set in the nearest face.
    pub nearest: bool,
    /// The glyphs, document mm.
    pub glyphs: Vec<BezPath>,
}

/// The lines of a finished fit's JSON: per line its texts and leftover outline indices; `None`
/// unless it is a finished result.
#[must_use]
pub fn read_fit_result(json: &str) -> Option<Vec<(Vec<FitTextRead>, Vec<usize>)>> {
    let value: Value = serde_json::from_str(json).ok()?;
    if value.get("kind")?.as_str()? != "finished" {
        return None;
    }
    let mut out = Vec::new();
    for line in value.get("lines")?.as_array()? {
        let mut texts = Vec::new();
        for t in line.get("texts")?.as_array()? {
            let p = numbers(t.get("placement"));
            let p: [f64; 6] = p.try_into().ok()?;
            let count = |name: &str| {
                t.get(name)
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
            };
            let mm = |name: &str| t.get(name).and_then(Value::as_f64).map(Mm);
            texts.push(FitTextRead {
                run: RecognisedRun {
                    face: 0,
                    text: t.get("text")?.as_str()?.to_owned(),
                    size: mm("size")?,
                    placement: Affine::new(p),
                    letter_spacing: mm("letter_spacing")?,
                    word_spacing: mm("word_spacing")?,
                    max_deviation: mm("max_deviation")?,
                    guessed: indices(t.get("guessed")),
                    outlines: indices(t.get("outlines")),
                    sure: count("sure")?,
                    letters: count("letters")?,
                },
                face: count("face")?,
                hash: t.get("hash")?.as_str()?.to_owned(),
                nearest: t.get("nearest")?.as_bool()?,
                glyphs: paths(t.get("glyphs")),
            });
        }
        out.push((texts, indices(line.get("leftovers"))));
    }
    Some(out)
}

impl FitWorker {
    fn face(&self, file: &(String, u32)) -> Option<(FaceKey, Arc<Face>)> {
        let key = self.loaded.iter().find(|(f, _)| f == file)?.1;
        Some((key, self.store.get(&key)?.clone()))
    }

    /// Starts the fit of `request` (see the module) and runs it; a malformed request finishes
    /// with no line.
    pub fn start(&mut self, request: &str, progress: &mut dyn FnMut(u8)) -> String {
        let value: Value = serde_json::from_str(request).unwrap_or_default();
        self.faces = value
            .get("faces")
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(file_index).collect())
            .unwrap_or_default();
        self.requested.clear();
        let mut tiers = Tiers::default();
        if let Some(list) = value.get("tiers").and_then(Value::as_array) {
            for (tier, faces) in tiers.faces.iter_mut().zip(list) {
                *tier = indices(Some(faces))
                    .into_iter()
                    .filter(|&i| i < self.faces.len())
                    .collect();
            }
        }
        let lines: Vec<FitLine> = value
            .get("lines")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|l| {
                        let d = numbers(l.get("direction"));
                        Some(FitLine {
                            text: l.get("text")?.as_str()?.to_owned(),
                            direction: Vec2::new(*d.first()?, *d.get(1)?),
                            outlines: paths(l.get("outlines")),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let tolerance = value
            .get("tolerance")
            .and_then(Value::as_f64)
            .unwrap_or(0.3);
        self.fit = Some(TraceFit::new(lines, Mm(tolerance), tiers));
        self.step(progress)
    }

    /// Hands in face `index` of `file` (the host's file id); `false` when it cannot be read.
    pub fn add_face(&mut self, file: &str, index: u32, bytes: &[u8]) -> bool {
        let id = (file.to_owned(), index);
        if self.loaded.iter().any(|(f, _)| *f == id) {
            return true;
        }
        match self.store.add(bytes.to_vec(), index) {
            Ok(key) => {
                self.loaded.push((id, key));
                true
            }
            Err(_) => false,
        }
    }

    /// Runs the fit until it needs faces or is finished; `progress` hears the lines decided in
    /// percent.
    pub fn step(&mut self, progress: &mut dyn FnMut(u8)) -> String {
        let Some(mut fit) = self.fit.take() else {
            return json!({ "kind": "finished", "lines": [] }).to_string();
        };
        loop {
            let state = |index: usize| match self.faces.get(index) {
                None => FaceState::Unavailable,
                Some(file) => match self.face(file) {
                    Some((_, face)) => FaceState::Ready(face),
                    None if self.requested.contains(&index) => FaceState::Unavailable,
                    None => FaceState::Loading,
                },
            };
            match fit.step(&state) {
                JobStep::NeedFaces(wanted) => {
                    let files: Vec<Value> = wanted
                        .iter()
                        .filter_map(|&i| self.faces.get(i))
                        .map(|(f, n)| json!([f, n]))
                        .collect();
                    self.requested.extend(wanted);
                    self.fit = Some(fit);
                    return json!({ "kind": "need", "faces": files }).to_string();
                }
                JobStep::Running { done, total } => {
                    progress(u8::try_from(100 * done / total.max(1)).unwrap_or(100));
                }
                JobStep::Finished => break,
            }
        }
        let results: Vec<FitResult> = fit.finish(&|index| {
            self.faces
                .get(index)
                .and_then(|file| self.face(file))
                .map(|(_, face)| face)
        });
        let hash_of = |index: usize| {
            self.faces
                .get(index)
                .and_then(|file| self.face(file))
                .map(|(key, _)| key.hash.to_hex())
                .unwrap_or_default()
        };
        let lines: Vec<Value> = results
            .iter()
            .map(|r| {
                json!({
                    "texts": r.texts.iter().map(|t| text_json(t, &hash_of(t.face))).collect::<Vec<_>>(),
                    "leftovers": r.leftovers,
                })
            })
            .collect();
        json!({ "kind": "finished", "lines": lines }).to_string()
    }
}
