//! A22 to A25, A34 (`specs/0128`): `curvyoctl` sets a fresh connector up in process, with the
//! texts, exit codes and local state of the UX notes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{CONFIG, computer, connector, connector_in};
use curvyo_connector_ctl::KNOWN_FILE;
use curvyo_connector_io::hub::HubMode;

#[test]
fn a23_a34_identify_ports_and_apply_config_with_the_fingerprint() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    let address = c.address();
    let out = marc.ok(&["identify", "--insecure", &address]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Name:     workshop-pi");
    assert_eq!(lines[1], "Software: Curvyo connector 0.1.0, protocol 0");
    assert!(
        lines[2].starts_with("Mode:     Maintenance mode, closes in 29 min ("),
        "{out}"
    );
    assert!(lines[2].ends_with(" UTC)"), "{out}");
    assert_eq!(lines[3], "Setup:    open to the local network");
    assert_eq!(
        lines[4],
        format!("Key:      {}", c.server.spki().fingerprint())
    );
    // `identify` reads `Hello` only and stores nothing (UX notes 4.1).
    assert!(!marc.dir().join(KNOWN_FILE).exists());
    let ports = marc.ok(&["ports", "--insecure", &address]);
    assert!(
        ports == "No serial ports\n" || ports.starts_with("/dev/"),
        "{ports}"
    );
    // A fingerprint that differs refuses before anything is sent.
    let config = marc.config.display().to_string();
    let wrong = "0000-0000-0000-0000-0000-0000-0000-0000";
    let ran = marc.run(&[
        "apply-config",
        "--insecure",
        &address,
        "--fingerprint",
        wrong,
        "-f",
        &config,
    ]);
    assert_eq!(ran.code, 1);
    assert_eq!(
        ran.err,
        format!(
            "curvyoctl: the key at {address} differs from --fingerprint\nExpected key:  0000 0000 0000 0000 0000 0000 0000 0000\nPresented key: {}\n",
            c.server.spki().fingerprint()
        )
    );
    assert!(matches!(c.server.mode(), HubMode::Maintenance(_)));
    let out = marc.setup(&c);
    assert_eq!(
        out,
        format!(
            "Configured. You are the owner.\nConnector:  workshop-laser\nController: Fake controller: nothing moves\nKey:        {}\n",
            c.server.spki().fingerprint()
        )
    );
    assert!(matches!(c.server.mode(), HubMode::Configured(_)));
    // get-config round-trips the file.
    assert_eq!(marc.ok(&["get-config"]), CONFIG);
    let status = marc.ok(&["status"]);
    assert_eq!(
        status,
        "Connector:  workshop-laser, belongs to Marc\nController: Fake controller\nState:      Idle\n"
    );
    let ran = marc.run(&["apply-config", "--insecure", &address, "-f", &config]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: workshop-laser is already configured\n")
    );
}

#[test]
fn a24_a36_the_token_is_only_in_the_credential_store_and_the_files_are_private() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let name = format!("curvyoctl/{}", c.server.spki().to_hex());
    let record = marc.ctx.store().unwrap().get(&name).unwrap().unwrap();
    let record: serde_json::Value = serde_json::from_slice(&record).unwrap();
    let token = record["token"].as_str().unwrap().to_owned();
    assert_eq!(token.len(), 64);
    for entry in std::fs::read_dir(marc.dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap() == curvyo_storage_io::KEY_FILE {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !String::from_utf8_lossy(&bytes).contains(&token),
            "{}",
            path.display()
        );
    }
    let known = std::fs::read_to_string(marc.dir().join(KNOWN_FILE)).unwrap();
    assert!(known.contains("name = \"workshop-laser\""), "{known}");
    assert!(known.contains(&c.server.spki().to_hex()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(marc.dir()), 0o700);
        assert_eq!(mode(&marc.dir().join(KNOWN_FILE)), 0o600);
    }
}

