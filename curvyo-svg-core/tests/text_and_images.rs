//! The font-free reading of texts and images (`specs/0024-svg-import-export` slice 2, criteria
//! 78 to 105): characters, styles, positions, lines, text paths and their carriers, flowed text,
//! `switch`, data URIs and the limits.
#![allow(clippy::unwrap_used)]

use curvyo_svg_core::{FontFamily, LeafKind, SvgDrawing, SvgNode, SvgText, TextAnchor, read_svg};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape""#;

fn mm(body: &str) -> SvgDrawing {
    read_svg(
        format!(r#"<svg {NS} width="100mm" height="100mm" viewBox="0 0 100 100">{body}</svg>"#)
            .as_bytes(),
    )
    .unwrap()
}

fn texts(nodes: &[SvgNode]) -> Vec<&SvgText> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => out.extend(texts(children)),
            SvgNode::Text { text, .. } => out.push(&**text),
            _ => {}
        }
    }
    out
}

fn string(text: &SvgText) -> String {
    text.chars.iter().map(|c| c.ch).collect()
}

/// Criteria 78, 79, 85: Inkscape's lines become line breaks, an empty line is kept, an empty text
/// or one in `defs` gives nothing, the label names the text.
#[test]
fn ac78_ac85_inkscape_lines() {
    let d = mm(
        r#"<text xml:space="preserve" inkscape:label="Title" style="font-size:10px;line-height:1.25;font-family:'Source Sans 3';text-anchor:middle" x="50" y="20"><tspan sodipodi:role="line" x="50" y="20">One</tspan><tspan sodipodi:role="line" x="50" y="32.5"/><tspan sodipodi:role="line" x="50" y="45">Three</tspan></text>
        <text> </text><defs><text>hidden</text></defs><text style="font-size:0">zero</text>"#,
    );
    let all = texts(&d.nodes);
    assert_eq!(all.len(), 1);
    assert_eq!(string(all[0]), "One\n\nThree");
    assert_eq!(d.nodes[0].state().name.as_deref(), Some("Title"));
    let style = &all[0].styles[all[0].chars[0].style];
    assert_eq!(style.families, [FontFamily::Named("Source Sans 3".into())]);
    assert_eq!(style.anchor, TextAnchor::Middle);
    assert_eq!(style.font_size, 10.0);
    assert_eq!(all[0].chars[5].y, Some(45.0));
    assert_eq!(all[0].chars[0].x, Some(50.0));
}

/// Criteria 86, 87: per-character lists, nested values win, `rotate` holds its last value.
#[test]
fn ac86_ac87_position_lists() {
    let d = mm(
        r#"<text x="10 20 30" y="50" rotate="5 15">ABCD<tspan dx="2" dy="-1 3">EF</tspan></text>"#,
    );
    let t = texts(&d.nodes)[0];
    let xs: Vec<_> = t.chars.iter().map(|c| c.x).collect();
    assert_eq!(xs, [Some(10.0), Some(20.0), Some(30.0), None, None, None]);
    assert_eq!(t.chars[4].dx, 2.0);
    assert_eq!(t.chars[5].dy, 3.0);
    assert_eq!(t.chars[5].rotate, Some(15.0));
}

/// Criterion 99: Illustrator's `switch` and a bare `foreignObject` are passed over.
#[test]
fn ac99_switch_takes_the_first_child_svg_draws() {
    for first in [
        r#"<foreignObject requiredExtensions="http://ns.adobe.com/AdobeIllustrator/10.0/"><x/></foreignObject>"#,
        r#"<foreignObject width="10" height="10"><x/></foreignObject>"#,
        r#"<g systemLanguage="de"><rect width="9" height="9"/></g>"#,
        r#"<g requiredFeatures="http://example.com/feature"><rect width="9" height="9"/></g>"#,
    ] {
        let d = mm(&format!(
            r#"<switch>{first}<g><rect width="1" height="1"/><text y="5">Hi</text></g></switch>"#
        ));
        let SvgNode::Group { children, .. } = &d.nodes[0] else {
            panic!("{first}: {:?}", d.nodes)
        };
        assert_eq!(children.len(), 2, "{first}");
        assert_eq!(texts(&d.nodes).len(), 1);
    }
}

