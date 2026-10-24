# Windows public fake RPC containment: source proposal

Base: local `005a4ffa6f2c0d985c5529d82f61d061c9e12c7b`. The public PR #532
remains on `2aff32c7`. No workflow is installed by this package. No Windows
runtime result, product execution or SQL acceptance is claimed.

## Exact gate

`windows_rpc_gate.py` admits Windows only, no CLI arguments, exactly seven named
tests and zero skips. It emits an exclusive-created receipt with exact source
hashes before/after and semantic candidate/API bindings. This receipt attests a
fake process contract, not a product binary build. The base must be freshly
reserved by the operator/CI, empty, and match STATUS_LAB_TEST_ROOT; test fixtures
use the existing single-use ownership capabilities. Every fixture is retained.

`windows-rpc-ci.yml.proposal` is inactive outside `.github/workflows`. Activation
needs a separately reviewed public commit and CI authorization. Its only work is
Python public fake processes on an ordinary Windows runner; no secrets, token
persistence, native software, Rust, SQL, VM or private inputs. It restores exact
LF bytes only in its disposable checkout to preserve pinned API hashes. The
checkout layout reproduces the explicit lab boundary without weakening it.
The eight-minute job limit is an outer fail-safe, not a passing cleanup result.
Artifact upload retains only newly owned synthetic fixtures, including failures;
no artifact expiration is applied to retained local lab fixtures.

## Discriminating controls

- Portable model: EOF must wait for normal peer exit before Job Object close.
  The `005a4ffa` order closes the job first, allowing kill-on-close to race a
  normally exiting interpreter. The new wait is <=grace; expiry stays failure.
- Assignment refusal reaps the peer and closes the job before bootstrap. Stale
  code seal refuses before spawning. These models do not prove Windows behavior.
- Real Windows: assert IsProcessInJob and current active membership=1 before the
  bootstrap write. Inspect exact isolated Python argv, finite case and minimal
  environment. The peer cannot spawn descendants before bootstrap admission.
- Real Windows descendant: open a SYNCHRONIZE handle while the ready child is
  alive, verify timeout before teardown and signal on the SAME retained handle
  after job close. Missing PID or skipped tests cannot stand in for termination.
  Explicit stdout/stderr arguments make the reviewed descendant retain BOTH
  pipes on Windows too (default Windows Popen inheritance is insufficient).
  Only the owned job is closed; no broad process search, PID kill or taskkill.
- Hang, stdout/stderr flood, source drift and seal drift must fail, with reader
  and writer joins confirmed and no live/product claim or raw path in report.
  The existing finite fake peer supplies all cases; no arbitrary executable or
  injection surface is added.

The actual Windows gate has not run. macOS model results or the earlier 69-test
POSIX gate cannot establish Job Object layout, assignment, Python DLL loading,
job nesting on hosted runners or Windows termination behavior. A failing
Windows run must retain its evidence and stop; do not retry or relax containment
until the specific failure has been reviewed. This proposal does not authorize
activation, publishing local runner commits or any actual ibcmd/SQL operation.


## Local evidence (POSIX, not Windows)

Fresh static review first found the missing explicit Windows descendant pipes;
that fixture was corrected and re-reviewed BEFORE any new execution. Final
source review: APPROVE. No Windows kernel test has run.

The order model run against the old `005a4ffa` ProcessScope failed exactly its
one expected assertion. On the revised source, **10 targeted tests passed**
(1.837 s recorded by unittest). Then the same-hash
complete suite passed: **76 run, 73 passed, 3 explicit Windows-only skips**
(2.371 s recorded by unittest). No additional skips, failures or errors; no ResourceWarning
in the retained logs. All fake fixtures remain retained. No compiler, host,
SQL/product process or heavy slot was used.

| Reviewed runtime source | SHA-256 |
| --- | --- |
| process_scope.py | e315c9c93dbb77efecc2da940d0aaadf8acceddd2828ce708628bac65e4c8354 |
| fake_rpc_peer.py | 6c3f68a78d2cde57cc58188be54cb829fe0258f3b0c4833ace21a77388f298ed |
| test_windows_rpc_probe.py | 6448739f7366a19529446307f0823b0f569889b2560a0ef117e6d50893786d74 |
| windows_rpc_gate.py | 07dd09069aae3ff351fd0b074a2afc0becdc2bfae991880be04be9198a2f6b73 |

External retained logs:

- `logs/issue-380-windows-model-e315c9c9-targeted.log`
- `logs/issue-380-windows-model-e315c9c9-full.log`

The seven-test Windows-only entry point, actual Job Object queries, same-handle
descendant wait and hosted Windows workflow remain unexecuted. Publication and
workflow activation remain separate boundaries; public PR #532 is unchanged.
