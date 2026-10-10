# Compiler compilation units

2026-10-10. Question: can multiple source files share declaration/signature
checks and CODE layout while retaining native/text parity and runtime calls?
SDK 9.2.0, Java 17, `fr265`, original memory allowance, our developer key.
This extends [the caller/callee experiment][arguments] using the documented
calling convention. No external implementation was consulted or copied.

## Implementation and boundary

Source files collect global functions and flat module members into a validated
`CompilationUnit`. Qualified calls resolve against all collected signatures,
including forward references. Existing function validation enforces integer
parameters, arguments, and returns; the unit checks ownership and exact argument
counts. It rejects duplicate identities, global/module name collisions, and
unresolved callees. Reopened modules and recursive signatures are accepted;
recursive runtime behavior is not established by this experiment.

Ownership remains separate from the function body. Lowering uses distinct,
length-prefixed CODE labels and internal aliases, combining bodies in source
order. Unit IR adds owner records around existing function documents. Native,
saved-unit-IR, and saved-assembly CODE are byte-identical. See
[design](../design/compiler-ir.md#compilation-units) and
[source/text rules](../specs/compiler-unit.md).

## Simulator observations

The [two-file example](../../examples/compiler-unit/README.md) compiles all four
methods: global `probe`, and module members `difference`, `left`, and `right`.
The helpers print integer markers `1`/`2`, making their generated execution
observable. `difference(a,b)` adds 3 before subtracting, distinguishing it from
the SDK reference's addition of 1. `probe()` prints the computed result before
returning it plus 1.

**Experimentally observed:** console marker order `1`, `2`, result `47`, then
return `48`; the captured screen displays `48`. A second entry passes
`Numbers.right()` followed by `Numbers.left()`, prints their difference, and
returns it without adding 1. The console shows `2`, `1`, `-40`, `-40`, and the
screen displays `-40`. View redraws repeat helper markers and result printing.
Both captures and logs were inspected and retained under ignored
`scratch/compiler-unit/simulator-48/` and
`scratch/compiler-unit/reversed/simulator-negative-40/`.

All four DATA method words select generated bodies. Lifecycle/view methods,
resources, import environment, symbols, and module/class records remain
SDK-provided. This validates the compilation unit step, not independent
complete-application generation.

## Payload preservation

Independent section comparison verifies the original 863-byte CODE prefix,
followed by the exact generated unit bytes. Splitting and separately encoding
its four labelled bodies reproduces the same suffix. DATA changes only four
method words at offsets `0x4b1`, `0x4e6`, `0x4ef`, and `0x4f8` respectively:

| Method | Main size | Main CODE target | Reversed size | Reversed target |
|---|---|---|---|---|
| probe | 61 | `0x35f` | 55 | `0x35f` |
| difference | 16 | `0x39c` | 16 | `0x396` |
| left | 23 | `0x3ac` | 23 | `0x3a6` |
| right | 20 | `0x3c3` | 20 | `0x3bd` |

Method words retain prefix `0x10000000`. Unit CODE growth is 120/114 bytes;
RESOURCES adds 8/14 zero bytes to preserve signature alignment, total growth
128 in either case. All other payloads remain byte-identical except regenerated
developer signatures, which verify with our key. Original method labels and
line references are renamed together so original debug PCs remain intact.

SHA-256:

```text
SDK       85be6040f0382af8449a0c6e898423c167a69e3f7535eb5d7cfa87f9d68106b9
main      9e51011b89d8245bdbcdb0eaf3da61fcf3d8ca6ed1358dba5839fc9d3e90e078
reversed  61c818774b9df99b8a717151b5a0b0771b66cbcb99c85b0dd004112f28812ca9
```

## Reproduction

Use fresh paths for repeats. Recreate the reference application using the
[argument experiment's source preparation][arguments-reproduction], substituting
root `scratch/compiler-unit` for `scratch/compiler-arguments`. Its original
`probe` calls `Numbers.difference(Numbers.left(), Numbers.right())`; the SDK
module's difference adds 1. Build with the same `scripts/build-example.sh`
command, device `fr265`, and output name `sdk-example.prg`.

Generate unit stages and compare CODE:

```sh
cargo build --workspace
target/debug/ciq-dis all scratch/compiler-unit/sdk-example/build/fr265/sdk-example.prg scratch/compiler-unit/project
target/debug/ciq-compiler --unit examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/compiler-unit/unit.mcasm
target/debug/ciq-compiler --unit --emit ir examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/compiler-unit/unit.ciqir
target/debug/ciq-compiler --unit --emit code --symbols scratch/compiler-unit/project/010-symbols-5717b015.mcasm examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/compiler-unit/native.bin
target/debug/ciq-compiler --unit --from ir --emit code --symbols scratch/compiler-unit/project/010-symbols-5717b015.mcasm scratch/compiler-unit/unit.ciqir scratch/compiler-unit/ir.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-unit/project/010-symbols-5717b015.mcasm scratch/compiler-unit/unit.mcasm scratch/compiler-unit/assembly.bin
cmp scratch/compiler-unit/native.bin scratch/compiler-unit/ir.bin
cmp scratch/compiler-unit/native.bin scratch/compiler-unit/assembly.bin
```

Append integration binds all four existing method records to unit labels:

```sh
python3 - <<'PY'
from pathlib import Path
import re
r = Path('scratch/compiler-unit')
p = r / 'project'
code = p / '003-code-c0debabe.mcasm'
s = code.read_text()
symbols = (p / '010-symbols-5717b015.mcasm').read_text()
bindings = []
for alias, name in [('__ciq_module_Numbers', 'globals/Numbers'),
                    ('__ciq_member_difference', 'difference'),
                    ('__ciq_member_left', 'left'), ('__ciq_member_right', 'right'),
                    ('__ciq_sdk_System', 'Toybox_System'),
                    ('__ciq_member_println', 'println')]:
    ids = re.findall(r'^\.symbol \S+ "' + re.escape(name) + r'" id=(0x[0-9a-f]+) ', symbols, re.M)
    assert len(ids) == 1
    bindings.append(f'.symbol {alias} "{name}" id={ids[0]}')
data = p / '002-data-da7ababe.mcasm'
d = data.read_text()
lines = p / '005-line_numbers-c0de7ab1.mcasm'
l = lines.read_text()
for name in ['probe', 'difference', 'left', 'right']:
    assert s.count(f'\n{name}:\n') == 1
    s = s.replace(f'\n{name}:\n', f'\nsdk_{name}:\n')
    l = re.sub(r'\bcode:' + name + r'\b', 'code:sdk_' + name, l)
    label = '__ciq_fn_g_5_probe' if name == 'probe' else f'__ciq_fn_m_7_Numbers_{len(name)}_{name}'
    assert len(re.findall(r'\bvalue=code:' + name + r'\b', d)) == 1
    d = re.sub(r'\bvalue=code:' + name + r'\b', 'value=code:' + label, d)
code.write_text(s + '\n' + '\n'.join(bindings) + '\n' + (r / 'unit.mcasm').read_text())
data.write_text(d)
lines.write_text(l)
added = len((r / 'native.bin').read_bytes())
padding = (-added) % 32
resources = p / '007-resources-f00d600d.bin'
resources.write_bytes(resources.read_bytes() + bytes(padding))
manifest = p / 'sections.ciq'
s = manifest.read_text()
for kind, growth in [('0xc0debabe', added), ('0xf00d600d', padding)]:
    pattern = rf'(section {kind} )(\d+)( )'
    assert len(re.findall(pattern, s)) == 1
    s = re.sub(pattern, lambda m: m[1] + str(int(m[2]) + growth) + m[3], s)
manifest.write_text(s)
PY

target/debug/ciq-asm project scratch/compiler-unit/project scratch/compiler-unit/generated.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-unit/generated.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/compiler-unit/generated.prg --device fr265 --observe-seconds 10
```

For the second entry, save this source in ignored `scratch/`:

```monkeyc
function probe() {
    var result = Numbers.difference(Numbers.right(), Numbers.left());
    System.println(result);
    return result;
}
```

Repeat all stages and append integration under root
`scratch/compiler-unit/reversed`, using that entry and the same tracked module
file, against a fresh export of the reference PRG. Save each simulator capture
before the next session overwrites `scratch/simulator/`.

## Tests and next work

Seven tests cover cross-file forward calls, source/typed/text checks, input-file
locations, duplicate/conflicting declarations, reserved module names, reopened
modules, recursive signature resolution, unique internal/string labels, text
framing/comments/CRLF, the combined 256-function bound, and CLI stage resumption
without overwriting files. Runtime order, call frames, and results use the
simulator as the compatibility oracle.

Next investigate the smallest application's lifecycle, receiver/API behavior,
and metadata requirements, then generate them independently. Bare global calls,
classes, arbitrary SDK signatures, resource generation, complete PRG production,
and Jungle resolution remain future work. The source and IR profiles are still
provisional; no new crate or linker was introduced.

[arguments]: compiler-arguments.md
[arguments-reproduction]: compiler-arguments.md#reproduction
