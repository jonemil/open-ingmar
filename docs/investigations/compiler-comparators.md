# Source Comparator matching and runtime dispatch

2026-10-17 to 2026-10-18, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Lang.Comparator][lang] requires
`compare(Object, Object) as Number`; [Array.sort][array] accepts a Comparator
or Null from API 5.0.0.

**Observed:** the [two-file specimen][example] passes source View instances to
sort without a Comparator superclass. Per-instance direction controls ascending
and descending String order; both instances record nonzero callback counts.
Aliases see sorted contents, typed source forwarding preserves the comparator,
and receiver/argument markers print once in that order. Numeric values ordered
by string conversion produce `[2, 10, 1]`, distinguishing custom dispatch from
default numeric order. Empty/singleton inputs complete.

Two isolated variants use the same View/compare shape but change both inputs
to Number, then Object-or-Null; the nullable variant returns constant zero so
no body type error obscures the result. Both SDK builds reject the class at
`sort(c)` as incompatible with the documented interface. Our source/unit
checker therefore requires exact input/result contracts, rather than assuming
function-parameter variance. Probe directory: `scratch/comparator-probe/`.

## Encoding and reproduction

Build tracked sources through an ignored Jungle using the shared native-names
manifest/resources and `scripts/build-example.sh`. The [console checker][check]
validates saved stages, structured SDK/generated CODE/DATA/SYMBOLS, lossless
reconstruction, and simulator output. Artifacts: `scratch/check-comparators/`.
The SDK symbol table binds `compare` to `0x00800150`; generated class metadata
must use that ID for runtime dispatch. `sort` retains `0x00800e51` and existing
invocation instructions; no new opcode or DATA record was encountered.

Using an existing View class isolates sorting from Object-rooted construction.
Nullable/object Array elements were not established. Callback counts, call order
within sorting, stability, and exception
behavior are not inferred. Current rules belong in the [sorting contract][spec];
the [plan][plan] owns remaining core work.

## Comparable signatures

**Officially documented:** [Lang.Comparable][comparable], reviewed 2026-10-18,
requires `compareTo(Object) as Number` from API 5.0.0. It is a structural
interface, rather than a native superclass.

**Experimentally observed:** an Object-rooted `Candidate` class with
`compareTo(other as Lang.Object) as Lang.Number` is accepted by this SDK consumer:

```monkeyc
function accept(value as Lang.Comparable) as Lang.Number {
    return value.compareTo("a");
}
```

Its implementation returns `"b".compareTo(other.toString())`.
`accept(new Candidate()) > 0` prints true. Changing only the parameter to Number,
Object-or-Null, or Candidate makes the SDK reject the argument at `accept`.
The last two bodies return constant zero to isolate the signature requirement.
This supports exact matching; projecting a source class to native Object loses
a restriction relevant to interface compatibility. Probe source/Jungle and
variant diagnostics: `scratch/comparable-contract-probe/`; runtime capture:
`scratch/comparable-contract-sdk-runtime/`.

The tracked `TextOrder` uses the same comparison body through direct source
calls. SDK/generated programs match all 71 expected lines. Structured CODE/DATA,
saved-stage equivalence, and reconstruction pass through the [console checker][check]
in `scratch/check-interface-arrays/`. Existing compareTo binding/instructions
handle these calls; no new opcode or DATA record was encountered.
The extended tracked consumer forwards Comparable and Comparator parameters and
results through locals, invokes both methods, and supplies an erased Comparator
to sort. Both SDK/generated runs print `interface-receiver` before
`interface-argument`, then the expected comparison results and `[c, b, a]`.
The direct `compare` call uses existing symbol `0x00800150` and ordinary invoke
instructions. No new opcode or DATA record was encountered. This does not
establish Dictionary interface storage,
or default sorting of source-object Arrays. The [API contract][api] owns the
signature-query behavior and limits.

## Native Comparable witnesses

