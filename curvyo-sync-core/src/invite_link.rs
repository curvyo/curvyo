//! The invite link's text form (ADR 0031 decision 8, ADR 0037 decision 9):
//! `curvyo://join?server=<url-encoded relay URL>#<version>.<base64url(room_id ‖ invite_seed)>`.
//!
//! Only the part after `#` carries the secret, so a web server that ever serves the link never
//! receives it (criterion 2). Version `1.` means one document whose room id is its document id,
//! version `2.` a workspace whose room id is its workspace id.

use crate::client::InviteSecret;
use crate::frame::RoomId;

/// The link's scheme and path as written.
const PREFIX: &str = "curvyo://join";
/// The version of a link to one document.
const DOCUMENT_VERSION: &str = "1.";
/// The version of a link to a workspace.
const WORKSPACE_VERSION: &str = "2.";
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// What an invite link opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteKind {
    /// One document (version `1.`).
    Document,
    /// A whole workspace (version `2.`).
    Workspace,
}

/// An invite link: the relay to reach, what it opens and the one-time secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteLink {
    /// The relay address, as the inviting device has it (`ws://host:port`).
    pub server: String,
    /// A document or a workspace.
    pub kind: InviteKind,
    /// The room and the invite seed.
    pub secret: InviteSecret,
}

/// Why a text is not an invite link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InviteLinkError {
    /// Not of the link's shape, or its secret is damaged.
    #[error("this link is not valid")]
    Malformed,
    /// A link version this build does not know.
    #[error("this link needs a newer version of Curvyo")]
    UnknownVersion,
}

impl InviteLink {
    /// The link's text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut secret = Vec::with_capacity(48);
        secret.extend_from_slice(&self.secret.room.0);
        secret.extend_from_slice(&self.secret.seed);
        let version = match self.kind {
            InviteKind::Document => DOCUMENT_VERSION,
            InviteKind::Workspace => WORKSPACE_VERSION,
        };
        format!(
            "{PREFIX}?server={}#{version}{}",
            percent_encode(&self.server),
            base64url(&secret)
        )
    }

    /// Reads a pasted link: `curvyo://join?server=…#1.…` or `#2.…`, or any other address with
    /// the same `server` parameter and fragment (an `https://…#1.…` shape). Spaces around it are
    /// ignored.
    ///
    /// # Errors
    /// `Malformed` for anything else, `UnknownVersion` for a fragment of another version.
    pub fn parse(text: &str) -> Result<Self, InviteLinkError> {
        let text = text.trim();
        let (address, fragment) = text.split_once('#').ok_or(InviteLinkError::Malformed)?;
        let (_, query) = address.split_once('?').ok_or(InviteLinkError::Malformed)?;
        let server = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("server="))
            .ok_or(InviteLinkError::Malformed)?;
        let server = percent_decode(server).ok_or(InviteLinkError::Malformed)?;
        if server.is_empty() {
            return Err(InviteLinkError::Malformed);
        }
        let found = [
            (DOCUMENT_VERSION, InviteKind::Document),
            (WORKSPACE_VERSION, InviteKind::Workspace),
        ]
        .into_iter()
        .find_map(|(version, kind)| Some((kind, fragment.strip_prefix(version)?)));
        let Some((kind, encoded)) = found else {
            let looks_versioned = fragment
                .split_once('.')
                .is_some_and(|(v, _)| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()));
            return Err(if looks_versioned {
                InviteLinkError::UnknownVersion
            } else {
                InviteLinkError::Malformed
            });
        };
        let bytes = unbase64url(encoded).ok_or(InviteLinkError::Malformed)?;
        let (room, seed) = bytes
            .split_at_checked(16)
            .ok_or(InviteLinkError::Malformed)?;
        let room: [u8; 16] = room.try_into().map_err(|_| InviteLinkError::Malformed)?;
        let seed: [u8; 32] = seed.try_into().map_err(|_| InviteLinkError::Malformed)?;
        Ok(Self {
            server,
            kind,
            secret: InviteSecret {
                room: RoomId(room),
                seed,
            },
        })
    }
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    out
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for k in 0..=chunk.len() {
            let index = (n >> (18 - 6 * k)) & 63;
            out.push(char::from(ALPHABET[index as usize]));
        }
    }
    out
}

