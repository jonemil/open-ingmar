# Compiler zero-argument user module calls

2026-10-10. Question: can our compiler call a scaffold-supplied function, retain
its integer result, and preserve observable call order? Result: independent
source/IR/assembly lowering, explicit symbol binding, own-key signing, and
official simulator execution are verified. Scope is zero-argument top-level
user-module calls, not general function or project compilation.

## Controlled SDK observations

SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`. Start from Hello World and
replace its global probe with:

```monkeyc
function probe() {
    var value = Numbers.answer();
    return value + Numbers.answer() - 40;
}
module Numbers {
    function answer() { return 40 + 3; }
}
```

**Experimentally observed:** the SDK caller occupies CODE `[0x30, 0x58)` and
begins `argcincsp 0x01 0x01`, then resolves globals into slot 0. Each call uses
`getmv @sym_00000002 @answer; invokemz`. The first result is stored in slot 1,
then loaded while the second call runs. Subsequent arithmetic produces `46`. The
helper begins at `0x58`.

The symbol table identifies module `globals/Numbers` with ID `0x00000002`, short
field `Numbers` with ID `0x00000082`, and `answer` with ID `0x00000084`. The
module lookup operand uses the first identity. These IDs are specimen-dependent,
not compiler constants. This matters for binding: the short field is not a
substitute for the module symbol.

Sources informing the independent implementation are
[our instruction reference](../specs/instructions.md),
[local-variable evidence](compiler-locals.md), and this controlled SDK output.
No external implementation code was incorporated.
[Frontend contract](../specs/compiler-subset.md) and
[IR text](../specs/compiler-ir-text.md) were extended before implementation.

## Runtime specimen and independent output

For observable order, a second SDK scaffold additionally supplies:

```monkeyc
function left() { System.println("left"); return 40 + 3; }
function right() { System.println("right"); return 40 + 5; }
```

Both functions live inside `Numbers`. Its original `probe()` still uses
`answer()` and would return `46`. Our tracked
[calls.mc](../../examples/compiler-probe/calls.mc) replaces the selected probe
with:

```monkeyc
function probe() {
    var value = Numbers.left();
    return value + Numbers.right() - 39;
}
```

Generated symbolic assembly:

```text
probe:
argcincsp 0x01 0x01
getmv @__ciq_module_Numbers @__ciq_member_left
invokemz
lputv 0x01
lgetv 0x01
getmv @__ciq_module_Numbers @__ciq_member_right
invokemz
addv
ipush1 0x27
subv
return
```

Bound CODE is 32 bytes:
`4a 01 01 45 00 00 00 02 00 00 00 85 48 13 01 12 01 45 00 00 00 02 00 00 00 86 48 03 38 27 04 16`.
Here `left`/`right` have IDs `0x85`/`0x86`.

**Experimentally observed:** simulator console shows greeting, `left`, `right`,
then `49`; subsequent view calls repeat `left`, `right`. Inspected screen
displays `49`. A second independently compiled caller
`return Numbers.right() - Numbers.left();` uses `argc 0x01`, two calls,
subtraction, and return (24 bytes). Console shows greeting, `right`, `left`,
then `2`, and repeats this order; inspected screen displays `2`. Both bodies
execute through the existing lifecycle/view call sites. The changed results
distinguish them from the retained SDK probe.

**Interpretation:** `getmv`/`invokemz` supports these zero-argument
integer-return module calls, including calls with a prior arithmetic operand on
the evaluation stack and with/without local allocation. Runtime prints support
left-to-right order for these call expressions. They do not establish arbitrary
receivers, arguments, exceptions, return types, recursion, or general physical
frame layout. The helper implementations, module/method DATA records, and
print/string generation are still SDK-provided; this is not frontend support for
`System.println`.

## IR and binding boundaries

`ModuleCall` defines an integer result from symbolic module/member names.
`.call %<result> <module> <member>` has optional source spans. Every expression
definition has one use; storing a call result in a local allows repeated reads
without repeated invocation. Calls are effectful. For call-containing functions,
shared validation requires all operations to match left-to-right expression
traversal and ordered initialization/return sinks. It rejects incompatible IR
schedules rather than changing effects. Pure-only functions keep their earlier
definition-order flexibility. This conservative limitation can evolve with a
general scheduling/effect policy.

The compiler emits generated aliases `__ciq_module_<name>` and
`__ciq_member_<name>`. `symbol_bindings` resolves these by unique stored names
`globals/<name>` and `<name>` in an explicit metadata-only `.symbol` source;
ordinary assembly aliases retain their declared IDs.
Missing/ambiguous/conflicting bindings, duplicate declarations, and non-symbol
source items fail before CODE output creation. Numeric IDs and DATA records are
not allocated. The CLI exposes this as `--symbols <file>` for CODE emission from
source, IR, or assembly. Earlier symbolic output and self-contained pure CODE
commands remain unchanged.

## Reproduction

Run from the repository root with new output paths and local SDK/device data/key
available:

```sh
mkdir -p scratch/compiler-calls
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler-calls/sdk-example')
shutil.copytree('examples/hello-world', root, ignore=shutil.ignore_patterns('build'))
p = root / 'source/HelloWorld.mc'
s = p.read_text().replace('    return 41;', '    var value = Numbers.answer();\n    return value + Numbers.answer() - 40;')
s += '\nmodule Numbers {\n    function answer() { return 40 + 3; }\n}\n'
p.write_text(s)
PY
sh scripts/build-example.sh scratch/compiler-calls/sdk-example fr265
target/debug/ciq-dis all scratch/compiler-calls/sdk-example/build/fr265/sdk-example.prg scratch/compiler-calls/sdk-project
python3 - <<'PY'
from pathlib import Path
import shutil
root = Path('scratch/compiler-calls/runtime-example')
shutil.copytree('scratch/compiler-calls/sdk-example', root, ignore=shutil.ignore_patterns('build'))
p = root / 'source/HelloWorld.mc'
p.write_text(p.read_text().replace('    function answer() { return 40 + 3; }', '''    function answer() { return 40 + 3; }
    function left() { System.println("left"); return 40 + 3; }
    function right() { System.println("right"); return 40 + 5; }'''))
