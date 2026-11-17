//! The machine connector program: arguments, wiring and terminal input around
//! `curvyo-connector-io`'s server.

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod args;
mod terminal;

use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use args::{Args, Command};
use curvyo_connector_core::PROTOCOL_VERSION;
use curvyo_connector_io::clients::Clients;
use curvyo_connector_io::hub::FakeSettings;
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

fn run(args: &Args) -> Result<()> {
    let config = ServerConfig {
        listen: args.listen,
        state_dir: args.state_dir.clone(),
        fake: args.fake_controller.then_some(FakeSettings {
            lines_per_second: args.fake_lines_per_second,
            alarm_at_line: args.fake_alarm_at_line,
        }),
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
    } else {
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
    if let Some(notices) = server.take_notices() {
        terminal::print_notices(notices);
    }
    let server = Arc::new(server);
    terminal::read_commands(Arc::clone(&server));
    loop {
        std::thread::park();
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
