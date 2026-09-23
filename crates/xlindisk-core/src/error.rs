//! Stable error model.
//!
//! One enumerated code space shared by CLI, desktop, MCP and tests, so that no
//! front-end has to interpret strings. Discriminants are the wire format:
//! adding a code is fine, renumbering one is a contract break.

use std::collections::HashMap;

use crate::model::ids::{EntryId, LocatorId, SourceId};

/// Stable error codes.
///
/// The discriminant *is* the wire format. Adding a code is fine; renumbering one
/// is a contract break. A `message` may accompany a code but is always
/// human-readable only — no consumer parses it.
///
/// * `1xx` — fatal: the run cannot produce anything meaningful.
/// * `2xx` — entry-level: record it, keep scanning.
/// * `3xx` — control flow: cancellation.
/// * `4xx` — internal bug.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u16)]
pub enum ErrorCode {
    // --- fatal -------------------------------------------------------------
    RootNotFound = 100,
    SourceInitFailed = 101,
    ExecutorUnavailable = 102,
    PlanInvalid = 103,
    SourceUnavailable = 104,

    // --- entry-level -------------------------------------------------------
    PermissionDenied = 200,
    /// Deleted, renamed or moved out of scope; also plain "does not exist".
    NotFound = 201,
    /// The object still exists but permissions changed mid-run.
    PermissionChanged = 202,
    /// A re-observation disagreed with the previous one.
    ChangedDuringScan = 203,
    /// Generic IO failure while reading content or metadata.
    IoError = 204,
    /// Locator belongs to another session/source, or was never issued.
    LocatorInvalid = 205,
    /// The source session ended; the locator cannot be re-opened.
    LocatorExpired = 206,
    ObjectIdUnavailable = 207,
    /// The source cannot do what was asked (e.g. `open_content` on a socket).
    UnsupportedOperation = 208,
    /// Budget exhausted: the run degrades to `Partial`, it does not OOM.
    ResourceLimitExceeded = 209,

    // --- control flow ------------------------------------------------------
    Cancelled = 300,

    // --- internal ----------------------------------------------------------
    Internal = 400,
}

impl ErrorCode {
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// A fatal error aborts the whole run; an entry-level one never does.
    pub const fn is_fatal(self) -> bool {
        matches!(
            self,
            ErrorCode::RootNotFound
                | ErrorCode::SourceInitFailed
                | ErrorCode::ExecutorUnavailable
                | ErrorCode::PlanInvalid
                | ErrorCode::SourceUnavailable
                | ErrorCode::Internal
        )
    }

    pub const fn is_entry_level(self) -> bool {
        !self.is_fatal() && !matches!(self, ErrorCode::Cancelled)
    }

    pub const fn is_cancelled(self) -> bool {
        matches!(self, ErrorCode::Cancelled)
    }
}

/// An error with just enough context to act on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub source: Option<SourceId>,
    pub entry: Option<EntryId>,
    pub locator: Option<LocatorId>,
    /// Static explanation; never a formatted string built on the hot path.
    pub detail: Option<&'static str>,
}

impl Error {
    pub const fn new(code: ErrorCode) -> Self {
        Self {
            code,
            source: None,
            entry: None,
            locator: None,
            detail: None,
        }
    }

    pub const fn with_detail(code: ErrorCode, detail: &'static str) -> Self {
        Self {
            code,
            source: None,
            entry: None,
            locator: None,
            detail: Some(detail),
        }
    }

    pub const fn on_entry(mut self, entry: EntryId) -> Self {
        self.entry = Some(entry);
        self
    }

    pub const fn on_locator(mut self, locator: LocatorId) -> Self {
        self.locator = Some(locator);
        self
    }

