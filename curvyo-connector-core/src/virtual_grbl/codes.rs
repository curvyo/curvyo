//! The GRBL 1.1 error and alarm numbers the virtual GRBL answers with, as the GRBL 1.1
//! documentation lists them (`doc/csv/error_codes_en_US.csv`, `alarm_codes_en_US.csv`).

/// `error:1` Expected command letter.
pub const EXPECTED_COMMAND_LETTER: u16 = 1;
/// `error:2` Bad number format.
pub const BAD_NUMBER_FORMAT: u16 = 2;
/// `error:3` Invalid statement (`$` command).
pub const INVALID_STATEMENT: u16 = 3;
/// `error:4` Value < 0.
pub const NEGATIVE_VALUE: u16 = 4;
/// `error:5` Setting disabled (`$H` with homing off).
pub const SETTING_DISABLED: u16 = 5;
/// `error:8` Not idle.
pub const NOT_IDLE: u16 = 8;
/// `error:9` G-code lock (alarm or jog state).
pub const GCODE_LOCK: u16 = 9;
/// `error:10` Homing not enabled (soft limits need homing).
pub const HOMING_NOT_ENABLED: u16 = 10;
/// `error:11` Line overflow (more than 79 characters after stripping).
pub const LINE_OVERFLOW: u16 = 11;
/// `error:15` Travel exceeded (a jog beyond the soft limits).
pub const TRAVEL_EXCEEDED: u16 = 15;
/// `error:16` Invalid jog command.
pub const INVALID_JOG: u16 = 16;
/// `error:20` Unsupported command (`M6`, `G81`, ...).
pub const UNSUPPORTED_COMMAND: u16 = 20;
/// `error:21` Modal group violation.
pub const MODAL_GROUP_VIOLATION: u16 = 21;
/// `error:22` Undefined feed rate.
pub const UNDEFINED_FEED_RATE: u16 = 22;
/// `error:25` Repeated word.
pub const REPEATED_WORD: u16 = 25;

/// `ALARM:2` Soft limit: a motion target exceeds the machine travel.
pub const ALARM_SOFT_LIMIT: u16 = 2;
/// `ALARM:3` Reset while in motion.
pub const ALARM_RESET_IN_MOTION: u16 = 3;
