//! The typed entry of the selected fixture (`specs/0102-laser-fixtures` criteria 14, 18, 22, 23,
//! 38; feature `adrs.md` §6a.4): the chip a double-click on a handle or the keys M, R and S open,
//! its fields for the rectangle's chips, and its commit through the one resolver of `edit.rs`.

use curvyo_document_core::{Angle, Length, NodeId, Page, Point};
use curvyo_laser_core::{
    ScrewPart, StopPart, board_side_range, screw_head_range, screw_thread_range,
};
use curvyo_laser_core::{stop_leg_range, stop_thickness_range};
use curvyo_ui_core::{
    EntryField, EntryOutcome, InvalidReason, MoveEntry, ResizeDirection, format_degrees, format_mm,
    parse_entry_number,
};

use super::edit::{self, FixtureEdit};
use super::handles::{FixtureHandle, fixture_box, fixture_handles};
use super::params::{clamp_notice, number};
use super::placed::{PlacedFixture, Shape};

/// The S chip of a stop: both legs and the thickness.
const STOP_ALL: &[StopPart] = &[StopPart::LegA, StopPart::LegB, StopPart::Thickness];
/// The S chip of a screw: the head and the thread.
const SCREW_BOTH: &[ScrewPart] = &[ScrewPart::Head, ScrewPart::Thread];

/// Which chip is open.
#[derive(Debug, Clone)]
pub(super) enum Chip {
    /// W and H of a holder or clamp, from the squircle `direction`, or about the centre (S).
    Board(Option<ResizeDirection>),
    /// The angle, about the box centre.
    Angle,
    /// A stop's leg or thickness fields.
    Stop(&'static [StopPart]),
    /// A screw's diameter fields.
    Screw(&'static [ScrewPart]),
    /// The rectangle's Move chip, over the fixture's box.
    Move(Box<MoveEntry>),
}

/// An open typed entry of a fixture: the fixture as it was when it opened, the chip and its
/// fields.
#[derive(Debug, Clone)]
pub(in crate::session) struct FixtureEntry {
    pub id: NodeId,
    start: PlacedFixture,
    page: Page,
    pub chip: Chip,
    /// The handle the chip opened on; `None` for the keys S, R and M (by the centre).
    pub handle: Option<FixtureHandle>,
    fields: Vec<EntryField>,
}

/// The chip a double-click on `handle` opens.
fn chip_of_handle(handle: FixtureHandle, fixture: &PlacedFixture, page: &Page) -> Option<Chip> {
    Some(match handle {
        FixtureHandle::Resize(direction) => Chip::Board(Some(direction)),
        FixtureHandle::Rotate(_) => Chip::Angle,
        FixtureHandle::Move => Chip::Move(Box::new(move_entry(fixture, page))),
        FixtureHandle::Leg(StopPart::LegA) => Chip::Stop(&[StopPart::LegA]),
        FixtureHandle::Leg(StopPart::LegB) => Chip::Stop(&[StopPart::LegB]),
        FixtureHandle::Leg(StopPart::Thickness) | FixtureHandle::Thickness => {
            Chip::Stop(&[StopPart::Thickness])
        }
        FixtureHandle::Head => Chip::Screw(&[ScrewPart::Head]),
        FixtureHandle::Thread => Chip::Screw(&[ScrewPart::Thread]),
        FixtureHandle::Hold => return None,
    })
}

/// The chip the key `key` (`m`, `r` or `s`) opens on `fixture`; no R on a screw (D27).
fn chip_of_key(key: char, fixture: &PlacedFixture, page: &Page) -> Option<Chip> {
    let screw = matches!(fixture.shape, Shape::Screw { .. });
    match (key, &fixture.shape) {
        ('m', _) => Some(Chip::Move(Box::new(move_entry(fixture, page)))),
        ('r', _) if !screw => Some(Chip::Angle),
        ('s', Shape::Board { .. } | Shape::Clamp { .. }) => Some(Chip::Board(None)),
        ('s', Shape::Stop { .. }) => Some(Chip::Stop(STOP_ALL)),
        ('s', Shape::Screw { .. }) => Some(Chip::Screw(SCREW_BOTH)),
        _ => None,
    }
}

/// The Move chip over `fixture`'s box, measured from the page's origin corner.
fn move_entry(fixture: &PlacedFixture, page: &Page) -> MoveEntry {
    let corners = fixture.corners(page);
    let (mut min, mut max) = (corners[0], corners[0]);
    for c in corners {
        min = Point::new(min.x.min(c.x), min.y.min(c.y));
        max = Point::new(max.x.max(c.x), max.y.max(c.y));
    }
    let mut entry = MoveEntry::for_box(fixture_box(fixture, page), (min, max));
    entry.measure_from(page);
    entry
}

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
fn fields_of(chip: &Chip, fixture: &PlacedFixture) -> Vec<EntryField> {
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
    }
}

const fn stop_index(part: StopPart) -> usize {
    match part {
        StopPart::LegA => 0,
        StopPart::LegB => 1,
        StopPart::Thickness => 2,
    }
}

/// The typed values of `texts` against `fields`: `None` for an untouched field (its text is the
/// prefill), the number otherwise, or the first refused field.
fn typed(fields: &[EntryField], texts: &[&str]) -> Result<Vec<Option<f64>>, EntryOutcome> {
    fields
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(index, (field, text))| {
            if *text == field.prefill {
                return Ok(None);
            }
            parse_entry_number(text, true)
                .map(Some)
                .ok_or(EntryOutcome::Invalid {
                    field: index,
                    reason: InvalidReason::NotANumber,
                })
        })
        .collect()
}

