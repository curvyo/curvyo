//! The one place the online library touches the network (ADR 0085 decision 2): a blocking GET to
//! a request `curvyo-library-core` built, `https` only, no cookies, no `Referer`, at most three
//! redirects each to `https`, 10 s to connect and 60 s in all, the answer cut at a size limit.

use std::time::Duration;

use curvyo_library_core::{Failure, Request, host_of};

use crate::catalogue_token::AccessToken;

/// Why a GET gave no body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    /// The connection failed (DNS, connect, timeout, TLS, or a cut answer).
    #[error("no connection")]
    Connection,
    /// The server answered another status than 200.
    #[error("the server answered {0}")]
    Status(u16),
    /// The answer is over the limit.
    #[error("the answer is too large")]
    TooLarge,
    /// A redirect to another host or to a plain address, which the guard refuses.
    #[error("the redirect is refused")]
    Redirect,
}

impl From<HttpError> for Failure {
    fn from(error: HttpError) -> Self {
        match error {
            HttpError::Connection => Self::Offline,
            HttpError::Status(404) => Self::NotFound,
            HttpError::Status(code) => Self::Status(code),
            HttpError::TooLarge => Self::TooLarge,
            HttpError::Redirect => Self::Redirect,
        }
    }
}

/// Sends a request and returns its body; the host's one network door.
pub trait Transport {
    /// GET `request`; the body at most `max_bytes` long.
    ///
    /// # Errors
    /// [`HttpError`].
    fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError>;

    /// GET `request` with `Authorization: Bearer <token>`, sent only to the request's own origin
    /// (ADR 0090 decision 5). A transport that cannot send a token fails closed: it never sends the
    /// request without it.
    ///
    /// # Errors
    /// [`HttpError`].
    fn get_bearer(
        &self,
        _request: &Request,
        _max_bytes: usize,
        _token: &AccessToken,
    ) -> Result<Vec<u8>, HttpError> {
        Err(HttpError::Connection)
    }
}

/// A [`Transport`] that sends the token of one Git catalogue with every request that goes through
/// it, to the request's own origin only (ADR 0090 decision 5).
pub struct Authorized<'a> {
    inner: &'a dyn Transport,
    token: &'a AccessToken,
}

impl<'a> Authorized<'a> {
    /// `inner`, with `token` on every request.
    #[must_use]
    pub const fn new(inner: &'a dyn Transport, token: &'a AccessToken) -> Self {
        Self { inner, token }
    }
}

impl Transport for Authorized<'_> {
    fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError> {
        self.inner.get_bearer(request, max_bytes, self.token)
    }

    fn get_bearer(
        &self,
        request: &Request,
        max_bytes: usize,
        _token: &AccessToken,
    ) -> Result<Vec<u8>, HttpError> {
        self.get(request, max_bytes)
    }
}

const MAX_REDIRECTS: usize = 3;

/// [`Transport`] over `ureq` with the operating system's trust roots.
pub struct UreqTransport {
    https_only: bool,
    /// Built on the first request: reading the system's roots is not done at start.
    agent: std::sync::OnceLock<ureq::Agent>,
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new(true)
    }
}

impl UreqTransport {
    /// The transport; `https_only` is false only in the tests, which talk to a local server.
    const fn new(https_only: bool) -> Self {
        Self {
            https_only,
            agent: std::sync::OnceLock::new(),
        }
    }

    fn agent(&self) -> &ureq::Agent {
        self.agent.get_or_init(|| {
            // `ureq` finds the `ring` provider this crate enables on `rustls`; a second install is
            // refused and harmless.
            let _ = rustls::crypto::ring::default_provider().install_default();
            let roots: Vec<ureq::tls::Certificate<'static>> =
                rustls_native_certs::load_native_certs()
                    .certs
                    .iter()
                    .map(|der| ureq::tls::Certificate::from_der(der.as_ref()).to_owned())
                    .collect();
            let tls = ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .root_certs(ureq::tls::RootCerts::from(roots))
                .build();
            ureq::Agent::config_builder()
                .https_only(self.https_only)
                .max_redirects(0)
                .http_status_as_error(false)
                .timeout_connect(Some(Duration::from_secs(10)))
                .timeout_global(Some(Duration::from_secs(60)))
                .user_agent("Curvyo")
                .tls_config(tls)
                .build()
                .new_agent()
        })
    }

