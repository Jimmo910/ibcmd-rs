# Design

The operator supplies one already-selected raw-deflate Form body file locally
inside the VM. It is never transferred to the development host. Bound compressed
and inflated bytes before calling the existing plain Form parser. Admit one root
at the first non-whitespace/optional BOM position and preflight depth/node work
against that same root before recursive parsing. Reuse shared raw attribute
views and existing slot accesses; admit all bag/query/collection bounds before
resolution, without running unrelated full attribute/DCS decoding. No new slot
guesses, callbacks or process-global state.

Reuse existing field-map, legacy twin, required-ID, shadowing and final-query
selection readers. Reports contain duplicate/required/order/equality facts only;
they do not assert why native omits a marker. Metadata-dependent universe results
are unavailable without the corresponding typed context. Wildcards, incomplete
or ambiguous evidence must not become false certainty.

Accumulate a bounded typed report and serialize once after success. The example
sanitizes input/parse/I/O/panic failures to static codes and prints no partial JSON.
No SQL connection, configuration edit, output export or acceptance normalization.
The library diagnostic is coupled to existing private parser code; the small
example wrapper adds no dependency or vendor adaptation.

Independent review and synthetic privacy/rejection tests precede operator use.
Its executable/source identity, input hash and bounds belong in operator receipts;
no private values are needed in the returned artifact.
