//! The virtual GRBL against `specs/0161-cnc-output` criteria 15 to 18.

use super::*;

const TWO_PARTS: &str =
    include_str!("../../../curvyo-cnc-core/tests/fixtures/cnc_grbl_two_parts.nc");

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// One line in, run until it is answered; the output.
fn ask(g: &mut VirtualGrbl, line: &str) -> String {
    g.receive(format!("{line}\n").as_bytes());
    let mut out = String::new();
    for _ in 0..10_000 {
        out += &text(&g.advance(0.01));
        if out.contains("ok") || out.contains("error:") || out.contains("ALARM:") {
            break;
        }
    }
    out
}

/// Streams `job` with character counting as the connector does; the replies.
fn stream(g: &mut VirtualGrbl, job: &str) -> Vec<String> {
    let lines: Vec<String> = job.lines().map(crate::gcode_line::strip).collect();
    let mut in_flight: VecDeque<usize> = VecDeque::new();
    let mut next = 0;
    let mut replies = Vec::new();
    let mut pending = String::new();
    for _ in 0..10_000_000 {
        while next < lines.len() && in_flight.iter().sum::<usize>() + lines[next].len() < RX_SIZE {
            g.receive(format!("{}\n", lines[next]).as_bytes());
            in_flight.push_back(lines[next].len() + 1);
            next += 1;
        }
        pending += &text(&g.advance(0.005));
        while let Some(at) = pending.find("\r\n") {
            let line: String = pending.drain(..at + 2).collect();
            let line = line.trim_end().to_owned();
            if line == "ok" || line.starts_with("error:") {
                in_flight.pop_front();
                replies.push(line);
            }
        }
        if next == lines.len() && in_flight.is_empty() && !g.moving() {
            break;
        }
    }
    replies
}

#[test]
fn c15_banner_ok_status_hold_resume_reset_settings_homing_and_unlock() {
    let mut g = VirtualGrbl::new(GrblSettings::default());
    assert_eq!(text(&g.take_output()), "\r\nGrbl 1.1h ['$' for help]\r\n");
    assert_eq!(ask(&mut g, "G21 G90"), "ok\r\n");
    let settings = ask(&mut g, "$$");
    assert!(settings.contains("$110=500.000\r\n") && settings.ends_with("ok\r\n"));
    assert!(ask(&mut g, "$I").contains("[OPT:V,15,128]"));
    g.receive(b"?");
    assert!(text(&g.take_output()).starts_with("<Idle|MPos:0.000,0.000,0.000|FS:0,0"));
    ask(&mut g, "G1 X50 F500");
    g.advance(0.5);
    g.receive(b"!");
    g.advance(2.0);
    g.receive(b"?");
    assert!(text(&g.take_output()).starts_with("<Hold:0|"));
    g.receive(b"~");
    g.advance(0.1);
    g.receive(b"?");
    assert!(text(&g.take_output()).starts_with("<Run|"));
    g.receive(&[0x18]);
    let reset = text(&g.take_output());
    assert!(
        reset.starts_with("ALARM:3\r\n") && reset.contains("Grbl 1.1h"),
        "{reset}"
    );
    assert!(ask(&mut g, "G0 X1").contains("error:9"));
    assert!(ask(&mut g, "$X").contains("[MSG:Caution: Unlocked]"));
    assert_eq!(ask(&mut g, "$H"), "error:5\r\n");
    assert_eq!(ask(&mut g, "$22=1"), "ok\r\n");
    assert_eq!(ask(&mut g, "$H"), "ok\r\n");
}

#[test]
fn c16_each_refusal_has_grbl_1_1_s_number() {
    // From GRBL 1.1 `doc/csv/error_codes_en_US.csv` and `alarm_codes_en_US.csv`.
    let table = [
        ("M6 T1", "error:20"),
        ("G81 X1 Y1 Z-1 R1 F100", "error:20"),
        ("G1 X10", "error:22"),
        ("G0 G1 X1", "error:21"),
        ("G1 X1 X2 F100", "error:25"),
        ("$99=1", "error:3"),
        ("$20=1", "error:10"),
    ];
    for (line, want) in table {
        let mut g = VirtualGrbl::new(GrblSettings::default());
        g.take_output();
        assert_eq!(ask(&mut g, line), format!("{want}\r\n"), "{line}");
    }
    let mut g = VirtualGrbl::new(GrblSettings::default());
    g.take_output();
    let long = format!("G1X{}F100", "1".repeat(80));
    assert_eq!(ask(&mut g, &long), "error:11\r\n");
    // Soft limits need homing; a move beyond the travel then raises alarm 2.
    let mut settings = GrblSettings::default();
    settings.set(22, "1").unwrap();
    settings.set(20, "1").unwrap();
    let mut g = VirtualGrbl::new(settings);
    g.take_output();
    ask(&mut g, "$X");
    let alarm = ask(&mut g, "G0 X-500");
    assert!(
        alarm.starts_with("ALARM:2\r\n[MSG:Reset to continue]"),
        "{alarm}"
    );
}

fn router() -> GrblSettings {
    let mut s = GrblSettings::default();
    s.set(30, "12000").unwrap();
    for n in [130, 131] {
        s.set(n, "300").unwrap();
    }
    s
}

#[test]
fn c17_c18_the_two_parts_job_runs_to_its_last_move_in_the_planner_s_time() {
    let mut g = VirtualGrbl::new(router());
    g.take_output();
    let replies = stream(&mut g, TWO_PARTS);
    assert!(replies.iter().all(|r| r == "ok"), "{replies:?}");
    assert!(g.faults.is_empty(), "{:?}", g.faults);
    let last = TWO_PARTS
        .lines()
        .rev()
        .find(|l| l.starts_with("G0 X") || l.starts_with("G1 X"))
        .unwrap();
    let mut x = None;
    for word in last.split_whitespace() {
        if let Some(v) = word.strip_prefix('X') {
            x = v.parse::<f64>().ok();
        }
    }
    let end = g.position();
    assert!((end[0] - x.unwrap()).abs() < 1e-3, "{end:?} {last}");
    assert!((end[2] - 5.0).abs() < 1e-3);
    assert_eq!(g.spindle().0, Spindle::Off);
    assert!(g.executed.iter().any(|e| e.spindle_on));
    let planned = plan_time(TWO_PARTS, &router()).total.get();
    let ran = g.busy_time().get();
    assert!(
        (ran - planned).abs() <= 0.01 * planned,
        "{ran} vs {planned}"
    );
}
