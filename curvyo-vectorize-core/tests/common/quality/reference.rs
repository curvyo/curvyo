//! The fixture type of the slice A reference set (`specs/0089-trace-quality-and-colour` criteria
//! 1, 2): its metadata, its annotations, its drawing at one of the three resolutions.

use kurbo::{BezPath, Point};

use super::super::Picture;
use super::degrade::Artefact;
use super::render::Scene;

/// The material groups of the spec's target material (criterion 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Graphics,
    Logos,
    LineDrawings,
    Sketches,
    AiDrawings,
    PaperScan,
    PaperPhoto,
    Diagram,
    Photo,
}

impl Group {
    /// The primary groups, each needing at least 3 fixtures.
    pub const PRIMARY: [Self; 7] = [
        Self::Graphics,
        Self::Logos,
        Self::LineDrawings,
        Self::Sketches,
        Self::AiDrawings,
        Self::PaperScan,
        Self::PaperPhoto,
    ];

    pub const ALL: [Self; 9] = [
        Self::Graphics,
        Self::Logos,
        Self::LineDrawings,
        Self::Sketches,
        Self::AiDrawings,
        Self::PaperScan,
        Self::PaperPhoto,
        Self::Diagram,
        Self::Photo,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Graphics => "graphics",
            Self::Logos => "logos",
            Self::LineDrawings => "line-drawings",
            Self::Sketches => "sketches",
            Self::AiDrawings => "ai-drawings",
            Self::PaperScan => "paper-scan",
            Self::PaperPhoto => "paper-photo",
            Self::Diagram => "diagram",
            Self::Photo => "photo",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|g| g.key() == key)
    }
}

/// What the maker makes of the result (criterion 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    AreaEngraving,
    CutContour,
    LinePlot,
}

impl Use {
    pub fn key(self) -> &'static str {
        match self {
            Self::AreaEngraving => "area engraving",
            Self::CutContour => "cut contour",
            Self::LinePlot => "line plot",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::AreaEngraving, Self::CutContour, Self::LinePlot]
            .into_iter()
            .find(|u| u.key() == key)
    }
}

/// The cases criterion 2 lists; every one is covered by at least one synthetic fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    SharpCorners,
    RoundCorners,
    Lines0,
    Lines7,
    Lines15,
    Lines30,
    Lines45,
    Circles,
    Web1px,
    Web2px,
    Parallels1px,
    Hatching,
    Stippling,
    OpenStrokes,
    NearlyClosedStrokes,
    PaleColours,
    Transparency,
    GreySeams,
    Jpeg50,
    ScanNoise,
    PaperTexture,
    Shadow,
    LightingGradient,
    SmallTextUmlauts,
}

impl Feature {
    pub const ALL: [Self; 24] = [
        Self::SharpCorners,
        Self::RoundCorners,
        Self::Lines0,
        Self::Lines7,
        Self::Lines15,
        Self::Lines30,
        Self::Lines45,
        Self::Circles,
        Self::Web1px,
        Self::Web2px,
        Self::Parallels1px,
        Self::Hatching,
        Self::Stippling,
        Self::OpenStrokes,
        Self::NearlyClosedStrokes,
        Self::PaleColours,
        Self::Transparency,
        Self::GreySeams,
        Self::Jpeg50,
        Self::ScanNoise,
        Self::PaperTexture,
        Self::Shadow,
        Self::LightingGradient,
        Self::SmallTextUmlauts,
    ];
}

/// The three resolutions of criterion 2: the same drawing at 1x, 0.5x and 0.25x of its pixels,
/// placed at the same physical size.
pub const SCALES: [(&str, f64); 3] = [("1x", 1.0), ("0.5x", 0.5), ("0.25x", 0.25)];

/// An annotated edge of the truth: a straight edge (two points) or a smooth curve (a polyline
/// that must stay free of corners), in 1x pixels.
#[derive(Debug, Clone)]
pub struct Edge {
    pub points: Vec<Point>,
    pub straight: bool,
    /// The distance to the nearest other boundary in 1x pixels; the measuring window around the
    /// edge is half of it (at most 3 px, wider than the 2 px bow of criterion 7).
    pub clearance: f64,
}

