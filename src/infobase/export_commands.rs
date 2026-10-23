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
    use std::io::Write;
    guarded_write_with(path, |file| file.write_all(bytes))
}

fn guarded_write_with(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<()> {
    use std::fs;
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
    let written = write(&mut file);
    drop(file); // Windows must close the owned staging handle before cleanup.
    let result = written.map_err(anyhow::Error::from).and_then(|()| {
        fs::rename(&staging, path).with_context(|| format!("failed to publish {}", path.display()))
    });
    if result.is_err()
        && let Err(cleanup) = fs::remove_file(&staging)
    {
        return result.with_context(|| {
            format!(
                "could not remove staging file {}: {cleanup}",
                staging.display()
            )
        });
    }
    result
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

#[cfg(test)]
mod publication_tests {
    use super::*;
    use std::{
        fs,
        io::{self, Write},
    };

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("export-write-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::canonicalize(root).unwrap()
    }

    #[test]
    fn partial_write_failure_removes_owned_staging_and_preserves_destination() {
        let root = root();
        let target = root.join("status.txt");
        fs::write(&target, b"original").unwrap();
        let error = guarded_write_with(&target, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains("injected write failure"));
        assert_eq!(fs::read(&target).unwrap(), b"original");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    }

    #[test]
    fn failed_rename_removes_owned_staging_and_preserves_directory() {
        let root = root();
        let target = root.join("directory");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"original").unwrap();
        let error = guarded_write(&target, b"replacement").unwrap_err();
        assert!(format!("{error:#}").contains("failed to publish"));
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"original");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    }
}
