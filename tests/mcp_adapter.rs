//! Real SDK client launches the binary over supported MCP stdio; no native platform.
//! Current descriptor capability boundary is Unix-only.
#![cfg(unix)]
use rmcp::{ServiceExt, model::*, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Stdio};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/external")
}
fn corpus() -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("ibcmd-mcp-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("rows")).unwrap();
    fs::create_dir_all(root.join("source")).unwrap();
    let cf = fixture_root().join("home_page/one_column/input.cf");
    let limits =
        ibcmd_core::limits::ResourceLimits::for_input_bytes(fs::metadata(&cf).unwrap().len());
    let profile = ibcmd_core::artifact::StorageProfileId::parse("storage:cf-cli").unwrap();
    let archive =
        ibcmd_cf::archive::decode_packed_archive(fs::File::open(cf).unwrap(), limits, profile)
            .unwrap();
    for entry in archive.entries() {
        fs::write(
            root.join("rows")
                .join(format!("{}__part0.bin", entry.name())),
            entry.payload(),
        )
        .unwrap();
    }
    let settings = root.join("ibcmd-rs.toml");
    fs::write(
        &settings,
        "[[database]]\nname = \"demo\"\nplatform = \"8.3.27\"\nrows-dir = \"rows\"\n",
    )
    .unwrap();
    let policy = root.join("mcp.toml");
    fs::write(
        &policy,
        "[mcp]\ndatabase = \"demo\"\nsource-root = \"source\"\n",
    )
    .unwrap();
    (root, settings, policy)
}
async fn call(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    name: &str,
    args: Value,
) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}
fn value(result: &CallToolResult) -> Value {
    assert_eq!(result.is_error, Some(false), "{result:?}");
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}
#[tokio::test]
async fn real_mcp_client_reads_fixed_fixture_and_enforces_policy() {
    let (root, settings, policy) = corpus();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"));
    command
        .arg("mcp")
        .arg("--settings")
        .arg(&settings)
        .arg("--policy")
        .arg(&policy)
        .stderr(Stdio::piped());
    let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 6);
    assert!(
        tools
            .iter()
            .all(|t| t.input_schema.get("additionalProperties") == Some(&json!(false)))
    );
    let tree = value(&call(&client, "tree", json!({})).await);
    assert!(!tree["nodes"].as_array().unwrap().is_empty());
    // Select the configuration itself and prove export/read/compare use the real engine.
    let exported = value(&call(&client, "export", json!({"objects":["Configuration"]})).await);
    let files = exported["objects"][0]["files"].as_object().unwrap();
    assert!(!files.is_empty());
    let reference = root.join("cli-reference");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(["cf", "export"])
        .arg(fixture_root().join("home_page/one_column/input.cf"))
        .arg(&reference)
        .args(["--source-version", "2.20", "--overwrite"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (path, file) in files {
        assert_eq!(file["encoding"], "utf-8", "fixture should be text: {path}");
        assert_eq!(
            fs::read(reference.join(path)).unwrap(),
            file["content"].as_str().unwrap().as_bytes()
        );
        let target = root.join("source").join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, file["content"].as_str().unwrap()).unwrap();
    }
    let path = files
        .keys()
        .find(|path| path.as_str() == "Configuration.xml")
        .unwrap();
    let read = value(&call(&client, "read", json!({"path":path})).await);
    assert_eq!(read["file"], files[path]);
    for tool in ["status", "compare"] {
        let comparison = value(&call(&client, tool, json!({"objects":["Configuration"]})).await);
        assert_eq!(comparison["comparisonScope"], "selected-objects");
        assert_eq!(comparison["objects"][0]["status"], "same");
        for field in ["changed", "baseOnly", "localOnly"] {
            assert_eq!(comparison["objects"][0][field], json!([]));
        }
    }
    let extra = "Ext/LocalOnly.bsl";
    fs::create_dir_all(root.join("source/Ext")).unwrap();
    fs::write(root.join("source").join(extra), "extra source file").unwrap();
    let local_only = value(&call(&client, "status", json!({"objects":["Configuration"]})).await);
    assert_eq!(local_only["objects"][0]["status"], "modified");
    assert!(
        local_only["objects"][0]["localOnly"]
            .as_array()
            .unwrap()
            .contains(&json!(extra))
    );
    fs::rename(
        root.join("source").join(extra),
        root.join("local-only-evidence.bsl"),
    )
    .unwrap();
    fs::rename(
        root.join("source").join(path),
        root.join("missing-evidence.xml"),
    )
    .unwrap();
    let missing = value(&call(&client, "compare", json!({"objects":["Configuration"]})).await);
    assert!(
        missing["objects"][0]["baseOnly"]
            .as_array()
            .unwrap()
            .contains(&json!(path))
    );
    fs::rename(
        root.join("missing-evidence.xml"),
        root.join("source").join(path),
    )
    .unwrap();
    fs::write(root.join("source").join(path), "changed").unwrap();
    let changed = value(&call(&client, "compare", json!({"objects":["Configuration"]})).await);
    assert!(
        changed["objects"][0]["changed"]
            .as_array()
            .unwrap()
            .contains(&json!(path))
    );
    fs::create_dir_all(root.join("source/CommonModules/OnlyLocal/Ext")).unwrap();
    fs::write(root.join("source/CommonModules/OnlyLocal.xml"), "<local />").unwrap();
    fs::write(
        root.join("source/CommonModules/OnlyLocal/Ext/Module.bsl"),
        "local module",
    )
    .unwrap();
    let absent = value(
        &call(
            &client,
            "status",
            json!({"objects":["CommonModule.OnlyLocal"]}),
        )
        .await,
    );
    assert_eq!(absent["objects"][0]["status"], "notInBase");
    assert_eq!(
        absent["objects"][0]["localOnly"].as_array().unwrap().len(),
        2
    );

    #[cfg(unix)]
    {
        // Source and row symlinks are refused, even if they point inside the root.
        let original = root.join("source").join(path);
        let saved = root.join("source").join("saved.xml");
        fs::rename(&original, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &original).unwrap();
        assert_eq!(
            call(&client, "compare", json!({"objects":["Configuration"]}))
                .await
                .is_error,
            Some(true)
        );
        let row = fs::read_dir(root.join("rows"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let moved = root.join("saved-row.bin");
        fs::rename(&row, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &row).unwrap();
        assert_eq!(call(&client, "tree", json!({})).await.is_error, Some(true));
        // Move the symlink to evidence rather than deleting it, then restore input.
        fs::rename(&row, root.join("row-symlink-evidence")).unwrap();
        fs::rename(moved, row).unwrap();
    }
    #[cfg(unix)]
    {
        // Even a symlink to the SAME original root inode is a changed binding.
        fs::rename(root.join("source"), root.join("saved-source-root")).unwrap();
        std::os::unix::fs::symlink(root.join("saved-source-root"), root.join("source")).unwrap();
        assert_eq!(call(&client, "tree", json!({})).await.is_error, Some(true));
        fs::rename(
            root.join("source"),
            root.join("source-root-symlink-evidence"),
        )
        .unwrap();
        fs::rename(root.join("saved-source-root"), root.join("source")).unwrap();
    }
    for args in [
        json!({"server":"other"}),
        json!({"database":"other"}),
        json!({"settings":"elsewhere"}),
    ] {
        assert_eq!(call(&client, "tree", args).await.is_error, Some(true));
    }
    for path in [
        "../secret",
        "/etc/passwd",
        "Catalogs/../secret",
        "Catalogs\\secret",
        "Catalogs/x:secret",
    ] {
        assert_eq!(
            call(&client, "read", json!({"path":path})).await.is_error,
            Some(true)
        );
    }
    assert_eq!(
        call(&client, "import", json!({"objects":["Configuration"]}))
            .await
            .is_error,
        Some(true)
    );
    assert!(
        client
            .call_tool(CallToolRequestParams::new("unknown"))
            .await
            .is_err()
    );
    fs::write(
        &policy,
        "[mcp]\ndatabase = \"other\"\nsource-root = \"source\"\n",
    )
    .unwrap();
    assert_eq!(call(&client, "tree", json!({})).await.is_error, Some(true));
    // Restore original policy, then replace the root: identity remains pinned.
    fs::write(
        &policy,
        "[mcp]\ndatabase = \"demo\"\nsource-root = \"source\"\n",
    )
    .unwrap();
    fs::rename(root.join("rows"), root.join("old-rows")).unwrap();
    fs::create_dir(root.join("rows")).unwrap();
    assert_eq!(call(&client, "tree", json!({})).await.is_error, Some(true));
    client.cancel().await.unwrap();
    // Unique external TMPDIR evidence retained; no cleanup outside Trash.
}

#[derive(Clone)]
struct LegacyClient;
impl rmcp::ClientHandler for LegacyClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new("ibcmd-mcp-test", "1"),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
    }
}
#[tokio::test]
async fn legacy_initialize_and_modern_discover_are_supported() {
    use rmcp::service::{ClientLifecycleMode, ClientServiceExt};
    let (root, settings, policy) = corpus();
    let combined = root.join("combined.toml");
    fs::write(
        &combined,
        format!(
            "{}\n{}",
            fs::read_to_string(&settings).unwrap(),
            fs::read_to_string(&policy).unwrap()
        ),
    )
    .unwrap();
    for modern in [false, true] {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"));
        command.arg("mcp").arg("--settings");
        if modern {
            command.arg(&combined);
        } else {
            command.arg(&settings).arg("--policy").arg(&policy);
        }
        command.stderr(Stdio::piped());
        let mode = if modern {
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            }
        } else {
            ClientLifecycleMode::Initialize
        };
        let client = LegacyClient
            .serve_with_lifecycle(TokioChildProcess::new(command).unwrap(), mode)
            .await
            .unwrap();
        let info = client.peer_info().unwrap();
        assert_eq!(info.server_info.as_ref().unwrap().name, "ibcmd-rs");
        assert_eq!(
            info.protocol_version,
            if modern {
                ProtocolVersion::V_2026_07_28
            } else {
                ProtocolVersion::V_2025_11_25
            }
        );
        assert!(info.capabilities.tools.is_some());
        assert_eq!(client.list_all_tools().await.unwrap().len(), 6);
        // Send a genuine MCP cancellation notification through the SDK handle.
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new("export").with_arguments(
                json!({"objects":["Configuration"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        ));
        let pending = client
            .send_cancellable_request(request, rmcp::service::PeerRequestOptions::no_options())
            .await
            .unwrap();
        pending
            .cancel(Some("integration cancellation".into()))
            .await
            .unwrap();
        assert_eq!(client.list_all_tools().await.unwrap().len(), 6);
        client.cancel().await.unwrap();
    }
}

#[tokio::test]
async fn real_mcp_client_reads_disposable_filedb_without_mutation() {
    use std::io::Read;
    let root = std::env::temp_dir().join(format!("ibcmd-mcp-filedb-{}", uuid::Uuid::new_v4()));
    let db = root.join("db");
    fs::create_dir_all(&db).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/file-infobase");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(fixture.join("tiny.1CD.gz")).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let target = db.join("1Cv8.1CD");
    fs::write(&target, &bytes).unwrap();
    let settings = root.join("ibcmd-rs.toml");
    fs::write(&settings, format!("[[database]]\nname='demo'\nplatform='8.5.1'\ndb-path='db'\nagent-write=false\n[mcp]\ndatabase='demo'\nsource-root='{}'\n",fixture.join("tiny-xml").display())).unwrap();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"));
    command
        .arg("mcp")
        .arg("--settings")
        .arg(&settings)
        .stderr(Stdio::piped());
    let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    assert!(
        !value(&call(&client, "tree", json!({})).await)["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let module = "CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl";
    let result = value(&call(&client, "read", json!({"path":module})).await);
    let source = fs::read(fixture.join("tiny-xml").join(module)).unwrap();
    assert_eq!(
        result["file"]["content"].as_str().unwrap().as_bytes(),
        source
    );
    let exported = value(
        &call(
            &client,
            "export",
            json!({"objects":["CommonModule.ФайловаяБазаПроверка"]}),
        )
        .await,
    );
    assert_eq!(
        exported["objects"][0]["files"][module]["content"],
        result["file"]["content"]
    );
    let comparison = value(
        &call(
            &client,
            "compare",
            json!({"objects":["CommonModule.ФайловаяБазаПроверка"]}),
        )
        .await,
    );
    assert_eq!(comparison["objects"][0]["status"], "same");
    assert_eq!(comparison["objects"][0]["localOnly"], json!([]));
    assert_eq!(
        call(
            &client,
            "import",
            json!({"objects":["CommonModule.ФайловаяБазаПроверка"]})
        )
        .await
        .is_error,
        Some(true)
    );
    assert_eq!(fs::read(&target).unwrap(), bytes);
    assert!(!ibcmd_filedb::backup_file(&db).exists());
    // Optional native oracle is only in this external test orchestrator, never
    // the production Rust adapter. All native state remains under this test root.
    if let Some(native) = std::env::var_os("IBCMD_MCP_NATIVE") {
        let native_out = root.join("native-export");
        let output = std::process::Command::new(native)
            .arg("infobase")
            .arg(format!("--db-path={}", db.display()))
            .arg(format!("--data={}", root.join("native-data").display()))
            .arg(format!("--temp={}", root.join("native-temp").display()))
            .args(["config", "export"])
            .arg(&native_out)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "native export: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(native_out.join(module)).unwrap(), source);
    }
    // Replacement under the same path cannot silently reconnect this session.
    fs::rename(&target, root.join("old.1CD")).unwrap();
    fs::write(&target, &bytes).unwrap();
    assert_eq!(call(&client, "tree", json!({})).await.is_error, Some(true));
    client.cancel().await.unwrap();
}
