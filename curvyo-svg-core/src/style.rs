//! The style cascade (`specs/0024-svg-import-export` criteria 36 to 43): presentation attribute,
//! then the matching `<style>` rules, then the `style` attribute, the later winning; inherited
//! properties flow down the tree, `opacity` and `display` do not. A value that cannot be read
//! is ignored, so the property keeps the value it would have without it.

use roxmltree::Node;

use crate::css::Sheets;
use crate::drawing::{Cap, Join, LeafStyle, Paint};
use crate::limits::MAX_DASHES;
use crate::paint::{BLACK, Ids, PaintValue, Rgba, colour, paint};
use crate::units::{Axis, Units};

/// The namespace of `xlink:href`.
pub(crate) const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// The properties a presentation attribute may set.
const PROPERTIES: [&str; 15] = [
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-dasharray",
    "color",
    "opacity",
    "display",
    "visibility",
    "clip-rule",
    "paint-order",
];

/// The style of one element after the cascade, lengths in its user units.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Computed {
    pub(crate) fill: PaintValue,
    pub(crate) fill_opacity: f64,
    pub(crate) even_odd: bool,
    pub(crate) stroke: PaintValue,
    /// `None` until a width is stated (SVG's default is one user unit).
    pub(crate) stroke_width: Option<f64>,
    pub(crate) stroke_opacity: f64,
    pub(crate) cap: Option<Cap>,
    pub(crate) join: Option<Join>,
    /// The dash lengths; empty is solid.
    pub(crate) dash: Vec<f64>,
    pub(crate) color: Rgba,
    pub(crate) hidden_visibility: bool,
    /// Not inherited.
    pub(crate) opacity: f64,
    /// Not inherited.
    pub(crate) display_none: bool,
}

impl Default for Computed {
    fn default() -> Self {
        Self {
            fill: PaintValue::Colour(BLACK),
            fill_opacity: 1.0,
            even_odd: false,
            stroke: PaintValue::None,
            stroke_width: None,
            stroke_opacity: 1.0,
            cap: None,
            join: None,
            dash: Vec::new(),
            color: BLACK,
            hidden_visibility: false,
            opacity: 1.0,
            display_none: false,
        }
    }
}

/// What the cascade reads besides the element.
pub(crate) struct Cascade<'a, 'd, 'input> {
    pub(crate) sheets: &'a Sheets<'a>,
    pub(crate) ids: &'a Ids<'d, 'input>,
}

