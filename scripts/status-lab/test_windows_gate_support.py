"""Pure bounded-log/artifact controls; no peer processes or Windows emulation."""
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

import rpc_probe as probe
import windows_gate_support as support


class GateSupport(unittest.TestCase):
    def root(self):
        return probe.create_fixture_root().root
    def test_log_has_exact_byte_cap_and_overflow_marker(self):
        root = self.root()
        log = support.CappedLog(root / "capped.log", cap=5)
        try:
            log.write("abc")
            log.write("defghi")
            log.flush()
            self.assertTrue(log.overflow)
            self.assertEqual(log.size, 5)
        finally:
            log.close()
        self.assertEqual((root / "capped.log").read_bytes(), b"abcde")
    def test_artifact_inventory_counts_real_owned_bytes(self):
        root = self.root(); (root / "nested").mkdir()
        (root / "nested" / "fake.txt").write_bytes(b"abc")
        self.assertEqual(support.artifact_inventory(root), {"files": 1, "bytes": 3, "entries": 2})
    def test_single_file_cap_refuses_without_reading_contents(self):
        root = self.root()
        with (root / "oversized-public-fake").open("xb") as out:
            out.truncate((1 << 20) + 1)
        with self.assertRaisesRegex(ValueError, "artifact-boundary"):
            support.artifact_inventory(root)
    def test_entry_cap_refuses_many_empty_directories(self):
        root = self.root()
        for i in range(257): (root / str(i)).mkdir()
        with self.assertRaisesRegex(ValueError, "artifact-boundary"):
            support.artifact_inventory(root)
    def test_committed_manifest_binds_all_executed_sources(self):
        result = support.verified_sources()
        self.assertEqual(set(result), support.SOURCE_NAMES)
    def test_failed_final_admission_never_mints_upload_marker(self):
        root = self.root()
        with patch("windows_gate_support.artifact_inventory", side_effect=[{"files": 128, "bytes": 4 << 20, "entries": 256}, ValueError("artifact-cap")]):
            with self.assertRaisesRegex(ValueError, "artifact-cap"):
                support.admit_artifacts(root)
        self.assertTrue((root / "artifact-pending.json").is_file())
        self.assertFalse((root / "artifact-safe.json").exists())
    def test_marker_is_promoted_only_after_final_admission(self):
        root = self.root()
        (root / "public.txt").write_bytes(b"abc")
        support.admit_artifacts(root)
        self.assertTrue((root / "artifact-safe.json").is_file())
        self.assertFalse((root / "artifact-pending.json").exists())
        self.assertEqual(support.artifact_inventory(root)["files"], 2)
    def test_stale_source_hash_refuses(self):
        real = support.json.loads
        def stale(data):
            manifest = real(data)
            manifest["sha256"]["windows_rpc_gate.py"] = "0" * 64
            return manifest
        with patch("windows_gate_support.json.loads", side_effect=stale):
            with self.assertRaisesRegex(ValueError, "source-binding"):
                support.verified_sources()


if __name__ == "__main__":
    unittest.main()
