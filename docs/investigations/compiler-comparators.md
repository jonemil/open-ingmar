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
Nullable/general Object Array elements were not established. Callback counts,
call order within sorting, stability, and exception
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
calls. SDK/generated programs match all 147 expected lines. Structured CODE/DATA,
saved-stage equivalence, and reconstruction pass through the [console checker][check]
in `scratch/check-interface-array-constants/`. Existing compareTo binding/instructions
handle these calls; no new opcode or DATA record was encountered.
The extended tracked consumer forwards Comparable and Comparator parameters and
results through locals, invokes both methods, and supplies an erased Comparator
to sort. Both SDK/generated runs print `interface-receiver` before
`interface-argument`, then the expected comparison results and `[c, b, a]`.
The direct `compare` call uses existing symbol `0x00800150` and ordinary invoke
instructions. No new opcode or DATA record was encountered. The
[API contract][api] owns the signature-query behavior and limits.

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
inheritance or exception recovery.

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
refinement and general native field types remain
outside the supported contract.

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
opcodes or DATA records. Mixed source/native literals preserve scalar tags
through private construction. These cases do not establish callback ordering
or exception recovery.

## Interface Array sorting

**Experimentally observed:** `InterfaceSorting` sorts `Array<Comparable>` values
containing Object-rooted `Ranked` instances. Null selects their `compareTo`
method and produces ascending ranks with duplicates retained. `RankOrder`
reverses the result through `compare(Object, Object)`, records callback activity,
and works as both an exact source instance and an erased Comparator. Shared
references observe each mutation. Forwarding Comparator-or-Null selects either
path. Homogeneous Number and String witnesses sort normally under the Comparable
element contract; an empty Array completes. `Array<Comparator>` containing Ranked
instances sorts through an explicit RankOrder, including a forwarded Comparator.

The shared console checker above covers SDK/generated text and structured
reconstruction. Existing compareTo/compare bindings and
invocation instructions suffice; no new opcode or DATA record was encountered.
This establishes neither ordering arbitrary mixed Comparable domains nor default
ordering of Comparator-only elements, nullable elements, callback sequence,
stability, or exception recovery. Source casts in the authored callbacks assert
the Ranked domain; they do not perform runtime conversion or validation.

## Interface Dictionaries

**Experimentally observed:** `InterfaceDictionaries` uses String-keyed
Dictionaries with Comparable/Comparator values, with and without Null. Source
instances enter through literals, indexed writes, and `put`; native scalar
Comparable witnesses also fit the declared value contract. Lookup of a missing
key returns Null. Guarded snapshots invoke comparison methods. Aliases observe
writes/removal; a `values()` Array retains its original membership after removal.
`keys()` returns a String Array. Retrieved nullable Comparators select default
or explicit ordering for scalar Arrays. A flat Comparable constant retains
distinct Number and Char runtime tags.

SDK/generated output matches the expected text through the console checker
above. Structured CODE/DATA reconstruction
and saved stages pass with existing dictionary constants, private builders,
lookup/write instructions, and native dispatch. No new opcode or DATA record
was encountered. These observations do not establish general Object containers,
formatting callback order, or exceptions.

## Interface keys

**Experimentally observed:** `InterfaceKeys` stores Comparable and Comparator
source instances as keys. `HashKey` returns a constant hash to force collisions
and compares its rank in `equals(Object or Null)`. Distinct equal instances
replace one entry; a different rank remains separate. `get`, indexed reads and
writes, `put`, `hasKey`, and `remove` work through source and interface keys.
Hash/equality counters show callback activity; ordering counters remain zero.
Enumeration returns interface keys whose array membership survives later removal.
TextOrder and Ordering keys without overrides use inherited identity.

Native Comparable keys inserted with explicit `put` calls retain Number, Char,
String, Boolean, Long, Float, and Double identities in this specimen. Number 97
and Char 'a' occupy separate entries and retrieve values 1 and 2. SDK/generated
programs match all 132 expected lines; artifacts:
`scratch/check-interface-keys-runtime/`, SDK build log:
`scratch/interface-keys-sdk-build.log`. Structured reconstruction and saved
stages pass with existing dictionary, hash/equality, class, and invocation
encodings. No new opcode or DATA record was encountered.

**SDK constant-literal discrepancy:** an earlier `nativeKeys()` used
`return {97=>1, 'a'=>2, "a"=>3, true=>4, 1l=>5, 1.0=>6, 1.0d=>7};` under the same
`Dictionary<Comparable, Number>` return annotation. The SDK program reports six
entries, finds Number 97 with value 2, and does not find Char 'a'. Our private
runtime insertion reports seven entries and retains both keys. Captures and
diff: `scratch/check-interface-keys/runtime/`; the original SDK PRG is retained
in that preparation directory. Replacing the literal with the tracked explicit
`put` sequence makes both programs agree. This distinguishes constant
construction from runtime insertion; it does not establish a portable
cross-kind normalization rule. Mixed scalar-token interface-key constants are
therefore outside the supported source profile, and interface-key DATA constant
emission remains deferred.

**SDK rejected overrides:** copies of the specimen in
`scratch/interface-key-overrides/` change `hashCode` to accept a Number argument,
then separately change `equals` to accept Number with a constant false body.
The SDK rejects these as incompatible native Object overrides. Diagnostics:
`scratch/interface-key-bad-hash-sdk.log` and
`scratch/interface-key-bad-equals-sdk.log`. Our declaration checks preserve the
native signatures even after erasure; they do not prove behavioral hash laws,
callback ordering, mutation safety, or exception recovery.

## Interface Array constants

**Experimentally observed:** `InterfaceConstants` returns scalar-only Comparable
Arrays containing Number, Char, Boolean, Long, Float, Double, and String. Each
element passes its runtime class test; Number 97 and Char 'a' retain distinct
tags. Mutating one returned Array updates its alias while a separate call
retains the original Number. Nullable Comparable contents retain Null and scalar
tags, and a guarded read invokes comparison. A Null-only Comparator Array
selects default sorting when indexed as a callback.

The generated program loads these literals from DATA constants. SDK/generated
simulator output matches all 147 expected lines, and saved IR, assembly,
structured CODE/DATA, signing, and reconstruction pass through the console
checker. Artifacts: `scratch/check-interface-array-constants/`; SDK build log:
`scratch/interface-array-constants-sdk-build.log`. Existing scalar tags, local
String pools, and `apush` suffice; no new opcode or binary record was encountered.
This establishes scalar constant storage, not source-object constant encoding
or a shared ordering domain for arbitrary Comparable mixtures.

## Native Comparable refinement

2026-10-21, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

The tracked `InterfaceRefinements` group passes each scalar Comparable through
an interface-or-Null parameter, tests its native class, and performs a concrete
operation. It also forwards the narrowed read to a Comparable consumer. Cases
cover Number/Long/Float/Double arithmetic, Boolean inversion, Char conversion,
String length, and Null. A loop changes an interface local from Number to
String; an Array element replacement leaves an earlier refined snapshot intact.
SDK and generated simulator output both match the expected additions: eight
true results and two `42` results. Both PRGs export/reassemble structurally and
saved-stage compilation agrees.

Reproduce with the shared console checker for `compiler-comparators`; artifacts
are in `scratch/core-interface-refinement/check/`. Native/text IR uses the same
storage facts and established VM operations; no new opcode or DATA record is
required. This does not establish interface runtime targets or new structural
witness classes.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparator-typedef
[comparable]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html#Comparable-typedef
[api]: ../specs/sdk/README.md#declaration-model
[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
[example]: ../../examples/compiler-comparators/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#source-comparators
[plan]: ../plan.md
