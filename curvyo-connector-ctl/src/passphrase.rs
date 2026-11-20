//! The passphrase of the key file, read from the terminal with echo off (addendum D12).

/// The passphrase typed on the terminal; `None` when stdin is not a terminal.
#[cfg(unix)]
#[must_use]
pub fn read() -> Option<zeroize::Zeroizing<String>> {
    use rustix::termios::{LocalModes, OptionalActions, isatty, tcgetattr, tcsetattr};
    let stdin = std::io::stdin();
    if !isatty(&stdin) {
        return None;
    }
    let before = tcgetattr(&stdin).ok()?;
    let mut quiet = before.clone();
    quiet.local_modes -= LocalModes::ECHO;
    tcsetattr(&stdin, OptionalActions::Now, &quiet).ok()?;
    eprint!("Passphrase: ");
    let mut line = zeroize::Zeroizing::new(String::new());
    let read = stdin.read_line(&mut line);
    let _ = tcsetattr(&stdin, OptionalActions::Now, &before);
    eprintln!();
    read.ok()?;
    let text = zeroize::Zeroizing::new(line.trim_end_matches(['\n', '\r']).to_owned());
    (!text.is_empty()).then_some(text)
}

/// No terminal passphrase off unix.
#[cfg(not(unix))]
#[must_use]
pub fn read() -> Option<zeroize::Zeroizing<String>> {
    None
}
