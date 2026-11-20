//! Why a command did not do its work, and the exit code that follows (UX notes sections 1, 5).

/// A refusal or failure; its text follows `curvyoctl: ` on stderr.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CtlError {
    /// The command line cannot be used (exit 2); the usage line of the command follows.
    #[error("{text}")]
    Usage {
        /// The refusal.
        text: String,
        /// The usage line printed after it, if the command is known.
        usage: Option<&'static str>,
    },
    /// The connector refused, did not answer, or a local file or store failed (exit 1).
    #[error("{text}")]
    Refused {
        /// The refusal.
        text: String,
        /// Labelled lines after it (the two keys of a key mismatch).
        details: Vec<(String, String)>,
    },
}

impl CtlError {
    /// A usage error without a usage line.
    #[must_use]
    pub fn usage(text: impl Into<String>) -> Self {
        Self::Usage {
            text: text.into(),
            usage: None,
        }
    }

    /// A refusal without detail lines.
    #[must_use]
    pub fn refused(text: impl Into<String>) -> Self {
        Self::Refused {
            text: text.into(),
            details: Vec::new(),
        }
    }

    /// A refusal with labelled detail lines.
    #[must_use]
    pub fn with_keys(text: impl Into<String>, details: Vec<(&str, String)>) -> Self {
        Self::Refused {
            text: text.into(),
            details: details
                .into_iter()
                .map(|(label, value)| (label.to_owned(), value))
                .collect(),
        }
    }

    /// 2 for usage, 1 for everything else.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage { .. } => 2,
            Self::Refused { .. } => 1,
        }
    }
}
