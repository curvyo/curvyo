//! The `curvyo-sync-server` binary as a process (`specs/0092-cooperative-editing` criteria 35
//! to 37): a flag wins over its environment variable, and SIGTERM or SIGINT closes every socket
//! with code 1012 and exits within 10 s.

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

/// Starts the binary with `args` and `env`; returns it, its stdout and the address it listens on.
fn spawn(args: &[&str], env: &[(&str, &str)]) -> (Child, BufReader<ChildStdout>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_curvyo-sync-server"))
        .args(args)
        .envs(env.iter().copied())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let addr = line
        .trim()
        .strip_prefix("curvyo-sync-server: listening on ")
        .unwrap_or_else(|| panic!("first line {line:?}"))
        .to_owned();
    (child, stdout, addr)
}

fn wait_with_limit(child: &mut Child, limit: Duration) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    while start.elapsed() < limit {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    None
}

fn signal(child: &Child, name: &str) {
    let status = Command::new("kill")
        .args([name, &child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
}

/// Criterion 35: `--listen` and `--data` win over `CURVYO_SYNC_LISTEN` and `CURVYO_SYNC_DATA`.
#[test]
fn flags_win_over_the_environment() {
    let env_data = tempfile::tempdir().unwrap();
    let flag_data = tempfile::tempdir().unwrap();
    let flag_dir = flag_data.path().join("rooms");
    let (mut child, _, addr) = spawn(
        &[
            "--listen",
            "127.0.0.1:0",
            "--data",
            flag_dir.to_str().unwrap(),
        ],
        &[
            ("CURVYO_SYNC_LISTEN", "127.0.0.1:1"),
            (
                "CURVYO_SYNC_DATA",
                env_data.path().join("rooms").to_str().unwrap(),
            ),
        ],
    );
    assert!(addr.starts_with("127.0.0.1:") && addr != "127.0.0.1:1");
    signal(&child, "-INT");
    assert!(wait_with_limit(&mut child, Duration::from_secs(10)).is_some_and(|s| s.success()));
    assert!(flag_dir.is_dir(), "the flag's data directory is used");
    assert!(!env_data.path().join("rooms").exists());
}

/// Criterion 37: SIGTERM closes every open socket with 1012 and the process exits within 10 s;
/// the log holds no client address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_closes_sockets_with_1012_and_exits() {
    let data = tempfile::tempdir().unwrap();
    let (mut child, mut stdout, addr) = spawn(
        &[
            "--listen",
            "127.0.0.1:0",
            "--data",
            data.path().to_str().unwrap(),
        ],
        &[],
    );
    let (mut first, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/"))
        .await
        .unwrap();
    let (mut second, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/"))
        .await
        .unwrap();
    let local = {
        let socket = first.get_ref();
        match socket {
            tokio_tungstenite::MaybeTlsStream::Plain(tcp) => tcp.local_addr().unwrap(),
            _ => unreachable!(),
        }
    };

    let signalled = Instant::now();
    signal(&child, "-TERM");
    for socket in [&mut first, &mut second] {
        let message = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .unwrap();
        let Some(Ok(Message::Close(Some(frame)))) = message else {
            panic!("expected a close frame, got {message:?}");
        };
        assert_eq!(u16::from(frame.code), 1012);
    }
    let status = tokio::task::spawn_blocking(move || {
        let status = wait_with_limit(&mut child, Duration::from_secs(10));
        (status, child)
    })
    .await
    .unwrap()
    .0;
    assert!(status.is_some_and(|s| s.success()), "exited cleanly");
    assert!(signalled.elapsed() < Duration::from_secs(10));

    let mut rest = String::new();
    std::io::Read::read_to_string(&mut stdout, &mut rest).unwrap();
    assert!(rest.contains("shutting down"));
    assert!(
        !rest.contains(&local.to_string()),
        "no client address in the log"
    );
}
