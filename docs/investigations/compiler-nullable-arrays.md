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

**Experimentally observed, 2026-10-24:** SDK 9.2.0 strict checking on `fr265`
accepts `Array<Null>` for constant/computed/empty literals, `new[n]`, indexed
writes, append/search/removal, reverse/slice, and positional shape loss through
dynamic writes and an unknown-length Null-only append tail. The specimen's
`NullArrays` module reproduces these cases. SDK and generated output match;
both PRGs reconstruct without required CODE/DATA fallback.

In contrast, `x.indexOf(1)` for `x as Array<Null>` is rejected as an invalid
Number argument for a Null parameter. This typed Array signature is stricter
than the independently observed positional tuple search profile. Null-only
elements use the existing Null DATA token and ordinary Array VM instructions.

## Nested Array elements

**Experimentally observed, 2026-10-24:** SDK 9.2.0 strict checking on `fr265`,
`-O 0`, accepts `Array<Array<Number>>`, `Array<Dictionary<String, Number>>`,
`Array<[Number, String]>`, and nullable inner Arrays in the
[nested specimen](../../examples/compiler-nested-arrays/README.md).
The specimen uses minimum API 5.0.0 for its custom Comparator. It covers nested
literal context, empty inner containers, three-level generic annotations with
adjacent closing brackets, guarded nullable reads, and sized outer allocation.
Unknown-length append tails and dynamic writes preserve nested contracts when
tuple shape is lost.

SDK and generated simulator output match. Reverse/slice produce independent
outer Arrays but share inner objects: a write through the inner reference
changes the original, while replacing a copied outer slot does not. Search and
removal accept the checked inner container; the example checks first/all removal
with shared references. Ordered fill markers execute once in source order.
Both PRGs reconstruct through structured CODE/DATA without required raw fallback.