PY
sh scripts/build-example.sh scratch/compiler-calls/runtime-example fr265
target/debug/ciq-dis all scratch/compiler-calls/runtime-example/build/fr265/runtime-example.prg scratch/compiler-calls/runtime-project
target/debug/ciq-compiler --emit ir examples/compiler-probe/calls.mc scratch/compiler-calls/probe.ciqir
target/debug/ciq-compiler --from ir scratch/compiler-calls/probe.ciqir scratch/compiler-calls/probe.mcasm
target/debug/ciq-compiler --emit code --symbols scratch/compiler-calls/runtime-project/010-symbols-5717b015.mcasm examples/compiler-probe/calls.mc scratch/compiler-calls/native.bin
target/debug/ciq-compiler --from ir --emit code --symbols scratch/compiler-calls/runtime-project/010-symbols-5717b015.mcasm scratch/compiler-calls/probe.ciqir scratch/compiler-calls/ir.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-calls/runtime-project/010-symbols-5717b015.mcasm scratch/compiler-calls/probe.mcasm scratch/compiler-calls/text.bin
cmp scratch/compiler-calls/native.bin scratch/compiler-calls/ir.bin
cmp scratch/compiler-calls/native.bin scratch/compiler-calls/text.bin
python3 - <<'PY'
from pathlib import Path
Path('scratch/compiler-calls/reversed.mc').write_text('function probe() { return Numbers.right() - Numbers.left(); }\n')
PY
target/debug/ciq-compiler scratch/compiler-calls/reversed.mc scratch/compiler-calls/reversed.mcasm
target/debug/ciq-compiler --emit code --symbols scratch/compiler-calls/runtime-project/010-symbols-5717b015.mcasm scratch/compiler-calls/reversed.mc scratch/compiler-calls/reversed.bin
```

Specific append integration for the measured 855-byte CODE/7770-byte RESOURCES
scaffold. Inline declarations bind compiler aliases to existing IDs; only the
manifest-selected symbol source is serialized, so SYMBOLS remains unchanged:

```sh
python3 - <<'PY'
from pathlib import Path
import shutil, re
root = Path('scratch/compiler-calls')
for name, payload in [('probe', 'native.bin'), ('reversed', 'reversed.bin')]:
    project = root / f'{name}-project'
    shutil.copytree(root / 'runtime-project', project)
    size = len((root / payload).read_bytes())
    padding = (-size) % 32
    manifest = project / 'sections.ciq'
    text = manifest.read_text()
    assert '0xc0debabe 855 assembly' in text and '0xf00d600d 7770 raw' in text
    symbols = (project / '010-symbols-5717b015.mcasm').read_text()
    bindings = []
    for alias, stored in [('__ciq_module_Numbers', 'globals/Numbers'), ('__ciq_member_left', 'left'), ('__ciq_member_right', 'right')]:
        ids = re.findall(r'^\.symbol \S+ "' + re.escape(stored) + r'" id=(0x[0-9a-f]+) ', symbols, re.M)
        assert len(ids) == 1
        bindings.append(f'.symbol {alias} "{stored}" id={ids[0]}')
    code = project / '003-code-c0debabe.mcasm'
    body = code.read_text()
    assert body.count('\nprobe:\n') == 1
    code.write_text(body.replace('\nprobe:\n', '\nprobe_original:\n') + '\n' +
                   '\n'.join(bindings) + '\n' + (root / f'{name}.mcasm').read_text())
    lines = project / '005-line_numbers-c0de7ab1.mcasm'
    lines.write_text(lines.read_text().replace('pc=code:probe ', 'pc=code:probe_original '))
    resources = project / '007-resources-f00d600d.bin'
    resources.write_bytes(resources.read_bytes() + bytes(padding))
    manifest.write_text(text.replace('0xc0debabe 855 assembly', f'0xc0debabe {855 + size} assembly')
                           .replace('0xf00d600d 7770 raw', f'0xf00d600d {7770 + padding} raw'))
