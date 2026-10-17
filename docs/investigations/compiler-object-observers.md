# Object equality and Array observers

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Array][array] inherits [Object][object]. Object
`equals(Object or Null)` and `hashCode()` date from API 1.0.0. Hashes are
specified to remain constant during an object's lifetime and agree for equal
objects. This establishes declarations, not a hash algorithm or Array equality
implementation.

**Observed:** the [authored specimen][example] compares Array references by
identity. An alias compares equal; a fresh equal-content literal, slice, or
reverse result compares unequal. The hash agrees across aliases and remains
unchanged after element replacement and append. No particular hash value or
collision-free allocation rule is inferred.

Array comparisons with other element contracts, scalar values, Null, and a
Dictionary return false. Reverse comparisons from Dictionary and all seven
scalar types also return false. A nullable Number argument is accepted without
proof. Receiver/argument markers run once, in that order, including discarded
results. These observations concern native `equals`, not `==`.

## Reproduction and encoding

Build the tracked source with an ignored Jungle pointing at it and the shared
native-names manifest/resources, then use the
[console checker](../../scripts/README.md#console-specimen-check).
SDK/generated programs match all 44 expected console lines. Source/saved-IR
programs produce identical unsigned PRGs; assembly and structured CODE/DATA
exports reconstruct without raw fallback. Artifacts:
`scratch/check-object-observers/`.

The existing native dispatch, Array references, and `equals`/`hashCode` symbols
suffice; no opcode, DATA record, or saved-IR directive is added. Dispatch remains
native and preserves object identity instead of comparing array contents or
synthesizing hashes.

Ordinary weak-reference creation, lifetime/get/clear behavior, callbacks, and
sorting were not tested by this specimen. In particular, immutable scalar
`weak()` behavior does not establish the Array result contract.

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-object-observers/README.md
