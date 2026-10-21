# Boolean method surface and grouped validation

Reviewed 2026-10-11 against Garmin's [Boolean][boolean] and [Object][object]
API pages. Runtime experiment: SDK/compiler/simulator 9.2.0, fr265,
optimization disabled. Scope: contracts and dispatch, not runtime bodies.

Supported method contracts are defined in [the source spec](../specs/frontend/README.md#native-value-calls);
[Core checklist](../plan.md#toyboxlang-worklist) records
remaining prerequisites. This record retains public attribution and the tests
that establish runtime behavior.

## Experiment

Inputs: the [complete Boolean example][example] plus an ignored AppBase wrapper
whose initializer calls the base initializer, whose `onStart(state)` calls
`start()`, and whose `getInitialView()` returns `[new Toybox.WatchUi.View()]`.
Create `scratch/compiler-boolean-methods/monkey.jungle` selecting the example
sources, wrapper, and local fr265 manifest/resources, then:

```sh
sh scripts/build-example.sh scratch/compiler-boolean-methods
cargo run -p ciq-dis -- all \
  scratch/compiler-boolean-methods/build/fr265/compiler-boolean-methods.prg \
  scratch/compiler-boolean-methods/sdk-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-boolean-methods/build/fr265/compiler-boolean-methods.prg
```

Build/sign/run our output using the example README. Save each console log before
another run overwrites it and compare actual lines with the expected output.
Both applications produced the same 27 lines.

**Observed:** SYMBOLS binds `toString` to `0x008000af`, `hashCode` to
`0x008000ad`, `equals` to `0x008000b3`, and `compareTo` to `0x00800e50`.
These reuse the existing native profile. SDK-generated CODE uses ordinary
Boolean pushes and member dispatch; our generic backend requires no Boolean
boxing or new call operation for this specimen.

Boolean/Number comparison signs matched, including zero/one equivalence and
negative Number arguments. Equality distinguished Boolean from Number and
String, accepted nullable arguments, and returned false for Null. Equal Boolean
hashes matched; particular hash values are not asserted. String conversion
produced `true`/`false`. Receiver and argument calls printed in that order before
the comparison result. These observations do not establish behavior for other
numeric types, firmware, or devices.

Shared tests cover source/resumed-IR PRG equality, assembly round trips, explicit
bindings, argument/result validation, edited IR and null guards, and source/IR
diagnostics around the API 5.0.0 threshold. Boolean comparison returns Number;
it cannot directly feed a branch. Native comparison's mixed arguments do not
expand operator equality, ordering, or arithmetic.

[boolean]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Boolean.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-boolean-methods/README.md

[weak-results]: compiler-immutable-weak.md

[char-results]: compiler-char-methods.md
