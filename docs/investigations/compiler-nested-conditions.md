# Compiler nested statements and joins

2026-10-10. Question: can source scopes, nested conditions, early returns, and
continuing arms use the existing forward-only IR without losing effect order?
SDK 9.2.0, Java 17, `fr265`, original device memory allowance, our developer key.
This extends our [terminal-condition experiment](compiler-conditions.md) and
uses its unchanged [SDK scaffold](control-flow.md#reproduction). No new external
research was consulted or implementation copied.

## Source construction and scope

The parser now represents statement sequences and nested `if/else` arms. A
separate internal source builder resolves names and emits the existing semantic
blocks, branches, jumps, and returns. No new IR directives or VM opcodes are
needed.

Arm scopes inherit enclosing names and discard new declarations on exit.
Sibling arms may reuse a name, with distinct local identities/frame slots.
Shadowing an enclosing local or parameter remains rejected. Continuing arms
jump to a shared block declared after both arms; returning arms do not contribute
an edge. Both returning arms omit the join. Every function path must return;
following unreachable statements are rejected.

The [source specification](../specs/compiler-subset.md#statement-arms-and-joins)
and [IR specification](../specs/compiler-ir-text.md#control-flow) record these
rules. They are our bounded subset, not a claim of complete Monkey C scoping.

## Simulator specimens and observations

The [checked-in example](../../examples/compiler-probe/nested-conditions.mc)
sets `base = 40`. Its outer true arm initializes `amount = 3`, prints the sum,
and uses a nested `amount > 2` condition. The nested true arm initializes and
prints `result = base + amount`, then returns it; the false arm returns `base`.
The outer false arm separately initializes `amount = 5` and prints its sum.
Continuing execution initializes/prints a new `result = base + 7` and returns.
This deliberately reuses arm-local names without sharing their identities.

Three independent outputs were integrated into the original scaffold:

| Specimen | Source change | CODE bytes | Padding | Returns for 0, 1 | Display |
|---|---|---|---|---|---|
| Nested true | Checked-in example | 128 | 0 | 43, 47 | 53 |
| Nested false | Outer `== 1`, inner `> 3` | 129 | 31 | 47, 40 | 46 |
| Joining arms | Replace nested returns with prints | 164 | 28 | 47, 47 | 53 |

The joining variant replaces `return result;` with
`System.println(result + 1);` and `return base;` with `System.println(base);`.
Both nested arms and both outer arms continue, exercising a two-predecessor
outer join instead of early returns.

**Experimentally observed:** selected-path console sequences agree with source:

- Nested true: `choose(0)` prints `43, 43`, then returns `43`;
  `choose(1)` prints `45, 47`, then returns `47`.
- Nested false: `choose(0)` prints `45, 47`, then returns `47`;
  `choose(1)` prints `43`, then returns `40`.
- Joining: `choose(0)` prints `43, 43, 44, 47`, then returns `47`;
  `choose(1)` prints `45, 47`, then returns `47`.

Scaffold output still reports `countdown(3) = 6`, `caught(false) = 41`, and
`caught(true) = 43`. `probe()` adds `choose(1)` to `6`, producing the displays
above. UI redraws call `probe()` again, explaining later repeated prints.
Console logs and captured displays were inspected. Retained ignored captures:
`scratch/compiler-nested-conditions/simulator-nested-true/`,
`simulator-nested-false/`, and `simulator-joining/` beneath the same directory.

**Verified pipeline and preservation:** for each specimen, native, saved-IR,
and saved-assembly CODE are byte-identical. Appended CODE equals those bytes,
with the complete original 1021-byte prefix intact. DATA only changes the method
word at `0x4a8..0x4ac` from `0x100000a7` to `0x100003fd`. RESOURCES only adds the
listed zero padding to retain 32-byte signature alignment. Other sections,
including EXCEPTIONS, DEBUG, LINE_NUMBERS, and SYMBOLS, remain byte-identical;
developer signatures are regenerated and verified with our key.

SHA-256 for the exact signed outputs:

```text
nested-true  35953d273db98ffd95ca8ad220ce2e934b39530ba94159063352cd3cfe5ef4d2
nested-false 6dc9cd21b8b5e67159182d5bacbbcd8211f0bd6372348488ebfdaffebe97a492
joining      c6a749b39d5d83abd13988c794a57275dcf63e4593d80ca52bd5aaf0c4f4f718
```

## Reproduction

Build the original control-flow PRG using [its instructions][scaffold]. Use new
output paths for repeated runs. For the checked-in specimen:

```sh
cargo build --workspace
mkdir -p scratch/compiler-nested-conditions
target/debug/ciq-dis all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/compiler-nested-conditions/project
target/debug/ciq-compiler examples/compiler-probe/nested-conditions.mc scratch/compiler-nested-conditions/choose.mcasm
target/debug/ciq-compiler --emit ir examples/compiler-probe/nested-conditions.mc scratch/compiler-nested-conditions/choose.ciqir
target/debug/ciq-compiler --emit code --symbols scratch/compiler-nested-conditions/project/010-symbols-5717b015.mcasm examples/compiler-probe/nested-conditions.mc scratch/compiler-nested-conditions/native.bin
target/debug/ciq-compiler --from ir --emit code --symbols scratch/compiler-nested-conditions/project/010-symbols-5717b015.mcasm scratch/compiler-nested-conditions/choose.ciqir scratch/compiler-nested-conditions/ir.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-nested-conditions/project/010-symbols-5717b015.mcasm scratch/compiler-nested-conditions/choose.mcasm scratch/compiler-nested-conditions/assembly.bin
cmp scratch/compiler-nested-conditions/native.bin scratch/compiler-nested-conditions/ir.bin
cmp scratch/compiler-nested-conditions/native.bin scratch/compiler-nested-conditions/assembly.bin
```

Bounded append integration, with explicit print-symbol bindings:

```sh
python3 - <<'PY'
from pathlib import Path
import re
root = Path('scratch/compiler-nested-conditions')
p = root / 'project'
code = p / '003-code-c0debabe.mcasm'
s = code.read_text()
assert s.count('\nchoose:\n') == 1
symbols = (p / '010-symbols-5717b015.mcasm').read_text()
bindings = []
for alias, name in [('__ciq_sdk_System', 'Toybox_System'),
                    ('__ciq_member_println', 'println')]:
    ids = re.findall(r'^\.symbol \S+ "' + re.escape(name)
                     + r'" id=(0x[0-9a-f]+) ', symbols, re.M)
    assert len(ids) == 1
    bindings.append(f'.symbol {alias} "{name}" id={ids[0]}')
code.write_text(s.replace('\nchoose:\n', '\nsdk_choose:\n') + '\n'
                + '\n'.join(bindings) + '\n'
                + (root / 'choose.mcasm').read_text())
lines = p / '005-line_numbers-c0de7ab1.mcasm'
lines.write_text(re.sub(r'\bcode:choose\b', 'code:sdk_choose', lines.read_text()))
added = len((root / 'native.bin').read_bytes())
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
target/debug/ciq-asm project scratch/compiler-nested-conditions/project scratch/compiler-nested-conditions/generated.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-nested-conditions/generated.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/compiler-nested-conditions/generated.prg --device fr265 --observe-seconds 10
```

Create the two source variants with the edits above, then repeat each pipeline
and append integration against a fresh export of the original PRG. Preserve
captures before the next session overwrites `scratch/simulator/`.

## Validation and remaining limits

Instruction-oracle checks cover both early-return directions, one/two-predecessor
joins, initialized locals, empty edge stacks, and ordered effects. Native/text
paths agree. Malformed graphs and source scope/type errors are rejected.
[The source spec](../specs/compiler-subset.md#statement-arms-and-joins) owns the
current grammar; these runtime observations remain limited to the stated inputs.

[scaffold]: control-flow.md#reproduction
