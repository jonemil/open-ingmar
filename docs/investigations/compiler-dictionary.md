# Dictionary construction and operations

## Identity and hashing

**Officially documented:** [Object][object-api] declares `equals(Object or Null)`
and `hashCode()` at API 1.0.0. Hashes remain constant during an object's lifetime,
and equal objects must have equal hashes. These declarations alone do not define
Dictionary equality as content equality.

**Observed (2026-10-16):** SDK 9.2.0 / `fr265` and our generated
[specimen](../../examples/compiler-dictionary/README.md) agree: shared Dictionary
references compare equal, while distinct empty or equal-content dictionaries do
not. Comparison with supported scalars and Null returns false. Generic key/value
contracts do not change runtime identity. Repeated hashes agree through entry
replacement, insertion, and removal. Tests compare hash consistency without
assuming particular values or uniqueness. Native symbol IDs reuse `equals`
`0x008000b3` and `hashCode` `0x008000ad`.

Effect markers establish receiver-before-argument evaluation once, including
discarded results. Nullable scalar and launch-state arguments execute; the
launch state is Null. A guarded non-null callback hash type-checks but is not
exercised by this launch. Both PRGs reconstruct through structured CODE/DATA.
Artifacts: `scratch/check-dictionary-identity/`. Reproduce with the shared console
checker and the example's SDK build. Array comparisons and ordinary weak
references were not established by this probe.

