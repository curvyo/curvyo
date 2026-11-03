//! The hand-written byte codec of the frames: tag byte, little-endian fixed fields, trailing
//! bytes field.

use curvyo_crypto_core::MemberId;

use super::{
    ClientFrame, ConnectionId, FrameError, MAX_MESSAGE_LEN, RecordKind, RefusalReason, RelayFrame,
    RoomId,
};

mod tag {
    pub(super) const HELLO: u8 = 0x01;
    pub(super) const PROOF: u8 = 0x02;
    pub(super) const KEYRING: u8 = 0x03;
    pub(super) const PUSH: u8 = 0x04;
    pub(super) const SNAPSHOT: u8 = 0x05;
    pub(super) const AWARENESS: u8 = 0x06;
    pub(super) const PING: u8 = 0x07;
    pub(super) const SNAPSHOT_PART: u8 = 0x08;
    pub(super) const CHALLENGE: u8 = 0x81;
    pub(super) const WELCOME: u8 = 0x82;
    pub(super) const RELAY_KEYRING: u8 = 0x83;
    pub(super) const RECORD: u8 = 0x84;
    pub(super) const ACK: u8 = 0x85;
    pub(super) const PEER: u8 = 0x86;
    pub(super) const RELAY_AWARENESS: u8 = 0x87;
    pub(super) const REFUSED: u8 = 0x88;
    pub(super) const PONG: u8 = 0x89;
}

impl RecordKind {
    fn tag(self) -> u8 {
        match self {
            Self::Update => 1,
            Self::Asset => 2,
            Self::Snapshot => 3,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, FrameError> {
        match tag {
            1 => Ok(Self::Update),
            2 => Ok(Self::Asset),
            3 => Ok(Self::Snapshot),
            _ => Err(FrameError::Malformed),
        }
    }
}

impl RefusalReason {
    fn tag(self) -> u8 {
        match self {
            Self::Used => 1,
            Self::Expired => 2,
            Self::NotAMember => 3,
            Self::TooLarge => 4,
            Self::Protocol => 5,
            Self::Removed => 6,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, FrameError> {
        match tag {
            1 => Ok(Self::Used),
            2 => Ok(Self::Expired),
            3 => Ok(Self::NotAMember),
            4 => Ok(Self::TooLarge),
            5 => Ok(Self::Protocol),
            6 => Ok(Self::Removed),
            _ => Err(FrameError::Malformed),
        }
    }
}

impl ClientFrame {
    /// The message bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Hello {
                protocol,
                room,
                since_seq,
                identity,
            } => {
                out.push(tag::HELLO);
                out.extend_from_slice(&protocol.to_le_bytes());
                out.extend_from_slice(&room.0);
                out.extend_from_slice(&since_seq.to_le_bytes());
                out.extend_from_slice(&identity.0);
            }
            Self::Proof { signature } => {
                out.push(tag::PROOF);
                out.extend_from_slice(signature);
            }
            Self::Keyring { entry } => {
                out.push(tag::KEYRING);
                out.extend_from_slice(entry);
            }
            Self::Push {
                client_id,
                kind,
                envelope,
            } => {
                out.push(tag::PUSH);
                out.extend_from_slice(&client_id.to_le_bytes());
                out.push(kind.tag());
                out.extend_from_slice(envelope);
            }
            Self::Snapshot {
                covers_seq,
                envelope,
            } => {
                out.push(tag::SNAPSHOT);
                out.extend_from_slice(&covers_seq.to_le_bytes());
                out.extend_from_slice(envelope);
            }
            Self::Awareness { envelope } => {
                out.push(tag::AWARENESS);
                out.extend_from_slice(envelope);
            }
            Self::Ping => out.push(tag::PING),
            Self::SnapshotPart {
                covers_seq,
                index,
                last,
                envelope,
            } => {
                out.push(tag::SNAPSHOT_PART);
                out.extend_from_slice(&covers_seq.to_le_bytes());
                out.extend_from_slice(&index.to_le_bytes());
                out.push(u8::from(*last));
                out.extend_from_slice(envelope);
            }
        }
        out
    }

    /// Reads one message.
    ///
    /// # Errors
    /// `TooLarge` over [`MAX_MESSAGE_LEN`]; `Malformed` otherwise (a `Push` of kind snapshot
    /// among them).
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes)?;
        let frame = match reader.u8()? {
            tag::HELLO => Self::Hello {
                protocol: reader.u16()?,
                room: RoomId(reader.array()?),
                since_seq: reader.u64()?,
                identity: MemberId(reader.array()?),
            },
            tag::PROOF => Self::Proof {
                signature: reader.array()?,
            },
            tag::KEYRING => Self::Keyring {
                entry: reader.rest_nonempty()?,
            },
            tag::PUSH => {
                let client_id = reader.u64()?;
                let kind = RecordKind::from_tag(reader.u8()?)?;
                if kind == RecordKind::Snapshot {
                    return Err(FrameError::Malformed);
                }
                Self::Push {
                    client_id,
                    kind,
                    envelope: reader.rest_nonempty()?,
                }
            }
            tag::SNAPSHOT => Self::Snapshot {
                covers_seq: reader.u64()?,
                envelope: reader.rest_nonempty()?,
            },
            tag::AWARENESS => Self::Awareness {
                envelope: reader.rest_nonempty()?,
            },
            tag::PING => Self::Ping,
            tag::SNAPSHOT_PART => Self::SnapshotPart {
                covers_seq: reader.u64()?,
                index: reader.u32()?,
                last: match reader.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(FrameError::Malformed),
                },
                envelope: reader.rest_nonempty()?,
            },
            _ => return Err(FrameError::Malformed),
        };
        reader.finish()?;
        Ok(frame)
    }
}

