# Number literal widths and signed values

Question: how are Number constants encoded beyond 127, and how do high bits
behave? Experiments on 2026-10-12: SDK/compiler/simulator 9.2.0, fr265,
optimization disabled. The public [Number API][number] specifies signed 32-bit
values from API 1.0.0. Immediate widths and execution below are our observations.

## SDK probe

An ignored probe prints zero, both i32 limits, and positive/negative boundaries
around signed 8-, 16-, and 24-bit ranges. It uses a local AppBase wrapper whose
`onStart(state)` calls `start()` and whose initial View is
`[new Toybox.WatchUi.View()]`, with fr265 manifest/resources.

```sh
sh scripts/build-example.sh scratch/compiler-number-literals
cargo run -p ciq-dis -- all \
  scratch/compiler-number-literals/build/fr265/compiler-number-literals.prg \
  scratch/compiler-number-literals/wide-probe-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-number-literals/build/fr265/compiler-number-literals.prg
```

**Observed:** the first unsupported instruction was at CODE offset `0x55`, with
bytes `39 00 80` for 128. Larger constants reveal `3a` plus three bytes and `25`
plus four bytes. Our names are `ipush2`, `ipush3`, and `ipush4`; these complete
the existing `ipushz`/`ipush1` family. Payloads are big-endian two's-complement
representations, sign-extended to Number. Representative exact bytes:

| Value | CODE offset | Bytes |
|---|---|---|
| 127 | `0x46` | `38 7f` |
| 128 | `0x55` | `39 00 80` |
| 32767 | `0x85` | `39 7f ff` |
| 32768 | `0x95` | `3a 00 80 00` |
| 2147483647 | `0xc8` | `25 7f ff ff ff` |
| -1 | `0xda` | `38 ff` |
| -128 | `0xe9` | `38 80` |
| -129 | `0xf8` | `39 ff 7f` |
| -32768 | `0x108` | `39 80 00` |
| -32769 | `0x118` | `3a ff 7f ff` |
| -2147483648 | `0x129` | `25 80 00 00 00` |
| 8388607 | `0x13b` | `3a 7f ff ff` |
| 8388608 | `0x14c` | `25 00 80 00 00` |
| -8388608 | `0x15e` | `3a 80 00 00` |
| -8388609 | `0x16f` | `25 ff 7f ff ff` |

Every printed Number matched its input. In particular, `38 ff` produced -1 and
`38 80` produced -128, resolving the earlier high-bit uncertainty. New decoding
permits full CODE export for this specimen. Assembly preserves raw operand bits
and does not rewrite wider instructions to a smaller encoding.

## Source boundary and complete application

The SDK probe also printed -1 for `-1.abs()` and 1 for `(-1).abs()`.
**Interpretation:** postfix calls bind before general unary negation.
**Design decision:** support a minus only on decimal literal magnitudes for now,
and require parentheses around a negative literal used as a call receiver.
The [negation follow-up][negation] adds general Number expression negation and
reveals a deeper literal-postfix discrepancy; the explicit-parentheses
restriction remains; current update syntax follows the source specification.

IR stores canonical decimal i32 values; lowering emits zero specially and
otherwise chooses the smallest signed immediate width. Negative integer zero
canonicalizes to zero. Its text grammar remains separate from the
[current source literal profile](../specs/frontend/README.md#integer-literals).

## Radix and floating token probes

2026-10-20, SDK 9.2.0, `fr265`, `-O0`. Authored literal probes establish:

| Spelling | SDK observation |
|---|---|
| `0xff`, `0xFF`, `077`, `077L` | Accepted; values 255, 255, 63, 63 |
| `0xffffffff` / `0xffffffffL` | Number -1 / Long 4294967295 |
| `-0xffffffff` | CODE uses `ipush1 0x01` |
| `0xffffffffffffffffL` | Accepted full-width Long pattern |
| `-0x80000000`, `-0x8000000000000000L` | Minimum signed patterns retained |
| `01777777777777777777777L` | CODE uses `lpush 0xffffffffffffffff` |
| `1.5f`, `2f`, `.5`, `.5f`, `.5d` | Accepted decimal floating forms |
| `NaN`, `-NaN` | CODE uses `fpush 0x7fc00000` |
| `0XFF`, `0b101`, `08`, `01f`, `01d`, `01.5f`, `1.` | Rejected syntax/radix |
| `1.5F`, `1.5D` | Rejected uppercase suffixes |
| Decimal `2147483648`, `4294967295`, `9223372036854775808L` | Rejected signed ranges |
| `0x100000000`, `040000000000`, `0x10000000000000000L` | Rejected width overflow |

Complete `.mc`, build logs, and accepted PRGs are under
`scratch/compiler-numeric-syntax/`. Copy a named `.mc` into its `source/app.mc`
and run `sh scripts/build-example.sh scratch/compiler-numeric-syntax` to repeat.
`nan-export/`, `nan-neg-export/`, `hex-export/`, and `boundary-export/` retain
structured section exports of the decisive constant encodings. These are
observations of authored SDK application output, not compiler disassembly.

The `CoreLiterals` group in
[source-types.mc](../../examples/compiler-object-tests/source-types.mc) matches
SDK/generated expected output for radix values/negation, leading-dot and suffix
types, NaN's Float type and unequal self-comparison, Float/Long Array storage,
and a Number field default. Source/saved-IR/assembly agree; both PRGs pass
structured reconstruction without raw CODE/DATA fallback and own-key signing
checks. Use the shared [console checker](../../scripts/README.md#console-specimen-check)
with `compiler-object-tests`; artifacts are `scratch/core-numeric-syntax/check/`.
Typed codec tests retain exact bit patterns and reject invalid digits/ranges.
These checks do not establish arbitrary NaN payload propagation or general
arithmetic-overflow rules.

## Decimal immediate reproduction

Create ignored `group.jungle` selecting the [complete example][example] and the
same wrapper/manifest/resources, then build with ordinary SDK checking:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-literals/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-literals
```

SDK and generated applications produced the same 27 console lines. The example
adds typed calls, local replacement, negative arguments, arithmetic across a
width boundary, and Number comparisons. Source/resumed-IR PRGs and assembly
round trips agree. Checks assert exact boundary bytes, truncated operands,
signed source spans, out-of-range diagnostics, and unsupported unary forms.
These do not specify arithmetic overflow or prove older VM compatibility.
The SDK example's complete section export/reconstruction was byte identical,
preserving its signed immediates and signing material.

[number]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Number.html
[example]: ../../examples/compiler-number-literals/README.md

[negation]: compiler-number-negation.md
