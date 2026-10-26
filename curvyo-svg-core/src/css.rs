//! The `<style>` subset (`specs/0024-svg-import-export` criterion 36): rules whose selector is an
//! element name, `.class`, `#id` or `*` (and comma lists of them) apply, by specificity and then
//! file order; a rule with a combinator, an attribute selector or a pseudo-class never matches,
//! and `@` rules are skipped by `simplecss` (`@import` is never followed).

use roxmltree::{Document, Node};
use simplecss::{AttributeOperator, PseudoClass, StyleSheet};

/// The text of every `<style>` element of type `text/css` (or none) in file order, one rule
/// block after the other.
pub(crate) fn style_text(doc: &Document<'_>) -> String {
    let mut text = String::new();
    for style in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "style")
    {
        let css_type = style.attribute("type").unwrap_or("text/css").trim();
        if !(css_type.is_empty() || css_type.eq_ignore_ascii_case("text/css")) {
            continue;
        }
        for child in style.children() {
            if let Some(part) = child.text() {
                text.push_str(part);
            }
        }
        text.push('\n');
    }
    text
}

/// The style sheet of a file, read once from [`style_text`].
pub(crate) struct Sheets<'css> {
    sheet: StyleSheet<'css>,
}

impl<'css> Sheets<'css> {
    /// The rules of `text`.
    pub(crate) fn new(text: &'css str) -> Self {
        Self {
            sheet: StyleSheet::parse(text),
        }
    }

    /// The declarations of the rules matching `node`, lowest priority first.
    pub(crate) fn declarations(&self, node: Node<'_, '_>) -> Vec<(&'css str, &'css str)> {
        let element = Simple(node);
        self.sheet
            .rules
            .iter()
            .filter(|rule| rule.selector.matches(&element))
            .flat_map(|rule| rule.declarations.iter().map(|d| (d.name, d.value)))
            .collect()
    }
}

/// An element seen by a selector: no parent and no sibling, so combinators never match; only
/// `class` and `id` take part in attribute tests; no pseudo-class matches.
struct Simple<'a, 'input>(Node<'a, 'input>);

impl simplecss::Element for Simple<'_, '_> {
    fn parent_element(&self) -> Option<Self> {
        None
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        None
    }

    fn has_local_name(&self, name: &str) -> bool {
        self.0.tag_name().name() == name
    }

    fn attribute_matches(&self, local_name: &str, operator: AttributeOperator<'_>) -> bool {
        match (local_name, operator) {
            ("class", AttributeOperator::Contains(_)) | ("id", AttributeOperator::Matches(_)) => {
                self.0
                    .attribute(local_name)
                    .is_some_and(|v| operator.matches(v))
            }
            _ => false,
        }
    }

    fn pseudo_class_matches(&self, _class: PseudoClass<'_>) -> bool {
        false
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn simple_selectors_apply_by_specificity_then_order() {
        let text = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>
            rect { fill: red }
            .a { fill: green }
            #r { fill: blue }
            * { stroke: black }
            g rect { fill: yellow }
            rect:first-child { fill: pink }
            rect[x] { fill: teal }
            @import url(http://example.com/x.css);
            .a, .b { stroke-width: 2 }
        </style><rect id="r" class="a b" x="1"/><rect class="b"/></svg>"#;
        let doc = Document::parse(text).unwrap();
        let css = style_text(&doc);
        let sheets = Sheets::new(&css);
        let rects: Vec<Node> = doc
            .descendants()
            .filter(|n| n.has_tag_name("rect"))
            .collect();
        let first = sheets.declarations(rects[0]);
        assert_eq!(
            first,
            [
                ("stroke", "black"),
                ("fill", "red"),
                ("fill", "green"),
                ("stroke-width", "2"),
                ("stroke-width", "2"),
                ("fill", "blue")
            ]
        );
        let second = sheets.declarations(rects[1]);
        assert_eq!(
            second,
            [("stroke", "black"), ("fill", "red"), ("stroke-width", "2")]
        );
    }
}
