//! The Trace tool's multicolour side (`specs/0089-trace-quality-and-colour` slice M, criteria
//! 29 and UX notes §7): the palette of the shown preview for the strip, the maker's recolours
//! (fills only, no refit) and Isolate (the other colours dimmed in every view).

use curvyo_document_core::{Color, NodeId, Opacity, PathSnapshot};
use curvyo_render_core::PreviewAnchor;
use curvyo_ui_core::{TraceColour, TraceLayerWire, TraceRgb};

use super::Session;
use super::trace_draw::filled_result;
use super::trace_mesh::MeshCache;

/// How strongly the colours Isolate does not pick are drawn (UX notes §7: 35 %).
pub(super) const DIMMED: f64 = 0.35;

/// The maker's view of the palette: recolours (palette colour to fill) and the isolated colour.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PaletteView {
    pub(super) recolours: Vec<(TraceRgb, TraceRgb)>,
    pub(super) isolated: Option<TraceRgb>,
}

impl PaletteView {
    /// The fill of the palette colour `colour`: its recolour, or itself.
    pub(super) fn fill(&self, colour: TraceRgb) -> TraceRgb {
        self.recolours
            .iter()
            .find(|(from, _)| *from == colour)
            .map_or(colour, |&(_, to)| to)
    }

    /// Whether `colour` is drawn dimmed: another colour is isolated.
    pub(super) fn dimmed(&self, colour: TraceRgb) -> bool {
        self.isolated.is_some_and(|isolated| isolated != colour)
    }

    /// The layers of a result as Create paths writes them: each recoloured fill and its count.
    pub(super) fn colours(&self, layers: &[TraceLayerWire]) -> Vec<TraceColour> {
        layers
            .iter()
            .map(|layer| TraceColour {
                fill: color(self.fill(layer.colour)),
                outlines: layer.outlines,
            })
            .collect()
    }
}

/// A trace colour as a document colour.
pub(super) fn color(c: TraceRgb) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
    }
}

/// The layer of every outline (`None` for a single-colour result).
pub(super) fn outline_layers(layers: &[TraceLayerWire], outlines: usize) -> Vec<Option<usize>> {
    if layers.is_empty() {
        return vec![None; outlines];
    }
    layers
        .iter()
        .enumerate()
        .flat_map(|(k, layer)| std::iter::repeat_n(Some(k), layer.outlines))
        .collect()
}

/// The preview of `image` filled as Create paths would write it (criterion 51 of 0025, slice M
/// criterion 29): one black compound path, or one per layer in its fill, the colours Isolate
/// leaves out at [`DIMMED`].
pub(super) fn filled_layers(
    image: NodeId,
    outlines: &[Vec<PreviewAnchor>],
    layers: &[TraceLayerWire],
    view: &PaletteView,
) -> Vec<PathSnapshot> {
    if layers.is_empty() {
        return vec![filled_result(image, outlines)];
    }
    let mut at = 0;
    layers
        .iter()
        .map(|layer| {
            let to = (at + layer.outlines).min(outlines.len());
            let mut path = filled_result(image, &outlines[at..to]);
            at = to;
            path.style.fill.color = color(view.fill(layer.colour));
            if view.dimmed(layer.colour) {
                path.style.fill.opacity = Opacity::new(DIMMED).unwrap_or(Opacity::OPAQUE);
            }
            path
        })
        .collect()
}

impl Session {
    /// The palette of the shown preview as JSON (`specs/0089` UX notes §7): `{"colours": [{hex,
    /// fixed, pixels}], "layers": [hex], "joined": n}`; empty for a single-colour preview or none.
    #[must_use]
    pub fn trace_palette_json(&self) -> String {
        let shown = self.trace_view().shown;
        let Some(preview) = self
            .trace
            .preview
            .as_ref()
            .filter(|p| Some(p.image) == shown && !p.palette.is_empty())
        else {
            return String::new();
        };
        let hex = |c: TraceRgb| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b);
        serde_json::json!({
            "colours": preview.palette.iter().map(|e| serde_json::json!({
                "hex": hex(e.colour),
                "fixed": e.fixed,
                "pixels": e.pixels,
            })).collect::<Vec<_>>(),
            "layers": preview.layers.iter().map(|l| hex(l.colour)).collect::<Vec<_>>(),
            "joined": preview.joined,
        })
        .to_string()
    }

    /// Sets the recolours (palette colour to fill) and the isolated colour: the preview's fills,
    /// outline strengths and the Mask view follow at once, nothing is traced again (criterion 29).
    pub fn set_trace_palette_view(
        &mut self,
        recolours: Vec<(TraceRgb, TraceRgb)>,
        isolated: Option<TraceRgb>,
    ) {
        let view = PaletteView {
            recolours,
            isolated,
        };
        if view == self.trace.palette_view {
            return;
        }
        self.trace.palette_view = view;
        if let Some(preview) = &mut self.trace.preview {
            preview.fills = filled_layers(
                preview.image,
                &preview.outlines,
                &preview.layers,
                &self.trace.palette_view,
            );
            preview.fill_mesh = MeshCache::default();
            preview.outline_mesh = MeshCache::default();
        }
        self.refresh_trace_mask();
    }
}
