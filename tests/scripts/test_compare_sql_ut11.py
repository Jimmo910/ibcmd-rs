"""Synthetic collector tests; all fixtures remain under the supplied external root."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest import mock
import uuid

SCRIPT = Path(__file__).resolve().parents[2] / "scripts/compare-sql-ut11.py"
spec = importlib.util.spec_from_file_location("collector", SCRIPT)
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class CollectorTests(unittest.TestCase):
    def setUp(self):
        self.work = Path(os.environ["IBCMD_SQL_UT11_TEST_ROOT"]) / uuid.uuid4().hex
        self.work.mkdir(parents=True)
        self.trees = [self.work / name for name in ("native", "ours", "roundtrip")]
        for tree in self.trees:
            tree.mkdir()
            (tree / "Configuration.xml").write_bytes(b"configuration")
            (tree / "ConfigDumpInfo.xml").write_bytes(b"versions")

    def test_equal_is_not_sql_acceptance(self):
        report = collector.collect(self.trees)
        self.assertTrue(report["all_bytes_equal"])
        self.assertEqual(report["evidence_kind"], "offline_tree_comparison_only")
        self.assertNotEqual(report["sql_acceptance"], "passed")
        self.assertEqual(len(report["trees"]["native_source"]), 2)

    def test_full_diff_includes_dumpinfo_missing_and_extra(self):
        (self.trees[1] / "ConfigDumpInfo.xml").write_bytes(b"changed")
        (self.trees[0] / "missing.xml").write_bytes(b"missing")
        (self.trees[1] / "extra.bin").write_bytes(b"extra")
        report = collector.collect(self.trees)
        self.assertFalse(report["all_bytes_equal"])
        self.assertEqual(report["comparisons"]["export"]["summary"],
                         dict(unchanged=1, different=1, left_only=1, right_only=1))
        self.assertEqual(len(report["comparisons"]["roundtrip"]["differences"]), 1)

    def test_independent_roundtrip_failure(self):
        (self.trees[2] / "Configuration.xml").write_bytes(b"changed")
        report = collector.collect(self.trees)
        self.assertEqual(report["comparisons"]["export"]["differences"], [])
        self.assertFalse(report["all_bytes_equal"])

    def test_budget_refusals(self):
        for kwargs in ({"max_files": 5}, {"max_bytes": 2}):
            with self.assertRaises(ValueError):
                collector.collect(self.trees, **kwargs)

    def test_alias_and_nested_refusals(self):
        with self.assertRaises(ValueError):
            collector.collect([self.trees[0], self.trees[0], self.trees[2]])
        nested = self.trees[0] / "nested"
        nested.mkdir()
        with self.assertRaises(ValueError):
            collector.collect([self.trees[0], nested, self.trees[2]])

    def test_symlink_refusal(self):
        (self.trees[1] / "alias").symlink_to(self.trees[0], target_is_directory=True)
        with self.assertRaises(ValueError):
            collector.collect(self.trees)

    def test_failed_subtree_scan_is_not_silent_equality(self):
        def broken_walk(root, *, followlinks, onerror):
            onerror(PermissionError("synthetic failed subtree scan"))
            yield  # Preserve os.walk's generator interface.
        with mock.patch.object(collector.os, "walk", broken_walk):
            with self.assertRaises(PermissionError):
                collector.collect(self.trees)

    def test_partial_tree_refusal(self):
        partial = self.work / "partial"
        partial.mkdir()
        with self.assertRaises(ValueError):
            collector.collect([self.trees[0], partial, self.trees[2]])

    def test_cli_exits_and_no_overwrite(self):
        output = self.work / "result.json"
        cmd = [sys.executable, str(SCRIPT), *map(str, self.trees), "--output", str(output)]
        self.assertEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
        original = output.read_bytes()
        self.assertTrue(json.loads(original)["all_bytes_equal"])
        self.assertEqual(subprocess.run(cmd, capture_output=True).returncode, 2)
        self.assertEqual(output.read_bytes(), original)
        output2 = self.work / "different.json"
        (self.trees[2] / "ConfigDumpInfo.xml").write_bytes(b"different")
        cmd[-1] = str(output2)
        self.assertEqual(subprocess.run(cmd, capture_output=True).returncode, 1)

    def test_cli_rejects_output_inside_tree(self):
        output = self.trees[0] / "report.json"
        cmd = [sys.executable, str(SCRIPT), *map(str, self.trees), "--output", str(output)]
        self.assertEqual(subprocess.run(cmd, capture_output=True).returncode, 2)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
