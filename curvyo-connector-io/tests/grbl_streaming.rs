//! `specs/0127` identify, status, streaming and completion against the GRBL simulator over an
//! in-memory channel with stepped time (G2 to G9, G17, G22). One test per behaviour.
#![allow(
    clippy::unwrap_used,
    clippy::format_push_string,
    clippy::naive_bytecount
)]

mod common;

use std::time::Duration;

use common::grbl_bench::{Bench, PORT};
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::controller::Controller;
use curvyo_connector_io::grbl::prepare::prepare;

const CUT_LINE: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/cut_line.gcode");
const COMPLETED: State = State::Done {
    result: JobResult::Completed,
};

fn golden_jobs() -> Vec<(String, String)> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../curvyo-laser-core/tests/fixtures/output/grbl"
    );
    let mut jobs: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "gcode"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(p).unwrap())
        })
        .collect();
    jobs.sort();
    assert!(jobs.len() >= 20, "every golden job of 0029 criterion 31");
    jobs
}

/// The lines the simulator received from line `mark` on.
fn job_lines_received(bench: &Bench, mark: usize) -> Vec<String> {
    bench.sim.lines[mark..].to_vec()
}

#[test]
fn g2_g17_reset_first_then_banner_info_settings_and_one_report() {
    let bench = Bench::connected(SimConfig::default());
    assert_eq!(bench.sim.log[0], 0x18, "the soft reset is the first byte");
    assert_eq!(&bench.sim.lines[..2], ["$I", "$$"]);
    assert_eq!(bench.state(), State::Idle);
    let info = bench.ctrl.info().unwrap();
    assert_eq!(
        (info.name.as_str(), info.firmware.as_str()),
        ("GRBL", "1.1h")
    );
    assert_eq!(info.port.as_deref(), Some(PORT));
    assert_eq!(info.baud, Some(115_200));
    let settings = info.settings.unwrap();
    assert_eq!(settings.laser_mode, Some(true));
    assert_eq!(settings.s_max, Some(1000));
    assert_eq!(
        settings.travel.map(|t| t.map(|m| m.0)),
        Some([400.0, 300.0])
    );
    assert_eq!(
        bench.notices[0],
        "GRBL 1.1h on /dev/ttyBench: laser mode on, max power 1000, travel 400 x 300 mm"
    );
}

#[test]
fn g2_a_silent_port_or_other_text_is_no_grbl_answer_after_five_seconds() {
    for garbage in [false, true] {
        let mut bench = Bench::new(SimConfig::default());
        bench.sim.silent = !garbage;
        bench.sim.garbage = garbage;
        bench.run_for(Duration::from_millis(4_990));
        assert!(bench.closed.is_none());
        bench.run_for(Duration::from_millis(20));
        assert_eq!(bench.state(), State::NoController);
        assert_eq!(
            bench.detail().as_deref(),
            Some("No GRBL answer on /dev/ttyBench")
        );
        let expected = if garbage {
            "No GRBL answer on /dev/ttyBench (got: AT+CGMI)"
        } else {
            "No GRBL answer on /dev/ttyBench"
        };
        assert_eq!(bench.closed.as_deref(), Some(expected));
        assert!(
            bench.last_write.is_empty(),
            "nothing sent to a device that is not GRBL"
        );
    }
}

#[test]
fn g2_grbl_0_9_is_not_supported() {
    let mut bench = Bench::new(SimConfig {
        version: "0.9j",
        ..SimConfig::default()
    });
    bench.run_for(Duration::from_millis(100));
    assert_eq!(bench.detail().as_deref(), Some("GRBL 0.9 is not supported"));
    assert_eq!(
        bench.closed.as_deref(),
        Some("GRBL 0.9 is not supported (on /dev/ttyBench)")
    );
}

