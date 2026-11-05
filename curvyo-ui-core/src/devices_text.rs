//! The words of My devices in the Share tab (`specs/0096-own-devices-sync` criteria 5, 8, 9,
//! 13, 22 to 28, `ux-notes.md` sections 2 and 5 to 9): rows, confirms, sync states and notices,
//! formatted with their names and numbers. The pairing steps are `pairing_text`.

use crate::share_text::{cut_label, server_shown};

/// The privacy sentence of choosing workspaces and of the first switch-on (criteria 9 and 30).
pub const PRIVACY_SENTENCE: &str = "Names, folders and files leave this device encrypted, to your \
                                    own devices only. The server can tell which devices belong \
                                    together, not their names.";

/// The two sentences of criterion 28 while only this device is paired.
pub const LOSS_SENTENCES: &str = "If you lose this device, the copy on the server cannot be \
                                  opened by anyone. Pair a second device to have a backup.";

/// The persistent warning while a workspace is synced and this is the only device (2.2).
pub const ONE_DEVICE_WARNING: &str = "Only this device is paired. If you lose it, the copy on the \
                                      server cannot be opened by anyone. Pair a second device as \
                                      a backup.";

/// The removed device's alert (criterion 24).
pub const REMOVED_ALERT: &str =
    "This device was removed from your devices. Your copies stay here as local files.";

/// The confirm of Leave my devices (7.1).
pub const LEAVE_CONFIRM: &str = "Leave your devices? This device stops syncing and keeps its \
                                 copies as local files. Your other devices renew the keys the \
                                 next time they connect.";

/// The name of a device the group's keyring lists but the devices list names not (yet).
pub const UNNAMED_DEVICE: &str = "A device";

/// An action that needs the connection was refused before anything was written.
pub const NOT_CONNECTED: &str = "Not connected. Try again when connected.";

/// The right of the block heading: "Only this device", "2 devices".
#[must_use]
pub fn devices_count_text(devices: usize) -> String {
    if devices <= 1 {
        "Only this device".to_owned()
    } else {
        format!("{devices} devices")
    }
}

/// Line 3 of another device's row: "Online", "Last seen 12:03" today, "Last seen 2 Nov 12:03"
/// before; a browser adds "Browser, " in front. Times are seconds since the Unix epoch, shown
/// with the local `offset_minutes`.
#[must_use]
pub fn seen_text(
    online: bool,
    last_seen: Option<u64>,
    now: u64,
    offset_minutes: i32,
    browser: bool,
) -> String {
    let state = match (online, last_seen) {
        (true, _) => "Online".to_owned(),
        (false, None) => "Not seen yet".to_owned(),
        (false, Some(seen)) => {
            let local = |t: u64| t.cast_signed() + i64::from(offset_minutes) * 60;
            let (seen_l, now_l) = (local(seen), local(now));
            let time = format!(
                "{:02}:{:02}",
                seen_l.rem_euclid(86_400) / 3600,
                seen_l.rem_euclid(86_400) % 3600 / 60
            );
            if seen_l.div_euclid(86_400) == now_l.div_euclid(86_400) {
                format!("Last seen {time}")
            } else {
                let (day, month) = day_month(seen_l.div_euclid(86_400));
                format!("Last seen {day} {month} {time}")
            }
        }
    };
    if browser {
        format!("Browser, {state}")
    } else {
        state
    }
}

/// The day of the month and the month's short name of `days` since 1970-01-01.
fn day_month(days: i64) -> (i64, &'static str) {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let index = usize::try_from(month - 1).unwrap_or(0);
    (day, MONTHS[index.min(11)])
}

/// The heading of choosing workspaces: "Workspaces for Laptop".
#[must_use]
pub fn workspaces_for_text(label: &str) -> String {
    format!("Workspaces for {}", cut_label(label, 24))
}

/// The line over the pick list naming workspaces that are synced already; empty when none.
#[must_use]
pub fn already_synced_text(names: &[String], label: &str) -> String {
    if names.is_empty() {
        return String::new();
    }
    format!(
        "Already synced, they arrive on {} too: {}.",
        cut_label(label, 24),
        names.join(", ")
    )
}

/// "1 document", "14 documents".
#[must_use]
pub fn documents_text(documents: usize) -> String {
    match documents {
        1 => "1 document".to_owned(),
        n => format!("{n} documents"),
    }
}

