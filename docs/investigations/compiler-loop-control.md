# Compiler loop control and optional else

2026-10-10. Question: can structured loop controls target the innermost loop
without weakening scope, return, initialization, or empty-stack requirements?
SDK 9.2.0, Java 17, `fr265`, original memory allowance, our developer key.
The experiment reuses the [control-flow scaffold][scaffold] and existing
[while-loop implementation][loops]. No external implementation was consulted.

## Implementation

The parser accepts optional `else` and semicolon-terminated `break`/`continue`.
An omitted false arm falls through. The source builder tracks enclosing loop
pairs: `break` selects the innermost exit and `continue` its test. They lower to
existing version-2 jumps; no IR syntax or VM opcode changes are required.

Statement sequences report path termination, including return and loop control.
Only falling-through paths contribute join edges or an implicit loop backedge.
Both terminating conditional arms omit their join. A loop's false-condition
exit remains available, so its body cannot replace a following required return.
Existing graph reachability, initialization, effect-order, and stack rules apply.
See [source syntax][source] and [design][design].

## Simulator observations

The [example](../../examples/compiler-probe/loop-control.mc) initializes
`remaining = value + 4`. It decrements before testing, skips outer iteration
`2`, and runs an inner counter from `3`, skipping inner value `1`. Remaining
value `1` breaks the inner loop; value `0` breaks the outer loop after processing
its inner iterations. Other inner iterations increment the total. It prints
that total and returns `40 + total`.

A second input initializes `remaining = value`, exercising zero iterations.
Observed console results and inspected displays:

| Initial counter | Argument | Printed total | Return | Display |
|---|---|---|---|---|
| `value + 4` | 0 | 4 | 44 | |
| `value + 4` | 1 | 6 | 46 | 52 |
| `value` | 0 | 0 | 40 | |
| `value` | 1 | 2 | 42 | 48 |

The unchanged scaffold adds `countdown(3) = 6` to `choose(1)` for display.
Its caught-exception results remain `41` and `43`. Redraws repeat prints.
Captures/logs are saved under ignored
`scratch/compiler-loop-control/simulator-multiple/` and
`scratch/compiler-loop-control/zero/simulator-zero/`.

Generated CODE is 158 bytes for the main source and 155 for the second input.
Native, saved-IR, and saved-assembly CODE are byte-identical for each input.
Both use three local slots, with the inner initializer rerun when reached.
The evaluation stack is empty at all loop-control targets.

Independent payload comparison verifies the original 1021-byte CODE prefix plus
the exact generated suffix. DATA only changes the method word at
`0x4a8..0x4ac` from `0x100000a7` to `0x100003fd`. RESOURCES adds 2 or 5 zero bytes
respectively, retaining signature alignment with total growth 160. All other
section payloads are unchanged except regenerated developer signatures; both
verify with our key.

Signed output SHA-256:

```text
multiple e5738019ed3db2890c303177e7af8db745992fcac5c1aa93fc83db48ffeef646
zero     15b314ab91dd12ea85818155df09bd742591e73e5d4feb78a78d7024f6a97f0b
```

## Reproduction

Build the original SDK PRG using [its instructions][scaffold]. Use fresh output
paths when repeating. Generate all pipeline stages against its stored symbols:

```sh
cargo build --workspace
mkdir -p scratch/compiler-loop-control
target/debug/ciq-dis all scratch/control-flow/sdk-probe/build/fr265/sdk-probe.prg scratch/compiler-loop-control/project
target/debug/ciq-compiler examples/compiler-probe/loop-control.mc scratch/compiler-loop-control/choose.mcasm
target/debug/ciq-compiler --emit ir examples/compiler-probe/loop-control.mc scratch/compiler-loop-control/choose.ciqir
target/debug/ciq-compiler --emit code --symbols scratch/compiler-loop-control/project/010-symbols-5717b015.mcasm examples/compiler-probe/loop-control.mc scratch/compiler-loop-control/native.bin
target/debug/ciq-compiler --from ir --emit code --symbols scratch/compiler-loop-control/project/010-symbols-5717b015.mcasm scratch/compiler-loop-control/choose.ciqir scratch/compiler-loop-control/ir.bin
target/debug/ciq-compiler --from assembly --emit code --symbols scratch/compiler-loop-control/project/010-symbols-5717b015.mcasm scratch/compiler-loop-control/choose.mcasm scratch/compiler-loop-control/assembly.bin
cmp scratch/compiler-loop-control/native.bin scratch/compiler-loop-control/ir.bin
cmp scratch/compiler-loop-control/native.bin scratch/compiler-loop-control/assembly.bin
```

Apply the [bounded append integration script][integration], substituting root
`scratch/compiler-loop-control`. It preserves the original method as
`sdk_choose` and its line references, appends `choose.mcasm`, binds existing
print symbols, and derives manifest growth and padding from `native.bin`.
The existing DATA method reference selects the appended `choose` label.

```sh
target/debug/ciq-asm project scratch/compiler-loop-control/project scratch/compiler-loop-control/generated.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/compiler-loop-control/generated.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/compiler-loop-control/generated.prg --device fr265 --observe-seconds 10
```

For the second input, copy the source to ignored `scratch/` and replace only
`value + 4` with `value`. Repeat the stages and integration under root
`scratch/compiler-loop-control/zero` against a fresh export of the original.
Save each simulator capture before the next session overwrites it.

## Validation and limits

Instruction-oracle tests check nested target selection, skipped updates,
zero/multiple iterations, mixed return/break/continue arms, and empty edge
stacks. They also check optional false paths, arm-local visibility, missing
returns, outside-loop controls, reserved keywords, missing semicolons, and
unreachable following statements. Typed/text IR and assembly remain equivalent.

This establishes the tested profile on one target, not termination guarantees
or complete Monkey C compatibility. Labeled controls, `for`, `do/while`,
brace-free arms, Boolean literals, and exception source lowering remain outside
the subset. Existing nesting, operation, slot, and type limits are unchanged.

[scaffold]: control-flow.md#reproduction
[loops]: compiler-loops.md
[source]: ../specs/compiler-subset.md#loop-control
[design]: ../design/compiler-ir.md#loops-and-initialization-analysis
[integration]: compiler-nested-conditions.md#reproduction
