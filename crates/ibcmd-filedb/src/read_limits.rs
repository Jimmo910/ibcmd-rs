//! Optional cumulative allocation/work budget for untrusted read-only inputs.
use crate::{Error, Result};

/// Cumulative limits for one reader, including table descriptors and repeated
/// CONFIG/CONFIGSAVE reads. max_records counts descriptors and live row records.
/// Object bytes cover descriptors, records and blob-object buffers; binary bytes
/// cover copied row values, including multiple references to the same blob.
#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    pub max_records: usize,
    pub max_binary_bytes: u64,
    pub max_object_bytes: u64,
}

pub(crate) struct ReadBudget {
    records: usize,
    binary_bytes: u64,
    object_bytes: u64,
    check: Box<dyn Fn() -> Result<()> + Send + Sync>,
}
impl ReadBudget {
    pub(crate) fn new(
        limits: ReadLimits,
        check: impl Fn() -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            records: limits.max_records,
            binary_bytes: limits.max_binary_bytes,
            object_bytes: limits.max_object_bytes,
            check: Box::new(check),
        }
    }
    pub(crate) fn check(&self) -> Result<()> {
        (self.check)()
    }
    pub(crate) fn object(&mut self, bytes: u64) -> Result<()> {
        self.check()?;
        self.object_bytes = self
            .object_bytes
            .checked_sub(bytes)
            .ok_or_else(|| Error::Refused("file database object materialization limit".into()))?;
        Ok(())
    }
    pub(crate) fn record(&mut self, bytes: u64) -> Result<()> {
        self.check()?;
        let records = self
            .records
            .checked_sub(1)
            .ok_or_else(|| Error::Refused("file database record limit".into()))?;
        let binary = self
            .binary_bytes
            .checked_sub(bytes)
            .ok_or_else(|| Error::Refused("file database binary materialization limit".into()))?;
        self.records = records;
        self.binary_bytes = binary;
        Ok(())
    }
}
