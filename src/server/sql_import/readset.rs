//! Physical-row arithmetic for SQL preflight; no SQL I/O or write authority.
//!
//! A builder cannot prove a coherent database snapshot: the acquisition adapter
//! must capture identity/schema and both tables in one bounded session. Likewise,
//! this local digest is not a cross-session SQL compare-and-swap. Publication must
//! compare the full expected rows under locks inside its own transaction.
use crate::mssql_live_continue::DatabaseIdentity;
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

pub type Check = Arc<dyn Fn() -> Result<()> + Send + Sync>;
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_parts: usize,
    pub max_part_bytes: u64,
    pub max_payload_bytes: u64,
    pub max_metadata_bytes: u64,
    pub max_name_units: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_parts: 100_000,
            max_part_bytes: 16 * 1024 * 1024,
            max_payload_bytes: 64 * 1024 * 1024,
            max_metadata_bytes: 8 * 1024 * 1024,
            max_name_units: 128,
        }
    }
}
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Key {
    pub file_name: String,
    pub part_no: u32,
}
#[derive(Clone, Eq, PartialEq)]
pub struct Part {
    pub data_size: u64,
    /// Complete canonical header image supplied by the measured SQL adapter.
    /// Includes creation/modified/attributes; no header is synthesized here.
    pub header: Vec<u8>,
    pub binary: Arc<Vec<u8>>,
}
impl fmt::Debug for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Part")
            .field("data_size", &self.data_size)
            .field("header_bytes", &self.header.len())
            .field("binary_bytes", &self.binary.len())
            .finish()
    }
}
/// Owned transport row; never deserialized from editor request parameters.
pub struct Input {
    pub key: Key,
    pub part: Part,
}
#[derive(Clone, Copy)]
pub enum Table {
    Config,
    ConfigSave,
}
type Parts = BTreeMap<Key, Part>;

pub struct Builder {
    identity: DatabaseIdentity,
    schema: [u8; 32],
    config: Parts,
    saved: Parts,
    limits: Limits,
    check: Check,
    count: usize,
    payload: u64,
    metadata: u64,
    failed: bool,
}
impl Builder {
    pub fn new(
        identity: DatabaseIdentity,
        schema: [u8; 32],
        limits: Limits,
        check: Check,
    ) -> Result<Self> {
        check()?;
        if identity.server.is_empty() || identity.database.is_empty() {
            bail!("SQL snapshot identity is incomplete");
        }
        for value in [
            &identity.database_guid,
            &identity.family_guid,
            &identity.recovery_fork,
        ] {
            uuid::Uuid::parse_str(value)
                .map_err(|_| anyhow::anyhow!("SQL snapshot identity GUID is invalid"))?;
        }
        let metadata = [
            &identity.server,
            &identity.database,
            &identity.database_guid,
            &identity.family_guid,
            &identity.recovery_fork,
        ]
        .into_iter()
        .try_fold(0u64, |total, value| total.checked_add(value.len() as u64))
        .ok_or_else(|| anyhow::anyhow!("SQL snapshot metadata size overflow"))?;
        if limits.max_parts == 0
            || limits.max_part_bytes == 0
            || limits.max_payload_bytes == 0
            || limits.max_metadata_bytes == 0
            || limits.max_name_units == 0
            || metadata > limits.max_metadata_bytes
        {
            bail!("SQL snapshot limits are invalid or exceeded");
        }
        Ok(Self {
            identity,
            schema,
            config: BTreeMap::new(),
            saved: BTreeMap::new(),
            limits,
            check,
            count: 0,
            payload: 0,
            metadata,
            failed: false,
        })
    }
    /// Admission errors poison the builder; partial reads cannot become a token.
    pub fn push(&mut self, table: Table, input: Input) -> Result<()> {
        if self.failed {
            bail!("SQL snapshot builder was invalidated");
        }
        let result = self.push_inner(table, input);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn push_inner(&mut self, table: Table, input: Input) -> Result<()> {
        (self.check)()?;
        let Input { key, part } = input;
        if key.file_name.is_empty()
            || key.file_name.contains('\0')
            || key
                .file_name
                .encode_utf16()
                .take(self.limits.max_name_units.saturating_add(1))
                .count()
                > self.limits.max_name_units
        {
            bail!("SQL snapshot row name is invalid");
        }
        if part.header.is_empty() {
            bail!("SQL snapshot header image is missing");
        }
        let bytes = part.binary.len() as u64;
        let count = self
            .count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("SQL part count overflow"))?;
        let payload = self
            .payload
            .checked_add(bytes)
            .ok_or_else(|| anyhow::anyhow!("SQL payload size overflow"))?;
        let metadata = self
            .metadata
            .checked_add(key.file_name.len() as u64)
            .and_then(|total| total.checked_add(part.header.len() as u64))
            .ok_or_else(|| anyhow::anyhow!("SQL metadata size overflow"))?;
        if count > self.limits.max_parts
            || bytes > self.limits.max_part_bytes
            || payload > self.limits.max_payload_bytes
            || metadata > self.limits.max_metadata_bytes
        {
            bail!("SQL snapshot resource limit exceeded");
        }
        let rows = match table {
            Table::Config => &mut self.config,
            Table::ConfigSave => &mut self.saved,
        };
        if rows.contains_key(&key) {
            bail!("SQL snapshot contains a duplicate physical part");
        }
        rows.insert(key, part);
        self.count = count;
        self.payload = payload;
        self.metadata = metadata;
        Ok(())
    }
    pub fn finish(self) -> Result<Readset> {
        if self.failed {
            bail!("SQL snapshot builder was invalidated");
        }
        validate_parts(&self.config, &self.check)?;
        validate_parts(&self.saved, &self.check)?;
        (self.check)()?;
        Ok(Readset {
            identity: self.identity,
            schema: self.schema,
            config: self.config,
            saved: self.saved,
            limits: self.limits,
            check: self.check,
        })
    }
}

