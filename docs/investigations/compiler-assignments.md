# Compiler local reassignment

2026-10-10. Question: can stored locals carry changing integer/string values
through branches while retaining native/text parity and read/write order?
SDK 9.2.0, Java 17, `fr265`, original memory allowance, our developer key. This
uses our [nested-condition scaffold](compiler-nested-conditions.md) and
[string layout/binding](compiler-strings.md); no new external research was used.

## Contract and experiment

`LocalAssign` replaces an initialized local without allocating another identity
or slot. Its value must preserve the initializer's integer/string type. Calls,
prints, or assignments require IR operation order matching statement and
left-to-right expression evaluation. This rejects a saved read defined before a
write but used after it, since dependency lowering would move the read. Source
snapshots use a separate initialized local. The [design][design] explains the
choice; [source syntax][source] and [IR][ir] specify the rules.

The [checked-in source](../../examples/compiler-probe/assignments.mc) initializes
`answer = 40` and `message = "start"`. Both condition arms update the same two
locals: the zero arm adds `3` and stores `"zero"`; the other adds `7` and stores
`"one"`. After the join it prints the selected message/number, increments the
number, and returns it. Parameters remain read-only and Boolean locals are
unsupported. Fixed local types are our bounded profile, not a general Monkey C
language claim.

**Experimentally observed:** generated CODE is 113 bytes, with prologue
`argcincsp 2 2`. Both arms use `lputv 2` for the number and `lputv 3` for the
string; the final increment also writes slot 2. Native, saved-IR, and saved
assembly CODE are byte-identical. The three new literal records occupy 24 DATA
bytes at offsets `0x3323` (`start`), `0x332c` (`zero`), and `0x3334` (`one`).

Append the body to the original control-flow PRG at CODE+`0x3fd`. Redirect its
existing `choose` method through that label; append the literal declarations to
DATA and bind print aliases against existing symbols. Add 23 zero bytes to
RESOURCES, giving total growth 160 and retaining signature alignment.

**Verified preservation:** CODE retains its original 1021-byte prefix and the
appended suffix exactly matches generated CODE. DATA retains its original bytes
except the method word at `0x4a8..0x4ac`, changed from `0x100000a7` to
`0x100003fd`, followed by exactly the three literal records. RESOURCES retains
its original prefix plus padding. All other section payloads, including
EXCEPTIONS, DEBUG, LINE_NUMBERS, and SYMBOLS, are unchanged except the regenerated
developer signature, which verifies with our key.

**Simulator observations:**

| Call | Generated prints | Return |
|---|---|---|
| `choose(0)` | `start`, `zero`, `43` | 44 |
| `choose(1)` | `start`, `one`, `47` | 48 |

Scaffold results stay `6`, `41`, and `43` for countdown/caught calls. `probe()`
adds `choose(1)` to `6`, displaying `54`; redraws repeat that call's prints.
Console logs and display were inspected, retained under ignored
`scratch/compiler-assignments/simulator/`. Signed output SHA-256:

```text
63aa30d0e0e9143fd857b65159d220238872fe4883a626a3be202eb634142a61
```

## Reproduction

Build the unchanged SDK scaffold using [its instructions][scaffold]. Commands
require new outputs; use fresh paths for repeats.

```sh
cargo build --workspace
mkdir -p scratch/compiler-assignments
target/debug/ciq-dis all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/compiler-assignments/project
target/debug/ciq-compiler examples/compiler-probe/assignments.mc scratch/compiler-assignments/choose.mcasm
target/debug/ciq-compiler --emit ir examples/compiler-probe/assignments.mc scratch/compiler-assignments/choose.ciqir
python3 - <<'PY'
from pathlib import Path
root = Path('scratch/compiler-assignments')
data = root / 'project/002-data-da7ababe.mcasm'
literals = [line for line in (root / 'choose.mcasm').read_text().splitlines()
            if line.startswith('.string ')]
data.write_text(data.read_text() + '\n' + '\n'.join(literals) + '\n')
PY
target/debug/ciq-compiler --emit code --symbols scratch/compiler-assignments/project/010-symbols-5717b015.mcasm --data scratch/compiler-assignments/project/002-data-da7ababe.mcasm examples/compiler-probe/assignments.mc scratch/compiler-assignments/native.bin
target/debug/ciq-compiler --from ir --emit code --symbols scratch/compiler-assignments/project/010-symbols-5717b015.mcasm --data scratch/compiler-assignments/project/002-data-da7ababe.mcasm scratch/compiler-assignments/choose.ciqir scratch/compiler-assignments/ir.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-assignments/project/010-symbols-5717b015.mcasm --data scratch/compiler-assignments/project/002-data-da7ababe.mcasm scratch/compiler-assignments/choose.mcasm scratch/compiler-assignments/assembly.bin
cmp scratch/compiler-assignments/native.bin scratch/compiler-assignments/ir.bin
cmp scratch/compiler-assignments/native.bin scratch/compiler-assignments/assembly.bin
```

