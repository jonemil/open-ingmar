# Default Array sorting and Comparator contracts

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Array.sort][array] takes `Lang.Comparator or Null`,
returns Void, and requires API 5.0.0. Null selects an ascending default
comparator for Numeric, Boolean, Char, and String values. [Comparator][lang]
is a structural interface requiring `compare(Object, Object) as Number`, also
from 5.0.0. It is not a Method callback or a native superclass. These contracts
correct the planned dependency between typed Methods and Array sorting.

**Observed:** the [authored specimen][example] sorts homogeneous Number, Long,
Float, Double, Boolean, Char, and String Arrays. It checks duplicate retention,
alias-visible mutation, independent slices, repeated sorting, empty/singleton
Arrays, and one evaluation of an effectful receiver. SDK/generated programs
match all 17 expected lines. No sort stability, NaN ordering, String collation
algorithm, or custom-comparator behavior is inferred.

Build the tracked source through an ignored Jungle using the shared
native-names manifest/resources and `scripts/build-example.sh`. Run the
[console checker][check] for saved-stage parity, structured CODE/DATA/SYMBOLS
reconstruction, and SDK/generated runtime comparison. Artifacts:
`scratch/check-array-sort/`; the SDK SYMBOLS binds `sort` to `0x00800e51`.
Existing member lookup, `invokem`, and `popv` suffice; no opcode or DATA record
extension was needed.

This specimen does not establish nullable/object element or custom Comparator
behavior. See the [current contract][spec],
[source-comparator evidence](compiler-comparators.md), and
[Object/reference sorting evidence](compiler-object-sort.md).

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparator-typedef
[example]: ../../examples/compiler-array-sort/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#default-array-sorting
