import * as vscode from "vscode";
import {
  DatabaseSource,
  Node,
  Rpc,
  Source,
  TreeModel,
  XmlSource,
  localPath,
} from "./configuration-tree";

export class ConfigurationView
  implements vscode.TreeDataProvider<Node>, vscode.Disposable
{
  private changed = new vscode.EventEmitter<Node | undefined>();
  readonly onDidChangeTreeData = this.changed.event;
  private model?: TreeModel;
  private view: vscode.TreeView<Node>;
  constructor(private context: vscode.ExtensionContext) {
    this.view = vscode.window.createTreeView("ibcmd.configuration", {
      treeDataProvider: this,
      showCollapseAll: true,
    });
    this.view.message = "Выберите подключение или каталог XML";
    context.subscriptions.push(this, this.view);
    const command = (id: string, run: (...args: any[]) => Promise<unknown>) => {
      context.subscriptions.push(
        vscode.commands.registerCommand(id, async (...args: any[]) => {
          if (!vscode.workspace.isTrusted) return;
          try {
            return await run(...args);
          } catch {
            void vscode.window.showErrorMessage(
              "Не удалось прочитать конфигурацию. Проверьте источник и повторите обновление.",
            );
          }
        }),
      );
    };
    command("ibcmd.tree.xml", async () => {
      const picked = await vscode.window.showOpenDialog({
        canSelectFolders: true,
        canSelectFiles: false,
        canSelectMany: false,
        openLabel: "Открыть конфигурацию XML",
      });
      if (picked?.[0]?.scheme === "file")
        this.use(new XmlSource(picked[0].fsPath));
    });
    command("ibcmd.tree.refresh", async () => {
      if (this.model) this.use(this.model.source);
    });
    command("ibcmd.tree.openLocal", async (node?: Node) => {
      if (!node?.path || !this.model || !this.owns(node)) return;
      const local = new XmlSource(this.model.source.directory);
      if (!(await local.read(node.path, new AbortController().signal)))
        throw new Error("Местный файл не найден");
      await vscode.commands.executeCommand(
        "vscode.open",
        vscode.Uri.file(localPath(this.model.source.directory, node.path)),
      );
    });
    command("ibcmd.tree.search", async () => {
      const model = this.model;
      if (!model) return;
      const query = await vscode.window.showInputBox({
        prompt: "Имя или синоним объекта",
      });
      if (!query?.trim() || this.model !== model) return;
      const result = await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "Поиск в конфигурации",
          cancellable: true,
        },
        async (_, token) => {
          const abort = new AbortController();
          if (token.isCancellationRequested) abort.abort();
          const sub = token.onCancellationRequested(() => abort.abort());
          try {
            return await model.search(query.trim(), abort.signal);
          } finally {
            sub.dispose();
          }
        },
      );
      if (this.model !== model) return;
      const picked = await vscode.window.showQuickPick(
        result.map((node) => ({
          label: node.label,
          description: node.synonym,
          detail: node.fullName,
          node,
        })),
        {
          placeHolder: result.length
            ? "Перейти к объекту"
            : "Объекты не найдены",
        },
      );
      if (picked && this.model === model)
        await this.view.reveal(picked.node, { select: true, focus: true });
    });
    const watcher = vscode.workspace.createFileSystemWatcher("**/*.{xml,bsl}");
    const invalidate = () => {
      if (this.model && !this.model.source.remote) this.use(this.model.source);
    };
    context.subscriptions.push(
      watcher,
      watcher.onDidChange(invalidate),
      watcher.onDidCreate(invalidate),
      watcher.onDidDelete(invalidate),
    );
  }
  private roots = new Set<Node>();
  owns(node: Node): boolean {
    let root = node;
    while (root.parent) root = root.parent;
    return this.roots.has(root);
  }
  source(node?: Node): Source | undefined {
    return !node || this.owns(node) ? this.model?.source : undefined;
  }
  use(source: Source) {
    this.model?.dispose();
    this.roots.clear();
    this.model = new TreeModel(source);
    this.view.message = undefined;
    this.view.description = source.remote ? "База данных" : "Файлы XML";
    this.changed.fire(undefined);
  }
  connect(rpc: Rpc, directory: string, infobase: string) {
    this.use(new DatabaseSource(rpc, directory, infobase));
  }
  disconnect() {
    if (!this.model?.source.remote) return;
    this.model.dispose();
    this.model = undefined;
    this.roots.clear();
    this.view.message = "Подключение закрыто. Выберите базу заново.";
    this.changed.fire(undefined);
  }
  getParent(node: Node) {
    return node.parent;
  }
  getTreeItem(node: Node) {
    const expandable =
      node.type === "object" ||
      node.type === "group" ||
      !!node.children?.length;
    const item = new vscode.TreeItem(
      node.label,
      expandable
        ? vscode.TreeItemCollapsibleState.Collapsed
        : vscode.TreeItemCollapsibleState.None,
    );
    item.id = node.id;
    item.description = node.synonym;
    item.tooltip = [
      node.fullName,
      node.synonym,
      node.type === "property" ? "Только просмотр" : "",
    ]
      .filter(Boolean)
      .join("\n");
    item.contextValue =
      node.type === "property"
        ? "ibcmd.property"
        : node.path
          ? "ibcmd.source"
          : "ibcmd.group";
    item.iconPath = new vscode.ThemeIcon(
      node.type === "group"
        ? "folder"
        : node.type === "file"
          ? "file-code"
          : node.type === "property"
            ? "symbol-field"
            : "symbol-class",
    );
    if (node.path)
      item.command = {
        command: "ibcmd.tree.openLocal",
        title: "Открыть местный файл",
        arguments: [node],
      };
    return item;
  }
  async getChildren(node?: Node): Promise<Node[]> {
    const model = this.model;
    if (!model || (node && !this.owns(node))) return [];
    try {
      const result = await model.children(node);
      if (model !== this.model) return [];
      if (!node) this.roots = new Set(result);
      return result;
    } catch {
      if (model === this.model)
        this.view.message =
          "Ошибка чтения. Проверьте источник и нажмите «Обновить дерево».";
      return [];
    }
  }
  dispose() {
    this.model?.dispose();
    this.changed.dispose();
  }
}