Apply bounded integration to this export; literals belong in DATA, while CODE
references them by alias:

```sh
python3 - <<'PY'
from pathlib import Path
import json, re
root = Path('scratch/compiler-assignments')
p = root / 'project'
asm = (root / 'choose.mcasm').read_text().splitlines()
literals = [line for line in asm if line.startswith('.string ')]
added_data = sum(len(json.loads(line.split(' ', 2)[2]).encode()) + 4
                 for line in literals)
added_code = len((root / 'native.bin').read_bytes())
padding = (-(added_data + added_code)) % 32
symbols = (p / '010-symbols-5717b015.mcasm').read_text()
bindings = []
for alias, name in [('__ciq_sdk_System', 'Toybox_System'),
                    ('__ciq_member_println', 'println')]:
    ids = re.findall(r'^\.symbol \S+ "' + re.escape(name)
                     + r'" id=(0x[0-9a-f]+) ', symbols, re.M)
    assert len(ids) == 1
    bindings.append(f'.symbol {alias} "{name}" id={ids[0]}')
code = p / '003-code-c0debabe.mcasm'
s = code.read_text()
assert s.count('\nchoose:\n') == 1
code.write_text(s.replace('\nchoose:\n', '\nsdk_choose:\n') + '\n'
                + '\n'.join(bindings) + '\n'
                + '\n'.join(line for line in asm
                            if not line.startswith('.string ')) + '\n')
lines = p / '005-line_numbers-c0de7ab1.mcasm'
lines.write_text(re.sub(r'\bcode:choose\b', 'code:sdk_choose', lines.read_text()))
resources = p / '007-resources-f00d600d.bin'
resources.write_bytes(resources.read_bytes() + bytes(padding))
manifest = p / 'sections.ciq'
s = manifest.read_text()
for kind, growth in [('0xda7ababe', added_data), ('0xc0debabe', added_code),
                     ('0xf00d600d', padding)]:
    pattern = rf'(section {kind} )(\d+)( )'
    assert len(re.findall(pattern, s)) == 1
    s = re.sub(pattern, lambda m: m[1] + str(int(m[2]) + growth) + m[3], s)
manifest.write_text(s)
PY
target/debug/ciq-asm project scratch/compiler-assignments/project scratch/compiler-assignments/generated.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-assignments/generated.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/compiler-assignments/generated.prg --device fr265 --observe-seconds 10
```

## Validation and limits

Rust tests exercise repeated/self assignments, stored snapshots, one/two-arm
updates, nested writes and early returns, source locations, stable slot counts,
type/scope/parameter errors, version gating, native/text parity, non-slot local
IDs, and writes at the operation limit. Direct typed and textual IR construction
both reject stale reads and targets initialized on only one incoming path.
String binding tests check literal offsets and writes to the existing slots.

This establishes the tested integer/string profile on one device. Dynamic local
types, parameter/member writes, assignment expressions, compound operators,
backward edges, loops, and exceptions remain unsupported. Loop validation must
account for repeated writes and initialization without assuming a body executes.

[design]: ../design/compiler-ir.md#mutable-locals
[source]: ../specs/compiler-subset.md#local-reassignment
[ir]: ../specs/compiler-ir-text.md#local-assignment
[scaffold]: control-flow.md#reproduction
