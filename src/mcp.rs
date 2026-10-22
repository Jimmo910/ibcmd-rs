//! Local MCP adapter. Transport/lifecycle belongs to rmcp; configuration operations
//! belong to the shared stored_objects engine. No native platform process is run.
use crate::{
    server::infobases::{self, Entry, Login, Session},
    settings::{ENV_CONFIG, Settings, SettingsSources},
    stored_objects,
};
use anyhow::{Context, Result, bail, ensure};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const MAX_RESULT: usize = 1024 * 1024;
const MAX_OBJECTS: usize = 32;
const TOOLS: &[(&str, &str)] = &[
    (
        "tree",
        "List children of the bound configuration tree. node is an optional engine tree id.",
    ),
    (
        "status",
        "Compare selected database objects with the fixed source root; reports per-file equality.",
    ),
    (
        "export",
        "Export selected objects as bounded UTF-8/base64 files in memory; does not write disk files.",
    ),
    (
        "import",
        "Stage selected objects from the fixed source root. Currently unavailable until shared staging integration; always fails closed.",
    ),
    (
        "read",
        "Read one configuration-relative file from the bound database, in UTF-8 or base64.",
    ),
    (
        "compare",
        "Compare selected database objects against the fixed source root, without publishing changes.",
    ),
];

#[derive(Debug, clap::Args)]
pub struct McpArgs {
    /// Explicit common ibcmd-rs.toml; no settings discovery.
    #[arg(long)]
    pub settings: PathBuf,
    /// Separate operator-owned TOML containing [mcp] database and source-root.
    #[arg(long)]
    pub policy: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Policy {
    database: String,
    source_root: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    mcp: Policy,
}

struct Bound {
    settings_path: PathBuf,
    settings_bytes: Vec<u8>,
    policy_path: PathBuf,
    policy_bytes: Vec<u8>,
    rows_handle: Option<fs::File>,
    settings: Settings,
    entry: Entry,
    root: PathBuf,
    root_handle: fs::File,
}

impl Bound {
    fn open(path: &Path, policy: &Path) -> Result<Self> {
        let settings_path = fs::canonicalize(path).context("MCP settings file unavailable")?;
        let settings_bytes = bounded_file(&settings_path, MAX_RESULT)?;
        ensure!(
            settings_bytes.len() <= MAX_RESULT,
            "MCP settings exceeds limit"
        );
        let policy_path = fs::canonicalize(policy)?;
        let policy_bytes = bounded_file(&policy_path, MAX_RESULT)?;
        ensure!(policy_bytes.len() <= MAX_RESULT, "MCP policy exceeds limit");
        let document: Document = toml::from_str(std::str::from_utf8(&policy_bytes)?)?;
        let settings = Settings::from_sources(&SettingsSources {
            env: &|name| (name == ENV_CONFIG).then(|| settings_path.display().to_string()),
            executable: None,
            current_dir: None,
            app_data: None,
            native_config: None,
        })?;
        ensure!(
            fs::read(&settings_path)? == settings_bytes,
            "settings changed during startup"
        );
        let entry = infobases::entries(&settings, &[])?
            .into_iter()
            .find(|entry| entry.id == document.mcp.database)
            .context("MCP database must name a configured fixed entry")?;
        ensure!(!entry.mask, "MCP database masks are forbidden");
        ensure!(
            entry.rows_dir.is_some() && entry.server.is_none(),
            "only offline rows supported until live identity integration"
        );
        // The common parser reads files itself. Bind its selected entry to the
        // exact initial bytes too, so a transient settings substitution cannot
        // redirect the session between the two reads.
        let raw: toml::Value = toml::from_str(std::str::from_utf8(&settings_bytes)?)?;
        let matches = raw
            .get("database")
            .and_then(toml::Value::as_array)
            .context("database table required")?
            .iter()
            .filter(|item| {
                item.get("name").and_then(toml::Value::as_str) == Some(entry.name.as_str())
            })
            .collect::<Vec<_>>();
        ensure!(matches.len() == 1, "database must have one exact binding");
        let original = matches[0];
        ensure!(original.get("server").is_none(), "offline target required");
        let original_rows = original
            .get("rows-dir")
            .and_then(toml::Value::as_str)
            .context("rows-dir required")?;
        let original_rows = settings_path.parent().unwrap().join(original_rows);
        ensure!(
            entry.rows_dir.as_deref() == Some(original_rows.as_path()),
            "settings identity changed during parse"
        );
        ensure!(
            original
                .get("platform")
                .and_then(toml::Value::as_str)
                .map(crate::platform::parse)
                .transpose()?
                .is_some_and(|platform| platform == entry.spec),
            "platform identity mismatch"
        );
        let requested = if document.mcp.source_root.is_absolute() {
            document.mcp.source_root
        } else {
            policy_path.parent().unwrap().join(document.mcp.source_root)
        };
        let root = fs::canonicalize(requested).context("MCP source root unavailable")?;
        ensure!(root.is_dir(), "MCP source root must be a directory");
        let root_handle = fs::File::open(&root)?;
        let rows_handle = entry.rows_dir.as_ref().map(fs::File::open).transpose()?;
        Ok(Self {
            settings_path,
            settings_bytes,
            policy_path,
            policy_bytes,
            rows_handle,
            settings,
            entry,
            root,
            root_handle,
        })
    }

