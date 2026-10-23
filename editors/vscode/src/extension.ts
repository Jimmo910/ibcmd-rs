import * as vscode from "vscode";
import * as path from "node:path";
import { RpcClient, RpcError } from "./rpc";
import { runImport, ImportValidationError, importErrorMessage } from "./import";
import { Database, editConfig, idOf, snapshot, saveConfig } from "./config";
import { Base, Metadata, Generation, pendingText } from "./status";

import { ObjectView } from "./object-view";

export function secretKey(folder: string, id: string, user: string): string {
  return `ibcmd:${JSON.stringify([folder, id, user])}`;
}
export async function activate(context: vscode.ExtensionContext) {
  if (!vscode.workspace.isTrusted) return;
  const status = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    40,
  );
  status.command = "ibcmd.select";
  status.text = "$(database) ibcmd-rs";
  status.show();
  context.subscriptions.push(status);
  const objects = new ObjectView(context);
  let client: RpcClient | undefined;
  let folder: vscode.WorkspaceFolder | undefined;
  let selected: Base | undefined;
  let activeSettings: string | undefined;
  const generation = new Generation();
  const config = (f: vscode.WorkspaceFolder) =>
    vscode.workspace.getConfiguration("ibcmd", f.uri);
  const file = (f: vscode.WorkspaceFolder) =>
    path.join(f.uri.fsPath, "ibcmd-rs.toml");
  const metadata = (f: vscode.WorkspaceFolder): Record<string, Metadata> =>
    config(f).get("connectionMetadata", {});
  const bound = (f: vscode.WorkspaceFolder): string[] =>
    config(f).get("connections", []);
  const safeError = (e: unknown) =>
    e instanceof RpcError
      ? `ibcmd-rs request failed (${e.code}). Check the executable version and connection settings.`
      : "ibcmd-rs operation failed. Check the executable, settings and connection; reload to retry.";
  async function chooseFolder() {
    const folders =
      vscode.workspace.workspaceFolders?.filter(
        (f) => f.uri.scheme === "file",
      ) ?? [];
    if (folders.length === 0) {
      void vscode.window.showInformationMessage(
        "Open a trusted local workspace folder first.",
      );
      return;
    }
    return folders.length === 1
      ? folders[0]
      : vscode.window.showWorkspaceFolderPick();
  }
  async function start(f: vscode.WorkspaceFolder, onCrash = () => {}) {
    // Executable is machine-scoped: workspace content cannot choose a program to execute.
    const rpc = new RpcClient(
      config(f).get<string>("executable", "ibcmd-rs"),
      f.uri.fsPath,
      onCrash,
    );
    try {
      await rpc.request("initialize", {
        protocolVersion: "1.2",
        clientInfo: { name: "ibcmd-rs-vscode", version: "0.1.0" },
        workspaceFolder: f.uri.fsPath,
      });
      return rpc;
    } catch (e) {
      await rpc.dispose();
      throw e;
    }
  }
  async function list(f: vscode.WorkspaceFolder): Promise<Base[]> {
    const rpc = await start(f);
    try {
      return (await rpc.request("infobases/list")).infobases;
    } finally {
      await rpc.dispose();
    }
  }
  const label = (f: vscode.WorkspaceFolder, b: Base) =>
    metadata(f)[b.id]?.label || b.name;
  async function pick(
    f: vscode.WorkspaceFolder,
    bases: Base[],
    placeHolder: string,
  ) {
    return (
      await vscode.window.showQuickPick(
        bases.map((base) => ({
          label: label(f, base),
          description: `${base.id} · ${base.platform}`,
          base,
        })),
        { placeHolder },
      )
    )?.base;
  }
  async function switchTo(f: vscode.WorkspaceFolder, b: Base) {
    objects.clear();
    const stamp = generation.next();
    const previous = client;
    client = undefined;
    folder = f;
    selected = b;
    activeSettings = undefined;
    status.text = `$(sync~spin) ${label(f, b)} · ${b.platform} · connecting`;
    await previous?.dispose();
    if (!generation.current(stamp)) return;
    let rpc: RpcClient | undefined;
    try {
      const settings = await snapshot(file(f));
      if (!generation.current(stamp)) return;
      activeSettings = settings;
      const m = metadata(f)[b.id] ?? {};
      const params: Record<string, string> = { infobase: b.id };
      let promptedSecret: { key: string; password: string } | undefined;
      if (b.mask) throw new Error("Bind a fixed database instead of a mask");
      if (b.source === "mssql") {
        if (!m.auth) {
          void vscode.window.showInformationMessage(
            "Choose Windows or SQL authentication with Edit Connection before connecting.",
          );
          throw new Error("Authentication is required");
        }
        params.auth = m.auth;
        if (m.auth === "sql") {
          if (!m.user) throw new Error("SQL user is required");
          params.user = m.user;
          const key = secretKey(f.uri.toString(), b.id, m.user);
          let password = await context.secrets.get(key);
          if (!generation.current(stamp)) return;
          if (password === undefined) {
            password = await vscode.window.showInputBox({
              prompt: `SQL password for ${label(f, b)} (stored in SecretStorage)`,
              password: true,
              ignoreFocusOut: true,
            });
            if (password === undefined) throw new Error("Cancelled");
            promptedSecret = { key, password };
          }
          params.password = password;
        }
      }
      if (!generation.current(stamp)) return;
      rpc = await start(f, () => {
        if (generation.current(stamp)) {
          // Report while this selection owns the failure, before invalidating pending work.
          void vscode.window.showErrorMessage(
            safeError(new Error("Connection stopped")),
          );
          generation.next();
          objects.clear();
          status.text = `$(warning) ${label(f, b)} · ${b.platform} · disconnected · pending unknown`;
        }
      });
      if (!generation.current(stamp)) {
        await rpc.dispose();
        return;
      }
      client = rpc;
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: `Connecting to ${label(f, b)}`,
          cancellable: true,
        },
        async (_, token) => {
          const abort = new AbortController();
          if (token.isCancellationRequested) abort.abort();
          const subscription = token.onCancellationRequested(() =>
            abort.abort(),
          );
          try {
            const connected = await rpc!.request(
              "infobases/connect",
              params,
              abort.signal,
            );
            if (abort.signal.aborted) throw new Error("Cancelled");
            if (!generation.current(stamp)) return;
            // A new password is committed only after this selection connected successfully.
            // Cached passwords keep their existing behavior, including on login failure.
            if (promptedSecret) {
              await context.secrets.store(
                promptedSecret.key,
                promptedSecret.password,
              );
              if (abort.signal.aborted) throw new Error("Cancelled");
              if (!generation.current(stamp)) return;
            }
            if (typeof connected?.platform === "string")
              b = { ...b, platform: connected.platform };
            selected = b;
            let pending = "pending unknown";
            try {
              pending = pendingText(
                await rpc!.request(
                  "config/pending",
                  { infobase: b.id },
                  abort.signal,
                ),
              );
            } catch {
              if (abort.signal.aborted) throw new Error("Cancelled");
            }
            if (generation.current(stamp)) {
              status.text = `$(database) ${label(f, b)} · ${b.platform} · ${pending}`;
              status.tooltip = `${f.name}: ${b.id}. Click to switch; Reload Connections refreshes pending state.`;
              await context.workspaceState.update(
                `selected:${f.uri.toString()}`,
                b.id,
              );
            }
          } finally {
            subscription.dispose();
          }
        },
      );
    } catch (e) {
      if (generation.current(stamp)) {
        status.text = `$(warning) ${label(f, b)} · ${b.platform} · disconnected · pending unknown`;
        void vscode.window.showErrorMessage(safeError(e));
      }
      if (client === rpc) client = undefined;
      await rpc?.dispose();
    }
  }
  async function importFiles(
    uri?: vscode.Uri | { object: string; uri: vscode.Uri },
    selectedUris?: (vscode.Uri | { object: string; uri: vscode.Uri })[],
    changed = false,
  ) {
    const rpc = client,
      f = folder,
      base = selected;
    if (!rpc || !f || !base)
      throw new ImportValidationError("Select a workspace connection first");
    if (base.source !== "file-db")
      throw new ImportValidationError(
        "Safe editor import currently requires a file-db connection",
      );
    const stamp = generation.capture(),
      settings = activeSettings;
    const current = async () => {
      if (
        !generation.current(stamp) ||
        client !== rpc ||
        folder !== f ||
        selected !== base ||
        settings === undefined
      )
        return false;
      const actual = await snapshot(file(f));
      return (
        generation.current(stamp) &&
        client === rpc &&
        folder === f &&
        selected === base &&
        actual === settings &&
        !vscode.workspace.textDocuments.some(
          (d) =>
            d.isDirty &&
            vscode.workspace.getWorkspaceFolder(d.uri)?.uri.toString() ===
              f.uri.toString(),
        )
      );
    };
    const uris = selectedUris?.length
      ? selectedUris
      : uri
        ? [uri]
        : !changed && vscode.window.activeTextEditor
          ? [vscode.window.activeTextEditor.document.uri]
          : [];
    const sourceRoot = path.resolve(
      f.uri.fsPath,
      config(f).get<string>("sourceDirectory", "."),
    );
    const rootRelative = path.relative(f.uri.fsPath, sourceRoot);
    if (
      rootRelative === ".." ||
      rootRelative.startsWith(".." + path.sep) ||
      path.isAbsolute(rootRelative)
    )
      throw new ImportValidationError(
        "Source directory must be inside the connected workspace",
      );
    if (uris.length)
      for (const input of uris) {
        const u = "object" in input ? input.uri : input;
        const rel = path.relative(sourceRoot, u.fsPath);
        if (
          u.scheme !== "file" ||
          rel === ".." ||
          rel.startsWith(".." + path.sep) ||
          path.isAbsolute(rel)
        )
          throw new ImportValidationError(
            "Select objects/files inside the connected source tree",
          );
      }
    const objects =
      !changed && uris.length && uris.every((u) => "object" in u)
        ? uris.map((u) => (u as { object: string }).object)
        : undefined;
    const files =
      changed || objects
        ? undefined
        : uris.map((input) => {
            const u = "object" in input ? input.uri : input;
            const relative = path
              .relative(sourceRoot, u.fsPath)
              .replace(/\\/g, "/");
            if (
              u.scheme !== "file" ||
              relative === ".." ||
              relative.startsWith("../") ||
              path.isAbsolute(relative)
            )
              throw new ImportValidationError(
                "Select files inside the connected workspace source tree",
              );
            return relative;
          });
    if (!changed && !files?.length && !objects?.length)
      throw new ImportValidationError("Select files to import");
    if (
      vscode.workspace.textDocuments.some(
        (d) =>
          d.isDirty &&
          vscode.workspace.getWorkspaceFolder(d.uri)?.uri.toString() ===
            f.uri.toString(),
      )
    )
      throw new ImportValidationError(
        "Save changed workspace files before import",
      );
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "ibcmd-rs: проверка и загрузка",
        cancellable: true,
      },
      async (_, token) => {
        const abort = new AbortController();
        const subscription = token.onCancellationRequested(() => abort.abort());
        if (token.isCancellationRequested) abort.abort();
        try {
          const result = await runImport(
            rpc,
            {
              infobase: base.id,
              sourceDir: sourceRoot,
              files,
              objects,
              changed,
            },
            {
              current,
              confirm: async (message) =>
                (await vscode.window.showWarningMessage(
                  message,
                  { modal: true },
                  "Загрузить",
                )) === "Загрузить",
            },
            abort.signal,
          );
          if (result)
            void vscode.window.showInformationMessage(
              "Выбранные изменения загружены в CONFIGSAVE. Примените их отдельной командой платформы.",
            );
        } catch (error) {
          void vscode.window.showErrorMessage(importErrorMessage(error));
        } finally {
          subscription.dispose();
        }
      },
    );
  }
  const commands: Record<string, (...args: any[]) => Promise<void>> = {
    "ibcmd.importSelected": async (
      uri?: vscode.Uri | { object: string; uri: vscode.Uri },
      uris?: (vscode.Uri | { object: string; uri: vscode.Uri })[],
    ) => importFiles(uri, uris),
    "ibcmd.importChanged": async (uri?: vscode.Uri, uris?: vscode.Uri[]) =>
      importFiles(uri, uris, true),
    "ibcmd.objects.refresh": async () => {
      if (client && folder && selected)
        await objects.refresh(client, folder, selected.id);
      else objects.clear("Выберите подключение перед обновлением");
    },
    "ibcmd.objects.changedOnly": async () => {
      objects.toggle();
    },
    "ibcmd.select": async () => {
      const f = await chooseFolder();
      if (!f) return;
      const bases = await list(f);
      const ids = bound(f);
      const b = await pick(
        f,
        bases.filter((b) => ids.includes(b.id)),
        "Select a bound connection (use Bind Workspace Connections to add more)",
      );
      if (b) await switchTo(f, b);
    },
    "ibcmd.bind": async () => {
      const f = await chooseFolder();
      if (!f) return;
      const bases = (await list(f)).filter((b) => !b.mask);
      const ids = bound(f);
      const picked = await vscode.window.showQuickPick(
        bases.map((base) => ({
          label: label(f, base),
          description: base.id,
          picked: ids.includes(base.id),
          base,
        })),
        {
          canPickMany: true,
          placeHolder: "Connections for this workspace folder",
        },
      );
      if (picked) {
        await config(f).update(
          "connections",
          picked.map((p) => p.base.id),
          vscode.ConfigurationTarget.WorkspaceFolder,
        );
        await invalidate();
      }
    },
    "ibcmd.add": async () => {
      await edit();
    },
    "ibcmd.edit": async () => {
      await edit(true);
    },
    "ibcmd.refresh": async () => {
      if (folder && selected) {
        const f = folder;
        const id = selected.id;
        const stamp = await invalidate();
        const current = () =>
          generation.current(stamp) && folder === f && selected?.id === id;
        if (!current()) return;
        let bases: Base[];
        try {
          bases = await list(f);
        } catch (error) {
          if (current()) throw error;
          return;
        }
        if (!current()) return;
        const b = bases.find((b) => b.id === id);
        if (b && bound(f).includes(id)) {
          await switchTo(f, b);
          if (folder === f && selected?.id === id && client)
            await objects.refresh(client, f, id);
        }
      } else await commands["ibcmd.select"]();
    },
    "ibcmd.forgetPassword": async () => {
      const f = await chooseFolder();
      if (!f) return;
      const b = await pick(f, await list(f), "Forget saved SQL password");
      if (!b) return;
      const m = metadata(f)[b.id];
      if (m?.user)
        await context.secrets.delete(secretKey(f.uri.toString(), b.id, m.user));
      await invalidate();
    },
  };
  async function invalidate() {
    objects.clear();
    const stamp = generation.next();
    const old = client;
    client = undefined;
    status.text = "$(database) ibcmd-rs · reload/select connection";
    await old?.dispose();
    return stamp;
  }
  async function edit(existing = false) {
    const f = await chooseFolder();
    if (!f) return;
    const bases = await list(f);
    const old = existing
      ? await pick(
          f,
          bases.filter((b) => b.source === "mssql" || b.source === "rows-dir"),
          "Edit SQL/offline connection (inherited entries are copied into workspace TOML)",
        )
      : undefined;
    if (existing && !old) return;
    const expected = await snapshot(file(f));
    if (
      vscode.workspace.textDocuments.some(
        (d) => d.uri.fsPath === file(f) && d.isDirty,
      )
    )
      throw new Error("Save TOML before editing");
    const previous = old ? (metadata(f)[old.id] ?? {}) : {};
    async function input(prompt: string, value = "", required = true) {
      return vscode.window.showInputBox({
        prompt,
        value,
        ignoreFocusOut: true,
        validateInput: (v) => (!required || v.trim() ? undefined : "Required"),
      });
    }
    const friendly = await input(
      "Friendly connection name",
      previous.label || old?.name,
    );
    if (friendly === undefined) return;
    const server = await input(
      "SQL Server (host or host\\instance; blank inherits settings)",
      old ? (old.server ?? "") : "localhost",
      false,
    );
    if (server === undefined) return;
    const name = await input(
      "Database name (one database; no wildcard)",
      old?.name,
    );
    if (name === undefined) return;
    const platform = await input(
      "1C platform version",
      old?.platform || "8.3.27",
    );
    if (platform === undefined) return;
    const auth = await vscode.window.showQuickPick(
      [
        { label: "Windows authentication", value: "windows" as const },
        { label: "SQL authentication", value: "sql" as const },
      ],
      {
        placeHolder:
          "Authentication (Windows integrated login requires Windows)",
      },
    );
    if (!auth) return;
    let user: string | undefined;
    let password: string | undefined;
    if (auth.value === "sql") {
      user = await input("SQL user", previous.user);
      if (user === undefined) return;
      user = user.trim();
      password = await vscode.window.showInputBox({
        prompt:
          "SQL password (SecretStorage only; cancel leaves connection unchanged)",
        password: true,
        ignoreFocusOut: true,
      });
      if (password === undefined) return;
    }
    const local = readLocal(expected, old?.id);
    if (old?.source === "rows-dir" && !local && !old.rowsDir)
      throw new Error(
        "Inherited rows directory is missing; reload before editing",
      );
    const database: Database = {
      ...(server.trim() ? { server: server.trim() } : {}),
      ...(old?.source === "rows-dir" && !local
        ? { "rows-dir": old.rowsDir }
        : {}),
      name: name.trim(),
      platform: platform.trim(),
    };
    const next = editConfig(expected, local ? old?.id : undefined, database);
    // Validate the exact platform with the real server before writing the file. No login/query.
    const validator = new RpcClient(
      config(f).get("executable", "ibcmd-rs"),
      f.uri.fsPath,
    );
    try {
      await validator.request("initialize", {
        protocolVersion: "1.2",
        workspaceFolder: f.uri.fsPath,
        infobases: [
          {
            name: database.name,
            server: database.server,
            platform: database.platform,
            ...(database["rows-dir"] ? { rowsDir: database["rows-dir"] } : {}),
          },
        ],
      });
    } finally {
      await validator.dispose();
    }
    // Prompts/validation may have taken time: refuse unsaved in-editor changes as well as disk edits.
    if (
      vscode.workspace.textDocuments.some(
        (d) => d.uri.fsPath === file(f) && d.isDirty,
      )
    )
      throw new Error("Save TOML before editing");
    await saveConfig(file(f), expected, next);
    const id = idOf(database);
    const all = { ...metadata(f) };
    all[id] = {
      label: friendly.trim(),
      auth: auth.value,
      ...(user ? { user } : {}),
    };
    await config(f).update(
      "connectionMetadata",
      all,
      vscode.ConfigurationTarget.WorkspaceFolder,
    );
    const ids = bound(f).map((v) => (v === old?.id ? id : v));
    if (!ids.includes(id)) ids.push(id);
    await config(f).update(
      "connections",
      [...new Set(ids)],
      vscode.ConfigurationTarget.WorkspaceFolder,
    );
    if (user && password !== undefined)
      await context.secrets.store(
        secretKey(f.uri.toString(), id, user),
        password,
      );
    if (
      old &&
      previous.user &&
      (old.id !== id || previous.user !== user || auth.value !== "sql")
    )
      await context.secrets.delete(
        secretKey(f.uri.toString(), old.id, previous.user),
      );
    await invalidate();
    void vscode.window.showInformationMessage(
      "Connection saved to workspace ibcmd-rs.toml. Select it to connect.",
    );
  }
  for (const [name, run] of Object.entries(commands))
    context.subscriptions.push(
      vscode.commands.registerCommand(name, async (...args: any[]) => {
        if (!vscode.workspace.isTrusted) return;
        try {
          await run(...args);
        } catch (e) {
          const importCommand =
            name === "ibcmd.importSelected" || name === "ibcmd.importChanged";
          void vscode.window.showErrorMessage(
            importCommand && e instanceof ImportValidationError
              ? e.message
              : safeError(e),
          );
        }
      }),
    );
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("ibcmd")) void invalidate();
    }),
  );
  const watcher = vscode.workspace.createFileSystemWatcher("**/ibcmd-rs.toml");
  for (const event of [
    watcher.onDidChange,
    watcher.onDidCreate,
    watcher.onDidDelete,
  ])
    context.subscriptions.push(
      event((uri) => {
        // VS Code may deliver a save event after a new connection already read it.
        // Compare the active snapshot, and ignore unrelated multi-root/subfolder files.
        const f = folder;
        const stamp = generation.capture();
        if (!f || uri.fsPath !== file(f)) return;
        void snapshot(file(f))
          .then((contents) => {
            if (
              generation.current(stamp) &&
              folder === f &&
              activeSettings !== undefined &&
              contents !== activeSettings
            )
              void invalidate();
          })
          .catch(() => {
            if (generation.current(stamp) && folder === f) void invalidate();
          });
      }),
    );
  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders((e) => {
      if (
        folder &&
        e.removed.some((f) => f.uri.toString() === folder!.uri.toString())
      ) {
        folder = undefined;
        selected = undefined;
        void invalidate();
      }
    }),
  );
  context.subscriptions.push(watcher, {
    dispose: () => {
      generation.next();
      void client?.dispose();
    },
  });
  return {
    status: () => status.text,
    objects: () => objects.snapshot(),
    visibleObjects: () => objects.getChildren(),
    objectDecoration: (uri: vscode.Uri) => objects.provideFileDecoration(uri),
  };
}
import { readConfig } from "./config";
function readLocal(text: string, id: string | undefined) {
  return id && readConfig(text).some((b) => idOf(b) === id);
}