/// The confirm of switching a workspace on (6.2): the documents shared on their own it takes in
/// and those it leaves out (`taken_in`, `not_added`, as Share workspace), and the loss
/// sentences while no other device is paired.
#[must_use]
pub fn sync_on_confirm_text(
    name: &str,
    documents: usize,
    only_device: bool,
    taken_in: &[String],
    not_added: &[String],
) -> String {
    let loss = if only_device {
        format!(" {LOSS_SENTENCES}")
    } else {
        String::new()
    };
    format!(
        "Sync {} ({}) to your devices? {PRIVACY_SENTENCE}{}{loss}",
        cut_label(name, 24),
        documents_text(documents),
        crate::workspace_share_text::takeover_text(taken_in, not_added)
    )
}

/// The switch row's alert when switching on failed (6.2).
pub const SYNC_FAILED: &str = "Could not sync: cannot reach the server.";

/// The confirm of switching a workspace off (6.3), naming the other devices and who else keeps
/// sharing it.
#[must_use]
pub fn sync_off_confirm_text(name: &str, devices: &[String], others: &[String]) -> String {
    let name = cut_label(name, 24);
    let devices_part = match devices {
        [] => String::new(),
        [one] => format!(
            " {} keeps its copy as local files and gets no later changes. It is removed from \
             this workspace.",
            cut_label(one, 24)
        ),
        many => format!(
            " {} keep their copies as local files and get no later changes. They are removed \
             from this workspace.",
            join_and(many)
        ),
    };
    let shared = if others.is_empty() {
        String::new()
    } else {
        format!(" {name} stays shared with {}.", join_and(others))
    };
    format!("Stop syncing {name}?{devices_part}{shared}")
}

fn join_and(names: &[String]) -> String {
    let names: Vec<String> = names.iter().map(|n| cut_label(n, 24)).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// The confirm of Remove on a device's row (7.1).
#[must_use]
pub fn remove_device_confirm_text(label: &str, browser: bool) -> String {
    let label = cut_label(label, 24);
    let place = if browser {
        "in that browser".to_owned()
    } else {
        format!("on {label}")
    };
    format!(
        "Remove {label}? It stops syncing. Removing does not reach back: the copies already \
         {place} stay there as local files."
    )
}

/// The row's line while a removal renews keys: "Renewing keys, 1 of 2 workspaces".
#[must_use]
pub fn renewing_keys_text(done: usize, total: usize) -> String {
    format!(
        "Renewing keys, {done} of {total} {}",
        workspaces_word(total)
    )
}

/// A removal that could not be finished everywhere (criterion 26).
#[must_use]
pub fn removal_incomplete_text(done: usize, total: usize) -> String {
    format!(
        "Removal incomplete, {done} of {total} {}",
        workspaces_word(total)
    )
}

fn workspaces_word(total: usize) -> &'static str {
    if total == 1 {
        "workspace"
    } else {
        "workspaces"
    }
}

/// The alert under a row whose device stays in a workspace this device cannot remove it from.
#[must_use]
pub fn still_in_text(label: &str, workspace: &str) -> String {
    let workspace = cut_label(workspace, 24);
    format!(
        "{} is still in {workspace}. Ask an admin of {workspace} to remove it.",
        cut_label(label, 24)
    )
}

/// The switch's reason where this device is not an admin (criterion 17).
#[must_use]
pub fn not_admin_text(workspace: &str) -> String {
    format!(
        "You are not an admin of {}. Ask an admin to add this device.",
        cut_label(workspace, 24)
    )
}

/// Another device's header while keys of workspaces a device left are not renewed (criterion
/// 27).
#[must_use]
pub fn key_renewal_pending_text(workspaces: usize) -> String {
    format!(
        "Key renewal pending for {workspaces} {}",
        workspaces_word(workspaces)
    )
}

/// One workspace's sync state (5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Current.
    InSync,
    /// Catching up: done of total documents.
    Syncing(usize, usize),
    /// Not connected, with changes waiting.
    Offline(usize),
    /// The other device has not connected since the switch-on.
    WaitingForFirstCopy,
    /// The first copy arriving here.
    Receiving(usize, usize),
    /// A decision row is open.
    WaitingForAnswer,
    /// This device is not an admin there.
    NotAdmin,
}

/// The words of a sync state.
#[must_use]
pub fn sync_state_text(state: SyncState) -> String {
    match state {
        SyncState::InSync => "In sync".to_owned(),
        SyncState::Syncing(done, total) => format!("Syncing {done} of {total} documents"),
        SyncState::Offline(1) => "Offline, 1 change waiting".to_owned(),
        SyncState::Offline(n) => format!("Offline, {n} changes waiting"),
        SyncState::WaitingForFirstCopy => "Waiting for first copy".to_owned(),
        SyncState::Receiving(done, total) => format!("Receiving {done} of {total}"),
        SyncState::WaitingForAnswer => "Waiting for your answer".to_owned(),
        SyncState::NotAdmin => "Not an admin here".to_owned(),
    }
}

