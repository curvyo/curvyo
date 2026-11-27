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
    /// The kind's label ("QR code", "EAN-13"), which the sentences about the content name.
    pub label: &'static str,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            one_piece: false,
            lid: true,
            seat: true,
            hollow: true,
            wall: curvyo_document_core::Length::from_mm(3.0),
            label: "code",
        }
    }
}

impl Context {
    /// The context of a generated object: its stored switches and whether it has badges.
    #[must_use]
    pub fn of(generated: &curvyo_extension_core::Generated, label: &'static str) -> Self {
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
            label,
        }
    }
}

/// Whether the warning names a row and the generator drew nothing: the reason stays under that
/// row as a lasting line (`0144` criterion 13), not as a notice.
#[must_use]
pub fn is_row_problem(warning: &Warning, drew_nothing: bool) -> bool {
    drew_nothing && warning.param.is_some()
}

fn count_of(value: Option<&curvyo_extension_core::ParamValue>) -> u32 {
    value
        .and_then(curvyo_extension_core::ParamValue::count)
        .unwrap_or(0)
}

/// "Too long for a QR code at error correction M: 2412 bytes, at most 2331." (the unit is bytes)
/// or "Too long for Code 128: 92 characters, at most 80." (the unit is characters).
fn too_long(group: &[&Warning], context: Context, unit: &str) -> String {
    let warning = group.first();
    let value = count_of(warning.and_then(|w| w.value.as_ref()));
    let limit = count_of(warning.and_then(|w| w.limit.as_ref()));
    let cause = warning
        .and_then(|w| w.subject)
        .map_or_else(String::new, |subject| format!(" at {subject}"));
    let name = if unit == "bytes" {
        format!("a {}", crate::instance_notice::noun(context.label))
    } else {
        context.label.to_owned()
    };
    format!("Too long for {name}{cause}: {value} {unit}, at most {limit}.")
}

/// "EAN-13 takes 12 or 13 digits.": the symbology and its two lengths.
fn wrong_length(group: &[&Warning], context: Context) -> String {
    let warning = group.first();
    format!(
        "{} takes {} or {} digits.",
        context.label,
        count_of(warning.and_then(|w| w.value.as_ref())),
        count_of(warning.and_then(|w| w.limit.as_ref()))
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
                "content_too_long" => too_long(&group, context, "bytes"),
                "too_many_characters" => too_long(&group, context, "characters"),
                "wrong_length" => wrong_length(&group, context),
                "digits_only" => format!("{} takes digits only.", context.label),
                "not_encodable" => format!(
                    "{} takes {} only.",
                    context.label,
                    group[0].subject.unwrap_or("other characters")
                ),
                "check_digit_corrected" => "Check digit corrected.".to_owned(),
                "glyph_missing" => {
                    "The font has no outline for some characters; they are left out.".to_owned()
                }
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
#[path = "instance_warnings_tests.rs"]
mod tests;
