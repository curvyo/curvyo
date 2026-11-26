//! The error kinds of the online library protocols.

/// Why an address or a request is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RequestError {
    /// The address does not start with `https://` or has no host.
    #[error("the address must start with https://")]
    NotHttps,
    /// The address holds a user name, a query or a fragment, or is too long.
    #[error("the address holds more than a host and a path")]
    Shape,
}

/// Why the sources file is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourcesError {
    /// The text is not a sources file.
    #[error("the sources file is damaged")]
    Damaged,
    /// A newer Curvyo wrote it.
    #[error("the sources file was made by a newer Curvyo")]
    Newer,
}

/// Why a catalogue index is not used (ADR 0085 decisions 3 and 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogueError {
    /// Not a catalogue index: "No Git catalogue at this address."
    #[error("not a catalogue index")]
    Invalid,
    /// A newer `catalogue_format`: "Made for a newer Curvyo".
    #[error("made for a newer Curvyo")]
    Newer,
    /// Over the index limits.
    #[error("the index is too large")]
    TooLarge,
}

/// Why an Iconify answer is not used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IconifyError {
    /// Not the shape the endpoint answers: "No Iconify API at this address."
    #[error("not an Iconify answer")]
    Invalid,
    /// Over the 8 MiB answer limit.
    #[error("the answer is too large")]
    TooLarge,
}

/// Why a downloaded file does not match its catalogue entry (ADR 0085 decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    /// The file is larger than the limit or than the entry says.
    #[error("the file is larger than its entry")]
    TooLarge,
    /// The size differs from the entry's.
    #[error("the size does not match its entry")]
    Size,
    /// The SHA-256 differs from the entry's.
    #[error("the hash does not match its entry")]
    Hash,
}

/// What went wrong with one request of one source, by the table of ADR 0085 decision 6. The words
/// are the UI layer's; each failure is shown only on the source it hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "status", rename_all = "snake_case")]
pub enum Failure {
    /// The connection failed (DNS, connect, timeout, TLS): "Offline".
    Offline,
    /// HTTP 404 on the index or the Iconify collections.
    NotFound,
    /// Another HTTP status.
    Status(u16),
    /// The answer is not what the endpoint answers.
    Invalid,
    /// A newer `catalogue_format`.
    Newer,
    /// The file does not match its catalogue entry.
    Mismatch,
    /// Over a size limit.
    TooLarge,
    /// A redirect the guard refuses (another host, or not `https`).
    Redirect,
    /// The source has a token in the key store and it could not be read: nothing was sent
    /// (ADR 0090 decision 5, `specs/0157` criterion 12).
    TokenUnreadable,
}

impl From<CatalogueError> for Failure {
    fn from(error: CatalogueError) -> Self {
        match error {
            CatalogueError::Invalid => Self::Invalid,
            CatalogueError::Newer => Self::Newer,
            CatalogueError::TooLarge => Self::TooLarge,
        }
    }
}

impl From<IconifyError> for Failure {
    fn from(error: IconifyError) -> Self {
        match error {
            IconifyError::Invalid => Self::Invalid,
            IconifyError::TooLarge => Self::TooLarge,
        }
    }
}

impl From<VerifyError> for Failure {
    fn from(error: VerifyError) -> Self {
        match error {
            VerifyError::TooLarge => Self::TooLarge,
            VerifyError::Size | VerifyError::Hash => Self::Mismatch,
        }
    }
}

/// Why the shipped keyword file is not used (spec `0153` criterion 4, ADR 0087 decision 3); the
/// search then runs without keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KeywordsError {
    /// The gzip layer does not inflate.
    #[error("the keyword file does not inflate")]
    Inflate,
    /// Larger than 4 MiB once inflated.
    #[error("the keyword file is too large")]
    TooLarge,
    /// Not a keyword file, or another format.
    #[error("not a keyword file of format 1")]
    NotKeywords,
    /// A count, index, length, text or the name order is wrong.
    #[error("the keyword file is damaged")]
    Damaged,
}
