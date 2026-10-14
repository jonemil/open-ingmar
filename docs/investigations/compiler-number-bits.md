# Number bit operations, precedence, and shifts

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: SDK and independently generated [apps][example] match all 25 lines
of [expected output][output]. Number AND/OR/XOR operate on 32-bit values.
`1 << 31` yields minimum Number; `-8 >> 1` yields -4 and minimum Number
shifted right by 31 yields -1. Source complement of zero and minimum Number
agrees with our XOR-with-minus-one lowering.

## Public reference and precedence probes

The public [Monkey C reference][reference], reviewed on the experiment date,
lists shifts and AND with multiplication/division/remainder, above addition.
Its precedence table omits OR/XOR although the operator tables describe them.
The underlying public article is
[Monkey_C_Reference.html][article]; no SDK implementation was inspected.

Parameter-based SDK probes establish these additional groupings:

| Expression and inputs | Result | Observed grouping |
|---|---|---|
| `a + b << c`, 1/2/2 | 9 | `a + (b << c)` |
| `a << b * c`, 2/1/3 | 12 | `(a << b) * c` |
| `a & b * c`, 6/3/2 | 4 | `(a & b) * c` |
| `a + b \| c`, 1/1/2 | 2 | `(a + b) \| c` |
| `a ^ b + c`, 3/1/2 | 4 | `(a ^ b) + c` |
| `a \| b ^ c`, 1/2/3 | 0 | `(a \| b) ^ c` |
| `a \| b & c`, 1/2/2 | 3 | `a \| (b & c)` |
| `a \| b == c`, 1/2/3 | true | `(a \| b) == c` |

Interpretation: OR/XOR share addition's left-associative level; AND/shifts
share multiplication's level. These observations guide our source grammar,
rather than importing C precedence. Calls retain left-to-right operand order,
including when the right operand needs logical-value staging.

## Reproduction and instruction evidence

Compile/sign our program with the example's commands. Create an ignored SDK
Jungle using the existing manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-bits
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-bits/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-bits
cargo run -p ciq-dis -- all \
  scratch/compiler-number-bits/build/fr265/compiler-number-bits.prg \
  scratch/compiler-number-bits/sdk-final-export
```

Our source-correlated SDK method bodies contain `12 01 12 02 <opcode> 16`:
load parameters 1 and 2, operate, return. The full CODE export falls back to
raw at unsupported opcode CODE+`0x2d1`; the known method boundaries remain
inspectable through the exported line metadata.

| Source function | CODE offset | Opcode | Independent mnemonic |
|---|---|---|---|
| `Bits.both` | `0x33a` | `07` | `andv` |
| `Bits.either` | `0x370` | `08` | `orv` |
| `Bits.different` | `0x3ce` | `0c` | `xorv` |
| `Bits.left` | `0x39f` | `0a` | `shlv` |
| `Bits.right` | `0x32a` | `0b` | `shrv` |

The successful abstract stack is two Numbers to one Number. Run SDK and
generated PRGs separately with the existing helper:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` and `screen.png` after each session.
Both logs exactly match the expected fixture, including `left`, `right`, then
11 for OR and `left`, `flag`, then 4 for the staged shift. The generated
display returns to idle after `System.exit()`.

## Count boundaries

Create an isolated replacement Bits module in
`scratch/compiler-number-bits/counts`, retaining the two parameter-based shift
functions and the example's AppBase lifecycle. Its run method prints each of
these calls in order, with markers before each count group:

| Calls | SDK/generated results |
|---|---|
| `left(1, -1)`, `right(-8, -1)` | -2147483648, -1 |
| `left(3, 32)`, `right(-8, 32)` | 3, -8 |
| `left(3, 33)`, `right(-8, 33)` | 6, -4 |

The probe's Jungle moves scaffold paths one directory deeper and selects
`base.sourcePath = .;../../../examples/compiler-number-bits/app.mc`. Build
with that Jungle and `sh scripts/build-example.sh scratch/compiler-number-bits/counts`.
Compile/sign the lifecycle and replacement module through the normal unit
PRG path, then run SDK/generated PRGs separately.

Both sessions complete and match all ten lines, including markers and `done`.
Interpretation: these counts agree with selecting their low five bits. No
compiler masking, clamping, or constant folding is emitted. This does not prove
all count boundaries or behavior on other VM/device versions.

## Compiler boundaries

Shared binary contracts require non-null Numbers. Tests cover exact parameter
instruction sequences, source/resumed IR and unsigned PRG equivalence, assembly
text, wrong operand types, guarded nullable reads, malformed saved operands,
unusual precedence, left associativity, and parser limits. Complement becomes
ordinary typed XOR with a separate -1 constant; no extra saved IR directive or
VM complement opcode is needed. Its operand executes once.

Complement construction uses a separate parser helper to retain the ordinary
nested-call stack budget. The existing nested-call boundary and long binary
expression tests pass without enlarged thread stacks. Unary plus, compound
assignments, increment/decrement, wider numeric values, and unsigned-right
shift remain outside this profile.

[example]: ../../examples/compiler-number-bits/README.md
[output]: ../../examples/compiler-number-bits/expected.txt
[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[article]: https://developer.garmin.com/connect-iq/articles/reference-guides/Monkey_C_Reference.html
