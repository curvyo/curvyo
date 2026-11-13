//! Where a test grid fits on the bed: the bed-fit cause and the first free spot (`specs/0028`
//! criteria 2 and 3, `specs/0112` criteria 2 and 5).

use curvyo_document_core::{DocumentSize, Length, Point};

/// The tolerance of every comparison of a grid's size and place, in mm.
pub const GRID_TOLERANCE_MM: f64 = 1e-3;

/// Whether a grid of `width` x `height` fits the bed `bed`; else the cause of criterion 2
/// ("Grid is 260 mm wide, the bed is 200 mm.").
///
/// # Errors
/// The cause text.
pub fn fit_bed(width: Length, height: Length, bed: DocumentSize) -> Result<(), String> {
    let too = |what: &str, grid: Length, bed: Length| {
        format!(
            "Grid is {} mm {what}, the bed is {} mm.",
            mm_text(grid),
            mm_text(bed)
        )
    };
    if width.as_mm() > bed.width.as_mm() + GRID_TOLERANCE_MM {
        return Err(too("wide", width, bed.width));
    }
    if height.as_mm() > bed.height.as_mm() + GRID_TOLERANCE_MM {
        return Err(too("high", height, bed.height));
    }
    Ok(())
}

/// Whether a grid of `width` x `height` whose top-left lies at `at` (bed coordinates) stays on
/// the bed `bed`: [`fit_bed`]'s cause first, else the edge it runs past and by how much
/// ("Grid runs 32 mm past the bed's right edge."; criterion 5, Proposal).
///
/// # Errors
/// The cause text.
pub fn fit_bed_at(
    at: Point,
    width: Length,
    height: Length,
    bed: DocumentSize,
) -> Result<(), String> {
    fit_bed(width, height, bed)?;
    let overhangs = [
        (at.x + width.as_mm() - bed.width.as_mm(), "right"),
        (at.y + height.as_mm() - bed.height.as_mm(), "bottom"),
        (-at.x, "left"),
        (-at.y, "top"),
    ];
    match overhangs
        .into_iter()
        .find(|(over, _)| *over > GRID_TOLERANCE_MM)
    {
        Some((over, edge)) => Err(format!(
            "Grid runs {} mm past the bed's {edge} edge.",
            mm_text(Length::from_mm(over))
        )),
        None => Ok(()),
    }
}

/// The refusal of the add button when no spot is free (UX 4).
#[must_use]
pub fn no_free_spot_text(width: Length, height: Length) -> String {
    format!(
        "No free spot on the bed for a {} × {} mm grid.",
        mm_text(width),
        mm_text(height)
    )
}

/// A length in mm as the grid texts write it: whole when it is, else one decimal, rounded up so
/// a grid never reads as fitting when it does not.
fn mm_text(length: Length) -> String {
    let tenths = (length.as_mm() * 10.0 - GRID_TOLERANCE_MM).ceil() / 10.0;
    if tenths.fract().abs() < 1e-9 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

/// The first free spot for a grid of `width` x `height` on the bed `bed` (bed coordinates, top
/// left at 0, 0): the top-left corner of the bed, else the first place found left to right, top
/// to bottom where it touches none of the `occupied` boxes (min, max) (criterion 3). `None` when
/// none is free.
#[must_use]
pub fn free_spot(
    width: Length,
    height: Length,
    bed: DocumentSize,
    occupied: &[(Point, Point)],
) -> Option<Point> {
    let (w, h) = (width.as_mm(), height.as_mm());
    let mut xs: Vec<f64> = std::iter::once(0.0)
        .chain(occupied.iter().map(|(_, max)| max.x))
        .collect();
    let mut ys: Vec<f64> = std::iter::once(0.0)
        .chain(occupied.iter().map(|(_, max)| max.y))
        .collect();
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    let fits = |x: f64, y: f64| {
        x >= -GRID_TOLERANCE_MM
            && y >= -GRID_TOLERANCE_MM
            && x + w <= bed.width.as_mm() + GRID_TOLERANCE_MM
            && y + h <= bed.height.as_mm() + GRID_TOLERANCE_MM
            && occupied.iter().all(|(min, max)| {
                x + w <= min.x + GRID_TOLERANCE_MM
                    || max.x <= x + GRID_TOLERANCE_MM
                    || y + h <= min.y + GRID_TOLERANCE_MM
                    || max.y <= y + GRID_TOLERANCE_MM
            })
    };
    ys.iter()
        .flat_map(|&y| xs.iter().map(move |&x| (x, y)))
        .find(|&(x, y)| fits(x, y))
        .map(|(x, y)| Point::new(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bed_fit_names_the_side_that_does_not_fit() {
        let bed = DocumentSize::from_mm(200.0, 100.0);
        assert!(fit_bed(Length::from_mm(200.0), Length::from_mm(100.0), bed).is_ok());
        assert_eq!(
            fit_bed(Length::from_mm(260.0), Length::from_mm(50.0), bed).unwrap_err(),
            "Grid is 260 mm wide, the bed is 200 mm."
        );
        assert_eq!(
            fit_bed(Length::from_mm(50.0), Length::from_mm(100.04), bed).unwrap_err(),
            "Grid is 100.1 mm high, the bed is 100 mm."
        );
        assert_eq!(
            no_free_spot_text(Length::from_mm(75.0), Length::from_mm(62.0)),
            "No free spot on the bed for a 75 × 62 mm grid."
        );
    }

    #[test]
    fn the_bed_fit_at_a_place_names_the_edge_the_grid_runs_past() {
        let bed = DocumentSize::from_mm(200.0, 100.0);
        let (w, h) = (Length::from_mm(80.0), Length::from_mm(40.0));
        assert!(fit_bed_at(Point::new(120.0, 60.0), w, h, bed).is_ok());
        assert_eq!(
            fit_bed_at(Point::new(152.0, 0.0), w, h, bed).unwrap_err(),
            "Grid runs 32 mm past the bed's right edge."
        );
        assert_eq!(
            fit_bed_at(Point::new(0.0, 70.5), w, h, bed).unwrap_err(),
            "Grid runs 10.5 mm past the bed's bottom edge."
        );
        assert_eq!(
            fit_bed_at(Point::new(-3.0, 0.0), w, h, bed).unwrap_err(),
            "Grid runs 3 mm past the bed's left edge."
        );
        assert_eq!(
            fit_bed_at(Point::new(0.0, 0.0), Length::from_mm(260.0), h, bed).unwrap_err(),
            "Grid is 260 mm wide, the bed is 200 mm.",
            "the size cause first"
        );
    }

    #[test]
    fn the_free_spot_is_the_corner_else_the_first_left_to_right_top_to_bottom() {
        let bed = DocumentSize::from_mm(200.0, 100.0);
        let (w, h) = (Length::from_mm(50.0), Length::from_mm(40.0));
        assert_eq!(free_spot(w, h, bed, &[]), Some(Point::new(0.0, 0.0)));
        let left = (Point::new(0.0, 0.0), Point::new(60.0, 30.0));
        assert_eq!(free_spot(w, h, bed, &[left]), Some(Point::new(60.0, 0.0)));
        let wide = (Point::new(0.0, 0.0), Point::new(180.0, 30.0));
        assert_eq!(free_spot(w, h, bed, &[wide]), Some(Point::new(0.0, 30.0)));
        let full = (Point::new(0.0, 0.0), Point::new(200.0, 70.0));
        assert_eq!(free_spot(w, h, bed, &[full]), None);
    }
}
