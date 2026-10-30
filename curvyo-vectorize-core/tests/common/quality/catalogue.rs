//! The catalogue of the synthetic reference set (`specs/0089-trace-quality-and-colour` criteria
//! 1, 2): per fixture its group, intended use, covered cases, what must be preserved, resolution.

use super::super::Picture;
use super::reference::{Feature, Group, Reference, Use};
use super::scenes::{
    clipart_badge, geo_corners, geo_round, line_angles, line_hatching, line_strokes_open,
    line_webs, logo_jpeg, logo_pale, logo_shapes, logo_transparent,
};
use super::scenes_colour::{
    logo_blue, logo_fringe, logo_pale_picked, logo_picked, logo_transparent_colour,
};
use super::scenes_more::{
    ai_hatching, ai_portrait, ai_symbol, diagram, photo_combined, photo_lighting, photo_shadow,
    photo_street, scan_jpeg, scan_noise, scan_paper, sketch_bowed, sketch_stippling,
    sketch_strokes,
};
use super::text::text_umlauts;
use curvyo_vectorize_core::{
    DEFAULT_BACKGROUND_TOLERANCE, DEFAULT_PICK_TOLERANCE, Rgb8, Selection, Swatches,
    suggest_background,
};

/// The synthetic reference set, in report order.
pub fn reference_set() -> Vec<Reference> {
    vec![
        Reference {
            name: "geo-corners",
            group: Group::Graphics,
            intended_use: Use::CutContour,
            covers: &[Feature::SharpCorners],
            preserve: "every vertex of the four squares (0, 15, 30, 45 degrees), the 30 degree \
                       triangle and the 36 degree star tips as a sharp node; straight sides",
            dpi: 96.0,
            build: geo_corners,
        },
        Reference {
            name: "geo-round",
            group: Group::Graphics,
            intended_use: Use::CutContour,
            covers: &[Feature::RoundCorners, Feature::Circles],
            preserve: "the 10 px rounded corners and the circles round (no corner on them), \
                       the ring's hole, the 5 px disc",
            dpi: 96.0,
            build: geo_round,
        },
        Reference {
            name: "clipart-badge",
            group: Group::Graphics,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::GreySeams],
            preserve: "ring, star with its round hole; the grey seam is not content",
            dpi: 96.0,
            build: clipart_badge,
        },
        Reference {
            name: "logo-shapes",
            group: Group::Logos,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::SharpCorners, Feature::Circles],
            preserve: "ring with hole, star tips sharp, the bar's straight sides and corners",
            dpi: 96.0,
            build: logo_shapes,
        },
        Reference {
            name: "logo-pale",
            group: Group::Logos,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::PaleColours],
            preserve: "all three pale shapes (#FFF2A8, #D9D9D9, #DCD6CF) on white",
            dpi: 96.0,
            build: logo_pale,
        },
        Reference {
            name: "logo-transparent",
            group: Group::Logos,
            intended_use: Use::CutContour,
            covers: &[Feature::Transparency],
            preserve: "the opaque ring and bar and the 50 % alpha star; the transparent \
                       background is never content",
            dpi: 96.0,
            build: logo_transparent,
        },
        Reference {
            name: "logo-jpeg",
            group: Group::Logos,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::Jpeg50],
            preserve: "as logo-shapes; JPEG ringing is not content",
            dpi: 96.0,
            build: logo_jpeg,
        },
        Reference {
            name: "line-angles",
            group: Group::LineDrawings,
            intended_use: Use::CutContour,
            covers: &[
                Feature::Lines0,
                Feature::Lines7,
                Feature::Lines15,
                Feature::Lines30,
                Feature::Lines45,
            ],
            preserve: "12 px bars and 3 px lines at 0, 7, 15, 30 and 45 degrees: straight long \
                       sides without staircase, square ends",
            dpi: 150.0,
            build: line_angles,
        },
        Reference {
            name: "line-webs",
            group: Group::LineDrawings,
            intended_use: Use::CutContour,
            covers: &[Feature::Web1px, Feature::Web2px, Feature::Parallels1px],
            preserve: "the 1 px and 2 px webs keep their blocks connected; bars 1 px and 2 px \
                       apart stay separate",
            dpi: 300.0,
            build: line_webs,
        },
        Reference {
            name: "line-hatching",
            group: Group::LineDrawings,
            intended_use: Use::LinePlot,
            covers: &[Feature::Hatching],
            preserve: "every 2 px hatch line separate and straight, the ring closed",
            dpi: 300.0,
            build: line_hatching,
        },
        Reference {
            name: "line-strokes-open",
            group: Group::LineDrawings,
            intended_use: Use::LinePlot,
            covers: &[Feature::OpenStrokes, Feature::NearlyClosedStrokes],
            preserve: "open arcs stay open, the 1 px and 3 px gaps of the nearly closed rings \
                       stay open (no hole), arcs smooth",
            dpi: 150.0,
            build: line_strokes_open,
        },
        Reference {
            name: "text-umlauts",
            group: Group::Logos,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::SmallTextUmlauts],
            preserve: "every letter, the counters of O, Ö, a, o, the two dots of each umlaut \
                       as separate shapes, ß as one shape",
            dpi: 150.0,
            build: text_umlauts,
        },
        Reference {
            name: "sketch-bowed",
            group: Group::Sketches,
            intended_use: Use::CutContour,
            covers: &[Feature::RoundCorners],
            preserve: "the 2 px bow over 200 px stays bowed (not straightened, no corner); the \
                       wobbly loop stays smooth; the four square corners stay sharp",
            dpi: 150.0,
            build: sketch_bowed,
        },
        Reference {
            name: "sketch-strokes",
            group: Group::Sketches,
            intended_use: Use::LinePlot,
            covers: &[Feature::OpenStrokes],
            preserve: "doubled box strokes and tapered curves, thin ends not cut off",
            dpi: 150.0,
            build: sketch_strokes,
        },
        Reference {
            name: "sketch-stippling",
            group: Group::Sketches,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::Stippling],
            preserve: "every dot as its own shape, none merged, the ring closed",
            dpi: 300.0,
            build: sketch_stippling,
        },
        Reference {
            name: "ai-portrait",
            group: Group::AiDrawings,
            intended_use: Use::AreaEngraving,
            covers: &[],
            preserve: "the white strands through the black hood, eyes and mouth as shapes",
            dpi: 150.0,
            build: ai_portrait,
        },
        Reference {
            name: "ai-hatching",
            group: Group::AiDrawings,
            intended_use: Use::AreaEngraving,
            covers: &[Feature::Hatching],
            preserve: "every engraving line separate, thin ends kept",
            dpi: 150.0,
            build: ai_hatching,
        },
        Reference {
            name: "ai-symbol",
            group: Group::AiDrawings,
            intended_use: Use::CutContour,
            covers: &[Feature::SharpCorners],
            preserve: "three thin rings separate, triangle frame with its hole, eye hole, sharp \
                       triangle corners",
            dpi: 150.0,
            build: ai_symbol,
        },
        Reference {
            name: "scan-noise",
            group: Group::PaperScan,
            intended_use: Use::LinePlot,
            covers: &[Feature::ScanNoise],
            preserve: "frame, box, rings, slanted line and arrow; noise and dust are not content",
            dpi: 300.0,
            build: scan_noise,
        },
        Reference {
            name: "scan-paper",
            group: Group::PaperScan,
            intended_use: Use::LinePlot,
            covers: &[Feature::PaperTexture],
            preserve: "as scan-noise; paper fibres are not content",
            dpi: 300.0,
            build: scan_paper,
        },
        Reference {
            name: "scan-jpeg",
            group: Group::PaperScan,
            intended_use: Use::LinePlot,
            covers: &[Feature::Jpeg50],
            preserve: "as scan-noise; JPEG blocks and ringing are not content",
            dpi: 300.0,
            build: scan_jpeg,
        },
        Reference {
            name: "photo-shadow",
            group: Group::PaperPhoto,
            intended_use: Use::LinePlot,
            covers: &[Feature::Shadow],
            preserve: "the drawing also inside the shadow; the shadow is not content",
            dpi: 150.0,
            build: photo_shadow,
        },
        Reference {
            name: "photo-lighting",
            group: Group::PaperPhoto,
            intended_use: Use::LinePlot,
            covers: &[Feature::LightingGradient],
            preserve: "the drawing in the dark corner; the gradient is not content",
            dpi: 150.0,
            build: photo_lighting,
        },
        Reference {
            name: "photo-combined",
            group: Group::PaperPhoto,
            intended_use: Use::LinePlot,
            covers: &[Feature::Shadow, Feature::LightingGradient, Feature::Jpeg50],
            preserve: "the drawing under texture, shadow, light fall-off, noise and JPEG",
            dpi: 150.0,
            build: photo_combined,
        },
        Reference {
            name: "diagram-boxes",
            group: Group::Diagram,
            intended_use: Use::LinePlot,
            covers: &[Feature::SharpCorners],
            preserve: "box frames with their holes, arrow heads sharp, the diamond and the node",
            dpi: 96.0,
            build: diagram,
        },
        Reference {
            name: "photo-street",
            group: Group::Photo,
            intended_use: Use::AreaEngraving,
            covers: &[],
            preserve: "no truth: the large dark shapes (buildings, trees, figure)",
            dpi: 96.0,
            build: photo_street,
        },
    ]
}