/// What a synthetic fixture is made of.
pub enum Source {
    /// A drawn scene; its layers are the truth.
    Scene(Scene),
    /// A raster without truth (the synthetic photograph), reduced by area averaging.
    Raster(Picture),
}

/// A synthetic fixture's content, built on demand.
pub struct Built {
    pub source: Source,
    /// Applied in order after drawing.
    pub artefacts: Vec<Artefact>,
    /// Annotated sharp corners, 1x pixels.
    pub corners: Vec<Point>,
    pub edges: Vec<Edge>,
}

/// One synthetic fixture of the reference set.
pub struct Reference {
    pub name: &'static str,
    pub group: Group,
    pub intended_use: Use,
    pub covers: &'static [Feature],
    /// What the trace must keep (criterion 1's note).
    pub preserve: &'static str,
    /// The resolution at 1x; at 0.5x and 0.25x the same physical size has half and a quarter.
    pub dpi: f64,
    pub build: fn() -> Built,
}

/// A fixture drawn at one resolution, with everything in its own pixels.
pub struct Variant {
    /// `<fixture>@<resolution>`.
    pub key: String,
    pub picture: Picture,
    /// The truth at 4 x 4 samples per pixel (`None` for the photograph).
    pub truth_4x: Option<Vec<bool>>,
    /// The vector truth boundary (`None` when the scene has implicit layers or no truth).
    pub truth_paths: Option<Vec<BezPath>>,
    pub corners: Vec<Point>,
    pub edges: Vec<Edge>,
    pub mm_per_px: f64,
}

impl Reference {
    /// The fixture at `SCALES[index]`.
    pub fn variant(&self, index: usize) -> Variant {
        let (label, scale) = SCALES[index];
        let built = (self.build)();
        let (mut picture, truth_4x, truth_paths) = match &built.source {
            Source::Scene(scene) => {
                let drawn = scene.draw(scale);
                (
                    drawn.picture,
                    Some(drawn.truth_4x),
                    scene.truth_paths(scale),
                )
            }
            Source::Raster(picture) => {
                (reduce(picture, (1.0 / scale).round() as usize), None, None)
            }
        };
        for artefact in &built.artefacts {
            artefact.apply(&mut picture, scale);
        }
        let at = |p: &Point| Point::new(p.x * scale, p.y * scale);
        Variant {
            key: format!("{}@{label}", self.name),
            picture,
            truth_4x,
            truth_paths,
            corners: built.corners.iter().map(at).collect(),
            edges: built
                .edges
                .iter()
                .map(|e| Edge {
                    points: e.points.iter().map(at).collect(),
                    straight: e.straight,
                    clearance: e.clearance * scale,
                })
                .collect(),
            mm_per_px: 25.4 / (self.dpi * scale),
        }
    }
}

/// `picture` reduced by the whole `factor` with area averaging.
fn reduce(picture: &Picture, factor: usize) -> Picture {
    if factor <= 1 {
        return picture.clone();
    }
    let (w, h) = (picture.width as usize, picture.height as usize);
    let (rw, rh) = (w / factor, h / factor);
    let mut rgba = Vec::with_capacity(rw * rh * 4);
    for y in 0..rh {
        for x in 0..rw {
            for k in 0..4 {
                let mut sum = 0_u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        sum += u32::from(
                            picture.rgba[((y * factor + dy) * w + x * factor + dx) * 4 + k],
                        );
                    }
                }
                rgba.push(((sum + (factor * factor / 2) as u32) / (factor * factor) as u32) as u8);
            }
        }
    }
    Picture {
        width: rw as u32,
        height: rh as u32,
        rgba,
    }
}

/// The corners of a closed polygon as annotations.
pub fn corners_of(points: &[Point]) -> Vec<Point> {
    points.to_vec()
}

/// The sides of a closed polygon as straight edges with `clearance`.
pub fn sides_of(points: &[Point], clearance: f64) -> Vec<Edge> {
    (0..points.len())
        .map(|i| Edge {
            points: vec![points[i], points[(i + 1) % points.len()]],
            straight: true,
            clearance,
        })
        .collect()
}

/// A smooth curve annotation through `points`.
pub fn smooth(points: Vec<Point>, clearance: f64) -> Edge {
    Edge {
        points,
        straight: false,
        clearance,
    }
}
