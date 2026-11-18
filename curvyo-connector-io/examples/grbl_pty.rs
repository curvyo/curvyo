//! A GRBL 1.1 simulator on a pseudo-terminal for the manual demo (`specs/0127` G22): prints the
//! slave's path, then answers on it until stopped. Run the connector with `--serial <that path>`.
//! Not part of the shipped connector; Linux and macOS.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[cfg(unix)]
#[path = "../tests/common/grbl_sim/mod.rs"]
mod grbl_sim;

#[cfg(not(unix))]
fn main() {
    eprintln!("grbl_pty needs a pseudo-terminal (Linux or macOS)");
}

#[cfg(unix)]
fn main() {
    use std::io::{Read as _, Write as _};
    use std::time::{Duration, Instant};

    use serialport::{SerialPort as _, TTYPort};

    /// One simulator step of real time.
    const STEP: Duration = Duration::from_millis(10);

    let (mut master, slave) = TTYPort::pair().expect("a pseudo-terminal pair");
    let path = slave.name().expect("the slave's path");
    drop(slave);
    println!("GRBL simulator on {path}");
    master
        .set_timeout(Duration::from_millis(2))
        .expect("a read timeout");
    let mut sim = grbl_sim::GrblSim::new(grbl_sim::SimConfig {
        steps_per_motion: 50,
        ..grbl_sim::SimConfig::default()
    });
    let mut buf = [0_u8; 512];
    let mut next = Instant::now();
    loop {
        let n = master.read(&mut buf).unwrap_or(0);
        sim.receive(&buf[..n]);
        while Instant::now() >= next {
            let out = sim.step();
            if !out.is_empty() {
                let _ = master.write_all(&out);
            }
            next += STEP;
        }
    }
}
