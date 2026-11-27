//! The job's lines as GRBL receives them, checked before the first byte is written (`specs/0127`
//! G5): the pure check lives in `curvyo-connector-core` so the CNC goldens and the virtual GRBL
//! use the same rule (`specs/0161-cnc-output` criterion 13).

pub use curvyo_connector_core::gcode_line::{Cause, MAX_LINE, Refused, is_system, prepare, strip};
