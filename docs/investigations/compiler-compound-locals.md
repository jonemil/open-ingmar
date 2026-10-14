# Compound local updates and read order

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: SDK and independently generated [apps][example] match all 22 lines
of [expected output][output]. Number locals support `+=`, `-=`, `*=`, `/=`,
`%=`, `&=`, `|=`, `^=`, `<<=`, and `>>=` through their existing binary
contracts. The value helper executes once before the updated Number 13 is
printed. Signed remainder/right shifts, minimum-Number left shift, `n += n`,
loop counters, and a logical RHS also agree.

String concatenation produces `n=21A`; Char additions produce `B`. These
results fit their locals' established types. The compiler does not introduce
new compound coercions or widen the existing binary operand contracts.

The public [Monkey C reference][reference], reviewed on the experiment date,
lists the ten corresponding compound operators as binary operation followed
by assignment. The runtime observations and byte sequence below come from
our controlled sources, not Garmin implementation inspection.

## Reproduction and instruction evidence

Compile/sign our app with the example's commands. For SDK comparison, create
`scratch/compiler-compound-locals/group.jungle` using the existing ignored
manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-compound-locals
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-compound-locals/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-compound-locals
cargo run -p ciq-dis -- all \
  scratch/compiler-compound-locals/build/fr265/compiler-compound-locals.prg \
  scratch/compiler-compound-locals/sdk-export
```

At CODE+`0x79`, the SDK's `n += Compound.value()` starts with
`12 01 43 00 80 01 53 48 03 13 01`: load local slot 1, resolve/invoke the
value helper, add, then store slot 1. The read precedes RHS invocation and the
write follows it. The full export falls back to raw CODE at unsupported
CODE+`0x1ec`; exported source line metadata identifies the known sequence.
No new VM instruction is required.

Run SDK and generated PRGs separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` and `screen.png` after each run.
Both complete logs match the fixture exactly. The generated display returns
to idle after `System.exit()`. No Garmin compiler/SDK implementation was
disassembled or decompiled.

## Compiler representation and boundaries

Local compounds become ordinary local read, binary expression, and local
assignment. Typed/saved IR therefore use the same operand, assignability,
null-proof, and effect-order checks. A branching RHS snapshots the old local
before its effects, using the existing temporary-local/frame budget. Plain
compound operands need no hidden locals. Reordering a saved read after its
RHS call fails validation.

The initializer fixes the local type. A Number receiving a Char-result sum,
or a Char receiving String concatenation, is rejected. Nullable Number locals
need a current proof before the compound read. The write invalidates it just
as ordinary assignment does; the specimen checks again before printing 42.
This conservative rule does not infer a new proof from the Number result.

Tests compare all ten compounds with explicit read/binary/write CODE and check
source/resumed IR, unsigned PRGs, assembly text, spans, invalid types/targets,
scope, nullable reads, effect order, staged locals, and frame limits. Assignment
construction is isolated from RHS parsing so ordinary and compound RHS calls
retain the 64-level nesting limit without enlarged thread stacks.

Compound assignments remain statements on bare locals. Fields and array
elements require separate location/read/failure-order evidence. An SDK-only
probe also rejects `n += (n = 20)` as an assignment-expression syntax error;
our source profile likewise rejects it. Parameters, increment/decrement,
logical compounds, unsigned shifts, and wider numeric types remain outside
this increment.

[example]: ../../examples/compiler-compound-locals/README.md
[output]: ../../examples/compiler-compound-locals/expected.txt
[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
