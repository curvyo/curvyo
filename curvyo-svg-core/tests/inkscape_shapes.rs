//! The reader's side of slice 3 of `specs/0024-svg-import-export` (criteria 112 to 124): it
//! exposes the numbers of Inkscape's arc, star, spiral and 3D box tools; the decisions are the
//! importer's.
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::needless_raw_string_hashes,
    clippy::approx_constant
)]

use curvyo_svg_core::{
    InkscapeObject, LeafKind, NodeState, SvgDrawing, SvgLeaf, SvgNode, read_svg,
};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;

fn mm(body: &str) -> SvgDrawing {
    read_svg(
        format!(r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#)
            .as_bytes(),
    )
    .unwrap()
}

fn leaves(nodes: &[SvgNode]) -> Vec<(&NodeState, &SvgLeaf)> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => out.extend(leaves(children)),
            SvgNode::Leaf { state, leaf } => out.push((state, leaf)),
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
    out
}

const PIE: &str = r#"<path sodipodi:type="arc" sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:start="0" sodipodi:end="1.5707963" sodipodi:arc-type="slice" d="M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z"/>"#;

#[test]
fn ac112_arc_numbers_are_read() {
    let d = mm(PIE);
    let LeafKind::Arc {
        arc, path, fine, ..
    } = &leaves(&d.nodes)[0].1.kind
    else {
        panic!("{:?}", leaves(&d.nodes)[0].1.kind)
    };
    assert_eq!(
        (arc.center.x, arc.center.y, arc.rx, arc.ry),
        (50.0, 50.0, 20.0, 20.0)
    );
    assert_eq!((arc.start, arc.end), (Some(0.0), Some(1.570_796_3)));
    assert_eq!(arc.arc_type.as_deref(), Some("slice"));
    assert!(!arc.open);
    assert_eq!(path.elements().len(), 5, "two eighth turns, a line and Z");
    assert!(
        fine.as_ref().unwrap().elements().len() > path.elements().len(),
        "the check path splits the arc finer"
    );
}

#[test]
fn ac113_arc_without_angles_and_with_open() {
    let d = mm(
        r#"<path sodipodi:type="arc" sodipodi:cx="30" sodipodi:cy="20" sodipodi:rx="10" sodipodi:ry="5" sodipodi:open="true" d="M 40,20 A 10,5 0 1 1 20,20 10,5 0 1 1 40,20 Z"/>"#,
    );
    let LeafKind::Arc { arc, .. } = &leaves(&d.nodes)[0].1.kind else {
        panic!()
    };
    assert_eq!((arc.start, arc.end, arc.open), (None, None, true));
    assert_eq!(arc.arc_type, None);
}

#[test]
fn ac111_ac124_unreadable_arc_numbers_give_a_path() {
    let d = mm(
        r#"<path sodipodi:type="arc" sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" d="M 70,50 A 20,20 0 0 1 50,70"/>
        <path sodipodi:type="arc" sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:start="NaN" d="M 70,50 A 20,20 0 0 1 50,70"/>
        <path sodipodi:type="arc" sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:end="x" d="M 70,50 A 20,20 0 0 1 50,70"/>"#,
    );
    let all = leaves(&d.nodes);
    assert_eq!(all.len(), 3);
    assert!(
        all.iter()
            .all(|(_, leaf)| matches!(leaf.kind, LeafKind::Path { .. }))
    );
}

#[test]
fn ac117_star_exposes_arg2() {
    let d = mm(
        r#"<path sodipodi:type="star" sodipodi:sides="5" sodipodi:cx="0" sodipodi:cy="0" sodipodi:r1="10" sodipodi:r2="4" sodipodi:arg1="-1.5707963" sodipodi:arg2="-0.9424778" inkscape:flatsided="false" d="M 0,-10 L 5,5 Z"/>
        <path sodipodi:type="star" sodipodi:sides="6" sodipodi:cx="0" sodipodi:cy="0" sodipodi:r1="10" sodipodi:arg1="0" inkscape:flatsided="true" d="M 10,0 L 5,8 Z"/>"#,
    );
    let all = leaves(&d.nodes);
    let LeafKind::Star { star, .. } = &all[0].1.kind else {
        panic!()
    };
    assert_eq!((star.r2, star.arg2), (Some(4.0), Some(-0.942_477_8)));
    let LeafKind::Star { star, .. } = &all[1].1.kind else {
        panic!()
    };
    assert_eq!(
        (star.flat, star.r2, star.arg2),
        (true, None, None),
        "r2 is optional"
    );
}

#[test]
fn ac119_ac120_spiral_and_box_are_marked() {
    let d = mm(
        r##"<path sodipodi:type="spiral" sodipodi:cx="40" sodipodi:cy="160" d="m 40,160 c 1,1 2,2 3,0"/>
        <g sodipodi:type="inkscape:box3d" inkscape:perspectiveID="#perspective1">
          <path sodipodi:type="inkscape:box3dside" inkscape:box3dsidetype="6" d="M 0,0 L 10,0 10,10 Z"/>
          <path d="M 0,0 L 5,0"/>
        </g>"##,
    );
    let SvgNode::Group {
        state, children, ..
    } = &d.nodes[1]
    else {
        panic!()
    };
    assert_eq!(state.inkscape, Some(InkscapeObject::Box3d));
    assert_eq!(
        children[0].state().inkscape,
        Some(InkscapeObject::Box3dSide)
    );
    assert_eq!(children[1].state().inkscape, None);
    assert_eq!(d.nodes[0].state().inkscape, Some(InkscapeObject::Spiral));
}
