# Interfaces

## Comparison interface values

`Toybox.Lang.Comparable` and `Toybox.Lang.Comparator`, optionally `or Null`, are
supported parameter and result contracts from API 5.0.0. Module aliases and
imports resolve them like other API types. Comparable exposes `compareTo(Object)
as Number`; Comparator exposes `compare(Object, Object) as Number`. Calls use
ordinary receiver/member dispatch, evaluate receiver before arguments once, and
preserve the implementing instance. Comparator values can also supply Array
`sort`.

User calls, returns, and interface-local assignments accept the same interface,
a verified native scalar witness, or an exact source-class witness. Unit
validation rechecks every required source member, arity, parameter, and result
contract. Opaque Object/View values, a `has` test, and source classes with
narrowed parameters do not establish the interface. A standalone function can
retain unresolved source witnesses; assembling a complete unit requires proof.

Number, Long, Float, Double, Boolean, Char, and String satisfy Comparable. The
bounded native witness check requires the catalogue's exact method signature and
an effective introduction level no later than Comparable's API 5.0.0
requirement. Checking the receiving interface therefore covers these native
members as well. No runtime conversion, allocation, or wrapper is added. Other
native types do not establish an interface witness. A nullable scalar needs a
non-null/concrete proof before non-null Comparable use; a positive concrete
class test on an Object read can likewise establish a scalar witness. Comparison
still follows the receiver's runtime method; a common interface does not
guarantee that arbitrary operand domains can be ordered or that a call cannot
fail.

An interface parameter or call result retains its contract through inferred
locals, aliases, and same-interface assignment and conditional joins. A
Comparable local also accepts a verified scalar assignment. A conditional with a
Comparable arm and a verified scalar arm retains Comparable in either arm order;
two different scalar types do not implicitly infer an interface. There is no
implicit conversion between Comparable and Comparator. Direct source assignments
defer their signature proof to unit validation, including after IR reload; an
incompatible source class cannot enter either interface local.

A nullable interface accepts Null and compatible non-null values. Nullable
scalars can widen to Comparable-or-Null. Nullable locals keep their declared
contract across writes and require a fresh non-null read proof before member
calls or non-null forwarding. Equality/inequality with Null works in either
operand order. The shared branch rules cover early returns, conditional arms,
and loops; writes invalidate local proofs, and aliases/stored Boolean guards do
not transfer them. A conditional combining an interface and Null produces the
corresponding nullable interface. Existing nullable-interface arms can also
accept compatible scalar/interface alternatives; this does not enable general
union inference.

`Array.sort` accepts Comparator-or-Null directly: Null selects default ordering,
and a non-null instance supplies custom ordering. This does not permit calling
`compare` on a nullable receiver without a non-null proof.

