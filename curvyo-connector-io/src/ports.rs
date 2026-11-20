//! The serial device paths of the connector's host for `ListPorts` (A2, A29): names only, read
//! from `/dev/serial/by-id/` and `/dev`; nothing is opened.

use std::path::Path;

/// The device name prefixes in `/dev` that `ListPorts` names.
const PREFIXES: [&str; 2] = ["ttyUSB", "ttyACM"];

/// The serial ports under `dev` (normally `/dev`): `serial/by-id/*` names first, then `ttyUSB*`
/// and `ttyACM*`, each group sorted; an empty list when there are none or `dev` is missing.
#[must_use]
pub fn list_ports(dev: &Path) -> Vec<String> {
    let names = |dir: &Path, keep: &dyn Fn(&str) -> bool| -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| keep(n))
            .map(|n| dir.join(n).to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let mut paths = names(&dev.join("serial/by-id"), &|_| true);
    paths.extend(names(dev, &|n| PREFIXES.iter().any(|p| n.starts_with(p))));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a2_by_id_names_first_then_usb_and_acm_devices() {
        let dev = tempfile::tempdir().unwrap();
        assert_eq!(list_ports(dev.path()), Vec::<String>::new());
        let by_id = dev.path().join("serial/by-id");
        std::fs::create_dir_all(&by_id).unwrap();
        std::fs::write(by_id.join("usb-1a86_USB2.0-Serial-if00-port0"), "").unwrap();
        for name in [
            "ttyUSB1", "ttyUSB0", "ttyACM0", "ttyS0", "ttyAMA0", "sda", "null",
        ] {
            std::fs::write(dev.path().join(name), "").unwrap();
        }
        let d = dev.path().display();
        assert_eq!(
            list_ports(dev.path()),
            vec![
                format!("{d}/serial/by-id/usb-1a86_USB2.0-Serial-if00-port0"),
                format!("{d}/ttyACM0"),
                format!("{d}/ttyUSB0"),
                format!("{d}/ttyUSB1"),
            ]
        );
        assert_eq!(
            list_ports(&dev.path().join("missing")),
            Vec::<String>::new()
        );
    }
}
