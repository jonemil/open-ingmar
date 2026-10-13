# Char arrays and read-only native calls

SDK 9.2.0 and our generated Forerunner 265 application agree on all 18 console
lines in the [example][example]. `toCharArray` produces a typed collection passed
through locals and user calls; selected Array methods use native dispatch.
SDK checking requires Char search arguments. The supplementary String specimen
converts to one full-code-point element. This does not establish construction,
other mutation, element indexing, or general Array contracts. Reviewed
2026-10-12.

## Public contracts and method inventory

The [String API][string] declares `toCharArray()` as returning `Array<Char>` since
1.3.0. The [Array API][array] identifies its Object superclass and declares these
methods; generic Array signatures use Object-or-Null element contracts:

| Method | Result | Since | Executable scope or prerequisite |
|---|---|---|---|
| `size()` | Number | 1.0.0 | Supported on Char arrays |
| `toString()` | String | 1.0.0 | Supported on Char arrays |
| `indexOf(element)` | Number | 1.3.0 | Supported for Char searches |
| `reverse()` | Array | 1.3.0 | Char result verified; see transform evidence |
| `slice(start, end)` | Array | 1.3.0 | Char result/bounds verified; see transform evidence |
| `add(element)` | Array | 1.3.0 | Char mutation verified; see append evidence |
| `addAll(array)` | Array | 1.3.0 | Char arrays; see mutation evidence |
| `remove(element)` | Boolean | 1.3.0 | Char argument; see mutation evidence |
| `removeAll(element)` | Boolean | 1.3.0 | Char argument; see mutation evidence |
| `sort(comparator)` | Void | 5.0.0 | Mutation and comparator/callback contracts |

Inherited Object equality/hash/weak-reference behavior needs array-specific
specimens. The immutable weak specialization does not apply to Arrays.

## Reproduction and bindings

The SDK wrapper at `scratch/compiler-char-arrays/sdk.mc` supplies the
[console lifecycle][wrapper]. Its Jungle file selects our sources:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-char-arrays;sdk.mc
base.resourcePath = ../native-names/resources
```

Build and export the SDK-produced PRG from our source:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-char-arrays/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-char-arrays
cargo run -p ciq-dis -- all \
  scratch/compiler-char-arrays/build/fr265/compiler-char-arrays.prg \
  scratch/compiler-char-arrays/sdk-export
```

The exported SYMBOLS records identify `toCharArray` as `0x008004c4`, `size` as
`0x008000ae`, and `indexOf` as `0x008004bf`. `toString` uses the existing
`0x008000af` binding. Our calls use receiver/member dispatch through `getsv` and
invoke; the element contract does not create another executable class identity.
These facts come from our compiled specimen, not the compiler implementation.

Build/sign the independent PRG with the example's commands. Run each PRG in the
simulator container:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` after each run and compare both full
logs with the example's expected 18 lines. Successful exits alone do not verify
values or single evaluation.

## Observations and limits

Experimentally observed in both applications:

- `"AbA"` converts to three elements and formats as `[A, b, A]`. Search returns
  the first matching index, or `-1` for an absent Char.
- Empty conversion has size 0 and formats as `[]`. `"éΩ"` has size 2 and finds
  Omega at index 1.
- `"😀"` converts to one element. Searching for `128512.toChar()` finds index 0;
  searching for `55357.toChar()` returns `-1`. This distinguishes full code-point
  conversion from the [SDK source Char-literal discrepancy][char-literals]. It
  does not establish every invalid Unicode or encoding case.
- Typed user arguments/results, reassigned array locals, and nullable String
  guards retain usable arrays. An effectful String helper prints once before
  conversion and size lookup. Numeric size/search results feed Boolean operators.

A separate SDK compilation of our source failed for Char-array searches with
Number `65`, Null, and Boolean true. Each diagnostic required a Char argument.
The original input is retained locally as
`scratch/compiler-char-arrays/invalid-index-of.mc`; replace the corresponding
valid calls to reproduce the failures. Interpretation: the instantiated element
contract constrains `indexOf` more tightly than its generic API signature.
Our current profile follows that checking rule without implicit conversions.

## Compiler boundaries

Semantic API types preserve an Array element contract; executable values admit
only non-null Char arrays. Introduction checks recurse into Char, so even a
Char-array parameter calling API-1.0 `size` needs context 1.3.0. Array type
invariance and assignability to Object are our checking policy; the read-only
specimens do not establish broader SDK generic variance or mutation rules.

Pipeline tests compare source/resumed unit IR, unsigned PRGs, and assembly round
trips. Edited IR cannot misdeclare a Char-array user-call result as String or
change an array parameter to String and retain size dispatch. Wrong arguments,
receiver types, scalar consumers, and insufficient API contexts are rejected.
Console entry results remain scalar/Void; array entries are rejected.

Element access has [separate evidence](compiler-array-reads.md); broader Array
methods remain in [the plan][plan]. A native
array result does not establish source array construction, general object
mutation, nullable arrays, or ordinary object/reference lifetime behavior.

[example]: ../../examples/compiler-char-arrays/README.md
[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[char-literals]: compiler-char-literals.md
[plan]: ../plan.md
[transform evidence]: compiler-array-transforms.md
[append evidence]: compiler-array-add.md
[mutation evidence]: compiler-array-mutation.md
