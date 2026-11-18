//! The connector's terminal: lines from stdin are commands, notices from the server are printed.

use std::io::BufRead as _;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use curvyo_connector_io::server::RunningServer;

/// Prints every notice on its own line until the notices end.
pub fn print_notices(notices: Receiver<String>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in notices {
            println!("{line}");
        }
    })
}

/// Reads stdin: an empty line is the start press, `pair` prints a new code. Ends with stdin.
pub fn read_commands(server: Arc<RunningServer>) {
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            match line.trim() {
                "" => {
                    let _ = server.local_start();
                }
                "pair" => match server.new_code() {
                    Some(code) => println!("Pairing code: {code}, valid for 10 minutes"),
                    None => println!("No random bytes are available."),
                },
                other => println!(
                    "Unknown line {other:?}; an empty line starts a waiting job, `pair` issues a code."
                ),
            }
        }
    });
}
