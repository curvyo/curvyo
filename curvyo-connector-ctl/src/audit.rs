//! `audit` (A18; UX notes 4.8): the admin log's lines in plain words, columns aligned.

use curvyo_connector_core::LogName;

use crate::context::Context;
use crate::error::CtlError;
use crate::jobs::log_lines;
use crate::texts::{columns, log_time};

/// The words of one `admin.log` event: `event`, `subject`, `detail` (addendum D9).
#[must_use]
pub fn event_words(event: &str, subject: &str, detail: &str) -> String {
    match event {
        "configured" => format!("configured from {detail}"),
        "config-changed" => "configuration changed".to_owned(),
        "code-issued" => format!("code issued for {subject}, {detail}"),
        "paired" => format!("paired as {detail}"),
        "revoked" => format!("revoked {subject}"),
        "unlocked" => "unlocked".to_owned(),
        "start-confirmed" => {
            let job = detail.split(' ').next().unwrap_or_default();
            let what = if detail.ends_with(" resume") {
                "resume"
            } else {
                "start"
            };
            format!("{what} confirmed: {job}, requested by {subject}")
        }
        other => other.to_owned(),
    }
}

/// `audit [--last <n>]`.
///
/// # Errors
/// The failure in words.
pub fn audit(ctx: &mut Context, name: Option<&str>, last: u32) -> Result<String, CtlError> {
    let lines = log_lines(ctx, name, LogName::Admin, last)?;
    if lines.is_empty() {
        return Ok("No entries\n".to_owned());
    }
    let rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let field = |i: usize| f.get(i).copied().unwrap_or_default();
            let by = if field(2).is_empty() {
                "the host"
            } else {
                field(2)
            };
            vec![
                log_time(field(0)),
                by.to_owned(),
                event_words(field(1), field(3), field(4)),
            ]
        })
        .collect();
    Ok(columns(&["Time (UTC)", "By", "Event"], &rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a18_the_event_words_of_the_ux_notes() {
        let cases = [
            (
                ("configured", "", "192.168.1.30"),
                "configured from 192.168.1.30",
            ),
            (
                ("code-issued", "Laptop", "owner"),
                "code issued for Laptop, owner",
            ),
            (("paired", "", "member"), "paired as member"),
            (("revoked", "Phone", "member"), "revoked Phone"),
            (("unlocked", "", ""), "unlocked"),
            (
                ("config-changed", "", "controller"),
                "configuration changed",
            ),
            (
                ("start-confirmed", "Phone", "sign.gcode 67e5 "),
                "start confirmed: sign.gcode, requested by Phone",
            ),
            (
                ("start-confirmed", "Phone", "sign.gcode 67e5 resume"),
                "resume confirmed: sign.gcode, requested by Phone",
            ),
        ];
        for ((event, subject, detail), words) in cases {
            assert_eq!(event_words(event, subject, detail), words);
        }
    }
}
