//! The QR code generator: stored parameters in, closed outlines out (ADR 0091 §3, §4, §5).

mod encode;
mod schema;

use std::borrow::Cow;

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, Clamp, Edit, GenerateRequest, Generated, Marks, Param, ParamGroup, ParamValue, Part,
    StoredParams, Warning,
};

use crate::clear_area::{self, ClearArea, LogoShape};
use crate::finder::FinderShape;
use crate::grid::ModuleGrid;
use crate::style::{ModuleShape, Style, draft, style};
use crate::values::Values;

pub use encode::{EcLevel, EncodeError, encode};
pub(crate) use schema::groups;

/// The key of the code's one part.
pub(crate) const PART: &str = "code";

fn schema() -> Vec<Param> {
    groups()
        .into_iter()
        .flat_map(|group: ParamGroup| group.params)
        .collect()
}

/// The result of a request: either the grid at a level, or the reason it cannot be drawn.
fn encoded(
    values: &mut Values<'_>,
    edit: Option<Edit<'_>>,
) -> Result<(ModuleGrid, EcLevel), EncodeError> {
    let content = values.text("content");
    let asked = EcLevel::from_id(&values.choice("ec")).unwrap_or(EcLevel::M);
    let raise = matches!(edit, Some(Edit::Param("logo"))) && values.switch("logo");
    if raise && asked < EcLevel::H {
        // Turning the clear area on raises the level, to the highest the content fits, no notice.
        for level in EcLevel::ALL
            .into_iter()
            .rev()
            .filter(|level| *level > asked)
        {
            if let Ok(grid) = encode(&content, level) {
                values.put("ec", &ParamValue::Choice(level.id().to_owned()));
                return Ok((grid, level));
            }
        }
    }
    match encode(&content, asked) {
        Ok(grid) => Ok((grid, asked)),
        Err(error @ EncodeError::TooLong { .. }) if matches!(edit, Some(Edit::Param("ec"))) => {
            // A level the content does not fit is lowered to the highest that fits.
            for level in EcLevel::ALL
                .into_iter()
                .rev()
                .filter(|level| *level < asked)
            {
                if let Ok(grid) = encode(&content, level) {
                    values.put("ec", &ParamValue::Choice(level.id().to_owned()));
                    values.set_clamp(
                        "ec",
                        ParamValue::Choice(level.id().to_owned()),
                        Bound::DependentFor("this content"),
                    );
                    return Ok((grid, level));
                }
            }
            Err(error)
        }
        Err(error) => Err(error),
    }
}

/// The subject of a clamp the error-correction level causes: "error correction M".
const fn subject(level: EcLevel) -> &'static str {
    match level {
        EcLevel::L => "error correction L",
        EcLevel::M => "error correction M",
        EcLevel::Q => "error correction Q",
        EcLevel::H => "error correction H",
    }
}

