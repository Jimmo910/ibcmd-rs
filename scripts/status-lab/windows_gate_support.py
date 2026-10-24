"""Bounded stdlib-only receipt/log helpers for public Windows fake CI."""
import hashlib
import json
from pathlib import Path

SOURCE_NAMES = {"rpc_probe.py", "process_scope.py", "readonly_probe.py", "fake_rpc_peer.py",
    "rpc-runner-contract.md", "test_rpc_probe.py", "test_windows_rpc_probe.py",
    "windows_rpc_gate.py", "windows_gate_support.py"}


def verified_sources():
    folder = Path(__file__).resolve().parent
    manifest_path = folder / "windows-source-sha256.json"
    if manifest_path.is_symlink() or manifest_path.stat().st_size > 16384:
        raise ValueError("manifest-boundary")
    manifest = json.loads(manifest_path.read_bytes())
    if set(manifest) != {"schema", "sha256"} or type(manifest["schema"]) is not int or manifest["schema"] != 1 or set(manifest["sha256"]) != SOURCE_NAMES:
        raise ValueError("manifest-schema")
    for name, expected in manifest["sha256"].items():
        path = folder / name
        if path.is_symlink() or path.stat().st_size > 1 << 20 or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError("source-binding")
    return manifest["sha256"]


class CappedLog:
    def __init__(self, path, cap=65536):
        self.stream = Path(path).open("xb")
        self.cap, self.size, self.overflow = cap, 0, False
    def write(self, text):
        data = text.encode("utf-8", errors="replace")
        left = self.cap - self.size
        self.overflow = self.overflow or len(data) > left
        data = data[:left]
        self.stream.write(data)
        self.size += len(data)
    def flush(self):
        self.stream.flush()
    def close(self):
        self.stream.close()


def artifact_inventory(root):
    # Public owned fixtures only; bound both enumeration and upload bytes.
    root = Path(root)
    pending = [root]
    files, entries, total = {}, 0, 0
    while pending:
        folder = pending.pop()
        import os
        with os.scandir(folder) as children:
            for entry in children:
                entries += 1
                if entries > 256 or entry.is_symlink() or (hasattr(Path(entry.path), "is_junction") and Path(entry.path).is_junction()):
                    raise ValueError("artifact-boundary")
                if entry.is_dir(follow_symlinks=False):
                    pending.append(Path(entry.path))
                else:
                    stamp = entry.stat(follow_symlinks=False)
                    if not entry.is_file(follow_symlinks=False) or stamp.st_nlink != 1 or stamp.st_size > 1 << 20 or len(files) >= 128:
                        raise ValueError("artifact-boundary")
                    total += stamp.st_size
                    if total > 4 << 20:
                        raise ValueError("artifact-cap")
                    files[Path(entry.path).relative_to(root).as_posix()] = stamp.st_size
    return {"files": len(files), "bytes": total, "entries": entries}


def admit_artifacts(root):
    root = Path(root)
    inventory = artifact_inventory(root)
    candidate = root / "artifact-pending.json"
    with candidate.open("x") as out:
        json.dump({"schema": 1, "capBytes": 4 << 20, **inventory}, out)
    artifact_inventory(root)  # candidate bytes/entry must pass before admission
    candidate.rename(root / "artifact-safe.json")
