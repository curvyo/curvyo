//! The Data Matrix generator: stored parameters in, closed outlines out (spec 0144 criterion 12).

mod encode;
mod schema;
mod style;

use std::borrow::Cow;

use curvyo_extension_core::{
    Clamp, Edit, GenerateRequest, Generated, Marks, Param, ParamGroup, ParamValue, Part,
    StoredParams, Warning,
};

use crate::finder::FinderShape;
use crate::qr::{EncodeError, resolve_size};
use crate::style::{ModuleShape, Style};
use crate::values::Values;

pub(crate) use schema::groups;

fn schema() -> Vec<Param> {
    groups()
        .into_iter()
        .flat_map(|group: ParamGroup| group.params)
        .collect()
}

/// No parts (the host keeps the last ones), the parameters as typed and the reason; the content
/// itself is never in it.
fn refused(values: Values<'_>, error: EncodeError) -> Generated {
    let (params, _) = values.finish();
    let warning = match error {
        EncodeError::Empty => Warning {
            id: "content_empty",
            pieces: Vec::new(),
            value: None,
            limit: None,
            param: Some("content"),
            subject: None,
        },
        EncodeError::TooLong { bytes, limit } => Warning {
            id: "content_too_long",
            pieces: Vec::new(),
            value: u32::try_from(bytes).ok().map(ParamValue::Count),
            limit: u32::try_from(limit).ok().map(ParamValue::Count),
            param: Some("content"),
            subject: None,
        },
    };
    Generated {
        params,
        parts: Vec::new(),
        handles: Vec::new(),
        clamps: Vec::new(),
        automatic: Vec::new(),
        states: Vec::new(),
        limits: Vec::new(),
        warnings: vec![warning],
        marks: Marks::default(),
    }
}

/// Generates the Data Matrix `request` names.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = schema();
    let mut values = Values::read(&schema, request.params);
    let grid = match encode::encode(&values.text("content")) {
        Ok(grid) => grid,
        Err(error) => return refused(values, error),
    };
    let quiet = values.count("quiet_zone");
    let module = resolve_size(&mut values, request.edit, grid.width(), quiet);
    let drawn = Style {
        module: curvyo_document_core::Length::from_mm(module),
        quiet,
        shape: ModuleShape::from_id(&values.choice("module_shape")).unwrap_or(ModuleShape::Square),
        rounding: f64::from(values.percent("rounding")) / 100.0,
        fill: f64::from(values.percent("module_fill")) / 100.0,
        finder_outer: FinderShape::Square,
        finder_inner: FinderShape::Square,
        merged: values.choice("output") != "separate",
        inverted: values.switch("inverted"),
        cleared: Vec::new(),
    };
    let paths = if matches!(request.edit, Some(Edit::Draft)) {
        style::draft(&grid, &drawn)
    } else {
        style::style(&grid, &drawn)
    };
    let (params, clamps): (StoredParams, Vec<Clamp>) = values.finish();
    Generated {
        params,
        parts: vec![Part {
            key: Cow::Borrowed(crate::qr::PART),
            paths,
        }],
        handles: Vec::new(),
        clamps,
        automatic: Vec::new(),
        states: Vec::new(),
        limits: Vec::new(),
        warnings: Vec::new(),
        marks: Marks::default(),
    }
}

/// The grid of `content` (the test and the golden read it).
///
/// # Errors
/// [`EncodeError::Empty`] or [`EncodeError::TooLong`].
pub fn encode_data_matrix(content: &str) -> Result<crate::ModuleGrid, EncodeError> {
    encode::encode(content)
}

/// Draws a Data Matrix grid in `style` (finder fields are ignored: its finder is always solid).
#[must_use]
pub fn style_data_matrix(
    grid: &crate::ModuleGrid,
    style: &Style,
) -> Vec<curvyo_extension_core::ClosedPath> {
    style::style(grid, style)
}
