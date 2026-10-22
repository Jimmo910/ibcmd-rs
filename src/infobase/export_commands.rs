//! Native export queries on the common SQL/filedb export pipeline.
use crate::{
    cli::InfobaseConfigExportArgs,
    mssql_dump::{DumpHooks, FileSink, dump_config_with},
};
use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Reads only the inventory, retaining the exact shared writer's bytes.
pub fn info(args: &InfobaseConfigExportArgs) -> Result<Vec<u8>> {
    let mut request = args.connection();
    request.need = super::PlatformNeed::Export {
        sqlcmd: args.sqlcmd.as_deref(),
        output_dir: &args.output_dir,
        overwrite: false,
        incremental: true,
    };
    let config = super::resolve_connection(request)?;
    if args.extension.is_some() {
        anyhow::bail!("export info/status of extensions is not supported yet");
    }
    let dump = super::dump_args(
        &config,
        args.sqlcmd.as_deref(),
        &args.db_pwd_env,
        PathBuf::from("ibcmd-rs-inventory"),
        Vec::new(),
        config.legacy_source_version()?,
    );
    crate::mssql_dump::read_config_dump_info(&dump)
}

/// Version comparison intentionally differs from editor byte-diff status.
/// A caller can serialize this without knowing the storage backend.
pub fn status(current: &[u8], base: &[u8], short: bool) -> Result<String> {
    use crate::native_export_status::ConfigDumpInfoSnapshot;
    Ok(ConfigDumpInfoSnapshot::parse(current)?
        .compare(&ConfigDumpInfoSnapshot::parse(base)?)
        .text(short))
}

/// Refuse linked path components and publish through a new sibling file.
/// The final rename replaces a destination entry without following it.
pub(crate) fn guarded_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::{fs, io::Write};
    let parent = path.parent().context("output file has no parent")?;
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(metadata) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = false;
                if linked || metadata.file_type().is_symlink() {
                    anyhow::bail!("linked output path is not allowed: {}", component.display());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".ibcmd-export-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staging)?;
    file.write_all(bytes)?;
    drop(file);
    fs::rename(&staging, path).with_context(|| format!("failed to publish {}", path.display()))
}

struct ObjectSink {
    root: PathBuf,
}
impl FileSink for ObjectSink {
    fn accept(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let relative = path
            .strip_prefix(&self.root)
            .context("object output escaped its root")?;
        if relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            anyhow::bail!("object output contains an unsafe path component");
        }
        guarded_write(path, bytes)
    }
}

pub fn objects(args: &InfobaseConfigExportArgs, names: &[String], recursive: bool) -> Result<()> {
    let mut request = args.connection();
    request.need = super::PlatformNeed::Export {
        sqlcmd: args.sqlcmd.as_deref(),
        output_dir: &args.output_dir,
        overwrite: false,
        incremental: true,
    };
    let config = super::resolve_connection(request)?;
    if args.extension.is_some() {
        anyhow::bail!("export objects of extensions is not supported yet");
    }
    if names.is_empty() {
        anyhow::bail!("no metadata objects selected");
    }
    let root = super::absolute_path(&args.output_dir)?;
    let mut dump = super::dump_args(
        &config,
        args.sqlcmd.as_deref(),
        &args.db_pwd_env,
        root.clone(),
        Vec::new(),
        config.legacy_source_version()?,
    );
    dump.objects = names.to_vec();
    dump_config_with(
        &dump,
        DumpHooks {
            sql: None,
            sink: Some(Arc::new(ObjectSink { root })),
            object_recursion: Some(recursive),
            native_versions: false,
        },
    )?;
    Ok(())
}