/// The switch's line while on (6.1): "In sync with 2 devices", or "In sync. Only this device."
#[must_use]
pub fn switch_on_line(state: SyncState, other_devices: usize) -> String {
    match (state, other_devices) {
        (SyncState::InSync, 0) => "In sync. Only this device.".to_owned(),
        (SyncState::InSync, 1) => "In sync with 1 device".to_owned(),
        (SyncState::InSync, n) => format!("In sync with {n} devices"),
        (other, _) => sync_state_text(other),
    }
}

/// The Workspace block's and File tab's header for a workspace whose members are all own
/// devices: "Synced, 2 devices".
#[must_use]
pub fn synced_header_text(devices: usize) -> String {
    match devices {
        1 => "Synced, 1 device".to_owned(),
        n => format!("Synced, {n} devices"),
    }
}

/// The notice of an arrived workspace (4.4).
#[must_use]
pub fn arrived_notice(name: &str) -> String {
    format!("Workspace {} arrived.", cut_label(name, 24))
}

/// The notice on a device whose workspace was switched off (6.4).
#[must_use]
pub fn no_longer_synced_notice(name: &str) -> String {
    format!(
        "{} is no longer synced. Your copy stays here.",
        cut_label(name, 24)
    )
}

/// The notice of a device that joined the group (7.1).
#[must_use]
pub fn device_added_notice(label: &str) -> String {
    format!("{} was added to your devices.", cut_label(label, 16))
}

/// The notice of a device that was removed (7.1).
#[must_use]
pub fn device_removed_notice(label: &str) -> String {
    format!("{} was removed.", cut_label(label, 16))
}

/// The notice of a device that left the group (7.1).
#[must_use]
pub fn device_left_notice(label: &str) -> String {
    format!("{} left your devices.", cut_label(label, 16))
}

/// The notice of a devices list change this build cannot read (ADR 0040 decision 11); the list
/// here stays as it was. (Proposal)
#[must_use]
pub fn newer_devices_list_notice() -> String {
    "Another device uses a newer Curvyo. Update this one to see changes to your devices.".to_owned()
}

/// The decision row of a copy that exists here already (2.3, criterion 14).
#[must_use]
pub fn connect_copy_text(name: &str, path: &str) -> String {
    format!(
        "{} is already in {path}. Connecting merges both copies; nothing is lost.",
        cut_label(name, 24)
    )
}

/// The notice of a document that arrived under a name this system refuses, written under its
/// safe name (criterion 15, `ux-notes.md` 4.4).
#[must_use]
pub fn renamed_file_notice(from: &str, to: &str) -> String {
    format!(
        "Renamed {} to {} on this device.",
        cut_label(from, 24),
        cut_label(to, 24)
    )
}

/// The progress line after Sync these (3.1): "Syncing 2 workspaces: uploading 3 of 14".
#[must_use]
pub fn syncing_workspaces_text(workspaces: usize, done: usize, total: usize) -> String {
    format!(
        "Syncing {workspaces} {}: uploading {done} of {total}",
        workspaces_word(workspaces)
    )
}

/// Why Sync these could not switch workspaces on (3.1, PR #281 review): named where the app can
/// tell, so a reachable server is not reported as unreachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncProblem<'a> {
    /// The server did not answer.
    Unreachable,
    /// The server did not answer, and the devices use another server than the Share tab names:
    /// `group` (the devices' since pairing), `tab` (the Share tab's).
    OtherServer {
        /// The devices' server.
        group: &'a str,
        /// The Share tab's server.
        tab: &'a str,
    },
    /// A browser on an https page cannot open a `ws://` address.
    NeedsTls,
    /// The server answered and refused the connection (not a member, an older relay).
    Refused,
    /// Sharing the workspace failed on this device before anything was sent.
    Local,
}

