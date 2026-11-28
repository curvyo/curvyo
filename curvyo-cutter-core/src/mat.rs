//! The Cricut mats as fixtures of a plotter page (`specs/0159-cutting-plotter` criteria 3 to 5,
//! `ux-notes.md` 5.1, `adrs.md` PL3, ADR 0098 decision 1): their sizes, grips and the outlines of
//! sheet, printed field, cut area and notch in mat mm (the mat origin at the field's top-left
//! corner); the page is the cut area, [`PAGE_ORIGIN_ON_MAT`] from the mat origin.

use curvyo_document_core::{DocumentSize, FixtureParams, Length, Point};
use serde_json::Value;

/// One inch in mm.
pub const INCH_MM: f64 = 25.4;
/// The inset of the cut area inside the printed field on every side (0.25 in, UX C6).
pub const CUT_INSET_MM: f64 = 6.35;
/// The sheet margin left and right of the field (0.5 in, UX C5).
pub const SHEET_SIDE_MM: f64 = 12.7;
/// The sheet margin above and below the field (1 in, UX C5).
pub const SHEET_END_MM: f64 = 25.4;
/// The sheet's corner radius.
pub const SHEET_RADIUS_MM: f64 = 3.0;

/// Where the page origin (the cut area's top-left corner) sits in mat mm: the only place that
/// knows the offset between page and mat (ADR 0098 decision 1).
pub const PAGE_ORIGIN_ON_MAT: Point = Point {
    x: CUT_INSET_MM,
    y: CUT_INSET_MM,
};

/// A point in page mm of a mat page in mat mm.
#[must_use]
pub fn page_to_mat(p: Point) -> Point {
    Point::new(p.x + PAGE_ORIGIN_ON_MAT.x, p.y + PAGE_ORIGIN_ON_MAT.y)
}

/// A point in mat mm in page mm of a mat page.
#[must_use]
pub fn mat_to_page(p: Point) -> Point {
    Point::new(p.x - PAGE_ORIGIN_ON_MAT.x, p.y - PAGE_ORIGIN_ON_MAT.y)
}

/// The fixture param of the grip.
pub const PARAM_GRIP: &str = "grip";
/// The fixture param of the Mat grid switch.
pub const PARAM_GRID: &str = "grid";

/// A mat size of the Maker 3 (criterion 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MatSize {
    /// 12 x 12 in, the default.
    #[default]
    In12x12,
    /// 12 x 24 in.
    In12x24,
}

impl MatSize {
    /// Every size, in pick order.
    pub const ALL: [Self; 2] = [Self::In12x12, Self::In12x24];

    /// The stored spelling inside a profile ("12x12").
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::In12x12 => "12x12",
            Self::In12x24 => "12x24",
        }
    }

    /// The size a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.key() == key)
    }

    /// The stored fixture kind (`curvyo.cutter/mat-12x12`, PL3).
    #[must_use]
    pub const fn fixture_kind(self) -> &'static str {
        match self {
            Self::In12x12 => "curvyo.cutter/mat-12x12",
            Self::In12x24 => "curvyo.cutter/mat-12x24",
        }
    }

    /// The size a stored fixture kind names, `None` for another kind.
    #[must_use]
    pub fn of_fixture_kind(kind: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|size| size.fixture_kind() == kind)
    }

    /// Whether a stored fixture kind is a mat of this family, known or not.
    #[must_use]
    pub fn is_mat_kind(kind: &str) -> bool {
        kind.starts_with("curvyo.cutter/mat-")
    }

    /// The segment's label ("12 × 12 in").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::In12x12 => "12 × 12 in",
            Self::In12x24 => "12 × 24 in",
        }
    }

    /// The field's width and height in whole inches.
    #[must_use]
    pub const fn inches(self) -> (u32, u32) {
        match self {
            Self::In12x12 => (12, 12),
            Self::In12x24 => (12, 24),
        }
    }

    /// The printed field from the mat origin.
    #[must_use]
    pub fn field(self) -> Rect {
        match self {
            Self::In12x12 => Rect::new(0.0, 0.0, 304.8, 304.8),
            Self::In12x24 => Rect::new(0.0, 0.0, 304.8, 609.6),
        }
    }

    /// The size of the printed field: the size the Design Space SVG declares.
    #[must_use]
    pub fn field_size(self) -> DocumentSize {
        let field = self.field();
        DocumentSize::from_mm(field.w, field.h)
    }

    /// The page size: the cut area (ADR 0098 decision 1).
    #[must_use]
    pub fn page_size(self) -> DocumentSize {
        let cut = self.cut_area();
        DocumentSize::from_mm(cut.w, cut.h)
    }

    /// The machine's cut area, 0.25 in inside the field on every side (criterion 4 c).
    #[must_use]
    pub fn cut_area(self) -> Rect {
        let field = self.field();
        Rect::new(
            CUT_INSET_MM,
            CUT_INSET_MM,
            field.w - 2.0 * CUT_INSET_MM,
            field.h - 2.0 * CUT_INSET_MM,
        )
    }

    /// The whole sheet around the field (UX 5.1).
    #[must_use]
    pub fn sheet(self) -> Rect {
        let field = self.field();
        Rect::new(
            -SHEET_SIDE_MM,
            -SHEET_END_MM,
            field.w + 2.0 * SHEET_SIDE_MM,
            field.h + 2.0 * SHEET_END_MM,
        )
    }

    /// The notch, a triangular hole in the bottom margin (criterion 4 a, UX M2): top edge 29 mm
    /// wide centred, 9 mm below the field, apex 18 mm below; left top, right top, apex.
    #[must_use]
    pub fn notch(self) -> [Point; 3] {
        let field = self.field();
        let cx = field.w / 2.0;
        let top = field.h + 9.0;
        [
            Point::new(cx - 14.5, top),
            Point::new(cx + 14.5, top),
            Point::new(cx, field.h + 18.0),
        ]
    }

    /// The centre of the field, where the printed cross sits (criterion 5).
    #[must_use]
    pub fn centre(self) -> Point {
        let field = self.field();
        Point::new(field.w / 2.0, field.h / 2.0)
    }
}

