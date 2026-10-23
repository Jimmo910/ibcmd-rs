"""Synthetic journal tests; no native tools/SQL or destructive teardown."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[2] / 'scripts/platform-lab/acceptance.py'
spec = importlib.util.spec_from_file_location('platform_acceptance', SOURCE)
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class Acceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if os.environ.get('IBCMD_TEST_KEEP_FIXTURES') != '1':
            raise RuntimeError('Set IBCMD_TEST_KEEP_FIXTURES=1; fixtures are preserved')
        cls.root = Path(tempfile.mkdtemp(prefix='issue361-synthetic-', dir=os.environ['TMPDIR']))
        cls.counter = 0

    def ref(self, value):
        type(self).counter += 1
        path = self.root / f'{self.counter}.json'
        path.write_text(json.dumps(value) if not isinstance(value, str) else value)
        return {'path': path.name, 'sha256': audit.digest(path)}

    def row(self, release):
        build = release + '.1234'
        blank = self.ref('')
        snapshot = self.ref({'synthetic': True})
        probe = self.ref({'platform_build': build, 'binary_sha256': 'a' * 64, 'exit_code': 0,
                          'stdout': self.ref('ibcmd ' + build), 'stderr': blank})
        empty = self.ref({'engine': 'mssql', 'platform_build': build,
                          'native_target_id': 'native', 'candidate_target_id': 'candidate',
                          'native_empty': True, 'candidate_empty': True,
                          'native_inventory': snapshot, 'candidate_inventory': snapshot})
        row = {'release': release, 'state': 'measured', 'platform_build': build,
               'native': {'binary_sha256': 'a' * 64, 'version_probe': probe},
               'candidate': {'binary_sha256': 'b' * 64, 'git_commit': 'c' * 40, 'clean_worktree': True},
               'corpus': {'id': 'synthetic', 'version': '1', 'origin': 'unit test',
                          'authorization': 'synthetic only', 'snapshot': snapshot},
               'database': {'engine': 'mssql', 'source_id': 'source', 'native_target_id': 'native',
                            'candidate_target_id': 'candidate', 'source_before': snapshot,
                            'source_after': snapshot, 'empty_targets_proof': empty},
               'observations': {'unknown_layouts': [], 'write_limits': ['no measured production writes'],
                                'layout_census': snapshot, 'write_policy': snapshot},
               'phases': {}, 'comparisons': {}}
        records = {}
        for phase in audit.PHASES:
            database_id = ('source' if phase.endswith('_source') else
                           'native' if phase in ('native_import_empty', 'native_reexport_native_target') else 'candidate')
            record = {
                'phase': phase, 'platform_build': build, 'candidate_commit': 'c' * 40,
                'binary_sha256': ('b' if phase.startswith('candidate_') else 'a') * 64,
                'corpus_snapshot_sha256': snapshot['sha256'], 'exit_code': 0,
                'argv_redacted': ['synthetic'], 'started_utc': '2026-10-23T00:00:00Z',
                'ended_utc': '2026-10-23T00:00:01Z', 'stdout': blank, 'stderr': blank,
                'output_inventory': self.ref({'phase': phase, 'synthetic': True}),
                'database_id': database_id}
            records[phase] = record
        for phase, record in records.items():
            if phase.endswith('_source'):
                input_hash = snapshot['sha256']
            elif phase.endswith('_empty'):
                input_hash = records['native_export_source']['output_inventory']['sha256']
            else:
                import_phase = ('native_import_empty' if phase == 'native_reexport_native_target' else 'candidate_import_empty')
                input_hash = records[import_phase]['output_inventory']['sha256']
            record['input_inventory_sha256'] = input_hash
            row['phases'][phase] = self.ref(record)
        summary = {'left_only': 0, 'right_only': 0, 'different': 0, 'unchanged': 1}
        diff = self.ref({'summary': summary, 'differences': [{'path': 'Configuration.xml', 'status': 'unchanged'}]})
        for kind in audit.COMPARISONS:
            sides = (('native_export_source', 'candidate_export_source') if kind == 'export' else
                     ('native_reexport_native_target', 'native_reexport_candidate_target'))
            row['comparisons'][kind] = self.ref({
                'scope': 'full', 'platform_build': build, 'candidate_commit': 'c' * 40,
                'corpus_snapshot_sha256': snapshot['sha256'], 'full_diff': diff, 'summary': summary,
                'left_inventory_sha256': records[sides[0]]['output_inventory']['sha256'],
                'right_inventory_sha256': records[sides[1]]['output_inventory']['sha256']})
        return row

    def document(self):
        return {'schema_version': 1, 'rows': [self.row(release) for release in audit.RELEASES]}

    def rewrite(self, ref, **changes):
        value = audit.load(self.root / ref['path'])
        value.update(changes)
        return self.ref(value)

    def test_complete_synthetic_journals_only_become_ready_for_review(self):
        result = audit.check(self.document(), self.root)
        self.assertEqual(result['state'], 'ready_for_review')
        self.assertEqual(result['native_acceptance'], 'not_determined')

    def test_differences_and_limits_remain_visible_for_review(self):
        row = self.row('8.3.24')
        row['observations']['unknown_layouts'] = ['unrecognized catalog layout']
        ref = row['comparisons']['export']
        report = audit.load(self.root / ref['path'])
        summary = {'left_only': 0, 'right_only': 0, 'different': 1, 'unchanged': 0}
        diff = self.ref({'summary': summary, 'differences': [{'path': 'Configuration.xml', 'status': 'different'}]})
        row['comparisons']['export'] = self.rewrite(ref, summary=summary, full_diff=diff)
        result = audit.check_row(row, self.root)
        self.assertEqual(result['differences'], 1)
        self.assertEqual(result['unknown_layouts'], row['observations']['unknown_layouts'])
        self.assertEqual(result['write_limits'], row['observations']['write_limits'])

    def test_missing_release_or_duplicate_is_rejected(self):
        doc = self.document()
        doc['rows'][1] = copy.deepcopy(doc['rows'][0])
        with self.assertRaisesRegex(audit.EvidenceError, 'one row per'):
            audit.check(doc, self.root)
        doc['rows'].pop()
        with self.assertRaisesRegex(audit.EvidenceError, 'three release'):
            audit.check(doc, self.root)

    def test_nearby_build_and_mismatched_exact_probe_are_rejected(self):
        row = self.row('8.3.24')
        row['platform_build'] = '8.3.27.2214'
        with self.assertRaisesRegex(audit.EvidenceError, 'does not match'):
            audit.check_row(row, self.root)
        row['platform_build'] = '8.3.24.9999'
        with self.assertRaisesRegex(audit.EvidenceError, 'exact binary/build'):
            audit.check_row(row, self.root)

    def test_wrong_probe_stdout_is_rejected_even_with_valid_hash(self):
        row = self.row('8.3.24')
        row['native']['version_probe'] = self.rewrite(row['native']['version_probe'], stdout=self.ref('8.3.24.9999'))
        with self.assertRaisesRegex(audit.EvidenceError, 'one exact build'):
            audit.check_row(row, self.root)

    def test_native_import_and_native_reexport_are_mandatory(self):
        for phase in ('native_import_empty', 'native_reexport_candidate_target'):
            row = self.row('8.3.24')
            del row['phases'][phase]
            with self.assertRaisesRegex(audit.EvidenceError, 'incomplete'):
                audit.check_row(row, self.root)

    def test_unrelated_diff_database_or_input_is_rejected(self):
        row = self.row('8.3.24')
        row['comparisons']['import'] = row['comparisons']['export']
        with self.assertRaisesRegex(audit.EvidenceError, 'output binding'):
            audit.check_row(row, self.root)
        for changes in ({'database_id': 'unrelated'}, {'input_inventory_sha256': 'f' * 64}):
            row = self.row('8.3.24')
            phase = 'candidate_import_empty'
            row['phases'][phase] = self.rewrite(row['phases'][phase], **changes)
            with self.assertRaises(audit.EvidenceError):
                audit.check_row(row, self.root)
        row = self.row('8.3.24')
        row['database']['empty_targets_proof'] = self.rewrite(
            row['database']['empty_targets_proof'], candidate_target_id='unrelated')
        with self.assertRaisesRegex(audit.EvidenceError, 'both fresh targets'):
            audit.check_row(row, self.root)

    def test_wrong_tool_commit_snapshot_or_failed_phase_is_rejected(self):
        for changes in ({'binary_sha256': 'd' * 64}, {'candidate_commit': 'd' * 40},
                        {'corpus_snapshot_sha256': 'd' * 64}, {'exit_code': 1}, {'exit_code': False}):
            row = self.row('8.3.25')
            phase = 'candidate_import_empty'
            row['phases'][phase] = self.rewrite(row['phases'][phase], **changes)
            with self.assertRaises(audit.EvidenceError):
                audit.check_row(row, self.root)

    def test_full_scope_and_full_diff_counts_are_required(self):
        for changes in ({'scope': 'scoped'}, {'summary': {'left_only': 0, 'right_only': 0, 'different': 0, 'unchanged': 2}},
                        {'full_diff': self.ref({'summary': {}, 'differences': []})}):
            row = self.row('8.3.26')
            row['comparisons']['export'] = self.rewrite(row['comparisons']['export'], **changes)
            with self.assertRaises(audit.EvidenceError):
                audit.check_row(row, self.root)

    def test_changed_source_shared_targets_and_missing_limits_are_rejected(self):
        for change in ('source', 'target', 'limits'):
            row = self.row('8.3.24')
            if change == 'source':
                row['database']['source_after'] = self.ref('changed')
            elif change == 'target':
                row['database']['candidate_target_id'] = 'native'
            else:
                del row['observations']['write_limits']
            with self.assertRaises(audit.EvidenceError):
                audit.check_row(row, self.root)

    def test_missing_tampered_and_escaping_evidence_are_rejected(self):
        for ref in ({'path': '../outside', 'sha256': 'a' * 64},
                    {'path': '/outside', 'sha256': 'a' * 64},
                    {'path': 'missing', 'sha256': 'a' * 64},
                    {**self.ref('data'), 'sha256': 'a' * 64}):
            with self.assertRaises(audit.EvidenceError):
                audit.artifact(self.root, ref)

    def test_symlink_and_duplicate_keys_are_rejected(self):
        target = self.ref('data')
        link = self.root / f'link-{self.counter}'
        link.symlink_to(self.root / target['path'])
        with self.assertRaisesRegex(audit.EvidenceError, 'symlink'):
            audit.artifact(self.root, {**target, 'path': link.name})
        ref = self.ref('{"state":"blocked","state":"measured"}')
        with self.assertRaisesRegex(audit.EvidenceError, 'duplicate JSON key'):
            audit.load(self.root / ref['path'])

    def test_malformed_nested_objects_have_controlled_errors(self):
        for key in ('native', 'candidate', 'corpus', 'database', 'observations', 'phases', 'comparisons'):
            row = self.row('8.3.24')
            row[key] = None
            with self.assertRaises(audit.EvidenceError):
                audit.check_row(row, self.root)
        row = self.row('8.3.24')
        report_ref = row['comparisons']['export']
        report = audit.load(self.root / report_ref['path'])
        malformed = self.rewrite(report['full_diff'], differences=[None])
        row['comparisons']['export'] = self.rewrite(report_ref, full_diff=malformed)
        with self.assertRaisesRegex(audit.EvidenceError, 'invalid full diff entry'):
            audit.check_row(row, self.root)

    def test_blockers_remain_blocked_without_evidence(self):
        doc = {'schema_version': 1, 'rows': [{'release': release, 'state': 'blocked',
               'blockers': ['exact binary and SQL unavailable']} for release in audit.RELEASES]}
        self.assertEqual(audit.check(doc, self.root)['state'], 'blocked')
        doc['rows'][0]['blockers'] = []
        with self.assertRaises(audit.EvidenceError):
            audit.check(doc, self.root)

    def test_inventory_does_not_promote_directory_names_to_versions(self):
        root = self.root / f'inventory-{self.counter}'
        directory = root / '8.3.24.1819'
        directory.mkdir(parents=True)
        (directory / 'ibcmd').write_text('synthetic binary')
        result = audit.inventory(root)
        self.assertFalse(result['installed'][0]['runtime_version_verified'])
        self.assertEqual(result['native_acceptance'], 'not_measured')


if __name__ == '__main__':
    unittest.main()
