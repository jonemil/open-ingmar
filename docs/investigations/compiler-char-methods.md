# Char contracts and conversion probes

Reviewed 2026-10-11 against Garmin's [Char][char], [Number][number], and
[Object][object] API pages. Runtime experiments: SDK/compiler/simulator 9.2.0,
fr265, optimization disabled.

## Method inventory

Char has five direct methods, including a `toString` override, and four other
inherited Object members. The table separates declared introductions from our
executable scope. Inherited calls also require Char's API 1.3.0 introduction.

| Method | Result | Since | Executable scope or prerequisite |
|---|---|---|---|
| `toLower()` | Char | 1.3.0 | Supported |
| `toUpper()` | Char | 1.3.0 | Supported |
| `toNumber()` | Number | 1.3.0 | Supported |
| `toString()` | String | 1.3.0 | Supported |
| `compareTo(Object)` | Number | 5.0.0 | Supported for Number/Boolean/Char arguments |
| inherited `equals(Object or Null)` | Boolean | 1.0.0 | Supported for scalar/nullable scalar arguments |
| inherited `hashCode()` | Number | 1.0.0 | Supported |
| inherited `method(Symbol)` | Method | 1.0.0 | Needs Symbols and callable values |
| inherited `weak()` | WeakReference | 1.2.0 | Needs ordinary weak-reference values and consumers |

**Documented:** Char represents Unicode characters, and `toNumber()` returns
the UTF-32 representation interpreted as Number. Comparison also accepts other
numeric types, which need executable support. The documented immutable-value
exception to `weak()` does not list Char; do not extend our Number/String/Boolean
specialization to it without evidence. `Number.toChar()` returns Char from
API 1.3.0; its page specifies no range restriction.

## Conversion boundaries

An ignored SDK probe created Chars through `Number.toChar()`, printed them or
their numeric representations, and exercised the supported methods. A literal
`'A'` was included in this SDK probe; the [literal follow-up][literals] establishes
its encoding and our source support. Use an AppBase wrapper whose `onStart(state)` calls `start()`
and whose initial View is `[new Toybox.WatchUi.View()]`, with local fr265
manifest/resources:

```sh
sh scripts/build-example.sh scratch/compiler-char-methods
cargo run -p ciq-dis -- all \
  scratch/compiler-char-methods/build/fr265/compiler-char-methods.prg \
  scratch/compiler-char-methods/probe-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-char-methods/build/fr265/compiler-char-methods.prg
```

**Observed:** `toChar` binds to `0x008004c6`; other method names reuse existing
native IDs. CODE export fell back to raw binary at unsupported opcode offset
`0xaa`. This does not identify the literal encoding; the isolated
[follow-up][literals] establishes `cpush` separately.

The probe's numeric round trips were:

| Number input | `toChar().toNumber()` |
|---|---|
| 65 | 65 |
| 29483 | 29483 |
| 128512 | 128512 |
| 0 | 0 |
| -1 | -1 |
| 55296 | 55296 |
| 1114112 | 1114112 |

ASCII case conversion and BMP string conversion (`é`, `猫`) worked. The console
capture of `128512.toChar().toString()` displayed a replacement character,
despite numeric preservation. Its String encoding/rendering remains unresolved;
do not infer complete supplementary-character support from the numeric result.
Invalid scalar values above were consumed only through numeric conversion.
Other widths, case mappings, invalid-value string conversion, devices, and
firmware remain unverified. No compiler Unicode range check follows from these
observations.

## Complete comparison

Create `group.jungle` selecting the [complete example][example], the same
wrapper, and local manifest/resources. Build without disabling SDK checking:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-char-methods/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-char-methods
```

Run the resulting PRG, then build/sign/run our output with the example README.
Save both console logs and compare actual lines with the expected output. Both
programs produced the same 31 lines. They preserve Char through native calls,
typed arguments/returns, local assignment, and guarded Number-to-Char conversion.
Equality distinguished Char from Number, String, and Boolean; equal Char hashes
matched. Comparison signs matched for Char/Number/Boolean arguments in both
receiver directions. Receiver and argument markers printed in that order.

Source/resumed-IR PRGs and symbolic assembly round trips agree. Contract tests
cover wrong arguments/consumers, exact user signatures, edited call results,
unsupported operator/nullable forms, and class/method API boundaries.
Char uses the existing generic value-call and slot machinery; it is distinct
from Number in semantic validation. The [literal follow-up][literals] establishes
constant encoding separately; generic call storage establishes no boxing rule
or runtime value layout.

[char]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Char.html
[number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html#toChar-instance_function
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-char-methods/README.md

[literals]: compiler-char-literals.md
