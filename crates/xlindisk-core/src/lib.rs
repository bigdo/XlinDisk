//! # xlindisk-core
//!
//! Read-only storage analysis kernel.
//!
//! Applications describe *what* they want; the core decides *how* to execute it.
//! The core never assumes that a storage object is a filesystem path, which is
//! what keeps PhotoKit / MediaStore / SAF adapters possible without a rewrite.
//!
//! ## Phase
//!
//! This crate currently contains the **PR0 contract freeze** types and nothing
//! else: identity, observation, locator, run status, error, determinism, plan
//! node IO, the duplicate candidate budget and the result schema. Execution
//! (executor, scheduler, walker) arrives in later PRs and must not change these
//! types without a contract revision.
//!
//! ## Read-only
//!
//! v0.0.1 is a read-only engine. Nothing in this crate can unlink, move, trash
//! or mutate a storage object; deletion belongs to a separate Action /
//! Validation layer that re-verifies the object before doing anything.

pub mod determinism;
pub mod duplicate;
pub mod error;
pub mod model;
pub mod plan;
pub mod runtime;
pub mod source;

pub use determinism::{canonical_cmp, sort_canonical, GroupOrderKey};
pub use duplicate::{CandidateBudget, DuplicateStage, SpillPolicy, SpillTarget};
pub use error::{EntryError, Error, ErrorBudget, ErrorCode};
pub use model::entry::{Entry, EntryFlags, EntryKind};
pub use model::fingerprint::{
    Fingerprint, FingerprintSpec, VerificationMode, VerificationStatus, FINGERPRINT_ALGORITHM,
    FINGERPRINT_ALGORITHM_VERSION,
};
pub use model::ids::{
    EntryId, LocatorId, ObjectId, ObjectIdUnavailable, RunId, ScanId, SessionId, SourceId, VolumeId,
};
pub use model::observation::{ObservationClass, Record, Snapshot, Timestamp};
pub use model::store::{ChildIter, EntryStore, Session};
pub use plan::{Plan, PlanNode, PLAN_VERSION};
pub use runtime::{
    CancellationToken, ProgressEvent, ProgressStage, RunReport, RunStatus, TerminalEvent,
};
pub use source::{
    ContentReader, ContentRequest, EntryBatch, EntrySink, ScanOutcome, ScanRequest, Source,
    SourceCapabilities,
};

/// Version of the frozen plan IR. Bumped only through a contract revision.
pub const CORE_CONTRACT_VERSION: u32 = 1;
