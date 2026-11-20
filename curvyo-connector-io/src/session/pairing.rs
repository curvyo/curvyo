//! A session's side of pairing (addendum A6): the two SPAKE2 rounds, the refusals and the stored
//! client.

use std::time::Instant;

use curvyo_connector_core::Message;

use super::{MAX_NAME, PAIRING_IDLE, Pairing, Session, Socket, error, send, send_frame};
use crate::frames::{Frame, IoFrame, RefusalReason};
use crate::hex;
use crate::pairing_code::Refusal;
use crate::spake;
use crate::token::Token;
use curvyo_connector_core::ErrorCode;

impl Session {
    pub(super) fn pair_round_one(
        &mut self,
        socket: &mut Socket,
        name: &str,
        message: &str,
    ) -> bool {
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control)
        {
            return send(
                socket,
                error(ErrorCode::Malformed, "The client name is not valid."),
            );
        }
        let now = Instant::now();
        let password = match self.hub.begin_pair(now) {
            Ok(password) => password,
            Err(Refusal::Expired) => return Self::refuse(socket, RefusalReason::CodeExpired, 0),
            Err(Refusal::NoCode) => return Self::refuse(socket, RefusalReason::NoCode, 0),
        };
        let (exchange, reply) = spake::start_connector(&password, &self.hub.spki());
        let key = hex::decode(message).and_then(|m| exchange.finish(&m).ok());
        let Some(key) = key else {
            let left = self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::WrongCode, left);
        };
        self.pairing = Some(Pairing {
            key,
            client_name: name.to_owned(),
            deadline: now + PAIRING_IDLE,
        });
        send_frame(
            socket,
            &Frame::Io(IoFrame::PairSpake {
                round: 1,
                client_name: None,
                message: hex::encode(&reply),
            }),
        )
    }

    pub(super) fn pair_round_two(&mut self, socket: &mut Socket, message: &str) -> bool {
        let Some(pairing) = self.pairing.take() else {
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        };
        let verified = hex::decode(message).is_some_and(|tag| pairing.key.verify_app_tag(&tag));
        if !verified {
            let left = self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::WrongCode, left);
        }
        let Some(token) = Token::generate() else {
            self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        };
        let Ok((name, role)) = self
            .hub
            .pair_succeeded(&pairing.client_name, &token.sha256_hex())
        else {
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        };
        let confirmation = Frame::Io(IoFrame::PairSpake {
            round: 2,
            client_name: None,
            message: hex::encode(&pairing.key.connector_tag()),
        });
        self.limiter.answered(Instant::now());
        send_frame(socket, &confirmation)
            && send(
                socket,
                Message::Paired {
                    token: token.to_hex().to_string(),
                },
            )
            && self.become_authenticated(socket, name, role)
            && send(socket, self.hub.status())
    }
}
