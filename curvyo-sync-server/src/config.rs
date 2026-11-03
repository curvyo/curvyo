//! The server's configuration from command-line flags and environment variables, the flag
//! winning (ADR 0031 decision 11, criterion 35).

use std::net::SocketAddr;
use std::path::PathBuf;

/// The listen address when neither `--listen` nor `CURVYO_SYNC_LISTEN` is given.
pub const DEFAULT_LISTEN: &str = "0.0.0.0:8787";
/// The data directory when neither `--data` nor `CURVYO_SYNC_DATA` is given.
pub const DEFAULT_DATA: &str = "/data";

/// The usage text printed for a wrong command line.
pub const USAGE: &str = "usage: curvyo-sync-server [--listen <address:port>] [--data <directory>]\n\
     environment: CURVYO_SYNC_LISTEN, CURVYO_SYNC_DATA (a flag wins)";

/// Everything the server needs to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The address the WebSocket and `/healthz` listen on.
    pub listen: SocketAddr,
    /// The one directory holding every room (criterion 38).
    pub data: PathBuf,
}

/// Why the command line or the environment is not a configuration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// An argument that is not one of the two flags.
    #[error("unknown argument `{0}`")]
    UnknownArgument(String),
    /// A flag without its value.
    #[error("`{0}` needs a value")]
    MissingValue(String),
    /// The listen address does not parse as `address:port`.
    #[error("`{0}` is not an address:port")]
    BadListen(String),
}

impl Config {
    /// The configuration from `args` (without the program name) and the environment variable
    /// lookup `env`: a flag wins over its variable, a variable over the default.
    ///
    /// # Errors
    /// See [`ConfigError`].
    pub fn from_args_and_env(
        args: impl IntoIterator<Item = String>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let mut listen = None;
        let mut data = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let (flag, inline) = match arg.split_once('=') {
                Some((flag, value)) => (flag.to_owned(), Some(value.to_owned())),
                None => (arg.clone(), None),
            };
            let slot = match flag.as_str() {
                "--listen" => &mut listen,
                "--data" => &mut data,
                _ => return Err(ConfigError::UnknownArgument(arg)),
            };
            let value = match inline {
                Some(value) => value,
                None => args.next().ok_or(ConfigError::MissingValue(flag))?,
            };
            *slot = Some(value);
        }
        let listen = listen
            .or_else(|| env("CURVYO_SYNC_LISTEN"))
            .unwrap_or_else(|| DEFAULT_LISTEN.to_owned());
        let data = data
            .or_else(|| env("CURVYO_SYNC_DATA"))
            .unwrap_or_else(|| DEFAULT_DATA.to_owned());
        Ok(Self {
            listen: listen
                .parse()
                .map_err(|_| ConfigError::BadListen(listen.clone()))?,
            data: PathBuf::from(data),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn defaults_without_flags_or_environment() {
        let config = Config::from_args_and_env(args(&[]), env_of(&[])).unwrap();
        assert_eq!(config.listen, "0.0.0.0:8787".parse().unwrap());
        assert_eq!(config.data, PathBuf::from("/data"));
    }

    #[test]
    fn the_environment_replaces_the_defaults() {
        let env = env_of(&[
            ("CURVYO_SYNC_LISTEN", "127.0.0.1:9000"),
            ("CURVYO_SYNC_DATA", "/srv/relay"),
        ]);
        let config = Config::from_args_and_env(args(&[]), env).unwrap();
        assert_eq!(config.listen, "127.0.0.1:9000".parse().unwrap());
        assert_eq!(config.data, PathBuf::from("/srv/relay"));
    }

    /// Criterion 35: a flag overrides the environment variable.
    #[test]
    fn a_flag_overrides_the_environment_variable() {
        let env = env_of(&[
            ("CURVYO_SYNC_LISTEN", "127.0.0.1:9000"),
            ("CURVYO_SYNC_DATA", "/srv/relay"),
        ]);
        let config = Config::from_args_and_env(
            args(&["--listen", "127.0.0.1:7000", "--data=/tmp/rooms"]),
            env,
        )
        .unwrap();
        assert_eq!(config.listen, "127.0.0.1:7000".parse().unwrap());
        assert_eq!(config.data, PathBuf::from("/tmp/rooms"));
    }

    #[test]
    fn a_wrong_command_line_is_refused() {
        let none = env_of(&[]);
        assert_eq!(
            Config::from_args_and_env(args(&["--config", "x"]), &none),
            Err(ConfigError::UnknownArgument("--config".to_owned()))
        );
        assert_eq!(
            Config::from_args_and_env(args(&["--data"]), &none),
            Err(ConfigError::MissingValue("--data".to_owned()))
        );
        assert_eq!(
            Config::from_args_and_env(args(&["--listen", "localhost"]), &none),
            Err(ConfigError::BadListen("localhost".to_owned()))
        );
    }
}