PY
target/debug/ciq-asm project scratch/compiler-calls/probe-project scratch/compiler-calls/probe-49.prg -y keys/developer_key.der
target/debug/ciq-asm project scratch/compiler-calls/reversed-project scratch/compiler-calls/reversed-2.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-calls/probe-49.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-calls/reversed-2.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-calls/probe-49.prg
sh scripts/run-simulator-container.sh --prg scratch/compiler-calls/reversed-2.prg
```

Actual simulator execution reused the existing Garmin container through
container-broker with
`xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`.
Captures/logs were copied after each run from `scratch/simulator/` to
`scratch/compiler-calls/simulator-49/` and `simulator-reversed-2/`; both screens
were inspected.

## Preservation and validation

Independent wrapper parsing confirms original CODE is an exact 855-byte prefix
and the generated body begins at `0x357`. DATA's `probe` value at `0x4b1`
changes from `0x10000030` to `0x10000357`, preserving all other DATA bytes. CODE
becomes 887/879 bytes. Positive output leaves RESOURCES unchanged; reversed
output appends eight zeros, giving 32 bytes combined growth in either case and
preserving signature-payload alignment modulo 32.

Other non-signature sections, including SYMBOLS and helper metadata, are byte
identical. Existing debug/line mappings stay with the original body; no new
mapping is generated. Empty EXCEPTIONS and zero header CODE offsets remain
unchanged. Both developer signatures verify. This is bounded append integration,
not general relocation. The positive program round-trips exactly:

```sh
target/debug/ciq-dis all scratch/compiler-calls/probe-49.prg scratch/compiler-calls/roundtrip-project
target/debug/ciq-asm project scratch/compiler-calls/roundtrip-project scratch/compiler-calls/roundtrip.prg
cmp scratch/compiler-calls/probe-49.prg scratch/compiler-calls/roundtrip.prg
```

| Artifact | SHA-256 |
|---|---|
| First SDK call specimen | `67ad10218d226a84002c362586854b31e52c2b17e3d0327cf98ca6da859ea707` |
| Runtime SDK scaffold | `c14b9f64cd79efc11f8d87f802ac9ca723998d71181cbb850166551a90ae3377` |
| Native `49` CODE | `456aa8fbd0aefbf0b87111052226afeef379d993d62fef28253d517ff93533aa` |
| Signed `49` PRG | `9b87fea885d96990dc7f993c88fd58ca23770afe31d62bd08255160018f1d68e` |
| Signed reversed `2` PRG | `42d926d0cf851aa73ce67a4183e6a517f176adb81299e2533060aa801c84f1f0` |

Six new tests cover call/result ordering across native/text paths,
names/spans/exact encodings, effectful IR schedule rejection, unsupported
source/IR shapes, absent/ambiguous/conflicting symbol bindings, and actual CLI
resumption/binding failures. Synthetic effectful integer callees in the
instruction test make dropped/repeated/reordered calls observable; this is not a
full VM/frame emulator. All 142 workspace tests pass with Rust 1.99, plus
formatting and workspace/all-target Clippy with warnings denied.

Next investigate parameter slots and a first argument-bearing call with
controlled specimens, then string/member calls such as `System.println`.
Multi-function/module compilation, metadata generation, recursion, and
project/Jungle builds remain later work.
