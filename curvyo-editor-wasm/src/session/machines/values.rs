//! The typed text of a Machines-tab field read into a family's record values, with the field
//! message of a refused value and the notice of a clamped one.

use curvyo_document_core::{DocumentSize, Length};
use curvyo_laser_core::{
    MachineProfile, OriginCorner, clamp_default_power, clamp_default_speed, clamp_power,
    clamp_work_area, edit_max_feed,
};

/// The typed number of `text`, thousands commas allowed.
fn typed(text: &str) -> Result<f64, String> {
    text.trim()
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| "Enter a number.".to_owned())
}

/// A whole typed number.
pub(super) fn whole(text: &str) -> Result<u64, String> {
    let n = typed(text)?;
    if n.fract() != 0.0 || n < 0.0 {
        return Err("Enter a whole number.".to_owned());
    }
    // invariant: a whole, non-negative, finite number checked above
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(n as u64)
}

/// A work area side typed into `side` of `area`.
fn work_side(area: DocumentSize, side: &str, text: &str) -> Result<DocumentSize, String> {
    let mm = Length::from_mm(typed(text)?);
    Ok(match side {
        "width" => DocumentSize::new(mm, area.height),
        _ => DocumentSize::new(area.width, mm),
    })
}

/// The laser record `p` with `key` set from `text`, and the notice of a clamped value.
pub(super) fn edit_laser(
    mut p: MachineProfile,
    key: &str,
    text: &str,
) -> Result<(MachineProfile, Option<String>), String> {
    let v = &mut p.values;
    let notice = match key {
        "width" | "height" => {
            let area = clamp_work_area(work_side(p.work_area, key, text)?);
            p.work_area = area.value;
            area.notice
        }
        "origin" => {
            v.origin = OriginCorner::from_name(text).ok_or_else(String::new)?;
            None
        }
        "max_feed" => edit_max_feed(v, typed(text)?).ok_or_else(String::new)?,
        "max_power" => {
            let limited = clamp_power(typed(text)?).ok_or_else(String::new)?;
            v.max_power = limited.value;
            limited.notice
        }
        "default_power" => {
            let limited = clamp_default_power(typed(text)?).ok_or_else(String::new)?;
            v.defaults.power = limited.value;
            limited.notice
        }
        "default_speed" => {
            let limited = clamp_default_speed(typed(text)?, v.max_feed).ok_or_else(String::new)?;
            v.defaults.speed = limited.value;
            limited.notice
        }
        "laser_mode" => {
            v.laser_mode = text == "on";
            None
        }
        "motorised_z" => {
            v.motorised_z = text == "on";
            None
        }
        "air_assist" => {
            v.air_assist = text == "on";
            None
        }
        "focus_pulse" => {
            v.focus_pulse.on = text == "on";
            None
        }
        "pulse_power" => {
            let limited =
                curvyo_laser_core::clamp_pulse_power(typed(text)?).ok_or_else(String::new)?;
            v.focus_pulse.power = limited.value;
            limited.notice
        }
        "pulse_length" => {
            let limited =
                curvyo_laser_core::clamp_pulse_length(typed(text)?).ok_or_else(String::new)?;
            v.focus_pulse.length = limited.value;
            limited.notice
        }
        // Use machine size (`specs/0131` criterion 2 in the Machines tab): "W×H" in mm.
        "work_area" => {
            let (w, h) = text.split_once('x').ok_or_else(String::new)?;
            let area = clamp_work_area(DocumentSize::new(
                Length::from_mm(typed(w)?),
                Length::from_mm(typed(h)?),
            ));
            p.work_area = area.value;
            area.notice
        }
        _ => return Err(String::new()),
    };
    Ok((p, notice))
}

/// The plotter values `v` with `key` set from `text`.
pub(super) fn edit_plotter(
    mut v: curvyo_cutter_core::CutterValues,
    key: &str,
    text: &str,
) -> Result<curvyo_cutter_core::CutterValues, String> {
    use curvyo_cutter_core::{PressurePct, SpeedMmS};
    let n = whole(text)?;
    match key {
        "default_pressure" => {
            v.default_pressure =
                PressurePct::new(n).ok_or_else(|| "Pressure is 1 to 100 %.".to_owned())?;
        }
        "default_speed" => {
            v.default_speed = SpeedMmS::new(n)
                .filter(|s| *s <= v.max_speed)
                .ok_or_else(|| format!("Speed is 1 to {} mm/s.", v.max_speed.get()))?;
        }
        "max_speed" => {
            v.max_speed = SpeedMmS::new(n)
                .ok_or_else(|| format!("Speed is 1 to {} mm/s.", SpeedMmS::LIMIT))?;
            v.default_speed = v.default_speed.min(v.max_speed);
        }
        _ => return Err(String::new()),
    }
    Ok(v)
}

/// The CNC values `v` with `key` set from `text`.
pub(super) fn edit_cnc(
    mut v: curvyo_cnc_core::CncValues,
    key: &str,
    text: &str,
) -> Result<curvyo_cnc_core::CncValues, String> {
    let n = if key == "connector" {
        0.0
    } else {
        typed(text)?
    };
    match key {
        "width" | "height" => {
            if !(10.0..=10_000.0).contains(&n) {
                return Err(format!(
                    "{} is 10 to 10,000 mm.",
                    if key == "width" { "Width" } else { "Height" }
                ));
            }
            v.work_area = work_side(v.work_area, key, text)?;
        }
        "z_travel_mm" => {
            if !(1.0..=1_000.0).contains(&n) {
                return Err("Z travel is 1 to 1,000 mm.".to_owned());
            }
            v.z_travel = Length::from_mm(n);
            if v.safe_height.as_mm() > n {
                v.safe_height = Length::from_mm(n);
            }
        }
        "spindle_max_rpm" => {
            v.spindle_max = curvyo_cnc_core::Rpm::new(whole(text)?)
                .ok_or("Spindle max is 0 to 100,000 rpm.")?;
        }
        "connector" => {
            let address = text.trim();
            let limit = curvyo_laser_core::MAX_CONNECTION_CHARS;
            if address.chars().count() > limit {
                return Err(format!("The address is at most {limit} characters."));
            }
            address.clone_into(&mut v.connection.connector_address);
        }
        "safe_height_mm" => {
            let max = v.z_travel.as_mm();
            if !(0.5..=max).contains(&n) {
                return Err(format!("Safe height is 0.5 to {max} mm."));
            }
            v.safe_height = Length::from_mm(n);
        }
        _ => return Err(String::new()),
    }
    Ok(v)
}
