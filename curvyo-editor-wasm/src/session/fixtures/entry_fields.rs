//! The prefilled fields of each chip of a fixture's typed entry (`specs/0102-laser-fixtures`
//! criteria 14, 18, 22, 23; `0113` criterion 21).

use curvyo_document_core::Length;
use curvyo_laser_core::{CentreGrid, ScrewPart, StopPart};
use curvyo_ui_core::{EntryField, ResizeDirection, format_degrees, format_mm};

use super::edit;
use super::entry::Chip;
use super::grid_handles::GridValue;
use super::placed::PlacedFixture;
use super::shape::Shape;

fn stop_field(part: StopPart, value: Length) -> EntryField {
    let (label, name) = match part {
        StopPart::LegA => ("Leg 1", "Leg 1 (mm)"),
        StopPart::LegB => ("Leg 2", "Leg 2 (mm)"),
        StopPart::Thickness => ("Thickness", "Thickness (mm)"),
    };
    EntryField::for_param(label, name, format_mm(value.as_mm()))
}

fn screw_field(part: ScrewPart, value: Length) -> EntryField {
    let (label, name) = match part {
        ScrewPart::Head => ("Head", "Head diameter (mm)"),
        ScrewPart::Thread => ("Thread", "Thread diameter (mm)"),
    };
    EntryField::for_param(label, name, format_mm(value.as_mm()))
}

/// The fields of `chip` on `fixture`, prefilled with the values as they are.
pub(super) fn fields_of(chip: &Chip, fixture: &PlacedFixture) -> Vec<EntryField> {
    match chip {
        Chip::Board(direction) => match fixture.shape {
            Shape::Board { width, height } | Shape::Clamp { width, height } => {
                let w = EntryField::for_param("W", "Width (mm)", format_mm(width.as_mm()));
                let h = EntryField::for_param("H", "Height (mm)", format_mm(height.as_mm()));
                // An edge squircle opens its one axis, a corner and S both (the rectangle's).
                match direction {
                    Some(ResizeDirection::N | ResizeDirection::S) => vec![h],
                    Some(ResizeDirection::E | ResizeDirection::W) => vec![w],
                    _ => vec![w, h],
                }
            }
            _ => Vec::new(),
        },
        // The chip shows the "°" as its unit: the prefill is the number alone, as the
        // rectangle's.
        Chip::Angle => vec![EntryField::for_param(
            "",
            "Angle (°)",
            format_degrees(fixture.matrix.b.atan2(fixture.matrix.a).to_degrees())
                .trim_end_matches('°')
                .to_owned(),
        )],
        Chip::Stop(parts) => edit::stop_sizes(fixture).map_or_else(Vec::new, |sizes| {
            parts
                .iter()
                .map(|part| stop_field(*part, sizes[stop_index(*part)]))
                .collect()
        }),
        Chip::Screw(parts) => edit::screw_sizes(fixture).map_or_else(Vec::new, |(thread, head)| {
            parts
                .iter()
                .map(|part| match part {
                    ScrewPart::Head => screw_field(*part, head),
                    ScrewPart::Thread => screw_field(*part, thread),
                })
                .collect()
        }),
        Chip::Move(entry) => entry.fields().to_vec(),
        Chip::Grid(values) => fixture.shape.centre_grid().map_or_else(Vec::new, |grid| {
            values
                .iter()
                .map(|value| grid_field(*value, &fixture.shape, grid))
                .collect()
        }),
    }
}

/// The field of one grid value, prefilled with its value as it is.
fn grid_field(value: GridValue, shape: &Shape, grid: CentreGrid) -> EntryField {
    let mm = |l: Length| format_mm(l.as_mm());
    match value {
        GridValue::Diameter => {
            let diameter = match shape {
                Shape::Pin { diameter, .. } => *diameter,
                _ => Length::from_mm(0.0),
            };
            EntryField::for_param("⌀", "Diameter (mm)", mm(diameter))
        }
        GridValue::Columns => {
            EntryField::for_param("Columns", "Columns", grid.counts.columns().to_string())
        }
        GridValue::Rows => EntryField::for_param("Rows", "Rows", grid.counts.rows().to_string()),
        GridValue::ColumnSpacing => {
            EntryField::for_param("Spacing", "Column spacing (mm)", mm(grid.column_spacing))
        }
        GridValue::RowSpacing => {
            EntryField::for_param("Spacing", "Row spacing (mm)", mm(grid.row_spacing))
        }
        GridValue::Mark => EntryField::for_param("Size", "Size (mm)", mm(grid.mark)),
    }
}

/// The index of `part` in the sizes of `edit::stop_sizes`.
pub(super) const fn stop_index(part: StopPart) -> usize {
    match part {
        StopPart::LegA => 0,
        StopPart::LegB => 1,
        StopPart::Thickness => 2,
    }
}
