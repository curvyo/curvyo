//! The Curvyo SVG reader (ADR 0018): bytes of an untrusted SVG in, an [`SvgDrawing`] out, in
//! millimetres, with `kurbo` paths, styles resolved per leaf and linked clones that name another
//! leaf by index (`specs/0024-svg-import-export`). Also the hostile-XML guard and the path data
//! parser that SVG fonts share ([`xml_guard`], [`path_data`]).
//!
//! Pure: no file, network, clock or thread; nothing a file names is ever fetched. Depends on no
//! Curvyo crate.
#![forbid(unsafe_code)]

pub mod path_data;
pub mod xml_guard;

mod css;
mod drawing;
pub mod limits;
mod page;
mod paint;
mod read;
mod reader;
mod shapes;
mod style;
mod units;
mod uses;

pub use drawing::{
    Cap, DisplayUnitName, Join, LeafKind, LeafStyle, NodeState, PageBackground, Paint, Rgb,
    StarData, SvgDrawing, SvgLeaf, SvgNode, rounded_rect,
};
pub use read::{SvgError, read_svg};