/// An axis-aligned rectangle in page mm, Y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
}

impl Rect {
    /// The rectangle from `(x, y)` of size `w` x `h`.
    #[must_use]
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// The width as a length.
    #[must_use]
    pub const fn width(self) -> Length {
        Length::from_mm(self.w)
    }

    /// The height as a length.
    #[must_use]
    pub const fn height(self) -> Length {
        Length::from_mm(self.h)
    }

    /// Whether the box from `min` to `max` lies inside, edges included, within `tolerance_mm`.
    #[must_use]
    pub fn holds(self, min: Point, max: Point, tolerance_mm: f64) -> bool {
        min.x >= self.x - tolerance_mm
            && min.y >= self.y - tolerance_mm
            && max.x <= self.x + self.w + tolerance_mm
            && max.y <= self.y + self.h + tolerance_mm
    }
}

/// The colour of a mat (criterion 3): the output never changes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Grip {
    /// `LightGrip`, blue.
    Light,
    /// `StandardGrip`, green, the default.
    #[default]
    Standard,
    /// `StrongGrip`, purple.
    Strong,
    /// `FabricGrip`, pink.
    Fabric,
}

impl Grip {
    /// Every grip, in pick order.
    pub const ALL: [Self; 4] = [Self::Light, Self::Standard, Self::Strong, Self::Fabric];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Standard => "standard",
            Self::Strong => "strong",
            Self::Fabric => "fabric",
        }
    }

    /// The grip a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|grip| grip.key() == key)
    }

    /// The word printed on the mat and shown in the pick (`StandardGrip`).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Light => "LightGrip",
            Self::Standard => "StandardGrip",
            Self::Strong => "StrongGrip",
            Self::Fabric => "FabricGrip",
        }
    }

    /// The canvas colour (UX 5.4, `--mat-*grip`), as `0xRRGGBB`.
    #[must_use]
    pub const fn colour(self) -> u32 {
        match self {
            Self::Light => 0x008F_B3CC,
            Self::Standard => 0x009C_B87D,
            Self::Strong => 0x00B6_A7DB,
            Self::Fabric => 0x00D6_A1B7,
        }
    }
}

/// A mat fixture as read: its size, grip, grid switch and how many values were damaged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatRead {
    /// The size; an unknown mat kind reads as 12 x 12 in (PL3).
    pub size: MatSize,
    /// The grip.
    pub grip: Grip,
    /// Whether the printing is drawn (criterion 5, default on).
    pub grid: bool,
    /// How many values read as their default.
    pub damaged: usize,
}