/// Criteria 93, 94: a text path on a drawn path names that leaf; an unpainted carrier becomes a
/// guide, a painted one stays; a text with its own transform gets a new guide path before it.
#[test]
fn ac93_ac94_carriers() {
    let d = mm(
        r##"<path id="ring" d="M10 50 A40 40 0 1 1 90 50" stroke="#000" fill="none"/>
        <path id="plain" d="M0 0 H100" fill="none"/>
        <text><textPath href="#ring" startOffset="50%">On ring</textPath></text>
        <text><textPath xlink:href="#plain">On plain</textPath></text>
        <text transform="translate(5 5)"><textPath href="#ring">Moved</textPath></text>
        <text><textPath href="#nowhere">Straight</textPath></text>"##,
    );
    let leaves: Vec<_> = d
        .nodes
        .iter()
        .filter_map(|n| match n {
            SvgNode::Leaf { state, leaf } => Some((state, leaf)),
            _ => None,
        })
        .collect();
    assert_eq!(leaves.len(), 3, "two drawn carriers and one new guide");
    assert!(!leaves[0].0.guide, "a stroked ring stays drawn");
    assert!(leaves[1].0.guide, "an unpainted path becomes a guide");
    assert!(leaves[2].0.guide);
    assert!(
        matches!(d.nodes[4], SvgNode::Leaf { .. }),
        "the guide stands before its text"
    );
    let t = texts(&d.nodes);
    assert_eq!(t[0].paths[0].carrier, Some(0));
    let half = std::f64::consts::PI * 40.0 / 2.0;
    assert!((t[0].paths[0].start_offset - half).abs() < 1e-3);
    assert_eq!(t[1].paths[0].carrier, Some(1));
    assert_eq!(t[2].paths[0].carrier, Some(2));
    let LeafKind::Path { path, .. } = &leaves[2].1.kind else {
        panic!()
    };
    assert!((path.elements()[0].end_point().unwrap().x - 15.0).abs() < 1e-9);
    assert_eq!(t[3].paths[0].carrier, None);
}

/// Criterion 96: a flowRoot's region is the box, each paragraph a line break.
#[test]
fn ac96_flow_root() {
    let d = mm(
        r#"<flowRoot style="font-size:5px;text-align:center"><flowRegion><rect x="10" y="20" width="60" height="40"/></flowRegion><flowPara>First paragraph</flowPara><flowPara>Second</flowPara></flowRoot>"#,
    );
    let t = texts(&d.nodes)[0];
    assert_eq!(string(t), "First paragraph\nSecond");
    let flow = t.flow.unwrap();
    assert_eq!((flow.left, flow.top, flow.width), (10.0, 20.0, 60.0));
}

/// Criteria 100, 102, 103, 105: a data URI image comes in, a linked one and one in `defs` do
/// not; a data URI over 16 MiB is read.
#[test]
fn ac100_ac103_ac105_images() {
    let d = mm(
        r#"<image href="data:image/png;base64,iVBORw0KGgo=" x="1" y="2" width="30" height="20" preserveAspectRatio="xMinYMax slice"/>
        <image xlink:href="photo.png" width="5" height="5"/><defs><image href="data:,x"/></defs>
        <image href="data:,x" width="0" height="5"/>"#,
    );
    let images: Vec<_> = d
        .nodes
        .iter()
        .filter_map(|n| match n {
            SvgNode::Image { image, .. } => Some(image),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].bytes, b"\x89PNG\r\n\x1a\n");
    assert_eq!(images[0].origin, (1.0, 2.0));
    assert_eq!(images[0].aspect.align, Some((0.0, 1.0)));
    assert!(images[0].aspect.slice);
    let big = "QUFB".repeat(5 * 1024 * 1024);
    let d = mm(&format!(r#"<image href="data:;base64,{big}"/>"#));
    let SvgNode::Image { image, .. } = &d.nodes[0] else {
        panic!()
    };
    assert_eq!(image.bytes.len(), 15 * 1024 * 1024);
    assert_eq!(image.size, (None, None));
}

/// Criterion 98: the file holds at most 1,000,000 text characters.
#[test]
fn ac98_text_character_limit() {
    let long = "x".repeat(1_200_000);
    let d = mm(&format!("<text>{long}</text><text>more</text>"));
    let t = texts(&d.nodes);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].chars.len(), 1_000_000);
    assert!(d.truncated);
}