impl Cascade<'_, '_, '_> {
    /// The computed style of `node` under `parent`, with `units` for lengths.
    pub(crate) fn computed(
        &self,
        node: Node<'_, '_>,
        parent: &Computed,
        units: &Units,
    ) -> Computed {
        let mut declarations: Vec<(String, &str)> = Vec::new();
        for attribute in node.attributes() {
            if attribute.namespace().is_none() && PROPERTIES.contains(&attribute.name()) {
                declarations.push((attribute.name().to_owned(), attribute.value()));
            }
        }
        for (name, value) in self.sheets.declarations(node) {
            declarations.push((name.to_ascii_lowercase(), value));
        }
        if let Some(style) = node.attribute("style") {
            for declaration in simplecss::DeclarationTokenizer::from(style) {
                declarations.push((declaration.name.to_ascii_lowercase(), declaration.value));
            }
        }
        let mut style = Computed {
            opacity: 1.0,
            display_none: false,
            ..parent.clone()
        };
        // `color` first, so a `currentColor` of the same element sees it.
        for (name, value) in declarations.iter().filter(|(name, _)| name == "color") {
            self.apply(&mut style, parent, name, value.trim(), units);
        }
        for (name, value) in declarations.iter().filter(|(name, _)| name != "color") {
            self.apply(&mut style, parent, name, value.trim(), units);
        }
        style
    }

    /// One declaration; an unreadable value changes nothing.
    fn apply(
        &self,
        style: &mut Computed,
        parent: &Computed,
        name: &str,
        value: &str,
        units: &Units,
    ) {
        if value.eq_ignore_ascii_case("inherit") {
            inherit(style, parent, name);
            return;
        }
        match name {
            "fill" => {
                if let Some(p) = paint(value, style.color, self.ids) {
                    style.fill = p;
                }
            }
            "stroke" => {
                if let Some(p) = paint(value, style.color, self.ids) {
                    style.stroke = p;
                }
            }
            "color" => {
                if !value.eq_ignore_ascii_case("currentcolor")
                    && let Some(c) = colour(value)
                {
                    style.color = c;
                }
            }
            "fill-opacity" => set(&mut style.fill_opacity, fraction(value)),
            "stroke-opacity" => set(&mut style.stroke_opacity, fraction(value)),
            "opacity" => set(&mut style.opacity, fraction(value)),
            "fill-rule" => match value {
                "evenodd" => style.even_odd = true,
                "nonzero" => style.even_odd = false,
                _ => {}
            },
            "stroke-width" => {
                if let Some(width) = units.parse(value, Axis::Diagonal) {
                    style.stroke_width = Some(width);
                }
            }
            "stroke-linecap" => {
                style.cap = match value {
                    "butt" => Some(Cap::Butt),
                    "round" => Some(Cap::Round),
                    "square" => Some(Cap::Square),
                    _ => style.cap,
                };
            }
            "stroke-linejoin" => {
                style.join = match value {
                    "miter" | "miter-clip" | "arcs" => Some(Join::Miter),
                    "round" => Some(Join::Round),
                    "bevel" => Some(Join::Bevel),
                    _ => style.join,
                };
            }
            "stroke-dasharray" => {
                if let Some(dash) = dash_list(value, units) {
                    style.dash = dash;
                }
            }
            "display" => style.display_none = value == "none",
            "visibility" => match value {
                "hidden" | "collapse" => style.hidden_visibility = true,
                "visible" => style.hidden_visibility = false,
                _ => {}
            },
            _ => {}
        }
    }
}

/// `inherit`: the parent's value of `name`.
fn inherit(style: &mut Computed, parent: &Computed, name: &str) {
    match name {
        "fill" => style.fill = parent.fill,
        "stroke" => style.stroke = parent.stroke,
        "color" => style.color = parent.color,
        "fill-opacity" => style.fill_opacity = parent.fill_opacity,
        "stroke-opacity" => style.stroke_opacity = parent.stroke_opacity,
        "opacity" => style.opacity = parent.opacity,
        "fill-rule" => style.even_odd = parent.even_odd,
        "stroke-width" => style.stroke_width = parent.stroke_width,
        "stroke-linecap" => style.cap = parent.cap,
        "stroke-linejoin" => style.join = parent.join,
        "stroke-dasharray" => style.dash.clone_from(&parent.dash),
        "display" => style.display_none = parent.display_none,
        "visibility" => style.hidden_visibility = parent.hidden_visibility,
        _ => {}
    }
}

fn set(slot: &mut f64, value: Option<f64>) {
    if let Some(value) = value {
        *slot = value;
    }
}

/// An opacity: a number or a percentage, clamped to 0..=1.
fn fraction(value: &str) -> Option<f64> {
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |n| (n, 0.01));
    let n: f64 = number.trim().parse().ok()?;
    let n = n * scale;
    n.is_finite().then(|| n.clamp(0.0, 1.0))
}

/// A dash list in user units: `none`, a negative length, or lengths that sum to 0 are solid
/// (empty); more than [`MAX_DASHES`] numbers keep the first ones. `None` when unreadable.
fn dash_list(value: &str, units: &Units) -> Option<Vec<f64>> {
    if value.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    let mut dash = Vec::new();
    for item in value
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|item| !item.is_empty())
    {
        if dash.len() == MAX_DASHES {
            break;
        }
        dash.push(units.parse(item, Axis::Diagonal)?);
    }
    if dash.iter().any(|d| *d < 0.0) || dash.iter().sum::<f64>() <= 0.0 {
        return Some(Vec::new());
    }
    Some(dash)
}

