# Typed scalar field storage

2026-10-19. Question: can concrete scalar field contracts preserve class,
nullability, and initialization through source, saved IR, and complete PRGs?

**Observed:** SDK/compiler/simulator 9.2.0, fr265, accept the
[authored specimen](../../examples/compiler-scalar-fields/app.mc). SDK and generated
apps match all [43 console lines](../../examples/compiler-scalar-fields/expected.txt).
Number, Long, Float, Double, Boolean, Char, and String fields retain their
runtime classes. Long `2147483648` is not narrowed; Float negative zero survives
initialization. Compound writes use the normal operator result classes, and a
second instance retains its original values.

All seven nullable field types initially read Null. A local snapshot remains
usable under a null guard after its field is cleared. The AppBase's required
Long field is initialized after the native base call and read at startup.

SDK/generated structured CODE/DATA reconstruction and saved IR/assembly parity
pass. Existing `getselfv`/`getself`/`spush`/`putv` operations and Null metadata
slots suffice; no new opcode or record framing was encountered. Our generated
classes contain 15 Null scalar slots across the two source classes.

For SDK reproduction, use minimal-console's manifest/resources and
`base.sourcePath=../../examples/compiler-scalar-fields` from
`scratch/compiler-scalar-fields/monkey.jungle`. Build with
`sh scripts/build-example.sh scratch/compiler-scalar-fields`, then use the console
specimen checker with `compiler-scalar-fields`. Captures:
`scratch/check-scalar-fields/`.

The [field contract](../specs/frontend/fields.md#typed-scalar-fields) owns our
required-initialization rules and declaration-default restrictions. This
specimen does not establish arbitrary declaration initializers, direct mutable
field narrowing, additional superclass layouts, or other runtime targets.