impl FixtureEntry {
    /// The entry of `chip` on `fixture`.
    fn new(
        fixture: &PlacedFixture,
        page: &Page,
        chip: Chip,
        handle: Option<FixtureHandle>,
    ) -> Self {
        Self {
            id: fixture.id,
            fields: fields_of(&chip, fixture),
            start: fixture.clone(),
            page: page.clone(),
            chip,
            handle,
        }
    }

    /// The entry a double-click on `handle` of `fixture` opens, if any.
    pub(super) fn at_handle(
        fixture: &PlacedFixture,
        page: &Page,
        handle: FixtureHandle,
    ) -> Option<Self> {
        let chip = chip_of_handle(handle, fixture, page)?;
        Some(Self::new(fixture, page, chip, Some(handle)))
    }

    /// The entry the key `key` opens on `fixture`, if any.
    pub(super) fn for_key(fixture: &PlacedFixture, page: &Page, key: char) -> Option<Self> {
        if !super::commands::movable(fixture) {
            return None;
        }
        let chip = chip_of_key(key, fixture, page)?;
        Some(Self::new(fixture, page, chip, None))
    }

    /// The fields as the chip shows them.
    pub(super) fn fields(&self) -> &[EntryField] {
        &self.fields
    }

    /// The fixture as it was when the entry opened.
    pub(super) const fn start(&self) -> &PlacedFixture {
        &self.start
    }

    /// The page of the fixture.
    pub(super) const fn start_page(&self) -> &Page {
        &self.page
    }

    /// The box centre and the anchor of the chip in document space; the anchor is the handle's
    /// position, or `None` for a chip by the centre.
    pub(super) fn anchor(
        &self,
        tolerances: &curvyo_ui_core::TransformHandleTolerances,
    ) -> (Point, Option<Point>) {
        let box_ = fixture_box(&self.start, &self.page);
        let centre = box_.to_document(box_.local_center());
        let handle = self.handle.and_then(|h| {
            fixture_handles(&self.start, &self.page, tolerances, true)
                .into_iter()
                .find(|(other, _)| *other == h)
                .map(|(_, at)| at)
        });
        (centre, handle)
    }

    /// The edit the typed `texts` mean, the label of its step and its notice; `Err` with the
    /// outcome when nothing is to be written (untouched, or a refused field).
    pub(super) fn resolve(
        &self,
        texts: &[&str],
        absolute: bool,
    ) -> Result<(FixtureEdit, &'static str, Option<String>), EntryOutcome> {
        use curvyo_document_core::{
            MOVE_FIXTURE_COMMIT_LABEL as MOVE, RESIZE_FIXTURE_COMMIT_LABEL as RESIZE,
            ROTATE_FIXTURE_COMMIT_LABEL as ROTATE,
        };
        if let Chip::Move(entry) = &self.chip {
            let pair = [
                texts.first().copied().unwrap_or(""),
                texts.get(1).copied().unwrap_or(""),
            ];
            return match entry.resolve(pair, absolute) {
                Err((field, reason)) => Err(EntryOutcome::Invalid { field, reason }),
                Ok(None) => Err(EntryOutcome::Unchanged),
                Ok(Some(offset)) => Ok((edit::translate(&self.start, offset), MOVE, None)),
            };
        }
        let values = typed(&self.fields, texts)?;
        if values.iter().all(Option::is_none) {
            return Err(EntryOutcome::Unchanged);
        }
        let resolved = match &self.chip {
            Chip::Board(direction) => self.board(*direction, &values),
            Chip::Angle => values[0].map(|degrees| (self.angle(degrees), None)),
            Chip::Stop(parts) => self.stop(parts, &values),
            Chip::Screw(parts) => self.screw(parts, &values),
            Chip::Move(_) => None,
        };
        let (edit, notice) = resolved.ok_or(EntryOutcome::Unchanged)?;
        let label = if matches!(self.chip, Chip::Angle) {
            ROTATE
        } else {
            RESIZE
        };
        Ok((edit, label, notice))
    }

