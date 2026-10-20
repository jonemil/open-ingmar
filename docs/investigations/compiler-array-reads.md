# Char-array reads and bounds failures

SDK 9.2.0 and our generated Forerunner 265 application agree on all 14 console
lines in the [example][example]. Array evaluates before index, once each, and a
read produces Char. Opcode `0x10` reads the stacked array/index with no immediate
operand. Isolated SDK bounds probes crash despite `try/catch`; our generated
negative-index probe reports the same fatal error. Experiment date: 2026-10-12.

## Reproduction and instruction evidence

The SDK wrapper at `scratch/compiler-array-reads/sdk.mc` supplies the
[console lifecycle][wrapper]. Its Jungle file selects our example:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-array-reads;sdk.mc
base.resourcePath = ../native-names/resources
```

Build and export the SDK-produced PRG from our sources:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-array-reads/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-array-reads
cargo run -p ciq-dis -- all \
  scratch/compiler-array-reads/build/fr265/compiler-array-reads.prg \
  scratch/compiler-array-reads/sdk-export
```

In that specimen, CODE+`0x46` contains `12 01 37 10 0f 02 02`: load local slot 1,
zero, read, then invoke the surrounding println. `Arrays.first` contains
`12 01 37 10 16` at CODE+`0x20c`, directly returning the read result. These
controlled sources identify the one-byte read opcode. The assembly mnemonic
`agetv` and successful stack contract are documented in [the instruction spec].
Other unsupported SDK instructions can still cause raw CODE export.

Build/sign our generated PRG with the example's commands. Run each PRG in the
simulator container:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` after each run; compare both complete
logs with the example's expected 14 lines. The observable helper order is
`receiver`, `index`, then `y`. Unicode reads, postfix conversions, a bounds-safe
loop, guarded nullable indices, and a logical index expression also agree.
No Garmin compiler/SDK implementation was inspected.

## Bounds probes

The ignored SDK-only bounds source uses a two-element Char array and tests one
read per session:

```monkeyc
using Toybox.System;
function start() {
    System.println("before negative");
    var a = "ab".toCharArray();
    try { System.println(a[-1]); }
    catch (e) { System.println("caught"); }
    System.println("after");
    System.exit();
}
```

Place this and the same lifecycle wrapper in
`scratch/compiler-array-reads/bounds/`. Its Jungle file uses:

```text
project.manifest = ../../native-names/manifest.xml
base.sourcePath = .
base.resourcePath = ../../native-names/resources
```

Build with `CIQ_JUNGLE_FILES` pointing to that file and
`sh scripts/build-example.sh scratch/compiler-array-reads/bounds`. Run the
resulting `build/fr265/bounds.prg` separately for these substitutions:

| Array contents | Index | Observed result |
|---|---|---|
| `ab` | `-1` | Fatal bounds error |
| `ab` | `2` | Fatal bounds error |
| empty | `0` | Fatal bounds error |

Each SDK run prints its `before` marker, reports `Array Out Of Bounds Error`
and an app crash, and exits monkeydo with status 2. Neither `caught` nor `after`
appears. The simulator display shows the crashed application indicator.
An active session with no output is insufficient evidence; use the 15-second
observation window and inspect the error itself.

Our independent negative-index source omits the unsupported exception syntax;
otherwise it performs the same read. Compile/sign it through the normal unit
PRG path. Its log likewise prints `before negative`, reports the fatal bounds
error, and never prints `after`. Generated upper/empty failures were not tested
separately; the successful profile uses the same read instruction. No index
clamping or negative-index adjustment is emitted.

## Validation boundaries

Saved `.array-read` retains both operands and its Char result. Pipeline checks
compare source/resumed IR, unsigned PRGs, and assembly text. Shared validation
rejects wrong receivers/index types and invalidated or missing null proofs.
Adversarial IR that reorders two bounds-sensitive reads is rejected even without
calls, writes, or printing. This preserves failure order as well as successful
operand effects.

Tests cover exact primitive read bytes, source spans, API introduction checks,
and combined indexing/member-call nesting. The long-arithmetic boundary test
continues to pass with ordinary thread stack limits; separate read emission
keeps that existing recursion budget intact.

These bounds observations cover Char-array reads on this runtime. They do not
establish other receiver types or VM/device behavior; bounds are not statically
proven. Array reverse/slice contracts are recorded in the
[follow-up investigation](compiler-array-transforms.md).

[example]: ../../examples/compiler-array-reads/README.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[the instruction spec]: ../specs/vm/README.md#agetv
[plan]: ../plan.md