impl RelayFrame {
    /// The message bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Challenge { nonce } => {
                out.push(tag::CHALLENGE);
                out.extend_from_slice(nonce);
            }
            Self::Welcome {
                head_seq,
                snapshot_seq,
                connection,
            } => {
                out.push(tag::WELCOME);
                out.extend_from_slice(&head_seq.to_le_bytes());
                out.extend_from_slice(&snapshot_seq.to_le_bytes());
                out.extend_from_slice(&connection.0.to_le_bytes());
            }
            Self::Keyring { entry } => {
                out.push(tag::RELAY_KEYRING);
                out.extend_from_slice(entry);
            }
            Self::Record {
                seq,
                kind,
                envelope,
            } => {
                out.push(tag::RECORD);
                out.extend_from_slice(&seq.to_le_bytes());
                out.push(kind.tag());
                out.extend_from_slice(envelope);
            }
            Self::Ack { client_id, seq } => {
                out.push(tag::ACK);
                out.extend_from_slice(&client_id.to_le_bytes());
                out.extend_from_slice(&seq.to_le_bytes());
            }
            Self::Peer {
                connection,
                identity,
            } => {
                out.push(tag::PEER);
                out.extend_from_slice(&connection.0.to_le_bytes());
                match identity {
                    Some(member) => {
                        out.push(1);
                        out.extend_from_slice(&member.0);
                    }
                    None => out.push(0),
                }
            }
            Self::Awareness {
                connection,
                envelope,
            } => {
                out.push(tag::RELAY_AWARENESS);
                out.extend_from_slice(&connection.0.to_le_bytes());
                out.extend_from_slice(envelope);
            }
            Self::Refused { reason } => {
                out.push(tag::REFUSED);
                out.push(reason.tag());
            }
            Self::Pong => out.push(tag::PONG),
        }
        out
    }

    /// Reads one message.
    ///
    /// # Errors
    /// `TooLarge` over [`MAX_MESSAGE_LEN`]; `Malformed` otherwise.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = Reader::new(bytes)?;
        let frame = match reader.u8()? {
            tag::CHALLENGE => Self::Challenge {
                nonce: reader.array()?,
            },
            tag::WELCOME => Self::Welcome {
                head_seq: reader.u64()?,
                snapshot_seq: reader.u64()?,
                connection: ConnectionId(reader.u64()?),
            },
            tag::RELAY_KEYRING => Self::Keyring {
                entry: reader.rest_nonempty()?,
            },
            tag::RECORD => Self::Record {
                seq: reader.u64()?,
                kind: RecordKind::from_tag(reader.u8()?)?,
                envelope: reader.rest_nonempty()?,
            },
            tag::ACK => Self::Ack {
                client_id: reader.u64()?,
                seq: reader.u64()?,
            },
            tag::PEER => {
                let connection = ConnectionId(reader.u64()?);
                let identity = match reader.u8()? {
                    0 => None,
                    1 => Some(MemberId(reader.array()?)),
                    _ => return Err(FrameError::Malformed),
                };
                Self::Peer {
                    connection,
                    identity,
                }
            }
            tag::RELAY_AWARENESS => Self::Awareness {
                connection: ConnectionId(reader.u64()?),
                envelope: reader.rest_nonempty()?,
            },
            tag::REFUSED => Self::Refused {
                reason: RefusalReason::from_tag(reader.u8()?)?,
            },
            tag::PONG => Self::Pong,
            _ => return Err(FrameError::Malformed),
        };
        reader.finish()?;
        Ok(frame)
    }
}

/// Reads fixed-width little-endian fields; every read past the end is `Malformed`.
struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, FrameError> {
        if bytes.len() > MAX_MESSAGE_LEN {
            return Err(FrameError::TooLarge);
        }
        Ok(Self { bytes })
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
        let (head, tail) = self
            .bytes
            .split_first_chunk::<N>()
            .ok_or(FrameError::Malformed)?;
        self.bytes = tail;
        Ok(*head)
    }

    fn u8(&mut self) -> Result<u8, FrameError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, FrameError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, FrameError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, FrameError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// The trailing bytes field; an empty one is malformed (no frame carries empty bytes).
    fn rest_nonempty(&mut self) -> Result<Vec<u8>, FrameError> {
        if self.bytes.is_empty() {
            return Err(FrameError::Malformed);
        }
        let rest = self.bytes.to_vec();
        self.bytes = &[];
        Ok(rest)
    }

    fn finish(self) -> Result<(), FrameError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(FrameError::Malformed)
        }
    }
}
