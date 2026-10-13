# Native calls as statements

SDK 9.2.0 and generated fr265 applications agree on the eight lines in
[the example][example]. Supported native value calls may run as statements,
including chained calls and nullable results. Discarding consumes the final
result while retaining mutations and operand effects. Checked 2026-10-13.

## Observations

**Experimentally observed:**

- A helper taking `Array<Char>` appends and removes elements with ordinary
  statements, then drops `size()`'s Number result. A copied reference observes
  `[b, c, d]`. A chained `add('x').remove('x');` leaves those contents unchanged.
- Effectful receiver and argument helpers print `receiver`, then `value`,
  once each. A logical argument prints `receiver` once before its condition and
  skips its right-hand helper. No `unexpected` output appears.
- A loop repeatedly appends/removes an element and reaches the next statement
  with unchanged contents. This exercises repeated result cleanup.
- String search and substring results, String-to-array conversion, and scalar
  native calls can be discarded. The absent search result is Null. A nullable
  String receiver still needs its existing guard before a call.
- Source/saved-IR paths produce equivalent unsigned applications and symbolic
  assembly. Primitive lowering checks find one final `popv` after invocation,
  including zero-argument and chained calls. Edited IR cannot move a later read
  ahead of a discarded mutation while retaining the original consumer order.

These cases establish the bounded native profile on this target. Discarding
does not enable unknown methods, wrong arguments, nullable receivers without
proof, or native Void methods such as Array sorting.

## Reproduction

Use the example's commands to build/sign the independent application. Create
the SDK console wrapper from the [reference specimen][wrapper] as
`scratch/compiler-native-statements/sdk.mc`. The Jungle file is:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-native-statements;sdk.mc
base.resourcePath = ../native-names/resources
```

Build our source with the SDK:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-native-statements/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-native-statements
```

Run the SDK PRG under `scratch/compiler-native-statements/build/fr265/` and our
generated PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each run. The retained full logs are
`sdk-monkeydo.log` and `generated-monkeydo.log` under
`scratch/compiler-native-statements/`; both match the example's expected output.
No new symbol binding or VM opcode is required by this feature.

## Representation

The source builder produces an ordinary native value call followed by a
`Discard` expression root. Shared validation therefore checks the same call
contract and operand order as a value consumer. The backend uses ordinary
expression lowering and then `popv`; only the final chain result is dropped.

The [IR grammar][ir] allows `.discard` for any supported non-Void value, while
source expression statements remain restricted to calls and assignments. Void
module/self calls and native module statements keep their existing contracts.
General native Void dispatch remains future work in [the plan][plan].

[example]: ../../examples/compiler-native-statements/README.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[ir]: ../specs/compiler-ir-text.md#native-value-calls
[plan]: ../plan.md