    fn authorize(&self) -> Result<()> {
        ensure!(
            bounded_file(&self.settings_path, MAX_RESULT)? == self.settings_bytes,
            "MCP settings changed; restart required"
        );
        ensure!(
            bounded_file(&self.policy_path, MAX_RESULT)? == self.policy_bytes,
            "MCP policy changed; restart required"
        );
        // Pin root identity for this process. A rename/replacement requires a new session.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let (Some(handle), Some(path)) = (&self.rows_handle, &self.entry.rows_dir) {
                let pinned = handle.metadata()?;
                let current = fs::symlink_metadata(path)?;
                ensure!(
                    current.is_dir()
                        && (pinned.dev(), pinned.ino()) == (current.dev(), current.ino()),
                    "MCP target identity changed; restart required"
                );
            }
            let pinned = self.root_handle.metadata()?;
            let current = fs::metadata(&self.root)?;
            ensure!(
                (pinned.dev(), pinned.ino()) == (current.dev(), current.ino()),
                "MCP source root changed; restart required"
            );
        }
        ensure!(!self.entry.mask, "fixed database required");
        Ok(())
    }

    fn execute(&self, name: &str, args: Args) -> Result<Value> {
        self.authorize()?;
        crate::cancel::check()?;
        // No request can opt in to writing, or replace the target. The common
        // staging backend and its database-specific opt-in are an integration gate.
        if name == "import" {
            bail!(
                "write_denied: shared agent-authorized staging is not integrated; no changes made"
            );
        }
        ensure!(
            self.rows_handle.is_some(),
            "live target identity API not integrated"
        );
        let before = self.rows_fingerprint()?;
        let rows = self.snapshot_rows()?;
        // A fresh engine view on every call avoids retaining stale inventory after
        // another client stages/applies changes. Auth comes only from operator env.
        let mut session = Session::open(&self.entry, None, Login::default(), &self.settings)?;
        session.configuration = stored_objects::StoredConfiguration::from_rows_snapshot(
            session.configuration.source().clone(),
            session.entry.spec,
            rows,
        );
        let config = &mut session.configuration;
        let result = match name {
            "tree" => json!({"nodes": config.children(args.node.as_deref())?}),
            "read" => {
                let path = args.path.context("read requires path")?;
                validate_relative(&path)?;
                let (object, path) = config.read_file(&path)?;
                json!({"path":path, "version":object.version, "file": object.files.get(&path).map(|bytes| encoded(bytes)).transpose()?})
            }
            "export" | "compare" | "status" => {
                ensure!(
                    !args.objects.is_empty() && args.objects.len() <= MAX_OBJECTS,
                    "select 1..32 objects"
                );
                let objects = config.export(&args.objects, &|_, _, _| {})?;
                let mut used = 0usize;
                let mut output = Vec::new();
                for object in objects {
                    let mut files = BTreeMap::new();
                    for (path, bytes) in &object.files {
                        crate::cancel::check()?;
                        used = used.checked_add(bytes.len()).context("output limit")?;
                        ensure!(used <= MAX_RESULT / 2, "output_limit: select fewer objects");
                        let value = if name == "export" {
                            encoded(bytes)?
                        } else {
                            let local = self.read_local(path)?;
                            json!({"exists":local.is_some(), "same":local.as_deref() == Some(bytes.as_slice())})
                        };
                        files.insert(path.clone(), value);
                    }
                    output.push(json!({"object":object.object.full_name,"version":object.version,"files":files}));
                }
                json!({"objects":output, "comparisonScope":"exported-files-only"})
            }
            _ => bail!("unknown tool"),
        };
        self.authorize()?;
        ensure!(
            before == self.rows_fingerprint()?,
            "target_changed: restart or retry with a stable target"
        );
        crate::cancel::check()?;
        ensure!(
            serde_json::to_vec(&result)?.len() <= MAX_RESULT,
            "output_limit: select fewer objects"
        );
        Ok(result)
    }

    #[cfg(unix)]
    fn snapshot_rows(&self) -> Result<BTreeMap<String, Arc<Vec<u8>>>> {
        let handle = self.rows_handle.as_ref().context("offline rows required")?;
        let root = self
            .entry
            .rows_dir
            .as_ref()
            .context("offline rows required")?;
        let mut parts = BTreeMap::<String, BTreeMap<usize, Vec<u8>>>::new();
        let mut total = 0usize;
        for (index, entry) in fs::read_dir(root)?.enumerate() {
            ensure!(index < 100_000, "snapshot part count limit");
            crate::cancel::check()?;
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid row name"))?;
            let stem = name.strip_suffix(".bin").context("invalid row file")?;
            let (row, part) = stem.rsplit_once("__part").context("invalid row part")?;
            ensure!(
                !row.is_empty() && part.bytes().all(|c| c.is_ascii_digit()),
                "invalid row part"
            );
            let part: usize = part.parse()?;
            ensure!(part < 100_000, "row part limit");
            let bytes =
                read_confined(handle, &name, 128 * 1024 * 1024)?.context("row disappeared")?;
            total = total.checked_add(bytes.len()).context("snapshot limit")?;
            ensure!(
                total <= 128 * 1024 * 1024 && parts.len() < 100_000,
                "snapshot input limit"
            );
            ensure!(
                parts
                    .entry(row.to_string())
                    .or_default()
                    .insert(part, bytes)
                    .is_none(),
                "duplicate row part"
            );
        }
        parts
            .into_iter()
            .map(|(name, parts)| {
                let mut bytes = Vec::new();
                for (expected, (part, payload)) in parts.into_iter().enumerate() {
                    ensure!(part == expected, "missing row part");
                    bytes.extend(payload);
                }
                Ok((name, Arc::new(bytes)))
            })
            .collect()
    }
    #[cfg(not(unix))]
    fn snapshot_rows(&self) -> Result<BTreeMap<String, Arc<Vec<u8>>>> {
        bail!("secure snapshot reads currently require Unix")
    }

    fn rows_fingerprint(&self) -> Result<Option<Vec<u8>>> {
        let Some(root) = &self.entry.rows_dir else {
            return Ok(None);
        };
        // Offline inputs must be ordinary files under a stable root. Refuse
        // symlinks (including ancestor symlinks) and suppress results on changes.
        ensure!(
            fs::canonicalize(root)? == *root,
            "rows root must be canonical and contain no symlink aliases"
        );
        let mut digest = Sha256::new();
        let mut entries = fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        ensure!(entries.len() <= 100_000, "rows input limit");
        for entry in entries {
            let metadata = fs::symlink_metadata(entry.path())?;
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "rows must contain ordinary files only"
            );
            digest.update(entry.file_name().to_string_lossy().as_bytes());
            digest.update(metadata.len().to_le_bytes());
            digest.update(format!("{:?}", metadata.modified()?));
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                digest.update(metadata.ino().to_le_bytes());
            }
        }
        Ok(Some(digest.finalize().to_vec()))
    }

    #[cfg(unix)]
    fn read_local(&self, path: &str) -> Result<Option<Vec<u8>>> {
        read_confined(&self.root_handle, path, MAX_RESULT)
    }

    #[cfg(not(unix))]
    fn read_local(&self, _path: &str) -> Result<Option<Vec<u8>>> {
        bail!("secure source reads are currently supported on Unix only")
    }
}