    fn board(
        &self,
        direction: Option<ResizeDirection>,
        values: &[Option<f64>],
    ) -> Option<(FixtureEdit, Option<String>)> {
        let (w0, h0) = board_values(&self.start)?;
        let typed_for = |label: &str| {
            self.fields
                .iter()
                .zip(values)
                .find(|(field, _)| field.label == label)
                .and_then(|(_, value)| *value)
        };
        let w = typed_for("W").unwrap_or(w0);
        let h = typed_for("H").unwrap_or(h0);
        let (edit, moved) = edit::board_sized(
            &self.start,
            direction,
            Length::from_mm(w),
            Length::from_mm(h),
        );
        let range = board_side_range(1.0);
        let [w, h] = board_values_of(&edit);
        let (min, max) = (range.min.as_mm(), range.max.as_mm());
        // Both set to a limit: one notice names both, within 70 characters.
        let notice = match moved {
            [true, true] => Some(format!(
                "Width and height are {} to {} mm. Set to {} and {} mm.",
                number(min),
                number(max),
                number(w),
                number(h)
            )),
            [true, false] => Some(clamp_notice("Width", min, max, "mm", w)),
            [false, true] => Some(clamp_notice("Height", min, max, "mm", h)),
            [false, false] => None,
        };
        Some((edit, notice))
    }

    fn angle(&self, degrees: f64) -> FixtureEdit {
        let now = self.start.matrix.b.atan2(self.start.matrix.a);
        let (min, max) = self.start.local_box(&self.page);
        let pivot = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        edit::rotate(
            &self.start,
            pivot,
            Angle::from_radians(degrees.to_radians() - now),
        )
    }

    fn stop(
        &self,
        parts: &[StopPart],
        values: &[Option<f64>],
    ) -> Option<(FixtureEdit, Option<String>)> {
        let mut current = self.start.clone();
        let mut out = None;
        let mut notice = None;
        for (part, value) in parts.iter().zip(values) {
            let Some(value) = value else { continue };
            let Some([a, b, t]) = edit::stop_sizes(&current) else {
                break;
            };
            let (edit, moved) = edit::stop_set(&current, *part, Length::from_mm(*value));
            if moved && notice.is_none() {
                let set = edit::stop_sizes(&current.with_edit(&self.page, &edit))
                    .map_or(0.0, |s| s[stop_index(*part)].as_mm());
                notice = Some(match part {
                    StopPart::Thickness => {
                        let max = a
                            .as_mm()
                            .min(b.as_mm())
                            .min(stop_thickness_range().max.as_mm());
                        clamp_notice("Thickness", 0.5, max, "mm", set)
                    }
                    StopPart::LegA | StopPart::LegB => {
                        let word = if *part == StopPart::LegA {
                            "Leg 1"
                        } else {
                            "Leg 2"
                        };
                        let min = t.as_mm().max(stop_leg_range().min.as_mm());
                        clamp_notice(word, min, stop_leg_range().max.as_mm(), "mm", set)
                    }
                });
            }
            current = current.with_edit(&self.page, &edit);
            out = Some(edit);
        }
        out.map(|edit| (edit, notice))
    }

    fn screw(
        &self,
        parts: &[ScrewPart],
        values: &[Option<f64>],
    ) -> Option<(FixtureEdit, Option<String>)> {
        let mut current = self.start.clone();
        let mut out = None;
        let mut notice = None;
        for (part, value) in parts.iter().zip(values) {
            let Some(value) = value else { continue };
            let Some((thread, head)) = edit::screw_sizes(&current) else {
                break;
            };
            let (edit, moved) = edit::screw_set(&current, *part, Length::from_mm(*value));
            if moved && notice.is_none() {
                let (t, h) = edit::screw_sizes(&current.with_edit(&self.page, &edit))
                    .unwrap_or((thread, head));
                notice = Some(match part {
                    ScrewPart::Thread => clamp_notice(
                        "Thread diameter",
                        screw_thread_range().min.as_mm(),
                        head.as_mm(),
                        "mm",
                        t.as_mm(),
                    ),
                    ScrewPart::Head => clamp_notice(
                        "Head diameter",
                        thread.as_mm().max(screw_head_range().min.as_mm()),
                        screw_head_range().max.as_mm(),
                        "mm",
                        h.as_mm(),
                    ),
                });
            }
            current = current.with_edit(&self.page, &edit);
            out = Some(edit);
        }
        out.map(|edit| (edit, notice))
    }
}

/// The width and height of a holder or clamp in mm.
fn board_values(fixture: &PlacedFixture) -> Option<(f64, f64)> {
    match fixture.shape {
        Shape::Board { width, height } | Shape::Clamp { width, height } => {
            Some((width.as_mm(), height.as_mm()))
        }
        _ => None,
    }
}

/// The width and height an edit of a board leaves, as `[w, h]`.
fn board_values_of(edit: &FixtureEdit) -> [f64; 2] {
    match edit.shape {
        Shape::Board { width, height } | Shape::Clamp { width, height } => {
            [width.as_mm(), height.as_mm()]
        }
        _ => [0.0, 0.0],
    }
}
