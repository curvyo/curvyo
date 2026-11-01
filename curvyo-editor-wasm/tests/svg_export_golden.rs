//! The exported goldens of the SVG export (`specs/0024-svg-import-export` criteria 63, 64 and
//! 153): every import fixture is opened through the session (texts in the included faces),
//! exported, and compared byte for byte with `name.exported.svg` in `fixtures/export/real/`; the
//! export opened and exported again gives the same bytes. The export documents of criterion 153
//! are opened from the hand-written sources `name.source.svg` in
//! `curvyo-svg-core/tests/fixtures/export/` (the sketch, which no SVG can make, is built here);
//! the document the export reads is pinned as `name.document.json`. Regenerate with
//! `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::map_unwrap_or,
    missing_docs
)]

use std::path::{Path, PathBuf};

use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_ui_core::{FontBlock, SvgExportOptions};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../curvyo-svg-core/tests/fixtures")
}

fn clock() -> curvyo_document_core::Timestamp {
    curvyo_document_core::Timestamp::from_secs(1_950_000_000)
}

/// `(file, family, style, index)` of every included face.
fn included_faces() -> Vec<(String, String, String, u32)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect()
}

/// Hands every wanted face over, as the host does; returns how many.
fn load(s: &mut Session) -> usize {
    let wanted = s.wanted_font_files();
    for file in &wanted {
        let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
        assert!(s.add_font_file(file, &bytes), "{file} loads");
    }
    wanted.len()
}

/// Open… of an SVG with the host's loop: load what it wants until it stops waiting, then lay
/// out the texts.
fn open(bytes: &[u8]) -> Session {
    let mut s = Session::open_file(32, bytes, "drawing.svg").unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    for _ in 0..8 {
        load(&mut s);
        if !s.continue_open_svg(false) {
            break;
        }
    }
    // The texts of the opened document ask for their faces too.
    for _ in 0..4 {
        if load(&mut s) == 0 {
            break;
        }
    }
    s
}

fn export(s: &Session, options: SvgExportOptions) -> String {
    match s.export_svg(options) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing selected"),
    }
}

fn golden(path: &Path, text: &str) {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(path, text).unwrap();
    }
    let want = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display()));
    assert!(text == want, "{} differs from its golden", path.display());
}

/// Every import fixture of the real-file folders.
fn import_fixtures() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [
        "svg_real/illustrator",
        "svg_real/inkscape",
        "svg_real/w3c",
        "svg_real/wikimedia",
        "svg_real/inkscape_text",
    ] {
        let Ok(entries) = std::fs::read_dir(fixtures_dir().join(dir)) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name.ends_with(".svg")
                && !name.ends_with(".paths.svg")
                && !name.ends_with(".exported.svg")
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn exported_path(source: &Path) -> PathBuf {
    // Beside the fixtures in a mirror folder: the import tests read every `.svg` of theirs.
    let stem = source.file_stem().unwrap().to_string_lossy().to_string();
    let dir = source
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let out = fixtures_dir().join("export/real").join(dir);
    std::fs::create_dir_all(&out).unwrap();
    out.join(format!("{stem}.exported.svg"))
}

/// Criteria 63 and 64 (b): export(open(f)) is its golden, and export(open(export(open(f)))) is
/// byte-identical to it.
#[test]
fn ac63_ac64_every_fixture_exports_its_golden_and_is_stable() {
    let fixtures = import_fixtures();
    assert!(fixtures.len() >= 10, "fixtures found: {}", fixtures.len());
    let mut unstable = Vec::new();
    for source in &fixtures {
        let bytes = std::fs::read(source).unwrap();
        // A file the import refuses (criterion 6) has nothing to export.
        if Session::open_file(32, &bytes, "drawing.svg").is_err() {
            continue;
        }
        let first = export(&open(&bytes), SvgExportOptions::default());
        golden(&exported_path(source), &first);
        let again = export(&open(first.as_bytes()), SvgExportOptions::default());
        if again != first {
            let line = again
                .lines()
                .zip(first.lines())
                .find(|(a, b)| a != b)
                .map(|(a, b)| format!("{a}\n{b}"))
                .unwrap_or_else(|| "length".to_owned());
            unstable.push(format!("{}:\n{line}", source.display()));
        }
    }
    assert!(
        unstable.is_empty(),
        "second export differs:\n{}",
        unstable.join("\n")
    );
}

/// The sketch of criterion 138: four connected lines, a circle, an arc and a construction line.
fn sketch_project() -> Vec<u8> {
    use curvyo_sketch_core as sk;
    let mut g = sk::Sketch::new("origin".into());
    let mut point = |id: &str, x: f64, y: f64| {
        g.entities.insert(
            id.into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, y),
                fixed: true,
            },
        );
    };
    point("origin", 0.0, 0.0);
    for (i, (x, y)) in [(10.0, 10.0), (50.0, 10.0), (50.0, 40.0), (10.0, 40.0)]
        .into_iter()
        .enumerate()
    {
        point(&format!("p{i}"), x, y);
    }
    point("c", 100.0, 100.0);
    point("ac", 150.0, 100.0);
    point("as", 160.0, 100.0);
    point("ae", 150.0, 110.0);
    point("k0", 0.0, 200.0);
    point("k1", 100.0, 200.0);
    for i in 0..4 {
        g.entities.insert(
            format!("line{i}").as_str().into(),
            sk::Entity::Line {
                start: format!("p{i}").as_str().into(),
                end: format!("p{}", (i + 1) % 4).as_str().into(),
            },
        );
    }
    g.entities.insert(
        "circle".into(),
        sk::Entity::Circle {
            center: "c".into(),
            radius: sk::Length(15.0),
            fixed_radius: true,
        },
    );
    g.entities.insert(
        "arc".into(),
        sk::Entity::Arc {
            center: "ac".into(),
            start: "as".into(),
            end: "ae".into(),
            radius: sk::Length(10.0),
            start_angle: sk::Angle(0.0),
            end_angle: sk::Angle(std::f64::consts::FRAC_PI_2),
            fixed_shape: true,
        },
    );
    g.entities.insert(
        "construction".into(),
        sk::Entity::Line {
            start: "k0".into(),
            end: "k1".into(),
        },
    );
    g.construction.insert("construction".into());
    let d = curvyo_document_core::Document::new(801);
    d.create_page_sketch(
        &d.pages()[0].id,
        &g,
        &curvyo_document_core::Style::default(),
    )
    .unwrap();
    curvyo_document_core::pack(&d, "golden").unwrap()
}