#[cfg(unix)]
fn read_confined(root: &fs::File, path: &str, limit: usize) -> Result<Option<Vec<u8>>> {
    use rustix::fs::{Mode, OFlags, openat};
    validate_relative(path)?;
    let mut directory = root.try_clone()?;
    let components = Path::new(path).components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let last = index + 1 == components.len();
        let flags = OFlags::RDONLY
            | OFlags::NOFOLLOW
            | OFlags::CLOEXEC
            | OFlags::NONBLOCK
            | if last {
                OFlags::empty()
            } else {
                OFlags::DIRECTORY
            };
        let fd = match openat(&directory, component.as_os_str(), flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        directory = fs::File::from(fd);
    }
    ensure!(
        directory.metadata()?.is_file(),
        "input must be an ordinary file"
    );
    let mut bytes = Vec::new();
    directory.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "input limit");
    Ok(Some(bytes))
}

fn validate_relative(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty() && !path.contains('\\') && !path.contains(':') && path.len() <= 4096,
        "invalid relative path"
    );
    ensure!(
        Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
            && !path
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == ".."),
        "invalid relative path"
    );
    Ok(())
}
fn encoded(bytes: &[u8]) -> Result<Value> {
    ensure!(
        bytes.len() <= MAX_RESULT / 2,
        "output_limit: file too large"
    );
    Ok(match std::str::from_utf8(bytes) {
        Ok(text) => json!({"encoding":"utf-8", "content":text}),
        Err(_) => json!({"encoding":"base64", "content":crate::server::rpc::base64(bytes)}),
    })
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    #[serde(default)]
    objects: Vec<String>,
    path: Option<String>,
    node: Option<String>,
}

