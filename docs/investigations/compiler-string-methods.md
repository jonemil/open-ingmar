# String method surface and grouped validation

Reviewed 2026-10-11 against Garmin's [String][string] and inherited
[Object][object] contracts. Runtime experiment: SDK/compiler/simulator 9.2.0,
fr265, optimization disabled. Scope: compiler contracts and native dispatch.

Supported method contracts are defined in [the source spec](../specs/compiler-subset.md#native-value-calls);
[API declarations](../specs/api-declarations.md#deferred-core-methods) record
remaining prerequisites. This record retains public attribution and the tests
that establish runtime behavior.

## Experiment

Inputs: the [complete String example][example] and an ignored SDK AppBase
wrapper with a base initializer, `onStart(state)` calling `start()`, and
`getInitialView()` returning `[new Toybox.WatchUi.View()]`. Create
`scratch/compiler-string-methods/monkey.jungle` selecting these sources and
local fr265 manifest/resources, then:

```sh
sh scripts/build-example.sh scratch/compiler-string-methods
cargo run -p ciq-dis -- all \
  scratch/compiler-string-methods/build/fr265/compiler-string-methods.prg \
  scratch/compiler-string-methods/sdk-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-string-methods/build/fr265/compiler-string-methods.prg
```

Build/sign/run our output using the example README. Observe for 15 seconds and
compare actual console lines, saving logs before another run overwrites them.
Both applications produced the same 33 lines.

**Observed:** SYMBOLS binds `compareTo` to `0x00800e50`, `equals` to
`0x008000b3`, `hashCode` to `0x008000ad`, and `toNumberWithBase` to
`0x008004c3`. Comparison signs, mixed/nullable equality, hash consistency,
ASCII case conversion, and receiver-before-argument order matched. Base
conversions exercised 0, 2, 10, 16, and 36, prefix detection, a negative value,
and a failed conversion returning Null. On `é猫z`, length was 3, search for
`猫` returned 1, and substring `(1, 2)` returned `猫`.

The UTF-8 specimen supports character-based indexing for those BMP characters
on this profile; non-BMP characters, combining sequences, and normalization
remain unverified. Invalid base ranges and numeric overflow remain runtime
obligations, not compiler-checked properties. General String operator equality
is unchanged; these are method calls. SDK CODE export encountered an unsupported
opcode; symbol observations and runtime comparison do not depend on its decoding.

Tests check source/resumed-IR PRG equality, assembly round trips, explicit
bindings, nullable results and guard edits, argument restrictions, and matching
source/IR diagnostics around API 1.4.1 and 5.0.0. Other firmware/device profiles
and older simulators were not tested.

[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-string-methods/README.md
[weak-results]: compiler-immutable-weak.md
