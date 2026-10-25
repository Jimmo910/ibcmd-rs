# Converter coverage baselines

Native inputs and oracle artifacts live in the pinned public submodule
[`../1c-fixtures`](../1c-fixtures), repository https://github.com/Untru/1c-fixtures.

This directory retains `baseline.json` and `storage-baseline.json` at the same
platform/set paths. They are converter promises and are not owned by the corpus.
Do not remove an equal variant or path to accommodate a corpus update.

```text
git submodule update --init --recursive
python tools/coverage/fixture_paths.py
cargo test --locked --no-default-features --test coverage_corpus
```

Source probe refusal expectations remain in
`../coverage-records/source-probe-expectations.json`, keyed by stable probe ID.