#[derive(Clone)]
struct Adapter(Arc<Bound>, Arc<Mutex<()>>, Arc<tokio::sync::Semaphore>);
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
fn definitions() -> Vec<Tool> {
    TOOLS.iter().map(|(name, description)| {
        let (properties, required) = match *name {
            "tree" => (json!({"node":{"type":"string", "maxLength":4096}}), json!([])),
            "read" => (json!({"path":{"type":"string", "maxLength":4096}}), json!(["path"])),
            _ => (json!({"objects":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","maxLength":512}}}), json!(["objects"])),
        };
        let mut tool = Tool::new(*name, *description, json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}).as_object().unwrap().clone());
        tool.annotations = Some(ToolAnnotations::new().read_only(*name != "import").destructive(*name == "import").open_world(false));
        tool
    }).collect()
}
impl ServerHandler for Adapter {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ibcmd-rs", env!("CARGO_PKG_VERSION")))
            .with_instructions("Fixed operator-bound read-only database tools. Import is unavailable pending shared agent-authorized staging. status/compare compare exported files only.")
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        definitions().into_iter().find(|tool| tool.name == name)
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: definitions(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        if !TOOLS.iter().any(|(name, _)| *name == request.name) {
            return Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                "unknown tool",
                None,
            ));
        }
        let raw = Value::Object(request.arguments.unwrap_or_default());
        let permitted = match request.name.as_ref() {
            "tree" => "node",
            "read" => "path",
            _ => "objects",
        };
        if raw
            .as_object()
            .is_some_and(|map| map.keys().any(|key| key != permitted))
        {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "invalid_arguments: parameter not allowed for this tool",
            )])
            .into());
        }
        if serde_json::to_vec(&raw).map_or(true, |b| b.len() > 32768) {
            return Ok(CallToolResult::error(vec![ContentBlock::text("input_limit")]).into());
        }
        let args: Args = match serde_json::from_value(raw) {
            Ok(args) => args,
            Err(_) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    "invalid_arguments: unknown or malformed parameters",
                )])
                .into());
            }
        };
        let permit = match self.2.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    "busy: retry after current operation",
                )])
                .into());
            }
        };
        if args.node.as_ref().is_some_and(|s| s.len() > 4096)
            || args.path.as_ref().is_some_and(|s| s.len() > 4096)
            || args.objects.len() > MAX_OBJECTS
            || args.objects.iter().any(|s| s.len() > 512)
        {
            return Ok(CallToolResult::error(vec![ContentBlock::text("input_limit")]).into());
        }
        let adapter = self.clone();
        let name = request.name.into_owned();
        let flag = Arc::new(AtomicBool::new(context.ct.is_cancelled()));
        let _guard = CancelOnDrop(flag.clone());
        let worker_flag = flag.clone();
        let mut work = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _serial = adapter
                .1
                .lock()
                .map_err(|_| anyhow::anyhow!("worker unavailable"))?;
            let _scope = crate::cancel::scope(worker_flag);
            adapter.0.execute(&name, args)
        });
        let outcome = tokio::select! {
            outcome = &mut work => outcome,
            _ = context.ct.cancelled() => {
                flag.store(true, Ordering::SeqCst);
                work.await
            }
        };
        if context.ct.is_cancelled() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "cancelled: no result published",
            )])
            .into());
        }
        let result = match outcome {
            Ok(Ok(value)) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            // Never forward backend diagnostics (SQL/user paths/secrets) to a model.
            Ok(Err(_)) => CallToolResult::error(vec![ContentBlock::text(
                "operation_denied_or_failed: policy, input, target stability or resource limit; import is unavailable",
            )]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text("worker_failed")]),
        };
        Ok(result.into())
    }
}