/// The one alert of the workspaces that Sync these could not switch on for `problem` (3.1): one
/// names its workspace, more are counted, so an outage shows one line.
#[must_use]
pub fn could_not_sync_text(names: &[String], problem: SyncProblem<'_>) -> String {
    let what = match names {
        [name] => cut_label(name, 24),
        _ => format!("{} {}", names.len(), workspaces_word(names.len())),
    };
    let why = match problem {
        SyncProblem::Unreachable => "cannot reach the server.".to_owned(),
        SyncProblem::OtherServer { group, tab } => format!(
            "cannot reach {}, the server your devices use. The Share tab names {}.",
            server_shown(group),
            server_shown(tab)
        ),
        SyncProblem::NeedsTls => {
            "a browser needs a server address that starts with wss://.".to_owned()
        }
        SyncProblem::Refused => "the server refused the connection.".to_owned(),
        SyncProblem::Local => "it could not be shared from this device.".to_owned(),
    };
    format!("Could not sync {what}: {why}")
}

/// The line under the pick list's row of a workspace in which this device is only a member.
pub const NOT_ADMIN_HERE: &str = "Not an admin here";

/// The browser's loss sentence in its join block and at pairing (criterion 34, 9).
pub const BROWSER_LOSS_SENTENCE: &str = "This browser keeps its keys in site data. If you clear \
                                         site data, this device is lost and has to be paired \
                                         again.";

/// The fourth line of the browser's own row (9).
pub const BROWSER_OWN_ROW_LINE: &str = "Clearing site data removes this device.";

/// The own row's line when the browser refused persistent storage (9, Proposal).
pub const BROWSER_NOT_PERSISTED_LINE: &str =
    "This browser may delete this device's data when space runs low.";

/// Why a browser cannot start a device group: it pairs with a desktop first (ADR 0040 note).
pub const BROWSER_CANNOT_START: &str = "Pair this browser from a desktop device first.";

/// Why a browser cannot pair with a server that has no `wss://` address (ADR 0040 decision 9).
pub const BROWSER_NEEDS_TLS: &str = "A browser needs a server address that starts with wss://.";

/// Another tab of this browser already runs this device (criterion 39).
pub const BROWSER_OTHER_TAB: &str = "This device is open in another tab of this browser.";

/// What the browser holds against what it may hold (criterion 37): "Stored 120 MB of 300 MB in
/// this browser: Box job, Poster."
#[must_use]
pub fn browser_storage_text(used: u64, quota: u64, workspaces: &[String]) -> String {
    let names = if workspaces.is_empty() {
        String::new()
    } else {
        format!(": {}", workspaces.join(", "))
    };
    format!(
        "Stored {} of {} in this browser{names}.",
        megabytes(used),
        megabytes(quota)
    )
}

/// The alert when a document could not be stored for lack of space (criterion 37).
#[must_use]
pub fn browser_storage_full_text(name: &str, used: u64, quota: u64) -> String {
    format!(
        "{} could not be stored: this browser holds {} of {}. Nothing was cut; free space or \
         stop syncing a workspace here.",
        cut_label(name, 24),
        megabytes(used),
        megabytes(quota)
    )
}

fn megabytes(bytes: u64) -> String {
    let mb = bytes.div_ceil(1_000_000);
    format!("{mb} MB")
}

