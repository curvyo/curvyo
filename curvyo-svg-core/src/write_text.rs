//! A live text and an embedded picture as written (criteria 132 to 135 and 137): `<text>` with
//! one `tspan` (or `textPath`) per line and the face reference as shadow data, `<image>` with
//! its bytes as a base64 `data:` URI.

use crate::drawing_text::TextAnchor;
use crate::export::{ExportImage, ExportLine, ExportState, ExportText};
use crate::write::Writer;
use crate::write_number::{coord, escape_name, escape_text, exact};

/// `font-family`: quoted when it holds anything but letters, digits and hyphens (criterion
/// 132); a quote inside the name is dropped.
fn family_value(family: &str, out: &mut String) {
    let plain = !family.is_empty() && family.chars().all(|c| c.is_alphanumeric() || c == '-');
    if plain {
        escape_name(family, out);
    } else {
        out.push('\'');
        let cleaned: String = family
            .chars()
            .filter(|c| *c != '\'' && *c != '\\')
            .collect();
        escape_name(&cleaned, out);
        out.push('\'');
    }
}

impl Writer<'_> {
    pub(crate) fn text(&mut self, state: &ExportState, text: &ExportText, depth: usize) {
        let id = self.take_id();
        self.indent(depth);
        self.out.push_str("<text");
        self.attr("id", &id);
        self.attr("xml:space", "preserve");
        if let Some(name) = &state.name {
            self.attr_name("inkscape:label", name);
        }
        if text.transform != kurbo::Affine::IDENTITY {
            self.matrix(text.transform);
        }
        if let Some(first) = text.lines.first().filter(|l| l.on_path.is_none()) {
            self.attr_coord("x", first.x);
            self.attr_coord("y", first.y);
        }
        self.out.push_str(" font-family=\"");
        family_value(&text.family, &mut self.out);
        self.out.push('"');
        self.attr("font-weight", &text.weight.to_string());
        self.attr("font-style", if text.italic { "italic" } else { "normal" });
        if let Some(stretch) = text.stretch {
            self.attr("font-stretch", stretch);
        }
        self.attr_coord("font-size", text.font_size);
        match text.anchor {
            TextAnchor::Start => {}
            TextAnchor::Middle => self.attr("text-anchor", "middle"),
            TextAnchor::End => self.attr("text-anchor", "end"),
        }
        if text.letter_spacing != 0.0 {
            self.attr_coord("letter-spacing", text.letter_spacing);
        }
        if text.word_spacing != 0.0 {
            self.attr_coord("word-spacing", text.word_spacing);
        }
        if let Some(line_height) = text.line_height {
            self.attr_exact("line-height", line_height);
        }
        if let Some(mode) = text.writing_mode {
            self.attr("writing-mode", mode);
        }
        if let Some(orientation) = text.orientation {
            self.attr("text-orientation", orientation);
        }
        self.state_flags(state);
        self.paint(&text.style, false);
        self.opacity(state);
        self.attr_exact("curvyo:size", text.size);
        self.attr_name("curvyo:face", &text.face);
        self.out.push('>');
        for line in &text.lines {
            self.line(line);
        }
        self.out.push_str("</text>\n");
    }

    fn line(&mut self, line: &ExportLine) {
        #[allow(clippy::single_match_else)] // the two elements a line can be
        match line.on_path {
            Some(path) => {
                debug_assert!(
                    self.ids.keys.contains_key(&path.carrier),
                    "the mapping gives every carrier a key"
                );
                self.out.push_str("<textPath");
                let href = format!(
                    "#{}",
                    self.ids
                        .keys
                        .get(&path.carrier)
                        .cloned()
                        .unwrap_or_default()
                );
                self.attr("xlink:href", &href);
                self.attr_coord("startOffset", path.start_offset);
                if path.side_right {
                    self.attr("side", "right");
                }
            }
            None => {
                self.out.push_str("<tspan");
                self.attr("sodipodi:role", "line");
                self.attr_coord("x", line.x);
                self.attr_coord("y", line.y);
            }
        }
        if !line.dx.is_empty() {
            self.attr_list("dx", &line.dx, coord);
        }
        if !line.dy.is_empty() {
            self.attr_list("dy", &line.dy, coord);
        }
        if !line.rotate.is_empty() {
            self.attr_list("rotate", &line.rotate, exact);
        }
        self.out.push('>');
        escape_text(&line.text, &mut self.out);
        self.out.push_str(if line.on_path.is_some() {
            "</textPath>"
        } else {
            "</tspan>"
        });
    }

    #[allow(clippy::many_single_char_names)] // the SVG names of the matrix entries
    pub(crate) fn image(&mut self, state: &ExportState, image: &ExportImage, depth: usize) {
        let id = self.take_id();
        self.indent(depth);
        self.out.push_str("<image");
        self.attr("id", &id);
        if let Some(name) = &state.name {
            self.attr_name("inkscape:label", name);
        }
        let [a, b, c, d, e, f] = image.placement.as_coeffs();
        if b == 0.0 && c == 0.0 && a > 0.0 && d > 0.0 {
            self.attr_coord("x", e);
            self.attr_coord("y", f);
            self.attr_coord("width", a);
            self.attr_coord("height", d);
            self.attr("preserveAspectRatio", "none");
        } else {
            self.attr("x", "0");
            self.attr("y", "0");
            self.attr("width", "1");
            self.attr("height", "1");
            self.attr("preserveAspectRatio", "none");
            self.matrix(image.placement);
        }
        self.state_flags(state);
        self.opacity(state);
        self.out.push_str(" xlink:href=\"data:");
        self.out.push_str(image.mime);
        self.out.push_str(";base64,");
        crate::base64::encode(&image.bytes, &mut self.out);
        self.out.push_str("\"/>\n");
    }
}
