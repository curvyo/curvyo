//! A connector run as a service in a temporary state directory and `curvyoctl` run in process
//! against it, with its own folder and a file credential store (addendum D16).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use curvyo_connector_ctl::{Context, run_code};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, connect, open_paired};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerTiming, ServiceConfig};
use curvyo_connector_io::token::Token;
use curvyo_storage_io::CredentialStore;

pub const WAIT: Duration = Duration::from_secs(20);

pub const CONFIG: &str = "format = 1\nname = \"workshop-laser\"\nowner = \"Marc\"\n\n[controller]\nkind = \"fake\"\nlines_per_second = 2000\n";

pub struct Connector {
    pub server: RunningServer,
    pub dir: tempfile::TempDir,
}

pub fn connector_in(dir: tempfile::TempDir) -> Connector {
    let service = ServiceConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir.path().to_owned(),
        host_name: "workshop-pi".into(),
        timing: ServerTiming::default(),
    };
    Connector {
        server: RunningServer::start_service(&service).unwrap(),
        dir,
    }
}

pub fn connector() -> Connector {
    connector_in(tempfile::tempdir().unwrap())
}

impl Connector {
    pub fn address(&self) -> String {
        self.server.addr().to_string()
    }

    pub fn fingerprint(&self) -> String {
        self.server.spki().fingerprint().replace(' ', "-")
    }
}

/// One computer running `curvyoctl`.
pub struct Computer {
    pub ctx: Context,
    pub home: tempfile::TempDir,
    pub config: PathBuf,
}

pub fn computer(host: &str) -> Computer {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join("curvyoctl");
    let store = CredentialStore::file(&dir, "test").unwrap();
    let config = home.path().join("connector.toml");
    std::fs::write(&config, CONFIG).unwrap();
    Computer {
        ctx: Context::with_store(dir, host, store),
        home,
        config,
    }
}

/// A computer with no credential store (no keychain, no terminal for a passphrase).
pub fn computer_without_store(host: &str) -> Computer {
    let mut computer = computer(host);
    let dir = computer.home.path().join("curvyoctl");
    computer.ctx = Context::without_store(dir, host);
    computer
}

/// What one run printed and its exit code.
#[derive(Debug)]
pub struct Ran {
    pub code: u8,
    pub out: String,
    pub err: String,
}

impl Computer {
    pub fn run(&mut self, line: &[&str]) -> Ran {
        let args: Vec<String> = line.iter().map(|a| (*a).to_owned()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_code(&args, &mut self.ctx, &mut out, &mut err);
        Ran {
            code,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }

    /// Runs and expects exit 0.
    pub fn ok(&mut self, line: &[&str]) -> String {
        let ran = self.run(line);
        assert_eq!(ran.code, 0, "{line:?}: {ran:?}");
        assert_eq!(ran.err, "", "{line:?}");
        ran.out
    }

    /// Sets `connector` up from this computer with the matching fingerprint.
    pub fn setup(&mut self, connector: &Connector) -> String {
        let config = self.config.display().to_string();
        let fingerprint = connector.fingerprint();
        self.ok(&[
            "apply-config",
            "--insecure",
            &connector.address(),
            "--fingerprint",
            &fingerprint,
            "-f",
            &config,
        ])
    }

    pub fn dir(&self) -> &Path {
        &self.ctx.dir
    }
}

/// The code in `pair`'s output.
pub fn code_of(out: &str) -> String {
    out.lines()
        .next()
        .and_then(|l| l.strip_prefix("Pairing code: "))
        .and_then(|l| l.split(',').next())
        .unwrap()
        .to_owned()
}

/// An app paired with `code`; its token.
pub fn app_pairs(connector: &Connector, code: &str) -> Token {
    let mut app = connect(&connector.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    app.pair(code, "app", Instant::now() + WAIT).unwrap().token
}

pub fn app_login(connector: &Connector, token: &Token) -> Connection {
    open_paired(
        &connector.address(),
        connector.server.spki(),
        token,
        CONNECT_TIMEOUT,
    )
    .unwrap()
    .0
}
