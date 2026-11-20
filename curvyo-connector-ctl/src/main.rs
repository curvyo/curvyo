//! `curvyoctl`: the command line around the library's `run`.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match curvyo_connector_ctl::Context::system().map_err(anyhow::Error::from) {
        Ok(mut ctx) => {
            let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
            curvyo_connector_ctl::run(&args, &mut ctx, &mut out, &mut err)
        }
        Err(e) => {
            eprintln!("curvyoctl: {e:#}");
            ExitCode::from(1)
        }
    }
}