/// A project opened in the session, with its faces.
fn open_project(bytes: &[u8]) -> Session {
    let mut s = Session::open(33, bytes).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    for _ in 0..4 {
        if load(&mut s) == 0 {
            break;
        }
    }
    s
}

/// The document the session holds, as `document.json`.
fn document_json(s: &Session) -> String {
    let d = curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    String::from_utf8(d.export_json().unwrap()).unwrap()
}

/// Selects the contents of the layer named "Selected" (criterion 131's document).
fn select_the_selected_layer(s: &mut Session) {
    let d = curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let layer = d
        .read_nodes()
        .into_iter()
        .find(|n| n.registers.name.as_deref() == Some("Selected"))
        .expect("a layer named Selected");
    s.select_layer_contents(&serde_json::to_string(&layer.id).unwrap());
}

/// One export document: its session (opened again from `reopen` for the second pass).
enum Source {
    Svg(PathBuf),
    Project(Vec<u8>),
}

impl Source {
    fn open(&self) -> Session {
        match self {
            Self::Svg(path) => open(&std::fs::read(path).unwrap()),
            Self::Project(bytes) => open_project(bytes),
        }
    }
}

/// The export documents of criterion 153: `name.source.svg` (or the sketch built here) opened,
/// exported with `name.options` (one option name per line) and compared with
/// `name.exported.svg`, the opened document with `name.document.json`; the export is stable
/// under the same options.
#[test]
fn ac153_export_documents_have_their_goldens() {
    let dir = fixtures_dir().join("export");
    let mut sources: Vec<(String, Source)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".source.svg"))
        .map(|p| {
            let name = p
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".source.svg", "");
            (name, Source::Svg(p))
        })
        .collect();
    sources.push((
        "export_sketch".to_owned(),
        Source::Project(sketch_project()),
    ));
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(sources.len() >= 14, "export documents: {}", sources.len());
    for (name, source) in sources {
        let options_text =
            std::fs::read_to_string(dir.join(format!("{name}.options"))).unwrap_or_default();
        let options = SvgExportOptions {
            text_as_paths: options_text.contains("text_as_paths"),
            strokes_as_paths: options_text.contains("strokes_as_paths"),
            selection_only: options_text.contains("selection_only"),
            active_page_only: options_text.contains("active_page_only"),
        };
        let mut s = source.open();
        golden(
            &dir.join(format!("{name}.document.json")),
            &document_json(&s),
        );
        if options.selection_only {
            select_the_selected_layer(&mut s);
        }
        let first = export(&s, options);
        golden(&dir.join(format!("{name}.exported.svg")), &first);
        let mut back = open(first.as_bytes());
        if options.selection_only {
            select_the_selected_layer(&mut back);
        }
        let again = export(&back, options);
        let line = again
            .lines()
            .zip(first.lines())
            .find(|(a, b)| a != b)
            .map(|(a, b)| {
                let at = a
                    .bytes()
                    .zip(b.bytes())
                    .position(|(x, y)| x != y)
                    .unwrap_or(0);
                let from = at.saturating_sub(120);
                format!(
                    "{}\n{}",
                    &a[from..(at + 80).min(a.len())],
                    &b[from..(at + 80).min(b.len())]
                )
            });
        assert!(
            again == first,
            "{name}: the second export differs:\n{line:?}"
        );
    }
}
