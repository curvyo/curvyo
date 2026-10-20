//! The cuttable check of one listed face with the per-computer cache (`specs/0075-font-library`
//! criterion 18, `adrs.md` decision 5), for the page's idle-time queue.

use curvyo_text_core::FaceHash;
use curvyo_ui_core::{CheckResult, FontChecks, cuttable_pieces};

/// What a check lookup gives the page: pieces, `-1` unreadable, `-2` not cached.
const NOT_CACHED: i64 = -2;

fn result_code(result: CheckResult) -> i64 {
    match result {
        CheckResult::Pieces(pieces) => i64::try_from(pieces).unwrap_or(i64::MAX),
        CheckResult::Unreadable => -1,
    }
}

/// The cuttable check of one face with the cache (criterion 18).
#[derive(Debug, Default)]
pub struct FontChecksGlue {
    checks: FontChecks,
}

impl FontChecksGlue {
    /// Reads the cache text (a bad one is an empty cache).
    pub fn load(&mut self, text: &str) {
        self.checks = FontChecks::parse(text);
    }

    /// The cache text.
    #[must_use]
    pub fn text(&self) -> String {
        self.checks.to_text()
    }

    /// Whether the system fonts were asked for on this computer.
    #[must_use]
    pub const fn system_requested(&self) -> bool {
        self.checks.system_requested()
    }

    /// Marks the system fonts as asked for; whether that is new.
    pub const fn request_system(&mut self) -> bool {
        self.checks.request_system()
    }

    /// The cached result of a listed face whose stamp still matches, else `-2`.
    #[must_use]
    pub fn by_stamp(&self, host_id: &str, index: u32, stamp: &str) -> i64 {
        self.checks
            .by_stamp(host_id, index, stamp)
            .map_or(NOT_CACHED, result_code)
    }

    /// Checks face `index` of `bytes` (by its hash from the cache when it is there) and keeps
    /// the result under the face's id and stamp; returns pieces or `-1`.
    pub fn check(&mut self, host_id: &str, index: u32, stamp: &str, bytes: &[u8]) -> i64 {
        let hash = FaceHash::of(bytes);
        let result = self.checks.by_hash(hash, index).unwrap_or_else(|| {
            cuttable_pieces(bytes, index).map_or(CheckResult::Unreadable, CheckResult::Pieces)
        });
        self.checks.record(host_id, index, stamp, hash, result);
        result_code(result)
    }
}
