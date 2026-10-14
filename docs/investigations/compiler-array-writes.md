# Number-array replacement and evaluation order

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: the SDK and independently generated [example][example] produce the
same 14-line [output][output]. Receiver, index, and value evaluate once in that
order. Replacement preserves length and is visible through aliases. A value
call that removes an element runs before the write uses its recorded index.
Logical value staging, loop writes, and guarded nullable Number values agree.

The public [Containers article][containers], reviewed on the experiment date,
documents indexed array assignment in a multidimensional initialization example.
It does not establish our runtime evaluation/error ordering. Our invariant
Number-array profile is narrower than the documented container language.

## Reproduction and instruction evidence

Use the example's commands for our compiler. For an SDK comparison, create
`scratch/compiler-array-writes/monkey.jungle` using the existing ignored
manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-array-writes
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-array-writes/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-array-writes
cargo run -p ciq-dis -- all \
  scratch/compiler-array-writes/build/fr265/compiler-array-writes.prg \
  scratch/compiler-array-writes/sdk-export
```

Only our source specimen was compiled and inspected. No Garmin compiler/SDK
implementation was disassembled or decompiled. The SDK export falls back to
raw CODE at unsupported opcode CODE+`0xb3`; this does not prevent inspecting
the known write sequence at a source-correlated method boundary.

`Writes.fill` at CODE+`0x1c7` contains
`12 01 12 02 12 02 38 0a 05 11`: load array slot 1, index slot 2, index slot 2,
Number 10, multiply, then `aputv`. The write consumes array/index/value without
producing a result. Our direct parameter-write test independently emits
`35 04 12 01 12 02 12 03 11 16`, including frame setup and Void return.

Run each signed PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` and `screen.png` after each session.
Both successful console logs exactly match the expected fixture; the generated
display returns to the idle simulator after `System.exit()`.

## Bounds failures

For isolated probes replace the example's Writes module with:

```monkeyc
using Toybox.System;
module Writes {
    function receiver(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> {
        System.println("receiver"); return a;
    }
    function index() { System.println("index"); return -1; }
    function value() { System.println("value"); return 9; }
    function run() {
        var a = [1, 2];
        Writes.receiver(a)[Writes.index()] = Writes.value();
        System.println("after");
    }
}
```

Place it in `scratch/compiler-array-writes/negative/writes.mc`. The probe's
Jungle uses `base.sourcePath = .;../../../examples/compiler-array-writes/app.mc`
and moves manifest/resource paths one directory deeper. Build with that Jungle
and `sh scripts/build-example.sh scratch/compiler-array-writes/negative`.
Compile/sign the same two sources through our normal unit PRG path. For the
SDK upper-bound probe use a separate directory and change the index to 2.

Both SDK indices print `receiver`, `index`, and `value`, then report
`Array Out Of Bounds Error` and an app crash; `after` never appears. The
generated negative-index probe prints the same markers and error, and its
display shows the crashed-app indicator. Interpretation: bounds are checked
at the write, after operand effects; a length-equal write does not append.
Generated upper-bound failure was not separately run. No exception recovery
or behavior on other VM/device versions is established.

## Compiler validation boundaries

Focused checks cover source/resumed IR and unsigned PRG equivalence, assembly
text, exact primitive bytes/spans, API 1.0.0 parameter writes, malformed saved
operands, wrong types, missing/invalidated null proofs, and read/write effect
order. Plain writes work at the existing frame limit; staged hidden locals
consume that budget and diagnose overflow. Char/View writes and broader Array
contracts remain separate follow-up work.

[example]: ../../examples/compiler-array-writes/README.md
[output]: ../../examples/compiler-array-writes/expected.txt
[containers]: https://developer.garmin.com/connect-iq/monkey-c/containers/
