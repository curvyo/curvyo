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
use curvyo_connector_io::hub::{ControllerChoice, ExitReason, FakeSettings, SerialSettings};
use curvyo_connector_io::server::{
    RunningServer, ServerConfig, ServerTiming, ServiceConfig, utc_clock,
};
use curvyo_connector_io::{admin_log, config_file, state_dir};

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
        let role = admin_log::role_word(client.role.into());
        println!("{}\t{}\t{role}", client.name, client.paired_at);
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

fn reset(args: &Args) -> Result<()> {
    state_dir::reset(&args.state_dir)?;
    println!("Deleted the contents of {}", args.state_dir.display());
    Ok(())
}

fn fingerprint(args: &Args) -> Result<()> {
    let spki = state_dir::fingerprint(&args.state_dir)?;
    println!("Key fingerprint: {}", spki.fingerprint());
    Ok(())
}

fn controller(args: &Args) -> ControllerChoice {
    if let Some(port) = &args.serial {
        ControllerChoice::Serial(SerialSettings {
            port: port.clone(),
            baud: args.baud,
            spindle: args.spindle,
            timing: Timing {
                homing: args.homing_timeout,
                ..Timing::default()
            },
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

fn print_listening(server: &RunningServer) {
    let addr = server.addr();
    if addr.ip().is_unspecified() {
        println!("Listening on all addresses, port {}", addr.port());
    } else {
        println!("Listening on {addr}");
    }
    println!("Key fingerprint: {}", server.spki().fingerprint());
}

/// Foreground mode (A28): the run flags of slices 2 and 4, held in memory, with the terminal.
fn start_foreground(args: &Args) -> Result<RunningServer> {
    let config = ServerConfig {
        listen: args.listen,
        state_dir: args.state_dir.clone(),
        controller: controller(args),
        require_local_start: args.require_local_start,
        connector_name: host_name(),
        issue_code: args.pair,
    };
    let server = RunningServer::start(&config).context("cannot start")?;
    print_version();
    if args.fake_controller {
        println!("Fake controller: no machine is attached, nothing moves.");
    } else if args.serial.is_none() {
        println!("No controller is attached.");
    }
    print_listening(&server);
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
    Ok(server)
}

/// The service (A1, A31): configured by `connector.toml`, else maintenance mode.
fn start_service(args: &Args) -> Result<RunningServer> {
    let service = ServiceConfig {
        listen: args.listen,
        state_dir: args.state_dir.clone(),
        host_name: host_name(),
        timing: ServerTiming::default(),
    };
    let server = RunningServer::start_service(&service).context("cannot start")?;
    print_version();
    match server.config() {
        Some(config) => {
            match &config.owner {
                Some(owner) => println!("Configured: {}, belongs to {owner}", config.name),
                None => println!("Configured: {}", config.name),
            }
            println!("Controller: {}", config.controller.describe());
            if let (Some(flag), Some(listen)) = (args.listen_flag, config.listen_addr())
                && listen != flag
            {
                println!("--listen {flag} ignored: connector.toml sets {listen}");
            }
        }
        None => println!(
            "Maintenance mode until {}: waiting for curvyoctl apply-config --insecure",
            utc_clock(ServerTiming::default().maintenance_window)
        ),
    }
    print_listening(&server);
    Ok(server)
}

fn print_version() {
    println!(
        "Curvyo connector {}, protocol {PROTOCOL_VERSION}",
        env!("CARGO_PKG_VERSION")
    );
}

fn run(args: &Args) -> Result<ExitCode> {
    // First, so that no thread can ever take the default action of SIGINT or SIGTERM (C11).
    #[cfg(unix)]
    let mut signals =
        signals::install().context("cannot install the SIGINT and SIGTERM handlers")?;
    let server = match &args.foreground_flag {
        Some(flag) => {
            let file = args.state_dir.join(config_file::CONFIG_FILE);
            if file.exists() {
                eprintln!(
                    "curvyo-connector-server: {flag} cannot be used: {} exists",
                    file.display()
                );
                return Ok(ExitCode::from(2));
            }
            start_foreground(args)?
        }
        None => start_service(args)?,
    };
    let exits = server.take_exits();
    #[cfg(unix)]
    {
        let sender = server.exit_sender();
        std::thread::spawn(move || {
            signals::wait(&mut signals);
            let _ = sender.send(ExitReason::Signal);
        });
    }
    let printer = server.take_notices().map(terminal::print_notices);
    let server = Arc::new(server);
    if args.foreground_flag.is_some() {
        terminal::read_commands(Arc::clone(&server));
    }
    let reason = exits
        .and_then(|e| e.recv().ok())
        .unwrap_or(ExitReason::Signal);
    let code = match reason {
        ExitReason::Signal => {
            stop_for_exit(&server);
            // The serial thread's last write is done and every notice printed before "Exiting".
            server.close();
            finish(printer);
            println!("Exiting");
            ExitCode::SUCCESS
        }
        ExitReason::Reset { by } => {
            server.close();
            finish(printer);
            // The lock goes last; the directory itself stays (A19, A30).
            state_dir::empty(&args.state_dir)?;
            println!("Reset by {by}: state directory deleted");
            ExitCode::SUCCESS
        }
    };
    Ok(code)
}

fn finish(printer: Option<std::thread::JoinHandle<()>>) {
    if let Some(printer) = printer {
        let _ = printer.join();
    }
}

/// A running or paused job or a go to point is stopped as a client's Stop would (G16); then the
/// program exits.
/// A second signal during the stop is not awaited, so it changes nothing.
fn stop_for_exit(server: &RunningServer) {
    if let Some(name) = server.stop_for_exit() {
        println!("Stopping {name}");
        server.await_exit_stop();
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
            Ok(ExitCode::SUCCESS)
        }
        Command::ListClients => list_clients(&args).map(|()| ExitCode::SUCCESS),
        Command::Revoke(name) => revoke(&args, name).map(|()| ExitCode::SUCCESS),
        Command::Reset => reset(&args).map(|()| ExitCode::SUCCESS),
        Command::Fingerprint => fingerprint(&args).map(|()| ExitCode::SUCCESS),
        Command::Run => run(&args),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("curvyo-connector-server: {e:#}");
            ExitCode::from(1)
        }
    }
}
