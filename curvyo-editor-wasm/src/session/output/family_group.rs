//! The output group and the play bar of a plotter, embroidery or CNC page in the laser's view
//! shape (ADR 0098 decision 3): machine line, checks, total, Export with the family's file type,
//! Send on CNC, and the play bar's axis and readout per family.

use curvyo_document_core::NodeId;
use serde_json::Value;

use super::super::Session;
use super::super::output_page::FamilyPage;
use super::play::PlayView;
use super::send::SendView;
use super::view_json::{
    CheckTargetView, CheckView, ExportView, OutputGroupView, RunsLineView, TotalView,
};

/// The ids of a JSON list of node ids.
fn ids(value: &Value) -> Vec<NodeId> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// The count words of a check text ("3 items", "1 item"), the line's button, and the text before
/// and after them; the whole text before when there is no count.
fn split_count(text: &str) -> (String, String, String) {
    let words: Vec<&str> = text.split(' ').collect();
    for (at, pair) in words.windows(2).enumerate() {
        let item = pair[1].trim_end_matches(['.', ',', ':']);
        if pair[0].chars().all(|c| c.is_ascii_digit()) && (item == "item" || item == "items") {
            let before = words[..at].join(" ");
            let before = if before.is_empty() {
                before
            } else {
                format!("{before} ")
            };
            let button = format!("{} {item}", pair[0]);
            let after = text[before.len() + button.len()..].to_owned();
            return (before, button, after);
        }
    }
    (text.to_owned(), String::new(), String::new())
}

/// One family check as the laser's check line.
fn check(value: &Value) -> CheckView {
    let text = value["text"].as_str().unwrap_or_default().to_owned();
    let items = ids(&value["items"]);
    let (before, button, after) = if items.is_empty() {
        (text.clone(), String::new(), String::new())
    } else {
        split_count(&text)
    };
    CheckView {
        severity: if value["severity"] == "block" {
            "block"
        } else {
            "warn"
        },
        count: items.len().max(1),
        target: if button.is_empty() {
            CheckTargetView::None
        } else {
            CheckTargetView::Items {
                ids: items,
                unfold: None,
            }
        },
        text,
        before,
        button,
        after,
    }
}

/// The work line of a family total: "Runs 2 operations, 5 items.", the item count its button.
fn runs(text: &str) -> RunsLineView {
    let (before, button, after) = split_count(text);
    RunsLineView {
        text: text.to_owned(),
        before,
        button,
        after,
    }
}

/// The save dialog's file type and extension of `family`'s profile format.
const fn file_kind(family: &FamilyPage) -> (&'static str, &'static str) {
    match family {
        FamilyPage::Plotter(p) => (
            p.copy.values.output_format.label(),
            p.copy.values.output_format.extension(),
        ),
        FamilyPage::Embroidery(p) => (
            p.copy.values.output_format.file_type(),
            p.copy.values.output_format.extension(),
        ),
        FamilyPage::Cnc(p) => (
            p.copy.values.output_format.file_type(),
            p.copy.values.output_format.extension(),
        ),
    }
}

