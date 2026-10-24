"""Lane selection of the offline matrix (scripts/ci_scope.py)."""
import importlib.util
from pathlib import Path
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[2] / "scripts/ci_scope.py"
spec = importlib.util.spec_from_file_location("ci_scope", SCRIPT)
ci_scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci_scope)

ALL = dict(rust=True, crates=True, lib=True)


class ScopeTests(unittest.TestCase):
    def scope(self, files, referenced=()):
        with mock.patch.object(ci_scope, "referenced", lambda path: path in referenced):
            return ci_scope.scope(files)

    def test_unreferenced_documents_and_the_editor_skip_rust(self):
        self.assertEqual(
            self.scope(["README.md", "docs/guide.md", "editors/vscode/src/a.ts"]),
            dict(rust=False, crates=False, lib=False),
        )

    def test_a_document_a_test_reads_runs_everything(self):
        self.assertEqual(self.scope(["docs/HISTORY.md"], {"docs/HISTORY.md"}), ALL)

    def test_root_source_skips_the_crates(self):
        self.assertEqual(
            self.scope(["src/mcp.rs", "tests/cf_load.rs", "README.md"]),
            dict(rust=True, crates=False, lib=True),
        )

    def test_integration_suites_skip_the_crates_and_the_library(self):
        self.assertEqual(
            self.scope(["tests/cf_load.rs", "tests/support/elf_debug.rs", "tests/scripts/x.py"]),
            dict(rust=True, crates=False, lib=False),
        )

    def test_shared_test_code_the_library_includes_runs_the_library(self):
        self.assertEqual(
            self.scope(["tests/common/mcp_export_fixture.rs"]),
            dict(rust=True, crates=False, lib=True),
        )

    def test_fixtures_profiles_and_manifests_run_everything(self):
        for path in ["tests/fixtures/cf/a.b64", "profiles/xml/2.20.json", "Cargo.lock",
                     "crates/ibcmd-xml/src/lib.rs", ".github/workflows/ci.yml",
                     "rust-toolchain.toml", "xtask/src/main.rs", "docs/evidence/a.json"]:
            self.assertEqual(self.scope([path]), ALL, path)

    def test_nested_test_directories_are_not_suites(self):
        self.assertEqual(self.scope(["tests/fixtures/x.rs"]), ALL)

    def test_no_change_runs_nothing(self):
        self.assertEqual(self.scope([]), dict(rust=False, crates=False, lib=False))

    def test_reference_lookup_finds_a_cited_document(self):
        # profiles cite evidence documents; a real lookup against the tree.
        self.assertTrue(ci_scope.referenced("docs/HISTORY.md"))
        # Built at run time: a literal path here would be a reference itself.
        missing = "docs/" + "no-such-document-for-ci-scope.md"
        self.assertFalse(ci_scope.referenced(missing))


if __name__ == "__main__":
    unittest.main()
