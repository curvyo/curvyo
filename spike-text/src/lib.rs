//! Throwaway spike for ADR 0020: shape a string with harfrust and outline it with skrifa.
#![forbid(unsafe_code)]

#[cfg(feature = "engine")]
mod engine {
    use harfrust::{Buffer, ShapeOptions, ShaperFont};
    use skrifa::{
        FontRef, GlyphId, MetadataProvider,
        instance::{LocationRef, Size},
        outline::{DrawSettings, OutlinePen},
    };
    use std::sync::Arc;

    /// Spike error.
    #[derive(Debug)]
    pub enum Error {
        Parse,
        Shape,
    }

    /// Result of one shape+outline pass.
    #[derive(Debug, Default, Clone)]
    pub struct Shaped {
        pub glyph_ids: Vec<u32>,
        pub advances: Vec<i32>,
        /// Flattened path commands (one tag + coordinates), in mm-like scaled units.
        pub path: Vec<f32>,
        pub draw_errors: usize,
    }

    struct Pen<'a> {
        out: &'a mut Vec<f32>,
        dx: f32,
        dy: f32,
    }
    impl OutlinePen for Pen<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.out.extend_from_slice(&[0.0, x + self.dx, self.dy - y]);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.out.extend_from_slice(&[1.0, x + self.dx, self.dy - y]);
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.out.extend_from_slice(&[
                2.0,
                cx + self.dx,
                self.dy - cy,
                x + self.dx,
                self.dy - y,
            ]);
        }
        fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
            self.out.extend_from_slice(&[
                3.0,
                a + self.dx,
                self.dy - b,
                c + self.dx,
                self.dy - d,
                x + self.dx,
                self.dy - y,
            ]);
        }
        fn close(&mut self) {
            self.out.push(4.0);
        }
    }

    /// A parsed face: harfrust's owned `Font` and the bytes skrifa borrows.
    pub struct Face {
        bytes: Arc<Vec<u8>>,
        index: u32,
        hr: harfrust::Font,
    }

    impl Face {
        /// Parses a face. Hostile bytes must give `Err`, never a panic.
        pub fn new(bytes: Vec<u8>, index: u32) -> Result<Self, Error> {
            let bytes = Arc::new(bytes);
            FontRef::from_index(&bytes, index).map_err(|_| Error::Parse)?;
            let blob: Arc<dyn AsRef<[u8]> + Send + Sync> = bytes.clone();
            let hr = harfrust::Font::new(blob, index).ok_or(Error::Parse)?;
            Ok(Self { bytes, index, hr })
        }

        /// Shapes `text` and draws every glyph (no outline cache) at `size_px` per em.
        pub fn shape_and_outline(&self, text: &str, size_px: f32) -> Result<Shaped, Error> {
            let font = FontRef::from_index(&self.bytes, self.index).map_err(|_| Error::Parse)?;
            let shaper = ShaperFont::new(&self.hr);
            let mut buf = Buffer::new();
            buf.push_str(text);
            buf.guess_segment_properties();
            harfrust::shape(&shaper, &mut buf, ShapeOptions::new()).map_err(|_| Error::Shape)?;
            let upem = f32::from(self.hr.units_per_em().max(1));
            let scale = size_px / upem;
            let outlines = font.outline_glyphs();
            let mut out = Shaped::default();
            let mut x = 0.0f32;
            for (info, pos) in buf.glyph_infos().iter().zip(buf.glyph_positions()) {
                out.glyph_ids.push(info.glyph_id);
                out.advances.push(pos.x_advance);
                if let Some(g) = outlines.get(GlyphId::new(info.glyph_id)) {
                    let mut pen = Pen {
                        out: &mut out.path,
                        dx: x + pos.x_offset as f32 * scale,
                        dy: -(pos.y_offset as f32) * scale,
                    };
                    let settings =
                        DrawSettings::unhinted(Size::new(size_px), LocationRef::default());
                    if g.draw(settings, &mut pen).is_err() {
                        out.draw_errors += 1;
                    }
                }
                x += pos.x_advance as f32 * scale;
            }
            Ok(out)
        }

        /// Draws every glyph of the face (hostile-input check); returns (drawn, errors).
        pub fn outline_all(&self) -> (usize, usize) {
            let Ok(font) = FontRef::from_index(&self.bytes, self.index) else {
                return (0, 0);
            };
            let outlines = font.outline_glyphs();
            let mut path = Vec::new();
            let (mut ok, mut err) = (0, 0);
            for gid in 0..self.hr.num_glyphs() {
                path.clear();
                if let Some(g) = outlines.get(GlyphId::new(gid)) {
                    let mut pen = Pen {
                        out: &mut path,
                        dx: 0.0,
                        dy: 0.0,
                    };
                    let settings = DrawSettings::unhinted(Size::new(20.0), LocationRef::default());
                    if g.draw(settings, &mut pen).is_ok() {
                        ok += 1
                    } else {
                        err += 1
                    }
                }
            }
            (ok, err)
        }

        /// Nominal advance of a character's glyph in font units (unshaped).
        pub fn nominal_advance(&self, c: char) -> Option<f32> {
            let font = FontRef::from_index(&self.bytes, self.index).ok()?;
            let gid = font.charmap().map(c)?;
            font.glyph_metrics(Size::unscaled(), LocationRef::default())
                .advance_width(gid)
        }
    }

    /// Cold path: parse + shape + outline.
    pub fn shape_cold(bytes: &[u8], text: &str) -> Result<Shaped, Error> {
        Face::new(bytes.to_vec(), 0)?.shape_and_outline(text, 20.0)
    }
}

