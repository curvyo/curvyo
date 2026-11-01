//! Reading the `curvyo:` profile v1 (criteria 57, 132, 141 to 144, 149): the raw shadow values
//! next to an element, read only when the root says `curvyo:format="1"`. The reader does not
//! judge them; the importer keeps a value only when the standard SVG next to it agrees.

use roxmltree::Node;

use crate::export::{MarkerShape, Shadow, ShadowKind, ShadowMarkers};
use crate::write::{CURVYO_FORMAT, CURVYO_NS};

/// The most numbers one `curvyo:` list attribute is read with.
const MAX_LIST: usize = crate::limits::MAX_DASHES;

/// Whether the root `root` declares the profile this reader knows.
pub(crate) fn profile_on(root: Node<'_, '_>) -> bool {
    root.attribute((CURVYO_NS, "format")).map(str::trim) == Some(CURVYO_FORMAT)
}

/// `curvyo:<name>` of `node`.
pub(crate) fn curvyo<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attribute((CURVYO_NS, name))
}

/// A list of finite numbers, or `None` when any item does not read (or there are too many).
fn numbers(text: &str) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    for item in text.split(|c: char| c.is_ascii_whitespace() || c == ',') {
        if item.is_empty() {
            continue;
        }
        let value: f64 = item.parse().ok()?;
        if !value.is_finite() || out.len() >= MAX_LIST {
            return None;
        }
        out.push(value);
    }
    Some(out)
}

#[allow(clippy::option_option)] // unreadable (outer) versus the slot `none` (inner)
fn marker_shape(text: &str) -> Option<Option<MarkerShape>> {
    match text {
        "none" => Some(None),
        "arrow" => Some(Some(MarkerShape::Arrow)),
        "dot" => Some(Some(MarkerShape::Dot)),
        _ => None,
    }
}

/// `curvyo:markers` (criterion 141 (d)): all five keys in their order, else `None`.
pub(crate) fn markers(text: &str) -> Option<ShadowMarkers> {
    let mut parts = text.split_ascii_whitespace();
    let mut value = |key: &str| {
        parts
            .next()
            .and_then(|part| part.strip_prefix(key))
            .and_then(|rest| rest.strip_prefix('='))
    };
    let start = marker_shape(value("start")?)?;
    let middle = marker_shape(value("middle")?)?;
    let at_nodes = match value("place")? {
        "nodes" => true,
        "spaced" => false,
        _ => return None,
    };
    let count = value("count")?.parse().ok()?;
    let end = marker_shape(value("end")?)?;
    Some(ShadowMarkers {
        start,
        middle,
        at_nodes,
        count,
        end,
    })
}

/// The shadow values of `node` as written; unreadable values are left out.
pub(crate) fn read_shadow(node: Node<'_, '_>) -> Shadow {
    let list = |name: &str| curvyo(node, name).and_then(numbers);
    Shadow {
        dash: list("dash"),
        stroke_width: list("stroke-width").and_then(|v| (v.len() == 1).then(|| v[0])),
        radii: list("radii").and_then(|v| <[f64; 4]>::try_from(v).ok()),
        kind: curvyo(node, "kind").and_then(|t| ShadowKind::from_name(t.trim())),
        frame: list("frame"),
        markers: curvyo(node, "markers").and_then(markers),
    }
}

/// Which of `marker-start`, `marker-mid`, `marker-end` `node` sets to a reference, as an
/// attribute or in its `style` (criterion 142).
pub(crate) fn marker_refs(node: Node<'_, '_>) -> [bool; 3] {
    let style = node.attribute("style").unwrap_or_default();
    ["marker-start", "marker-mid", "marker-end"].map(|name| {
        let in_style = style.split(';').any(|declaration| {
            declaration
                .split_once(':')
                .is_some_and(|(k, v)| k.trim() == name && v.trim().starts_with("url("))
        });
        node.attribute(name)
            .is_some_and(|v| v.trim().starts_with("url("))
            || in_style
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac141_markers_text_reads_back() {
        let m = ShadowMarkers {
            start: Some(MarkerShape::Arrow),
            middle: None,
            at_nodes: true,
            count: 3,
            end: Some(MarkerShape::Dot),
        };
        assert_eq!(markers(&crate::write_leaf::markers_text(m)), Some(m));
        assert_eq!(markers("start=arrow"), None);
        assert_eq!(
            markers("middle=none start=none place=spaced count=1 end=none"),
            None
        );
    }

    #[test]
    fn ac57_lists_refuse_bad_items() {
        assert_eq!(numbers("1 2,3"), Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(numbers("1 x"), None);
        assert_eq!(numbers("1 NaN"), None);
    }
}
