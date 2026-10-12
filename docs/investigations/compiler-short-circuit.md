# Short-circuit Boolean conditions

SDK 9.2.0 and our generated Forerunner 265 application agree on all 23 console
lines in the [example][example]. Boolean conditions skip the right operand when
its result is unnecessary and preserve required call order. Guarded nullable
reads run safely through these edges. The supported profile is condition-only;
this does not establish logical operators for other types or values across
joins. Experiment date: 2026-10-12.

## References and reproduction

The [Monkey C reference][language] describes logical AND/OR and broader
truth-value rules. The [Functions article][functions] lists `&&` before `||`
in its precedence table. These public articles were retained under ignored
`scratch/official-docs/` during the [reference review][reading]. Our Boolean-only
contract is narrower; bytecode behavior is established by our own specimens.

The SDK wrapper at `scratch/compiler-short-circuit/sdk.mc` uses the same console
lifecycle as the [Boolean-negation specimen][negation]. Its local Jungle file
selects these inputs:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-short-circuit;sdk.mc
base.resourcePath = ../native-names/resources
```

Build with local device data and our developer key:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-short-circuit/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-short-circuit
```

Run `build/fr265/compiler-short-circuit.prg` under that scratch directory, then
our signed PRG built with the example's commands. In the simulator container:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` after each run. Compare both entire
logs against the example's expected 23 lines, including absent `bad-*` markers.
Process exit status alone does not establish evaluation order.

## Observations

Experimentally observed in both applications:

- False-left AND and true-left OR skip the right helper. Required helpers print
  left before right, exactly once per evaluated operand.
- `true || false && effect()` skips the effect and selects the true arm.
  Negating a grouped OR reverses the final branch without making operands eager.
- A negated null check followed by Number comparison or String `length()` is
  safe on the selected AND edge. The null String argument takes the alternate
  arm without invoking `length()`.
- OR's right operand can use a Number after the left null comparison is false.
  A direct negated null guard also permits the selected arm's numeric use.
- A loop retests its operands, prints its helper twice, and skips it on the final
  false-left test. A nullable local loop stops after assigning Null.

Our frontend builds explicit Boolean branches; existing VM branch instructions
execute the graph. No eager logical instruction or trusted frontend narrowing
annotation is introduced. Saved IR resumes the same graph, and shared validation
recomputes its facts. The SDK compiler and SDK implementation were not inspected.

## Proof checks and limits

The bounded instruction oracle covers all Boolean triples for AND, OR, mixed
precedence, grouping, and negation, and checks empty stacks at block edges.
Pipeline tests compare source/resumed IR, unsigned PRGs, and assembly round trips.
These tests support the implementation contract separately from simulator evidence.

Edited IR negated guards recompute proof polarity. Reversing destinations or
writing the checked local before its branch prevents non-null use. Source tests
also reject facts absent on one join path, invalidated by writes, or bypassed by
loop breaks. Every logical operand is checked, including runtime-skipped operands.

Logical value results remain deferred: assignments, call arguments, returns,
and logical subexpressions consumed by comparison/postfix operations receive
an explicit diagnostic. Other truth-value types, bitwise operators, and the
`and`/`or` aliases remain unsupported. See [the plan][plan] for Boolean values
across joins.

[example]: ../../examples/compiler-short-circuit/README.md
[language]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[functions]: https://developer.garmin.com/connect-iq/monkey-c/functions/
[reading]: official-garmin-documentation.md
[negation]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
