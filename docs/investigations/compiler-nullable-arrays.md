# Nullable Array elements

Date: 2026-10-16. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: can typed scalar-or-Null elements reuse the established Array
storage, native methods, and scalar null-proof rules?

The [authored specimen](../../examples/compiler-nullable-arrays/README.md)
checks constant/computed literals, indexed access, Null writes, append/search/
removal, transforms, empty/Null-only literals, shared references, and repeated
loads. SDK and generated simulator runs match the complete
[expected output](../../examples/compiler-nullable-arrays/expected.txt). The console
checker confirms saved-stage equivalence, fully structured CODE/DATA, exact
reconstruction, and stable re-export for both PRGs. Artifacts:
`scratch/check-nullable-boolean-char-arrays/`.

Null occupies a real Array slot. Search finds it, `remove` deletes its first
occurrence, and `removeAll` deletes remaining occurrences. Reverse/slice retain
nullable contents. A scalar saved before a later slot write keeps its earlier
value; local Array copies share writes. Repeated calls loading the same mixed
scalar/Null constant remain independently mutable. A present false Boolean is
distinct from Null; guarded Char reads preserve their numeric character values.
Conditional String/Boolean/Char markers confirm that skipped branches do not execute and later elements execute once.

No new opcode or DATA framing is needed: scalar Null uses its existing tag;
String-containing records retain local pools. Current source inference is
intentionally narrower than general nullable literal inference: direct literals
use an explicit nullable expected contract. Existing Array references remain
invariant. See the [source contract](../specs/frontend/README.md#nullable-array-elements)
and [Array IR](../specs/ir/README.md#constant-arrays).