#[cfg(feature = "engine")]
pub use engine::*;

// ---- wasm entry points (also the size probe) ----
use wasm_bindgen::prelude::*;

/// Parses a face once (warm benchmark handle).
#[wasm_bindgen]
pub struct WasmFace {
    #[cfg(feature = "engine")]
    face: engine::Face,
    #[cfg(not(feature = "engine"))]
    bytes: Vec<u8>,
}

#[wasm_bindgen]
impl WasmFace {
    /// Parses the bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: Vec<u8>) -> Result<WasmFace, JsValue> {
        #[cfg(feature = "engine")]
        {
            engine::Face::new(bytes, 0)
                .map(|face| WasmFace { face })
                .map_err(|_| JsValue::from_str("parse"))
        }
        #[cfg(not(feature = "engine"))]
        {
            Ok(WasmFace { bytes })
        }
    }

    /// Shapes and outlines; returns the number of path floats.
    pub fn shape(&self, text: &str) -> usize {
        #[cfg(feature = "engine")]
        {
            self.face
                .shape_and_outline(text, 20.0)
                .map(|s| s.path.len())
                .unwrap_or(0)
        }
        #[cfg(not(feature = "engine"))]
        {
            self.bytes.len() + text.len()
        }
    }
}

/// Cold: parse + shape + outline; returns the number of path floats.
#[wasm_bindgen]
pub fn shape_cold_js(bytes: &[u8], text: &str) -> usize {
    #[cfg(feature = "engine")]
    {
        engine::shape_cold(bytes, text)
            .map(|s| s.path.len())
            .unwrap_or(0)
    }
    #[cfg(not(feature = "engine"))]
    {
        bytes.iter().map(|b| usize::from(*b)).sum::<usize>() + text.len()
    }
}

/// Size probe: harfrust shaping only.
#[cfg(feature = "hr-only")]
#[wasm_bindgen]
pub fn probe_hr(bytes: Vec<u8>, text: &str) -> usize {
    let blob: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> = std::sync::Arc::new(bytes);
    let Some(font) = harfrust::Font::new(blob, 0) else {
        return 0;
    };
    let shaper = harfrust::ShaperFont::new(&font);
    let mut buf = harfrust::Buffer::new();
    buf.push_str(text);
    buf.guess_segment_properties();
    let _ = harfrust::shape(&shaper, &mut buf, harfrust::ShapeOptions::new());
    buf.glyph_infos().iter().map(|g| g.glyph_id as usize).sum()
}

/// Size probe: skrifa outlines only.
#[cfg(feature = "sk-only")]
#[wasm_bindgen]
pub fn probe_sk(bytes: &[u8], text: &str) -> usize {
    use skrifa::{
        MetadataProvider,
        instance::{LocationRef, Size},
        outline::{DrawSettings, OutlinePen},
    };
    struct P(usize);
    impl OutlinePen for P {
        fn move_to(&mut self, _: f32, _: f32) {
            self.0 += 1
        }
        fn line_to(&mut self, _: f32, _: f32) {
            self.0 += 1
        }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1
        }
        fn close(&mut self) {
            self.0 += 1
        }
    }
    let Ok(font) = skrifa::FontRef::from_index(bytes, 0) else {
        return 0;
    };
    let o = font.outline_glyphs();
    let cm = font.charmap();
    let mut p = P(0);
    for c in text.chars() {
        if let Some(g) = cm.map(c).and_then(|g| o.get(g)) {
            let _ = g.draw(
                DrawSettings::unhinted(Size::new(20.0), LocationRef::default()),
                &mut p,
            );
        }
    }
    p.0
}

/// Fallback pair: rustybuzz shaping + ttf-parser outlines.
#[cfg(feature = "rb")]
pub mod rb {
    use ttf_parser::OutlineBuilder;
    struct P<'a>(&'a mut Vec<f32>, f32);
    impl OutlineBuilder for P<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.extend_from_slice(&[0.0, x + self.1, y])
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.extend_from_slice(&[1.0, x + self.1, y])
        }
        fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
            self.0
                .extend_from_slice(&[2.0, a + self.1, b, x + self.1, y])
        }
        fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
            self.0
                .extend_from_slice(&[3.0, a + self.1, b, c + self.1, d, x + self.1, y])
        }
        fn close(&mut self) {
            self.0.push(4.0)
        }
    }
    /// Shape + outline; returns path float count.
    pub fn shape_and_outline(face: &rustybuzz::Face<'_>, text: &str) -> usize {
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(text);
        buf.guess_segment_properties();
        let gb = rustybuzz::shape(face, &[], buf);
        let mut out = Vec::new();
        let mut x = 0.0;
        for (i, p) in gb.glyph_infos().iter().zip(gb.glyph_positions()) {
            let _ = face.outline_glyph(ttf_parser::GlyphId(i.glyph_id as u16), &mut P(&mut out, x));
            x += p.x_advance as f32;
        }
        out.len()
    }
}

/// Size probe for the fallback pair.
#[cfg(feature = "rb")]
#[wasm_bindgen]
pub fn probe_rb(bytes: &[u8], text: &str) -> usize {
    rustybuzz::Face::from_slice(bytes, 0).map_or(0, |f| rb::shape_and_outline(&f, text))
}
