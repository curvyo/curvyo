//! The go to point of the Focus point mode as the webview sends it (`specs/0118` 2d, adrs P10).

use curvyo_connector_core::{Message, Millimetres, Pulse, PulseLength, PulsePower};
use serde::Deserialize;

/// The pulse of a press, as the page's profile copy gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PulseRequest {
    power_permille: u16,
    length_ms: u16,
}

/// A press on the canvas: the target in job file coordinates and the pulse, if any.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct PointRequest {
    x: f64,
    y: f64,
    #[serde(default)]
    pulse: Option<PulseRequest>,
}

impl PointRequest {
    /// The `GoToPoint` message; `None` for a pulse outside the connector's caps (the connector
    /// would refuse it, so nothing is sent).
    pub(crate) fn message(self) -> Option<Message> {
        let pulse = match self.pulse {
            None => None,
            Some(p) => Some(Pulse {
                power_permille: PulsePower::new(p.power_permille)?,
                length_ms: PulseLength::new(p.length_ms)?,
            }),
        };
        Some(Message::GoToPoint {
            x: Millimetres(self.x),
            y: Millimetres(self.y),
            pulse,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(json: &str) -> PointRequest {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_press_becomes_go_to_point_with_its_pulse_or_none() {
        let plain = request(r#"{"x": 10.5, "y": -3}"#).message().unwrap();
        assert_eq!(
            plain,
            Message::GoToPoint {
                x: Millimetres(10.5),
                y: Millimetres(-3.0),
                pulse: None,
            }
        );
        let with = request(r#"{"x": 1, "y": 2, "pulse": {"powerPermille": 10, "lengthMs": 300}}"#);
        assert!(matches!(
            with.message(),
            Some(Message::GoToPoint { pulse: Some(p), .. }) if p.length_ms.get() == 300
        ));
        let over = request(r#"{"x": 1, "y": 2, "pulse": {"powerPermille": 51, "lengthMs": 300}}"#);
        assert_eq!(over.message(), None);
    }
}
