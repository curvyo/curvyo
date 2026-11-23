//! A10, A11, A13 to A15, A18, A19, A34 to A38 (`specs/0128`): `curvyoctl` as the owner of a
//! configured connector, and a second computer that logs in with an owner code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Instant;

use common::{WAIT, app_login, app_pairs, code_of, computer, computer_without_store, connector};
use curvyo_connector_core::{Message, State};
use curvyo_connector_io::hub::ExitReason;

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

fn wait_state(c: &mut curvyo_connector_io::client::Connection, want: fn(State) -> bool) {
    let until = Instant::now() + WAIT;
    loop {
        if let Message::Status { state, .. } = c.recv_message(until).unwrap()
            && want(state)
        {
            return;
        }
    }
}

/// `specs/0118` slice 2a, criterion 37: a member's Home waits and `confirm-start` confirms it.
#[test]
fn c37_a_members_home_is_confirmed_from_curvyoctl_and_audited_as_home() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let code = code_of(&marc.ok(&["pair", "--name", "Phone"]));
    let mut phone = app_login(&c, &app_pairs(&c, &code));
    phone.send(&Message::Home).unwrap();
    wait_state(&mut phone, |s| s == State::AwaitingLocalStart);
    let status = marc.ok(&["status"]);
    assert!(
        status.contains("State:      Home requested by Phone\n"),
        "{status}"
    );
    assert_eq!(marc.ok(&["confirm-start"]), "Homing, requested by Phone\n");
    wait_state(&mut phone, |s| s == State::Homing);
    wait_state(&mut phone, |s| s == State::Idle);
    let audit = marc.ok(&["audit", "--last", "3"]);
    assert!(
        audit.contains("start confirmed: Home, requested by Phone"),
        "{audit}"
    );
    // A second Home that waits can be thrown away with `stop`.
    phone.send(&Message::Home).unwrap();
    wait_state(&mut phone, |s| s == State::AwaitingLocalStart);
    assert_eq!(marc.ok(&["stop"]), "Discarded Home\n");
}

#[test]
fn a10_a13_a14_a15_a18_a_member_start_confirmed_from_curvyoctl() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let out = marc.ok(&["pair", "--name", "Phone"]);
    let code = code_of(&out);
    assert_eq!(
        out,
        format!("Pairing code: {code}, valid for 10 minutes\nFor:          Phone, member\n")
    );
    assert_eq!(code.len(), 9);
    let mut phone = app_login(&c, &app_pairs(&c, &code));
    let ran = marc.run(&["confirm-start"]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: no start is waiting on workshop-laser\n")
    );
    let job = phone
        .send_job("sign.gcode", JOB, Instant::now() + WAIT)
        .unwrap();
    phone.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_state(&mut phone, |s| s == State::AwaitingLocalStart);
    let status = marc.ok(&["status"]);
    assert!(
        status.contains("State:      Start requested by Phone\n"),
        "{status}"
    );
    assert!(
        status.contains("Job:        sign.gcode, 6 lines\n"),
        "{status}"
    );
    assert!(status.contains("Sent by:    Phone\n"), "{status}");
    let ran = marc.run(&["confirm-start", "cut.gcode"]);
    assert_eq!(
        ran.err,
        "curvyoctl: workshop-laser waits for sign.gcode, not cut.gcode\n"
    );
    assert_eq!(
        marc.ok(&["confirm-start", "sign.gcode"]),
        "Started sign.gcode, requested by Phone\n"
    );
    wait_state(&mut phone, |s| matches!(s, State::Done { .. }));
    let jobs = marc.ok(&["jobs"]);
    let lines: Vec<&str> = jobs.lines().collect();
    assert_eq!(
        lines[0],
        "Time (UTC)           Job         Sent by  Started by             Result"
    );
    assert!(
        lines[1].ends_with("  sign.gcode  Phone    curvyoctl@marc-laptop  Completed"),
        "{jobs}"
    );
    let audit = marc.ok(&["audit", "--last", "5"]);
    for words in [
        "configured from 127.0.0.1",
        "code issued for Phone, member",
        "paired as member",
        "start confirmed: sign.gcode, requested by Phone",
    ] {
        assert!(audit.contains(words), "{words} in {audit}");
    }
    assert!(!audit.contains(&code), "{audit}");
    assert_eq!(marc.ok(&["stop"]), "No job\n");
    assert_eq!(marc.ok(&["unlock"]), "No alarm\n");
}

