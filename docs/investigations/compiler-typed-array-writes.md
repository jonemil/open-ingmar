# Char and View indexed replacement

Experimentally observed on 2026-10-14 with SDK 9.2.0 and the Forerunner 265
simulator: SDK and independently generated [applications][example] match all
19 lines of [expected output][output]. Char-array aliases observe replacements
with `Ω`, its lowercase `ω`, and a character read from a staged expression.
View-array aliases retain assigned native/source View references, established
by `indexOf` before and after replacement. Both arrays retain length 2.

The marked View assignment evaluates receiver and index before the logical
value call; each marker appears once. Our shared `.array-write` lowers through
the existing `aputv` instruction without a result. Primitive parameter writes
for Char and View each encode as `35 04 12 01 12 02 12 03 11 16`, just like
the verified [Number write][number]. No new instruction encoding is inferred.

## Reproduction

Compile/sign our app with the example's commands. For the SDK reference create
`scratch/compiler-typed-array-writes/monkey.jungle` with the existing ignored
manifest/resource scaffold:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-typed-array-writes
base.resourcePath = ../native-names/resources
```

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-typed-array-writes/monkey.jungle" \
  sh scripts/build-example.sh scratch/compiler-typed-array-writes
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Run SDK and generated PRGs separately, preserving
`scratch/simulator/monkeydo.log` and `screen.png` after each run. Both complete
logs exactly match the fixture. The generated display returns to idle after
`System.exit()`. No Garmin compiler/SDK implementation was inspected.

## Isolated bounds probes

Use the [Number bounds procedure][bounds] with the typed-array example's
lifecycle file. Each probe has one assignment, receiver/index/value helpers
that print their names, and an unreachable `after` marker:

| Probe | Array | Index | Value |
|---|---|---|---|
| Char length-equal | `['a', 'b']` | `2` | `'Ω'` |
| View negative | `[new Toybox.WatchUi.View()]` | `-1` | `new Toybox.WatchUi.View()` |

Annotate helper array parameters/results and value results with the appropriate
Char or View contract. Store each replacement Writes module and Jungle in
`scratch/compiler-typed-array-writes/char-bounds` or `view-bounds`. Jungle
manifest/resource paths move one directory deeper and sourcePath selects
`.` plus `../../../examples/compiler-typed-array-writes/app.mc`.
Build with that Jungle and the probe directory as the build-script argument;
compile/sign the same two source files through our unit PRG path.

All four SDK/generated sessions print `receiver`, `index`, `value`, then
`Array Out Of Bounds Error` and an app crash. None prints `after`. Generated
captures show the crashed-app indicator. These observations extend the Number
write's bounds/error order to the tested Char/View profiles; they do not
establish exception recovery, other index types, or other VM/device behavior.

## Compiler boundaries

Shared validation derives the value contract from the receiver's element type.
Tests reject Number/Char mixing, Null, wrong indices, and delegate/Dc/array
values in View slots. Saved IR cannot relabel an array or value to bypass those
checks. Char operands retain their API 1.3.0 requirement; View parameter writes
use the existing 1.0.0 type contract, while construction/native calls have their
own availability requirements.

Source View objects project to native View when stored in arrays. This does
not expand named source signatures, inheritance, nullable object support, or
the set of supported array element types. Complete source/resumed IR,
assembly text, and unsigned PRG checks preserve the shared pipeline.

[example]: ../../examples/compiler-typed-array-writes/README.md
[output]: ../../examples/compiler-typed-array-writes/expected.txt
[number]: compiler-array-writes.md
[bounds]: compiler-array-writes.md#bounds-failures
