# Number comparisons and null inequality

SDK/compiler/simulator 9.2.0, Forerunner 265, 2026-10-14. The independently
written [comparison example][example] produces identical SDK/generated console
output. Both normal captures show the exited blank watch display without an
error indicator. This verifies the bounded scalar profiles and direct null
inequality guards; it does not establish general object comparison, other
numeric types, or String ordering.

## Contracts and instruction evidence

Garmin's [Functions documentation][functions] lists `<`, `<=`, `>`, `>=`, `==`,
and `!=` at the same precedence, below arithmetic and above logical operators.
Reviewed 2026-10-14; its client-loaded [article][article] supplies the text.
Our grammar accepts one comparison per expression level; chained comparisons
remain outside the supported subset.

The SDK-built run method starts at CODE+`0x98`. Decoding its supported prefix
using our local opcode widths identifies these single-byte, operand-free
instructions:

| Operator | Mnemonic | Byte | First observed PC |
|---|---|---|---|
| `<` | lt | `0x1b` | `0xd1` |
| `<=` | lte | `0x1c` | `0xf5` |
| `>=` | gte | `0x1e` | `0x119` |
| `!=` | ne | `0x1f` | `0x14f` |

Opcode names/order were cross-checked against the previously pinned
[optimizer opcode enum][enum]. Our implementation uses this repository's
[comparison instruction reference](../specs/vm/instructions.md#lt--lte--gte--ne).
No external implementation code was copied.

Full SDK CODE decoding stops at CODE+`0x81` in parsed(), so the exporter keeps
CODE raw. The run prefix can be inspected independently at its metadata method
boundary; it stops at another unsupported opcode at `0x211`. We make no claim
about complete SDK instruction decoding or the semantics of those unsupported
instructions. Our generated CODE decodes completely.

Artifact SHA-256 values (SDK signed PRG, then our unsigned PRG):

```text
c0fc8a9d506ca6082afae614d9dbd9cdc7e87e5f9a2b03bd8fe8025ed073343f
c73b5a4c7c342a6ba12f10cbe51158586db5322ea8ed61bf51429f6c6a9999a8
```

## Runtime observations

[Expected output](../../examples/compiler-number-comparisons/expected.txt)
checks signed minimum/maximum ordering, equal and unequal values, arithmetic
precedence, negative values, Char/Number numeric comparisons, Boolean inequality,
and null inequality. A String and its copied reference compare not-unequal;
String/null compares unequal. This uses the existing identity profile, not a
content-comparison method.

Each new operator's operand functions print `left` before `right`, once each.
The guarded parse function returns 34 for "17", 0 for "18", and 0 for a failed
parse; the failed parse skips its Number comparison and arithmetic. The loop
using `<` stops at 3. SDK/generated logs match byte for byte.

Shared IR validation tests establish that `!= null` proves non-null on the true
edge, with negation reversing the edge. Both operand orders work. Swapped
branches, changing the comparison to equality without changing branches, local
writes between read and branch, stored Boolean guards, and joins lacking a proof
are rejected. These are compiler guarantees, not observations of SDK checking.

## Reproduction

Build/sign our PRG with [the example's commands][example]. For the SDK, create
`scratch/compiler-number-comparisons/monkey.jungle`:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-comparisons
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-comparisons/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-comparisons
cargo run -p ciq-dis -- all \
  scratch/compiler-number-comparisons/build/fr265/compiler-number-comparisons.prg \
  scratch/compiler-number-comparisons/sdk-export
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
cmp examples/compiler-number-comparisons/expected.txt scratch/simulator/monkeydo.log
```

Run separately and retain console/image captures as `sdk.{log,png}` and
`generated.{log,png}` under the specimen directory. Source/unit-IR/assembly paths
produce equivalent complete unsigned PRGs. The SDK manifest/resource scaffold
under `scratch/native-names/` has minimum API 3.0; our generated profile selects
minimum 2.1.0 and target 5.2.0. Physical-watch and other-device behavior are not
established by this run.

[example]: ../../examples/compiler-number-comparisons/README.md
[functions]: https://developer.garmin.com/connect-iq/monkey-c/functions/
[article]: https://developer.garmin.com/connect-iq/articles/monkey-c/Functions.html
[enum]: https://github.com/markw65/monkeyc-optimizer/blob/2920dc88c23caadded9f66ee0613a7de6ab2d1ea/src/readprg/opcodes.ts
