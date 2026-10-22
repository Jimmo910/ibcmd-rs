import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { safePath } from "./object-status";
const exec = promisify(execFile);
export async function gitBaseline(
  source: string,
  ref: string,
  cache: string,
  signal: AbortSignal,
): Promise<string> {
  if (!path.isAbsolute(cache))
    throw new Error("Set an absolute statusCacheDirectory for Git snapshots");
  const git = async (...args: string[]) =>
    (
      await exec("git", ["-C", source, ...args], {
        signal,
        encoding: "buffer",
        maxBuffer: 64 * 1024 * 1024,
      })
    ).stdout;
  const root = (await git("rev-parse", "--show-toplevel")).toString().trim();
  const prefix = path.relative(root, source).split(path.sep).join("/");
  if (prefix.startsWith(".."))
    throw new Error("Source is outside Git repository");
  const tree = (
    await git("rev-parse", "--verify", "--end-of-options", `${ref}^{tree}`)
  )
    .toString()
    .trim();
  if (!/^[a-f0-9]{40,64}$/.test(tree)) throw new Error("Invalid Git tree");
  const entries = (await git("ls-tree", "-rz", "--full-tree", tree))
    .toString()
    .split("\0")
    .filter(Boolean);
  await fs.mkdir(cache, { recursive: true });
  const target = await fs.mkdtemp(path.join(cache, "ibcmd-status-"));
  // Retained as an inspectable cache snapshot; cleanup follows the user's Trash policy.
  for (const entry of entries) {
    if (signal.aborted) throw new Error("Cancelled");
    const tab = entry.indexOf("\t");
    const entryPath = entry.slice(tab + 1);
    if (prefix && !entryPath.startsWith(prefix + "/")) continue;
    const match = /^(\d+) blob ([a-f0-9]+)\t([\s\S]+)$/.exec(entry);
    if (!match) throw new Error("Unsupported Git entry");
    const [, mode, blob, name] = match;
    const relative = prefix
      ? name.startsWith(prefix + "/")
        ? name.slice(prefix.length + 1)
        : undefined
      : name;
    if (!relative) continue;
    if (!["100644", "100755"].includes(mode))
      throw new Error("Git symlinks are not supported");
    const dest = safePath(target, relative);
    await fs.mkdir(path.dirname(dest), { recursive: true });
    await fs.writeFile(dest, await git("cat-file", "blob", blob));
  }
  return target;
}
