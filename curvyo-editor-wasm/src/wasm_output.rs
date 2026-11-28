//! Thin host bridge of the output group, the Toolpath view and Export job file
//! (`specs/0029-laser-job-preview-and-output`): views as JSON, the commands, the play bar and the
//! job file. Ids are the JSON texts of node ids, as the other bridges take them.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// A job file for the save dialog (criterion 27).
#[wasm_bindgen]
pub struct WasmJobFile {
    /// The file's bytes.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
    /// The default name ("Untitled - Laser 1.gcode").
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    /// The dialog's file type ("GRBL G-code").
    #[wasm_bindgen(getter_with_clone)]
    pub file_type: String,
    /// The extension without its dot ("gcode").
    #[wasm_bindgen(getter_with_clone)]
    pub extension: String,
    /// The token [`WasmSession::export_job_done`] takes after a successful write.
    pub token: u32,
}

/// A job for the connector (`specs/0125-send-to-machine` S6).
#[wasm_bindgen]
pub struct WasmSentJob {
    /// The job text's bytes: the file Export would write.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
    /// The page's name.
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    /// The machine profile whose pairing is used.
    #[wasm_bindgen(getter_with_clone)]
    pub profile_id: String,
    /// The connector's address.
    #[wasm_bindgen(getter_with_clone)]
    pub address: String,
    /// The token `send_job_done` and `send_job_failed` take.
    pub token: u32,
}

#[wasm_bindgen]
impl WasmSession {
    /// The output group of the active laser page's Operations tab as JSON, `null` on another
    /// page.
    #[must_use]
    pub fn output_group_json(&self) -> String {
        self.session.output_group_json()
    }

    /// The play bar as JSON (`{"fraction", "playing", "readout", "enabled"}`), `null` while the
    /// Toolpath view is not pressed.
    #[must_use]
    pub fn play_view_json(&self) -> String {
        serde_json::to_string(&self.session.play_view()).unwrap_or_else(|_| "null".to_owned())
    }

    /// The rings' labels in canvas CSS px (`[{"digit", "x", "y"}]`, `digit` `null` for the
    /// fixture glyph).
    #[must_use]
    pub fn toolpath_ring_labels_json(&self) -> String {
        self.session.toolpath_ring_labels_json()
    }

    /// The Toolpath view toggle in the label of laser page `page` (criterion 8). Whether it is a
    /// laser page. Session state, no step.
    pub fn toggle_toolpath_view(&mut self, page: &str) -> bool {
        self.session.toggle_toolpath_view(page)
    }

    /// Selects the items of a check line (JSON array of id texts). Whether it selected.
    pub fn select_output_items(&mut self, ids_json: &str) -> bool {
        self.session
            .select_output_items(&crate::wasm_operations::nodes(ids_json))
    }

    /// Unfolds operation row `id` for a check's button. Whether it did.
    pub fn reveal_operation_row(&mut self, id: &str) -> bool {
        crate::wasm_operations::node(id).is_some_and(|id| self.session.reveal_operation_row(id))
    }

    /// Picks the fixture jobs' material `id` (empty removes the pick). Whether it changed.
    pub fn set_fixture_material(&mut self, id: &str) -> bool {
        self.session.set_fixture_material(id)
    }

    /// Start: `start` is `machine_origin`, `head_position`, `home_then_origin` or, with a stored
    /// point, `user_origin`. Whether it changed.
    pub fn set_start(&mut self, start: &str) -> bool {
        self.session.set_start(start)
    }

    /// Set origin here and the typed X, Y pair (`specs/0118` criterion 29): stores the point in
    /// bed mm as the page's user origin with Start User origin, one step. Whether it changed.
    pub fn set_user_origin(&mut self, x_mm: f64, y_mm: f64) -> bool {
        self.session.set_user_origin(x_mm, y_mm)
    }

    /// The typed X, Y pair of the User origin row: as `set_user_origin`, its own step
    /// "User origin" (`specs/0118` W43).
    pub fn type_user_origin(&mut self, x_mm: f64, y_mm: f64) -> bool {
        self.session.type_user_origin(x_mm, y_mm)
    }

    /// The user origin's cross and dashed burn box in canvas CSS px (`{"cross", "box"}`).
    #[must_use]
    pub fn user_origin_marks_json(&self) -> String {
        self.session.user_origin_marks_json()
    }

    /// The go to point target of a canvas press at document `(x_mm, y_mm)` (`specs/0118` 34,
    /// adrs P1): `{"x", "y"}` in job file coordinates or `{"cause"}`; `null` off a laser page.
    #[must_use]
    pub fn focus_target_json(&self, x_mm: f64, y_mm: f64) -> String {
        serde_json::to_string(&self.session.focus_target(x_mm, y_mm)).unwrap_or_default()
    }

    /// The pulse a press asks for, `{"power_permille", "length_ms"}`, or `null` while the copy's
    /// Focus pulse or Laser mode is off (adrs P7).
    #[must_use]
    pub fn focus_pulse_json(&self) -> String {
        serde_json::to_string(&self.session.focus_pulse()).unwrap_or_default()
    }

    /// The target ring at the connector's clamped target (job file mm, W59); whether placed.
    pub fn set_focus_ring(&mut self, x_mm: f64, y_mm: f64) -> bool {
        self.session.set_focus_ring(x_mm, y_mm)
    }

