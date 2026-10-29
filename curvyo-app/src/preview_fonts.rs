//! The faces the import dialog's SVG preview draws texts in (`specs/0087-image-clipboard-and-drop`
//! criterion 26): the font sources of `0074` as Import SVG matches them (Curvyo fonts, Included,
//! This computer), listed once per dialog when the first file with text needs them, and each
//! face's bytes read once, the first time a file asks for that face.

use std::path::PathBuf;

use curvyo_svg_core::{SvgDrawing, SvgNode};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{FontBlock, ImportFaces, ListedFace, substitute_face, svg_faces_needed};

/// The most faces one file loads; its other texts are drawn in the substitute, so a file cannot
/// fill the store with hundreds of installed faces.
pub(crate) const MAX_FACES_PER_FILE: usize = 16;

/// Reads one file of the included fonts by its path below `fonts/` ("index.json",
/// "source-sans-3/SourceSans3-Regular.otf").
pub(crate) type IncludedReader = Box<dyn Fn(&str) -> Option<Vec<u8>> + Send>;

/// Where the faces come from.
pub(crate) struct FontSources {
    /// The Curvyo font folder, when the data folder is known.
    pub(crate) curvyo_dir: Option<PathBuf>,
    /// The included fonts, as the page loads them.
    pub(crate) included: IncludedReader,
    /// Whether the installed fonts are listed (always in the app; off in tests).
    pub(crate) system: bool,
}

/// Where the bytes of a listed face are.
enum FaceFile {
    Path(PathBuf),
    Included(String),
}

/// The listing and the faces read so far, for the life of one dialog.
pub(crate) struct PreviewFonts {
    sources: FontSources,
    /// The listing in block order, made when the first file with text is previewed.
    listed: Option<(Vec<ListedFace>, Vec<FaceFile>)>,
    /// Listed faces whose bytes could not be read or used: drawn in the substitute.
    failed: Vec<usize>,
    store: FaceStore,
}

/// The `index.json` of the included fonts: what the page lists as the Included block.
#[derive(serde::Deserialize)]
struct IncludedIndex {
    faces: Vec<IncludedFace>,
}

#[derive(serde::Deserialize)]
struct IncludedFace {
    family: String,
    style: String,
    file: String,
    #[serde(default)]
    kind: Option<String>,
}

impl PreviewFonts {
    pub(crate) fn new(sources: FontSources) -> Self {
        Self {
            sources,
            listed: None,
            failed: Vec::new(),
            store: FaceStore::new(),
        }
    }

    /// No font source at all: every text is left out.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self::new(FontSources {
            curvyo_dir: None,
            included: Box::new(|_| None),
            system: false,
        })
    }

    /// The faces the texts of `drawing` are drawn in, the ones it needs read first; the listing is
    /// made on the first call for a drawing with text.
    pub(crate) fn faces_for(&mut self, drawing: &SvgDrawing) -> ImportFaces<'_> {
        if self.listed.is_none() && has_text(&drawing.nodes) {
            self.listed = Some(list(&self.sources));
        }
        let Some((faces, files)) = &mut self.listed else {
            return ImportFaces {
                faces: &[],
                store: &self.store,
                substitute: None,
            };
        };
        let needs = svg_faces_needed(drawing, faces);
        let mut wanted = needs
            .load
            .into_iter()
            .filter(|at| !self.failed.contains(at))
            .collect::<Vec<_>>();
        let over_limit = wanted.len() > MAX_FACES_PER_FILE;
        wanted.truncate(MAX_FACES_PER_FILE);
        let substitute = substitute_face(faces);
        let any_failed = !self.failed.is_empty();
        if let Some(at) = substitute
            && (needs.substitute || any_failed || over_limit)
            && faces[at].key.is_none()
            && !self.failed.contains(&at)
            && !wanted.contains(&at)
        {
            wanted.push(at);
        }
        for at in wanted {
            let bytes = match &files[at] {
                FaceFile::Path(path) => read_font_file(path),
                FaceFile::Included(file) => (self.sources.included)(file),
            };
            match bytes.and_then(|bytes| self.store.add(bytes, faces[at].index).ok()) {
                Some(key) => faces[at].key = Some(key),
                None => self.failed.push(at),
            }
        }
        ImportFaces {
            faces,
            store: &self.store,
            substitute: substitute.and_then(|at| faces[at].key),
        }
    }
}

/// The bytes of a font file, refused over the engine's limit before it is read (ADR 0020 §2).
fn read_font_file(path: &std::path::Path) -> Option<Vec<u8>> {
    let length = std::fs::metadata(path).ok()?.len();
    if length > curvyo_storage_io::MAX_FONT_FILE_BYTES {
        return None;
    }
    curvyo_storage_io::read_to_vec(path).ok()
}

fn has_text(nodes: &[SvgNode]) -> bool {
    nodes.iter().any(|node| match node {
        SvgNode::Group { children, .. } => has_text(children),
        SvgNode::Text { .. } => true,
        SvgNode::Leaf { .. } | SvgNode::Image { .. } => false,
    })
}

/// The three blocks of `0074` in order, as the page lists them.
fn list(sources: &FontSources) -> (Vec<ListedFace>, Vec<FaceFile>) {
    let mut faces = Vec::new();
    let mut files = Vec::new();
    let mut push_listed = |block, listed: Vec<curvyo_storage_io::FontFace>| {
        for face in listed {
            faces.push(listed_face(
                block,
                face.family,
                face.style,
                face.index,
                face.single_line,
            ));
            files.push(FaceFile::Path(face.path));
        }
    };
    if let Some(dir) = &sources.curvyo_dir {
        push_listed(
            FontBlock::Curvyo,
            curvyo_storage_io::list_font_folder(dir).faces,
        );
    }
    let index = (sources.included)("index.json")
        .and_then(|bytes| serde_json::from_slice::<IncludedIndex>(&bytes).ok());
    let included = index.map(|index| index.faces).unwrap_or_default();
    if sources.system {
        push_listed(FontBlock::System, curvyo_storage_io::list_system_fonts());
    }
    for face in included {
        faces.push(listed_face(
            FontBlock::Included,
            face.family,
            face.style,
            0,
            face.kind.as_deref() == Some("single_line"),
        ));
        files.push(FaceFile::Included(face.file));
    }
    (faces, files)
}

fn listed_face(
    block: FontBlock,
    family: String,
    style: String,
    index: u32,
    single_line: bool,
) -> ListedFace {
    ListedFace {
        block,
        family,
        style,
        host_id: String::new(),
        index,
        key: None,
        kind: if single_line {
            curvyo_document_core::FontKind::SingleLine
        } else {
            curvyo_document_core::FontKind::Outline
        },
    }
}
