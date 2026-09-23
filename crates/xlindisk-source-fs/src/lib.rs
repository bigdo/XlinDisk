//! Filesystem source for [`xlindisk_core`].
//!
//! This crate exists from day one so that the core never learns what a path is.
//! iOS PhotoKit and Android MediaStore / SAF do not offer "an openable path",
//! and modelling `PathBuf` inside the core would force a rewrite the moment a
//! mobile adapter appears.
//!
//! **Status: not implemented.** The trait impls below are placeholders so the
//! dependency direction compiles today. The real work lands in:
//!
//! * PR3 — single-threaded `ReferenceWalker`, the correctness oracle;
//! * PR4 — `ParallelWalker` (work stealing, batches, backpressure), whose
//!   normalized output must equal the reference one on every fixture.

use xlindisk_core::error::{Error, ErrorCode};
use xlindisk_core::model::ids::{LocatorId, SourceId};
use xlindisk_core::model::observation::{Observation, ObservationValidation};
use xlindisk_core::runtime::CancellationToken;
use xlindisk_core::source::{
    ContentReader, ContentRequest, EntrySink, ScanOutcome, ScanRequest, Source, SourceCapabilities,
    FILESYSTEM_CAPABILITIES,
};

const NOT_IMPLEMENTED: &str = "xlindisk-source-fs is a skeleton until PR3";

/// Placeholder filesystem source.
pub struct FilesystemSource {
    id: SourceId,
}

impl FilesystemSource {
    pub fn new(id: SourceId) -> Self {
        Self { id }
    }
}

impl Source for FilesystemSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn capabilities(&self) -> SourceCapabilities {
        FILESYSTEM_CAPABILITIES
    }

    fn scan(
        &self,
        _request: &ScanRequest,
        _sink: &mut dyn EntrySink,
        _cancel: &CancellationToken,
    ) -> Result<ScanOutcome, Error> {
        Err(Error::with_detail(
            ErrorCode::UnsupportedOperation,
            NOT_IMPLEMENTED,
        ))
    }

    fn open_content(
        &self,
        _locator: LocatorId,
        _request: ContentRequest,
    ) -> Result<Box<dyn ContentReader + Send + '_>, Error> {
        Err(Error::with_detail(
            ErrorCode::UnsupportedOperation,
            NOT_IMPLEMENTED,
        ))
    }

    fn stat(&self, _locator: LocatorId) -> Result<Observation, Error> {
        Err(Error::with_detail(
            ErrorCode::UnsupportedOperation,
            NOT_IMPLEMENTED,
        ))
    }

    fn display_locator(&self, _locator: LocatorId) -> Result<String, Error> {
        Err(Error::with_detail(
            ErrorCode::UnsupportedOperation,
            NOT_IMPLEMENTED,
        ))
    }

    /// Canonical ordering key. PR3 implements the spec rule: raw filename bytes
    /// on Unix, lossless UTF-16 code units on Windows, lexicographic either way
    /// — no case folding, no Unicode normalization.
    fn sort_key(&self, _locator: LocatorId) -> Result<Vec<u8>, Error> {
        Err(Error::with_detail(
            ErrorCode::UnsupportedOperation,
            NOT_IMPLEMENTED,
        ))
    }

    /// The filesystem rule: object id + size + mtime + revision must agree.
    /// PhotoKit will compare asset id + resource version instead; the core never
    /// hard-codes the comparison, the source owns it.
    fn validate_observation(
        &self,
        _locator: LocatorId,
        before: &Observation,
        after: &Observation,
    ) -> Result<ObservationValidation, Error> {
        Ok(Observation::strict_compare(before, after))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xlindisk_core::source::EntryBatch;

    #[test]
    fn skeleton_reports_the_capabilities_it_will_have() {
        let source = FilesystemSource::new(SourceId(1));
        assert_eq!(source.id(), SourceId(1));
        assert!(source
            .capabilities()
            .contains(SourceCapabilities::STABLE_OBJECT_ID));
    }

    #[test]
    fn skeleton_refuses_to_scan() {
        let source = FilesystemSource::new(SourceId(1));
        let request = ScanRequest::new(SourceId(1), xlindisk_core::SessionId(1), vec![]);
        struct Noop;
        impl EntrySink for Noop {
            fn push(&mut self, _batch: EntryBatch<'_>) -> Result<(), Error> {
                Ok(())
            }
        }
        let err = source
            .scan(&request, &mut Noop, &CancellationToken::new())
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::UnsupportedOperation);
    }
}
