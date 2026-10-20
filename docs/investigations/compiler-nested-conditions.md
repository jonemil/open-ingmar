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

The [source specification](../specs/frontend/README.md#statement-arms-and-joins)
and [IR specification](../specs/ir/README.md#control-flow) record these
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

## Standalone scopes and else-if chains

2026-10-20, SDK 9.2.0, `fr265`, `-O0`. An authored SDK probe accepts sibling
standalone blocks reusing a local name and an `else if` chain. The grouped
`CoreScopes` specimen in
[source-types.mc](../../examples/compiler-object-tests/source-types.mc) then
compares complete SDK/generated applications:

- Sibling locals contribute `3`; reusing the name after scope exit produces `7`.
- Effect markers show later conditions execute only after earlier tests fail;
  selected returns are `41`, `42`, and `43`.
- Scoped loop controls produce `2`, and nullable guards select `scope-null`,
  `3`, and `-1` without evaluating invalid nullable arithmetic.

Both programs match [expected output](../../examples/compiler-object-tests/expected.txt).
Source/saved-IR/assembly agree; SDK/generated PRGs pass structured
export/reassembly without raw CODE/DATA fallback and own-key signing checks.
Use the shared [console checker](../../scripts/README.md#console-specimen-check)
with `compiler-object-tests` and its SDK-built PRG. Local artifacts:
`scratch/core-scopes/check/`; the minimal acceptance probe and build log are in
`scratch/compiler-block-probe/`.

Focused compiler checks cover out-of-scope names, existing no-shadowing rules,
unreachable statements after scoped returns, nested loop targets, and 64/65-level
nesting boundaries. Plain scoped and unscoped bodies emit the same assembly;
scoping introduces no VM operation. These tests do not establish unrestricted
Monkey C shadowing or unbraced-body support.

## Switch selection and dispatch

2026-10-20, SDK 9.2.0, `fr265`, `-O0`. Garmin's
[language reference][switch-guide] describes ordered cases, optional default,
fall-through, and `case instanceof`. Authored acceptance probes establish:

- Dynamic value expressions, duplicate value labels, and default before a case
  compile. An empty switch and duplicate default are rejected.
- Case groups share a scope: redeclaring the same bare local in another group
  is rejected. A read bypassing an earlier initializer fails with a possibly
  uninitialized diagnostic. Switch locals are invisible after the switch.
- `continue` within a switch nested in `for` compiles and targets the loop.

Complete probe inputs/logs are in `scratch/compiler-switch/`. Copy a named
`.mc` into its `source/app.mc` and run
`sh scripts/build-example.sh scratch/compiler-switch` to rebuild it.

The grouped `CoreSwitch` specimen in
[source-types.mc](../../examples/compiler-object-tests/source-types.mc) compares
complete SDK/generated applications. Markers establish selector evaluation
before ordered case effects, skipping later tests after a match, and selection
of a matching case after an earlier textual default. Numeric outputs establish
fall-through across default and correct loop/switch break/continue targets.
Computed and literal equal-content Strings match the same value case. A source
`equals` override prints `switch-equals` and selects its case, confirming dynamic
dispatch rather than identity comparison.

An isolated failure within the initial specimen used
`switch (value) { case null: return 0; default: return value + 1; }` with
`value as Number or Null` and a Null argument. The SDK app crashed with
Unexpected Type Error at `nullable()`; retained output is under
`scratch/core-switch/ready/runtime/sdk/`. Structured SDK CODE shows selector
duplication, `getsv @equals`, `invokem`, and a conditional branch, explaining why
Null cannot serve as a value-case receiver. The corrected tracked specimen
guards Null before switching. Our ordinary method checks reject an unproved
nullable receiver for value cases instead of generating that failure.

SDK 9.2.0 also rejects arithmetic/member access on its original Object parameter
inside type-case bodies without casts; explicit casts let the reference app
compile. Our existing branch proofs accept safe local/parameter type-case reads
and reject refinements lost through fall-through joins.

Both final applications match checked-in expected output. Source/saved-IR/
assembly agree, and SDK/generated PRGs pass structured reconstruction without
raw CODE/DATA fallback plus own-key signing checks. Use the shared
[console checker](../../scripts/README.md#console-specimen-check) selecting
`compiler-object-tests`; final artifacts are `scratch/core-switch/equals/`.
Focused checks cover declaration availability, unmatched return paths, nested
switch/loop targets, shared and explicitly braced scopes, unknown targets,
malformed labels, and nesting/node limits.

[switch-guide]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/#switch-casestatements

## Required body braces

2026-10-20, SDK 9.2.0, `fr265`, `-O0`. Independent authored probes put
`if (i==0) i=1;`, `while (i<1) i++;`,
`for (i=0;i<1;i++) System.println(i);`, and `do i++; while(i<1);` inside
`onStart`, with `i` declared and a final `System.exit()`. All fail compilation
(exit 100), reporting a missing opening brace. A nested unbraced `if` likewise
fails before any dangling-else interpretation. A bare semicolon as a `for`
body is rejected. Leading semicolons and `{;}` are also rejected. A semicolon
following an existing declaration/call/update is instead an additional
terminator; it does not establish general empty-statement support. The focused
[terminator probes](compiler-locals.md#declaration-lists-and-extra-terminators)
separate these contexts, including standalone scoped blocks.

Complete authored inputs and build logs are under `scratch/compiler-unbraced/`.
Rebuild a case by copying its saved `.mc` into that project's `source/app.mc`
and running `sh scripts/build-example.sh scratch/compiler-unbraced`.
Compiler regression tests retain rejection of unbraced bodies and standalone
empty statements.

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
[The source spec](../specs/frontend/README.md#statement-arms-and-joins) owns the
current grammar; these runtime observations remain limited to the stated inputs.

[scaffold]: control-flow.md#reproduction