#[test]
fn g4_polls_once_a_second_idle_and_five_times_while_running() {
    let mut bench = Bench::connected(SimConfig {
        steps_per_motion: 100,
        ..SimConfig::default()
    });
    let polls = |b: &Bench| b.sim.log.iter().filter(|x| **x == b'?').count();
    let before = polls(&bench);
    bench.run_for(Duration::from_secs(10));
    let idle = polls(&bench) - before;
    assert!((9..=11).contains(&idle), "{idle} polls in 10 s idle");
    bench.start(&"G1 X10 Y10 F100 S500\nG1 X0 Y0\n".repeat(20));
    let before = polls(&bench);
    bench.run_for(Duration::from_secs(4));
    assert_eq!(bench.state(), State::Running);
    let running = polls(&bench) - before;
    assert!(
        (19..=21).contains(&running),
        "{running} polls in 4 s running"
    );
}

#[test]
fn g4_the_position_is_the_work_position_in_both_report_modes() {
    for mpos in [false, true] {
        let mut bench = Bench::connected(SimConfig {
            mpos_reports: mpos,
            wco: [10.0, 20.0, 0.0],
            ..SimConfig::default()
        });
        bench.start("G0 X50 Y60\n");
        assert!(bench.run_until(|b| b.state() == COMPLETED, 2_000));
        bench.run_for(Duration::from_secs(2));
        let Message::Status { position, .. } = bench.status() else {
            panic!()
        };
        let p = position.unwrap().map(|m| m.0);
        // The simulator moves the machine to the target; the job's coordinates are work ones.
        assert!(
            (p[0] - 40.0).abs() < 1e-6 && (p[1] - 40.0).abs() < 1e-6,
            "{p:?} (MPos {mpos})"
        );
    }
}

#[test]
fn g5_comments_with_realtime_characters_never_reach_the_controller() {
    let job = "; Curvyo job: Hello! ~?\n; Machine: Diode? (x!)\nG21\nG90 (go~)\nM4 S0\nG1 X10 Y10 F600 S500 ; done!\nM5\n";
    let mut bench = Bench::connected(SimConfig::default());
    let mark = bench.sim.lines.len();
    bench.start(job);
    assert!(bench.run_until(|b| b.state() == COMPLETED, 3_000));
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
    assert_eq!(
        job_lines_received(&bench, mark),
        ["G21", "G90", "M4S0", "G1X10Y10F600S500", "M5"]
    );
    assert!(!bench.pushed.iter().any(|p| matches!(
        p.status,
        Message::Status {
            state: State::Paused,
            ..
        }
    )));
    assert_eq!(bench.line(), 7);
}

#[test]
fn g5_a_refused_line_fails_the_job_before_the_first_byte() {
    let table = [
        (
            "G21\nG90\nM4 S0\n$32=0\nG1 X1\n",
            "Failed at line 4: Changes a machine setting",
        ),
        ("G21\n$X\n", "Failed at line 2: Unlocks the machine"),
        ("G21\nG1 X1 ! Y2\n", "Failed at line 2: Contains !, ~ or ?"),
    ];
    for (job, detail) in table {
        let mut bench = Bench::connected(SimConfig::default());
        let before = bench.sim.lines.len();
        bench.start(job);
        assert_eq!(
            bench.state(),
            State::Done {
                result: JobResult::Failed
            }
        );
        assert_eq!(bench.detail().as_deref(), Some(detail));
        bench.run_for(Duration::from_secs(1));
        assert_eq!(
            bench.sim.lines.len(),
            before,
            "no line of the job was written"
        );
    }
    let mut bench = Bench::connected(SimConfig::default());
    bench.start(&format!("G1 X{}\n", "1".repeat(80)));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Failed at line 1: Longer than 79 characters")
    );
}

fn stream_and_check(name: &str, job: &str) {
    let mut bench = Bench::connected(SimConfig::default());
    bench.sim.set_setting(22, "1");
    let mark = bench.sim.lines.len();
    bench.start(job);
    let lines = job.lines().count();
    let done = bench.run_until(|b| b.state() == COMPLETED, 200 * lines + 5_000);
    assert!(done, "{name}: {:?} {:?}", bench.state(), bench.detail());
    assert!(
        bench.sim.faults.is_empty(),
        "{name}: {:?}",
        bench.sim.faults
    );
    let sent: Vec<String> = prepare(job)
        .unwrap()
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        job_lines_received(&bench, mark),
        sent,
        "{name}: in order, byte for byte"
    );
    assert!(!bench.sim.laser_lit(), "{name}");
}

