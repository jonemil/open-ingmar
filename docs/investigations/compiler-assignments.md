# Compiler local reassignment

2026-10-10. Question: can stored locals carry changing integer/string values
through branches while retaining native/text parity and read/write order?
SDK 9.2.0, Java 17, `fr265`, original memory allowance, our developer key. This
uses our [nested-condition scaffold](compiler-nested-conditions.md) and
[string layout/binding](compiler-strings.md); no new external research was used.

## Contract and experiment

The [source][source] and [IR][ir] specs define assignment, fixed local types,
read/write order, and snapshots. This experiment uses:

The [checked-in source](../../examples/compiler-probe/assignments.mc) initializes
`answer = 40` and `message = "start"`. Both condition arms update the same two
locals: the zero arm adds `3` and stores `"zero"`; the other adds `7` and stores
`"one"`. After the join it prints the selected message/number, increments the
number, and returns it. This runtime specimen covers integer/string locals and read-only parameters.

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

Integrate into a fresh scaffold export: rename the old entry label and its line
references to an unused alias, preserving their body/PCs. Append generated
assembly under the original alias, redirect its method reference, and apply the
DATA/resource lengths and offsets recorded in this record. Reconstruct/sign,
verify our key, and run each case separately using the
[shared procedure](../research.md#shared-compiler-reproduction).

## Validation and limits

Rust checks covered snapshots, nested/arm writes, early returns, fixed slots,
type/scope errors, and native/text equivalence. Both typed and text IR reject
stale reads and targets initialized on only one incoming path. Runtime evidence
here covers integer/string reassignment on the stated device; other forms have
separate records. Current [assignment rules](../specs/frontend/README.md#local-reassignment)
do not depend on this investigation's implementation stage.

## Parameter writes and local annotations

Authored SDK/compiler/simulator 9.2.0 probes on 2026-10-20 use fr265 and `-O 0`.
Compile an AppBase console wrapper with `scripts/build-example.sh` and these
global helpers:

```monkeyc
function advance(n as Number) as Number { n=2; n+=3; n++; return n; }
function guarded(n as Number or Null) as Number {
    if(n!=null){n=null;}
    n=7;
    if(n!=null){return n;}
    return -1;
}
```

**Observed:** parameter writes compile. The [shared application's parameter
groups][parameters] additionally cover String/Float/Char writes, nullable
snapshots, for-header writes, replacing Array and nullable source-object
parameters, branch effects, and an instance method. SDK and generated output
agree; callers retain their original scalar/reference bindings. Both PRGs
reconstruct without raw CODE/DATA fallback. Our lowering stores directly into
the existing parameter slots rather than copying them into locals.

A separate source with explicit local Number, String/Boolean, Array, and
nullable source-class annotations fails SDK compilation with the diagnostic
“Invalid explicit typing of a local variable. Local variable types are inferred.”
The frontend rejects these forms; supported parameter/result/field annotations
do not imply typed-local syntax. Uninitialized-local semantics were not
established by this probe.

[parameters]: ../../examples/compiler-object-tests/README.md
[design]: ../specs/ir/README.md#local-assignment
[source]: ../specs/frontend/README.md#local-reassignment
[ir]: ../specs/ir/README.md#local-assignment
[scaffold]: control-flow.md#reproduction

## Deferred local declarations

2026-10-21, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Observed:** `var x;` is accepted when unused or assigned before executable
reads. A mixed `var x, y=2; x=y;` list is accepted. Reads immediately following
an uninitialized declaration, after a one-arm assignment, or after a loop
redeclares the slot without assigning on every path are rejected with
“Value may not be initialized.” Assigning in both conditional arms or in a
`do` body permits a later read. These checks do not establish a readable Null
default for an uninitialized local.

The SDK also accepts an unassigned variable's update in `for(;;x++){return;}`.
Our frontend still requires a selectable update operand type there; accepting
untyped values in disconnected headers remains a checking/inference gap.
The isolated authored probes and SDK logs are in
`scratch/core-local-inference/` (`probe.py`).

The tracked `DeferredLocals` group in [the console specimen][parameters] covers
mixed declaration/effect order, branch and early-return joins, post-test/for
loops, and scalar, source-reference, Array, and ByteArray assignments.
SDK/generated simulator output matches the expected text; structured export,
reassembly, and native/text resumption pass through the shared checker.
Artifacts: `scratch/core-local-inference/check/`. The new declaration has no
runtime write; ordinary `lputv` implements the first assignment. No new opcode
or DATA record is required. Inferred type changes after initialization remain
outside the fixed-contract profile.

The authored loop probe in `scratch/core-local-lifetime/source/app.mc` assigns
a ByteArray only on its first iteration and retains a weak handle. SDK CODE
has one write to the value's slot 4 at CODE+0x54, with no clearing write at the
loop declaration or backedge. This establishes the observed absence of a
redeclaration write; no separate simulator-liveness result is claimed for this
isolated probe. Its build log and structured export are retained alongside it.
