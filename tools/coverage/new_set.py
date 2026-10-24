#!/usr/bin/env python3
"""Creates the committed inputs of a new coverage set.

    new_set.py --ibcmd <ibcmd> --collection Documents --corpus <native tree> \
        --corpus-name <name> --set tests/fixtures/coverage/<platform>/<set> --work <scratch dir>

1. `input/base/`: the native export of a new empty infobase, without
   `ConfigDumpInfo.xml`.
2. `input/template.xml`: an object named `CovTemplate` the platform writes from
   a minimal seed (name, empty synonym and comment, and the generated types an
   object of the same class in the corpus declares, with fresh identifiers).
3. `input/values.json`: `observe_values.py` over the corpus.

Then `generate.py` and `native_round_trip.py` prove the set.
"""

import argparse
import pathlib
import re
import shutil
import subprocess
import sys
import uuid

HERE = pathlib.Path(__file__).parent
sys.path.insert(0, str(HERE))
from generate import NAMESPACE  # noqa: E402
from native_round_trip import create_from_tree, export, first_error, run_ibcmd  # noqa: E402

TEMPLATE = "CovTemplate"


def seed_object(corpus_object):
    """A minimal object of the corpus object's class, as XML."""
    text = corpus_object.read_text(encoding="utf-8-sig")
    head = text.split("\n")[1]
    tag = re.search(r"<MetaDataObject[^>]*>\s*<(\w+) uuid=", text).group(1)
    generated = re.findall(r'<xr:GeneratedType name="(\w+)\.[^"]+" category="(\w+)"', text)
    ids = (str(uuid.uuid5(NAMESPACE, f"{TEMPLATE}-{tag}/{index}")) for index in range(100))
    # The same envelope as the corpus object: generated types and a children
    # list only where its class has them.
    internal = ""
    if generated:
        types = "".join(
            f'\t\t\t<xr:GeneratedType name="{prefix}.{TEMPLATE}" category="{category}">\n'
            f"\t\t\t\t<xr:TypeId>{next(ids)}</xr:TypeId>\n"
            f"\t\t\t\t<xr:ValueId>{next(ids)}</xr:ValueId>\n"
            "\t\t\t</xr:GeneratedType>\n"
            for prefix, category in generated
        )
        internal = f"\t\t<InternalInfo>\n{types}\t\t</InternalInfo>\n"
    children = "\t\t<ChildObjects/>\n" if re.search(r"<ChildObjects\s*/?>", text) else ""
    return tag, (
        f'\ufeff<?xml version="1.0" encoding="UTF-8"?>\n{head}\n'
        f'\t<{tag} uuid="{next(ids)}">\n{internal}'
        f"\t\t<Properties>\n\t\t\t<Name>{TEMPLATE}</Name>\n\t\t\t<Synonym/>\n\t\t\t<Comment/>\n"
        f"\t\t</Properties>\n{children}\t</{tag}>\n</MetaDataObject>"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--ibcmd", required=True)
    parser.add_argument("--collection", required=True)
    parser.add_argument("--corpus", required=True, type=pathlib.Path)
    parser.add_argument("--corpus-name", required=True)
    parser.add_argument("--set", required=True, type=pathlib.Path)
    parser.add_argument("--work", required=True, type=pathlib.Path)
    parser.add_argument("--locale", default="ru_RU")
    args = parser.parse_args()

    corpus_objects = sorted((args.corpus / args.collection).glob("*.xml"))
    if not corpus_objects:
        raise SystemExit(f"the corpus has no {args.collection}")
    work = args.work
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)

    # 1. An empty configuration, as the platform writes it.
    ok, output = run_ibcmd(
        args, args.ibcmd, work, "infobase", "create", f"--db-path={work / 'empty'}", "--create-database",
        f"--locale={args.locale}",
    )
    if not ok:
        raise SystemExit(f"empty infobase: {first_error(output)}")
    export(args, work, work / "empty", work / "empty-xml")
    inputs = args.set / "input"
    if inputs.exists():
        shutil.rmtree(inputs)
    shutil.copytree(work / "empty-xml", inputs / "base", ignore=shutil.ignore_patterns("ConfigDumpInfo.xml"))

    # 2. The template: a minimal seed the platform completes.
    tag, seed = seed_object(corpus_objects[0])
    tree = work / "seed"
    shutil.copytree(inputs / "base", tree)
    (tree / args.collection).mkdir()
    (tree / args.collection / f"{TEMPLATE}.xml").write_text(seed, encoding="utf-8")
    configuration = tree / "Configuration.xml"
    text = configuration.read_text(encoding="utf-8-sig")
    anchor = "\t\t</ChildObjects>"
    configuration.write_text("\ufeff" + text.replace(anchor, f"\t\t\t<{tag}>{TEMPLATE}</{tag}>\n{anchor}"), encoding="utf-8")
    ok, output = create_from_tree(args, work, tree, work / "seeded")
    if not ok:
        raise SystemExit(f"the platform refuses a minimal {tag}: {first_error(output)}")
    export(args, work, work / "seeded", work / "seeded-xml")
    shutil.copyfile(work / "seeded-xml" / args.collection / f"{TEMPLATE}.xml", inputs / "template.xml")

    # 3. The value space.
    subprocess.run(
        [sys.executable, "-I", str(HERE / "observe_values.py"), "--template", str(inputs / "template.xml"),
         "--corpus", str(args.corpus), "--collection", args.collection, "--corpus-name", args.corpus_name,
         "--output", str(inputs / "values.json")],
        check=True,
    )


if __name__ == "__main__":
    main()
