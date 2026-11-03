//! Reads the arguments of a command the page called with file bytes as a raw binary body
//! (ADR 0001 §5: bytes cross the IPC as bytes, never as a JSON number array).
//!
//! The page sends the bytes as the body and the other arguments as ASCII-only JSON in the
//! header [`ARGS_HEADER`] (`frontend/src/platform/host.ts` `invokeWithBytes`); a call without
//! bytes keeps the plain JSON body.

use serde::de::DeserializeOwned;
use tauri::ipc::{InvokeBody, Request};

/// The header that carries a binary call's other arguments as JSON.
pub(crate) const ARGS_HEADER: &str = "curvyo-args";

/// The arguments and the bytes of `request`: from the header and the raw body, or from a JSON
/// body with no bytes.
pub(crate) fn call_args<T: DeserializeOwned>(
    request: &Request<'_>,
) -> Result<(T, Vec<u8>), String> {
    match request.body() {
        InvokeBody::Raw(bytes) => {
            let header = request
                .headers()
                .get(ARGS_HEADER)
                .map(tauri::http::HeaderValue::as_bytes);
            Ok((header_args(header)?, bytes.clone()))
        }
        InvokeBody::Json(value) => serde_json::from_value(value.clone())
            .map(|args| (args, Vec::new()))
            .map_err(|_| "bad-arguments".to_owned()),
    }
}

/// The arguments in the header's JSON; `bad-arguments` when absent or unreadable.
pub(crate) fn header_args<T: DeserializeOwned>(header: Option<&[u8]>) -> Result<T, String> {
    let header = header.ok_or_else(|| "bad-arguments".to_owned())?;
    serde_json::from_slice(header).map_err(|_| "bad-arguments".to_owned())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Args {
        path: Vec<String>,
    }

    /// A name outside ASCII travels as JSON escapes in the header and reads back unchanged.
    #[test]
    fn header_arguments_carry_escaped_names() {
        let header = "{\"path\":[\"Deckel\",\"Gr\\u00fc\\u00dfe.curvyo\"]}".as_bytes();
        let args: Args = header_args(Some(header)).unwrap();
        assert_eq!(
            args.path,
            vec!["Deckel".to_owned(), "Grüße.curvyo".to_owned()]
        );
    }

    #[test]
    fn missing_or_damaged_header_is_refused() {
        assert_eq!(
            header_args::<Args>(None).unwrap_err(),
            "bad-arguments".to_owned()
        );
        assert!(header_args::<Args>(Some(b"{")).is_err());
    }
}