Source classes also support [comparison-interface
fields](fields.md#comparison-interface-fields) with the same write contracts and
checked initialization. Arrays follow the
[interface element contract](#comparison-interface-arrays). Dictionaries follow
their [interface value contract](#comparison-interface-dictionaries). Unchecked
`as` assertions and interface runtime type targets remain unsupported. A
non-null interface can widen to Object without proving additional members. A
nullable interface can widen to Object-or-Null; Object requires a non-null
proof. Ordinary Object casts and concrete type tests retain their existing
rules.

Erased comparison calls, including sorting through a Comparator value, may
request shutdown. Constructor checks conservatively propagate that effect
through source wrappers and require reference fields to be initialized first.
This does not establish exception safety or callback termination.

## Comparison interface Arrays

Non-null Arrays support Comparable and Comparator elements, optionally `or
Null`, from API 5.0.0. Parameter/result annotations use
`Array<Toybox.Lang.Comparable>` and the corresponding Comparator/nullable forms.
Array contracts stay invariant: `Array<Number>` cannot widen to
`Array<Comparable>`, and non-null element Arrays cannot widen to
nullable-element Arrays.

An expected Array annotation or argument contract supplies literal element
typing, including empty and mixed native/source literals. Otherwise the first
element must already establish a supported element contract; arbitrary mixtures
do not infer an interface. Scalar-token literals use tagged DATA constants,
preserving Number/Char distinctions and each native runtime tag. Nullable
contracts also permit Null tokens, including Null-only literals. Each evaluation
produces a fresh mutable Array. Source-object and computed literals allocate
privately before evaluating and filling elements in order. The same declared
element contract applies to either construction path.

Indexed reads retain the declared interface, including Null where allowed; they
do not recover an exact source class. Member calls on nullable elements require
a guarded local snapshot. A test of one indexed read does not refine another
read. Writes, private literal fills, and typed native element arguments accept
compatible native/interface values or an exact source witness. Unit validation
checks required source members at every such ingress after IR reload. Opaque
Object values and incompatible signatures cannot bypass that proof.

The existing `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`,
`removeAll`, `reverse`, and `slice` profiles apply; reverse/slice preserve the
element type. `addAll` requires the same invariant Array contract, though a
direct argument literal can receive that context. Inherited Object methods
retain their existing profiles. Collection operations that may call erased
element methods (`toString`, lookup, and removal) conservatively propagate
possible shutdown effects through constructor checking.

`Array<Comparable>.sort` accepts Null, a checked exact source Comparator, or a
Comparator(-or-Null) value. Null dispatches through element `compareTo` methods;
an explicit Comparator supplies `compare` instead. `Array<Comparator>.sort`
requires a non-null Comparator: its element contract alone does not prove
`compareTo` for default ordering. A guarded nullable Comparator snapshot can
satisfy this requirement. Exact source arguments retain their non-null proof
while unit checking verifies their comparison signature.

Callers must provide mutually comparable elements or a callback that handles
every element pair. The interface proves a signature, not a common comparison
domain: native scalar witnesses and unrelated source implementations need not
accept each other. Sorting mutates shared contents and retains the invariant
element type. Erased element/callback calls conservatively propagate possible
shutdown effects through constructor checking, including default sorting.
Runtime comparison failures, sort stability, and callback count/order are not
guaranteed. Existing [sort
evaluation/lowering](containers.md#default-array-sorting) applies.

Sorting nullable elements, nested/general Object containers, and Array field
declarations remain outside this profile. Comparator values read from an Array
can supply an ordering callback. See the
[example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md#interface-arrays).

## Comparison interface Dictionaries

`Dictionary<K, Comparable>` and `Dictionary<K, Comparator>`, optionally `or
Null` on the value type, support scalar keys and the interface-key profile
below. Interfaces require API 5.0.0. Key/value parameters stay invariant across
assignments and calls; scalar-valued Dictionaries do not widen to
interface-valued ones. Erased Object values alone do not establish these
interface contracts;
[Object-valued Dictionaries](containers.md#object-containers) use a separate erased profile.

An expected Dictionary contract types direct literals and empty construction.
Values accept compatible native/interface witnesses or exact source instances;
unit validation checks required source members for every private literal fill,
indexed write, and `put` argument, including after IR reload. Opaque Object
values do not establish an interface. Computed literals retain ordered
evaluation and private construction. Scalar-only literals can use the existing
Dictionary constant payload, which distinguishes Number and Char tags under an
erased Comparable contract. Implicit inference of interface-valued literals is
deferred.

Lookup always produces the value type or Null, including for a non-null value
contract. Calls require a guarded local snapshot; `hasKey` or a guard on another
read does not refine the lookup. Indexed writes and `put` accept Null only for
nullable value contracts. `values()` returns `Array<V>`, preserving value
nullability without adding lookup's missing-key Null. `keys()` retains the key
contract. Enumeration snapshots membership, sharing element references under the
existing [Dictionary rules](lang-dictionary.md#dictionary-values).

Other supported Dictionary methods retain their profiles. Formatting may call
erased element methods, so constructor checking treats `toString` as possibly
requesting shutdown while required reference fields are pending. Opaque
Dictionary/Object string conversions retain this conservative effect: erasing
the container must not bypass initialization checks. No guarantees about
callback sequence, exception recovery, or element string representations follow.

See the [example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md#interface-dictionaries).

### Interface Dictionary keys

`Dictionary<Comparable, V>` and `Dictionary<Comparator, V>` support non-null
interface keys from API 5.0.0, combined with any supported scalar/interface
value contract. Container parameters remain invariant. `keys()` produces an
Array of the declared interface; lookup's result type and missing-key Null are
unchanged. Nullable or general Object key contracts remain unsupported. A
nullable key argument requires a fresh non-null proof.

Every literal fill, indexed read/write, and native `get`, `put`, `hasKey`, or
`remove` argument requires an interface witness. Unit validation checks exact
source members after IR reload as well as source compilation. Key identity uses
runtime hashing/equality, not `compareTo` or `compare`. Source instances may use
inherited identity or compatible [Object
overrides](classes.md#object-rooted-classes). Callers must keep key hashes
stable while stored and make equal keys hash consistently; signature checking
does not establish these behavioral laws.

Interface-key literals use private runtime construction and ordered insertion.
They do not become Dictionary DATA constants, even with scalar-token contents;
saved IR rejects `.dictionary-constant` with interface keys. All-scalar-token
literals require a single key token kind. Mixed scalar-token literals are
rejected because SDK constant construction can conflate Number and Char keys
that runtime insertion distinguishes. Heterogeneous native witnesses can enter
through explicit `put` calls or computed/runtime key operands. No compile-time
key normalization or deduplication is applied to this path.

Construction fills and key lookups/writes/removal may invoke erased key methods.
Constructor checking conservatively propagates possible shutdown through those
operations and source wrappers while reference fields are pending. Formatting
may also invoke key methods. Enumeration does not itself compare keys. Callback
count/order, mutation during callbacks, and exception recovery are not
guaranteed. See
[evidence](../../investigations/compiler-comparators.md#interface-keys).
