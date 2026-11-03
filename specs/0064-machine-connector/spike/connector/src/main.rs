//! Spike 0064: a static binary with `serialport` (no libudev) and sync `tungstenite`.
//! `spike-0064-connector ports` lists serial ports; `spike-0064-connector open <dev>` opens one at
//! 115200 baud and sends `?`; `spike-0064-connector echo <addr>` runs a WebSocket echo server that
//! requires the subprotocol `curvyo-connector.1` (one thread per client). Throwaway code.

use std::io::Write as _;
use std::net::TcpListener;
use std::time::Duration;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::http::HeaderValue;

const SUBPROTOCOL: &str = "curvyo-connector.1";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("ports") => match serialport::available_ports() {
            Ok(ports) => {
                println!("{} port(s)", ports.len());
                for p in ports {
                    println!("{} {:?}", p.port_name, p.port_type);
                }
            }
            Err(e) => println!("available_ports error: {e}"),
        },
        Some("open") => {
            let dev = args.get(2).expect("spike: device path");
            match serialport::new(dev, 115_200).timeout(Duration::from_millis(500)).open() {
                Ok(mut port) => {
                    let _ = port.write_all(b"?");
                    println!("opened {dev}");
                }
                Err(e) => println!("open {dev}: {e}"),
            }
        }
        Some("echo") => echo(args.get(2).map_or("0.0.0.0:7979", String::as_str)),
        _ => eprintln!("usage: ports | open <dev> | echo <addr>"),
    }
}

fn echo(addr: &str) {
    let listener = TcpListener::bind(addr).expect("spike: bind");
    println!("listening on {addr}");
    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || {
            let peer = stream.peer_addr().ok();
            let callback = |req: &Request, mut resp: Response| {
                let origin = req.headers().get("origin").cloned();
                let offered = req
                    .headers()
                    .get("sec-websocket-protocol")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_owned();
                println!("upgrade from {peer:?} origin={origin:?} protocols={offered:?}");
                if offered.split(',').any(|p| p.trim() == SUBPROTOCOL) {
                    resp.headers_mut()
                        .insert("sec-websocket-protocol", HeaderValue::from_static(SUBPROTOCOL));
                    Ok(resp)
                } else {
                    let mut r = tungstenite::handshake::server::ErrorResponse::new(Some(
                        "subprotocol required".into(),
                    ));
                    *r.status_mut() = tungstenite::http::StatusCode::BAD_REQUEST;
                    Err(r)
                }
            };
            let Ok(mut ws) = tungstenite::accept_hdr(stream, callback) else {
                println!("upgrade refused");
                return;
            };
            while let Ok(msg) = ws.read() {
                if msg.is_binary() || msg.is_text() {
                    if let tungstenite::Message::Text(t) = &msg {
                        println!("text: {t}");
                    } else {
                        println!("echo {} bytes", msg.len());
                    }
                    if ws.send(msg).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