fn unbase64url(text: &str) -> Option<Vec<u8>> {
    let mut values = Vec::with_capacity(text.len());
    for byte in text.bytes() {
        let index = ALPHABET.iter().position(|&c| c == byte)?;
        values.push(u32::try_from(index).ok()?);
    }
    if values.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    for chunk in values.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (k, v)| n | (v << (18 - 6 * k)));
        let produced = chunk.len() - 1;
        for k in 0..produced {
            out.push(u8::try_from((n >> (16 - 8 * k)) & 0xff).ok()?);
        }
        // Bits past the last whole byte must be zero, so each link has one spelling.
        let spare = 6 * chunk.len() - 8 * produced;
        if spare > 0 && chunk.len() < 4 && (n >> (24 - 6 * chunk.len())) & ((1 << spare) - 1) != 0 {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link() -> InviteLink {
        InviteLink {
            server: "ws://192.168.1.20:8787".to_owned(),
            kind: InviteKind::Document,
            secret: InviteSecret {
                room: RoomId([0xd0; 16]),
                seed: [7; 32],
            },
        }
    }

    #[test]
    fn a_link_round_trips_and_keeps_its_secret_after_the_hash() {
        let text = link().to_text();
        assert!(text.starts_with("curvyo://join?server=ws%3A%2F%2F192.168.1.20%3A8787#1."));
        let (before, _) = text.split_once('#').unwrap();
        assert!(!before.contains("0NDQ"), "no secret before the #");
        assert_eq!(InviteLink::parse(&text).unwrap(), link());
        assert_eq!(
            text.len(),
            "curvyo://join?server=ws%3A%2F%2F192.168.1.20%3A8787#1.".len() + 64
        );
    }

    #[test]
    fn an_https_shape_and_surrounding_spaces_are_read() {
        let text = link().to_text();
        let fragment = text.split_once('#').unwrap().1;
        let other = format!(
            "  https://curvyo.example/join?x=1&server=ws%3A%2F%2F192.168.1.20%3A8787#{fragment}\n"
        );
        assert_eq!(InviteLink::parse(&other).unwrap(), link());
    }

    #[test]
    fn damaged_links_are_refused() {
        let text = link().to_text();
        for bad in [
            "",
            "hello",
            "curvyo://join#1.AAAA",
            &text.replace("server=", "srv="),
            &text[..text.len() - 1],
            &format!("{text}A"),
            &text.replace("#1.", "#1.!"),
            &text.replace("?server=ws%3A", "?server=ws%3"),
        ] {
            assert_eq!(
                InviteLink::parse(bad),
                Err(InviteLinkError::Malformed),
                "{bad}"
            );
        }
        assert_eq!(
            InviteLink::parse(&text.replace("#1.", "#3.")),
            Err(InviteLinkError::UnknownVersion)
        );
    }

    /// `specs/0095-workspace` criterion 26: a workspace link is version `2.`, with the
    /// workspace id and the seed after `#`; version `1.` keeps its meaning.
    #[test]
    fn a_workspace_link_is_version_2() {
        let workspace = InviteLink {
            kind: InviteKind::Workspace,
            ..link()
        };
        let text = workspace.to_text();
        assert!(text.starts_with("curvyo://join?server=ws%3A%2F%2F192.168.1.20%3A8787#2."));
        assert_eq!(InviteLink::parse(&text).unwrap(), workspace);
        assert_eq!(
            InviteLink::parse(&text.replace("#2.", "#1.")).unwrap().kind,
            InviteKind::Document
        );
    }

    #[test]
    fn base64url_matches_the_standard_alphabet_without_padding() {
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(unbase64url("Zm9v").unwrap(), b"foo");
        assert_eq!(unbase64url("Zg").unwrap(), b"f");
        assert!(unbase64url("Zh").is_none(), "non-zero spare bits");
    }
}