#[test]
fn a23_after_a_reset_on_the_host_the_stored_key_stays_authoritative_until_a_matching_fingerprint() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let address = c.address();
    let old = c.server.spki().fingerprint();
    // The host-side reset: stop, empty, start again on the same port.
    let port = c.server.addr().port();
    let dir = c.dir;
    c.server.shutdown();
    curvyo_connector_io::state_dir::reset(dir.path()).unwrap();
    let service = curvyo_connector_io::server::ServiceConfig {
        listen: format!("127.0.0.1:{port}").parse().unwrap(),
        state_dir: dir.path().to_owned(),
        host_name: "workshop-pi".into(),
        timing: curvyo_connector_io::server::ServerTiming::default(),
    };
    let again = common::Connector {
        server: curvyo_connector_io::server::RunningServer::start_service(&service).unwrap(),
        dir,
    };
    let new = again.server.spki().fingerprint();
    let config = marc.config.display().to_string();
    let ran = marc.run(&["apply-config", "--insecure", &address, "-f", &config]);
    assert_eq!(ran.code, 1);
    assert_eq!(
        ran.err,
        format!(
            "curvyoctl: the key of workshop-laser changed\nStored key:    {old}\nPresented key: {new}\n"
        )
    );
    let ran = marc.run(&["status"]);
    assert_eq!(ran.code, 1);
    assert!(
        ran.err
            .starts_with("curvyoctl: the key of workshop-laser changed\n"),
        "{ran:?}"
    );
    // The way on: a --fingerprint that matches the presented key replaces the entry.
    let out = marc.setup(&again);
    assert!(out.starts_with("Configured. You are the owner.\n"), "{out}");
    assert_eq!(
        marc.ok(&["whoami"]).lines().nth(3),
        Some("Role:      owner")
    );
}

#[test]
fn a22_a34_usage_errors_exit_2_with_the_usage_line_and_help_fits_80_columns() {
    let mut marc = computer("marc-laptop");
    let ran = marc.run(&["frob"]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (2, "curvyoctl: unknown command frob\n")
    );
    let ran = marc.run(&["pair", "--name"]);
    assert_eq!(ran.code, 2);
    assert_eq!(
        ran.err,
        "curvyoctl: --name needs a value\nUsage: curvyoctl pair [<connector>] --name <name> [--owner]\n"
    );
    let ran = marc.run(&["status"]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (2, "curvyoctl: no connector is known\n")
    );
    let ran = marc.run(&["login", "127.0.0.1:1", "--code", "K7QM"]);
    assert_eq!(ran.code, 2);
    assert!(
        ran.err
            .starts_with("curvyoctl: --code is not a pairing code\n")
    );
    let ran = marc.run(&[
        "apply-config",
        "--insecure",
        "127.0.0.1:1",
        "-f",
        "/nonexistent/connector.toml",
    ]);
    assert_eq!(ran.code, 1);
    assert!(
        ran.err
            .starts_with("curvyoctl: cannot read /nonexistent/connector.toml: "),
        "{ran:?}"
    );
    let bad = marc.home.path().join("bad.toml");
    std::fs::write(&bad, CONFIG.replace("2000", "0")).unwrap();
    let bad = bad.display().to_string();
    let ran = marc.run(&["apply-config", "--insecure", "127.0.0.1:1", "-f", &bad]);
    assert_eq!(
        ran.err,
        format!("curvyoctl: {bad}: controller.lines_per_second: 0 is not from 1 to 10,000\n")
    );
    let ran = marc.run(&["identify", "--insecure", "127.0.0.1:1"]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: nothing answers at 127.0.0.1:1\n")
    );
    for line in [
        &["--help"][..],
        &["pair", "--help"],
        &["apply-config", "--help"],
    ] {
        let out = marc.ok(line);
        for l in out.lines() {
            assert!(l.len() <= 80 && l.is_ascii(), "{l}");
        }
    }
    assert_eq!(marc.ok(&["version"]), "curvyoctl 0.1.0, protocol 0\n");
}

#[test]
fn a25_a_lost_connector_is_left_by_logout_alone() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    drop(c);
    assert_eq!(
        marc.ok(&["logout"]),
        "Removed workshop-laser from this computer\n"
    );
    assert_eq!(marc.run(&["status"]).code, 2);
    let _ = connector_in;
}
