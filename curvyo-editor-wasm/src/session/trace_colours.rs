//! The Trace tool's multicolour side (`specs/0089-trace-quality-and-colour` slice M, criteria
//! 29 and UX notes §7): the palette of the shown preview for the strip, the maker's recolours
//! (fills only, no refit) and Isolate (the other colours dimmed in every view).

use curvyo_document_core::Color;
use curvyo_ui_core::{TraceColour, TraceLayerWire, TraceRgb};

use super::Session;
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
}

/// A trace colour as a document colour.
pub(super) fn color(c: TraceRgb) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
    }
}

/// The fill layer of `outlines` paths of a text or text line of a multicolour result
/// (`specs/0091` slice 2): its fill, black without one.
pub(super) fn text_layer(fill: Option<TraceRgb>, outlines: usize) -> TraceLayerWire {
    TraceLayerWire {
        colour: fill.unwrap_or(TraceRgb::new(0, 0, 0)),
        outlines,
    }
}

/// The outlines and colours Create paths writes for a result of `outlines` in `layers` with the
/// `leftovers` of its text lines (`specs/0091` decision 6): one colour, all in the black path;
/// multicolour, each leftover in the layer of its fill (recoloured as the layer), or in a path of
/// its fill after the layers when no layer has it. The third value is where each of `outlines`
/// went (`specs/0088` Part B: the recognised shapes follow their outlines).
pub(super) fn with_leftovers(
    outlines: &[kurbo::BezPath],
    layers: &[TraceLayerWire],
    leftovers: &[(kurbo::BezPath, TraceRgb)],
    view: &PaletteView,
) -> (Vec<kurbo::BezPath>, Vec<TraceColour>, Vec<usize>) {
    if layers.is_empty() {
        let mut all = outlines.to_vec();
        all.extend(leftovers.iter().map(|(p, _)| p.clone()));
        return (all, Vec::new(), (0..outlines.len()).collect());
    }
    let mut out = Vec::with_capacity(outlines.len() + leftovers.len());
    let mut places = Vec::with_capacity(outlines.len());
    let mut colours = Vec::with_capacity(layers.len());
    let mut at = 0;
    for layer in layers {
        let to = (at + layer.outlines).min(outlines.len());
        places.extend(out.len()..out.len() + (to - at));
        out.extend_from_slice(&outlines[at..to]);
        at = to;
        let own = leftovers.iter().filter(|(_, f)| *f == layer.colour);
        let before = out.len();
        out.extend(own.map(|(p, _)| p.clone()));
        colours.push(TraceColour {
            fill: color(view.fill(layer.colour)),
            outlines: layer.outlines + out.len() - before,
        });
    }
    let mut rest: Vec<TraceRgb> = Vec::new();
    for (_, fill) in leftovers {
        if !layers.iter().any(|l| l.colour == *fill) && !rest.contains(fill) {
            rest.push(*fill);
        }
    }
    for fill in rest {
        let before = out.len();
        out.extend(
            leftovers
                .iter()
                .filter(|(_, f)| *f == fill)
                .map(|(p, _)| p.clone()),
        );
        colours.push(TraceColour {
            fill: color(view.fill(fill)),
            outlines: out.len() - before,
        });
    }
    (out, colours, places)
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

/// The colours the shown result fills, for its colour count: its layers' colours, then the
/// palette colours only its texts use (`specs/0091` slice 2), so the count and the strip agree.
fn used_colours(preview: &super::trace::TracePreview) -> Vec<TraceRgb> {
    let mut used: Vec<TraceRgb> = preview.layers.iter().map(|l| l.colour).collect();
    for layer in &preview.fill_layers {
        let in_palette = preview.palette.iter().any(|e| e.colour == layer.colour);
        if layer.outlines > 0 && in_palette && !used.contains(&layer.colour) {
            used.push(layer.colour);
        }
    }
    used
}

impl Session {
    /// The palette of the shown preview as JSON (`specs/0089` UX notes §7): `{"colours": [{hex,
    /// fixed, pixels}], "layers": [hex] (the colours it fills, texts included), "joined": n}`; empty for a single-colour preview or none.
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
            "layers": used_colours(preview).into_iter().map(hex).collect::<Vec<_>>(),
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
            preview.outline_mesh = MeshCache::default();
        }
    }
}
