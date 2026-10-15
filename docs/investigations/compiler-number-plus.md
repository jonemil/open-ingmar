# Number unary plus and checked identity

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: SDK and independently generated [apps][example] match all 18 lines
of [expected output][output]. Unary plus preserves minimum/maximum Number,
zero, negative results, and guarded nullable Numbers. Call operands execute
once; the logical operand prints its `flag` marker before its Number result.
Postfix `+1.abs()` and parenthesized mixed/repeated signs agree.

The public [Monkey C reference][reference], reviewed on the experiment date,
lists unary positive in the arithmetic operator table. That documents the
operator's existence; the identity and prefix restrictions below come from
our controlled SDK specimens. Our compiler profile remains Number-only.

## Reproduction and instruction evidence

Compile/sign our program with the example's commands. For SDK comparison,
create `scratch/compiler-number-plus/group.jungle` using the existing ignored
manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-plus
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-plus/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-plus
cargo run -p ciq-dis -- all \
  scratch/compiler-number-plus/build/fr265/compiler-number-plus.prg \
  scratch/compiler-number-plus/sdk-export
```

`Plus.value(n) { return +n; }` has `12 01 16` at CODE+`0x1f3`: load parameter
1 and return, without a unary opcode. Our corresponding complete primitive
body is `35 02 12 01 16`, including frame setup. SDK CODE falls back to raw at
unsupported CODE+`0x141`; exported source line metadata identifies the known
method boundary. No Garmin compiler/SDK implementation was inspected.

Run SDK and generated PRGs separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` and `screen.png` after each run.
Both logs match the fixture exactly, and the generated display returns to idle
after `System.exit()`. The final source compilation produces an unsigned PRG
identical to the specimen used for generated runtime validation.

## Prefix syntax and broader SDK values

For syntax probes use a replacement Plus module with one function
`value(n) { return <expression>; }`, retaining the lifecycle wrapper. SDK
builds reject `+ +n`, `+-n`, `+~n`, and `+!n` as extraneous prefix input.
The grouped equivalents using Numbers in the main example compile and run.
This is why our positive parser consumes a primary/postfix operand rather than
another unparenthesized unary prefix. Contiguous `++` is reserved for the
separate [increment operator](compiler-increments.md), outside unary expressions.

A separate SDK-only module prints `+true`, `+'A'`, `+"text"`, and `+null`.
The build succeeds and the simulator prints `true`, `A`, `text`, `null`, then
`done`. This is broader identity behavior in the tested SDK profile, not
evidence that Garmin requires Number operands. Our compiler rejects those
values deliberately; broader type-preserving positive contracts would need
their own shared source/IR design.

These probes live under ignored `scratch/compiler-number-plus/<probe>`.
Their Jungle moves scaffold paths one directory deeper and selects
`base.sourcePath = .;../app.mc`, with a replacement module in `.`. Build with
that Jungle and the probe directory as the build-script argument. Runtime
claims apply only to the stated SDK/device profile.

## Compiler validation boundaries

`Positive` retains a Number identity in semantic IR, so parsing cannot erase its
type requirement before validation. `.positive` consumes one non-null Number
and defines an unchanged Number. Source candidate tracking, saved IR, and
lowering share that contract. The backend emits the operand once and no
conversion or arithmetic instruction.

Tests cover complete source/resumed IR and unsigned PRG equivalence, assembly
text, identity bytes and source spans, API 1.0.0 operands, wrong types, malformed
saved operands, null-proof invalidation, reads ordered before writes, and
sign/group nesting. Ordinary nested-call stress tests retain their existing
stack budget. Source `+1` is supported, but `.integer +1` remains outside the
saved constant grammar. Compound assignments and increment/decrement are
separate work.

[example]: ../../examples/compiler-number-plus/README.md
[output]: ../../examples/compiler-number-plus/expected.txt
[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
