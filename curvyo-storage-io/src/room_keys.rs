//! The stored value of one shared document's epoch keys (ADR 0031 decision 12):
//! `version u8 = 1`, then `epoch u32 ‖ key[32]` repeated, little-endian.

use zeroize::Zeroizing;

/// The room-key value format this build writes and reads.
const ROOM_KEYS_VERSION: u8 = 1;
/// One `epoch u32 ‖ key[32]` record.
const RECORD_LEN: usize = 4 + 32;

/// The stored value of `keys` (`(epoch, key)`).
#[must_use]
pub fn encode_room_keys(keys: &[(u32, [u8; 32])]) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(1 + keys.len() * RECORD_LEN));
    out.push(ROOM_KEYS_VERSION);
    for (epoch, key) in keys {
        out.extend_from_slice(&epoch.to_le_bytes());
        out.extend_from_slice(key);
    }
    out
}

/// The keys of a stored value; `None` for another version or a cut record.
#[must_use]
pub fn decode_room_keys(bytes: &[u8]) -> Option<Vec<(u32, Zeroizing<[u8; 32]>)>> {
    let (&version, records) = bytes.split_first()?;
    if version != ROOM_KEYS_VERSION {
        return None;
    }
    let (chunks, rest) = records.as_chunks::<RECORD_LEN>();
    if !rest.is_empty() {
        return None;
    }
    Some(
        chunks
            .iter()
            .map(|chunk| {
                let (epoch, key) = chunk.split_at(4);
                let mut number = [0; 4];
                number.copy_from_slice(epoch);
                let mut secret = Zeroizing::new([0; 32]);
                secret.copy_from_slice(key);
                (u32::from_le_bytes(number), secret)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_keys_round_trip_and_other_versions_are_refused() {
        let bytes = encode_room_keys(&[(3, [9; 32]), (4, [1; 32])]);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes.len(), 1 + 2 * 36);
        let keys = decode_room_keys(&bytes).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!((keys[1].0, *keys[1].1), (4, [1; 32]));
        let mut other = bytes.to_vec();
        other[0] = 2;
        assert!(decode_room_keys(&other).is_none());
        assert!(decode_room_keys(&bytes[..bytes.len() - 1]).is_none());
        assert!(decode_room_keys(&[]).is_none());
    }
}
