# Compiler parameters and positional arguments

2026-10-10. Question: can our compiler pass integer arguments through nested
calls, then read them correctly in an independently compiled callee with locals?
Result: controlled SDK comparison, native/text equivalence, generated
caller/callee execution, signing/preservation checks, and simulator results `48`
and `-40` are verified.

## Scope and controlled observations

SDK/compiler/simulator 9.2.0; `fr265`; `-O 0`. Independent Rust implementation
against [our instruction reference](../specs/instructions.md),
[local slots](compiler-locals.md), and [zero-argument calls](compiler-calls.md),
without incorporating external implementation code. The
[grammar/lowering contract](../specs/compiler-subset.md) was extended before
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

The frontend accepts untyped integer parameter lists and integer expression
arguments. Duplicate parameter names, local shadowing, typed/default parameters,
malformed lists, and local/object receivers are rejected with locations. Call
arity and integer types must match the supplied scaffold; whole-program
arity/type checking is not implemented.

Function IR records a parameter count, default zero. `ParameterRead` defines a
value from a zero-based semantic position; repeated reads create fresh values.
Parameter position is independent of value/local IDs and VM slots.
`Function::with_parameters` and the
[IR text parser](../specs/compiler-ir-text.md) share validation; `Function::new`
preserves zero-parameter construction. Text adds `parameters=<count>` on
`.function`, `.parameter %<result> <index>`, and ordered argument IDs after
`.call %<result> <module> <member>`. Old source/IR/assembly paths stay
compatible.

Backend code selects the callable, emits `frpush` for nonempty argument lists,
evaluates arguments left to right, and emits `invokem` with count+1. Empty calls
retain `invokemz`. Argument dependencies and nested calls participate in the
existing call-containing evaluation-order validator. Counts are limited to 254
parameters/arguments; parameters plus locals to 255; operations to 1024;
combined parenthesis/call nesting to 64. Maximum count tests verify encoding
bounds, not device/runtime limits.

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

```sh
python3 - <<'PY'
from pathlib import Path
import shutil, re
root = Path('scratch/compiler-arguments')
for name in ['probe', 'reversed']:
    project = root / f'{name}-project'
    shutil.copytree(root / 'sdk-project', project)
    size = len((root / f'{name}.bin').read_bytes()) + len((root / 'difference.bin').read_bytes())
    padding = (-size) % 32
    manifest = project / 'sections.ciq'
    text = manifest.read_text()
    assert '0xc0debabe 863 assembly' in text and '0xf00d600d 7770 raw' in text
    symbols = (project / '010-symbols-5717b015.mcasm').read_text()
    bindings = []
    for alias, stored in [('__ciq_module_Numbers','globals/Numbers'), ('__ciq_member_difference','difference'), ('__ciq_member_left','left'), ('__ciq_member_right','right')]:
        ids = re.findall(r'^\.symbol \S+ "' + re.escape(stored) + r'" id=(0x[0-9a-f]+) ', symbols, re.M)
        assert len(ids) == 1
        bindings.append(f'.symbol {alias} "{stored}" id={ids[0]}')
    code = project / '003-code-c0debabe.mcasm'
    body = code.read_text()
    for label in ['probe', 'difference']:
        assert body.count(f'\n{label}:\n') == 1
        body = body.replace(f'\n{label}:\n', f'\n{label}_original:\n')
    code.write_text(body + '\n' + '\n'.join(bindings) + '\n' +
                   (root / f'{name}.mcasm').read_text() + '\n' + (root / 'difference.mcasm').read_text())
    lines = project / '005-line_numbers-c0de7ab1.mcasm'
    linebody = lines.read_text()
    for label in ['probe', 'difference']:
        linebody = linebody.replace(f'pc=code:{label} ', f'pc=code:{label}_original ')
    lines.write_text(linebody)
    resources = project / '007-resources-f00d600d.bin'
    resources.write_bytes(resources.read_bytes() + bytes(padding))
    manifest.write_text(text.replace('0xc0debabe 863 assembly', f'0xc0debabe {863 + size} assembly')
                           .replace('0xf00d600d 7770 raw', f'0xf00d600d {7770 + padding} raw'))
PY
target/debug/ciq-asm project scratch/compiler-arguments/probe-project scratch/compiler-arguments/probe-48.prg -y keys/developer_key.der
target/debug/ciq-asm project scratch/compiler-arguments/reversed-project scratch/compiler-arguments/reversed-negative-40.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-arguments/probe-48.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-arguments/reversed-negative-40.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-arguments/probe-48.prg
sh scripts/run-simulator-container.sh --prg scratch/compiler-arguments/reversed-negative-40.prg
```

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
stack/context markers; it is not a full VM/frame implementation. All 148
workspace tests pass on Rust 1.99; formatting and workspace/all-target Clippy
with warnings denied pass.

Follow-up evidence: [strings and printing](compiler-strings.md) and
[compilation units](compiler-unit.md).
