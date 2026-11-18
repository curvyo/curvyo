//! `specs/0127` G21: GRBL 1.1's output as golden files in `tests/fixtures/grbl/` (status reports
//! in both coordinate modes, banners, `$I`, `$$`, answers, alarms and messages), read by the pure
//! reply parser and the settings reader.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_connector_io::grbl::reply::{Lines, Reply, parse};
use curvyo_connector_io::grbl::settings::Settings;

fn fixture(name: &str) -> Vec<String> {
    let path = format!("{}/tests/fixtures/grbl/{name}", env!("CARGO_MANIFEST_DIR"));
    Lines::default().push(&std::fs::read(path).unwrap())
}

#[test]
fn status_reports_in_work_and_machine_coordinates() {
    let reports: Vec<_> = fixture("status_reports.txt")
        .iter()
        .map(|l| match parse(l) {
            Reply::Report(r) => r,
            other => panic!("{other:?}"),
        })
        .collect();
    let states: Vec<_> = reports.iter().map(|r| (r.state.as_str(), r.sub)).collect();
    assert_eq!(
        states,
        [
            ("Idle", None),
            ("Run", None),
            ("Hold", Some(1)),
            ("Hold", Some(0)),
            ("Alarm", None),
            ("Door", Some(1)),
            ("Home", None),
        ]
    );
    assert_eq!(reports[1].wpos, Some([12.5, 40.0, -1.25]));
    assert_eq!(reports[1].accessories, "S");
    assert_eq!(reports[2].mpos, Some([22.5, 60.0, 0.0]));
    assert_eq!(reports[2].wco, Some([10.0, 20.0, 0.0]));
    assert!(reports[3].hold_complete() && !reports[2].hold_complete());
    assert_eq!(
        reports[3].wco, None,
        "WCO comes now and then; the controller caches it"
    );
    assert_eq!(reports[6].rx_free, Some(128), "Bf's free receive bytes");
    assert_eq!(reports[0].rx_free, None, "no Bf without $10 asking for it");
}

#[test]
fn banners_of_grbl_and_its_relatives() {
    let versions: Vec<_> = fixture("banners.txt")
        .iter()
        .map(|l| match parse(l) {
            Reply::Banner(v) => v,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(versions, ["1.1h", "0.9j", "1.1f", "3.7.8"]);
}

#[test]
fn settings_and_info() {
    let mut settings = Settings::default();
    let lines = fixture("settings.txt");
    assert_eq!(lines.last().map(String::as_str), Some("ok"));
    for line in &lines {
        if let Reply::Setting(n, v) = parse(line) {
            settings.set(n, &v);
        }
    }
    assert_eq!(settings.laser_mode(), Some(true));
    assert_eq!(settings.s_max(), Some(1000));
    assert_eq!(settings.travel(), Some([400.0, 300.0]));
    assert!(!settings.reports_inches());
    let info = fixture("info.txt");
    assert_eq!(parse(&info[0]), Reply::Other("[VER:1.1h.20190825:]".into()));
    assert_eq!(parse(&info[2]), Reply::Ok);
}

#[test]
fn answers_alarms_and_messages() {
    let replies: Vec<_> = fixture("answers.txt").iter().map(|l| parse(l)).collect();
    assert_eq!(
        replies,
        [
            Reply::Ok,
            Reply::Error(22),
            Reply::Error(9),
            Reply::Alarm(1),
            Reply::Message("Reset to continue".into()),
            Reply::Message("'$H'|'$X' to unlock".into()),
            Reply::Message("Caution: Unlocked".into()),
            Reply::Alarm(3),
        ]
    );
}
