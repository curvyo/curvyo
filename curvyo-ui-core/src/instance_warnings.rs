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

fn sharp(group: &[&Warning]) -> String {
    if let [one] = group {
        let angle = one
            .value
            .as_ref()
            .and_then(curvyo_extension_core::ParamValue::degrees)
            .unwrap_or(0.0);
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

fn tight(group: &[&Warning]) -> String {
    let limit = group.first().and_then(|w| mm(w, true)).unwrap_or(0.0);
    if let [one] = group {
        return format!(
            "Piece {} bends to {} mm; below {} mm the wall may crack.",
            pieces(&one.pieces),
            number(mm(one, false).unwrap_or(0.0)),
            number(limit)
        );
    }
    let all: Vec<u32> = group.iter().flat_map(|w| w.pieces.clone()).collect();
    format!(
        "Pieces {} bend below {} mm; the wall may crack there.",
        pieces(&all),
        number(limit)
    )
}

/// One sentence per warning id, in the order the ids first appear.
#[must_use]
pub fn sentences(warnings: &[Warning]) -> Vec<String> {
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
                "sharp_corner" => sharp(&group),
                "tight_bend" => tight(&group),
                "ring_closed" => "The ring has no opening at this ring thickness.".to_owned(),
                "ring_opening_failed" => "The ring opening could not be built.".to_owned(),
                "hollow_left_out" => "No place for the finger hollow on this lid.".to_owned(),
                "smallest_size" => format!(
                    "The box is at its smallest for {} mm walls.",
                    number(mm(group[0], false).unwrap_or(3.0))
                ),
                "outline_crosses" => {
                    "The outline crosses itself; the default outline is built.".to_owned()
                }
                "outline_curved" => {
                    "This outline has curves; they are built as straight edges.".to_owned()
                }
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
}