#[test]
fn a11_clients_revoke_and_the_last_owner() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let code = code_of(&marc.ok(&["pair", "--name", "Phone"]));
    app_pairs(&c, &code);
    let clients = marc.ok(&["clients"]);
    let lines: Vec<&str> = clients.lines().collect();
    assert_eq!(lines[0], "Name                   Role    Paired (UTC)");
    assert!(
        lines[1].starts_with("curvyoctl@marc-laptop  owner   20"),
        "{clients}"
    );
    assert!(lines[1].ends_with("  (this computer)"), "{clients}");
    assert!(
        lines[2].starts_with("Phone                  member  20"),
        "{clients}"
    );
    let ran = marc.run(&["revoke", "Phon"]);
    assert_eq!(
        ran.err,
        "curvyoctl: workshop-laser has no client called Phon\n"
    );
    let ran = marc.run(&["revoke", "curvyoctl@marc-laptop"]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (
            1,
            "curvyoctl: curvyoctl@marc-laptop is the last owner of workshop-laser\n"
        )
    );
    let ran = marc.run(&["pair", "--name", "curvyoctl@marc-laptop"]);
    assert_eq!(
        ran.err,
        "curvyoctl: curvyoctl@marc-laptop is the last owner of workshop-laser\n"
    );
    assert_eq!(marc.ok(&["revoke", "Phone"]), "Revoked Phone\n");
}

#[test]
fn a19_reset_checks_the_name_here_stops_the_service_and_forgets_it() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let ran = marc.run(&["reset", "--confirm", "workshop-lasr"]);
    assert_eq!(ran.code, 2);
    assert!(
        ran.err
            .starts_with("curvyoctl: --confirm workshop-lasr does not match workshop-laser\n"),
        "{ran:?}"
    );
    let ran = marc.run(&["reset"]);
    assert_eq!(ran.code, 2);
    assert!(
        ran.err
            .starts_with("curvyoctl: reset needs --confirm <connector name>\n")
    );
    assert_eq!(
        marc.ok(&["reset", "--confirm", "workshop-laser"]),
        "Reset workshop-laser: stopped, configuration, key, clients and logs deleted\nRemoved workshop-laser from this computer\n"
    );
    let exit = c.server.take_exits().unwrap().recv_timeout(WAIT).unwrap();
    assert_eq!(
        exit,
        ExitReason::Reset {
            by: "curvyoctl@marc-laptop".into()
        }
    );
    let common::Connector { server, dir } = c;
    server.wipe().unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    assert_eq!(marc.run(&["status"]).code, 2, "the connector is forgotten");
}

