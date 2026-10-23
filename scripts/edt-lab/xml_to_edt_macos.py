#!/usr/bin/env python3
"""Strict, retained macOS XML→EDT lab capture; never production code.

A PASS describes this corpus/build only, not unrestricted EDT compatibility.
All installed-EDT workspaces, tool homes, logs and intermediate trees are new.
No cleanup runs: move an entire completed run to Trash when no longer needed.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

sys.dont_write_bytecode = True
_spec = importlib.util.spec_from_file_location("xml_edt_oracle", Path(__file__).with_name("oracle.py"))
oracle = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(oracle)
Error = oracle.OracleError


def contained(path: Path, root: Path) -> None:
    if ".." in path.parts or ".." in root.parts:
        raise Error("Parent traversal is refused")
    oracle.no_links(path)
    if path == root or not path.is_relative_to(root):
        raise Error(f"Path must be below the external lab root: {path}")


def require_exact_build(build: str) -> None:
    if not re.fullmatch(r"20\d{2}\.\d+\.\d+\.\d+", build):
        raise Error("An exact four-component installed EDT build is required")


def is_external_lab_root(path: Path) -> bool:
    return path.is_dir() and str(path).startswith("/Volumes/")


def preflight(args) -> None:
    if sys.platform != "darwin":
        raise Error("This capture requires macOS; control tests are not acceptance")
    for field in ("lab_root", "native", "run", "ours_exe", "edt_exe", "edt_vm", "edt_installation_manifest"):
        path = getattr(args, field).absolute()
        if ".." in path.parts:
            raise Error("Parent traversal is refused")
        oracle.no_links(path)
        setattr(args, field, path)
    if not is_external_lab_root(args.lab_root):
        raise Error("An existing external /Volumes/ lab root is required")
    contained(args.run, args.lab_root)
    contained(args.native, args.lab_root)
    contained(args.edt_installation_manifest, args.lab_root)
    contained(args.edt_exe, args.lab_root)  # Use #358’s complete external installation, including p2/configuration.
    if args.run.exists() or not args.run.parent.is_dir():
        raise Error("Run must be new with an existing parent; resuming evidence is refused")
    if args.run.is_relative_to(args.native) or args.native.is_relative_to(args.run):
        raise Error("Run and immutable native input must not overlap")
    require_exact_build(args.edt_build)
    if not 1 <= args.heap_gib <= 64 or args.timeout < 1:
        raise Error("Invalid heap or timeout")
    for tool in (args.ours_exe, args.edt_exe, args.edt_vm):
        if not tool.is_file() or not os.access(tool, os.X_OK):
            raise Error(f"Missing executable: {tool}")
    bind_installation(args)
    oracle.require_xml(args.native)
    oracle.snapshot(args.native)  # Reject links, special files and collisions before any launch.


class Capture:
    def __init__(self, args):
        self.args, self.run = args, args.run
        self.env = dict(os.environ)
        for name, directory in (("HOME", "home"), ("TMPDIR", "tmp"),
                                ("XDG_CACHE_HOME", "cache"), ("XDG_CONFIG_HOME", "config"),
                                ("XDG_DATA_HOME", "data"), ("XDG_STATE_HOME", "state")):
            path = self.run / directory
            path.mkdir()
            self.env[name] = str(path)
        self.env["IBCMD_TEST_KEEP_FIXTURES"] = "1"
        self.labels = []

    def command(self, label, argv, workspace=None):
        record = {"argv": [str(x) for x in argv], "cwd": str(self.run),
                  "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                  "timeout_seconds": self.args.timeout, "isolated_home": self.env["HOME"]}
        started = time.monotonic()
        try:
            with (self.run / f"{label}.stdout").open("xb") as out, \
                 (self.run / f"{label}.stderr").open("xb") as err:
                proc = subprocess.Popen(record["argv"], cwd=self.run, env=self.env,
                    stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
                try:
                    record["exit_code"] = proc.wait(timeout=self.args.timeout)
                except BaseException as error:
                    record["timeout" if isinstance(error, subprocess.TimeoutExpired) else "cancelled"] = True
                    try:
                        os.killpg(proc.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    proc.wait()
                    record["exit_code"] = proc.returncode
                    if not isinstance(error, subprocess.TimeoutExpired):
                        raise
        except OSError as error:
            record["launch_error"] = str(error)
            raise Error(f"{label} could not launch; see preserved receipt") from error
        finally:
            record["duration_seconds"] = round(time.monotonic() - started, 3)
            oracle.write_json(self.run / f"{label}.command.json", record)
            if workspace:
                for name in (".log", "1cedtcli.log", "1cedtcli-shutdown-hook.log"):
                    source = workspace / ".metadata" / name
                    if source.exists():
                        oracle.no_links(source)
                        shutil.copyfile(source, self.run / f"{label}.workspace-{name.lstrip('.')}")
        self.labels.append(label)
        if record.get("timeout") or record.get("exit_code") != 0:
            raise Error(f"{label} failed; see preserved receipt and streams")
        return (self.run / f"{label}.stdout").read_bytes()

    def edt(self, label, operation, workspace_name=None):
        workspace = self.run / (workspace_name or f"{label}-workspace")
        if workspace_name is None and workspace.exists():
            raise Error("EDT workspace must be fresh")
        return self.command(label, [self.args.edt_exe, "-vm", self.args.edt_vm, "-data", workspace,
            "-timeout", str(self.args.timeout), "-nl", "en_US", "-vmargs",
            f"-Xmx{self.args.heap_gib}g", f"-Duser.home={self.run / 'home'}",
            f"-Djava.io.tmpdir={self.run / 'tmp'}", "-command", *operation], workspace)

    def convert(self, label, source, target, reverse=False):
        xml, edt = f"xml-{self.args.source_version}", f"edt-2025.2.3-xml-{self.args.source_version}"
        self.command(label, [self.args.ours_exe, "convert", source, target,
            "--source-format", "edt" if reverse else "xml",
            "--target-format", "xml" if reverse else "edt",
            "--source-profile", edt if reverse else xml,
            "--target-profile", xml if reverse else edt,
            "--report", self.run / f"{label}.report.json"])


def copy_without_provenance(project: Path, target: Path) -> dict:
    before = oracle.snapshot(project)
    if not any(row["path"].startswith(".ibcmd-provenance/") for row in before["files"]):
        raise Error("Generated provenance is missing; stripping control is unproven")
    target.mkdir()
    for row in before["files"]:
        if row["path"].split("/")[0] == ".ibcmd-provenance":
            continue
        output = target / row["path"]
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(project / row["path"], output)
    after = oracle.snapshot(target)
    expected = [row for row in before["files"] if row["path"].split("/")[0] != ".ibcmd-provenance"]
    if after["files"] != expected or oracle.snapshot(project) != before:
        raise Error("Project copy or immutable generated project changed")
    return after


def diagnostics_for_all_logs(run: Path, labels: list[str]) -> dict:
    result = oracle.command_diagnostics(run, labels)
    for label in labels:
        for suffix in ("1cedtcli.log", "1cedtcli-shutdown-hook.log"):
            path = run / f"{label}.workspace-{suffix}"
            if not path.is_file():
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if re.search(r"\b(?:ERROR|FATAL)\b|^Exception in thread |^!ENTRY \S+ [48] ", line):
                    result[label]["error_lines"].append({"file": path.name, "line": number, "text": line})
                    result[label]["error_count"] += 1
    return result


def bind_installation(args) -> dict:
    path = args.edt_installation_manifest
    oracle.no_links(path)
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1 or Path(manifest.get("edt_cli", "")) != args.edt_exe:
        raise Error("Installation manifest schema or CLI binding mismatch")
    if manifest.get("edt_cli_sha256") != oracle.digest(args.edt_exe):
        raise Error("Installation manifest CLI hash mismatch")
    plugins = manifest["plugins"]
    plugin_root = Path(plugins["root"])
    contained(plugin_root, args.lab_root)
    if oracle.snapshot(plugin_root) != plugins:
        raise Error("Installed plugin tree differs from installation manifest")
    runtime_files = manifest["runtime_files"]
    if not runtime_files or not {str(args.edt_exe), str(args.edt_vm)}.issubset({row["path"] for row in runtime_files}):
        raise Error("Installation runtime manifest must bind both CLI and Java executable")
    for row in runtime_files:
        runtime = Path(row["path"])
        oracle.no_links(runtime)
        if not runtime.is_absolute() or ".." in runtime.parts or runtime.stat().st_size != row["bytes"] or oracle.digest(runtime) != row["sha256"]:
            raise Error("Installation runtime file changed")
    return {"manifest_path": str(path), "manifest_sha256": oracle.digest(path),
            "plugins_tree_sha256": plugins["tree_sha256"], "runtime_files": runtime_files,
            "status": manifest.get("status")}


def execute(args) -> dict:
    cap = Capture(args)
    installation_before = bind_installation(args)
    oracle.write_json(args.run / "installation-binding-before.json", installation_before)
    legacy_schema = Path(__file__).resolve().parents[2] / "crates/ibcmd-schema/data"
    schema_files = ("model-inventory", "package-features", "writer-rules", "metadata-order")
    oracle.write_json(args.run / "writer-schema-binding.json", {
        "model_release": "2025.2.3", "current_schema_equivalence_claimed": False,
        "resources": [{"path": f"crates/ibcmd-schema/data/edt-2025.2.3-{name}.json",
                       "sha256": oracle.digest(legacy_schema / f"edt-2025.2.3-{name}.json")} for name in schema_files]})
    native_before = oracle.snapshot(args.native)
    oracle.write_json(args.run / "native-before.json", native_before)
    tools = {name: {"path": str(path), "sha256": oracle.digest(path)}
             for name, path in (("candidate", args.ours_exe), ("edt", args.edt_exe), ("java", args.edt_vm))}
    oracle.write_json(args.run / "tools-before.json", tools)
    cap.command("ours-version", [args.ours_exe, "--version"])
    actual = cap.edt("edt-version", ["version"]).decode("utf-8", errors="strict").strip()
    oracle.write_json(args.run / "edt-version.json", {"actual": actual, "expected": args.edt_build})
    if actual != args.edt_build:
        raise Error(f"Installed EDT exact version mismatch: {actual!r}")
    runtime = {"2.20": "8.3.27", "2.21": "8.5.1"}[args.source_version]
    # A genuine import/export from the same XML supplies the same-serializer control.
    cap.edt("edt-import-native", ["import", "--version", runtime,
        "--configuration-files", str(args.native), "--project-name", "NativeControl", "--build", "true"],
        "native-workspace")
    baseline = args.run / "native-workspace/NativeControl"
    oracle.require_project(baseline, runtime)
    authentic = args.run / "authentic-xml"
    cap.edt("edt-export-native", ["export", "--project-name", "NativeControl",
        "--configuration-files", str(authentic)], "native-workspace")
    oracle.require_xml(authentic, require_dump_info=False)
    authentic_before = oracle.snapshot(authentic)
    oracle.write_json(args.run / "authentic-xml.json", authentic_before)
    project = args.run / "generated-project"
    cap.convert("xml-to-edt", args.native, project)
    oracle.require_project(project, runtime)
    project_before = oracle.snapshot(project)
    oracle.write_json(args.run / "generated-project-before.json", project_before)
    stripped = args.run / "generated-without-provenance"
    stripped_before = copy_without_provenance(project, stripped)
    oracle.write_json(args.run / "stripped-project-before.json", stripped_before)
    installed = args.run / "installed-generated-xml"
    cap.edt("edt-export-generated", ["export", "--project", str(stripped),
        "--configuration-files", str(installed)])
    oracle.require_xml(installed, require_dump_info=False)
    installed_before = oracle.snapshot(installed)
    oracle.write_json(args.run / "installed-generated-xml.json", installed_before)
    tsv = args.run / "generated-validation.tsv"
    cap.edt("edt-validate-generated", ["validate", "--file", str(tsv), "--project-list", str(stripped)])
    if not tsv.is_file():
        raise Error("Installed EDT validation did not produce its diagnostic TSV")
    validation = oracle.summarize_validation_tsv(tsv)
    oracle.write_json(args.run / "validation.json", validation)
    returned = args.run / "returned-xml"
    cap.convert("edt-to-xml", project, returned, reverse=True)
    oracle.require_xml(returned)
    returned_before = oracle.snapshot(returned)
    oracle.write_json(args.run / "returned-xml.json", returned_before)
    exact = oracle.compare_tree_snapshots(native_before, returned_before)
    serialized = oracle.compare_tree_snapshots(authentic_before, installed_before)
    oracle.write_json(args.run / "exact-return-comparison.json", exact)
    oracle.write_json(args.run / "installed-serializer-comparison.json", serialized)
    diagnostics = diagnostics_for_all_logs(args.run, cap.labels)
    oracle.write_json(args.run / "diagnostics.json", diagnostics)
    native_after, project_after, stripped_after = (oracle.snapshot(path) for path in (args.native, project, stripped))
    for label, value in (("native", native_after), ("generated-project", project_after), ("stripped-project", stripped_after)):
        oracle.write_json(args.run / f"{label}-after.json", value)
    tools_unchanged = all(oracle.digest(Path(row["path"])) == row["sha256"] for row in tools.values())
    installation_after = bind_installation(args)
    oracle.write_json(args.run / "installation-binding-after.json", installation_after)
    gates = {"installation_unchanged_and_bound": installation_before == installation_after,
             "exact_original_xml_tree_including_dump_info": exact["equal"],
             "same_installed_serializer_exact_tree": serialized["equal"],
             "no_source_errors_or_unknown_validation_rows": not validation["unresolved_source_diagnostics"],
             "no_command_or_workspace_errors": not oracle.has_error_diagnostics(diagnostics.values()),
             "native_unchanged": native_before == native_after,
             "generated_project_unchanged": project_before == project_after,
             "stripped_project_unchanged": stripped_before == stripped_after,
             "tools_unchanged": tools_unchanged,
             "authentic_export_unchanged": oracle.snapshot(authentic) == authentic_before,
             "installed_export_unchanged": oracle.snapshot(installed) == installed_before}
    return {"status": "PASS" if all(gates.values()) else "FAIL", "gates": gates,
        "installed_edt_build": actual, "writer_model_profile": f"edt-2025.2.3-xml-{args.source_version}",
        "model_schema_evidence": "Legacy model profile exercised against the named installed build; no new model/schema profile is asserted",
        "scope": "This supplied corpus and exact tool build only. Exact XML return uses the retained generated project with provenance; installed EDT loads the separate provenance-free copy. This does not claim exact native XML from EDT's own serializer or unrestricted configuration support."}


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    for name in ("lab-root", "native", "run", "ours-exe", "edt-exe", "edt-vm", "edt-installation-manifest"):
        result.add_argument(f"--{name}", type=Path, required=True)
    result.add_argument("--source-version", choices=("2.20", "2.21"), required=True)
    result.add_argument("--edt-build", required=True)
    result.add_argument("--timeout", type=int, default=7200)
    result.add_argument("--heap-gib", type=int, default=4)
    return result


def main():
    args = parser().parse_args()
    created = False
    try:
        preflight(args)
        args.run.mkdir()
        created = True
        for path in (Path(__file__), Path(oracle.__file__)):
            shutil.copyfile(path, args.run / path.name)
        oracle.write_json(args.run / "invocation.json", {
            **{key: str(value) if isinstance(value, Path) else value for key, value in vars(args).items()},
            "harness_sha256": oracle.digest(Path(__file__)), "oracle_sha256": oracle.digest(Path(oracle.__file__))})
        result = execute(args)
        oracle.write_json(args.run / "acceptance.json", result)
        print(json.dumps(result, ensure_ascii=False))
        return 0 if result["status"] == "PASS" else 1
    except (Error, OSError, ValueError, KeyError, TypeError) as error:
        result = {"status": "FAIL", "error": str(error), "installed_acceptance": False}
        if created:
            oracle.write_json(args.run / "failure.json", result)
        print(json.dumps(result, ensure_ascii=False), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
