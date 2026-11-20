//! The supervisor of the controller's thread (addendum D14): it starts the fake's streamer or the
//! serial thread, and on a new configuration stops and joins the old one before it starts the
//! new, so a port is never open twice. It also tells the hub when the setup window closed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::grbl::Timing;
use crate::hub::{ControllerChoice, Hub, TICK};
use crate::server::choice_of;

/// One controller thread and its own stop flag.
struct Running {
    thread: JoinHandle<()>,
    stop: Arc<AtomicBool>,
}

impl Running {
    fn end(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
    }
}

fn spawn_streamer(hub: Arc<Hub>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(TICK);
            hub.tick(Instant::now());
        }
    })
}

/// The thread for `choice`; none for no controller.
fn start(hub: &Arc<Hub>, choice: &ControllerChoice) -> Option<Running> {
    let stop = Arc::new(AtomicBool::new(false));
    let thread = match choice {
        ControllerChoice::None => return None,
        ControllerChoice::Fake(_) => spawn_streamer(Arc::clone(hub), Arc::clone(&stop)),
        #[cfg(feature = "serial")]
        ControllerChoice::Serial(serial) => {
            crate::serial_link::spawn(serial.clone(), Arc::clone(hub), Arc::clone(&stop))
        }
        #[cfg(not(feature = "serial"))]
        ControllerChoice::Serial(serial) => {
            let detail = "This build has no serial ports".to_owned();
            hub.open_failed(&serial.port, detail, Instant::now());
            return None;
        }
    };
    Some(Running { thread, stop })
}

/// Starts the supervisor with the controller `initial`; it ends with `shutdown`, the controller's
/// thread first. `closes_at` is the window's end as the terminal prints it.
#[must_use]
pub fn spawn(
    hub: Arc<Hub>,
    initial: ControllerChoice,
    timing: Timing,
    closes_at: String,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let (requests, received) = channel();
    hub.set_supervisor(requests);
    std::thread::spawn(move || {
        let mut current = start(&hub, &initial);
        while !shutdown.load(Ordering::Relaxed) {
            match received.recv_timeout(TICK) {
                Ok(config) => {
                    if let Some(old) = current.take() {
                        old.end();
                    }
                    // The hub installed the new controller already; only its thread changes.
                    current = start(&hub, &choice_of(&config, timing));
                }
                Err(RecvTimeoutError::Timeout) => hub.check_window(Instant::now(), &closes_at),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        if let Some(old) = current.take() {
            old.end();
        }
    })
}
