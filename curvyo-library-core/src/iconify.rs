//! The Iconify-compatible API (ADR 0085 decision 5, `specs/0138` criteria 2, 7 and 10): the
//! request builders, the parsers of the four answers and the local SVG of an icon. Nothing here
//! sends or reads anything; the host does.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::catalogue::Licence;
use crate::error::IconifyError;
use crate::request::Request;

/// An answer is at most 8 MiB (ADR 0085 decision 5).
pub const MAX_ICONIFY_BYTES: usize = 8 * 1024 * 1024;
/// The most results one search keeps and asks for.
pub const MAX_SEARCH_RESULTS: usize = 64;
/// The search words are cut at this many characters (Proposal, ADR 0085 decision 5).
pub const MAX_WORDS_CHARS: usize = 100;
/// The most icon bodies one request asks for.
const MAX_ICONS_PER_REQUEST: usize = 100;

/// One icon set of an Iconify API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconSet {
    /// The set's prefix.
    pub prefix: String,
    /// The set's name.
    pub name: String,
    /// How many icons it holds.
    pub total: u64,
    /// The author's name.
    pub author: String,
    /// The author's address.
    pub author_url: String,
    /// The licence of the set (every icon of it).
    pub licence: Option<Licence>,
    /// Up to three sample icon names.
    pub samples: Vec<String>,
}

/// An icon by set and name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconRef {
    /// The set's prefix.
    pub prefix: String,
    /// The icon's name.
    pub name: String,
}

/// The answer of `/search`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAnswer {
    /// The icons found, in the order the source answered, at most [`MAX_SEARCH_RESULTS`].
    pub icons: Vec<IconRef>,
    /// The info of the sets the answer names, for the licence and author of a result.
    pub sets: Vec<IconSet>,
}

/// One icon's drawing data, aliases resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct IconBody {
    /// The icon's name.
    pub name: String,
    /// The SVG markup inside the `svg` element.
    pub body: String,
    /// The view box's left edge.
    pub left: f64,
    /// The view box's top edge.
    pub top: f64,
    /// The view box's width.
    pub width: f64,
    /// The view box's height.
    pub height: f64,
    /// Quarter turns clockwise, 0 to 3.
    pub rotate: u8,
    /// Mirrored left to right.
    pub h_flip: bool,
    /// Mirrored top to bottom.
    pub v_flip: bool,
}

