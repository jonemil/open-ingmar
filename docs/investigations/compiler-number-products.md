# Number multiplication, division, and remainder

SDK/compiler/simulator 9.2.0, Forerunner 265, 2026-10-14. The independently
written [Number-products example][example] produces exactly the same console
output with SDK compilation and our generated, own-key-signed PRG. Separate
failure specimens establish zero-divisor failures and expose a simulator
process failure at the minimum-Number/-1 boundary. These observations cover
Number pairs, not promotion to other numeric types or exception recovery.

## Contracts and provenance

Official [Functions documentation][functions], reviewed 2026-10-14, places
`*`, `/`, and `%` at the same precedence above `+` and `-`, below unary/postfix
operations. Its client-loaded article is available directly as [Functions.html][article].
Source-derived opcode names were cross-checked against the optimizer's
[opcode enum][opcode-enum] at revision
`2920dc88c23caadded9f66ee0613a7de6ab2d1ea`; no implementation code was copied.

Inspection of our SDK-built specimen confirms single-byte, operand-free
instructions `mulv` (`0x05`, CODE+`0x76`), `divv` (`0x06`, CODE+`0x88`), and
`modv` (`0x09`, CODE+`0x9a`). Both disassembly and generated code use these
encodings. The public contracts, controlled specimens, and
[local instruction reference](../specs/vm/instructions.md) are the basis for our
independent Rust implementation.

Artifact SHA-256 values from this run (SDK signed PRG, then our unsigned PRG):

```text
1c8c4bf6c060a99fddf9772edb28e70b1f0c145e268801e54cb6d0c440f62886
71acf701b7e2f35bdf390dfc9c2878cda86843bc11133c5e0289f5b2477c9c2a
```

## Runtime observations

The [expected output](../../examples/compiler-number-products/expected.txt)
covers these cases:

- `17 * 5 = 85`; Number division truncates toward zero for all tested sign
  combinations (`17 / 5 = 3`, `-17 / 5 = -3`).
- Remainder follows the dividend's sign: `-17 % 5 = -2`, `17 % -5 = 2`.
- `2 + 3 * 4 = 14`, `(2 + 3) * 4 = 20`, `20 / 3 / 2 = 3`, and
  `20 / (3 / 2) = 20` establish precedence/grouping and left association.
- `20 % 6 * 3 = 6` and `20 * 6 % 7 = 1` check mixed product chains.
- Operand methods print `left` before `right` for each operator; each executes
  once before the arithmetic result is printed.
- Multiplication probes yield `2147483647 * 2 = -2`,
  `-2147483648 * -1 = -2147483648`, and `65536 * 65536 = 0`, consistent with
  signed 32-bit wrapping. This does not characterize mixed numeric types.

Normal SDK/generated captures show the exited blank watch display without an
error indicator. Exact console comparison supplies the arithmetic verdict.

Zero-divisor probes use a local Number zero rather than relying on SDK constant
folding. Both SDK and generated apps print the before marker, then report
`Error: Invalid Value` and an app crash; neither reaches the after marker.
The helper returns nonzero and the generated capture shows the IQ error
indicator. This establishes runtime failure, not catchability or an exception
class. Our compiler retains literal zero divisors as VM operations too.

`-2147483648 / -1` exits the SDK simulator process before the after marker;
our generated division probe reproduces a simulator exit status of -8 (SIGFPE
on this Linux environment). The SDK and generated remainder probes with the
same operands also exit with -8. This is a host-process boundary failure, not an
established
Monkey C result/exception contract or behavior on physical watches. No compiler
constant folding, runtime guard, or Number result is inferred from it.
An immediate retry after the process failure failed to start the simulator;
later sessions started normally. That startup failure is not arithmetic evidence.

## Reproduction

Use [the example's build/sign commands][example]. For the SDK, create
`scratch/compiler-number-products/monkey.jungle`:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-products
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-products/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-products
cargo run -p ciq-dis -- all \
  scratch/compiler-number-products/build/fr265/compiler-number-products.prg \
  scratch/compiler-number-products/sdk-export-final
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
cmp examples/compiler-number-products/expected.txt scratch/simulator/monkeydo.log
```

Run each PRG separately. Save console/capture artifacts after successful normal
sessions: `sdk-final.{log,png}` and `generated.{log,png}` under
`scratch/compiler-number-products/`. Compare console content; inspect images
for unexpected error indicators. Source/unit-IR/assembly paths produce equivalent
complete unsigned PRGs.

For an isolated failure, replace Products with:

```text
using Toybox.System;
module Products {
    function run() {
        var zero = 0;
        var min = -2147483648;
        System.println("before");
        System.println(17 / zero);
        System.println("after");
    }
}
```

Keep the example's app source and select exactly this module. Replace the
arithmetic expression with `17 % zero`, `min / (-1)`, or `min % (-1)` for the
other probes. Each needs its own PRG/session because execution may stop the app
or simulator. Retained artifacts use `zero-div/`, `zero-rem/`, `min-div/`, and
`min-rem/` subdirectories, with `sdk`/`generated` log prefixes. The successful
SDK remainder retry uses `sdk-retry` files. The simulator helper reports native
exit codes and captures the display even after a runtime process exit; startup
failures do not establish program execution.

[example]: ../../examples/compiler-number-products/README.md
[functions]: https://developer.garmin.com/connect-iq/monkey-c/functions/
[article]: https://developer.garmin.com/connect-iq/articles/monkey-c/Functions.html
[opcode-enum]: https://github.com/markw65/monkeyc-optimizer/blob/2920dc88c23caadded9f66ee0613a7de6ab2d1ea/src/readprg/opcodes.ts
