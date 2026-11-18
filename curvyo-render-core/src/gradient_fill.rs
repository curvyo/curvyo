//! The gradient data of the artwork (ADR 0067 §1, `specs/0124-gradient-fill` criteria 9 to 11):
//! the spans of gradient-filled vertices with their gradient coordinates, the stop table every
//! span reads, and the texture rows the host uploads it as.
//!
//! A gradient leaf is tessellated exactly as its Colour fill; its vertices are white with the
//! leaf's shown alpha, and the host's fragment shader takes the colour from the stop table. The
//! coordinates are computed here per vertex (`u, v` of ADR 0066 §4): an affine map commutes with
//! linear interpolation, so the per-pixel coordinate the GPU interpolates is exact, and a later
//! move of the vertices (a placement, [`DrawList::map_positions`]) leaves the shade where it was.

use std::collections::HashMap;
use std::ops::Range;

use curvyo_document_core::{
    GradientStops, ObjectSnapshot, PlacedGradient, PremultipliedRgba8, over_white_grey,
};

use crate::glyphs::DrawList;

/// The width of the stop texture in texels: the smallest maximum texture size WebGL2 guarantees,
/// so every adapter takes it; rows are added as stops need them (ADR 0067 §2).
pub const STOP_TEXTURE_WIDTH: u32 = 2048;

/// One stop of the table: its offset and its colour premultiplied by its alpha, each channel
/// in `0..=1`. The host stores it as two texels (offset, then colour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopTexel {
    /// Where the stop sits, `0..=1`.
    pub offset: f32,
    /// Red, green and blue times alpha, then alpha.
    pub colour: [f32; 4],
}

impl StopTexel {
    /// The bits that identify the entry, for sharing one table range between equal lists.
    fn key(self) -> [u32; 5] {
        [
            self.offset.to_bits(),
            self.colour[0].to_bits(),
            self.colour[1].to_bits(),
            self.colour[2].to_bits(),
            self.colour[3].to_bits(),
        ]
    }
}

/// The table entries of `stops`, each premultiplied; on a laser page each is its grey over
/// white, opaque (ADR 0067 §3), so interpolating the greys gives the grey of the shade.
#[must_use]
pub fn stop_texels(stops: &GradientStops, grey: bool) -> Vec<StopTexel> {
    stops
        .as_slice()
        .iter()
        .map(|stop| {
            #[allow(clippy::cast_possible_truncation)]
            let offset = stop.offset.get() as f32;
            if grey {
                let a = u16::from(stop.alpha);
                // invariant: a channel times an alpha over 255 is at most 255.
                #[allow(clippy::cast_possible_truncation)]
                let times = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
                let g = over_white_grey(PremultipliedRgba8 {
                    r: times(stop.colour.r),
                    g: times(stop.colour.g),
                    b: times(stop.colour.b),
                    a: stop.alpha,
                });
                let g = f32::from(g) / 255.0;
                return StopTexel {
                    offset,
                    colour: [g, g, g, 1.0],
                };
            }
            // Premultiplied without rounding, as `colour_at` blends on the wide scale.
            let a = f32::from(stop.alpha);
            let times = |c: u8| f32::from(c) * a / 65_025.0;
            StopTexel {
                offset,
                colour: [
                    times(stop.colour.r),
                    times(stop.colour.g),
                    times(stop.colour.b),
                    a / 255.0,
                ],
            }
        })
        .collect()
}

/// One gradient-filled tessellation in the list: its vertices, their gradient coordinates and
/// the range of the stop table it reads.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientSpan {
    vertices: Range<usize>,
    coords: Vec<[f32; 2]>,
    radial: bool,
    stops: Range<u32>,
}

impl GradientSpan {
    /// The vertices of the list this span shades.
    #[must_use]
    pub fn vertices(&self) -> Range<usize> {
        self.vertices.clone()
    }

    /// The stop table entries it reads.
    #[must_use]
    pub fn stops(&self) -> Range<u32> {
        self.stops.clone()
    }

    /// Whether `t` is the length of `(u, v)` (radial) rather than `u` (linear).
    #[must_use]
    pub const fn is_radial(&self) -> bool {
        self.radial
    }

    /// The span moved by `vertex_base` vertices and `stop_base` table entries (one list
    /// appended to another).
    pub(crate) fn shifted(mut self, vertex_base: usize, stop_base: u32) -> Self {
        self.vertices = self.vertices.start + vertex_base..self.vertices.end + vertex_base;
        self.stops = self.stops.start + stop_base..self.stops.end + stop_base;
        self
    }
}

/// The stop lists already in a list's table during one artwork build: equal lists share one
/// range, so a thousand shapes of one gradient upload four entries.
#[derive(Debug, Default)]
pub(crate) struct StopRanges {
    by_content: HashMap<Vec<[u32; 5]>, Range<u32>>,
}

/// The texture the host uploads the stop table as: `width` texels per row, two texels per stop
/// (offset, colour), rows as needed, at least one.
#[derive(Debug, Clone, PartialEq)]
pub struct StopTexture {
    /// Texels per row.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// `width * height` texels, row by row; unused texels are zero.
    pub texels: Vec<[f32; 4]>,
}