impl Session {
    /// The output group of the family page `family`.
    pub(in crate::session) fn family_output_group(&self, family: &FamilyPage) -> OutputGroupView {
        let view = self.family_view(family).unwrap_or(Value::Null);
        let checks: Vec<CheckView> = view["checks"]
            .as_array()
            .map(|c| c.iter().map(check).collect())
            .unwrap_or_default();
        let first_block = checks
            .iter()
            .find(|c| c.severity == "block")
            .map(|c| c.text.clone());
        let allowed = view["exportAllowed"] == true;
        let nothing = checks.iter().any(|c| c.text.starts_with("Nothing runs"));
        let total = &view["total"];
        let runs_text = total["runs"].as_str().unwrap_or_default();
        let lengths = [&total["lengths"], &total["stitches"]]
            .iter()
            .find_map(|v| v.as_str())
            .unwrap_or_default()
            .to_owned();
        let (file_type, extension) = file_kind(family);
        let manufacturing = family.manufacturing();
        let send = view["send"].as_object().map(|send| SendView {
            enabled: send.get("enabled") == Some(&Value::Bool(true)),
            cause: None,
            text: None,
            profile_id: send["profileId"].as_str().unwrap_or_default().to_owned(),
            address: send["address"].as_str().unwrap_or_default().to_owned(),
            starts: false,
            job: 0,
        });
        OutputGroupView {
            machine: view["machineLine"].as_str().unwrap_or_default().to_owned(),
            start: None,
            total: (!nothing).then(|| TotalView {
                time: total["time"].as_str().map(str::to_owned),
                runs: (!runs_text.is_empty()).then(|| runs(runs_text)),
                fixtures: None,
                lengths,
            }),
            checks,
            export: ExportView {
                enabled: allowed,
                tooltip: first_block,
                file_type,
                extension,
            },
            send,
            toolpath_view: self.toolpath_view_pressed(manufacturing),
            frame: None,
        }
    }

    /// The play bar of the family page `family` while its Toolpath view is pressed: the stitch
    /// axis on embroidery, the planner's time on CNC; `None` on a plotter page, which has no play
    /// bar yet.
    pub(in crate::session) fn family_play_view(&self, family: &FamilyPage) -> Option<PlayView> {
        if matches!(family, FamilyPage::Plotter(_)) {
            return None;
        }
        let view = self.family_view(family)?;
        let play = &view["play"];
        let position = play["position"].as_f64().unwrap_or(0.0);
        let length = play["length"].as_f64().unwrap_or(0.0);
        let mut readout = play["text"].as_str().unwrap_or_default().to_owned();
        if let Some(colour) = play["colourText"].as_str() {
            readout = format!("{readout} · {colour}");
        }
        let embroidery = matches!(family, FamilyPage::Embroidery(_));
        // A plan with no position yet shows its end, as the MVP's slider did.
        let fraction = if length > 0.0 {
            (position / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some(PlayView {
            fraction,
            playing: self.output.play.playing.is_some(),
            readout,
            enabled: length > 0.0,
            step: (embroidery && length > 0.0).then(|| 1.0 / length),
            colour: embroidery
                .then(|| play["colour"].as_str().map(str::to_owned))
                .flatten(),
        })
    }

    /// Moves the family page's play head to `fraction` of its axis.
    pub(in crate::session) fn set_family_play(&mut self, family: &FamilyPage, fraction: f64) {
        let Some(view) = self.family_view(family) else {
            return;
        };
        let length = view["play"]["length"].as_f64().unwrap_or(0.0);
        match family {
            FamilyPage::Embroidery(_) => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                // invariant: a stitch count is far below usize::MAX and the product is positive
                let position = (fraction * length).round().max(0.0) as usize;
                self.set_embroidery_play(position);
            }
            FamilyPage::Cnc(_) => {
                self.set_cnc_play(fraction * length);
            }
            FamilyPage::Plotter(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_of_a_check_is_its_button() {
        assert_eq!(
            split_count("Cut 1: 2 items lie outside the hoop."),
            (
                "Cut 1: ".to_owned(),
                "2 items".to_owned(),
                " lie outside the hoop.".to_owned()
            )
        );
        assert_eq!(
            split_count("1 item lies outside the mat's cut area."),
            (
                String::new(),
                "1 item".to_owned(),
                " lies outside the mat's cut area.".to_owned()
            )
        );
        assert_eq!(
            split_count("Runs 2 operations, 5 items."),
            (
                "Runs 2 operations, ".to_owned(),
                "5 items".to_owned(),
                ".".to_owned()
            )
        );
        assert_eq!(
            split_count("Nothing runs."),
            ("Nothing runs.".to_owned(), String::new(), String::new())
        );
    }
}