fn bounded_file(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    #[cfg(unix)]
    let file = {
        // openat supplies the portable platform flags; do not block on FIFOs.
        let fd = rustix::fs::open(
            path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::NONBLOCK
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )?;
        fs::File::from(fd)
    };
    #[cfg(not(unix))]
    let file = fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "policy/settings must be ordinary files"
    );
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "file input limit");
    Ok(bytes)
}

pub fn run(args: McpArgs) -> Result<()> {
    // Research override paths are not part of the MCP capability boundary.
    for name in [
        "IBCMD_RS_ROW_OVERRIDE_DIR",
        "IBCMD_RS_FORM_BODY_OVERRIDE_DIR",
        "IBCMD_RS_EXTENSION_MODE",
        "IBCMD_RS_EXTENSION_NORMALIZED_ROWS_OUT",
    ] {
        ensure!(
            std::env::var_os(name).is_none(),
            "MCP refuses research environment overrides"
        );
    }
    #[cfg(not(unix))]
    bail!("secure MCP input snapshots currently require Unix");
    let adapter = Adapter(
        Arc::new(Bound::open(&args.settings, &args.policy).map_err(|_| {
            anyhow::anyhow!("MCP startup rejected settings, policy, target or input capability")
        })?),
        Arc::default(),
        Arc::new(tokio::sync::Semaphore::new(1)),
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_stack_size(256 * 1024 * 1024)
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let (input, output) = rmcp::transport::stdio();
        let transport = rmcp::transport::async_rw::AsyncRwTransport::new(input, output)
            .with_max_line_length(65536);
        let service = adapter.serve(transport).await?;
        service.waiting().await?;
        Ok(())
    })
}