fn name_ok(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 80
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn encode(text: &str) -> String {
    use std::fmt::Write as _;
    text.bytes().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

/// The search text as sent: trimmed, inner whitespace collapsed to one space, cut at
/// [`MAX_WORDS_CHARS`] characters; `None` for an empty text, which sends nothing.
#[must_use]
pub fn search_words(text: &str) -> Option<String> {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = collapsed.chars().take(MAX_WORDS_CHARS).collect();
    let cut = cut.trim_end().to_owned();
    (!cut.is_empty()).then_some(cut)
}

/// `GET /collections`.
#[must_use]
pub fn collections_request(base: &str) -> Request {
    Request::below(base, "/collections")
}

/// `GET /collection?prefix=<prefix>`; `None` for a prefix that is not a plain name.
#[must_use]
pub fn collection_request(base: &str, prefix: &str) -> Option<Request> {
    name_ok(prefix).then(|| Request::below(base, &format!("/collection?prefix={prefix}")))
}

/// `GET /<prefix>.json?icons=<a,b,...>` for at most 100 plain names; `None` when none is left.
#[must_use]
pub fn icons_request(base: &str, prefix: &str, names: &[String]) -> Option<Request> {
    let names: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| name_ok(n))
        .take(MAX_ICONS_PER_REQUEST)
        .collect();
    (name_ok(prefix) && !names.is_empty())
        .then(|| Request::below(base, &format!("/{prefix}.json?icons={}", names.join(","))))
}

/// `GET /search?query=<words>&limit=64` with the words of [`search_words`], percent-encoded and
/// nothing else (criterion 10).
#[must_use]
pub fn search_request(base: &str, words: &str) -> Request {
    Request::below(
        base,
        &format!("/search?query={}&limit={MAX_SEARCH_RESULTS}", encode(words)),
    )
}

fn limit(bytes: &[u8]) -> Result<(), IconifyError> {
    if bytes.len() > MAX_ICONIFY_BYTES {
        Err(IconifyError::TooLarge)
    } else {
        Ok(())
    }
}

#[derive(Deserialize, Default)]
struct WireAuthor {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize, Default)]
struct WireLicence {
    #[serde(default)]
    title: String,
    #[serde(default)]
    spdx: String,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct WireSet {
    #[serde(default)]
    name: String,
    #[serde(default)]
    total: u64,
    #[serde(default)]
    author: Option<WireAuthor>,
    #[serde(default)]
    license: Option<WireLicence>,
    #[serde(default)]
    samples: Vec<String>,
}

fn set_of(prefix: &str, wire: WireSet) -> IconSet {
    let author = wire.author.unwrap_or_default();
    IconSet {
        prefix: prefix.to_owned(),
        name: if wire.name.is_empty() {
            prefix.to_owned()
        } else {
            wire.name
        },
        total: wire.total,
        author: author.name,
        author_url: author.url,
        licence: wire
            .license
            .filter(|l| !l.title.is_empty() || !l.spdx.is_empty())
            .map(|l| Licence {
                spdx: l.spdx,
                name: l.title,
                url: l.url,
            }),
        samples: wire
            .samples
            .into_iter()
            .filter(|s| name_ok(s))
            .take(3)
            .collect(),
    }
}

/// The sets of `/collections`, in prefix order.
///
/// # Errors
/// [`IconifyError`] for an answer that is not an object of sets or is over the limit.
pub fn parse_collections(bytes: &[u8]) -> Result<Vec<IconSet>, IconifyError> {
    limit(bytes)?;
    let map: BTreeMap<String, serde_json::Value> =
        serde_json::from_slice(bytes).map_err(|_| IconifyError::Invalid)?;
    Ok(map
        .into_iter()
        .filter(|(prefix, _)| name_ok(prefix))
        .filter_map(|(prefix, value)| {
            serde_json::from_value::<WireSet>(value)
                .ok()
                .map(|wire| set_of(&prefix, wire))
        })
        .collect())
}

/// The icon names of `/collection`, each once, in the order the set lists them.
///
/// # Errors
/// [`IconifyError`] for an answer that is not a set's list or is over the limit.
pub fn parse_collection(bytes: &[u8]) -> Result<Vec<String>, IconifyError> {
    #[derive(Deserialize)]
    struct Wire {
        #[serde(default)]
        uncategorized: Vec<String>,
        #[serde(default)]
        categories: BTreeMap<String, Vec<String>>,
        #[serde(default)]
        prefix: Option<String>,
    }
    limit(bytes)?;
    let wire: Wire = serde_json::from_slice(bytes).map_err(|_| IconifyError::Invalid)?;
    if wire.prefix.is_none() && wire.uncategorized.is_empty() && wire.categories.is_empty() {
        return Err(IconifyError::Invalid);
    }
    let mut seen = HashSet::new();
    Ok(wire
        .uncategorized
        .into_iter()
        .chain(wire.categories.into_values().flatten())
        .filter(|name| name_ok(name) && seen.insert(name.clone()))
        .collect())
}

#[derive(Deserialize, Clone, Default)]
struct WireIcon {
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    left: Option<f64>,
    #[serde(default)]
    top: Option<f64>,
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
    #[serde(default)]
    rotate: Option<u8>,
    #[serde(default, rename = "hFlip")]
    h_flip: Option<bool>,
    #[serde(default, rename = "vFlip")]
    v_flip: Option<bool>,
}

#[derive(Deserialize)]
struct IconsWire {
    #[serde(default)]
    icons: BTreeMap<String, WireIcon>,
    #[serde(default)]
    aliases: BTreeMap<String, WireIcon>,
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
}

impl IconsWire {
    /// The icon `name` and its parents, at most four deep; `None` for a missing link.
    fn chain(&self, name: &str) -> Option<Vec<WireIcon>> {
        let find = |n: &str| self.icons.get(n).or_else(|| self.aliases.get(n)).cloned();
        let mut chain = vec![find(name)?];
        while chain.len() < 4 {
            let Some(parent) = chain.last().and_then(|i| i.parent.clone()) else {
                break;
            };
            chain.push(find(&parent)?);
        }
        Some(chain)
    }

