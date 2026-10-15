# Char-array literal construction

SDK 9.2.0 and generated fr265 applications agree on the 12 lines in
[the example][example]. Nonempty literals construct usable `Array<Char>` values
with ordered element effects, including logical expressions and typed user
results. Checked 2026-10-13. Empty source literals and other element types are
outside the compiler profile.

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

The [lowering follow-up](natural-lowering.md#array-construction) replaces native
append with private final-length construction and direct stores. The original
**design decision** was to allocate an empty Char array first and fill it with typed
`add` calls. The builder retains the array and current element in two temporary
locals, reusing the element slot. This preserves allocation before element
effects and keeps short-circuit branch stacks empty. The array always satisfies
its element contract; no uninitialized Null slots are exposed as Char values.

This is an independent lowering strategy. Equal output does not establish equal
SDK allocation counts, memory use, or allocation-failure timing. Direct fixed
length allocation/filling needs initialized-element rules and its own evidence.
Empty source `[]` has no inferred element contract yet; explicit IR allocation
names `char` and therefore can construct an empty array without guessing.

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
The compiler reuses existing `newa`, typed append, local storage, and result
discarding; no new VM opcode or native symbol binding is required.

Source and saved-IR checks reject unsupported element types/API contexts and
malformed allocation text. Literal construction shares the normal operation,
nesting, and local-frame budgets. General Array typing, contextual empty
literals, direct stores, and native object construction remain in [the plan][plan].

[example]: ../../examples/compiler-array-literals/README.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