impl MatRead {
    /// The mat a fixture of stored kind `kind` with `params` holds; damage reads as the default
    /// with a count, nothing is rewritten (PL3).
    #[must_use]
    pub fn of(kind: &str, params: &FixtureParams) -> Self {
        let mut damaged = 0;
        let size = MatSize::of_fixture_kind(kind).unwrap_or_else(|| {
            damaged += 1;
            MatSize::default()
        });
        let grip = match params.get(PARAM_GRIP) {
            None => Grip::default(),
            Some(value) => value.as_str().and_then(Grip::parse).unwrap_or_else(|| {
                damaged += 1;
                Grip::default()
            }),
        };
        let printing = match params.get(PARAM_GRID) {
            None => true,
            Some(value) => value.as_bool().unwrap_or_else(|| {
                damaged += 1;
                true
            }),
        };
        Self {
            size,
            grip,
            grid: printing,
            damaged,
        }
    }
}

/// The params of a mat fixture with `grip` and the Mat grid switch `printing`.
#[must_use]
pub fn mat_params(grip: Grip, printing: bool) -> FixtureParams {
    let mut params = FixtureParams::new();
    params.insert(PARAM_GRIP.to_owned(), Value::from(grip.key()));
    params.insert(PARAM_GRID.to_owned(), Value::from(printing));
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 0.01;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= TOL
    }

    #[test]
    fn the_12x12_field_cut_area_and_centre_are_the_published_values() {
        let mat = MatSize::In12x12;
        let field = mat.field();
        assert!(near(field.w, 304.8) && near(field.h, 304.8));
        let cut = mat.cut_area();
        assert!(near(cut.x, 6.35) && near(cut.y, 6.35), "criterion 4");
        assert!(near(cut.w, 292.1) && near(cut.h, 292.1));
        let centre = mat.centre();
        assert!(
            near(centre.x, 152.4) && near(centre.y, 152.4),
            "criterion 5"
        );
    }

    #[test]
    fn the_page_is_the_cut_area_and_counts_from_its_corner_on_the_mat() {
        let page = MatSize::In12x12.page_size();
        assert!(near(page.width.as_mm(), 292.1) && near(page.height.as_mm(), 292.1));
        let page = MatSize::In12x24.page_size();
        assert!(near(page.width.as_mm(), 292.1) && near(page.height.as_mm(), 596.9));
        let on_mat = page_to_mat(Point::new(0.0, 0.0));
        let cut = MatSize::In12x12.cut_area();
        assert!(near(on_mat.x, cut.x) && near(on_mat.y, cut.y));
        let back = mat_to_page(page_to_mat(Point::new(10.0, 20.0)));
        assert!(near(back.x, 10.0) && near(back.y, 20.0));
    }

    #[test]
    fn the_12x24_mat_is_twice_as_long() {
        let mat = MatSize::In12x24;
        assert_eq!(mat.field_size(), DocumentSize::from_mm(304.8, 609.6));
        let cut = mat.cut_area();
        assert!(near(cut.w, 292.1) && near(cut.h, 596.9));
        let sheet = mat.sheet();
        assert!(near(sheet.w, 330.2) && near(sheet.h, 660.4));
        let notch = mat.notch();
        assert!(near(notch[2].y, 609.6 + 18.0) && near(notch[2].x, 152.4));
    }

    #[test]
    fn a_damaged_mat_reads_as_the_default_with_a_count() {
        let mut params = mat_params(Grip::Strong, false);
        let read = MatRead::of("curvyo.cutter/mat-12x24", &params);
        assert_eq!(
            (read.size, read.grip, read.grid, read.damaged),
            (MatSize::In12x24, Grip::Strong, false, 0)
        );
        params.insert(PARAM_GRIP.into(), Value::from("sticky"));
        params.insert(PARAM_GRID.into(), Value::from(3));
        let read = MatRead::of("curvyo.cutter/mat-9x9", &params);
        assert_eq!(
            (read.size, read.grip, read.grid, read.damaged),
            (MatSize::In12x12, Grip::Standard, true, 3)
        );
        assert!(MatSize::is_mat_kind("curvyo.cutter/mat-9x9"));
    }

    #[test]
    fn the_grip_changes_the_colour_only() {
        let colours: Vec<u32> = Grip::ALL.iter().map(|g| g.colour()).collect();
        assert_eq!(colours, [0x8F_B3CC, 0x9C_B87D, 0xB6_A7DB, 0xD6_A1B7]);
        assert_eq!(Grip::default().word(), "StandardGrip");
    }
}
