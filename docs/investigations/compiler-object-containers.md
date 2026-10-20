# Object container storage

2026-10-19. Question: can heterogeneous Object containers preserve runtime
classes and reference ownership without inventing concrete read contracts?

**Experimentally observed:** SDK/compiler/simulator 9.2.0, fr265, accept
[the Object container example](../../examples/compiler-object-containers/app.mc).
Mixed scalar Array/Dictionary constants preserve their classes and values.
Computed contents retain source objects, nested containers, ByteArrays, Methods,
WeakReferences, and Symbols. Saved Object snapshots support the existing class
guards, including source-class method calls and ByteArray mutation. Enum
constants NumberFormat, Endian, and FontDefinition enter Object Arrays; runtime
Number tests/string conversion expose their numeric representation.

A helper-created ByteArray stays live while held only by an Object Array, then
expires after its last entry is removed. Dictionary `values()` retains its
references after entry removal; removing the last enumerated reference expires
the weak target. SDK and generated programs match all
[47 console lines](../../examples/compiler-object-containers/expected.txt),
including these ownership transitions. Both PRGs support structured CODE/DATA
export and reconstruction using existing scalar framing and container calls.
This establishes the tested ownership paths, not arbitrary alias-analysis or
collection timing guarantees.

The [current Object storage contract](../specs/frontend/README.md#object-containers)
owns erasure, nullability, invariance, and payload API checks. Typed nested
parameter recovery, general Object keys, and Object-element sorting are outside
this evidence. [Lang.format][lang] takes an erased Array; this container work
provides mixed contents for that consumer. Its module dispatch and substitutions
have [separate evidence](compiler-lang-format.md).

For SDK reproduction, reuse the minimal-console manifest/resources with
`../../examples/compiler-object-containers` as the source path from
`scratch/compiler-object-containers/monkey.jungle`. The example owns its AppBase
class. Build with `scripts/build-example.sh` and use the specimen checker with
`compiler-object-containers`. Current captures are in
`scratch/check-object-containers-v2/`.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#format-instance_function
