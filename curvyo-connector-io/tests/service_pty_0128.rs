//! A3, A6 end to end (`specs/0128`, addendum D16): a connector in maintenance mode whose later
//! configuration names a real pty writes zero bytes to it until that configuration is applied,
//! whatever it is sent before; then the GRBL controller opens the port and identifies.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

mod common;

use std::io::Read as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::service::{code_of, deadline, fake_config, service};
use curvyo_connector_core::{ControllerConfig, ErrorCode, JobKind, Message};
use serialport::{SerialPort as _, TTYPort};

#[test]
fn a3_the_port_of_the_later_configuration_reads_zero_bytes_until_it_is_applied() {
    let (mut master, slave) = TTYPort::pair().unwrap();
    let slave_path = slave.name().unwrap();
    drop(slave);
    master.set_timeout(Duration::from_millis(5)).unwrap();
    let read = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (read, stop) = (Arc::clone(&read), Arc::clone(&stop));
        std::thread::spawn(move || {
            let mut buf = [0_u8; 256];
            while !stop.load(Ordering::Relaxed) {
                let n = master.read(&mut buf).unwrap_or(0);
                read.fetch_add(n, Ordering::SeqCst);
            }
        })
    };
    let s = service();
    let mut c = s.insecure();
    let job_id = uuid::Uuid::nil();
    for message in [
        Message::SubmitJob {
            job_id,
            name: "x.gcode".into(),
            kind: JobKind::Job,
            bytes: 6,
            sha256: "00".repeat(32),
        },
        Message::Start { job_id },
        Message::Home,
        Message::Unlock,
        Message::Resume,
    ] {
        let answer = c.request(&message, deadline()).unwrap();
        assert_eq!(code_of(&answer), Some(ErrorCode::Maintenance));
    }
    // Every refusal was answered, so it was handled: the port has seen nothing.
    assert_eq!(read.load(Ordering::SeqCst), 0);
    let mut config = fake_config(20);
    config.controller = ControllerConfig::GrblSerial {
        port: slave_path,
        baud: 115_200,
    };
    let answer = s.apply(&mut c, config);
    assert!(
        matches!(answer, Message::ConfigApplied { token: Some(_) }),
        "{answer:?}"
    );
    let until = Instant::now() + common::WAIT;
    while read.load(Ordering::SeqCst) == 0 {
        assert!(
            Instant::now() < until,
            "the controller never opened the port"
        );
        std::thread::yield_now();
    }
    stop.store(true, Ordering::Relaxed);
    drop(s);
    reader.join().unwrap();
}
