//! Read the shared ConfigDumpInfo inventory without converting object bodies.
use super::{DumpHooks, FileSink, dump_config_with};
use crate::cli::MssqlDumpConfigArgs;
use anyhow::{Result, anyhow, bail};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

struct InventorySink(Mutex<Option<Vec<u8>>>);
impl FileSink for InventorySink {
    fn inventory_only(&self) -> bool {
        true
    }
    fn accept(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if path
            .file_name()
            .is_some_and(|name| name == "ConfigDumpInfo.xml")
        {
            *self
                .0
                .lock()
                .map_err(|_| anyhow!("inventory sink poisoned"))? = Some(bytes.to_vec());
        }
        Ok(())
    }
}

/// Reads ConfigDumpInfo using the export's view and exact XML writer. No files
/// are written. `main_configuration` selects completed ConfigSave over Config;
/// this does not change the editor's custom byte-diff or raw Config APIs.
pub fn read_config_dump_info(args: &MssqlDumpConfigArgs) -> Result<Vec<u8>> {
    read_config_dump_info_with_sql(args, None)
}

pub(crate) fn read_config_dump_info_with_sql(
    args: &MssqlDumpConfigArgs,
    sql: Option<crate::sql::SqlExec>,
) -> Result<Vec<u8>> {
    if args.runtime_journal.is_some()
        || !args.file_names.is_empty()
        || !args.file_name_lists.is_empty()
        || !args.objects.is_empty()
        || args.include_config_save
        || args.base.is_some()
        || args.sync
        || !args.extract_metadata_xml
        || !args.extract_module_text
    {
        bail!("ConfigDumpInfo requires an unfiltered read-only inventory without runtime_journal");
    }
    let sink = Arc::new(InventorySink(Mutex::new(None)));
    dump_config_with(
        args,
        DumpHooks {
            sql,
            sink: Some(sink.clone()),
            object_recursion: None,
        },
    )?;
    let bytes = sink
        .0
        .lock()
        .map_err(|_| anyhow!("inventory sink poisoned"))?
        .take();
    bytes.ok_or_else(|| anyhow!("configuration inventory was not emitted"))
}
