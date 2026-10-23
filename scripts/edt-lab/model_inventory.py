#!/usr/bin/env python3
"""Read-only EDT model evidence; structural comparison is not codec acceptance."""
import argparse
import hashlib
import json
import os
import stat
from pathlib import Path
import zipfile


CATEGORIES = ("modelTypes", "importers", "exporters")
MODEL_SUFFIXES = (".ecore", ".xcore", ".genmodel")
MANIFEST_LIMIT = 65536
RESOURCE_LIMIT = 256 * 1024 * 1024


def external_output(path, lab):
    # This Mac laboratory must never create evidence or caches on its internal disk.
    if not lab.is_absolute() or lab.parts[:2] != ("/", "Volumes") or len(lab.parts) < 4:
        raise ValueError("lab root must name an external /Volumes laboratory")
    if not path.is_absolute() or ".." in path.parts or ".." in lab.parts:
        raise ValueError("output and lab root must be absolute without parent traversal")
    path.relative_to(lab)  # lexical containment before inspecting ancestors
    for ancestor in [*reversed(path.parent.parents), path.parent]:
        mode = ancestor.lstat().st_mode
        if stat.S_ISLNK(mode) or not stat.S_ISDIR(mode):
            raise ValueError("output has a linked or non-directory ancestor")
    if not lab.is_dir() or path == lab:
        raise ValueError("output must be a file below the existing lab")
    return path


def stream_digest(stream, limit):
    total = 0
    sha = hashlib.sha256()
    while chunk := stream.read(65536):
        total += len(chunk)
        if total > limit:
            raise ValueError("diagnostic resource byte limit exceeded")
        sha.update(chunk)
    return total, sha.hexdigest()



def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def manifest_fields(raw):
    """Unfold JAR manifest continuation lines before extracting bundle identity."""
    lines = []
    for line in raw.decode("utf-8").splitlines():
        if not line:
            break  # bundle identity belongs to the main section only
        if line.startswith(" "):
            if not lines:
                raise ValueError("orphan manifest continuation")
            lines[-1] += line[1:]
        else:
            lines.append(line)
    fields = {}
    for line in lines:
        key, separator, value = line.partition(": ")
        if not separator or key in fields:
            raise ValueError("malformed or duplicate manifest field")
        fields[key] = value
    return fields


def inspect_bundle(source, expected):
    """Inspect a path or in-memory ZIP without extracting or executing code."""
    with zipfile.ZipFile(source) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError("duplicate ZIP entry")
        # Signed JAR manifests contain large per-entry digest sections. Only
        # the bounded main section contains bundle identity; do not buffer tails.
        raw = bytearray()
        with archive.open("META-INF/MANIFEST.MF") as stream:
            while True:
                line = stream.readline(MANIFEST_LIMIT + 1 - len(raw))
                if not line or line in (b"\n", b"\r\n"):
                    break
                raw.extend(line)
                if len(raw) > MANIFEST_LIMIT:
                    raise ValueError("manifest exceeds diagnostic byte limit")
        fields = manifest_fields(bytes(raw))
        identity = fields["Bundle-SymbolicName"].split(";", 1)[0]
        if identity != expected["symbolicName"]:
            raise ValueError("bundle filename/manifest identity mismatch")
        version = fields["Bundle-Version"]
        if not version:
            raise ValueError("empty bundle version")
        classes = {name[:-6].replace("/", ".") for name in names
                   if name.endswith(".class") and not name.startswith("META-INF/")}
        delta = {}
        for category in CATEGORIES:
            previous = expected[category]
            if len(previous) != len(set(previous)):
                raise ValueError("duplicate baseline class identity")
            delta[category] = {
                "baseline_count": len(previous),
                "present_count": len(set(previous) & classes),
                "missing": sorted(set(previous) - classes),
            }
        resources = []
        for name in sorted(names):
            if name.endswith(MODEL_SUFFIXES):
                with archive.open(name) as stream:
                    size, sha = stream_digest(stream, RESOURCE_LIMIT)
                resources.append({"path": name, "bytes": size, "sha256": sha})
        return {"symbolicName": identity, "baseline_version": expected["version"],
                "installed_version": version, "class_count": len(classes),
                "baseline_class_delta": delta, "model_resources": resources}


def inventory(plugins, baseline, release):
    """Account for every baseline bundle; missing/ambiguous jars stay explicit."""
    rows = []
    identities = set()
    for expected in baseline["bundles"]:
        identity = expected["symbolicName"]
        if identity in identities:
            raise ValueError("duplicate baseline bundle identity")
        identities.add(identity)
        candidates = sorted(plugins.glob(identity + "_*.jar"))
        if len(candidates) != 1:
            rows.append({"symbolicName": identity, "status": "unresolved",
                         "candidate_count": len(candidates)})
            continue
        path = candidates[0]
        # One descriptor binds ZIP inspection and both hashes. Path replacement
        # or in-place mutation invalidates the capture rather than mixing evidence.
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, "rb") as stream:
            initial = os.fstat(stream.fileno())
            if not stat.S_ISREG(initial.st_mode):
                raise ValueError("bundle must be a regular unlinked JAR")
            before = hashlib.file_digest(stream, "sha256").hexdigest()
            stream.seek(0)
            row = inspect_bundle(stream, expected)
            stream.seek(0)
            after = hashlib.file_digest(stream, "sha256").hexdigest()
            final = os.fstat(stream.fileno())
            current = path.lstat()
            identity = lambda st: (st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns)
            if before != after or identity(initial) != identity(final) or identity(final) != identity(current):
                raise ValueError("bundle changed during inspection")
        row["jar_sha256"] = before
        row["jar_bytes"] = initial.st_size
        row["status"] = "inspected"
        rows.append(row)
    return {"schema_version": 1, "declared_installed_release": release,
            "baseline_source": baseline["source"],
            "scope": "Every legacy inventory bundle; unrelated/new installed bundles are outside this comparison.",
            "semantic_compatibility": "NOT_ESTABLISHED",
            "summary": {"baseline_bundles": len(rows),
                        "inspected_bundles": sum(r["status"] == "inspected" for r in rows),
                        "missing_baseline_class_names": sum(
                            len(d["missing"]) for r in rows
                            for d in r.get("baseline_class_delta", {}).values())},
            "bundles": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plugins", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--installed-release", required=True,
                        help="Exact separately verified build label; never inferred as latest")
    parser.add_argument("--lab-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = external_output(args.output, args.lab_root)
    raw = args.baseline.read_bytes()
    report = inventory(args.plugins, json.loads(raw), args.installed_release)
    report["baseline_sha256"] = digest(raw)
    # Refuse replacement; preserve earlier evidence and avoid source mutation.
    with output.open("x", encoding="utf-8") as stream:
        json.dump(report, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