impl Computed {
    /// The leaf's style in millimetres under the map with `scale` (√|det| of user units to
    /// millimetres).
    pub(crate) fn leaf_style(&self, scale: f64) -> LeafStyle {
        let paint_of = |value: PaintValue, opacity: f64| match value {
            PaintValue::None => None,
            PaintValue::Colour(c) => Some(Paint {
                rgb: c.rgb,
                opacity: (opacity * c.alpha).clamp(0.0, 1.0),
            }),
        };
        let width = self.stroke_width;
        let stroked = width.is_none_or(|w| w > 0.0);
        let stroke = paint_of(self.stroke, self.stroke_opacity).filter(|_| stroked);
        let width_user = match (stroke.is_some(), width) {
            (true, None) => Some(1.0),
            (_, Some(w)) if w > 0.0 => Some(w),
            _ => None,
        };
        let dash = match width_user {
            Some(w) if !self.dash.is_empty() => self.dash.iter().map(|d| d / w).collect(),
            _ => Vec::new(),
        };
        LeafStyle {
            fill: paint_of(self.fill, self.fill_opacity),
            even_odd: self.even_odd,
            stroke,
            stroke_width_mm: width_user.map(|w| w * scale),
            dash,
            cap: self.cap,
            join: self.join,
        }
    }
}

/// The local `href` (SVG 2) or `xlink:href` of `node`.
pub(crate) fn href<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    node.attribute("href")
        .or_else(|| node.attribute((XLINK_NS, "href")))
        .map(str::trim)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn computed_of(text: &str) -> Vec<Computed> {
        let doc = roxmltree::Document::parse(text).unwrap();
        let css = crate::css::style_text(&doc);
        let sheets = Sheets::new(&css);
        let ids = Ids::new();
        let cascade = Cascade {
            sheets: &sheets,
            ids: &ids,
        };
        let units = Units {
            px_mm: 25.4 / 96.0,
            viewport: (100.0, 100.0),
        };
        let mut out = Vec::new();
        let mut stack = vec![(doc.root_element(), Computed::default())];
        while let Some((node, parent)) = stack.pop() {
            let style = cascade.computed(node, &parent, &units);
            for child in node
                .children()
                .filter(roxmltree::Node::is_element)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                stack.push((child, style.clone()));
            }
            if node.has_tag_name("path") {
                out.push(style);
            }
        }
        out
    }

    #[test]
    fn later_sources_win_and_unreadable_values_are_ignored() {
        let styles = computed_of(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.c{fill:blue}</style>
            <g fill="red" stroke="green" opacity="0.5">
              <path/>
              <path class="c" fill="lime"/>
              <path class="c" style="FILL: yellow !important"/>
              <path fill="bogus"/>
              <path style="fill:currentColor;color:#00f"/>
              <path stroke-dasharray="2 1" stroke-width="0.5"/>
              <path stroke-dasharray="2 -1"/>
            </g></svg>"#,
        );
        let fill = |i: usize| match styles[i].fill {
            PaintValue::Colour(c) => c.rgb,
            PaintValue::None => [1, 1, 1],
        };
        assert_eq!(fill(0), [255, 0, 0]);
        assert_eq!(styles[0].opacity, 1.0, "opacity is not inherited");
        assert_eq!(fill(1), [0, 0, 255]);
        assert_eq!(fill(2), [255, 255, 0]);
        assert_eq!(fill(3), [255, 0, 0]);
        assert_eq!(fill(4), [0, 0, 255]);
        let leaf = styles[5].leaf_style(1.0);
        assert_eq!(leaf.dash, [4.0, 2.0]);
        assert_eq!(leaf.stroke_width_mm, Some(0.5));
        assert_eq!(styles[6].dash, Vec::<f64>::new());
    }
}
