# Source Comparator matching and runtime dispatch

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Lang.Comparator][lang] requires
`compare(Object, Object) as Number`; [Array.sort][array] accepts a Comparator
or Null from API 5.0.0.

**Observed:** the [two-file specimen][example] passes source View instances to
sort without a Comparator superclass. Per-instance direction controls ascending
and descending String order; both instances record nonzero callback counts.
Aliases see sorted contents, typed source forwarding preserves the comparator,
and receiver/argument markers print once in that order. Numeric values ordered
by string conversion produce `[2, 10, 1]`, distinguishing custom dispatch from
default numeric order. Empty/singleton inputs complete. SDK/generated programs
match all nine expected lines.

Two isolated variants use the same View/compare shape but change both inputs
to Number, then Object-or-Null; the nullable variant returns constant zero so
no body type error obscures the result. Both SDK builds reject the class at
`sort(c)` as incompatible with the documented interface. Our source/unit
checker therefore requires exact input/result contracts, rather than assuming
function-parameter variance. Probe directory: `scratch/comparator-probe/`.

## Encoding and reproduction

Build tracked sources through an ignored Jungle using the shared native-names
manifest/resources and `scripts/build-example.sh`. The [console checker][check]
validates saved stages, structured SDK/generated CODE/DATA/SYMBOLS, lossless
reconstruction, and simulator output. Artifacts: `scratch/check-comparators/`.
The SDK symbol table binds `compare` to `0x00800150`; generated class metadata
must use that ID for runtime dispatch. `sort` retains `0x00800e51` and existing
invocation instructions; no new opcode or DATA record was encountered.

Using an existing View class isolates structural matching from Object-rooted
source construction. General interface storage, nullable/object Array elements,
Object input casts/refinement, and Comparable were not established. Callback
counts, call order within sorting, stability, and exception behavior are not
inferred. Current rules belong in the [sorting contract][spec]; the [plan][plan]
owns remaining core work.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparator-typedef
[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[example]: ../../examples/compiler-comparators/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/compiler-subset.md#source-comparators
[plan]: ../plan.md
