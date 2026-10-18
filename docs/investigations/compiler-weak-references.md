# Ordinary weak references and hidden ownership

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [WeakReference][weak] and `get()`/`stillAlive()`
require API 1.2.0. `get()` returns Object or Null after the target loses its
strong references. There is no explicit clear method. [Object.weak()][object]
distinguishes ordinary weak results from immutable scalar values.

**Observed:** the [authored specimen][example] retrieves live Array/Dictionary
objects, sees mutations through the retrieved reference, and compares them with
the original target. A saved `get()` result keeps the target alive after the
original local is replaced. Clearing that retrieved local lets the weak result
expire. Results returned from functions that retain no other target reference
are expired after the function returns. `stillAlive()` is false and `get()` is
Null for those cases. These are simulator observations, not collection timing
guarantees for every device.

Aliases and repeated weak results for the same live target compare equal, with
matching hashes in the tested calls. `toString()` is repeatable within the
probe; no representation or hash algorithm is inferred. Calling `weak()` on the
expired weak result produces another expired result in this specimen, so it
must not be modeled as constructing a strong wrapper around that WeakReference.
Keep native dispatch; do not assume wrapper identity semantics.

## Lifetime discrepancy

An initial generated program retained the Array after both source owners were
replaced. Its IR exposed an extra hidden local for the constant literal; the
backend kept that strong reference until frame exit. The SDK program expired
the target. This establishes a semantic ownership issue, rather than a need to
change the expected output or insert a weak-reference-specific guard.

The retained specimen also exercises a compound write with a branching RHS,
computed Array/Dictionary builders, nullable retrieval joins, opaque Object
parameters/results, and function-frame exit. Explicit temporary transfers and
builder publication release hidden owners; named locals retain their normal
lifetimes. The source and saved-IR pipelines must preserve those operations.

## Encoding and reproduction

Compile the tracked source with an ignored Jungle and the shared native-names
manifest/resources. Use the
[console checker](../../scripts/README.md#console-specimen-check) for saved-stage
parity, structured SDK/generated reconstruction, own-key signing, and simulator
comparison. Both programs match all 58 expected console lines. Artifacts:
`scratch/check-weak-containers/`.

SDK symbols use `weak` = `0x00800418`, `stillAlive` = `0x00800417`, and
`get` = `0x008000b5` (already used by Dictionary). Existing member invocation,
local reads/writes, and `npush` suffice. No VM opcode or PRG record is added;
`.take` is a semantic IR ownership operation lowered to these instructions.

## Nullable handles and fields

2026-10-18, same SDK/simulator and target.

**Observed:** nullable WeakReference parameters, results, conditional joins,
locals, and `WeakStore`/AppBase fields retain a usable weak handle after another
slot is cleared. Its target expires while non-null handles remain stored. An
Object retrieved before replacing the original owner keeps the target alive;
clearing that Object expires it. A Null handle and an expired handle therefore
remain distinct even though neither produces a live target.

Required/nullable field metadata uses the existing Null slots. Saved IR and
structured section reconstruction preserve the specimen's behavior without
new opcodes or PRG records. API checks reject declarations below 1.2.0, including
unused nullable fields after IR reload. Lexical assertions are checked only as
unchanged-value lowering; they establish no runtime type or liveness proof.

## Container storage

2026-10-18, same SDK/simulator and target.

**Observed:** Arrays, reverse/slice copies, and Dictionary values keep weak
handles usable while their original Array target expires. An independently
saved `get()` Object delays expiry until released. Nullable indexed/lookup
snapshots remain usable after their entries are cleared or removed; their
`get()` results remain Null for the expired target.

A whole-Array conditional join aliases its selected Array. Mutating that Array
changes the alias's size while its reversed copy remains independent. Search
and removal accept the saved expired handle in this specimen. Null-only values
enumerate as nullable WeakReferences, and literal markers run once in order
(`60`, `61`). No general equality/hash algorithm or weak-key behavior is inferred.

Source/IR validation rejects contract widening, unguarded nullable element use,
scalar payloads under weak constant contracts, and API declarations below 1.2.0.
Structured SDK/generated reconstruction needs no new opcode or DATA record.
Other native/source weak targets, checked concrete recovery, Dictionary weak
keys, and broader callback interactions remain open.

[weak]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/WeakReference.html
[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-weak-references/README.md
