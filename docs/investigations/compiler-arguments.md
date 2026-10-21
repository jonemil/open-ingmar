# Compiler parameters and positional arguments

2026-10-10. Question: can our compiler pass integer arguments through nested
calls, then read them correctly in an independently compiled callee with locals?
Result: controlled SDK comparison, native/text equivalence, generated
caller/callee execution, signing/preservation checks, and simulator results `48`
and `-40` are verified.

## Scope and controlled observations

SDK/compiler/simulator 9.2.0; `fr265`; `-O 0`. Independent Rust implementation
against [our instruction reference](../specs/vm/README.md#instruction-reference),
[local slots](compiler-locals.md), and [zero-argument calls](compiler-calls.md),
without incorporating external implementation code. The
[grammar/lowering contract](../specs/frontend/README.md) was extended before
implementation.

Controlled SDK Hello World probe:

```monkeyc
function probe() {
    return Numbers.difference(Numbers.left(), Numbers.right());
}
module Numbers {
    function left() { System.println("left"); return 40 + 5; }
    function right() { System.println("right"); return 2; }
    function difference(first, second) {
        var adjusted = first + 1;
        return adjusted - second;
    }
}
```

**Experimentally observed:** SDK probe CODE `[0x30, 0x5b)` selects the
difference callable with `getmv`, emits `frpush`, evaluates left/right
zero-argument calls in that order, then uses `invokem 0x03` and returns. This
count is two explicit arguments plus one context slot.

SDK difference CODE `[0x7d, 0x95)` begins `argcincsp 0x03 0x01`, resolves
globals into slot 0, then reads parameter 1 from slot 1. It stores the adjusted
result into local slot 3, reads that local and parameter 2 from slot 2,
subtracts, and returns. Thus parameter slots precede locals. These are
observations for this specimen, not a complete physical frame model.

## Independently generated caller and callee

Tracked [arguments.mc](../../examples/compiler-probe/arguments.mc):

```monkeyc
function probe() {
    var result = Numbers.difference(Numbers.left() + 1, Numbers.right());
    return result + 1;
}
```

Tracked [parameters.mc](../../examples/compiler-probe/parameters.mc):

```monkeyc
function difference(first, second) {
    var adjusted = first + 3;
    return adjusted - second;
}
```

Each is compiled independently to semantic IR, assembly, and CODE. The original
SDK difference adds 1; ours adds 3. The generated caller and callee jointly
return `(45 + 1) + 3 - 2 + 1 = 48`. Executing the generated caller with the
original SDK callee would return `46`; the untouched SDK probe/callee would
return `44`. The observed result therefore distinguishes the generated path.

Generated caller is 46 bytes. It selects difference, pushes context, calls left
and adds 1, calls right, invokes difference with count 3, stores/loads its
result, adds 1, and returns. Generated callee is 16 bytes:

```text
difference:
argcincsp 0x03 0x01
lgetv 0x01
ipush1 0x03
addv
lputv 0x03
lgetv 0x03
lgetv 0x02
subv
return
```

Exact callee bytes: `4a 03 01 12 01 38 03 03 13 03 12 03 12 02 04 16`.

**Experimentally observed:** own-key signed program console shows greeting,
`left`, `right`, `48`; subsequent view calls retain that order. Inspected screen
displays `48`. A second generated caller
`return Numbers.difference(Numbers.right(), Numbers.left());` with the same
generated callee logs `right`, `left`, and displays/prints `-40` (`2 + 3 - 45`).
This distinguishes parameter positions and exercises the no-local caller
prologue as well.

**Interpretation:** nested calls preserve the pending outer call's
context/arguments for these integer-return functions. The extra context slot
participates in both `argc` and `invokem` counts; parameters load from index+1,
and locals follow them. The results do not establish arbitrary receiver binding,
physical context representation, variadic/default parameters, exceptions,
noninteger types, or runtime acceptance at the maximum encodable counts.

Module declarations, DATA method/signature environment, lifecycle/resources, and
printing helpers remain SDK-provided. This experiment compiles and redirects two
independent function bodies; it does not implement module/project generation or
`System.println` in the frontend.

## Implementation boundaries

Native/text checks covered parameter/local slot mapping, repeated reads, nested
argument order, malformed signatures, binding failures, and resource limits.
The test evaluator uses synthetic callees/context markers, not a full VM.
Current contracts are in [source](../specs/frontend/README.md) and
[IR](../specs/ir/README.md#parameter-metadata-and-reads).

## Reproduction

Run from the repository root with new paths and the local SDK/device data/key
available:

```sh
mkdir -p scratch/compiler-arguments
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler-arguments/sdk-example')
shutil.copytree('examples/hello-world', root, ignore=shutil.ignore_patterns('build'))
p = root / 'source/HelloWorld.mc'
s = p.read_text().replace('    return 41;', '    return Numbers.difference(Numbers.left(), Numbers.right());')
s += '''
module Numbers {
    function left() { System.println("left"); return 40 + 5; }
    function right() { System.println("right"); return 2; }
    function difference(first, second) {
        var adjusted = first + 1;
        return adjusted - second;
    }
}
'''
p.write_text(s)
PY
sh scripts/build-example.sh scratch/compiler-arguments/sdk-example fr265
target/debug/ciq-dis all scratch/compiler-arguments/sdk-example/build/fr265/sdk-example.prg scratch/compiler-arguments/sdk-project
target/debug/ciq-compiler --emit ir examples/compiler-probe/arguments.mc scratch/compiler-arguments/probe.ciqir
target/debug/ciq-compiler --from ir scratch/compiler-arguments/probe.ciqir scratch/compiler-arguments/probe.mcasm
target/debug/ciq-compiler --emit code --symbols scratch/compiler-arguments/sdk-project/010-symbols-5717b015.mcasm examples/compiler-probe/arguments.mc scratch/compiler-arguments/probe.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-arguments/sdk-project/010-symbols-5717b015.mcasm scratch/compiler-arguments/probe.mcasm scratch/compiler-arguments/probe-text.bin
target/debug/ciq-compiler --emit ir examples/compiler-probe/parameters.mc scratch/compiler-arguments/difference.ciqir
target/debug/ciq-compiler --from ir scratch/compiler-arguments/difference.ciqir scratch/compiler-arguments/difference.mcasm
target/debug/ciq-compiler --emit code examples/compiler-probe/parameters.mc scratch/compiler-arguments/difference.bin
target/debug/ciq-compiler --from assembly --emit code scratch/compiler-arguments/difference.mcasm scratch/compiler-arguments/difference-text.bin
cmp scratch/compiler-arguments/probe.bin scratch/compiler-arguments/probe-text.bin
cmp scratch/compiler-arguments/difference.bin scratch/compiler-arguments/difference-text.bin
python3 - <<'PY'
from pathlib import Path
Path('scratch/compiler-arguments/reversed.mc').write_text('function probe() { return Numbers.difference(Numbers.right(), Numbers.left()); }\n')
PY
target/debug/ciq-compiler scratch/compiler-arguments/reversed.mc scratch/compiler-arguments/reversed.mcasm
target/debug/ciq-compiler --emit code --symbols scratch/compiler-arguments/sdk-project/010-symbols-5717b015.mcasm scratch/compiler-arguments/reversed.mc scratch/compiler-arguments/reversed.bin
```

Specific append integration for the measured 863-byte CODE/7770-byte RESOURCES
scaffold. Assertions deliberately fail for a different build:

Integrate into a fresh scaffold export: rename the old entry label and its line
references to an unused alias, preserving their body/PCs. Append generated
assembly under the original alias, redirect its method reference, and apply the
DATA/resource lengths and offsets recorded in this record. Reconstruct/sign,
verify our key, and run each case separately using the
[shared procedure](../research.md#shared-compiler-reproduction).

Actual execution reused the existing Garmin container through container-broker
with
`xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`.
Captures/logs were copied from `scratch/simulator/` after each run to
`scratch/compiler-arguments/simulator-48/` and
`simulator-reversed-negative-40/`; both screens were inspected.

## Preservation and checks

Independent wrapper parsing confirms original CODE is an exact 863-byte prefix.
Generated callers start at `0x35f`; generated difference starts at `0x38d` in
the positive program and `0x382` in the reversed program. DATA's probe value at
`0x4b1` becomes `0x1000035f`; difference's value at `0x4e6` becomes
`0x1000038d`/`0x10000382`. All other DATA bytes are preserved.

Combined body growth is 62/51 bytes; RESOURCES gains 2/13 trailing zeros, giving
total growth 64 and preserving signature-payload alignment modulo 32. Only DATA,
CODE, RESOURCES, and DEVELOPER_SIGNATURE change. SYMBOLS, helper
implementations/metadata, existing debug/line mappings, empty EXCEPTIONS, and
zero header CODE offsets stay unchanged. Old line PCs name the preserved
original bodies; no new debug maps are generated. Both signature algorithms
verify against the supplied key. This is bounded scaffold integration, not
general relocation.

Positive output round-trips byte identically:

```sh
target/debug/ciq-dis all scratch/compiler-arguments/probe-48.prg scratch/compiler-arguments/roundtrip-project
target/debug/ciq-asm project scratch/compiler-arguments/roundtrip-project scratch/compiler-arguments/roundtrip.prg
cmp scratch/compiler-arguments/probe-48.prg scratch/compiler-arguments/roundtrip.prg
```

| Artifact | SHA-256 |
|---|---|
| SDK scaffold | `2d43ce78923206cf8e35938133b307f0529090120456774fd92125796cc5c5e6` |
| Generated caller CODE | `c31e50c6382d2c429c56315561c59bc4c91da52fd24728fd42baf72ca40cfa45` |
| Generated callee CODE | `eedbeeeb7473e0899d94237c7618b77c22926a028557ecfcd7320b250e984d5e` |
| Signed `48` PRG | `02df1c0ca1849b78646a60e6f64f6ee44f685ec40bb876c19bb63a25b136363a` |
| Signed `-40` PRG | `e39bb606f3837229e0c59aa42fbd37ee28e7399d4eb357de37d082059bc5d5c2` |

Six new tests cover parameter/local slot mapping, repeated/unused parameters,
nested argument context/order, semantic/IR position and dependency validation,
malformed signatures/lists, source/IR encoding/nesting bounds, and CLI text
resumption. The restricted test evaluator uses synthetic callees and
stack/context markers; it is not a full VM/frame implementation.

## Dynamic IR contracts

2026-10-21; SDK 9.2.0, Java 17, `fr265`, `-O 0`. Authored probes in
`scratch/core-any/source/app.mc` establish that omitted parameter/result
annotations permit String and Null forwarding and dynamic member calls. Bare
`Any` annotations are rejected as an unresolved type. The SDK rejects both a
value-returning branch with implicit fallthrough and mixed value/bare returns;
explicit value/Null returns compile. These are SDK-profile observations, not
claims about all type-checking policies.

The tracked [source and hand-authored IR](../../examples/ir-any/README.md)
compare dynamic scalar calls, arithmetic, indexing/mutation, conditions,
source-class member dispatch, a dynamic argument to a known native method,
runtime-class narrowing, empty/bare-return Null results, explicit mixed
value/Null returns, dynamic Array literals, and a parameter-driven loop.
Our source-compiled and SDK-compiled signed programs match `expected.txt` in the
simulator. Structured CODE/DATA exports reconstruct both complete PRGs byte for
byte, and source/saved-IR/saved-assembly resumption agrees. No new opcode or
DATA record is required. Passing source/runtime artifacts are in
`scratch/source-any/check/`; the hand-authored IR experiment is retained in
`scratch/core-any/check-final/`.

Reproduction: build the saved IR with its README command. The SDK reference uses
`scripts/build-example.sh scratch/core-any` with a local Jungle pointing at
`examples/ir-any` and the console manifest/resources in `scratch/native-names`.
The first SDK run printed the expected values but failed afterward because its
console app omitted `System.exit()`; the tracked specimen includes that call.
Static fixtures now spell their intended Number/Void boundaries explicitly.
Their migrated SDK source sets were rebuilt under SDK 9.2.0 at `-O 0`; local
scaffolds supply imports and external Numbers helpers for standalone fragments.
The immutable-weak fixture requires `-l 0` because the SDK's declared weak-result
types differ from the [observed immutable runtime behavior](compiler-immutable-weak.md).
Rebuild inputs, logs, and the specimen index are under `scratch/source-any/`.
