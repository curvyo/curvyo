//! The app's check of the connector's certificate: the pinned key is the trust, nothing else is
//! looked at (addendum A4). The server name, the chain and the dates are not checked.

use std::sync::{Arc, Mutex, PoisonError};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, ring, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};

use crate::spki::SpkiHash;

/// What the verifier does with the key it sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
    /// Every paired connection: only this key is accepted.
    Expect(SpkiHash),
    /// First contact while pairing: any key is taken and remembered; the SPAKE2 exchange then
    /// decides whether it is the connector's.
    Capture,
}

/// The pinned-key verifier.
#[derive(Debug)]
pub struct PinVerifier {
    mode: PinMode,
    seen: Mutex<Option<SpkiHash>>,
    algorithms: WebPkiSupportedAlgorithms,
}

impl PinVerifier {
    /// A verifier in `mode`.
    #[must_use]
    pub fn new(mode: PinMode) -> Arc<Self> {
        Arc::new(Self {
            mode,
            seen: Mutex::new(None),
            algorithms: ring::default_provider().signature_verification_algorithms,
        })
    }

    /// The hash of the key the peer presented, once it did.
    #[must_use]
    pub fn seen(&self) -> Option<SpkiHash> {
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let parsed = ParsedCertificate::try_from(end_entity)?;
        let hash = SpkiHash::of_spki_der(parsed.subject_public_key_info().as_ref());
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner) = Some(hash);
        match self.mode {
            PinMode::Expect(pinned) if pinned != hash => Err(rustls::Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            )),
            _ => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not used".to_owned()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}
