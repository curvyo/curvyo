//! The one entry point to every output format (ADR 0064 §12): `write(format, &job)` and the
//! format facts the UI needs, one `match` each over [`OutputFormat`].

use crate::job_moves::Job;
use crate::job_write_grbl::write_grbl;
use crate::machine_profile::OutputFormat;

/// The bytes of `job` in `format`.
#[must_use]
pub fn write(format: OutputFormat, job: &Job) -> Vec<u8> {
    match format {
        OutputFormat::Grbl => write_grbl(job),
    }
}

/// The format's name in the output group's machine line ("GRBL").
#[must_use]
pub const fn name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Grbl => "GRBL",
    }
}

/// The job file's extension without the dot ("gcode").
#[must_use]
pub const fn extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Grbl => "gcode",
    }
}

/// The save dialog's file type ("GRBL G-code").
#[must_use]
pub const fn file_type(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Grbl => "GRBL G-code",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grbl_names_its_file() {
        assert_eq!(name(OutputFormat::Grbl), "GRBL");
        assert_eq!(extension(OutputFormat::Grbl), "gcode");
        assert_eq!(file_type(OutputFormat::Grbl), "GRBL G-code");
    }
}
