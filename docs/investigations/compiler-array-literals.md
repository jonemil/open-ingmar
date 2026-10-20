# Char-array literal construction

SDK 9.2.0 and generated fr265 applications agree on the 12 lines in
[the example][example]. Nonempty literals construct usable `Array<Char>` values
with ordered element effects, including logical expressions and typed user
results. Checked 2026-10-13. This specimen covers Char elements; current source support
is specified independently of the experiment.

## Observations and interpretation

**Experimentally observed:**

- `['a', 'b']` is indexed and mutated through ordinary Array calls. A shared
  reference observes an appended `'c'`.
- Effectful Char helpers print `x`, then `y`, once each, and construct `[x, y]`.
  A logical element skips its right-hand helper before conversion to Char;
  indexing returns Number code 1 and the following literal `'z'`.
- A BMP Char and a supplementary code-point conversion form two elements.
  A typed user function returns `['p', 'q']`. Repeated construction in a loop
  produces a one-element array on each iteration.
- Source/saved-IR/application paths agree. The explicit empty-allocation
  primitive encodes `ipushz; newa` as `37 14`. An edited allocation definition
  cannot move ahead of an earlier call while retaining its later consumer.

Allocation counts, memory use, and allocation-failure timing were not compared.
[The lowering experiments](natural-lowering.md#array-construction) separately
exercise direct stores; the [source](../specs/frontend/README.md#char-arrays) and
[IR](../specs/ir/README.md#private-array-construction) specs own the
construction/initialization contracts.

## Reproduction

Use the example's commands to build/sign our independent application. Create
the SDK console wrapper from the [reference specimen][wrapper] as
`scratch/compiler-array-literals/sdk.mc`. The Jungle file is:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-array-literals;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the source with the SDK:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-array-literals/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-array-literals
```

Run the SDK PRG under `scratch/compiler-array-literals/build/fr265/` and our
generated PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each run. The retained full logs are
`sdk-monkeydo.log` and `generated-monkeydo.log` under
`scratch/compiler-array-literals/`; both match the example's expected output.
Source/text checks reject invalid elements/API contexts and malformed allocation
text. Construction uses the common operation, nesting, and frame budgets.

## Container separators

Isolated authored-source probes on 2026-10-20 use SDK 9.2.0, fr265, `-O 0`.
Compile one statement at a time inside an AppBase `onStart(state)` wrapper
using `scripts/build-example.sh`, with the ordinary console manifest/resources.

**Observed:** `[1,2,].size()`, `[1,2,]b.size()`, and
`{"a"=>1,"b"=>2,}.size()` compile. Trailing commas in function parameters,
ordinary calls, constructors, local declarations, and `for` initializer/update
lists fail SDK parsing. Examples are `function f(a,b,)`, `f(1,2,)`,
`new Token(1,)`, `var a=1,b=2,;`, `for(var a=0,b=1,;a<1;a++)`, and
`for(var a=0;a<1;a++,)`. Comma-only `[,]` and `{,}` also fail parsing.

The [shared object's `CoreLists` group][lists] checks constant and computed
Array/ByteArray/Dictionary construction, trailing comments, and branching Array
elements. It retains ordered effect markers and result reads. The source
[grammar](../specs/frontend/grammar.md#lexical-rules-and-literals) defines the
accepted separators independently of literal element/type contracts.
SDK and generated applications match its expected output; both PRGs reconstruct
without raw CODE/DATA fallback.

[example]: ../../examples/compiler-array-literals/README.md
[lists]: ../../examples/compiler-object-tests/README.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
