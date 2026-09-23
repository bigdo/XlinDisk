//! RUNTIME contract: status, progress, cancellation and error accounting.
//!
//! Concurrency itself is owned by a single scheduler (ADR-006) and arrives with
//! the executor; what is frozen here is the *observable* behaviour every
//! front-end depends on.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::ErrorBudget;
use crate::model::ids::RunId;

/// End state of a run.
///
/// `Partial` means the run stopped early (cancelled, or budget/entry errors
/// forced it to stop) **and** still returned usable results. Callers must not
/// treat a partial run as a complete one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunStatus {
    Completed,
    /// Finished with results, but incompletely.
    Partial {
        errors: u64,
    },
    Cancelled,
    Failed,
}

impl RunStatus {
    pub fn is_usable(&self) -> bool {
        matches!(self, RunStatus::Completed | RunStatus::Partial { .. })
    }
}

/// Carried by the last progress event so consumers know the stream ended
/// instead of going quiet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalEvent {
    Completed,
    Partial { errors: u64 },
    Cancelled,
    Failed,
}

impl From<RunStatus> for TerminalEvent {
    fn from(status: RunStatus) -> Self {
        match status {
            RunStatus::Completed => TerminalEvent::Completed,
            RunStatus::Partial { errors } => TerminalEvent::Partial { errors },
            RunStatus::Cancelled => TerminalEvent::Cancelled,
            RunStatus::Failed => TerminalEvent::Failed,
        }
    }
}

/// Coarse pipeline phase, for progress reporting only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ProgressStage {
    Scan,
    Filter,
    Group,
    Fingerprint,
    Sort,
    Materialize,
}

/// One progress sample.
///
/// The channel is bounded and lossy: a slow UI, or an MCP client that stopped
/// reading, must never block a scanner. `sequence` therefore has gaps by design,
/// and consumers rely on `terminal` to know the run is over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProgressEvent {
    pub run_id: RunId,
    /// Monotonic within a run; gaps are normal (lossy channel).
    pub sequence: u64,
    pub stage: ProgressStage,
    pub entries_seen: u64,
    pub bytes_read: u64,
    pub candidates: u64,
    pub elapsed: Duration,
    /// `Some` only on the final event.
    pub terminal: Option<TerminalEvent>,
}

impl ProgressEvent {
    pub fn sample(run_id: RunId, sequence: u64, stage: ProgressStage) -> Self {
        Self {
            run_id,
            sequence,
            stage,
            entries_seen: 0,
            bytes_read: 0,
            candidates: 0,
            elapsed: Duration::ZERO,
            terminal: None,
        }
    }

    pub fn finish(mut self, status: RunStatus) -> Self {
        self.terminal = Some(TerminalEvent::from(status));
        self
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }
}

/// Cooperative cancellation.
///
/// Sources and the runtime check it at a reasonable granularity; it is never a
/// signal to kill a thread mid-syscall.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Summary handed back with the results.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub run_id: RunId,
    pub status: RunStatus,
    pub entries_seen: u64,
    pub bytes_read: u64,
    pub errors: ErrorBudget,
}

impl RunReport {
    pub fn new(run_id: RunId) -> Self {
        Self {
            run_id,
            status: RunStatus::Completed,
            entries_seen: 0,
            bytes_read: 0,
            errors: ErrorBudget::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{EntryError, ErrorCode};
    use crate::model::ids::RunId;

    #[test]
    fn cancelled_runs_can_still_be_usable() {
        // Frozen decision: cancellation returns the partial results it has.
        assert!(RunStatus::Partial { errors: 3 }.is_usable());
        assert!(!RunStatus::Cancelled.is_usable());
        assert!(!RunStatus::Failed.is_usable());
    }

    #[test]
    fn terminal_event_closes_the_progress_stream() {
        let event = ProgressEvent::sample(RunId(1), 0, ProgressStage::Scan)
            .finish(RunStatus::Partial { errors: 2 });
        assert!(event.is_terminal());
        assert_eq!(event.terminal, Some(TerminalEvent::Partial { errors: 2 }));
    }

    #[test]
    fn cancellation_is_visible_to_clones() {
        let token = CancellationToken::new();
        let worker = token.clone();
        assert!(!worker.is_cancelled());
        token.cancel();
        assert!(worker.is_cancelled());
    }

    #[test]
    fn report_tracks_error_totals() {
        let mut report = RunReport::new(RunId(7));
        report
            .errors
            .record(EntryError::new(ErrorCode::ReadFailed, None, None));
        assert_eq!(report.errors.total(), 1);
    }
}