#[test]
fn g6_every_golden_job_of_0029_streams_without_overflow_and_in_order() {
    for (name, job) in golden_jobs() {
        stream_and_check(&name, &job);
    }
}

#[test]
fn g6_a_20000_line_job_streams_without_overflow() {
    let mut job = String::from("G21\nG90\nM4 S0\n");
    for i in 0..19_996 {
        job += &format!(
            "G1 X{}.{} Y{} S{}\n",
            i % 300,
            i % 10,
            (i * 7) % 200,
            i % 1000
        );
    }
    job += "M5\n";
    assert_eq!(job.lines().count(), 20_000);
    stream_and_check("20000 lines", &job);
}

#[test]
fn g8_completed_only_after_the_last_ok_and_an_idle_report() {
    let mut bench = Bench::connected(SimConfig {
        steps_per_motion: 80,
        ..SimConfig::default()
    });
    bench.start(CUT_LINE);
    let lines = u32::try_from(CUT_LINE.lines().count()).unwrap();
    assert!(bench.run_until(|b| b.line() == lines, 5_000));
    assert!(
        bench.sim.planner_busy(),
        "the head still moves after the last ok"
    );
    assert_eq!(bench.state(), State::Running);
    assert!(bench.run_until(|b| b.state() != State::Running, 5_000));
    assert_eq!(bench.state(), COMPLETED);
    assert!(!bench.sim.planner_busy());
    let Message::Status { job, .. } = bench.status() else {
        panic!()
    };
    assert_eq!(job.unwrap().line, lines);
}

#[test]
fn g9_homing_first_waits_without_a_limit_and_polling_goes_on() {
    let job = include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");
    let mut bench = Bench::connected(SimConfig {
        homing_steps: 900, // 9 s: longer than the 3 s loss limit
        ..SimConfig::default()
    });
    bench.sim.set_setting(22, "1");
    bench.start(job);
    bench.run_for(Duration::from_secs(5));
    assert_eq!(bench.state(), State::Running);
    assert_eq!(
        bench.line(),
        3,
        "the comment lines count, $H is still running"
    );
    assert_eq!(bench.detail().as_deref(), Some("Controller state: Home"));
    assert_eq!(
        bench.sim.lines.last().map(String::as_str),
        Some("$H"),
        "nothing after $H"
    );
    assert!(bench.run_until(|b| b.state() == COMPLETED, 5_000));
    assert!(bench.closed.is_none());
}

#[test]
fn g9_stop_during_homing_resets_without_a_hold_and_shows_the_alarm() {
    let job = include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");
    let mut bench = Bench::connected(SimConfig {
        homing_steps: 900,
        ..SimConfig::default()
    });
    bench.sim.set_setting(22, "1");
    bench.start(job);
    bench.run_for(Duration::from_secs(1));
    let mark = bench.sim.log.len();
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == State::Alarm, 500));
    assert!(
        !bench.sim.log[mark..].contains(&b'!'),
        "GRBL ignores ! while homing"
    );
    let stopped = bench.pushed.iter().any(|p| {
        matches!(
            p.status,
            Message::Status {
                state: State::Done {
                    result: JobResult::Stopped
                },
                ..
            }
        )
    });
    assert!(
        stopped,
        "Done {{ Stopped }} after the banner, then the alarm"
    );
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset during homing (alarm 6)")
    );
    assert_eq!(bench.sim.spindle, Spindle::Off);
}

#[test]
fn g21_the_byte_transcript_of_cut_line_is_golden() {
    let mut bench = Bench::connected(SimConfig::default());
    bench.start(CUT_LINE);
    assert!(bench.run_until(|b| b.state() == COMPLETED, 3_000));
    // `?` polls depend on the poll period only; the transcript keeps everything else.
    let sent: Vec<u8> = bench
        .sim
        .log
        .iter()
        .copied()
        .filter(|b| *b != b'?')
        .collect();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/grbl/cut_line_transcript.txt"
    );
    if std::env::var_os("CURVYO_WRITE_GOLDEN").is_some() {
        std::fs::write(path, &sent).unwrap();
    }
    assert_eq!(
        String::from_utf8_lossy(&sent),
        std::fs::read_to_string(path).unwrap()
    );
}
