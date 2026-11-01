//! The Curvyo SVG reader and writer (ADR 0018): bytes of an untrusted SVG in, an [`SvgDrawing`] out, in
//! millimetres, with `kurbo` paths, styles resolved per leaf and linked clones that name another
//! leaf by index (`specs/0024-svg-import-export`); and the writer, an [`SvgExport`] in, the SVG
//! text out ([`write_svg`]). Also the hostile-XML guard and the path data
//! parser that SVG fonts share ([`xml_guard`], [`path_data`]).
//!
//! Pure: no file, network, clock or thread; nothing a file names is ever fetched. Depends on no
//! Curvyo crate.
#![forbid(unsafe_code)]

pub mod path_data;
pub mod xml_guard;

mod base64;
mod css;
mod data_uri;
mod drawing;
mod drawing_text;
mod export;
mod image;
pub mod limits;
mod page;
mod paint;
mod read;
mod reader;
mod shadow;
mod shapes;
mod style;
mod text;
mod text_chars;
mod text_style;
mod units;
mod uses;
mod write;
mod write_leaf;
mod write_number;
mod write_text;

pub use data_uri::decode_data_uri;
pub use drawing::{
    ArcData, Cap, DisplayUnitName, InkscapeObject, Join, LeafKind, LeafStyle, NodeState,
    PageBackground, Paint, Rgb, StarData, SvgDrawing, SvgLeaf, SvgNode, SvgPage, rounded_rect,
};
pub use drawing_text::{
    Baseline, BaselineShift, FlowAlign, FlowBox, FontFamily, ImageAspect, LineHeight, SvgChar,
    SvgImage, SvgOrientation, SvgText, SvgTextPath, SvgWritingMode, TextAnchor, TextStyle,
};
pub use export::{
    ExportImage, ExportKind, ExportLeaf, ExportLine, ExportNode, ExportPage, ExportState,
    ExportStroke, ExportStyle, ExportText, ExportTextPath, MarkerShape, PathId, Shadow, ShadowKind,
    ShadowMarkers, SvgExport,
};
pub use read::{SvgError, read_svg};
pub use uses::is_isometry;
pub use write::write_svg;
