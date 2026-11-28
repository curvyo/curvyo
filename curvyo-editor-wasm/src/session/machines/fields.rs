//! The Machines tab's body fields per family (UX unify 4.3): what each record shows, in order,
//! as plain lines for a built-in.

use curvyo_laser_core::MachineProfile;

use super::{FieldKind, MachineFieldView};

fn field(
    key: &'static str,
    label: &'static str,
    value: String,
    unit: &'static str,
    kind: FieldKind,
    builtin: bool,
) -> MachineFieldView {
    MachineFieldView {
        key,
        label,
        value,
        unit,
        kind: if builtin { FieldKind::Text } else { kind },
        greyed: None,
    }
}

fn on(flag: bool) -> String {
    if flag { "on" } else { "off" }.to_owned()
}

/// The body of a laser machine (UX unify 4.3).
pub(super) fn laser_fields(p: &MachineProfile, builtin: bool) -> Vec<MachineFieldView> {
    let v = &p.values;
    let n = FieldKind::Number;
    let s = FieldKind::Switch;
    let mut out = vec![
        field(
            "width",
            "Width",
            super::number(p.work_area.width.as_mm()),
            "mm",
            n,
            builtin,
        ),
        field(
            "height",
            "Height",
            super::number(p.work_area.height.as_mm()),
            "mm",
            n,
            builtin,
        ),
        field(
            "origin",
            "Origin",
            v.origin.name().to_owned(),
            "",
            FieldKind::Corner,
            builtin,
        ),
        field(
            "max_feed",
            "Max feed",
            v.max_feed.get().to_string(),
            "mm/min",
            n,
            builtin,
        ),
        field(
            "max_power",
            "Max power",
            v.max_power.get().to_string(),
            "S",
            n,
            builtin,
        ),
        field(
            "default_power",
            "Default power",
            super::number(v.defaults.power.percent()),
            "%",
            n,
            builtin,
        ),
        field(
            "default_speed",
            "Default speed",
            v.defaults.speed.get().to_string(),
            "mm/min",
            n,
            builtin,
        ),
        field("laser_mode", "Laser mode", on(v.laser_mode), "", s, builtin),
    ];
    out.extend(pulse_fields(v, builtin));
    out.extend([
        field(
            "motorised_z",
            "Motorised Z axis",
            on(v.motorised_z),
            "",
            s,
            builtin,
        ),
        field("air_assist", "Air assist", on(v.air_assist), "", s, builtin),
        field(
            "output",
            "Output",
            curvyo_laser_core::job::name(v.output_format).to_owned(),
            "",
            FieldKind::Text,
            builtin,
        ),
    ]);
    out
}

/// The Focus point mode's pulse (`specs/0118` 35 a, W60 to W62), greyed while Laser mode is off;
/// GRBL is the only laser output.
fn pulse_fields(v: &curvyo_laser_core::ProfileValues, builtin: bool) -> [MachineFieldView; 3] {
    let (s, n) = (FieldKind::Switch, FieldKind::Number);
    let pulse = |mut f: MachineFieldView| {
        if !v.laser_mode && f.kind != FieldKind::Text {
            f.greyed = Some("Laser mode is off");
        }
        f
    };
    [
        pulse(field(
            "focus_pulse",
            "Focus pulse",
            on(v.focus_pulse.on),
            "",
            s,
            builtin,
        )),
        pulse(field(
            "pulse_power",
            "Pulse power",
            super::number(f64::from(v.focus_pulse.power.get()) / 10.0),
            "%",
            n,
            builtin,
        )),
        pulse(field(
            "pulse_length",
            "Pulse length",
            v.focus_pulse.length.get().to_string(),
            "ms",
            n,
            builtin,
        )),
    ]
}

/// The body of a plotter machine: the mats or the work area, and the numbers of an HPGL one.
pub(super) fn plotter_fields(
    v: &curvyo_cutter_core::CutterValues,
    builtin: bool,
) -> Vec<MachineFieldView> {
    let mut out = Vec::new();
    if !v.mats.is_empty() {
        let mats: Vec<&str> = v.mats.iter().map(|m| m.label()).collect();
        out.push(field(
            "mats",
            "Mats",
            mats.join(", "),
            "",
            FieldKind::Text,
            builtin,
        ));
    }
    if !v.output_format.sets_numbers() {
        let n = FieldKind::Number;
        out.push(field(
            "max_speed",
            "Max speed",
            v.max_speed.get().to_string(),
            "mm/s",
            n,
            builtin,
        ));
        out.push(field(
            "default_pressure",
            "Default pressure",
            v.default_pressure.get().to_string(),
            "%",
            n,
            builtin,
        ));
        out.push(field(
            "default_speed",
            "Default speed",
            v.default_speed.get().to_string(),
            "mm/s",
            n,
            builtin,
        ));
    }
    out.push(field(
        "output",
        "Output",
        v.output_format.label().to_owned(),
        "",
        FieldKind::Text,
        builtin,
    ));
    out
}

/// The body of an embroidery machine.
pub(super) fn embroidery_fields(
    v: &curvyo_embroidery_core::EmbroideryValues,
    builtin: bool,
) -> Vec<MachineFieldView> {
    let hoops: Vec<String> = v
        .hoops
        .iter()
        .map(|h| {
            let (w, h) = h.field_mm();
            format!("{w} × {h} mm")
        })
        .collect();
    vec![
        field(
            "hoops",
            "Hoops",
            hoops.join(", "),
            "",
            FieldKind::Text,
            builtin,
        ),
        field(
            "speed",
            "Speed",
            v.speed.get().to_string(),
            "stitches/min",
            FieldKind::Number,
            builtin,
        ),
        field(
            "max_stitch",
            "Max stitch",
            format!("{:.1} mm", v.max_stitch.mm()),
            "",
            FieldKind::Text,
            builtin,
        ),
        field(
            "output",
            "Output",
            v.output_format.label().to_owned(),
            "",
            FieldKind::Text,
            builtin,
        ),
    ]
}

/// The body of a CNC machine.
pub(super) fn cnc_fields(v: &curvyo_cnc_core::CncValues, builtin: bool) -> Vec<MachineFieldView> {
    let n = FieldKind::Number;
    vec![
        field(
            "width",
            "Width",
            super::number(v.work_area.width.as_mm()),
            "mm",
            n,
            builtin,
        ),
        field(
            "height",
            "Height",
            super::number(v.work_area.height.as_mm()),
            "mm",
            n,
            builtin,
        ),
        field(
            "z_travel_mm",
            "Z travel",
            super::number(v.z_travel.as_mm()),
            "mm",
            n,
            builtin,
        ),
        field(
            "spindle_max_rpm",
            "Spindle max",
            v.spindle_max.get().to_string(),
            "rpm",
            n,
            builtin,
        ),
        field(
            "safe_height_mm",
            "Safe height",
            super::number(v.safe_height.as_mm()),
            "mm",
            n,
            builtin,
        ),
        // Send's connector (`0161` criterion 19): a record value, no page follows it.
        field(
            "connector",
            "Connector",
            v.connection.connector_address.clone(),
            "",
            FieldKind::Entry,
            builtin,
        ),
        field(
            "output",
            "Output",
            v.output_format.label().to_owned(),
            "",
            FieldKind::Text,
            builtin,
        ),
    ]
}
