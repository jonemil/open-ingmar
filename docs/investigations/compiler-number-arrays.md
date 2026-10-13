# Number arrays and UTF-8 conversion

SDK 9.2.0 and generated fr265 applications agree on all 17 lines in
[the example][example]. UTF-8 conversion produces Number arrays usable through
typed calls, numeric reads, mutation, and transforms. Checked 2026-10-13.

## Contracts and observations

**Officially documented:** [String `toUtf8Array`][string] returns `Array<Number>`
since API 1.3.0, with one Number for each UTF-8 byte. Array and Number class
introduction levels are 1.0.0; signature availability is distinct from the
conversion/mutation methods' 1.3.0 requirement.

**Experimentally observed:**

- `"Aé😀"` converts to `[65, 195, 169, 240, 159, 152, 128]`. Indexed Numbers
  participate directly in arithmetic; the second and third sum to 364.
- Typed user parameters/results retain Number elements. Appending 300 succeeds
  and reading it returns 300, so the array is not restricted to byte values.
- Search, single/all removal, reverse, and slice operate on Number elements.
  Bulk append updates shared references while leaving its distinct argument
  unchanged. Transforms retain a usable Number-array result.
- Empty conversion has size 0. A receiver helper prints once before conversion;
  a guarded nullable String converts and permits indexing.
- SDK compilation rejects Char/Null/Boolean/String element arguments to the
  probed Number-array methods and rejects `Array<Char>` for `addAll`. Diagnostics
  require Number and `Array<Number>` respectively.

**Interpretation:** the same generic Array substitution applies to Number and
Char elements. Neither scalar numeric/Char operators nor the common Array
runtime identity imply interchangeable mutable element contracts.

## Model and encoding

The executable model uses `Array(ArrayElement)` for values, parameters, and
results. `ArrayElement` currently admits Number and Char. Native resolution
substitutes that contract into arguments/results, and element reads derive
their result type from the receiver. [Literal inference][literals] supplies
Number arrays directly from source expressions.

The SDK-produced SYMBOLS records bind `toUtf8Array` to `0x008004c5`; the
independent application uses that binding with existing member invocation.
The CODE export falls back to raw bytes at unsupported opcode offset `0x1e7`;
this does not establish additional instruction support.

Explicit Number-array allocation uses `ipushz; newa`, just as Char allocation.
The element type changes checking rather than the allocation opcode. Ordinary
`agetv` produces a Number for a Number receiver. Pipeline checks preserve
source/saved-IR/assembly and unsigned-application equivalence. Edited IR cannot
misdeclare a user-call result or bypass invariant element checking. Context
checks allow API-1.0 Number-array signatures but require 1.3.0 for conversion
and the selected mutation/search/transform methods.

## Reproduction

Build/sign the independent program with the example's commands. Create the SDK
console wrapper from [the reference specimen][wrapper] as
`scratch/compiler-number-arrays/sdk.mc`. The Jungle file is:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-arrays;sdk.mc
base.resourcePath = ../native-names/resources
```

Build/export the SDK-produced specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-arrays/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-arrays
cargo run -p ciq-dis -- all \
  scratch/compiler-number-arrays/build/fr265/compiler-number-arrays.prg \
  scratch/compiler-number-arrays/sdk-export
```

Run each PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each run. Retained SDK/generated
logs are under `scratch/compiler-number-arrays/` and match the example's full
output. The SDK argument probe is `invalid.mc` in the same directory, selected
by `invalid.jungle`; `invalid-sdk.log` retains its diagnostics. Reproduce by
substituting the rejected argument types above into methods on
`"A".toUtf8Array()`.

Other element types, contextual empty literals, native object values,
and array stores remain separate work in [the plan][plan].

[example]: ../../examples/compiler-number-arrays/README.md
[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
[literals]: compiler-number-array-literals.md
