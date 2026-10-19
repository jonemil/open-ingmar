# Object and container field storage

2026-10-19. Question: can instance fields preserve Object erasure and invariant
container contracts without retaining extra hidden reference owners?

**Observed:** SDK/compiler/simulator 9.2.0, fr265, accept the
[authored specimen](../../examples/compiler-container-fields/app.mc).
SDK and generated programs match all
[35 console lines](../../examples/compiler-container-fields/expected.txt).
Object class guards recover source objects and ByteArray operations. Typed
Array/Dictionary fields preserve mutation, replacement, aliases, numeric
typedefs, and stored Method invocation. A second instance has separate contents.

Helper-created ByteArrays remain live while held by Object fields, Array
elements, or Dictionary entries. Replacement/removal releases that ownership;
saved element snapshots and `values()` enumerations retain their references.
Removing those last owners expires the weak target in the tested simulator.
Conditional Object/Dictionary assignments likewise retain only their selected
value and release it on field replacement. These observations do not establish
garbage-collection timing on all devices.

The conditional Dictionary assignment exposed a missing semantic join contract.
Matching typed Dictionaries now use their existing `dictionary(K=>V)` tag at
`.block-value`. No runtime conversion is involved. Fields supply literal context
to both arms, and join results are staged before field receiver setup.

Saved IR/assembly parity and structured CODE/DATA reconstruction pass for SDK
and generated PRGs. Existing field access, local-transfer instructions, and Null
class slots suffice; no new opcode or DATA framing was encountered. The generated
classes contain nine Null Object/container slots.

For SDK reproduction, reuse minimal-console's manifest/resources and set
`base.sourcePath=../../examples/compiler-container-fields` from
`scratch/compiler-container-fields/monkey.jungle`. Build with
`sh scripts/build-example.sh scratch/compiler-container-fields`, then use the
console specimen checker with `compiler-container-fields`. Captures:
`scratch/check-container-fields/`.

The [field contract](../specs/compiler-classes.md#object-and-container-fields)
owns typing and initialization; the [Dictionary contract][dictionary] owns
conditional joins. Nullable whole containers, typed nested containers, source
inheritance, and broader declaration defaults are outside this evidence.

[dictionary]: ../specs/compiler-subset.md#dictionary-values
