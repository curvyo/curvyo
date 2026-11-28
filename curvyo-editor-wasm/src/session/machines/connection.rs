//! The laser rows' controller links in the Machines tab (UX unify 4.2, 4.3): the Connection group
//! as a plain library write, and the machine-size offer of `specs/0131` keyed by record id.

use curvyo_document_core::MachineStep;
use curvyo_laser_core::{
    Connection, clamp_address, clamp_baud, clamp_port, is_valid_work_side, machine_size_offer,
};

use super::super::Session;

impl Session {
    /// The connection of the maker's laser `id` (`0026` criterion 9 in the Machines tab): a
    /// library write, no history step and no page follows it. The first notice.
    pub fn set_machine_connection_of(
        &mut self,
        id: &str,
        address: &str,
        port: &str,
        baud: f64,
        now_ms: i64,
    ) -> Option<String> {
        let address = clamp_address(address);
        let port = clamp_port(port);
        let baud = clamp_baud(baud)?;
        let connection = Connection {
            connector_address: address.value,
            controller_port: port.value,
            baud_rate: baud.value,
        };
        let machine = &mut self.manufacturing.machine;
        if let Some(write) = machine.profiles.set_connection(id, connection, now_ms) {
            machine.push_library_write(write);
        }
        address.notice.or(port.notice).or(baud.notice)
    }

    /// The size the controller of the maker's laser `id` reports, offered while it is usable and
    /// differs from the record's work area (`specs/0131` criteria 1, 3, 4).
    #[must_use]
    pub fn machine_size_offer_of(&self, id: &str, x_mm: f64, y_mm: f64) -> Option<[f64; 2]> {
        let profiles = &self.manufacturing.machine.profiles;
        if !profiles.is_user(id) {
            return None;
        }
        machine_size_offer(profiles.find(id)?.work_area, [x_mm, y_mm])
            .map(|size| [size.width.as_mm(), size.height.as_mm()])
    }

    /// Whether the report is a usable size for the offer row's place.
    #[must_use]
    pub fn machine_size_reported_of(&self, id: &str, x_mm: f64, y_mm: f64) -> bool {
        is_valid_work_side(x_mm)
            && is_valid_work_side(y_mm)
            && self.manufacturing.machine.profiles.is_user(id)
    }

    /// Use machine size in the Machines tab: the record takes the reported size in one step "Use
    /// machine size", then the offer to update the pages (Frage 434 C). Whether it changed.
    pub fn use_machine_size_of(&mut self, id: &str, x_mm: f64, y_mm: f64, now_ms: i64) -> bool {
        let Some([w, h]) = self.machine_size_offer_of(id, x_mm, y_mm) else {
            return false;
        };
        let name = Some("Use machine size".to_owned());
        self.machine_step(MachineStep::Edit, name, |this| {
            this.edit_machine_record(id, "work_area", &format!("{w}x{h}"), now_ms)
        })
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::Session;

    #[test]
    fn use_machine_size_is_one_step_on_the_record_and_offers_the_pages() {
        let mut s = Session::new(98_301);
        s.add_page_of_kind("curvyo.laser", None).unwrap();
        let id = s.duplicate_machine("generic-laser", [2; 16], 1).unwrap();
        assert!(s.pick_output_profile(&id));
        assert!(s.machine_size_reported_of(&id, 300.0, 200.0));
        assert_eq!(
            s.machine_size_offer_of(&id, 300.0, 200.0),
            Some([300.0, 200.0])
        );
        assert_eq!(s.machine_size_offer_of("generic-laser", 300.0, 200.0), None);
        let steps = s.undo_len();
        assert!(s.use_machine_size_of(&id, 300.0, 200.0, 2));
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(
            s.machine_size_offer_of(&id, 300.0, 200.0),
            None,
            "no offer once equal"
        );
        assert!(s.machines_view().unwrap().follow.is_some());
        s.undo();
        assert_eq!(
            s.machine_size_offer_of(&id, 300.0, 200.0),
            Some([300.0, 200.0])
        );
    }
}
