# Compound field/indexed targets and mutation order

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: SDK and independently generated [apps][example] match all 15 lines
of [expected output][output]. Compound updates retain the old field/element
value before RHS effects and store the binary result afterward. Receiver/index
helpers run once each. No Garmin compiler/SDK implementation was inspected.

## Decisive mutation cases

The source Counter starts with field 10. Its RHS sets that field to 20 and
returns 3; the compound result is 13. A staged logical RHS sets the field to
40 and returns true; the result after adding its hash code is 11. Reading the
field after those RHS effects would produce different results.

The Number array starts as `[10, 20, 30]`. The first compound captures index 1
and old value 20. Its RHS removes 10 and returns 3; the copied reference prints
`[20, 23]`. After a separate multiplication produces `[40, 23]`, a logical RHS
removes 40 and appends 99. The compound still uses old value 23, then writes
index 1; the result is `[23, 24]`. Char additions through a shared reference
produce `C` and `A`.

These are observations for the tested profile. They establish old-value and
location preservation despite RHS mutation; they do not establish broader
receiver/property forms or other VM/device behavior.

## Reproduction

Compile/sign our app with the example's commands. Create the ignored SDK
Jungle using the existing manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-compound-targets
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-compound-targets/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-compound-targets
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run SDK/generated PRGs separately, preserving `scratch/simulator/monkeydo.log`
and `screen.png` after each session. Both complete logs match the fixture; the
generated display returns to idle after `System.exit()`.
The [member-get follow-up](member-get-opcode.md) establishes the stacked field
read used at CODE+`0x5b`. Full structured export still needs the gaps below.

### Outstanding toolchain coverage

The compiler and simulator comparisons above do not establish complete
assembler/disassembler coverage. Export reports two unresolved gaps:

- Opcode `0x40` at CODE+`0x1d8` is unsupported, so the entire CODE payload is
  preserved as raw binary. The instruction-boundary audit also finds it at `0x26e`.
- The DATA record at offset `0x3397` starts with `0x03`, which the complete
  record walker does not recognize. Export attempts its supported string
  reference fallback and preserves the raw DATA payload.

The export returns success because raw preservation is supported. These
diagnostics require investigation and the corresponding structured read/write,
assembly/disassembly, and reconstruction support under the
[toolchain completion rule](../plan.md#toolchain-completion). Record/opcode
semantics remain unverified; resolve these gaps before the next language group.

## Read and write bounds failures

Create separate ignored replacement Writes modules, retaining the example's
lifecycle. Receiver and index helpers print their names before returning the
array and index; the RHS prints `rhs`, performs the indicated action, then
returns 3. The run method executes one compound, then prints `after`:

| Probe | Initial array | Index | RHS action | Observed markers |
|---|---|---|---|---|
| Initial read invalid | `[10, 20]` | 2 | none | receiver, index |
| Final write invalid | `[10, 20, 30]` | 2 | remove 10 | receiver, index, rhs |

Use `scratch/compiler-compound-targets/read-bounds` and `write-bounds`, with
Jungle scaffold paths one directory deeper and
`base.sourcePath = .;../../../examples/compiler-compound-targets/app.mc`.
Annotate receiver parameters/results as `Array<Number>`. Build with that
Jungle and the probe directory as the build-script argument. Compile/sign the
same two sources through our unit PRG path.

All four SDK/generated runs report `Array Out Of Bounds Error` and an app
crash; none prints `after`. Interpretation: initial bounds fail before RHS
effects, while the final write rechecks bounds after them. Exception recovery,
Char bounds compounds, and other runtime versions were not separately tested.

## Compiler contracts and validation

Native/text checks compare all ten Number field compounds with explicit
read/binary/write assembly. Indexed checks cover Number operators, Char addition,
single receiver/index evaluation, one read/write, nullable guards, invalid types,
read-before-RHS order, frame limits, and 64-level RHS nesting. Complete saved-IR,
assembly, and unsigned-PRG paths agree. See the
[current target contracts](../specs/compiler-subset.md#indexed-compound-assignment).

[example]: ../../examples/compiler-compound-targets/README.md
[output]: ../../examples/compiler-compound-targets/expected.txt
