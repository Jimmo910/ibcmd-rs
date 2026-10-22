# Design and dependency boundary

Compare DB/source and DB/reference with `objects/status`. A four-state pairwise RPC
cannot alone distinguish DB-side from local-side edits. For common objects, verify
matching DB version tokens and read the union of changed/baseOnly/localOnly paths
from both comparisons. Any local/reference byte difference must occur in that union.
Classify DB/reference difference and local/reference difference into four directions;
preserve only-in-DB and only-in-files existence states separately. Unknown is explicit.

A minimal flat view uses server full names, with `tree/children` solely to resolve
source ownership for Explorer badges. It is not #379's complete tree/performance work.
Folder baselines are read-only. Git resolves one tree then materializes only the
configured source subtree through argv-only Git plumbing in an explicit cache root.
No native shell adapter, checkout, implicit synchronization, or new filedb code.

Generation invalidation plus AbortController reject stale answers on newer refresh,
connection/configuration/workspace changes and crashes. Refresh clears previous badges
before reading. RPC errors are not forwarded to UI. Unsupported, malformed, missing,
unsaved, changed-version and cancelled evidence cannot become clean.

Remaining integration contract: #382 supplies real filedb reader/session backend;
#362 supplies measured native version semantics. Native status is not inferred from
this byte model. Parent must review the commit before a PR is created.
