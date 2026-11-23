//! What happens to the waiting jobs when the current job ends (`specs/0132` criterion 11): one
//! pure decision, taken only at the transition that ends the job.

/// How the current job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The controller ran every line.
    Completed,
    /// The controller reported an error.
    Failed,
    /// The maker stopped it.
    Stopped,
    /// The controller raised an alarm while the job ran.
    Alarm,
    /// The controller was lost while the job ran.
    Lost,
}

impl End {
    /// The word after "Chain stopped: ".
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            End::Completed => "Completed",
            End::Failed => "Failed",
            End::Stopped => "Stopped",
            End::Alarm => "Alarm",
            End::Lost => "Lost the controller",
        }
    }
}

/// What the end of a job does to the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    /// No flag is set on the first waiting job: it waits for a press.
    Nothing,
    /// The first waiting job starts as its flag's owner would start it.
    Start,
    /// The chain is cancelled: every flag is cleared and, if one was set, the list says so.
    Stop,
}

/// The decision for a job that ended as `end`; `flagged` says the first waiting job carries the
/// mark, `ready` that the controller can take a job now (connected, no alarm, no door, check or
/// sleep state, laser mode on).
#[must_use]
pub fn next(end: End, flagged: bool, ready: bool) -> Chain {
    match end {
        End::Completed if !flagged => Chain::Nothing,
        End::Completed if ready => Chain::Start,
        End::Completed | End::Failed | End::Stopped | End::Alarm | End::Lost => Chain::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_x_flag_x_ready_table() {
        let ends = [
            End::Completed,
            End::Failed,
            End::Stopped,
            End::Alarm,
            End::Lost,
        ];
        let mut rows = Vec::new();
        for end in ends {
            let cells: Vec<String> = [(false, true), (true, true), (true, false)]
                .iter()
                .map(|&(flagged, ready)| format!("{:?}", next(end, flagged, ready)))
                .collect();
            rows.push(format!("{end:?}: {}", cells.join(" ")));
        }
        assert_eq!(
            rows,
            [
                "Completed: Nothing Start Stop",
                "Failed: Stop Stop Stop",
                "Stopped: Stop Stop Stop",
                "Alarm: Stop Stop Stop",
                "Lost: Stop Stop Stop",
            ]
        );
    }
}
