//! The terminal's note about `ModemManager` on Linux (`specs/0127` G27; addendum C10): it may
//! probe serial devices and disturb the controller, and the connector cannot stop it.

use std::path::Path;

/// Whether a process called `ModemManager` runs, read once from `/proc/*/comm`.
#[must_use]
pub fn modem_manager_runs(proc_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(proc_dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|e| {
        std::fs::read_to_string(e.path().join("comm")).is_ok_and(|c| c.trim() == "ModemManager")
    })
}

/// The terminal line for `port` when `ModemManager` runs.
#[must_use]
pub fn modem_manager_line(port: &str, runs: bool) -> Option<String> {
    runs.then(|| format!("ModemManager is running and may probe {port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modem_manager_is_found_by_its_process_name() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!modem_manager_runs(dir.path()));
        for (pid, name) in [("1", "systemd\n"), ("812", "ModemManager\n")] {
            std::fs::create_dir(dir.path().join(pid)).unwrap();
            std::fs::write(dir.path().join(pid).join("comm"), name).unwrap();
        }
        assert!(modem_manager_runs(dir.path()));
        assert_eq!(
            modem_manager_line("/dev/ttyUSB0", true).as_deref(),
            Some("ModemManager is running and may probe /dev/ttyUSB0")
        );
        assert_eq!(modem_manager_line("/dev/ttyUSB0", false), None);
    }
}
