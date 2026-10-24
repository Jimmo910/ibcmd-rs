# Proposed Windows CI selection (not enabled or run)

This document is not a workflow/settings change or execution grant. After exact test-pin review and separate authorization, build the Windows integration target once with pinned Rust1.95/no-default-features/jobs1 and capture Cargo compiler-artifact JSON, exact source stack/tree, test executable SHA256 and fixture hashes:

```
cargo test --locked -p ibcmd-rs --test editor_partial_import_windows --no-default-features --no-run --message-format=json
```

Require the actual Windows target to list the meaningful named parent controls. Execute exact positive and refusal parent names with --nocapture --test-threads=1, one selected/zero ignored each. Every parent runs exact sentinel children using the same executable; save child stdout/stderr/exit and phase receipts under the owned case root. An empty non-Windows target, a helper-only result, ignored/missing case, child sentinel failure or incidental sharing refusal is failure. Do not remove existing cfg(unix) or modify production OS guards to obtain green.

Use a physical local fixed NTFS RUNNER_TEMP, KEEP_FIXTURES=1 and distinct child TMP/TEMP/TMPDIR. Retain only public fixture copies, logs and scoped receipts, not credentials/unrelated caches. No native1C/SQL/VM/MCP invocation. Do not manually dispatch/rerun or publish this branch before review/authorization. Full Windows persistent multi-RPC compiler reuse, real source/target alias/security profiles and broader required CI remain separate acceptance.
