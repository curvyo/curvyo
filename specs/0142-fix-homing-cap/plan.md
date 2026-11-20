# Plan for the homing cap

## Affected crates/modules

- `curvyo-connector-core`: `machine` gains the event `ControllerLostWith { detail }` (a loss
  whose sentence replaces "Lost the controller at line <n>", as `StopUnconfirmed` already does);
  internal API only, no wire change (HC8).
- `curvyo-connector-io`: `grbl::Timing` gains `homing` (120 s by default); `grbl::controller`
  starts the cap when `$H` is written (job or Home command) and gives the link up at expiry;
  `grbl::texts` holds the sentence; `hub_log::log_text` words it for `jobs.log`.
- `curvyo-connector-server`: `--homing-timeout <seconds>` in `args`, passed into `Timing`.
- Tests: `curvyo-connector-io/tests/grbl_homing_cap.rs` (stepped time, the simulator with silent
  homing), `grbl_pty_homing_cap.rs` (a real pty, waiting by status),
  `curvyo-connector-server/tests/serial_and_exit.rs` (exit code 2), `args` unit tests.

## Tasks

- [x] 1. Failing tests first: cap expiry for a job's `$H` and the Home command's, bytes
  `0x18` < `M5` < `M9`, texts, `jobs.log` words (HC1, HC3, HC4, HC5, HC6, HC7).
- [x] 2. Core: `ControllerLostWith` drops the job and shows its sentence, sticky through the
  reconnect (HC3 c to e, HC4).
- [x] 3. `-io`: `Timing::homing`, one clock started at the `$H` write in `command` and `advance`,
  judged after the step's input is read, cleared by an answer, an alarm or a Stop (HC1, HC3, HC5,
  HC6); `log_text` for "Lost the controller: ..." (HC3 f).
- [x] 4. `-server`: `--homing-timeout`, 10 to 3600, refused with exit code 2 before the port
  opens, `--help` (HC2, HC9).
- [x] 5. Docs: Linux guide "Homing" paragraph (HC9); amendments to `0127` G9, `0129`, `0118`,
  `0133` criterion 6.
- [x] 6. Identify after a loss (HC7, tester finding): `grbl::identify` skips an `ok` in the
  settings step before any `$<n>=` line (it answers the `$I` of a stale banner), so `$$` is read
  again after the cap's and after a G15 loss; unit tests for the stale banner, a banner while `$$`
  is read and a bare `ok`; the tester's pty tests for both losses, 20 times under CPU load.
- [x] 7. Gate (HC11).

## Validation

- Stepped-time bench (`tests/common/grbl_bench.rs`) with a 5 s test cap; every byte the
  connector wrote at the loss is asserted.
- One pty run through `RunningServer` with a 1 s cap, waiting by status, never by sleep.
- The new tests 20 times under CPU load.
- Real laser (customer): pull the USB cable while `$H` runs; the footer reads "Lost the
  controller: homing took longer than 120 s" after at most 120 s (HC11 keeps 🟡 until then).

## Decisions (Proposal)

- The loss ends in `NoController` with the cap's sentence and no `Done` frame (HC3 c, e as
  amended) rather than a separate `Done { Failed }` state: one state per status, as every other
  loss of `0127` G15; `jobs.log` records the job as `Failed` (HC3 f).
- `--homing-timeout` needs `--serial`, as `--baud` does.
- An `ALARM:<n>` during the Home command's `$H` counts as its answer (HC1) and clears the cap.
