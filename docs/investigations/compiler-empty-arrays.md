# Contextual empty array construction

SDK 9.2.0 and independently generated fr265 applications agree on all 22
lines in [the example][example], checked 2026-10-18. The specimen asks whether
empty arrays supplied to typed returns, arguments, and assignments remain
usable without defaulting to Char or relaxing element contracts, and how sized
allocation initializes its elements.

## Observations and limits

**Experimentally observed:** both compilers accept empty Number/Char returns,
typed user-call arguments, array-local reassignment, and `addAll([])` on typed
receivers. Empty results have size zero and permit a subsequent append of their
declared element type. Reassigning a Number-array local to `[]` leaves an alias
holding its previous `[7]` array. Helpers before and after an empty argument
print `10`, then `20`; the call returns `30`.

**Implementation checks:** source, saved IR, assembly text, and unsigned PRG
paths agree. Direct empty returns lower to `ipushz; newa; ret`, without `add` or
temporary local storage. Empty Number returns pass API 1.0.0 checking; nonempty
Number literals require API 1.3.0 for append, and Char contracts require 1.3.0.
Edited IR cannot change an allocation's element type while retaining an
incompatible `addAll` use. Cross-file/self calls, logical argument joins, and
loop assignments retain their concrete contracts.

**Limits:** runtime output does not establish the SDK's allocation strategy or
resource usage. Our bounded context rules do not claim full SDK inference:
untyped local initialization, receiver-based inference such as `[].size()`, and
propagation through `return [].reverse()` remain unsupported. Object elements
and nested arrays need separate contracts and runtime evidence.

## Reproduction

Use the example's commands to build/sign our program. Copy the
[console wrapper][wrapper] into `scratch/compiler-empty-arrays/sdk.mc` and
create `group.jungle` there:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-empty-arrays;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the SDK specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-empty-arrays/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-empty-arrays
```

Run each PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each session and compare both full
logs with the expected output. Retained SDK/generated captures are under
`scratch/compiler-empty-arrays/`.

[example]: ../../examples/compiler-empty-arrays/README.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen

## Sized allocation

**Experimentally observed (2026-10-18):** SDK/compiler/simulator 9.2.0, fr265,
`-O 0` accepts `return new[n]` under `Array<Number or Null>`. Length three
produces `[null, null, null]`; replacing index one produces `[null, 7, null]`.
Zero length produces an empty array. An effectful size helper executes once.
The isolated authored probe and captures are in `scratch/sized-array-probe/`;
`runtime-long/monkeydo.log` holds the complete six-line positive result.

The [tracked example][example] also checks reassignment aliases, typed argument
context, and a conditional length with only the selected helper executing. Use
its existing SDK wrapper/Jungle and the console specimen checker to compare
SDK/generated execution and structured reconstruction. Both programs match all
22 expected lines, with structured CODE/DATA reconstruction passing. Captures
are in `scratch/check-sized-arrays/`. Sized allocation lowers
to the existing `newa` instruction; no additional DATA payload is required.

**Contract choice:** require nullable elements for all sized allocations,
including zero length, because allocation supplies Null slots. This is our
bounded safety rule, not a claim that the SDK rejects non-null element
annotations. Negative/oversized lengths and allocation failure recovery were
not characterized; no portable limit or error class follows from this probe.