[object-api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html

## Boolean and Char values

**Observed (2026-10-16):** SDK 9.2.0 / `fr265` accepts typed Boolean/Char values,
including nullable value contracts and `values()` results with matching Array
elements. The extended [specimen](../../examples/compiler-dictionary/README.md)
checks all four key kinds, constants/computed entries, indexed/native access,
mutation, and typed enumeration. SDK/generated simulator consoles match the
complete expected output. Structured reconstruction uses the established
Boolean/Char/Null DATA tags and native/index instructions without raw fallback.
Artifacts: `scratch/check-dictionary-scalars/`.

A present false value remains distinct from Null. Missing reads return Null even
for non-null V. Null writes retain keys; removal deletes them. Saved scalar
reads and enumeration Arrays retain their values across later Dictionary
mutation, while Dictionary reference copies share writes. Conditional markers
confirm skipped branches and ordered, single evaluation of computed values.
Null-only dictionaries use explicit context. These observations do not establish
heterogeneous value contracts or inherited Object equality/hash behavior.

## Enumeration Arrays

**Observed:** SDK 9.2.0 / `fr265` binds `keys` to symbol `0x008000b9` and
`values` to `0x008000ba`, using ordinary native calls. In
[`compiler-dictionary`](../../examples/compiler-dictionary/README.md), both SDK
and generated programs check typed Array results for every supported key/value
group, empty Arrays, nullable entries, and effectful receivers. Membership checks
avoid assuming order. Dictionary writes/additions/removals leave saved Arrays
unchanged; editing/appending returned Arrays leaves the Dictionary unchanged.
These scalar observations do not establish deep-copy behavior for object entries.

Reproduce with the example's SDK build and the
[console specimen check](../../scripts/README.md#console-specimen-check).
Structured export/reassembly checks both PRGs without CODE/DATA raw fallback.
The executable contracts live in the
[source profile](../specs/frontend/lang-dictionary.md#dictionary-values).

Date: 2026-10-15. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: how are Dictionaries created, accessed, and represented in bytecode?
Inputs are our authored Monkey C sources compiled with the SDK.

## Reference and broader probe

**Officially documented:** [Dictionary][api] is an Object subclass available at
API 1.0.0, with potentially heterogeneous Object keys/values. Missing lookups
return Null. `size`/`isEmpty` are API 1.0.0; the class's `toString` override is
1.0.1. Iteration/string order is not guaranteed.

**Experimentally observed:** `scratch/compiler-dictionary-probe/app.mc` creates
`{}` and `{"one" => 1, "two" => 2}`, then exercises indexed reads/writes,
`get`, `put`, `hasKey`, `remove`, `size`, and `isEmpty`. Its SDK simulator output
is `0, true, 1, null, 3, true, null, false, 2`, one value per line. Storing Null
retains the key; removing it makes `hasKey` false. This probe does not establish
all key equality/hashing rules or arbitrary heterogeneous value behavior.

The empty allocation has `ipushz` at CODE+`0x27`, byte `2f` at `0x28`, then
`lputv 2`. Treating `0x2f` as the operand-free `newd` permits decoding past this
point. An external corroboration is the Ghidra plugin's `newd` entry in
`doc/opcodes.md` and `data/languages/MonkeyC.sinc`, at the revision pinned in the
[instruction research](instruction-semantics.md). No implementation was copied.
The SDK SYMBOLS table identifies `isEmpty` as `0x008000b8`; `size` uses the
already supported `0x008000ae`.

The nonempty literal uses `42 00 00 32 ac` at CODE+`0x53`, referencing a
Dictionary record at DATA+`0x32ac`. The pinned Ghidra research names this opcode
`hpush`. The constant's local string pool uses offsets 0 and 6 for `one`/`two`;
the value block has root `0x0b` and two entries. Additional SDK specimens under
`scratch/dictionary-constants/` cover numeric-only entries, Char/Boolean/Number
mixing, shared string references, empty strings, and UTF-8. The mixed-key output
record reorders source entries; preserve the encoded order without inferring a
source iteration guarantee.

All five specimens, including the original probe, export structured CODE/DATA,
reconstruct byte-for-byte, and re-export identically. Verdicts/artifacts are under
`scratch/dictionary-constants/verified/`; the earlier opcode/DATA gaps are closed
for this flat scalar profile. Unsupported nested values and other scalar types
still stop the complete walk.

An edit of the original probe's Dictionary entry changes `one` from 1 to 41.
After project assembly and own-key signing, the simulator prints
`0, true, 41, null, 3, true, null, false, 2`, one per line. The other output is
unchanged. Artifacts are `scratch/dictionary-constants/edited*`. This establishes
editable constant contents, not fresh-versus-shared repeated-load behavior.
The [constant format](../specs/formats/data-dictionaries.md) records the verified framing.

## Compiler specimen

The [tracked example](../../examples/compiler-dictionary/README.md) isolates
empty `{}` construction, local reference copies/reassignment, typed Dictionary
arguments/results, native observers, chained calls, and repeated loop allocation.
Both SDK and generated simulator runs match all 101 expected lines. Typed
functions also check contextual empty returns/arguments/reassignment, nullable
value annotations, chaining, and erasure into an opaque parameter. The launch
provides Null callback state, so its guarded observer is checked structurally,
not executed against a populated launch Dictionary. Unguarded access is rejected
by compiler tests.

The console checker confirms source/saved-IR parity, structured CODE/DATA export,
exact PRG reconstruction, and stable re-export for both programs. No DATA format
extension is needed for this profile. It uses `ipushz; newd` for empty values,
`hpush` for constants, and existing local/member-call instructions. The codec and
assembler/disassembler recognize operand-free `newd`. Nonempty literals cover
all four key types, Number/String/null values, duplicate keys, and shared pool
strings. Indexed and native reads/writes verify values, missing-key Null results,
alias visibility, Null-valued key presence versus removal, and absent-key removal.
Two calls through `populated` produce independently mutable Dictionaries:
changing/removing entries from one leaves the other at 41 with its `two` key.
Receiver/key/value markers also agree for indexed writes, native calls, and
conditional RHS/key staging. Computed entries additionally verify duplicate
key/value effects, conditional keys/values, nullable inferred values, Array-read
producers, observation before a later value mutates state, short-circuit keys,
and repeated construction inside a loop. Both PRGs export/rebuild exactly.
Artifacts: `scratch/check-dictionary-computed/`.

The SDK SYMBOLS table binds `put` to `0x008000b4`, `get` to `0x008000b5`,
`hasKey` to `0x008000b6`, and `remove` to `0x008000b7`. The last matches Array's
existing `remove` ID; receiver-specific contracts distinguish Void Dictionary
removal from Boolean Array removal. Both programs reconstruct structurally
through the existing indexed and member-call opcodes; no additional opcode or
DATA profile is needed for these operations.

Current executable contracts are in the
[source profile](../specs/frontend/lang-dictionary.md#dictionary-values) and
[IR allocation format](../specs/ir/containers.md#ir-container-operations).

### String conversion

The specimen's `strings` function confirms API 1.0.1 `toString` on empty opaque
and populated typed references. SDK and generated runs match: `{}`, `{one=>1}`,
`{1=>one}`, `{true=>yes}`, `{a=>7}`, and `{nothing=>null}` for the isolated cases.
A saved String retains `{one=>1}` after the source Dictionary changes to
`{one=>2}`. Multi-entry checks use `find` rather than assuming entry order.
Receiver markers, chained String methods, and discarded conversions also agree.
SDK SYMBOLS uses the existing `toString` ID `0x008000af`; structured export and
exact reconstruction need no new VM or DATA support.
Artifacts: `scratch/check-dictionary-string/` (SDK 9.2.0, `fr265`, 2026-10-16).

## Literal semantics

**Experimentally observed:** SDK-only `scratch/dictionary-semantics/app.mc`
uses a method returning `{"one" => 1, "two" => 2}`. Its body is `hpush` at
CODE+`0x26`, referencing DATA+`0x3315`, followed by `return`. Three calls execute
that same instruction. After the first call, a local reference copy sees a
replacement of `one` with 41. The second call still reads 1, compares unequal
to the first result, and the copy compares equal to the first. Removing `two`
from the first result leaves a third result with both entries.

**Interpretation:** this flat scalar constant creates a fresh mutable Dictionary
per load; local copies share the returned reference. This does not establish
deep copying of nested containers or arbitrary Object equality.

The same source constructs a dynamic literal with two entries. Each key/value
comes from `key(marker, value)`, which prints the marker and returns the value:

```monkeyc
var dynamic = {key("key1", "same") => key("value1", 1),
               key("key2", "same") => key("value2", 2)};
```

The console prints `key1, value1, key2, value2`, then size 1 and value 2.
At CODE+`0xf5`, the SDK emits `ipush1 2; newd; dup 0`, evaluates the first
key/value, uses `aputvdup` at `0x11b`, then evaluates the second pair and uses
`aputv` at `0x13d`. Thus allocation precedes entry effects, and insertion occurs
between successive pairs. A constant literal `{"same" => 1, "same" => 2}`
also yields size 1/value 2; its DATA record contains only the last entry.
This does not justify eliminating effectful overwritten expressions.

A mixed literal `{1 => 10, 'a' => 20, true => 30}` returns 10, 20, and 30
for those keys, and Null for a missing String key. These results do not
establish cross-type key equality, Null keys, or iteration order.

The complete 19-line console matches `scratch/dictionary-semantics/expected.txt`.
Structured export/reassembly is byte-exact and re-export is identical, including
the dynamic construction instructions. Verdicts are in `report.json`; SDK PRG
SHA-256 is
`53493a349c8ad890e06feb03f27e4c7c7d2f6bba95260f596968af8f2a051ba4`.

## Typed key/value contracts

**Experimentally observed:** eighteen isolated SDK builds under
`scratch/dictionary-types/` use `-O 0 -l 3`. Method parameters/results accept
`Lang.Dictionary<Lang.String, Lang.Number>`; locals use inferred types.
The accepted typed application prints 1 for `one` and Null for an absent key.

| Check | SDK result |
|---|---|
| Return `d[k]` as `Number or Null` | Accepted |
| Return `d[k]` or `d.get(k)` as non-null Number | Rejected: nullable result |
| Read a Number key from `Dictionary<String, Number>` | Rejected |
| Write String through indexing or `put` to that Dictionary | Rejected |
| Write Null through indexing or `put` to that Dictionary | Rejected |
| Write Null to `Dictionary<String, Number or Null>` | Accepted |
| Return `{"one" => 1}` or `{}` as `Dictionary<String, Number>` | Accepted |
| Return `{"one" => "bad"}` with that result annotation | Rejected |
| Return `Dictionary<String, Number>` as `Dictionary<Object, Number>` | Rejected |
| Return `Dictionary<String, Number>` as `Dictionary<String, Object>` | Rejected |
| Return `keys()` as `Array<String>` | Accepted |
| Return `values()` as `Array<Number>` for non-null Number values | Accepted |
| Return `values()` as `Array<Number or Null>` for nullable Number values | Accepted |
| Return those nullable values as `Array<Number>` | Rejected |

The broad published `put` signature permits Null, but a concrete non-null value
contract rejects it. Preserve that distinction when specializing API methods.
The widening cases support invariant mutable contracts; they do not establish
all union or subclass combinations. Build diagnostics and verdicts are in each
case's `build.log` and the parent `report.json`.

## Reproduction

Use the shared console scaffold with its Jungle source path set to
`../../examples/compiler-dictionary`. Build `scratch/compiler-dictionary`, then
prepare the [console checker](../../scripts/README.md#console-specimen-check)
with `--sdk-prg scratch/compiler-dictionary/build/fr265/compiler-dictionary.prg`
and a fresh output directory. Run simulation in the SDK environment. The
example's README has standalone compile/sign/run commands.

For the broader probe, use the operations described above in an SDK-only
`onStart` with `System.exit()`. Build and run it separately, then export all
sections with `ciq-dis all` and reconstruct with `ciq-asm project`. Both exports
should be structured for the tested scalar records.
Sources, build logs, console captures, and exports are retained under
`scratch/compiler-dictionary-probe/`.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Dictionary.html

To reproduce the edit, copy the original probe's exported project and change
`.entry string=&pool_00000000 number=0x00000001` to use `number=0x00000029`.
Assemble the copied project, sign the changed PRG with the developer key, and
run it in a fresh simulator capture directory. Compare its complete console log
against the nine values above; the process exit alone is insufficient.

For literal semantics, place the described operations in an AppBase `onStart`,
print each observation, and exit. Use the shared console Jungle/manifest,
`scripts/build-example.sh`, and `scripts/simulator-session.py`. Export,
reconstruct, and re-export with the usual CLI commands, checking bytes and the
entire console. For typed cases, compile each method independently with `-l 3`;
use a minimal `onStart` that exits so unrelated code cannot hide a diagnostic.