fn validate_parts(rows: &Parts, check: &Check) -> Result<()> {
    let mut name: Option<&str> = None;
    let mut next = 0u32;
    let mut size = 0u64;
    let mut total = 0u64;
    for (key, part) in rows {
        check()?;
        if name != Some(key.file_name.as_str()) {
            if name.is_some() && total != size {
                bail!("SQL snapshot row payload size mismatch");
            }
            name = Some(&key.file_name);
            next = 0;
            size = part.data_size;
            total = 0;
        }
        if key.part_no != next || part.data_size != size {
            bail!("SQL snapshot has a part gap or inconsistent row size");
        }
        next = next
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("SQL row part overflow"))?;
        total = total
            .checked_add(part.binary.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("SQL row payload overflow"))?;
    }
    if name.is_some() && total != size {
        bail!("SQL snapshot row payload size mismatch");
    }
    Ok(())
}

pub struct Readset {
    identity: DatabaseIdentity,
    schema: [u8; 32],
    config: Parts,
    saved: Parts,
    limits: Limits,
    check: Check,
}
impl Readset {
    pub fn parts(&self, table: Table) -> impl Iterator<Item = (&Key, &Part)> {
        match table {
            Table::Config => &self.config,
            Table::ConfigSave => &self.saved,
        }
        .iter()
    }
    /// Local plan binding/fault oracle only; not SQL transaction authorization.
    pub fn digest(&self) -> Result<[u8; 32]> {
        let mut hash = Sha256::new();
        feed(&mut hash, b"ibcmd-sql-readset-v1", &self.check)?;
        for value in [
            &self.identity.server,
            &self.identity.database,
            &self.identity.database_guid,
            &self.identity.family_guid,
            &self.identity.recovery_fork,
        ] {
            feed(&mut hash, value.as_bytes(), &self.check)?;
        }
        feed(&mut hash, &self.schema, &self.check)?;
        for (label, rows) in [
            (b"CONFIG".as_slice(), &self.config),
            (b"CONFIGSAVE".as_slice(), &self.saved),
        ] {
            feed(&mut hash, label, &self.check)?;
            hash.update((rows.len() as u64).to_le_bytes());
            for (key, part) in rows {
                feed(&mut hash, key.file_name.as_bytes(), &self.check)?;
                hash.update(key.part_no.to_le_bytes());
                hash.update(part.data_size.to_le_bytes());
                feed(&mut hash, &part.header, &self.check)?;
                feed(&mut hash, &part.binary, &self.check)?;
            }
        }
        (self.check)()?;
        Ok(hash.finalize().into())
    }
    /// Pure row arithmetic. The transaction adapter still needs compiler proof,
    /// exact target identity/readset CAS and same-session publication.
    pub fn with_selected_stage(
        &self,
        replacements: impl IntoIterator<Item = Input>,
    ) -> Result<Self> {
        let mut selected = Builder::new(
            self.identity.clone(),
            self.schema,
            self.limits,
            self.check.clone(),
        )?;
        for input in replacements {
            selected.push(Table::ConfigSave, input)?;
        }
        let selected = selected.finish()?;
        let mut names = BTreeSet::new();
        for key in selected.saved.keys() {
            (self.check)()?;
            names.insert(key.file_name.clone());
        }
        if names.is_empty() {
            bail!("SQL replacement stage is empty");
        }
        let mut existing = BTreeSet::new();
        for key in self.config.keys().chain(self.saved.keys()) {
            (self.check)()?;
            existing.insert(key.file_name.as_str());
        }
        for name in &names {
            (self.check)()?;
            if !existing.contains(name.as_str()) {
                bail!("SQL partial import cannot add a new stored row");
            }
        }
        let mut merged = Builder::new(
            self.identity.clone(),
            self.schema,
            self.limits,
            self.check.clone(),
        )?;
        for (key, part) in &self.config {
            merged.push(
                Table::Config,
                Input {
                    key: key.clone(),
                    part: part.clone(),
                },
            )?;
        }
        for (key, part) in &self.saved {
            if !names.contains(key.file_name.as_str()) {
                merged.push(
                    Table::ConfigSave,
                    Input {
                        key: key.clone(),
                        part: part.clone(),
                    },
                )?;
            }
        }
        for (key, part) in selected.saved {
            merged.push(Table::ConfigSave, Input { key, part })?;
        }
        merged.finish()
    }
}
fn feed(hash: &mut Sha256, bytes: &[u8], check: &Check) -> Result<()> {
    hash.update((bytes.len() as u64).to_le_bytes());
    for chunk in bytes.chunks(64 * 1024) {
        check()?;
        hash.update(chunk);
    }
    check()?;
    Ok(())
}