**Experimentally observed:** the tracked `NativeComparables` passes Number,
Long, Float, Double, Boolean, Char, and String to a Comparable parameter, then
compares each with an equal value through `compareTo(Object)`. All return zero.
A Number returned as Comparable compares greater than a smaller Long. Assigning
String to that interface local changes subsequent dispatch to String comparison.
Conditional arms combining Comparable and scalar values retain the selected
String in both arm orders. A positive Number test lets a fresh nullable Object
read cross the Comparable boundary; Null follows the rejected branch.

The SDK accepts these authored sources; SDK/generated simulator output matches
the expected text through the console checker above. Saved stages and structured
reconstruction pass. Dispatch uses the existing `compareTo` symbol and invocation
instructions, without new opcodes or DATA records. This establishes these
witnesses and comparisons, not arbitrary mixed-domain comparison behavior or
exception recovery.

## Nullable interfaces and source assignments

**Experimentally observed:** the tracked `NullableInterfaces` returns source
instances or Null through Comparable/Comparator-or-Null signatures. A null-first
early return permits comparison on the surviving path. Null guards on locals
also permit member calls in an if/while body; assigning Null ends the loop.
Sorting with a Null Comparator prints `[a, b, c]`; the non-null descending
Comparator prints `[c, b, a]`.

The SDK accepts direct assignment of source implementations to existing nullable
and non-null interface locals, plus later Null/scalar writes where the declared
contract permits them. Both simulator programs match the tracked expected text.
The shared console checker above verifies saved stages and structured
reconstruction with existing Null, comparison, branch, and invocation encodings;
no new opcode or DATA record was encountered. Source assignments still need
exact member proofs; successful runtime cases do not establish broader source
inheritance, Dictionary interface storage, or exception recovery.

## Interface fields

**Experimentally observed:** the tracked `InterfaceHolder` stores both interfaces
in nullable and non-null fields. The SDK accepts these declarations and a literal
Null initializer on the nullable Comparator field. Reading unset nullable fields
produces Null: guarded comparison takes the `-9` path and sorting uses default
order. Source constructors fill non-null fields before use. Replacing a stored
Comparable with Number or a source implementation changes subsequent dispatch;
stored source Comparators remain callable after their constructor returns.
Two holders retain independent values.

`takeMaybe` snapshots a nullable Comparable, clears its field, and successfully
compares through the guarded local. A later field read takes the Null path.
SDK/generated simulator text matches the expected output. The shared console
checker above covers saved stages, structured CODE/DATA, and reconstruction.
Ordinary Null field slots and existing field/invocation instructions suffice;
no new opcode or DATA record was encountered. Direct repeated-field guard
refinement, general native field types, and Dictionary interfaces remain outside
the supported contract.

## Interface Arrays

**Experimentally observed:** the tracked `InterfaceArrays` builds invariant
Comparable/Comparator Arrays, with and without Null elements. Literals combine
source instances and native witnesses under an explicit element contract.
Indexing, indexed replacement, `add`, `addAll`, `indexOf`, `remove`, `removeAll`,
`reverse`, and `slice` complete; an alias observes the new size. Two references
to the same source instance are found and removed together. A mixed literal's
Char element still passes `instanceof Lang.Char`. A nullable indexed snapshot
can invoke its interface after a null guard. Reading a Null Comparator element
and passing it to scalar Array.sort selects default ordering.

SDK/generated programs match the expected text through the console checker
above. Both CODE/DATA walks and structured reconstruction succeed without new
opcodes or DATA records. Our builder path preserves mixed scalar tags without
an interface constant-payload grammar. These cases do not establish sorting the
interface-valued Arrays themselves, callback ordering, or exception recovery.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparator-typedef
[comparable]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparable-typedef
[api]: ../specs/api-declarations.md#declaration-model
[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[example]: ../../examples/compiler-comparators/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/compiler-subset.md#source-comparators
[plan]: ../plan.md
