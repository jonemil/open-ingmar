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

The original builder snapshotted surrounding operands and materialized Boolean outcomes
through ordinary IR locals and explicit branches. Native/text pipeline checks
produce equal unsigned PRGs, and assembly text resumes the same program.
This establishes our lowering's behavior without matching SDK CODE or inspecting
Garmin's compiler/SDK implementation.

## Validation and limits

Instruction-oracle checks cover Boolean triples, reassigned loop flags, and empty
edge stacks. Typed/text checks cover module/self-call arguments, missing
initialization, and wrong temporary types; this module-only runtime specimen does
not establish self-call behavior. Stored guards do not carry narrowing facts.
The 254-parameter-plus-one-temporary boundary passes; an additional local fails.
That is a compiler budget check, not proof of maximum runtime frame capacity.

## Runtime truth values and operand combination

**Observed, 2026-11-25:** SDK 9.2.0, Forerunner 265, `-O 0`. Typed `if` and
`?:` probes agree. Null, zero Number and zero Long are false. Nonzero Number
and Long are true. Zero and nonzero Float, Double, and Char are true. Empty
String, Array, and Dictionary values and a source instance are also true.
Object-typed parameters test the actual runtime value. Branching on `!0`, `!1`,
and `!(-1)` gives true, true, and false respectively, consistent with Number
complement followed by the truth test.

The SDK default checker accepts Boolean and untyped logical operands. It
rejects homogeneous Number, Long, String, and `Number or String` operands
in both condition and value position. The same typed Number case compiles
with `-l 0` and shows a distinction between those positions:

| Use of `2 && 5` | SDK result |
|---|---|
| Store or return the value | Number `0` |
| Test directly in `if` | True arm |

The value form loads the left operand, uses `dup 0` and `bf`/`bt` to skip the
right operand, and executes `andv`/`orv` when both operands run. The condition
form branches directly on each evaluated operand and emits no combination.
An untyped `null && value` returns Null without evaluating the right operand.
An untyped `null || number` evaluates the right operand and then raises
`UnexpectedTypeException` in `orv`. An unchecked Number/String pair also
raises this exception when both operands run. These observations replace the
earlier hypothesis that the value form always returns one of its operands.

The `bt` shortcut used by OR differs from the ordinary `bf` test. An Array
passes a standalone `if`, but `array || rhs` evaluates `rhs`. In value
position the evaluated pair can raise `UnexpectedTypeException` at `orv`.

The updated [example][example] reproduces numeric loop tests, numeric and
reference truth values, dynamic integer combination, a retained Null operand,
and the Array case in an OR condition. Build it using the global-start wrapper
above.

[example]: ../../examples/compiler-logical-values/README.md
[condition specimen]: compiler-short-circuit.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