    pub const fn on_source(mut self, source: SourceId) -> Self {
        self.source = Some(source);
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E{} {:?}", self.code.code(), self.code)?;
        if let Some(detail) = self.detail {
            write!(f, ": {detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

/// An entry-level failure retained for reporting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EntryError {
    pub code: ErrorCode,
    pub entry: Option<EntryId>,
    pub locator: Option<LocatorId>,
}

impl EntryError {
    pub fn new(code: ErrorCode, entry: Option<EntryId>, locator: Option<LocatorId>) -> Self {
        Self {
            code,
            entry,
            locator,
        }
    }
}

/// Entry-level errors are counted, not accumulated without limit.
///
/// One permission-denied directory on a network mount can produce millions of
/// errors. Past the cap we keep the first N detailed errors plus aggregated
/// counters per code — "PermissionDenied: 1,923,339" is a better answer than an
/// OOM, and it is what an MCP client can actually consume.
#[derive(Clone, Debug)]
pub struct ErrorBudget {
    /// How many individual entry errors to retain.
    pub max_retained: usize,
    retained: Vec<EntryError>,
    total: u64,
    /// Per-code totals, including errors that were not retained.
    counts: HashMap<ErrorCode, u64>,
}

impl ErrorBudget {
    pub fn new(max_retained: usize) -> Self {
        Self {
            max_retained,
            retained: Vec::with_capacity(max_retained.min(1_024)),
            total: 0,
            counts: HashMap::new(),
        }
    }

    pub fn record(&mut self, error: EntryError) {
        self.total += 1;
        *self.counts.entry(error.code).or_insert(0) += 1;
        if self.retained.len() < self.max_retained {
            self.retained.push(error);
        }
    }

    /// Total number of entry-level errors seen, retained or not.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Count for one code, retained or not.
    pub fn count_of(&self, code: ErrorCode) -> u64 {
        self.counts.get(&code).copied().unwrap_or(0)
    }

    /// All per-code counters.
    pub fn counts(&self) -> &HashMap<ErrorCode, u64> {
        &self.counts
    }

    /// Number of errors dropped by the cap.
    pub fn dropped(&self) -> u64 {
        self.total.saturating_sub(self.retained.len() as u64)
    }

    pub fn retained(&self) -> &[EntryError] {
        &self.retained
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

/// Default retention: first 1024 detailed errors, then counters only.
pub const MAX_DETAILED_ENTRY_ERRORS: usize = 1024;

impl Default for ErrorBudget {
    fn default() -> Self {
        Self::new(MAX_DETAILED_ENTRY_ERRORS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::EntryId;

    #[test]
    fn entry_level_errors_never_abort_a_run() {
        assert!(!ErrorCode::PermissionDenied.is_fatal());
        assert!(!ErrorCode::NotFound.is_fatal());
        assert!(!ErrorCode::IoError.is_fatal());
        assert!(!ErrorCode::ResourceLimitExceeded.is_fatal());
        assert!(ErrorCode::RootNotFound.is_fatal());
        assert!(ErrorCode::SourceUnavailable.is_fatal());
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(ErrorCode::RootNotFound.code(), 100);
        assert_eq!(ErrorCode::PermissionDenied.code(), 200);
        assert_eq!(ErrorCode::ResourceLimitExceeded.code(), 209);
        assert_eq!(ErrorCode::Cancelled.code(), 300);
        assert_eq!(ErrorCode::Internal.code(), 400);
    }

    #[test]
    fn budget_counts_beyond_the_cap() {
        let mut budget = ErrorBudget::new(3);
        for i in 0..10u32 {
            budget.record(EntryError::new(
                ErrorCode::PermissionDenied,
                Some(EntryId(i)),
                None,
            ));
        }
        assert_eq!(budget.total(), 10);
        assert_eq!(budget.retained().len(), 3);
        assert_eq!(budget.dropped(), 7);
        assert_eq!(budget.count_of(ErrorCode::PermissionDenied), 10);
        assert_eq!(budget.count_of(ErrorCode::NotFound), 0);
    }

    #[test]
    fn default_retention_is_1024() {
        let budget = ErrorBudget::default();
        assert_eq!(budget.max_retained, 1024);
    }
}