    /// The drawing data of `name`, aliases resolved.
    fn resolve(&self, name: &str) -> Option<IconBody> {
        let chain = self.chain(name)?;
        let body = chain.last()?.body.clone()?;
        let pick = |f: fn(&WireIcon) -> Option<f64>| chain.iter().find_map(f);
        let turns: u32 = chain
            .iter()
            .map(|i| u32::from(i.rotate.unwrap_or(0) % 4))
            .sum();
        let flip = |f: fn(&WireIcon) -> Option<bool>| {
            chain.iter().filter(|i| f(i) == Some(true)).count() % 2 == 1
        };
        Some(IconBody {
            name: name.to_owned(),
            body,
            left: pick(|i| i.left).unwrap_or(0.0),
            top: pick(|i| i.top).unwrap_or(0.0),
            width: pick(|i| i.width).or(self.width).unwrap_or(16.0),
            height: pick(|i| i.height).or(self.height).unwrap_or(16.0),
            rotate: u8::try_from(turns % 4).unwrap_or(0),
            h_flip: flip(|i| i.h_flip),
            v_flip: flip(|i| i.v_flip),
        })
    }
}

/// The bodies of `/<prefix>.json?icons=`, aliases resolved (at most three parents deep).
///
/// # Errors
/// [`IconifyError`] for an answer that is not an icon set's data or is over the limit.
pub fn parse_icons(bytes: &[u8]) -> Result<Vec<IconBody>, IconifyError> {
    limit(bytes)?;
    let wire: IconsWire = serde_json::from_slice(bytes).map_err(|_| IconifyError::Invalid)?;
    Ok(wire
        .icons
        .keys()
        .chain(wire.aliases.keys())
        .filter(|name| name_ok(name))
        .filter_map(|name| wire.resolve(name))
        .collect())
}

/// The answer of `/search`, at most [`MAX_SEARCH_RESULTS`] icons given as `prefix:name`.
///
/// # Errors
/// [`IconifyError`] for an answer without an `icons` list or over the limit.
pub fn parse_search(bytes: &[u8]) -> Result<SearchAnswer, IconifyError> {
    #[derive(Deserialize)]
    struct Wire {
        icons: Vec<String>,
        #[serde(default)]
        collections: BTreeMap<String, serde_json::Value>,
    }
    limit(bytes)?;
    let wire: Wire = serde_json::from_slice(bytes).map_err(|_| IconifyError::Invalid)?;
    let icons = wire
        .icons
        .iter()
        .filter_map(|text| {
            let (prefix, name) = text.split_once(':')?;
            (name_ok(prefix) && name_ok(name)).then(|| IconRef {
                prefix: prefix.to_owned(),
                name: name.to_owned(),
            })
        })
        .take(MAX_SEARCH_RESULTS)
        .collect();
    let sets = wire
        .collections
        .into_iter()
        .filter(|(prefix, _)| name_ok(prefix))
        .filter_map(|(prefix, value)| {
            serde_json::from_value::<WireSet>(value)
                .ok()
                .map(|wire| set_of(&prefix, wire))
        })
        .collect();
    Ok(SearchAnswer { icons, sets })
}

fn number(value: f64) -> String {
    format!("{value}")
}

/// The SVG of an icon, built locally: its view box and body, mirrored and turned as the data says.
/// The text goes through the SVG reader and its limits (ADR 0018) like any file.
#[must_use]
pub fn icon_svg(icon: &IconBody) -> String {
    svg_with_size(icon, None)
}

/// A length in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Millimetres(pub f64);

/// [`icon_svg`] drawn `width` wide (its height by the icon's proportion): the size of an
/// imported icon (`specs/0138` criterion 3, 24 mm).
#[must_use]
pub fn icon_svg_mm(icon: &IconBody, width: Millimetres) -> String {
    svg_with_size(icon, Some(width.0))
}

fn svg_with_size(icon: &IconBody, width_mm: Option<f64>) -> String {
    let (w, h) = if icon.rotate % 2 == 1 {
        (icon.height, icon.width)
    } else {
        (icon.width, icon.height)
    };
    let (cx, cy) = (icon.left + icon.width / 2.0, icon.top + icon.height / 2.0);
    let mut transforms = Vec::new();
    if icon.h_flip {
        transforms.push(format!("translate({} 0) scale(-1 1)", number(2.0 * cx)));
    }
    if icon.v_flip {
        transforms.push(format!("translate(0 {}) scale(1 -1)", number(2.0 * cy)));
    }
    if icon.rotate != 0 {
        transforms.push(format!(
            "rotate({} {} {})",
            u16::from(icon.rotate) * 90,
            number(cx),
            number(cy)
        ));
    }
    let (vx, vy) = if icon.rotate % 2 == 1 {
        (cx - w / 2.0, cy - h / 2.0)
    } else {
        (icon.left, icon.top)
    };
    let body = if transforms.is_empty() {
        icon.body.clone()
    } else {
        format!(
            "<g transform=\"{}\">{}</g>",
            transforms.join(" "),
            icon.body
        )
    };
    let (width, height) = width_mm.map_or_else(
        || (number(w), number(h)),
        |mm| {
            let high = if w > 0.0 { mm * h / w } else { mm };
            (format!("{}mm", number(mm)), format!("{}mm", number(high)))
        },
    );
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{width}\" height=\"{height}\">{body}</svg>",
        number(vx),
        number(vy),
        number(w),
        number(h),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_words_are_trimmed_collapsed_and_cut() {
        assert_eq!(search_words("  home   icon "), Some("home icon".into()));
        assert_eq!(search_words("   "), None);
        assert_eq!(search_words(""), None);
        assert!(search_words(&"ab ".repeat(100)).unwrap().chars().count() <= 100);
        assert_eq!(search_words(&"x".repeat(150)).unwrap().chars().count(), 100);
    }

    #[test]
    fn requests_carry_only_the_protocols_path() {
        let base = "https://api.iconify.design";
        assert_eq!(
            collections_request(base).url(),
            "https://api.iconify.design/collections"
        );
        assert_eq!(
            collection_request(base, "mdi").unwrap().url(),
            "https://api.iconify.design/collection?prefix=mdi"
        );
        assert!(collection_request(base, "../x").is_none());
        let names = vec![
            "home".to_owned(),
            "bad name".to_owned(),
            "account-box".to_owned(),
        ];
        assert_eq!(
            icons_request(base, "mdi", &names).unwrap().url(),
            "https://api.iconify.design/mdi.json?icons=home,account-box"
        );
        assert!(icons_request(base, "mdi", &[]).is_none());
        assert_eq!(
            search_request(base, "home & garden").url(),
            "https://api.iconify.design/search?query=home%20%26%20garden&limit=64"
        );
    }

    #[test]
    fn at_most_100_names_go_in_one_request() {
        let names: Vec<String> = (0..150).map(|i| format!("n{i}")).collect();
        let url = icons_request("https://a.b", "x", &names).unwrap();
        assert_eq!(url.url().matches(',').count(), 99);
    }

    #[test]
    fn an_icon_with_a_flip_and_a_turn_is_wrapped() {
        let icon = IconBody {
            name: "a".into(),
            body: "<path d=\"M0 0\"/>".into(),
            left: 0.0,
            top: 0.0,
            width: 24.0,
            height: 12.0,
            rotate: 1,
            h_flip: true,
            v_flip: false,
        };
        let svg = icon_svg(&icon);
        assert!(svg.contains("viewBox=\"6 -6 12 24\""), "{svg}");
        assert!(
            svg.contains("scale(-1 1)") && svg.contains("rotate(90 12 6)"),
            "{svg}"
        );
    }
}