    fn get_url(
        &self,
        url: &str,
        pin_host: bool,
        max_bytes: usize,
        token: Option<&AccessToken>,
    ) -> Result<Vec<u8>, HttpError> {
        let origin = host_of(url);
        let full_origin = origin_of(url);
        // A request with a token stays on its own origin: another one is refused, not followed.
        let pin_host = pin_host || token.is_some();
        let mut url = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            let mut request = self.agent().get(&url);
            if let Some(token) = token
                && origin_of(&url) == full_origin
            {
                // Per request, never on the agent: a hop elsewhere cannot carry it.
                request = request.header("Authorization", token.bearer().as_str());
            }
            let mut response = request.call().map_err(|_| HttpError::Connection)?;
            let status = response.status().as_u16();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                let next = response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok())
                    .ok_or(HttpError::Redirect)?;
                url = redirect_target(&url, next, pin_host, origin.as_deref())?;
                if token.is_some() && origin_of(&url) != full_origin {
                    return Err(HttpError::Redirect);
                }
                continue;
            }
            if status != 200 {
                return Err(HttpError::Status(status));
            }
            let limit = u64::try_from(max_bytes).unwrap_or(u64::MAX);
            return response
                .body_mut()
                .with_config()
                .limit(limit)
                .read_to_vec()
                .map_err(|error| match error {
                    ureq::Error::BodyExceedsLimit(_) => HttpError::TooLarge,
                    _ => HttpError::Connection,
                });
        }
        Err(HttpError::Redirect)
    }
}

/// Scheme, host and port of a URL, lower case: the unit a token is bound to.
fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    (!authority.is_empty()).then(|| format!("{scheme}://{authority}").to_ascii_lowercase())
}

/// The URL a redirect leads to: an absolute `https` URL, or a path on the same host; for the
/// ready entry only on the host it started on (ADR 0085 decision 1).
fn redirect_target(
    from: &str,
    location: &str,
    pin_host: bool,
    origin: Option<&str>,
) -> Result<String, HttpError> {
    let next = if location.starts_with('/') && !location.starts_with("//") {
        let host = from.split('/').nth(2).ok_or(HttpError::Redirect)?;
        let scheme = from.split("://").next().ok_or(HttpError::Redirect)?;
        return Ok(format!("{scheme}://{host}{location}"));
    } else {
        location.to_owned()
    };
    let next_host = host_of(&next).ok_or(HttpError::Redirect)?;
    if pin_host && Some(next_host.as_str()) != origin {
        return Err(HttpError::Redirect);
    }
    Ok(next)
}

impl Transport for UreqTransport {
    fn get(&self, request: &Request, max_bytes: usize) -> Result<Vec<u8>, HttpError> {
        self.get_url(request.url(), request.pins_host(), max_bytes, None)
    }

