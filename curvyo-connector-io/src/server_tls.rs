//! The connector's TLS configuration: TLS 1.3 only, no client authentication, its one key.

use std::sync::Arc;

use rustls::ServerConfig;
use rustls::crypto::ring;

use crate::identity::Identity;

/// The server side of every connection. The version list is explicit, so TLS 1.3 only holds even
/// if feature unification ever turned `tls12` on.
///
/// # Errors
/// `rustls::Error` when the key does not fit the certificate.
pub fn server_config(identity: &Identity) -> Result<Arc<ServerConfig>, rustls::Error> {
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![identity.certificate.clone()],
            identity.key.clone_key().into(),
        )?;
    Ok(Arc::new(config))
}
