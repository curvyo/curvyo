//! The sentences of a generator's warnings (ADR 0083 §11, UX notes `0149` §6): the generator names
//! facts, the shell words them. Several warnings of one id become one plural sentence. Lengths are
//! shown in millimetres.

use curvyo_extension_core::Warning;

fn pieces(numbers: &[u32]) -> String {
    match numbers {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!(
            "{} and {last}",
            rest.iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn mm(warning: &Warning, limit: bool) -> Option<f64> {
    let value = if limit {
        &warning.limit
    } else {
        &warning.value
    };
    value
        .as_ref()?
        .length()
        .map(curvyo_document_core::Length::as_mm)
}

fn number(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

fn sharp(group: &[&Warning], one_piece: bool) -> String {
    if let [one] = group {
        let angle = one
            .value
            .as_ref()
            .and_then(curvyo_extension_core::ParamValue::degrees)
            .unwrap_or(0.0);
        if one_piece {
            return format!("The corner of the wall is very sharp: {}°.", number(angle));
        }
        return format!(
            "The corner between pieces {} is very sharp: {}°.",
            pieces(&one.pieces),
            number(angle)
        );
    }
    let between: Vec<String> = group.iter().map(|w| pieces(&w.pieces)).collect();
    format!(
        "{} corners are very sharp: between pieces {}.",
        group.len(),
        pieces_list(&between)
    )
}

fn pieces_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// A bend too short for two slits at the least gap: built anyway.
fn short(group: &[&Warning], one_piece: bool) -> String {
    let at = |w: &Warning| number(mm(w, false).unwrap_or(0.0));
    match group {
        [one] if one_piece => format!(
            "The wall has a bend too short for two slits ({} mm); it may crack there.",
            at(one)
        ),
        [one] => format!(
            "Piece {} has a bend too short for two slits ({} mm); it may crack there.",
            pieces(&one.pieces),
            at(one)
        ),
        _ => {
            let all: Vec<u32> = group.iter().flat_map(|w| w.pieces.clone()).collect();
            format!(
                "Pieces {} have a bend too short for two slits; they may crack there.",
                pieces(&all)
            )
        }
    }
}

/// Tabs that kept their flat window although Bend through tabs asks for the arc.
fn windows(group: &[&Warning]) -> String {
    let count: u32 = group
        .iter()
        .filter_map(|w| {
            w.value
                .as_ref()
                .and_then(curvyo_extension_core::ParamValue::count)
        })
        .sum();
    if count == 1 {
        "1 tab does not fit on one arc, so it stands in a flat window.".to_owned()
    } else {
        format!("{count} tabs do not fit on one arc, so they stand in flat windows.")
    }
}

/// One sentence per limit: convex stretches ("bends to"), concave ones ("bends inward").
fn tight(group: &[&Warning], context: Context) -> Vec<String> {
    let one_piece = context.one_piece;
    let mut limits: Vec<f64> = Vec::new();
    for w in group {
        let limit = mm(w, true).unwrap_or(0.0);
        if !limits.iter().any(|l| (l - limit).abs() < 1e-9) {
            limits.push(limit);
        }
    }
    limits
        .into_iter()
        .map(|limit| {
            let same: Vec<&&Warning> = group
                .iter()
                .filter(|w| (mm(w, true).unwrap_or(0.0) - limit).abs() < 1e-9)
                .collect();
            // ADR 0083 section 13.12: a convex stretch's limit is twice the wall thickness, a concave
            // one's the wall thickness itself; no other limit exists, so equality within 1e-9 mm
            // (the limits are computed from the same thickness) names the concave kind.
            let inward = (limit - context.wall.as_mm()).abs() < 1e-9;
            let verb = if inward { "bends inward" } else { "bends" };
            if let [one] = same.as_slice() {
                let radius = number(mm(one, false).unwrap_or(0.0));
                let subject = if one_piece {
                    "The wall".to_owned()
                } else {
                    format!("Piece {}", pieces(&one.pieces))
                };
                let reach = if one_piece { "it" } else { "the wall" };
                return format!(
                    "{subject} {verb} to {radius} mm; below {} mm {reach} may crack.",
                    number(limit)
                );
            }
            let all: Vec<u32> = same.iter().flat_map(|w| w.pieces.clone()).collect();
            let plural = if inward { "bend inward" } else { "bend" };
            format!(
                "Pieces {} {plural} below {} mm; the wall may crack there.",
                pieces(&all),
                number(limit)
            )
        })
        .collect()
}

/// What the sentences need to know about the box besides its warnings (UX notes `0149` §6).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Context {
    /// The box is one wall piece: the text says "The wall", never "Piece 1".
    pub one_piece: bool,
    /// The Lid switch.
    pub lid: bool,
    /// The Lid seat switch.
    pub seat: bool,
    /// The Finger hollow switch.
    pub hollow: bool,
    /// The wall thickness: a limit equal to it belongs to a concave stretch, twice it to a
    /// convex one.
    pub wall: curvyo_document_core::Length,
    /// The error-correction letter of a QR code (`M`), for the sentence of content that is too
    /// long.
    pub level: char,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            one_piece: false,
            lid: true,
            seat: true,
            hollow: true,
            wall: curvyo_document_core::Length::from_mm(3.0),
            level: 'M',
        }
    }
}

impl Context {
    /// The context of a generated object: its stored switches and whether it has badges.
    #[must_use]
    pub fn of(generated: &curvyo_extension_core::Generated) -> Self {
        let on = |key: &str, default: bool| {
            generated
                .params
                .get(key)
                .and_then(curvyo_extension_core::ParamValue::from_stored)
                .and_then(|value| value.switch())
                .unwrap_or(default)
        };
        Self {
            one_piece: generated.marks.on_parts.is_empty(),
            lid: on("lid", true),
            seat: on("lid_seat", true),
            hollow: on("finger_hollow", true),
            wall: generated
                .params
                .get("thickness")
                .and_then(curvyo_extension_core::ParamValue::from_stored)
                .and_then(|value| value.length())
                .unwrap_or_else(|| curvyo_document_core::Length::from_mm(3.0)),
            level: generated
                .params
                .get("ec")
                .and_then(curvyo_extension_core::ParamValue::from_stored)
                .and_then(|value| value.choice().and_then(|id| id.chars().next()))
                .map_or('M', |letter| letter.to_ascii_uppercase()),
        }
    }
}

/// Whether the warning `id` says the content of a code cannot be drawn (it stays under the field).
#[must_use]
pub fn is_content_problem(id: &str) -> bool {
    matches!(id, "content_empty" | "content_too_long")
}

/// "Too long for a QR code at error correction M: 2412 bytes, at most 2331."
fn too_long(group: &[&Warning], level: char) -> String {
    let count = |value: Option<&curvyo_extension_core::ParamValue>| {
        value
            .and_then(curvyo_extension_core::ParamValue::count)
            .unwrap_or(0)
    };
    let warning = group.first();
    format!(
        "Too long for a QR code at error correction {level}: {} bytes, at most {}.",
        count(warning.and_then(|w| w.value.as_ref())),
        count(warning.and_then(|w| w.limit.as_ref()))
    )
}

/// One sentence per warning id, in the order the ids first appear.
#[must_use]
pub fn sentences(warnings: &[Warning]) -> Vec<String> {
    sentences_in(warnings, Context::default())
}

fn rounded(wall: f64, context: Context) -> String {
    let what = if context.lid && context.seat {
        "the cover and lid are rounded there"
    } else {
        "the cover is rounded there"
    };
    let hollow = if context.lid && context.seat && context.hollow {
        ", without a finger hollow"
    } else {
        ""
    };
    format!(
        "The notch is too narrow for the cover at {} mm walls: {what}{hollow}.",
        number(wall)
    )
}

/// One sentence per warning id for a box described by `context`.
#[must_use]
pub fn sentences_in(warnings: &[Warning], context: Context) -> Vec<String> {
    let mut ids: Vec<&str> = Vec::new();
    for warning in warnings {
        if !ids.contains(&warning.id) {
            ids.push(warning.id);
        }
    }
    ids.into_iter()
        .map(|id| {
            let group: Vec<&Warning> = warnings.iter().filter(|w| w.id == id).collect();
            match id {
                "sharp_corner" => sharp(&group, context.one_piece),
                "tight_bend" => tight(&group, context).join(" "),
                "ring_closed" => "The ring has no opening at this ring thickness.".to_owned(),
                "ring_opening_failed" => "The ring opening could not be built.".to_owned(),
                "cover_offset_failed" => {
                    "The cover could not be rounded; it is built exactly and may cross itself."
                        .to_owned()
                }
                "tab_window" => windows(&group),
                "content_empty" => "Enter a text to encode.".to_owned(),
                "content_too_long" => too_long(&group, context.level),
                "unknown_joint_pattern" => {
                    "The Joint pattern is not known here; Even is used.".to_owned()
                }
                "bend_too_short" => short(&group, context.one_piece),
                "fillet_failed" => {
                    "A slight bend could not be rounded; it is built as a sharp bend.".to_owned()
                }
                "hollow_left_out" => "No place for the finger hollow on this lid.".to_owned(),
                "smallest_size" => format!(
                    "The box is at its smallest for {} mm walls.",
                    number(mm(group[0], false).unwrap_or(3.0))
                ),
                "outline_crosses" => {
                    "The outline crosses itself; the default outline is built.".to_owned()
                }
                "cover_rounded" => rounded(mm(group[0], false).unwrap_or(3.0), context),
                other => other.to_owned(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::Length;
    use curvyo_extension_core::{Degrees, ParamValue};

    fn warning(id: &'static str, pieces: &[u32]) -> Warning {
        Warning {
            id,
            pieces: pieces.to_vec(),
            value: None,
            limit: None,
        }
    }

    #[test]
    fn one_and_several_sharp_corners_read_as_in_the_ux_notes() {
        let mut a = warning("sharp_corner", &[3, 4]);
        a.value = Some(ParamValue::Angle(Degrees(22.0)));
        assert_eq!(
            sentences(std::slice::from_ref(&a)),
            ["The corner between pieces 3 and 4 is very sharp: 22°."]
        );
        let b = warning("sharp_corner", &[6, 1]);
        assert_eq!(
            sentences(&[a, b]),
            ["2 corners are very sharp: between pieces 3 and 4, and 6 and 1."]
        );
    }

    #[test]
    fn tight_bends_name_the_radius_and_the_limit() {
        let mut a = warning("tight_bend", &[2]);
        a.value = Some(ParamValue::Length(Length::from_mm(2.5)));
        a.limit = Some(ParamValue::Length(Length::from_mm(6.0)));
        assert_eq!(
            sentences(std::slice::from_ref(&a)),
            ["Piece 2 bends to 2.5 mm; below 6 mm the wall may crack."]
        );
        let mut b = warning("tight_bend", &[5]);
        b.limit = a.limit.clone();
        assert_eq!(
            sentences(&[a, b]),
            ["Pieces 2 and 5 bend below 6 mm; the wall may crack there."]
        );
    }

    #[test]
    fn tabs_that_keep_their_window_say_how_many_and_why() {
        let mut one = warning("tab_window", &[]);
        one.value = Some(ParamValue::Count(1));
        assert_eq!(
            sentences(std::slice::from_ref(&one)),
            ["1 tab does not fit on one arc, so it stands in a flat window."]
        );
        let mut three = warning("tab_window", &[]);
        three.value = Some(ParamValue::Count(3));
        assert_eq!(
            sentences(&[three]),
            ["3 tabs do not fit on one arc, so they stand in flat windows."]
        );
    }

    #[test]
    fn an_unknown_joint_pattern_says_even_is_used() {
        assert_eq!(
            sentences(&[warning("unknown_joint_pattern", &[])]),
            ["The Joint pattern is not known here; Even is used."]
        );
    }

    #[allow(clippy::fn_params_excessive_bools)]
    fn with_context(one_piece: bool, lid: bool, seat: bool, hollow: bool) -> Context {
        Context {
            one_piece,
            lid,
            seat,
            hollow,
            wall: curvyo_document_core::Length::from_mm(3.0),
            level: 'M',
        }
    }

    #[test]
    fn a_rounded_cover_names_what_was_built_and_the_wall_thickness() {
        let mut w = warning("cover_rounded", &[]);
        w.value = Some(ParamValue::Length(Length::from_mm(3.0)));
        let said = |c| sentences_in(std::slice::from_ref(&w), c)[0].clone();
        assert_eq!(
            said(with_context(false, true, true, true)),
            "The notch is too narrow for the cover at 3 mm walls: the cover and lid are rounded there, without a finger hollow."
        );
        for c in [
            with_context(false, false, true, true),
            with_context(false, true, false, true),
        ] {
            assert_eq!(
                said(c),
                "The notch is too narrow for the cover at 3 mm walls: the cover is rounded there."
            );
        }
        assert_eq!(
            said(with_context(false, true, true, false)),
            "The notch is too narrow for the cover at 3 mm walls: the cover and lid are rounded there."
        );
    }

    fn tight_warning(piece: u32, radius: f64, limit: f64) -> Warning {
        let mut w = warning("tight_bend", &[piece]);
        w.value = Some(ParamValue::Length(Length::from_mm(radius)));
        w.limit = Some(ParamValue::Length(Length::from_mm(limit)));
        w
    }

    #[test]
    fn one_piece_says_the_wall() {
        let one = with_context(true, true, true, true);
        assert_eq!(
            sentences_in(&[tight_warning(1, 2.5, 6.0)], one),
            ["The wall bends to 2.5 mm; below 6 mm it may crack."]
        );
        let mut sharp = warning("sharp_corner", &[1, 1]);
        sharp.value = Some(ParamValue::Angle(Degrees(22.0)));
        assert_eq!(
            sentences_in(&[sharp], one),
            ["The corner of the wall is very sharp: 22°."]
        );
    }

    #[test]
    fn a_concave_stretch_has_its_own_sentence_and_limit() {
        let all = [
            tight_warning(2, 2.5, 6.0),
            tight_warning(5, 4.0, 6.0),
            tight_warning(3, 2.0, 3.0),
        ];
        assert_eq!(
            sentences_in(&all, Context::default()),
            [
                "Pieces 2 and 5 bend below 6 mm; the wall may crack there. Piece 3 bends inward to 2 mm; below 3 mm the wall may crack."
            ]
        );
        let two = [tight_warning(3, 2.0, 3.0), tight_warning(4, 1.0, 3.0)];
        assert_eq!(
            sentences_in(&two, Context::default()),
            ["Pieces 3 and 4 bend inward below 3 mm; the wall may crack there."]
        );
    }

    #[test]
    fn a_bend_too_short_for_two_slits_has_its_own_sentence() {
        let mut w = warning("bend_too_short", &[2]);
        w.value = Some(ParamValue::Length(Length::from_mm(4.1)));
        assert_eq!(
            sentences(std::slice::from_ref(&w)),
            ["Piece 2 has a bend too short for two slits (4.1 mm); it may crack there."]
        );
        let mut v = warning("bend_too_short", &[5]);
        v.value = Some(ParamValue::Length(Length::from_mm(3.0)));
        assert_eq!(
            sentences(&[w, v]),
            ["Pieces 2 and 5 have a bend too short for two slits; they may crack there."]
        );
        let mut one = warning("bend_too_short", &[1]);
        one.value = Some(ParamValue::Length(Length::from_mm(2.0)));
        let single = Context {
            one_piece: true,
            ..Context::default()
        };
        assert_eq!(
            sentences_in(&[one], single),
            ["The wall has a bend too short for two slits (2 mm); it may crack there."]
        );
    }
}