    /// The target ring leaves. Whether one was shown.
    pub fn clear_focus_ring(&mut self) -> bool {
        self.session.clear_focus_ring()
    }

    /// Moves the play slider to `fraction` (0 to 1) of the job's time.
    pub fn set_play_position(&mut self, fraction: f64) {
        self.session.set_play_position(fraction);
    }

    /// Steps the play slider by `percent` of the run (the arrow and page keys).
    pub fn step_play_position(&mut self, percent: f64) {
        self.session.step_play_position(percent);
    }

    /// The embroidery play bar's Speed segment: 1, 4 or 16.
    pub fn set_play_speed(&mut self, speed: u32) {
        self.session.set_play_speed(speed);
    }

    /// Play or Pause at the clock `now_ms`. Whether it plays now.
    pub fn toggle_play(&mut self, now_ms: f64) -> bool {
        self.session.toggle_play(now_ms)
    }

    /// One animation frame of Play at the clock `now_ms`. Whether it still plays.
    pub fn play_tick(&mut self, now_ms: f64) -> bool {
        self.session.play_tick(now_ms)
    }

    /// The job file of the active laser page (criterion 27); `undefined` while Export is
    /// disabled. Nothing is written to the project.
    pub fn export_job(&mut self, project_file: &str) -> Option<WasmJobFile> {
        let job = self.session.export_job(project_file)?;
        Some(WasmJobFile {
            bytes: job.bytes,
            name: job.name,
            file_type: job.file_type.to_owned(),
            extension: job.extension.to_owned(),
            token: job.token,
        })
    }

    /// The job of `token` was written as `name` (criterion 28): the step "Export job file" and
    /// Lasered at `now_ms`. The notice as `"<ms>:<text>"` (its second line after a `\n`), empty
    /// for an unknown token. Cancel or a failed write calls nothing.
    pub fn export_job_done(
        &mut self,
        token: u32,
        name: &str,
        downloaded: bool,
        now_ms: f64,
    ) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: a clock in ms since 1970, positive and far below 2^53
        let now = now_ms.max(0.0) as u64;
        self.session
            .export_job_done(token, name, downloaded, now)
            .map(|(text, ms)| format!("{ms}:{text}"))
            .unwrap_or_default()
    }

    /// How the app stands to the page's connector: `browser`, `not_paired`, `no_longer_paired`,
    /// `key_changed` or `paired`. Session state, never saved.
    pub fn set_send_link(&mut self, link: &str) {
        self.session.set_send_link(link);
    }

    /// How the page's connector stands for a start (`0134` SS9): the last status's state
    /// spelling (`""` before any status) and the controller's own word (`""` for none). Session
    /// state, never saved.
    pub fn set_send_machine(&mut self, state: &str, controller_word: &str) {
        self.session.set_send_machine(state, controller_word);
    }

    /// Whether a press of Send to machine now also starts the job (`0134` UX P4), read at the
    /// arming and at the confirming press.
    #[must_use]
    pub fn send_will_start(&self) -> bool {
        self.session.send_will_start()
    }

    /// The job of the active laser page for the connector; `undefined` unless Send is enabled
    /// (the checks are read again at this moment). Nothing is written to the project.
    pub fn send_job(&mut self) -> Option<WasmSentJob> {
        let job = self.session.send_job()?;
        Some(WasmSentJob {
            bytes: job.bytes,
            name: job.name,
            profile_id: job.profile_id,
            address: job.address,
            token: job.token,
        })
    }

    /// The frame text of the active laser page for the connector (`specs/0118` 2b): `shape` is
    /// `rectangle` or `rubber_band`; `undefined` while the page cannot be framed ("Nothing to
    /// frame", "Outside the work area"). Nothing is written to the project, no history step.
    #[must_use]
    pub fn frame_bytes(&self, shape: &str) -> Option<Vec<u8>> {
        self.session
            .frame_bytes(crate::session::output_frame_shape(shape))
    }

    /// The head ring of a running frame at the controller's position in millimetres; `relative`
    /// for Head position, where it is the offset from where the frame began. Whether the page
    /// has a job to show it on.
    pub fn set_frame_head(&mut self, x_mm: f64, y_mm: f64, relative: bool) -> bool {
        self.session.set_frame_head(x_mm, y_mm, relative)
    }

    /// The frame ended: the ring leaves. Whether one was shown.
    pub fn clear_frame_head(&mut self) -> bool {
        self.session.clear_frame_head()
    }

    /// The connector accepted the job of `token` (`0125` S10): the step "Send job to machine"
    /// and Lasered at `now_ms`. The notice as `"<ms>:<text>"`, empty when there is none and for
    /// an unknown token.
    pub fn send_job_done(&mut self, token: u32, now_ms: f64) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: a clock in ms since 1970, positive and far below 2^53
        let now = now_ms.max(0.0) as u64;
        self.session
            .send_job_done(token, now)
            .map(|(text, ms)| format!("{ms}:{text}"))
            .unwrap_or_default()
    }

    /// The send of `token` got no answer in time: Send is enabled again, a late acceptance of
    /// `token` still commits once.
    pub fn send_job_timed_out(&mut self, token: u32) {
        self.session.send_job_timed_out(token);
    }

    /// The send of `token` ended without an accepted job: nothing is committed.
    pub fn send_job_failed(&mut self, token: u32) {
        self.session.send_job_failed(token);
    }
}