/// The gradient a leaf paints, placed in its box (ADR 0066 §4); `None` when it paints none or
/// has no box (a text not laid out yet, a clone of a picture).
#[must_use]
pub fn placed_gradient(object: &ObjectSnapshot) -> Option<PlacedGradient> {
    let gradient = object.style().fill.painted_gradient()?;
    Some(PlacedGradient::new(gradient, &object.gradient_box()?))
}

impl DrawList {
    /// Appends `layer` (a fill tessellated in white) as a new artwork layer shaded by `placed`:
    /// every vertex gets its gradient coordinates and the span names the stop entries.
    pub(crate) fn extend_gradient_artwork(
        &mut self,
        mut layer: Self,
        placed: &PlacedGradient,
        grey: bool,
        ranges: &mut StopRanges,
    ) {
        if layer.triangles.is_empty() {
            return;
        }
        let to_gradient = placed.to_gradient();
        #[allow(clippy::cast_possible_truncation)]
        let coords = layer
            .triangles
            .iter()
            .map(|vertex| {
                let q = to_gradient.apply_point(vertex.position);
                [q.x as f32, q.y as f32]
            })
            .collect();
        let stops = self.stop_range(stop_texels(placed.stops(), grey), ranges);
        let start = self.triangles.len();
        self.triangles.append(&mut layer.triangles);
        self.close_layer();
        self.gradients.push(GradientSpan {
            vertices: start..self.triangles.len(),
            coords,
            radial: placed.is_radial(),
            stops,
        });
    }

    /// The table range holding `texels`, appended unless an equal list is already there.
    fn stop_range(&mut self, texels: Vec<StopTexel>, ranges: &mut StopRanges) -> Range<u32> {
        let key: Vec<[u32; 5]> = texels.iter().map(|t| t.key()).collect();
        if let Some(range) = ranges.by_content.get(&key) {
            return range.clone();
        }
        let start = self.table_len();
        self.gradient_stops.extend(texels);
        let range = start..self.table_len();
        ranges.by_content.insert(key, range.clone());
        range
    }

    fn table_len(&self) -> u32 {
        // invariant: a table of more than `u32::MAX` stops cannot be allocated.
        u32::try_from(self.gradient_stops.len()).unwrap_or(u32::MAX)
    }

    /// The gradient spans, in vertex order.
    #[must_use]
    pub fn gradients(&self) -> &[GradientSpan] {
        &self.gradients
    }

    /// The stop table every span reads.
    #[must_use]
    pub fn gradient_stops(&self) -> &[StopTexel] {
        &self.gradient_stops
    }

    /// Per vertex `(u, v, first stop, signed count)` (ADR 0067 §1): zero for a solid vertex; the
    /// count is negative for a radial span. The host uploads it as vertex buffer 1, so it holds
    /// no gradient logic of its own.
    #[must_use]
    pub fn gradient_attributes(&self) -> Vec<[f32; 4]> {
        let mut attributes = vec![[0.0; 4]; self.triangles.len()];
        for span in &self.gradients {
            #[allow(clippy::cast_precision_loss)]
            let (first, count) = (
                span.stops.start as f32,
                (span.stops.end - span.stops.start) as f32,
            );
            let count = if span.radial { -count } else { count };
            for (slot, uv) in attributes[span.vertices.clone()]
                .iter_mut()
                .zip(&span.coords)
            {
                *slot = [uv[0], uv[1], first, count];
            }
        }
        attributes
    }

    /// The stop table as texture rows of [`STOP_TEXTURE_WIDTH`] texels: stop `i` at texels
    /// `2i` (offset in red) and `2i + 1` (premultiplied colour).
    #[must_use]
    pub fn stop_texture(&self) -> StopTexture {
        self.stop_texture_within(u32::MAX)
    }

    /// [`DrawList::stop_texture`] cut to at most `max_rows` rows (the device's largest 2D
    /// texture, `docs/technical-debt.md`): the stops past the cut are left out, and a span that
    /// reads them shades with zeros (transparent) instead of failing the frame.
    #[must_use]
    pub fn stop_texture_within(&self, max_rows: u32) -> StopTexture {
        let width = STOP_TEXTURE_WIDTH as usize;
        let used = 2 * self.gradient_stops.len();
        // invariant: a `u32` fits in a `usize` on every target Curvyo builds for.
        let limit = usize::try_from(max_rows.max(1)).unwrap_or(usize::MAX);
        let rows = used.div_ceil(width).clamp(1, limit);
        let mut texels = Vec::with_capacity(rows * width);
        for stop in self.gradient_stops.iter().take(rows * width / 2) {
            texels.push([stop.offset, 0.0, 0.0, 0.0]);
            texels.push(stop.colour);
        }
        texels.resize(rows * width, [0.0; 4]);
        StopTexture {
            width: STOP_TEXTURE_WIDTH,
            // invariant: `rows` is at most `max_rows`.
            height: u32::try_from(rows).unwrap_or(u32::MAX),
            texels,
        }
    }

    /// Whether any artwork of this list is shaded by a gradient.
    #[must_use]
    pub fn has_gradients(&self) -> bool {
        !self.gradients.is_empty()
    }
}

#[cfg(test)]
pub(crate) mod shader_reference;

#[cfg(test)]
mod tests;
