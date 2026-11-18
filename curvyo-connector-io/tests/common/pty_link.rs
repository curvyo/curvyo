//! The GRBL simulator on a real pseudo-terminal (Linux): a driver thread steps it by one quantum
//! per read of the master side, never by sleep. The connector opens a symlink to the slave, so
//! "unplug" drops the master and "replug" opens a new pair and moves the symlink.
#![cfg(target_os = "linux")]
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value
)]

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serialport::{SerialPort as _, TTYPort};

use super::grbl_sim::GrblSim;

/// The simulator behind a symlinked pty.
pub struct PtySim {
    pub sim: Arc<Mutex<GrblSim>>,
    pub link: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn drive(mut master: TTYPort, sim: Arc<Mutex<GrblSim>>, stop: Arc<AtomicBool>) {
    master.set_timeout(Duration::from_millis(5)).unwrap();
    let mut buf = [0_u8; 512];
    while !stop.load(Ordering::Relaxed) {
        // EIO while no slave is open, a timeout while it is quiet: nothing read.
        let n = master.read(&mut buf).unwrap_or(0);
        let out = {
            let mut sim = sim.lock().unwrap();
            sim.receive(&buf[..n]);
            sim.step()
        };
        if !out.is_empty() {
            let _ = master.write_all(&out);
        }
    }
}

impl PtySim {
    /// The simulator on a new pty whose slave is reached through `link`.
    pub fn plug(sim: GrblSim, link: &Path) -> Self {
        let mut pty = Self {
            sim: Arc::new(Mutex::new(sim)),
            link: link.to_owned(),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        pty.replug();
        pty
    }

    /// Drops the master: the connector's reads and writes fail as with a pulled cable.
    pub fn unplug(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }

    /// A new pty for the same machine, the symlink moved to its slave.
    pub fn replug(&mut self) {
        self.unplug();
        let (master, slave) = TTYPort::pair().unwrap();
        let slave_path = slave.name().unwrap();
        drop(slave);
        let _ = std::fs::remove_file(&self.link);
        std::os::unix::fs::symlink(&slave_path, &self.link).unwrap();
        self.stop = Arc::new(AtomicBool::new(false));
        let (sim, stop) = (Arc::clone(&self.sim), Arc::clone(&self.stop));
        self.thread = Some(std::thread::spawn(move || drive(master, sim, stop)));
    }
}

impl Drop for PtySim {
    fn drop(&mut self) {
        self.unplug();
    }
}
