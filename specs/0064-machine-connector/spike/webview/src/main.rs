//! Spike 0064: a Tauri window whose page opens `ws://<addr>` with the connector subprotocol.
//! The echo server's log shows whether the upgrade and the round trip happened. Throwaway code.
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("spike: tauri runs");
}
