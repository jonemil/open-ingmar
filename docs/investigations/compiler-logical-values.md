# Boolean results across expression joins

SDK 9.2.0 and our generated Forerunner 265 application agree on all 25 console
lines in the [example][example]. Logical results work as stored values,
arguments, return values, and native receivers. Earlier operands execute before
later conditional operands, once each. This extends the [condition specimen]
without admitting non-Boolean truth-value rules. Experiment date: 2026-10-12.

## Specimen and reproduction

The SDK wrapper at `scratch/compiler-logical-values/sdk.mc` uses the
[console lifecycle][wrapper]. Its Jungle file selects our example sources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-logical-values;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the SDK reference with local device data and our developer key:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-logical-values/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-logical-values
```

Build/sign our generated PRG with the example's commands. Run each PRG in the
simulator container:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

The SDK output is `scratch/compiler-logical-values/build/fr265/` followed by
`compiler-logical-values.prg`. Preserve `scratch/simulator/monkeydo.log` after
each run, then compare both full logs with the example's expected output.
Successful exits alone do not establish operand order.

## Observations

Experimentally observed in both applications:

- False-left initialization and true-left assignment skip their `bad-*` calls.
  User-method returns preserve Boolean results for both AND outcomes.
- Grouped logical values support negation, Boolean equality, and native
  `equals()` receiver calls.
- String concatenation prints `left` before `right`, then `xtrue`.
  A native call prints `receiver` before `native-argument`, then false.
- A three-argument user call prints `arg-first`, `arg-middle`, `arg-right`, and
  `arg-last` in that order, then returns `xtrue7`. The statement call likewise
  evaluates its Number argument before the logical Boolean argument.
- Two logical operands compare false; guarded nullable conversion produces a
  stored Boolean true. Loop assignment evaluates its helper twice and stops at 3.

Our builder snapshots surrounding operands and materializes Boolean outcomes
through ordinary IR locals and explicit branches. Native/text pipeline checks
produce equal unsigned PRGs, and assembly text resumes the same program.
This establishes our lowering's behavior without matching SDK CODE or inspecting
Garmin's compiler/SDK implementation.

## Validation and limits

The instruction oracle checks Boolean triples through logical returns,
comparison/grouping/negation, and reassigned loop flags. It also checks empty
stacks at branch edges. Typed/unit text tests cover native arguments and source
self-call argument forms; self-call runtime behavior is not established by this
module-only specimen.

All temporary types and initialization are rechecked by ordinary IR validation.
Edited text missing initialization and typed edits assigning Number to a
Boolean temporary are rejected. A stored guard does not narrow the checked
original storage, even when its right operand used a valid non-null proof.
A snapshot made from a proved scalar read retains that scalar value's type.

Temporary locals share the existing slot budget: tests admit a logical return
with 254 parameters and one temporary, but reject adding a source local beyond
that combined budget. This is a compiler-boundary check, not runtime evidence
for maximum frame sizes. Slots are not reused and snapshots are conservative;
optimization is separate work. Other logical operand types remain unsupported.

[example]: ../../examples/compiler-logical-values/README.md
[condition specimen]: compiler-short-circuit.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
