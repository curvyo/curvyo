//! The virtual GRBL 1.1 on a pseudo-terminal (`specs/0161-cnc-output` criterion 15, `adrs.md`
//! CN5): prints the slave's path, then answers on it as a GRBL router (the Generic GRBL router's
//! settings) until stopped. Run the connector with `--serial <that path> --spindle`. By default
//! its clock runs as fast as the host allows; `--real-time` runs it at the wall clock for the
//! demo. Not part of the shipped connector; Linux and macOS.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[cfg(not(unix))]
fn main() {
    eprintln!("grbl_virtual needs a pseudo-terminal (Linux or macOS)");
}

#[cfg(unix)]
fn main() {
    use std::io::{Read as _, Write as _};
    use std::time::{Duration, Instant};

    use curvyo_connector_core::virtual_grbl::{GrblSettings, VirtualGrbl};
    use serialport::{SerialPort as _, TTYPort};

    /// The machine time one read of the port lets pass when the clock runs fast.
    const FAST_STEP_S: f64 = 0.05;

    let real_time = std::env::args().any(|a| a == "--real-time");
    let mut settings = GrblSettings::default();
    // The Generic GRBL router (`0161` criterion 2): spindle up to 12 000 rpm, 300 x 300 x 60 mm.
    for (n, v) in [(30, "12000"), (130, "300"), (131, "300"), (132, "60")] {
        settings.set(n, v).expect("a GRBL setting");
    }
    let (mut master, slave) = TTYPort::pair().expect("a pseudo-terminal pair");
    let path = slave.name().expect("the slave's path");
    drop(slave);
    println!("Virtual GRBL 1.1 on {path}");
    println!(
        "Clock: {}",
        if real_time {
            "real time"
        } else {
            "as fast as the host allows"
        }
    );
    master
        .set_timeout(Duration::from_millis(2))
        .expect("a read timeout");
    let mut grbl = VirtualGrbl::new(settings);
    let mut buf = [0_u8; 512];
    let mut last = Instant::now();
    loop {
        let n = master.read(&mut buf).unwrap_or(0);
        grbl.receive(&buf[..n]);
        let now = Instant::now();
        let step = if real_time {
            (now - last).as_secs_f64()
        } else {
            FAST_STEP_S
        };
        last = now;
        let out = grbl.advance(step);
        if !out.is_empty() {
            let _ = master.write_all(&out);
        }
    }
}
