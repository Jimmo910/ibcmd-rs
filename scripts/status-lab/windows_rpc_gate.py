"""Proposed Windows CI entry: fixed suite; no arbitrary command/test selection."""
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

import rpc_probe as probe

EXPECTED = {
    "WindowsOrderingModel.test_eof_waits_for_normal_exit_before_closing_job",
    "WindowsOrderingModel.test_eof_does_not_turn_a_stalled_exit_into_success",
    "WindowsOrderingModel.test_failed_job_assignment_reaps_peer_without_bootstrap",
    "WindowsOrderingModel.test_stale_code_seal_refuses_before_process_creation",
    "WindowsKernelControls.test_fixed_argv_and_real_job_membership_before_bootstrap",
    "WindowsKernelControls.test_descendant_handle_is_signalled_after_job_close",
    "WindowsKernelControls.test_windows_hang_flood_and_drift_never_succeed",
}


def leaves(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from leaves(item)
        else:
            yield item


def main():
    # This gate cannot promote a POSIX skipped run into Windows evidence.
    if os.name != "nt" or len(sys.argv) != 1:
        raise SystemExit("Windows fixed-suite gate required")
    root = probe.FIXTURE_BASE
    if os.environ.get("STATUS_LAB_TEST_ROOT") != str(root) or not root.is_dir() or any(root.iterdir()):
        raise SystemExit("new empty owned fixture base required")
    suite = unittest.defaultTestLoader.loadTestsFromName("test_windows_rpc_probe")
    tests = list(leaves(suite))
    ids = {test.id().removeprefix("test_windows_rpc_probe.") for test in tests}
    if ids != EXPECTED or len(tests) != len(EXPECTED):
        raise SystemExit("exact test inventory required")
    sources = (*probe.CODE, "test_rpc_probe.py", "test_windows_rpc_probe.py", "windows_rpc_gate.py")
    def digests():
        return {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest() for name in sources}
    before = digests()
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(tests))
    unchanged = before == digests()
    passed = result.wasSuccessful() and result.testsRun == len(EXPECTED) and not result.skipped and unchanged
    receipt = {"schema": 1, "platform": "windows", "fixtureOnly": True,
        "productExecuted": False, "liveAcceptance": False, "publishedStatus": False,
        "success": passed, "testsRun": result.testsRun, "skipped": len(result.skipped),
        "sourcesUnchanged": unchanged, "sourceSha256": before,
        "candidateCommit": probe.CANDIDATE, "apiBase": probe.API_BASE, "protocol": probe.PROTOCOL}
    with (root / "windows-gate-receipt.json").open("x") as out:
        json.dump(receipt, out, indent=2)
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
