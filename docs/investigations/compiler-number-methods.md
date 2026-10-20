# Number method surface and grouped validation

Reviewed 2026-10-11 against Garmin's [Number][number] and [Object][object] API
pages. Runtime experiment: SDK/compiler/simulator 9.2.0, fr265, optimization
disabled. Scope: contracts and native dispatch, not runtime implementation.

Supported method contracts are defined in [the source spec](../specs/frontend/lang-values.md#native-value-calls);
[API declarations](../specs/sdk/declarations.md#deferred-core-methods) record
remaining prerequisites. This record retains public attribution and the tests
that establish runtime behavior.

## Experiment

Inputs: the [complete Number example][example], plus an ignored SDK AppBase
wrapper with a base initializer, `onStart(state)` calling `start()`, and
`getInitialView()` returning `[new Toybox.WatchUi.View()]`. Create
`scratch/compiler-number-methods/monkey.jungle` selecting the example sources,
wrapper, and local fr265 manifest/resources, then:

```sh
sh scripts/build-example.sh scratch/compiler-number-methods
cargo run -p ciq-dis -- all \
  scratch/compiler-number-methods/build/fr265/compiler-number-methods.prg \
  scratch/compiler-number-methods/sdk-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-number-methods/build/fr265/compiler-number-methods.prg
```

Build/sign/run our output using the example README. Observe for 15 seconds,
save each console log before another run overwrites it, and compare its actual
lines with the expected output. Both applications produced the same 22 lines.

**Observed:** SYMBOLS binds `toNumber` to `0x008000a7`, `hashCode` to
`0x008000ad`, `equals` to `0x008000b3`, and `compareTo` to `0x00800e50`.
Comparison signs and Number/Boolean comparisons matched. Equality distinguished
Number from String and Boolean, accepted nullable values, and returned false
for Null. Equal Number hashes matched; no particular hash value is asserted.
Receiver and argument calls printed in that order before the comparison result.

Shared tests verify source/resumed-IR PRG equality, assembly round trips,
explicit bindings, supported alternatives, rejected consumers and IR edits,
and source/IR diagnostics around the 5.0.0 comparison threshold. Known
inheritance permits Object arguments; it does not make arbitrary types or
undeclared classes acceptable. Native comparison has its additional bounded
Number/Boolean/Char argument check. General operator equality remains unchanged.
Overflow, wide numeric values, and other firmware/device profiles remain
unverified.

[number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-number-methods/README.md

[weak-results]: compiler-immutable-weak.md

[char-results]: compiler-char-methods.md
