"""Pure in-memory controls: no fixture extraction, teardown, or installed EDT."""
import importlib.util
import io
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
import zipfile

spec = importlib.util.spec_from_file_location("model_inventory", Path(__file__).with_name("model_inventory.py"))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class BundleInventory(unittest.TestCase):
    def expected(self):
        return {"symbolicName": "example.model", "version": "1.0",
                "modelTypes": ["example.Present", "example.Missing"],
                "importers": [], "exporters": ["example.Present"]}

    def jar(self, manifest=None, entries=None):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as archive:
            archive.writestr("META-INF/MANIFEST.MF", manifest or
                b"Manifest-Version: 1.0\r\nBundle-SymbolicName: example.model;\r\n singleton:=true\r\nBundle-Version: 2.0\r\n\r\n")
            for name, raw in (entries or {"example/Present.class": b"bytecode", "model/model.ecore": b"model"}).items():
                archive.writestr(name, raw)
        data.seek(0)
        return data

    def test_all_categories_and_resource_bytes_accounted(self):
        row = m.inspect_bundle(self.jar(), self.expected())
        self.assertEqual(row["installed_version"], "2.0")
        self.assertEqual(row["baseline_class_delta"]["modelTypes"],
                         {"baseline_count": 2, "present_count": 1, "missing": ["example.Missing"]})
        self.assertEqual(row["baseline_class_delta"]["exporters"]["present_count"], 1)
        self.assertEqual(row["model_resources"],
                         [{"path": "model/model.ecore", "bytes": 5, "sha256": m.digest(b"model")}])

    def test_changed_resource_bytes_change_digest(self):
        one = m.inspect_bundle(self.jar(), self.expected())
        two = m.inspect_bundle(self.jar(entries={"example/Present.class": b"bytecode", "model/model.ecore": b"new"}), self.expected())
        self.assertNotEqual(one["model_resources"], two["model_resources"])

    def test_identity_mismatch_rejected(self):
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            m.inspect_bundle(self.jar(manifest=b"Bundle-SymbolicName: other\nBundle-Version: 2\n"), self.expected())

    def test_manifest_duplicate_and_orphan_rejected(self):
        for raw in [b"Bundle-Version: 1\nBundle-Version: 2\n", b" continuation\n"]:
            with self.assertRaises(ValueError):
                m.manifest_fields(raw)

    def test_named_manifest_sections_do_not_replace_main_identity(self):
        fields = m.manifest_fields(b"Bundle-Version: 1\n\nName: example\nBundle-Version: 2\n")
        self.assertEqual(fields, {"Bundle-Version": "1"})

    def test_duplicate_baseline_class_rejected(self):
        expected = self.expected()
        expected["modelTypes"].append("example.Present")
        with self.assertRaisesRegex(ValueError, "duplicate baseline"):
            m.inspect_bundle(self.jar(), expected)

    def test_missing_and_ambiguous_bundles_are_explicit(self):
        plugins = Mock()
        plugins.glob.return_value = []
        baseline = {"source": {"release": "legacy"}, "bundles": [self.expected()]}
        missing = m.inventory(plugins, baseline, "current")
        self.assertEqual(missing["summary"]["inspected_bundles"], 0)
        self.assertEqual(missing["bundles"][0]["candidate_count"], 0)
        self.assertEqual(missing["semantic_compatibility"], "NOT_ESTABLISHED")
        plugins.glob.return_value = [Path("one.jar"), Path("two.jar")]
        ambiguous = m.inventory(plugins, baseline, "current")
        self.assertEqual(ambiguous["bundles"][0]["candidate_count"], 2)
        self.assertEqual(ambiguous["summary"]["baseline_bundles"], 1)

    def test_duplicate_baseline_bundle_rejected(self):
        plugins = Mock()
        plugins.glob.return_value = []
        with self.assertRaisesRegex(ValueError, "duplicate baseline bundle"):
            m.inventory(plugins, {"source": {}, "bundles": [self.expected(), self.expected()]}, "current")

    def test_internal_relative_and_traversing_output_rejected(self):
        for path, root in [("/tmp/result.json", "/tmp/lab"),
                           ("result.json", "/Volumes/Work/lab"),
                           ("/Volumes/Work/lab/../result.json", "/Volumes/Work/lab"),
                           ("/Volumes/Other/result.json", "/Volumes/Work/lab")]:
            with self.assertRaises(ValueError):
                m.external_output(Path(path), Path(root))

    def test_resource_streaming_limit_rejected(self):
        with self.assertRaisesRegex(ValueError, "byte limit"):
            m.stream_digest(io.BytesIO(b"12345"), 4)
        self.assertEqual(m.stream_digest(io.BytesIO(b"12345"), 5), (5, m.digest(b"12345")))

    def test_oversized_manifest_rejected(self):
        with self.assertRaisesRegex(ValueError, "manifest exceeds"):
            m.inspect_bundle(self.jar(manifest=b"x" * (m.MANIFEST_LIMIT + 1)), self.expected())

    def test_large_signed_manifest_tail_does_not_change_main_identity(self):
        manifest = b"Bundle-SymbolicName: example.model\nBundle-Version: 2\n\nName: file\nDigest: " + b"x" * (m.MANIFEST_LIMIT * 2)
        row = m.inspect_bundle(self.jar(manifest=manifest), self.expected())
        self.assertEqual(row["installed_version"], "2")

    def test_candidate_open_cannot_block_on_fifo(self):
        plugins = Mock()
        plugins.glob.return_value = [Path("example.model_1.jar")]
        with patch.object(m.os, "open", side_effect=OSError("stop before descriptor ownership")) as opened:
            with self.assertRaises(OSError):
                m.inventory(plugins, {"source": {}, "bundles": [self.expected()]}, "current")
        flags = opened.call_args.args[1]
        self.assertTrue(flags & m.os.O_NONBLOCK)
        self.assertTrue(flags & m.os.O_NOFOLLOW)

    def test_empty_version_rejected(self):
        with self.assertRaisesRegex(ValueError, "empty bundle version"):
            m.inspect_bundle(self.jar(manifest=b"Bundle-SymbolicName: example.model\nBundle-Version: \n"), self.expected())


if __name__ == "__main__":
    unittest.main()