/// How a fixture of the selection methods is selected (`specs/0089` slice C), at the method's
/// default colour tolerance.
#[derive(Debug, Clone, Copy)]
pub enum Method {
    /// Remove background with Auto: the suggested background colour (none on a transparent
    /// border).
    RemoveBackground,
    /// Pick colours with these colours picked.
    Pick(&'static [[u8; 3]]),
}

impl Method {
    /// The selection of `picture` as the bar would hand it to the tracer.
    pub fn selection(self, picture: &Picture) -> Selection {
        match self {
            Self::RemoveBackground => Selection::RemoveBackground {
                background: suggest_background(picture.raster()),
                tolerance: DEFAULT_BACKGROUND_TOLERANCE,
            },
            Self::Pick(colours) => {
                let colours: Vec<Rgb8> = colours
                    .iter()
                    .map(|&[r, g, b]| Rgb8::new(r, g, b))
                    .collect();
                Selection::PickColours {
                    swatches: Swatches::from_slice(&colours),
                    background: suggest_background(picture.raster()),
                    tolerance: DEFAULT_PICK_TOLERANCE,
                }
            }
        }
    }
}

/// A fixture of the selection methods with its method.
pub struct Selected {
    pub reference: Reference,
    pub method: Method,
}

/// The fixtures of the selection methods (`specs/0089` slice C, criteria 15 to 19), measured in
/// the reference table with their method.
pub fn selection_set() -> Vec<Selected> {
    vec![
        Selected {
            reference: Reference {
                name: "logo-blue",
                group: Group::Logos,
                intended_use: Use::AreaEngraving,
                covers: &[Feature::PaleColours],
                preserve: "white ring, dark bar and the pale #FFF2A8 panel on #3366CC paper",
                dpi: 96.0,
                build: logo_blue,
            },
            method: Method::RemoveBackground,
        },
        Selected {
            reference: Reference {
                name: "logo-transparent-colour",
                group: Group::Logos,
                intended_use: Use::CutContour,
                covers: &[Feature::Transparency],
                preserve: "blue ring, pale star and black bar; the transparent background is \
                           never content",
                dpi: 96.0,
                build: logo_transparent_colour,
            },
            method: Method::RemoveBackground,
        },
        Selected {
            reference: Reference {
                name: "logo-pale-picked",
                group: Group::Logos,
                intended_use: Use::AreaEngraving,
                covers: &[Feature::PaleColours],
                preserve: "the #FFF2A8 rectangle and the #D9D9D9 disc picked; the #DCD6CF star \
                           stays out",
                dpi: 96.0,
                build: logo_pale_picked,
            },
            method: Method::Pick(&[[0xFF, 0xF2, 0xA8], [0xD9, 0xD9, 0xD9]]),
        },
        Selected {
            reference: Reference {
                name: "logo-fringe",
                group: Group::Logos,
                intended_use: Use::CutContour,
                covers: &[Feature::GreySeams],
                preserve: "the red star picked; its grey fringe is not content",
                dpi: 96.0,
                build: logo_fringe,
            },
            method: Method::Pick(&[[0xCC, 0x22, 0x22]]),
        },
        Selected {
            reference: Reference {
                name: "logo-picked",
                group: Group::Logos,
                intended_use: Use::AreaEngraving,
                covers: &[Feature::SharpCorners],
                preserve: "the touching yellow and red panels as one shape; the blue disc stays \
                           out",
                dpi: 96.0,
                build: logo_picked,
            },
            method: Method::Pick(&[[0xF2, 0xC4, 0x00], [0xD0, 0x20, 0x20]]),
        },
    ]
}
