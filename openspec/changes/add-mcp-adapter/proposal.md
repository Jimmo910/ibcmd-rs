# MCP adapter for ibcmd-rs (#386)

Expose the common configuration engine to AI clients through a genuine local MCP
stdio command, without exposing editor connect/settings overrides to the model.
The existing editor Content-Length JSON-RPC protocol remains separate.

This first independently reviewable slice supports offline read-only Config row
fixtures. Import deliberately fails closed. Native filedb staging, per-database
`agent-write`, live immutable database identity and native roundtrip acceptance
remain integration gates owned jointly with #382; this slice does not close #386.
