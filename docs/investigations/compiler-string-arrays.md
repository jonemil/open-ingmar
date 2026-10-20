# String-array contracts and lowering

Date: 2026-10-16. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: can non-null String-element arrays use the established Array pipeline?

The [authored specimen](../../examples/compiler-string-arrays/README.md) checks
constants, computed/conditional construction, typed parameters/results, reads,
writes, search/removal, append, slicing/reversal, and contextual empty arrays.
SDK and generated programs match all 29 expected console lines. The console
checker confirms native/saved-stage equivalence, structured CODE/DATA, exact
reconstruction, and stable re-export for both PRGs. Artifacts:
`scratch/check-string-arrays/`.

Repeated calls returning `["one", "two", "one", "é", ""]` remain independently
mutable; a saved `toString` result retains its text after element replacement.
Local Array copies share writes. `indexOf`, `remove`, and `removeAll` find String
text; `reverse`/`slice` preserve element contracts and leave the original order
unchanged. The generated constant shares its repeated `one` pool entry.

Markers confirm receiver/index/value order for writes and element evaluation
order through conditional construction. An indexed String read before a later
element mutates its source retains the earlier text. A nullable substring feeds
construction only under a non-null guard.

The [Array API][array] defines Array/String types, `size`, and `toString` at
1.0.0; mutation/search/transforms used here start at 1.3.0. Source and saved IR
check those levels separately. This profile does not establish nullable element
arrays, heterogeneous/nested arrays, sorting, or inherited Object behavior.
Current contracts are in the [source profile](../specs/frontend/README.md#string-arrays)
and [constant IR](../specs/ir/README.md#constant-arrays).

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
