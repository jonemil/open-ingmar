# Numeric runtime boundaries

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`, Linux x86-64.

**Observed:** exceptional arithmetic and numeric conversions do not share one
failure rule. The results below describe this simulator, not a portable device
contract. The compiler must retain native operations rather than substitute
host-language casts, constant results, or guards.

## Isolated arithmetic

Compile each row as its own application. Store operands in locals and print
`before`, the operation's result, and `after`, then call `System.exit()`.

| Operand type | Operation | Observation |
|---|---|---|
| Long | minimum / -1 | Simulator process exits with signal 8 after `before` |
| Long | minimum % -1 | Simulator process exits with signal 8 after `before` |
| Long | 17 / 0 | `Invalid Value` app error; no result or `after` |
| Long | 17 % 0 | `Invalid Value` app error; no result or `after` |
| Float | 1.0 / 0.0 | `Invalid Value` app error; no result or `after` |
| Double | 1.0d / 0.0d | `Invalid Value` app error; no result or `after` |
| Double | 0.0d / 0.0d | `Invalid Value` app error; no result or `after` |

Long minimum is `-9223372036854775808l`; both operands use Long literals.
These observations extend the earlier [Number division probes][products] and
[Float zero/zero probe][keys]. A simulator process failure does not establish a
catchable language exception. The app-error cases were not tested with catches.

The first minimum-Long division terminated the process with exit code `-8`.
Two following sessions failed during simulator startup, before uploading their
PRGs; fresh-session retries produced the results above. Keep startup failures
separate from numeric results. Retry artifacts use `long-zero-div-retest/` and
`long-min-rem-retest/`; the latter also records process exit `-8`.

## Floating-to-integer conversions

For each input, print `x.toNumber()` and `x.toLong()` between the same markers.
Finite inputs use source literals; non-finite inputs use the controlled
immediate substitutions below.

All four finite inputs below returned `-2147483648` from `toNumber()` and
`-9223372036854775808` from `toLong()`, then reached `after`:

- Float: `3.4028235e38` and `-3.4028235e38`.
- Double: `1.7976931348623157e308d` and `-1.7976931348623157e308d`.

Both conversions returned those same minimum signed values for each injected
positive infinity, negative infinity, and quiet NaN, for both floating types.
The marker between the conversions and final `after` establish that each call
returned and the application completed normally.

These results do not establish saturation, wrapping, the exact transition near
an integer boundary, behavior of other NaN payloads, or device behavior. In
particular, a positive out-of-range value did not clamp to the positive maximum.
Keep conversions as native calls with their declared result types.

## Reproduction

The ignored probe projects and logs are under `scratch/numeric-boundaries/`.
Each source uses this wrapper, with the body selected above:

```monkeyc
using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("before");
        // Operand locals and one isolated operation go here.
        System.println("after");
        System.exit();
    }
}
```

Use a Jungle selecting that source and the shared ignored native-names manifest
and resources. Build with `scripts/build-example.sh`; copy the resulting PRG
away from its neighboring SDK debug XML. Run with:

```sh
xvfb-run -a python3 scripts/simulator-session.py --device fr265 \
  --prg scratch/numeric-boundaries/CASE.prg \
  --output-dir scratch/numeric-boundaries/CASE/runtime --observe-seconds 10
```

For non-finite values, compile `var x = 1.234567;` (Float) or
`var x = 9.87654321d;` (Double), followed by both conversions. Export all
sections with `ciq-dis all`. Replace the single CODE immediate, originally
`3f9e064b` for Float or `4023c0ca4588f633` for Double:

| Value | Float bits | Double bits |
|---|---|---|
| Positive infinity | `7f800000` | `7ff0000000000000` |
| Negative infinity | `ff800000` | `fff0000000000000` |
| Quiet NaN | `7fc00000` | `7ff8000000000000` |

Keep the `fpush`/`dpush` instruction and operand width unchanged. Reassemble
with `ciq-asm project` and sign with our developer key. Only our authored
application's output is edited. Both template exports use structured CODE/DATA
without fallback; all six patched projects reconstruct, verify their new
signatures, and reach the expected conversion markers in the simulator.

[products]: compiler-number-products.md
[keys]: compiler-numeric-dictionaries.md#signed-zero-and-nan-probes