/// The refusal of a Synced workspaces folder by the host's code; empty for an unknown code.
#[must_use]
pub fn synced_folder_refusal_text(code: &str) -> &'static str {
    match code {
        "inside-workspace" => "Choose a folder outside any workspace.",
        "not-writable" => "Cannot write to this folder.",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOV_4_2026_1203_UTC: u64 = 1_793_793_780;

    #[test]
    fn arrival_and_browser_lines_read_as_the_notes_say() {
        assert_eq!(
            renamed_file_notice("a:b", "a_b"),
            "Renamed a:b to a_b on this device."
        );
        assert_eq!(
            syncing_workspaces_text(2, 3, 14),
            "Syncing 2 workspaces: uploading 3 of 14"
        );
        assert_eq!(
            could_not_sync_text(&["Box job".to_owned()], SyncProblem::Unreachable),
            "Could not sync Box job: cannot reach the server."
        );
        let two = ["Box job".to_owned(), "Poster".to_owned()];
        assert_eq!(
            could_not_sync_text(&two, SyncProblem::Unreachable),
            "Could not sync 2 workspaces: cannot reach the server."
        );
        assert_eq!(
            could_not_sync_text(
                &two,
                SyncProblem::OtherServer {
                    group: "ws://localhost:8787",
                    tab: "ws://192.168.1.20:8787"
                }
            ),
            "Could not sync 2 workspaces: cannot reach localhost:8787, the server your devices \
             use. The Share tab names 192.168.1.20:8787."
        );
        assert_eq!(
            could_not_sync_text(&two[..1], SyncProblem::NeedsTls),
            "Could not sync Box job: a browser needs a server address that starts with wss://."
        );
        assert_eq!(
            could_not_sync_text(&two[..1], SyncProblem::Refused),
            "Could not sync Box job: the server refused the connection."
        );
        assert_eq!(
            could_not_sync_text(&two[..1], SyncProblem::Local),
            "Could not sync Box job: it could not be shared from this device."
        );
        assert_eq!(
            browser_storage_text(120_000_000, 300_000_001, &["Box job".to_owned()]),
            "Stored 120 MB of 301 MB in this browser: Box job."
        );
        assert_eq!(
            browser_storage_text(0, 0, &[]),
            "Stored 0 MB of 0 MB in this browser."
        );
        assert!(browser_storage_full_text("Lid", 1, 2).starts_with("Lid could not be stored"));
    }

    #[test]
    fn counts_and_rows_read_as_the_notes_say() {
        assert_eq!(devices_count_text(1), "Only this device");
        assert_eq!(devices_count_text(3), "3 devices");
        let now = NOV_4_2026_1203_UTC + 3600;
        assert_eq!(seen_text(true, None, now, 0, false), "Online");
        assert_eq!(
            seen_text(false, Some(NOV_4_2026_1203_UTC), now, 0, false),
            "Last seen 12:03"
        );
        assert_eq!(
            seen_text(false, Some(NOV_4_2026_1203_UTC - 2 * 86_400), now, 0, true),
            "Browser, Last seen 2 Nov 12:03"
        );
        assert_eq!(
            seen_text(false, Some(NOV_4_2026_1203_UTC), now, 60, false),
            "Last seen 13:03"
        );
    }

    #[test]
    fn confirms_name_workspaces_and_devices() {
        assert_eq!(
            sync_on_confirm_text("Box job", 14, true, &[], &[]),
            format!(
                "Sync Box job (14 documents) to your devices? {PRIVACY_SENTENCE} {LOSS_SENTENCES}"
            )
        );
        assert_eq!(
            sync_on_confirm_text(
                "Box job",
                3,
                false,
                &["Lid".to_owned(), "Base".to_owned()],
                &["Hinge".to_owned()]
            ),
            format!(
                "Sync Box job (3 documents) to your devices? {PRIVACY_SENTENCE} Lid and 1 other \
                 document are shared on their own. They become part of the workspace and their \
                 own links stop working. Not added: Hinge (shared with people outside this \
                 workspace)."
            )
        );
        assert_eq!(
            sync_off_confirm_text(
                "Box job",
                &["Laptop".to_owned(), "Phone".to_owned()],
                &["Ben".to_owned()]
            ),
            "Stop syncing Box job? Laptop and Phone keep their copies as local files and get no \
             later changes. They are removed from this workspace. Box job stays shared with Ben."
        );
        assert_eq!(
            remove_device_confirm_text("Laptop", false),
            "Remove Laptop? It stops syncing. Removing does not reach back: the copies already on \
             Laptop stay there as local files."
        );
        assert!(remove_device_confirm_text("Chrome", true).contains("in that browser"));
    }

    #[test]
    fn states_and_notices() {
        assert_eq!(
            sync_state_text(SyncState::Offline(2)),
            "Offline, 2 changes waiting"
        );
        assert_eq!(
            sync_state_text(SyncState::Syncing(3, 14)),
            "Syncing 3 of 14 documents"
        );
        assert_eq!(
            switch_on_line(SyncState::InSync, 0),
            "In sync. Only this device."
        );
        assert_eq!(
            switch_on_line(SyncState::InSync, 2),
            "In sync with 2 devices"
        );
        assert_eq!(synced_header_text(2), "Synced, 2 devices");
        assert_eq!(
            removal_incomplete_text(1, 2),
            "Removal incomplete, 1 of 2 workspaces"
        );
        assert_eq!(
            key_renewal_pending_text(1),
            "Key renewal pending for 1 workspace"
        );
        assert_eq!(arrived_notice("Box job"), "Workspace Box job arrived.");
        assert_eq!(
            device_added_notice("A very long device name indeed"),
            format!(
                "{} was added to your devices.",
                cut_label("A very long device name indeed", 16)
            )
        );
        assert_eq!(
            not_admin_text("Box job"),
            "You are not an admin of Box job. Ask an admin to add this device."
        );
        assert_eq!(
            synced_folder_refusal_text("inside-workspace"),
            "Choose a folder outside any workspace."
        );
    }
}