    fn get_bearer(
        &self,
        request: &Request,
        max_bytes: usize,
        token: &AccessToken,
    ) -> Result<Vec<u8>, HttpError> {
        self.get_url(request.url(), request.pins_host(), max_bytes, Some(token))
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    /// A local server that answers each connection with the next canned response.
    fn serve(responses: Vec<String>) -> String {
        serve_recording(responses).0
    }

    /// [`serve`], and the requests the server read, in order.
    fn serve_recording(
        responses: Vec<String>,
    ) -> (String, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0_u8; 4096];
                let n = stream.read(&mut buffer).unwrap_or(0);
                log.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buffer[..n]).into_owned());
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://127.0.0.1:{port}"), seen)
    }

    fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn a_get_returns_the_body_and_maps_statuses() {
        let base = serve(vec![
            ok("hello"),
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        ]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport
                .get_url(&format!("{base}/a"), false, 100, None)
                .unwrap(),
            b"hello"
        );
        assert_eq!(
            transport.get_url(&format!("{base}/b"), false, 100, None),
            Err(HttpError::Status(404))
        );
    }

    #[test]
    fn an_answer_over_the_limit_is_refused() {
        let base = serve(vec![ok("0123456789")]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport.get_url(&format!("{base}/a"), false, 5, None),
            Err(HttpError::TooLarge)
        );
    }

    #[test]
    fn a_closed_port_is_a_connection_failure() {
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport.get_url("http://127.0.0.1:1/", false, 5, None),
            Err(HttpError::Connection)
        );
    }

    #[test]
    fn plain_http_is_refused_by_the_real_transport() {
        let transport = UreqTransport::default();
        assert_eq!(
            transport.get_url("http://127.0.0.1:1/", false, 5, None),
            Err(HttpError::Connection)
        );
    }

    #[test]
    fn a_redirect_is_followed_on_the_same_host_and_a_pinned_one_stays_there() {
        let base = serve(vec![
            "HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            ok("moved"),
        ]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport
                .get_url(&format!("{base}/a"), true, 100, None)
                .unwrap(),
            b"moved"
        );
        assert_eq!(
            redirect_target(
                "https://raw.githubusercontent.com/a",
                "https://evil.example/x",
                true,
                Some("raw.githubusercontent.com")
            ),
            Err(HttpError::Redirect)
        );
        assert!(
            redirect_target(
                "https://example.org/a",
                "https://other.example/x",
                false,
                Some("example.org")
            )
            .is_ok()
        );
        assert_eq!(
            redirect_target(
                "https://example.org/a",
                "http://other.example/x",
                false,
                Some("example.org")
            ),
            Err(HttpError::Redirect),
            "every hop is https"
        );
    }

    #[test]
    fn failures_map_to_the_source_states() {
        assert_eq!(Failure::from(HttpError::Connection), Failure::Offline);
        assert_eq!(Failure::from(HttpError::Status(404)), Failure::NotFound);
        assert_eq!(Failure::from(HttpError::Status(500)), Failure::Status(500));
    }

    #[test]
    fn a_redirect_to_a_plain_address_is_refused() {
        let base = serve(vec![
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        ]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport.get_url(&format!("{base}/a"), false, 100, None),
            Err(HttpError::Redirect)
        );
    }

    #[test]
    fn more_than_three_redirects_are_refused() {
        let hop = "HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let base = serve(vec![
            hop.into(),
            hop.into(),
            hop.into(),
            hop.into(),
            ok("late"),
        ]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport.get_url(&format!("{base}/a"), false, 100, None),
            Err(HttpError::Redirect)
        );
    }

    #[test]
    fn a_request_carries_no_cookie_no_referer_and_the_plain_user_agent() {
        let set_cookie = "HTTP/1.1 200 OK\r\nSet-Cookie: id=1\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx";
        let (base, seen) = serve_recording(vec![set_cookie.into(), ok("y")]);
        let transport = UreqTransport::new(false);
        transport
            .get_url(&format!("{base}/a"), false, 100, None)
            .unwrap();
        transport
            .get_url(&format!("{base}/b"), false, 100, None)
            .unwrap();
        let requests = seen.lock().unwrap().clone();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            let lower = request.to_ascii_lowercase();
            assert!(request.starts_with("GET "), "{request}");
            assert!(lower.contains("user-agent: curvyo\r\n"), "{request}");
            assert!(!lower.contains("cookie:"), "{request}");
            assert!(!lower.contains("referer:"), "{request}");
        }
    }

    #[test]
    fn the_token_is_sent_on_the_first_hop_and_a_same_origin_redirect_only_when_given() {
        let hop = "HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let token = AccessToken::parse("SENTINEL-tok_123").unwrap();
        let (base, seen) = serve_recording(vec![hop.into(), ok("a"), ok("b")]);
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport
                .get_url(&format!("{base}/x"), false, 100, Some(&token))
                .unwrap(),
            b"a"
        );
        transport
            .get_url(&format!("{base}/y"), false, 100, None)
            .unwrap();
        let requests = seen.lock().unwrap().clone();
        assert_eq!(requests.len(), 3);
        for with in &requests[..2] {
            assert!(
                with.contains("authorization: Bearer SENTINEL-tok_123\r\n")
                    || with.contains("Authorization: Bearer SENTINEL-tok_123\r\n"),
                "{with}"
            );
        }
        assert!(
            !requests[2].to_ascii_lowercase().contains("authorization"),
            "no token, no header"
        );
    }

    #[test]
    fn a_redirect_to_another_host_is_refused_and_never_carries_the_token() {
        let (base, seen) = serve_recording(vec![
            "HTTP/1.1 302 Found\r\nLocation: https://localhost:9/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        ]);
        let token = AccessToken::parse("SENTINEL-tok_123").unwrap();
        let transport = UreqTransport::new(false);
        assert_eq!(
            transport.get_url(&format!("{base}/x"), false, 100, Some(&token)),
            Err(HttpError::Redirect)
        );
        assert_eq!(seen.lock().unwrap().len(), 1, "nothing was sent on the hop");
    }

    #[test]
    fn an_origin_is_scheme_host_and_port() {
        assert_eq!(
            origin_of("HTTPS://Example.org:8443/a/b").as_deref(),
            Some("https://example.org:8443")
        );
        assert_ne!(
            origin_of("https://e.org/a"),
            origin_of("https://e.org:8443/a")
        );
        assert_ne!(origin_of("https://e.org/a"), origin_of("http://e.org/a"));
        assert_eq!(origin_of("no scheme"), None);
    }

    #[test]
    fn authorized_hands_its_token_to_the_inner_transport_and_a_plain_double_fails_closed() {
        struct Plain;
        impl Transport for Plain {
            fn get(&self, _: &Request, _: usize) -> Result<Vec<u8>, HttpError> {
                Ok(b"anonymous".to_vec())
            }
        }
        let token = AccessToken::parse("t").unwrap();
        let request = curvyo_library_core::Catalogue::request("https://example.org/cat/");
        assert_eq!(
            Authorized::new(&Plain, &token).get(&request, 10),
            Err(HttpError::Connection),
            "never an anonymous request in place of an authorised one"
        );
    }
}
