//! The machine connector program: arguments, wiring and terminal input around
//! `curvyo-connector-io`'s server.

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![forbid(unsafe_code)]

mod args;
mod serial_notes;
#[cfg(unix)]
mod signals;
mod terminal;

use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use args::{Args, Command};
use curvyo_connector_core::PROTOCOL_VERSION;
use curvyo_connector_io::clients::Clients;
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings, SerialSettings};
use curvyo_connector_io::server::{RunningServer, ServerConfig};

fn host_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "curvyo-connector".to_owned())
}

fn list_clients(args: &Args) -> Result<()> {
    let clients = Clients::new(&args.state_dir).list()?;
    for client in clients {
        println!("{}\t{}", client.name, client.paired_at);
    }
    Ok(())
}

fn revoke(args: &Args, name: &str) -> Result<()> {
    if Clients::new(&args.state_dir).remove(name)? {
        println!("Revoked {name}");
        Ok(())
    } else {
        anyhow::bail!("no paired client is called {name}")
    }
}

fn controller(args: &Args) -> ControllerChoice {
    if let Some(port) = &args.serial {
        ControllerChoice::Serial(SerialSettings {
            port: port.clone(),
            baud: args.baud,
            timing: Timing::default(),
        })
    } else if args.fake_controller {
        ControllerChoice::Fake(FakeSettings {
            lines_per_second: args.fake_lines_per_second,
            alarm_at_line: args.fake_alarm_at_line,
        })
    } else {
        ControllerChoice::None
    }
}

fn run(args: &Args) -> Result<()> {
    // First, before any thread starts, so that every thread inherits the mask (C11).
    #[cfg(unix)]
    let signals = signals::block().context("cannot block SIGINT and SIGTERM")?;
    let config = ServerConfig {
        listen: args.listen,
        state_dir: args.state_dir.clone(),
        controller: controller(args),
        require_local_start: args.require_local_start,
        connector_name: host_name(),
        issue_code: args.pair,
    };
    let server = RunningServer::start(&config).context("cannot start")?;
    println!(
        "Curvyo connector {}, protocol {PROTOCOL_VERSION}",
        env!("CARGO_PKG_VERSION")
    );
    if args.fake_controller {
        println!("Fake controller: no machine is attached, nothing moves.");
    } else if args.serial.is_none() {
        println!("No controller is attached.");
    }
    let addr = server.addr();
    if addr.ip().is_unspecified() {
        println!("Listening on all addresses, port {}", addr.port());
    } else {
        println!("Listening on {addr}");
    }
    println!("Key fingerprint: {}", server.spki().fingerprint());
    if let Some(code) = server.current_code() {
        println!("Pairing code: {code}, valid for 10 minutes");
    }
    if let Some(port) = &args.serial {
        println!("Serial port {port}, {} baud", args.baud);
        let runs = serial_notes::modem_manager_runs(std::path::Path::new("/proc"));
        if let Some(line) = serial_notes::modem_manager_line(port, runs) {
            println!("{line}");
        }
    }
    let printer = server.take_notices().map(terminal::print_notices);
    let server = Arc::new(server);
    terminal::read_commands(Arc::clone(&server));
    #[cfg(unix)]
    {
        signals::wait(&signals).context("cannot wait for a signal")?;
        stop_for_exit(&server);
        // The serial thread's last write is done and every notice printed before "Exiting".
        server.close();
        if let Some(printer) = printer {
            let _ = printer.join();
        }
        println!("Exiting");
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = printer;
        loop {
            std::thread::park();
        }
    }
}

/// A running or paused job is stopped as a client's Stop would (G16); then the program exits.
/// A second signal during the stop is not awaited, so it changes nothing.
#[cfg(unix)]
fn stop_for_exit(server: &RunningServer) {
    use curvyo_connector_core::State;
    use std::time::{Duration, Instant};
    /// The longest a stop at exit may take: G12's hold and banner limits plus `M5` and `M9`.
    const EXIT_STOP: Duration = Duration::from_secs(6);

    if let Some(name) = server.stop_for_exit() {
        println!("Stopping {name}");
        let until = Instant::now() + EXIT_STOP;
        while matches!(server.state(), State::Running | State::Paused) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn main() -> ExitCode {
    let args = match args::parse(
        std::env::args().skip(1),
        dirs::data_dir().map(|d| d.join("curvyo-connector")),
    ) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("curvyo-connector-server: {e}\nTry --help.");
            return ExitCode::from(2);
        }
    };
    let result = match &args.command {
        Command::Help => {
            print!("{}", args::HELP);
            Ok(())
        }
        Command::ListClients => list_clients(&args),
        Command::Revoke(name) => revoke(&args, name),
        Command::Run => run(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("curvyo-connector-server: {e:#}");
            ExitCode::from(1)
        }
    }
}