/// The result when the content cannot be drawn: no parts (the host keeps the last ones), the
/// parameters as typed, and the reason as a warning (the content itself is never in it).
fn refused(values: Values<'_>, error: EncodeError) -> Generated {
    let (params, _) = values.finish();
    let warning = match error {
        EncodeError::Empty => Warning {
            id: "content_empty",
            pieces: Vec::new(),
            value: None,
            limit: None,
        },
        EncodeError::TooLong { bytes, limit } => Warning {
            id: "content_too_long",
            pieces: Vec::new(),
            value: u32::try_from(bytes).ok().map(ParamValue::Count),
            limit: u32::try_from(limit).ok().map(ParamValue::Count),
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

fn clamped_module(values: &mut Values<'_>, module: f64) -> f64 {
    let (min, max) = values.length_range("module_size");
    let clamped = module.clamp(min, max);
    if (clamped - module).abs() > 1e-12 {
        let bound = if clamped > module {
            Bound::Minimum
        } else {
            Bound::Maximum
        };
        values.set_clamp(
            "module_size",
            ParamValue::Length(Length::from_mm(clamped)),
            bound,
        );
    }
    clamped
}

/// Module size and Width after the edit: Module size is the truth, Width is derived from it, unless
/// Width was the field edited; both are written back. Returns the module size in mm.
fn resolve_size(values: &mut Values<'_>, edit: Option<Edit<'_>>, width: usize, quiet: u32) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let total = (width + 2 * quiet as usize) as f64;
    let mut module = values.mm("module_size");
    match edit {
        Some(Edit::Scale(factor)) => module = clamped_module(values, module * factor.0),
        Some(Edit::Param("width")) => {
            let asked = values.mm("width") / total;
            module = clamped_module(values, asked);
            values.drop_clamp("module_size");
            let width = module * total;
            if (width - values.mm("width")).abs() > 1e-9 {
                let bound = if width > values.mm("width") {
                    Bound::DependentMinimum
                } else {
                    Bound::Dependent
                };
                values.set_clamp("width", ParamValue::Length(Length::from_mm(width)), bound);
            }
        }
        _ => {}
    }
    values.put_mm("module_size", module);
    values.put_mm("width", module * total);
    module
}

/// The modules the logo clear area clears, limited to what the level's budget allows; a limited
/// size or margin is written back and reported.
fn clear_area_of(values: &mut Values<'_>, grid: &ModuleGrid, level: EcLevel) -> Vec<bool> {
    let asked = ClearArea {
        shape: LogoShape::from_id(&values.choice("logo_shape")).unwrap_or(LogoShape::Circle),
        size: values.percent("logo_size"),
        margin: values.count("logo_margin"),
    };
    let area = clear_area::limited(grid, asked).unwrap_or(asked);
    if area.size != asked.size {
        let value = ParamValue::Percent(area.size);
        values.put("logo_size", &value);
        values.set_clamp("logo_size", value, Bound::DependentFor(subject(level)));
    }
    if area.margin != asked.margin {
        let value = ParamValue::Count(area.margin);
        values.put("logo_margin", &value);
        values.set_clamp("logo_margin", value, Bound::DependentFor(subject(level)));
    }
    clear_area::cleared(grid, area)
}

/// The style the values describe.
fn style_of(values: &Values<'_>, module: f64, quiet: u32, cleared: Vec<bool>) -> Style {
    Style {
        module: Length::from_mm(module),
        quiet,
        shape: ModuleShape::from_id(&values.choice("module_shape")).unwrap_or(ModuleShape::Square),
        rounding: f64::from(values.percent("rounding")) / 100.0,
        fill: f64::from(values.percent("module_fill")) / 100.0,
        finder_outer: FinderShape::from_id(&values.choice("finder_outer"))
            .unwrap_or(FinderShape::Square),
        finder_inner: FinderShape::from_id(&values.choice("finder_inner"))
            .unwrap_or(FinderShape::Square),
        merged: values.choice("output") != "separate",
        inverted: values.switch("inverted"),
        cleared,
    }
}

/// Generates the QR code `request` names.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = schema();
    let mut values = Values::read(&schema, request.params);
    let (grid, level) = match encoded(&mut values, request.edit) {
        Ok(found) => found,
        Err(error) => return refused(values, error),
    };
    let quiet = values.count("quiet_zone");
    let module = resolve_size(&mut values, request.edit, grid.width(), quiet);
    let cleared = if values.switch("logo") {
        clear_area_of(&mut values, &grid, level)
    } else {
        Vec::new()
    };
    let drawn = style_of(&values, module, quiet, cleared);
    let paths = if matches!(request.edit, Some(Edit::Draft)) {
        draft(&grid, &drawn)
    } else {
        style(&grid, &drawn)
    };
    let (params, clamps): (StoredParams, Vec<Clamp>) = values.finish();
    Generated {
        params,
        parts: vec![Part {
            key: Cow::Borrowed(PART),
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
