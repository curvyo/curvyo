//! The app's side of pairing: SPAKE2 over the open connection (addendum A6).

use std::time::Instant;

use curvyo_connector_core::Message;

use crate::client::{ClientError, Connection};
use crate::frames::{Frame, IoFrame, RefusalReason};
use crate::hex;
use crate::pairing_code::normalise;
use crate::record::PairingRecord;
use crate::spake;
use crate::token::Token;

/// Why pairing did not end with a token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PairError {
    /// The typed text is not a code.
    #[error("not a pairing code")]
    BadCode,
    /// The connector refused: a wrong code, or another key in between (the two look the same).
    #[error("pairing refused: {reason:?}, {attempts_left} attempts left")]
    Refused {
        /// Why.
        reason: RefusalReason,
        /// What the code still allows.
        attempts_left: u8,
    },
    /// The connector's confirmation did not verify; nothing was stored.
    #[error("the connector's confirmation did not verify")]
    Unverified,
    /// The connection failed.
    #[error(transparent)]
    Client(#[from] ClientError),
}

fn expect_spake(frame: Frame, round: u8) -> Result<Vec<u8>, PairError> {
    match frame {
        Frame::Io(IoFrame::PairSpake {
            round: r, message, ..
        }) if r == round => hex::decode(&message).ok_or(PairError::Unverified),
        Frame::Io(IoFrame::PairRefused {
            reason,
            attempts_left,
        }) => Err(PairError::Refused {
            reason,
            attempts_left,
        }),
        Frame::Core(_) | Frame::Io(_) => Err(PairError::Client(ClientError::Protocol(
            "not a pairing answer".to_owned(),
        ))),
    }
}

impl Connection {
    /// Pairs with the typed `code`; on success the record to store (the key this connection saw,
    /// the token and the connector's name). Nothing is stored here.
    ///
    /// # Errors
    /// [`PairError`].
    pub fn pair(
        &mut self,
        code: &str,
        client_name: &str,
        deadline: Instant,
    ) -> Result<PairingRecord, PairError> {
        let password = normalise(code).ok_or(PairError::BadCode)?;
        let (exchange, message) = spake::start_app(&password, &self.spki);
        self.send_frame(&Frame::Io(IoFrame::PairSpake {
            round: 1,
            client_name: Some(client_name.to_owned()),
            message: hex::encode(&message),
        }))?;
        let reply = expect_spake(self.recv(deadline)?, 1)?;
        let key = exchange.finish(&reply).map_err(|_| PairError::Unverified)?;
        self.send_frame(&Frame::Io(IoFrame::PairSpake {
            round: 2,
            client_name: None,
            message: hex::encode(&key.app_tag()),
        }))?;
        let confirmation = expect_spake(self.recv(deadline)?, 2)?;
        if !key.verify_connector_tag(&confirmation) {
            return Err(PairError::Unverified);
        }
        let token = loop {
            match self.recv_message(deadline)? {
                Message::Paired { token } => break token,
                Message::Status { .. } => {}
                other => {
                    return Err(PairError::Client(ClientError::Protocol(format!(
                        "{other:?}"
                    ))));
                }
            }
        };
        Ok(PairingRecord {
            spki: self.spki,
            token: Token::from_hex(&token)
                .ok_or_else(|| ClientError::Protocol("the token is not valid".to_owned()))?,
            connector_name: self.greeting.name.clone(),
        })
    }
}
