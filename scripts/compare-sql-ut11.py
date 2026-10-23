#!/usr/bin/env python3
"""Offline, byte-exact three-tree evidence collector; never executes SQL."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys


def inventory(root, budget):
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("tree must be a real directory")
    files = {}
    def traversal_error(error):
        raise error
    for directory, dirs, names in os.walk(root, followlinks=False, onerror=traversal_error):
        for name in dirs + names:
            path = Path(directory) / name
            mode = path.lstat().st_mode
            if not (stat.S_ISDIR(mode) or stat.S_ISREG(mode)):
                raise ValueError("tree contains a symlink or special file")
        for name in sorted(names):
            path = Path(directory) / name
            budget[0] -= 1
            if budget[0] < 0:
                raise ValueError("file count limit exceeded")
            digest = hashlib.sha256()
            fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
            with os.fdopen(fd, "rb") as stream:
                before = os.fstat(stream.fileno())
                if not stat.S_ISREG(before.st_mode) or before.st_size > budget[1]:
                    raise ValueError("special file or byte limit exceeded")
                for chunk in iter(lambda: stream.read(min(1024 * 1024, budget[1] + 1)), b""):
                    budget[1] -= len(chunk)
                    if budget[1] < 0:
                        raise ValueError("byte limit exceeded")
                    digest.update(chunk)
                after = os.fstat(stream.fileno())
            if (before.st_size, before.st_mtime_ns, before.st_ino) != (after.st_size, after.st_mtime_ns, after.st_ino):
                raise ValueError("tree changed while hashing")
            files[path.relative_to(root).as_posix()] = {
                "bytes": before.st_size, "sha256": digest.hexdigest()}
    if "Configuration.xml" not in files or "ConfigDumpInfo.xml" not in files:
        raise ValueError("full tree requires Configuration.xml and ConfigDumpInfo.xml")
    return dict(sorted(files.items()))


def compare(left, right):
    counts = dict(unchanged=0, different=0, left_only=0, right_only=0)
    differences = []
    for path in sorted(left.keys() | right.keys()):
        status = ("right_only" if path not in left else
                  "left_only" if path not in right else
                  "unchanged" if left[path] == right[path] else "different")
        counts[status] += 1
        if status != "unchanged":
            differences.append({"path": path, "status": status,
                                "left": left.get(path), "right": right.get(path)})
    return {"summary": counts, "differences": differences}


def collect(roots, max_files=600000, max_bytes=32 * 1024**3):
    resolved = [Path(p).resolve(strict=True) for p in roots]
    for i, first in enumerate(resolved):
        for second in resolved[i + 1:]:
            if first == second or first in second.parents or second in first.parents:
                raise ValueError("three distinct, non-overlapping trees required")
    budget = [max_files, max_bytes]
    trees = {name: inventory(path, budget) for name, path in zip(
        ("native_source", "ours_source", "native_after_import"), roots)}
    comparisons = {"export": compare(trees["native_source"], trees["ours_source"]),
                   "roundtrip": compare(trees["native_source"], trees["native_after_import"])}
    equal = all(not value["differences"] for value in comparisons.values())
    return {"schema_version": 1, "evidence_kind": "offline_tree_comparison_only",
            "sql_acceptance": "requires_independent_sql_provenance_and_run_receipts",
            "method": "all relative file paths, sizes and SHA-256; no exclusions or normalization",
            "trees": trees, "comparisons": comparisons, "all_bytes_equal": equal}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_source", type=Path)
    parser.add_argument("ours_source", type=Path)
    parser.add_argument("native_after_import", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-files", type=int, default=600000)
    parser.add_argument("--max-bytes", type=int, default=32 * 1024**3)
    args = parser.parse_args()
    try:
        if args.max_files < 1 or args.max_bytes < 1:
            raise ValueError("limits must be positive")
        roots = [args.native_source, args.ours_source, args.native_after_import]
        output = args.output.resolve()
        if any(output == p.resolve() or p.resolve() in output.parents for p in roots):
            raise ValueError("output must be outside input trees")
        if args.output.exists() or args.output.is_symlink():
            raise ValueError("output already exists")
        report = collect(roots, args.max_files, args.max_bytes)
        with args.output.open("x", encoding="utf-8") as stream:
            json.dump(report, stream, ensure_ascii=False, indent=2)
            stream.write("\n")
        print(json.dumps({key: value["summary"] for key, value in report["comparisons"].items()}))
        return 0 if report["all_bytes_equal"] else 1
    except (OSError, ValueError):
        # Paths and OS exception details can contain private corpus metadata.
        print("comparison failed: invalid, changing, inaccessible or over-budget inputs/output", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
