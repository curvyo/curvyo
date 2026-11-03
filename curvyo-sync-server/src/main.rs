//! `curvyo-sync-server`: reads its configuration from flags and environment, runs the relay and
//! shuts down on SIGTERM or SIGINT within 10 s (criteria 35 to 37).

#![forbid(unsafe_code)]

use std::time::Duration;

use anyhow::Context;
use curvyo_sync_server::{Config, Log, Server, USAGE};

/// The shutdown's own budget, inside criterion 37's 10 s from the signal.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(8);

fn main() -> anyhow::Result<()> {
    let config =
        match Config::from_args_and_env(std::env::args().skip(1), |key| std::env::var(key).ok()) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("curvyo-sync-server: {e}\n{USAGE}");
                std::process::exit(2);
            }
        };
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the async runtime")?
        .block_on(run(config))
}

async fn run(config: Config) -> anyhow::Result<()> {
    let log = Log::stdout();
    let stop = stop_signal()?;
    let server = Server::start(&config, log.clone()).await?;
    tokio::select! {
        () = stop => {}
        () = server.failed() => {}
    }
    let Ok(result) = tokio::time::timeout(SHUTDOWN_LIMIT, server.shutdown()).await else {
        log.line("shutdown took too long; exiting");
        return Ok(());
    };
    Ok(result?)
}

/// A future completing on SIGTERM or SIGINT; the handlers are installed before it returns.
#[cfg(unix)]
fn stop_signal() -> anyhow::Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).context("installing the SIGTERM handler")?;
    let mut int = signal(SignalKind::interrupt()).context("installing the SIGINT handler")?;
    Ok(async move {
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
    })
}

/// A future completing on Ctrl-C.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn stop_signal() -> anyhow::Result<impl Future<Output = ()>> {
    Ok(async {
        let _ = tokio::signal::ctrl_c().await;
    })
}
