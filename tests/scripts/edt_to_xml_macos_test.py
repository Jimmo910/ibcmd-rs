"""Synthetic controls only; fixtures retained, never native/EDT acceptance."""
import contextlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts/edt-lab'))
spec = importlib.util.spec_from_file_location('edt_to_xml_macos', ROOT / 'scripts/edt-lab/edt_to_xml_macos.py')
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)


class NativeReferenceControls(unittest.TestCase):
    def setUp(self):
        # Never run this test with a deleting teardown or an internal-disk temp.
        if os.environ.get('IBCMD_TEST_KEEP_FIXTURES') != '1':
            self.fail('Set IBCMD_TEST_KEEP_FIXTURES=1; all synthetic fixtures are retained')
        temporary = Path(os.environ.get('TMPDIR', '')).absolute()
        harness.lab_path(temporary)
        retained = Path(tempfile.mkdtemp(prefix='issue356-controls-', dir=temporary))
        self.root = retained / 'Eclipse'
        self.root.mkdir()

    def fixture(self):
        project = self.root / 'project'
        for relative, content in [('.project', 'synthetic'), ('DT-INF/PROJECT.PMF', 'Runtime-Version: 8.5.1\n'),
                                  ('src/Configuration/Configuration.mdo', '<synthetic/>')]:
            path = project / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        tools = []
        for name in ('edt', 'ours', 'native'):
            path = self.root / name
            path.write_text('synthetic tool; never executed')
            path.chmod(0o700)
            tools.append(path)
        (self.root / 'configuration').mkdir()
        (self.root / 'configuration/config.ini').write_text('synthetic=true')
        (self.root / 'plugins').mkdir()
        (self.root / 'plugins/synthetic.model_1.0.jar').write_bytes(b'synthetic model')
        (self.root / '1cedt.ini').write_text('-startup\nsynthetic.jar\n-vmargs\n-Xmx1g\n')
        installation = self.root / 'installation.json'
        java = self.root / 'jdk/bin/java'
        java.parent.mkdir(parents=True)
        java.write_bytes(b'synthetic java')
        (self.root / 'jdk/release').write_bytes(b'synthetic runtime release')
        (self.root / '1cedtcli-startup.txt').write_text('synthetic startup')
        macos = self.root.parent / 'MacOS'
        macos.mkdir()
        (macos / '1cedt').write_bytes(b'synthetic launcher')
        bundles = self.root / 'configuration/org.eclipse.equinox.simpleconfigurator/bundles.info'
        bundles.parent.mkdir()
        bundles.write_text('synthetic bundles')
        # Each retained fixture has its own installation parent.
        plist = self.root.parent / 'Info.plist'
        plist.write_text('synthetic plist')
        rows = [{'path': str(p), 'bytes': p.stat().st_size, 'sha256': harness.digest(p)}
                for p in (tools[0], self.root / '1cedt.ini', self.root / 'configuration/config.ini',
                          self.root / '1cedtcli-startup.txt', plist, macos / '1cedt', bundles, java, self.root / 'jdk/release')]
        harness.write_json(installation, {'schema_version': 1, 'edt_cli': str(tools[0]),
            'edt_cli_sha256': harness.digest(tools[0]), 'plugins': harness.snapshot(self.root / 'plugins'),
            'runtime_files': rows})
        schema = self.root / 'schema.json'
        baseline = self.root / 'baseline.json'
        baseline.write_text('{"bundles": [{"symbolicName": "synthetic.model"}]}')
        jar = self.root / 'plugins/synthetic.model_1.0.jar'
        harness.write_json(schema, {'schema_version': 1, 'declared_installed_release': '2025.2.3+30',
            'baseline_sha256': harness.digest(baseline),
            'bundles': [{'symbolicName': 'synthetic.model', 'status': 'inspected',
                         'jar_sha256': harness.digest(jar), 'jar_bytes': jar.stat().st_size}]})
        return SimpleNamespace(corpus_label='synthetic control, never BSP', installation=installation, schema_evidence=schema, project=project, run=self.root / 'run', edt=tools[0], ours=tools[1], native=tools[2],
                               runtime='8.5.1', edt_build='2025.2.3.30', native_build='8.5.1.1592',
                               timeout=10, heap_gib=1, xml_version='2.21', source_profile='edt-2025.2.3-xml-2.21')

    def fake_command(self, args, calls, *, changed=False, mutation=None, failed=None):
        def command(run, label, argv, timeout):
            calls.append((label, [str(x) for x in argv]))
            (run / f'{label}.stdout').write_bytes(b'')
            (run / f'{label}.stderr').write_bytes(b'')
            harness.write_json(run / f'{label}.command.json', {'argv': [str(x) for x in argv],
                'exit_code': 1 if label == failed else 0, 'synthetic': True})
            if label == failed:
                raise harness.OracleError('synthetic command failure')
            if label.endswith('-version'):
                return {'native-version': b'8.5.1.1592', 'ours-version': b'ibcmd-rs synthetic',
                        'edt-version': args.edt_build.encode()}[label]
            if label in ('edt-export', 'native-export', 'convert'):
                directory = run / {'edt-export': 'edt-xml', 'native-export': 'native-xml', 'convert': 'ours-xml'}[label]
                directory.mkdir()
                (directory / 'Configuration.xml').write_bytes(b'<Configuration/>' if not (changed and label == 'convert') else b'<Changed/>')
                if label == 'native-export':
                    (directory / 'ConfigDumpInfo.xml').write_bytes(b'<storage-generation/>')
            if label == 'convert' and mutation:
                mutation(args, run)
            return b''
        return command

    def run_synthetic(self, args, **options):
        calls = []
        with patch.object(harness, 'MODEL_BASELINE', self.root / 'baseline.json'), patch.object(harness.sys, 'platform', 'darwin'), patch.object(harness, 'native_slot', lambda _: contextlib.nullcontext()), \
             patch.object(harness, 'command', side_effect=self.fake_command(args, calls, **options)):
            code = harness.capture(args)
        return code, calls

    def test_same_project_export_is_native_import_input_and_full_diffs_retained(self):
        args = self.fixture()
        code, calls = self.run_synthetic(args)
        self.assertEqual(code, 0)
        imported = dict(calls)['native-import']
        self.assertEqual(imported[-1], str(args.run / 'edt-xml'))
        self.assertNotIn(str(args.run / 'ours-xml'), imported)
        converted = dict(calls)['convert']
        self.assertEqual(converted[2], str(args.project))
        result = json.loads((args.run / 'result.json').read_text())
        self.assertEqual(result['status'], 'PASS')
        self.assertFalse(result['strict_equal'])
        report = json.loads((args.run / 'comparison.json').read_text())
        self.assertEqual(report['strict']['differing_rows'][0]['path'], 'ConfigDumpInfo.xml')
        self.assertIsNotNone(report['configuration_content']['original_excluded_rows'][0]['right']['sha256'])
        self.assertTrue((args.run / 'project-after.json').is_file())
        edt_argv = dict(calls)['edt-export']
        self.assertIn('-ini-file', edt_argv)
        self.assertNotIn('-configuration', edt_argv)
        self.assertIn(str(args.run / 'edt-configuration'), (args.run / '1cedt.ini').read_text())

    def test_compatibility_probe_never_claims_acceptance(self):
        args = self.fixture()
        args.compatibility_probe = True
        args.edt_build = '2026.2.1.3'
        schema = json.loads(args.schema_evidence.read_text())
        schema['declared_installed_release'] = '2026.2.1+3'
        args.schema_evidence.write_text(json.dumps(schema))
        self.run_synthetic(args)
        self.assertEqual(json.loads((args.run / 'result.json').read_text())['status'], 'COMPATIBILITY_PROBE_MATCH')

    def test_plugin_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'plugin inventory'):
            self.run_synthetic(args, mutation=lambda a, _: (a.edt.parent / 'plugins/synthetic.model_1.0.jar').write_bytes(b'changed'))
        self.assertFalse((args.run / 'result.json').exists())

    def test_java_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'Runtime inventory differs'):
            self.run_synthetic(args, mutation=lambda a, _: (a.edt.parent / 'jdk/bin/java').write_bytes(b'changed'))
        self.assertFalse((args.run / 'result.json').exists())

    def test_schema_evidence_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'Source project or executable changed'):
            self.run_synthetic(args, mutation=lambda a, _: a.schema_evidence.write_text('{}'))
        self.assertFalse((args.run / 'result.json').exists())

    def test_incomplete_installation_manifest_refused(self):
        args = self.fixture()
        data = json.loads(args.installation.read_text())
        data['runtime_files'] = [row for row in data['runtime_files'] if not row['path'].endswith('/java')]
        args.installation.write_text(json.dumps(data))
        with self.assertRaisesRegex(harness.OracleError, 'explicit Java'):
            self.run_synthetic(args)
        self.assertFalse((args.run / 'result.json').exists())

    def test_unbound_schema_evidence_refused(self):
        args = self.fixture()
        args.schema_evidence.write_text('{}')
        with self.assertRaisesRegex(harness.OracleError, 'Schema evidence release'):
            self.run_synthetic(args)
        self.assertFalse((args.run / 'result.json').exists())

    def test_wrong_schema_jar_hash_refused(self):
        args = self.fixture()
        schema = json.loads(args.schema_evidence.read_text())
        schema['bundles'][0]['jar_sha256'] = '0' * 64
        args.schema_evidence.write_text(json.dumps(schema))
        with self.assertRaisesRegex(harness.OracleError, 'Schema bundle hash'):
            self.run_synthetic(args)

    def test_any_content_difference_is_failure(self):
        args = self.fixture()
        code, _ = self.run_synthetic(args, changed=True)
        self.assertEqual(code, 1)
        report = json.loads((args.run / 'comparison.json').read_text())
        self.assertEqual(report['configuration_content']['differing_rows'][0]['path'], 'Configuration.xml')
        self.assertEqual(json.loads((args.run / 'result.json').read_text())['status'], 'FAIL')

    def test_source_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'Source project'):
            self.run_synthetic(args, mutation=lambda a, _: (a.project / '.project').write_text('mutated'))
        self.assertFalse((args.run / 'result.json').exists())
        self.assertFalse(json.loads((args.run / 'immutability.json').read_text())['project_unchanged'])

    def test_binary_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'executable changed'):
            self.run_synthetic(args, mutation=lambda a, _: a.ours.write_text('changed tool'))
        self.assertFalse((args.run / 'result.json').exists())

    def test_native_reference_mutation_cannot_leave_pass_result(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'Native reference changed'):
            self.run_synthetic(args, mutation=lambda _, r: (r / 'native-xml/Configuration.xml').write_text('changed'))
        self.assertFalse((args.run / 'result.json').exists())
        self.assertTrue((args.run / 'native-xml-after.json').is_file())

    def test_interruption_kills_only_started_group_and_retains_receipt(self):
        run = self.root / 'interrupted'
        run.mkdir(); (run / 'tmp').mkdir()
        from unittest.mock import Mock
        proc = Mock(pid=987654)
        proc.wait.side_effect = [KeyboardInterrupt(), -9]
        with patch.object(harness.subprocess, 'Popen', return_value=proc), patch.object(harness.os, 'killpg') as kill:
            with self.assertRaises(KeyboardInterrupt):
                harness.command(run, 'interrupted', ['synthetic'], 1)
        kill.assert_called_once_with(987654, harness.signal.SIGKILL)
        record = json.loads((run / 'interrupted.command.json').read_text())
        self.assertTrue(record['interrupted'])
        self.assertEqual(record['exit_code'], -9)

    def test_native_failure_preserves_evidence_and_does_not_convert(self):
        args = self.fixture()
        with self.assertRaisesRegex(harness.OracleError, 'synthetic command failure'):
            self.run_synthetic(args, failed='native-import')
        self.assertTrue((args.run / 'failure.json').is_file())
        self.assertTrue((args.run / 'project-after.json').is_file())
        self.assertFalse((args.run / 'convert.command.json').exists())
        self.assertFalse((args.run / 'result.json').exists())

    def test_existing_run_provenance_and_overlap_refused(self):
        args = self.fixture()
        args.run.mkdir()
        with self.assertRaisesRegex(harness.OracleError, 'fresh'):
            self.run_synthetic(args)
        args.run = self.root / 'unused'
        provenance = args.project / '.ibcmd-provenance'
        provenance.mkdir()
        with self.assertRaisesRegex(harness.OracleError, 'provenance'):
            self.run_synthetic(args)
        with self.assertRaisesRegex(harness.OracleError, 'Overlapping'):
            harness.disjoint([args.project, args.project / 'output'])

    def test_internal_disk_and_parent_traversal_refused(self):
        for path in (Path('/tmp/issue356'), harness.LAB / 'tmp/../private'):
            with self.assertRaises(harness.OracleError):
                harness.lab_path(path)

    def test_symlink_source_refused(self):
        original = self.root / 'original'
        original.mkdir()
        link = self.root / 'link'
        link.symlink_to(original, target_is_directory=True)
        with self.assertRaisesRegex(harness.OracleError, 'Symlink'):
            harness.lab_path(link)

    def test_comparator_reports_extra_missing_and_changed_bytes(self):
        left, right = self.root / 'left', self.root / 'right'
        left.mkdir(); right.mkdir()
        for name, data in [('same', b'x'), ('changed', b'a'), ('extra', b'e')]:
            (left / name).write_bytes(data)
        for name, data in [('same', b'x'), ('changed', b'b'), ('missing', b'm')]:
            (right / name).write_bytes(data)
        report = harness.comparison(harness.snapshot(left), harness.snapshot(right))
        self.assertEqual([x['path'] for x in report['strict']['differing_rows']], ['changed', 'extra', 'missing'])
        self.assertFalse(report['configuration_content']['equal'])

    def test_edt_workspace_error_logs_are_captured_without_suppression(self):
        run = self.root / 'workspace-control'
        run.mkdir(); (run / 'tmp').mkdir()
        metadata = run / 'workspace/.metadata'
        metadata.mkdir(parents=True)
        raw = b'!ENTRY synthetic.plugin 4 0 2026-10-23\n!MESSAGE retained error\n'
        (metadata / '.log').write_bytes(raw)
        harness.command(run, 'edt', [sys.executable, '-c', 'pass', '-data', str(metadata.parent)], 5)
        self.assertEqual((run / 'edt.workspace-log').read_bytes(), raw)
        report = harness.command_diagnostics(run, ['edt'])
        self.assertEqual(report['edt']['workspace_error_count'], 1)
        self.assertTrue(harness.has_error_diagnostics(report.values()))

    def test_command_failure_records_raw_streams_and_exit(self):
        run = self.root / 'command'
        run.mkdir(); (run / 'tmp').mkdir()
        with self.assertRaises(harness.OracleError):
            harness.command(run, 'bad', [sys.executable, '-c', 'import sys; print("raw"); sys.exit(7)'], 5)
        self.assertEqual((run / 'bad.stdout').read_bytes(), b'raw\n')
        self.assertEqual(json.loads((run / 'bad.command.json').read_text())['exit_code'], 7)


if __name__ == '__main__':
    unittest.main()