#[test]
fn a35_to_a38_a_second_computer_logs_in_as_an_owner() {
    let c = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    let address = c.address();
    let mut desk = computer("marc-desktop");
    // A member code stores nothing.
    let code = code_of(&marc.ok(&["pair", "--name", "Kiosk"]));
    let ran = desk.run(&["login", &address, "--code", &code]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: the code was issued for a member\n")
    );
    assert!(!desk.dir().join(curvyo_connector_ctl::KNOWN_FILE).exists());
    // A wrong code spends one attempt.
    let code = code_of(&marc.ok(&["pair", "--name", "Second", "--owner"]));
    let wrong = if code.starts_with('Z') {
        "YYYY-YYYY"
    } else {
        "ZZZZ-ZZZZ"
    };
    let ran = desk.run(&["login", &address, "--code", wrong]);
    assert_eq!(ran.err, "curvyoctl: wrong code, 2 attempts left\n");
    let lower = code.replace('-', "").to_lowercase();
    let out = desk.ok(&["login", &address, "--code", &lower]);
    assert_eq!(
        out,
        format!(
            "Logged in to workshop-laser. This computer is an owner.\nConnector:  workshop-laser\nAddress:    {address}\nName:       curvyoctl@marc-desktop\nStored key: {}\n",
            c.server.spki().fingerprint()
        )
    );
    let ran = desk.run(&["login", &address, "--code", &code]);
    assert_eq!(ran.err, "curvyoctl: code not valid\n");
    let whoami = desk.ok(&["whoami"]);
    assert!(
        whoami.contains("Name:      curvyoctl@marc-desktop\nRole:      owner\n"),
        "{whoami}"
    );
    // Both computers are listed, each marks its own row; owner commands work from the second.
    let clients = desk.ok(&["clients"]);
    assert!(
        clients
            .lines()
            .any(|l| l.starts_with("curvyoctl@marc-laptop ") && l.contains(" owner ")),
        "{clients}"
    );
    assert!(
        clients
            .lines()
            .any(|l| l.starts_with("curvyoctl@marc-desktop") && l.ends_with("(this computer)"))
    );
    let code = code_of(&desk.ok(&["pair", "--name", "Phone"]));
    let mut phone = app_login(&c, &app_pairs(&c, &code));
    let job = phone
        .send_job("sign.gcode", JOB, Instant::now() + WAIT)
        .unwrap();
    phone.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_state(&mut phone, |s| s == State::AwaitingLocalStart);
    assert_eq!(
        desk.ok(&["confirm-start"]),
        "Started sign.gcode, requested by Phone\n"
    );
    assert_eq!(
        desk.ok(&["logout"]),
        "Removed workshop-laser from this computer\n"
    );
    assert!(marc.ok(&["clients"]).contains("curvyoctl@marc-desktop"));
}

#[test]
fn a37_a38_login_refusals_maintenance_and_a_changed_key() {
    let fresh = connector();
    let mut desk = computer("marc-desktop");
    let ran = desk.run(&["login", &fresh.address(), "--code", "K7QM-4XTR"]);
    assert_eq!(ran.err, "curvyoctl: workshop-pi is in maintenance mode\n");
    // A stored key that differs is refused before the code is sent.
    let other = connector();
    let mut marc = computer("marc-laptop");
    marc.setup(&other);
    let code = code_of(&marc.ok(&["pair", "--name", "Second", "--owner"]));
    let toml = format!(
        "format = 1\n\n[[connectors]]\nname = \"workshop-laser\"\naddress = \"{}\"\nspki_sha256 = \"{}\"\n",
        other.address(),
        "ab".repeat(32)
    );
    std::fs::create_dir_all(desk.dir()).unwrap();
    std::fs::write(desk.dir().join(curvyo_connector_ctl::KNOWN_FILE), toml).unwrap();
    let ran = desk.run(&["login", &other.address(), "--code", &code]);
    assert_eq!(ran.code, 1);
    assert!(
        ran.err
            .starts_with("curvyoctl: the key of workshop-laser changed\nStored key:    ABAB"),
        "{ran:?}"
    );
    // The code keeps its attempts: after a logout the login pins the new key.
    assert_eq!(
        desk.ok(&["logout"]),
        "Removed workshop-laser from this computer\n"
    );
    let out = desk.ok(&["login", &other.address(), "--code", &code]);
    assert!(out.starts_with("Logged in to workshop-laser."), "{out}");
}

#[test]
fn without_a_credential_store_apply_config_and_login_fail_before_anything_is_sent() {
    let c = connector();
    // The store is checked before ApplyConfig: the connector stays in maintenance mode.
    let mut bare = computer_without_store("bare-laptop");
    let config = bare.config.display().to_string();
    let fingerprint = c.fingerprint();
    let ran = bare.run(&[
        "apply-config",
        "--insecure",
        &c.address(),
        "--fingerprint",
        &fingerprint,
        "-f",
        &config,
    ]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: no credential store on this computer\n")
    );
    let mut marc = computer("marc-laptop");
    marc.setup(&c);
    // `login` checks it before the code is sent, so the code is not spent.
    let code = code_of(&marc.ok(&["pair", "--name", "Second", "--owner"]));
    let ran = bare.run(&["login", &c.address(), "--code", &code]);
    assert_eq!(
        (ran.code, ran.err.as_str()),
        (1, "curvyoctl: no credential store on this computer\n")
    );
    let mut desk = computer("bare-laptop");
    desk.ok(&["login", &c.address(), "--code", &code]);
}
