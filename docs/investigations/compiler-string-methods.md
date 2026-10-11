# String method surface and grouped validation

Reviewed 2026-10-11 against Garmin's [String][string] and inherited
[Object][object] contracts. Runtime experiment: SDK/compiler/simulator 9.2.0,
fr265, optimization disabled. Scope: compiler contracts and native dispatch.

## Method inventory

The String page lists 17 methods, including its Object overrides; two additional
Object members are inherited. Signatures and introductions below are publicly
documented. Prerequisites describe our executable subset.

| Method | Result | Since | Executable scope or prerequisite |
|---|---|---|---|
| `compareTo(Object)` | Number | 5.0.0 | Supported for String arguments |
| `equals(Object or Null)` | Boolean | 1.0.0 | Scalar/nullable scalar arguments |
| `find(String)` | Number or Null | 1.0.0 | Supported |
| `hashCode()` | Number | 1.0.0 | Supported |
| `length()` | Number | 1.0.0 | Supported |
| `substring(Number or Null, Number or Null)` | String or Null | 1.0.0 | Nullable indices require 3.3.2 |
| `toCharArray()` | Array of Char | 1.3.0 | Needs arrays and Char values |
| `toDouble()` | Double or Null | 3.1.0 | Needs Double and nullable Double |
| `toFloat()` | Float or Null | 1.0.0 | Needs Float and nullable Float |
| `toLong()` | Long or Null | 3.1.0 | Needs Long and nullable Long |
| `toLongWithBase(Number)` | Long or Null | 3.1.0 | Needs Long and nullable Long |
| `toLower()` | String | 1.0.0 | Supported |
| `toNumber()` | Number or Null | 1.0.0 | Supported |
| `toNumberWithBase(Number)` | Number or Null | 1.4.1 | Supported |
| `toString()` | String | 1.0.0 | Supported |
| `toUpper()` | String | 1.0.0 | Supported |
| `toUtf8Array()` | Array of Number | 1.3.0 | Needs arrays |
| inherited `method(Symbol)` | Method | 1.0.0 | Needs Symbols and callable values |
| inherited `weak()` | WeakReference; effective String | 1.2.0 | Supported; [separate probe][weak-results] |

Comparison's Object declaration admits only String at runtime. Our lowering
rejects other arguments, including unchecked String-or-Null. Equality's broader
contract uses the shared declared-inheritance check and bounded scalar profile.
The [immutable weak probe][weak-results] establishes the receiver-specific result
and SDK checker discrepancy. Deferred conversions, arrays, callables, and
ordinary object weak references were not tested here.

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
