import * as vscode from "vscode";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { RpcClient } from "./rpc";
import {
  candidatePaths,
  classify,
  comparisons,
  labels,
  localChanged,
  safePath,
  State,
} from "./object-status";
import { gitBaseline } from "./git-baseline";
import { Generation } from "./status";

interface Row {
  object: string;
  state: State;
  uri: vscode.Uri;
}
export class ObjectView
  implements
    vscode.TreeDataProvider<Row>,
    vscode.FileDecorationProvider,
    vscode.Disposable
{
  private tree = new vscode.EventEmitter<Row | undefined>();
  private decorations = new vscode.EventEmitter<vscode.Uri[] | undefined>();
  readonly onDidChangeTreeData = this.tree.event;
  readonly onDidChangeFileDecorations = this.decorations.event;
  private rows: Row[] = [];
  private files = new Map<string, State>();
  private generation = new Generation();
  private abort?: AbortController;
  private roots: { descriptor: string; folder: string; state: State }[] = [];
  private changedOnly = false;
  private message = "Выберите подключение и обновите статусы";
  constructor(private context: vscode.ExtensionContext) {
    const view = vscode.window.createTreeView("ibcmd.objects", {
      treeDataProvider: this,
    });
    view.description = "Байты экспорта · preview";
    context.subscriptions.push(
      view,
      vscode.window.registerFileDecorationProvider(this),
      this,
    );
  }
  clear(message = "Статус неизвестен — требуется обновление") {
    this.generation.next();
    this.abort?.abort();
    this.rows = [];
    this.files.clear();
    this.roots = [];
    this.message = message;
    this.tree.fire(undefined);
    this.decorations.fire(undefined);
  }
  toggle() {
    this.changedOnly = !this.changedOnly;
    this.tree.fire(undefined);
  }
  snapshot() {
    return this.rows.map(({ object, state }) => ({ object, state }));
  }
  getTreeItem(row: Row) {
    const item = new vscode.TreeItem(row.object);
    item.description = labels[row.state];
    item.tooltip = `${row.object}: ${labels[row.state]}. Сравнение байтов экспорта; native-версии не проверены.`;
    item.resourceUri = row.uri;
    return item;
  }
  getChildren() {
    if (!this.rows.length)
      return [
        {
          object: this.message,
          state: "unknown" as State,
          uri: vscode.Uri.from({ scheme: "ibcmd-status", path: "/message" }),
        },
      ];
    return this.rows.filter((r) => !this.changedOnly || r.state !== "same");
  }
  provideFileDecoration(uri: vscode.Uri): vscode.FileDecoration | undefined {
    const state =
      this.files.get(uri.toString()) ??
      (uri.scheme === "file"
        ? this.roots.find(
            (r) =>
              uri.fsPath === r.descriptor ||
              uri.fsPath.startsWith(r.folder + path.sep),
          )?.state
        : undefined);
    if (!state) return;
    const badges: Record<State, string> = {
      same: "=",
      database: "Б",
      file: "Ф",
      both: "БФ",
      databaseOnly: "+Б",
      fileOnly: "+Ф",
      unknown: "?",
    };
    return {
      badge: badges[state],
      tooltip: `${labels[state]}. Сравнение байтов экспорта; native-версии не проверены.`,
      color: new vscode.ThemeColor(
        state === "same"
          ? "gitDecoration.untrackedResourceForeground"
          : state === "unknown"
            ? "disabledForeground"
            : "gitDecoration.modifiedResourceForeground",
      ),
    };
  }
  async refresh(
    rpc: RpcClient,
    folder: vscode.WorkspaceFolder,
    infobase: string,
  ) {
    this.clear("Обновление статусов…");
    const stamp = this.generation.capture();
    const abort = (this.abort = new AbortController());
    const cfg = vscode.workspace.getConfiguration("ibcmd", folder.uri);
    const source = path.resolve(
      folder.uri.fsPath,
      cfg.get<string>("sourceDirectory", "."),
    );
    const base = cfg.get<string>("statusBaselineDirectory", "");
    let baseline = base ? path.resolve(folder.uri.fsPath, base) : undefined;
    try {
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "ibcmd-rs: статусы объектов",
          cancellable: true,
        },
        async (_, token) => {
          if (token.isCancellationRequested) abort.abort();
          const sub = token.onCancellationRequested(() => abort.abort());
          try {
            const refName = cfg.get<string>("statusGitRef", "");
            if (refName)
              baseline = await gitBaseline(
                source,
                refName,
                cfg.get<string>("statusCacheDirectory", ""),
                abort.signal,
              );
            for (const dir of [source, ...(baseline ? [baseline] : [])]) {
              const stat = await fs.lstat(dir);
              if (!stat.isDirectory() || stat.isSymbolicLink())
                throw new Error("Invalid source/baseline directory");
            }
            if (
              vscode.workspace.textDocuments.some(
                (d) =>
                  d.isDirty &&
                  d.uri.scheme === "file" &&
                  d.uri.fsPath.startsWith(source + path.sep),
              )
            )
              throw new Error("Save source files before comparing");
            const current = comparisons(
              await rpc.request(
                "objects/status",
                { infobase, sourceDir: source },
                abort.signal,
              ),
            );
            const reference = baseline
              ? comparisons(
                  await rpc.request(
                    "objects/status",
                    { infobase, sourceDir: baseline },
                    abort.signal,
                  ),
                )
              : [];
            const byName = new Map(reference.map((r) => [r.object, r]));
            const paths = new Map<string, string>();
            const visited = new Set<string>();
            const visit = async (node?: string): Promise<void> => {
              const key = node ?? "";
              if (visited.has(key)) throw new Error("Cyclic tree response");
              visited.add(key);
              if (abort.signal.aborted) throw new Error("Cancelled");
              const result = await rpc.request(
                "tree/children",
                { infobase, ...(node ? { node } : {}) },
                abort.signal,
              );
              if (!Array.isArray(result?.nodes))
                throw new Error("Invalid tree response");
              for (const child of result.nodes) {
                if (
                  typeof child.fullName === "string" &&
                  typeof child.path === "string"
                )
                  paths.set(child.fullName, safePath(source, child.path));
                if (child.hasChildren) await visit(child.id);
              }
            };
            await visit();
            const roots: {
              descriptor: string;
              folder: string;
              state: State;
            }[] = [];
            const rows: Row[] = [];
            const files = new Map<string, State>();
            for (const row of current) {
              if (abort.signal.aborted) throw new Error("Cancelled");
              const ref = byName.get(row.object);
              const changed =
                baseline && ref
                  ? await localChanged(source, baseline, row, ref)
                  : undefined;
              const state = classify(row, ref, changed);
              const uri = vscode.Uri.from({
                scheme: "ibcmd-status",
                authority: "object",
                path:
                  "/" +
                  encodeURIComponent(
                    folder.uri.toString() + "|" + infobase + "|" + row.object,
                  ),
              });
              rows.push({ object: row.object, state, uri });
              const descriptor = paths.get(row.object);
              if (descriptor)
                roots.push({
                  descriptor,
                  folder:
                    row.object === "Configuration"
                      ? path.join(source, "Ext")
                      : descriptor.replace(/\.xml$/, ""),
                  state,
                });
              files.set(uri.toString(), state);
              for (const p of candidatePaths(row, ref))
                files.set(
                  vscode.Uri.file(safePath(source, p)).toString(),
                  state,
                );
            }
            if (!this.generation.current(stamp) || abort.signal.aborted) return;
            this.rows = rows;
            this.files = files;
            this.roots = roots.sort(
              (a, b) => b.folder.length - a.folder.length,
            );
            this.message = "Нет объектов";
            this.tree.fire(undefined);
            this.decorations.fire(undefined);
          } finally {
            sub.dispose();
          }
        },
      );
    } catch {
      if (this.generation.current(stamp))
        this.clear(
          "Статус неизвестен — обновление отменено или завершилось ошибкой",
        );
    }
  }
  dispose() {
    this.clear();
    this.tree.dispose();
    this.decorations.dispose();
  }
}
