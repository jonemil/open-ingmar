# Char-array bulk append and removals

SDK 9.2.0 and independently generated fr265 applications agree on all 22 lines
in [the example][example]. `addAll` preserves a usable Char-array result;
`remove` and `removeAll` return Boolean and mutate shared references. Checked
2026-10-13. Other element types, object equality, and other devices are outside
this experiment.

## Public contracts and observations

**Officially documented:** the [Array API][array] introduces all three methods
at API 1.3.0. `addAll` appends the supplied elements and returns the receiver.
`remove` deletes the lowest-index match, while `removeAll` deletes every match.
Both return whether a match was removed. Generic signatures expose
Object-or-Null elements.

**Experimentally observed:**

- Appending `"ca"` to `"aba"` produces `[a, b, a, c, a]`; the argument remains
  `[c, a]`. The receiver and a previously copied reference both see the append.
  Removing through the returned array also updates the original receiver.
- `remove('a')` returns true and leaves `[b, a, c, a]`. A missing `'z'` returns
  false. `removeAll('a')` returns true and leaves `[b, c]`; repeating it returns
  false. Removing from an empty array returns false.
- Appending an empty array leaves the receiver unchanged. Appending `"xy"` to
  an empty array produces `[x, y]`. Self-appending `"xy"` produces `[x, y, x, y]`
  and completes normally in the tested simulator.
- Receiver and removal argument helpers print once each, in that order.
  `false && array.removeAll('a')` leaves the array unchanged; the same call
  under `true &&` removes the matches.
- SDK checking rejects Null/String arguments to `addAll` and Number/Null
  arguments to `remove`, plus Boolean/String arguments to `removeAll`.
  Diagnostics require `Array<Char>` and Char respectively.

**Interpretation:** these signatures instantiate the Array element parameter
in both arguments and results. The bounded native profile follows that rule;
it does not widen `Array<Char>` to mutable `Array<Object or Null>`.

## Bindings and reproduction

The exported SYMBOLS records from our SDK-compiled source identify:

| Member | Symbol ID |
|---|---|
| `addAll` | `0x008004be` |
| `remove` | `0x008000b7` |
| `removeAll` | `0x008004c0` |

These use existing receiver/member invocation. The SDK CODE export falls back
to raw bytes at an unsupported opcode at offset `0x214`; no broader instruction
support is inferred from this experiment.

Create the SDK console wrapper described in the [reference specimen][wrapper]
as `scratch/compiler-array-mutation/sdk.mc`. Its Jungle file is:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-array-mutation;sdk.mc
base.resourcePath = ../native-names/resources
```

Build and export our source specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-array-mutation/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-array-mutation
cargo run -p ciq-dis -- all \
  scratch/compiler-array-mutation/build/fr265/compiler-array-mutation.prg \
  scratch/compiler-array-mutation/sdk-export
```

Build/sign the independent application with the example's commands. Run each
program separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Preserve `scratch/simulator/monkeydo.log` after each run and compare the full
logs with the example's output. The retained logs are
`scratch/compiler-array-mutation/sdk-monkeydo.log` and
`generated-monkeydo.log` in the same directory.

For argument checking, replace calls with the rejected arguments above in a
separate source file. Select that file and the SDK wrapper in another Jungle
file. The local specimen and diagnostics are `invalid.mc` and `invalid-sdk.log`
under the same scratch directory.

Source/saved-IR checks retain generic argument/result contracts, API boundaries,
assembly bindings, and equivalent unsigned applications. Adversarial IR moving
a size query ahead of a mutation consumed first is rejected. This uses existing
call-order validation; it does not establish object alias analysis or general
mutation optimization. Native value calls as statements have
[separate evidence](compiler-native-statements.md). Sorting, construction, and
element writes remain separate work in [the plan][plan].

[example]: ../../examples/compiler-array-mutation/README.md
[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
[plan]: ../plan.md
