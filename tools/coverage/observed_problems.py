"""Validate exact native witnesses and distinguish known symptoms from regressions.

The corpus owns native inputs. This consumer-owned ledger describes converter
diagnostics; a missing widget/file never substitutes for one missing property.
"""
import argparse
import base64
import collections
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET


def canonical(element):
    if element is None:
        return None
    value = element.text or ''
    # Leaf whitespace belongs to the value (for example an input mask).
    # Indentation preceding child elements is XML formatting only.
    if len(element) and not value.strip():
        value = ''
    return [element.tag, sorted(element.attrib.items()), value,
            [canonical(child) for child in element]]


def condition(root, case):
    anchors = [node for node in root.iter()
               if node.tag == case['anchor']['tag']
               and node.get('name') == case['anchor']['name']]
    if len(anchors) != 1:
        raise ValueError(f"{case['id']}: expected one intact widget, got {len(anchors)}")
    children = [node for node in anchors[0] if node.tag == case['property']]
    if len(children) > 1:
        raise ValueError(f"{case['id']}: repeated property")
    # JSON uses arrays for attribute pairs; normalize tuples before comparing.
    return json.loads(json.dumps(canonical(children[0] if children else None)))


def classify(native, ours, case):
    if condition(native, case) != case['native']:
        raise ValueError(f"{case['id']}: native no longer retains the witness")
    observed = condition(ours, case)
    if observed == case['native']:
        return 'equal'
    if observed == case['known_symptom']:
        return 'known-symptom'
    raise ValueError(f"{case['id']}: an unrelated mismatch replaced the known symptom")


def validate_cases(corpus, ledger):
    if ledger['schema_version'] != 1 or not ledger['cases']:
        raise ValueError('invalid or empty observed-problem ledger')
    ids = set()
    root = corpus.resolve()
    for case in ledger['cases']:
        if case['id'] in ids or case['native'] == case['known_symptom']:
            raise ValueError('duplicate case or indistinguishable symptom')
        ids.add(case['id'])
        source = (corpus / 'coverage' / case['set']).resolve()
        if not source.is_relative_to(root):
            raise ValueError('case escapes the pinned corpus')
        manifest = json.loads((source / 'manifest.json').read_text(encoding='utf-8'))
        if manifest.get('evidence', {}).get('status') != 'platform_proven':
            raise ValueError(f"{case['id']}: source lacks native proof")
        files = [item for variant in manifest['variants']
                 if variant['status'] == 'platform_proven' and variant['name'] == case['variant']
                 for item in variant['files'] if item['path'] == case['file']]
        target = (source / 'native' / case['file']).resolve()
        if not target.is_relative_to(source / 'native') or len(files) != 1:
            raise ValueError(f"{case['id']}: witness is not owned by the proven variant")
        data = target.read_bytes()
        if hashlib.sha256(data).hexdigest() != files[0]['sha256']:
            raise ValueError(f"{case['id']}: native file hash differs")
        if condition(ET.fromstring(data), case) != case['native']:
            raise ValueError(f"{case['id']}: native condition is not present")


def run(corpus, ledger, binary):
    validate_cases(corpus, ledger)
    counts = collections.Counter()
    grouped = collections.defaultdict(list)
    for case in ledger['cases']:
        grouped[case['set']].append(case)
    with tempfile.TemporaryDirectory(prefix='ibcmd-observed-problems-') as tmp:
        for index, (name, cases) in enumerate(sorted(grouped.items())):
            source = corpus / 'coverage' / name
            manifest = json.loads((source / 'manifest.json').read_text(encoding='utf-8'))
            cf = Path(tmp) / f'{index}.cf'
            raw = base64.b64decode((source / 'configuration.cf.b64').read_bytes())
            if hashlib.sha256(raw).hexdigest() != manifest['configuration_cf']['decoded_sha256']:
                raise ValueError(f'{name}: CF hash differs')
            cf.write_bytes(raw)
            output = Path(tmp) / str(index)
            completed = subprocess.run([str(binary), 'cf', 'export', str(cf), str(output),
                                        '--source-version', manifest['evidence']['source_version']],
                                       capture_output=True, env=dict(os.environ, PATH='', LANG='ru_RU.UTF-8'))
            if completed.returncode:
                raise ValueError(f'{name}: exporter refused the witness set: {completed.stderr.decode(errors="replace")[-1500:]}')
            trees = {}
            for case in cases:
                file = case['file']
                if file not in trees:
                    # Missing files fail here rather than being called a symptom.
                    trees[file] = (ET.parse(source / 'native' / file).getroot(),
                                   ET.parse(output / file).getroot())
                counts[classify(*trees[file], case)] += 1
    print('Observed problem witnesses: ' + ', '.join(f'{key}={value}' for key, value in sorted(counts.items())))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus', type=Path, required=True)
    parser.add_argument('--ledger', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    run(args.corpus, json.loads(args.ledger.read_text(encoding='utf-8')), args.binary)
