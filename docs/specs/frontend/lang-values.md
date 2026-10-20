# Core Lang values and methods

## Strings and printing

Double-quoted literals accept UTF-8 characters and source escapes `\n`, `\r`,
`\t`, `\"`, and `\\`. Raw newlines, unfinished/unknown escapes, and values
longer than 65535 UTF-8 bytes are rejected. Empty strings are allowed. Source
Unicode escape syntax is not supported; IR/assembly use their own documented
escape syntax.

String literals, native conversion results, and string-valued local reads can
initialize locals or supply `System.println(expression);`. Print statements may
occur between local initializations and accept one Number, Long, Float, Double,
String, Boolean, or Char value. String types propagate through locals and
annotated function parameters/results; incompatible uses are rejected. Addition
follows the [operator contracts](operators.md#operator-contracts), including
Char concatenation and supported scalar conversions. Arbitrary SDK calls, and
using `System.println` as an expression remain unsupported. A local/parameter
named `System` cannot be used as the print receiver. The function's scaffold
must provide the SDK module. The [native name
profile](declarations.md#native-name-declarations) resolves its supported
spellings; Jungle resolution remains unimplemented.

## Object equality and container observers

Supported numeric, String, Boolean, Char, Symbol, Array, Dictionary, ByteArray,
opaque Object, Method, and WeakReference receivers accept `equals(other)` →
Boolean. Arguments follow the catalogue's Object-or-Null contract, including
supported source/native objects, comparison interfaces, containers, numeric
aliases, and nullable forms. Nullable arguments require no proof; nullable
receivers still require a non-null proof. Container comparison neither requires
matching element/key/value contracts nor converts their types. General Object
storage and recovery retain their existing restrictions.

All supported Arrays and Dictionaries inherit `equals` and `hashCode()` → Number
from Object, since API 1.0.0. Element/key/value availability is checked
separately. Container equality tests reference identity: aliases compare equal,
while separately allocated containers with equal contents compare unequal.
Hashes remain stable across mutations and agree for aliases. The runtime
supplies their values; distinct containers need not have different hashes.

Both calls preserve receiver-before-argument evaluation and support discarded
results. Native dispatch retains runtime semantics for scalar equality,
including String content comparison and numeric class distinctions; no result or
hash is folded. Scalar override/API requirements remain in their respective
sections. This profile does not enable Array/Dictionary equality operators, or
general callback consumers. See the
[observer evidence](../../investigations/compiler-object-observers.md).

## Comparison Object arguments

Number, Long, Float, Double, Boolean, Char, and String `compareTo(other)` take
one non-null Object and return Number, since API 5.0.0. Supported Object
arguments include scalar classes, numeric aliases, containers, source/native
objects, and comparison interfaces. A nullable argument requires a current
non-null proof. Source and saved IR check the declared API signature without
narrowing it to the receiver's runtime comparison domain.

The runtime still checks that domain: numeric/Boolean/Char receivers compare
numeric, Boolean, or Char values; String compares Strings. An incompatible
Object argument is valid at the static call boundary but can throw
`UnexpectedTypeException`. The compiler inserts no conversion, dynamic class
guard, or fallback comparison. Null is rejected statically by the non-null
parameter contract. Comparison signs are meaningful; particular magnitudes and
exception recovery are not established by this profile. See the
[Object argument example](../../../examples/compiler-comparison-objects/README.md)
and [runtime evidence](../../investigations/compiler-comparison-objects.md).

## Native value calls

A Number-valued expression may receive these catalogue-resolved methods:

| Method | Declaring class | Result |
|---|---|---|
| `toString()` | `Lang.Object` | String |
| `format(String)` | `Lang.Number` | String |
| `abs()` | `Lang.Number` | Number |
| `toNumber()` | `Lang.Number` | Number |
| `toLong()` | `Lang.Number` | Long |
| `compareTo(Object)` | `Lang.Number` | Number |
| `equals(other)` | `Lang.Object` | Boolean |
| `hashCode()` | `Lang.Object` | Number |

Number `compareTo` requires API 5.0.0 and follows the
[shared Object argument contract](#comparison-object-arguments). The other
listed Number methods require 1.0.0. `equals` uses the
[shared equality contract](#object-equality-and-container-observers).
Object arguments use catalogue-declared inheritance, without implicit
conversions or acceptance of unknown classes. Scalar user-function contracts
remain exact. Comparison/equality/hash behavior is supplied by the runtime;
comparison signs and hash consistency are the contracts, not particular
magnitudes or hashes. See the [Number group
evidence](../../investigations/compiler-number-methods.md).

String-valued expressions support these zero-argument methods, declared by
`Lang.String`: `length()` and `hashCode()` return Number; `toLower()`,
`toUpper()`, and `toString()` return String. These methods are available since
API 1.0.0. Runtime implementations determine character counts and case mapping;
the compiler does not evaluate these calls at compile time.

String `compareTo(Object)` follows the
[shared comparison contract](#comparison-object-arguments) and requires API 5.0.0. String `equals` returns Boolean and accepts
the shared Object equality arguments, since API 1.0.0.
`toNumberWithBase(Number)` returns Number or Null, since API 1.4.1, and follows
the existing null-check rules. The base is 0 for prefix detection or 2..=36 for
an explicit radix. Numeric ranges and conversion content are passed to the
runtime without compiler validation. See the
[String group evidence](../../investigations/compiler-string-methods.md).

Boolean receivers support `toString()` returning String, `hashCode()` returning
Number, and `equals` returning Boolean with the shared Object argument profile
as Number/String equality. These inherited methods date from API 1.0.0. Boolean
`compareTo` follows the
[shared Object argument contract](#comparison-object-arguments) since API 5.0.0.
This native-call support does not enable Boolean arithmetic or ordering
operators. See the
[Boolean group evidence](../../investigations/compiler-boolean-methods.md).

Char values originate from literals or Number's zero-argument `toChar()` and are
distinct from Number. They support `toLower()`/`toUpper()` returning Char,
`toNumber()` returning Number, and `toString()` returning String. Char's direct
`compareTo` follows the [shared Object argument
contract](#comparison-object-arguments) since API 5.0.0. Inherited `equals`
accepts the shared Object argument profile and returns Boolean; `hashCode()`
returns Number. Char itself and its conversions require API 1.3.0, including
when it appears only in a function signature. Typed arguments, returns, local
initialization/assignment, printing, and chained calls retain the exact Char
type. Char operators follow the [operator
contracts](operators.md#operator-contracts); nullable Char requires a non-null
proof for scalar operators and method receivers. Conversion ranges and
case/String behavior are runtime obligations; the compiler adds no Unicode
scalar-range validation. See the
[Char group evidence](../../investigations/compiler-char-methods.md).

Numeric, String, Boolean, and Char receivers also support zero-argument `weak()`
since API 1.2.0, with Char receivers additionally requiring 1.3.0. Its effective
result is the exact receiver type, not WeakReference. It remains an effectful
native call; the compiler does not replace it with its receiver. Nullable scalar
receivers require a direct non-null proof before the call. Ordinary references
follow the separate profile below. See the [weak-result
evidence](../../investigations/compiler-immutable-weak.md).

String results may initialize/replace a String local or be printed. Number
results support integer locals, arithmetic, comparisons, user-call arguments,
integer returns, and further Number calls. For example, `(0 -
41).abs().format("%03d")` produces `"041"` on the validated profile. Existing
user-call arguments and return restrictions remain unchanged.

Postfix calls bind more tightly than addition/subtraction; parenthesize a
computed arithmetic receiver, for example `(n + 1).format("%03d")`. Literal,
local, numeric/String/Boolean/Char parameter, and supported call-result
receivers are accepted. Evaluate the receiver first, then arguments left to
right, then invoke the member.

The frontend distinguishes visible local/parameter receivers from module calls.
Supported native value calls may also be statements: `n.abs();`,
`chars.remove('a');`, and `factory.make().add('x').size();` evaluate the call
and discard its final result. Receiver/argument order, API checks, and nullable
receiver proofs are the same as in value contexts. A discarded result does not
contribute to function return inference. Nullable results need no guard when
discarded. Bare arithmetic, literals, and element reads are not source
expression statements. Native Void methods still require their own supported
statement contract; discarding does not enable unsupported members or receiver
profiles. See the
[statement evidence](../../investigations/compiler-native-statements.md).

Null receivers remain unsupported; Dictionary receivers follow their bounded
[profile](lang-dictionary.md#dictionary-values). Chained calls
must resolve each member against the preceding result's type, for example
`41.toString().toUpper().length().format("%03d")`. String and Boolean user
parameters/results require the [function
annotations](types.md#types-and-value-contracts). String `find(String)`,
`toNumber()`, and `toNumberWithBase(Number)` return Number or Null;
`substring(Number or Null, Number or Null)` returns String or Null. These follow
the
[null-check rules](types.md#nullable-results). Unknown catalogue
members, wrong arity/types, and known members lacking lowering are distinct
errors. Explicit [API context inputs](../sdk/context.md#api-checking-inputs)
check introduction levels; general native dispatch remains unsupported. Format
content is passed to the runtime; type checking does not establish that a String
contains a valid format pattern.

[Native call IR](../ir/calls.md#native-value-calls) preserves receiver/member
identity separately from executable IDs. Runtime evidence and reproduction are
in the [native call experiment](../../investigations/compiler-native-calls.md).

## Symbol values

`:name` produces a non-null `Toybox.Lang.Symbol` since API 1.0.0. Names use the
ASCII identifier grammar. A Symbol is distinct from Number and String; matching
names denote the same identity within a build. Numeric IDs are assigned during
application/assembly binding and need not agree between builds or toolchains. Do
not treat them as persistent identifiers.

Symbols support annotated parameters/results, exact local assignment, matching
conditional arms, `==`/`!=` between Symbols, and the shared native `equals`
argument profile. They support `toString()` → String, inherited `hashCode()` →
Number and `equals` → Boolean since 1.0.0, immutable `weak()` → Symbol since
1.2.0, and `toNumber()` → Number since 2.3.0. Equality and conversions remain
native operations; no numeric ID or hash is folded into semantic IR.

Complete applications retain names for literal Symbols in a SYMBOLS section, so
their development `toString()` results use those names. This does not promise
names for arbitrary runtime-created values or release builds without reflection
metadata. Raw CODE output needs an explicit symbol source; it cannot allocate an
independent identity space.

`Symbol or Null` supports annotated parameters/results, local storage, and
Symbol/Null conditional joins from API 1.0.0. Forwarding and returns preserve
the declared union even when the current value is a Symbol. Local writes may
replace it with Symbol, Null, or the same union. `==`/`!=` with literal Null
support fresh local/parameter proofs in either operand order. Guarded reads may
compare with other Symbols, invoke methods, or act as `has`/`method` selectors.
Existing
[null-proof rules](types.md#nullable-results) apply across branches, short circuit
expressions, and loops; writes invalidate the local's proof. Copies and stored
Boolean guards do not transfer it. Nullable Symbols cannot satisfy a non-null
Symbol consumer without proof. Native `equals` arguments explicitly permitting
Null retain their shared argument profile. Method API requirements still apply
after a guard, including 2.3.0 for `toNumber`.

Symbol arithmetic, ordering, truthiness, and direct printing remain unsupported.
Classes follow the [Symbol field contract](fields.md#symbol-fields). Method
lookup follows its separate contract below. See the [specimen and binding
evidence](../../investigations/compiler-symbols.md).

### Symbol containers

Arrays support invariant Symbol and Symbol-or-Null elements from API 1.0.0.
Symbol Dictionary keys are non-null; values may be Symbol or Symbol-or-Null,
combined with other supported key/value contracts. An expected contract types
empty and nullable literals. Non-null Symbol literals can infer homogeneous
Array/Dictionary contracts; arbitrary scalar/Symbol mixtures do not infer a
broader union. Existing mutation, collection, Object observer, and enumeration
profiles apply. `reverse` and `slice` return independent Arrays. Symbol is not a
Comparable witness; non-null elements require a non-null checked
[Comparator](containers.md#source-comparators) for sorting.

Indexed reads, `get`, `keys`, and `values` preserve their declared types.
Dictionary lookup always adds missing-key Null; `values()` does not add it.
Snapshot a nullable element and guard the local before Symbol consumers.
Repeated indexed reads and `hasKey` do not establish a proof. Nullable key
arguments need a fresh non-null proof; nullable key contracts and nullable
container references remain unsupported.

Symbol-bearing literals use ordered private construction with symbolic `spush`
operands. Raw CODE resolves names through the supplied symbol source; complete
applications share the normal name/ID map, including pinned native member IDs.
No numeric Symbol ID is placed in semantic IR or treated as Number. Flat DATA
constants do not accept Symbol tokens. A Null-only payload under a nullable
Symbol element/value contract can still use the existing Null constant encoding.
Pooled Symbol encodings require separate evidence/contracts.

## Method values and invocation

Supported scalar, Array, Dictionary, opaque Object, WeakReference, and Method
values and supported native/source-class receivers support `method(Symbol)` →
non-null `Toybox.Lang.Method`, since API 1.0.0. Receiver class availability and
construction retain their own API requirements. Method lookup and missing-member
errors remain runtime operations. The Method retains its bound receiver;
replacing the original variable does not release that receiver while a Method
still owns it.

Method supports annotated parameters/results, local storage/assignment, matching
conditional arms, and inherited `toString`/`equals` plus its own `hashCode`,
since 1.0.0. `weak()` returns WeakReference from 1.2.0. Native observers retain
runtime identity/hash behavior: repeated bindings need not compare equal even
for the same target/member.

`Method or Null` supports explicit parameter/result contracts, locals,
assignment, and conditional joins with Method or Null alternatives. Nullable
locals retain their declared union when assigned a Method or Null. Equality and
inequality with literal Null establish proofs for fresh reads of that storage;
invocation and every Method member require a non-null proof. Writes invalidate
local proofs; aliases and stored Boolean tests do not transfer them. Releasing
the last Method alias also releases its bound receiver. Lexical `as Method or
Null` is an unchecked type assertion, with no runtime conversion or signature
recovery. [Method fields](fields.md#method-fields) support required/nullable
storage. Arrays and Dictionaries follow the
[Array](#method-arrays) and [Dictionary](#method-dictionaries) profiles below.

`invoke(arguments...)` accepts up to 254 supported Object/Null values and
returns opaque `Object or Null`. This erased result covers scalar/container
values, nullable returns, and Void callbacks. Save and guard it before Object
member calls; it cannot become a Number, typed container, or Method without a
separate refinement contract. Statement invocation discards the result. Receiver
evaluation precedes arguments, left to right, and each runs once.

The compiler checks the Method API and VM argument-count limit. The target's
existence, arity, accepted types, introduction level, and result behavior remain
runtime/caller obligations; dynamic lookup does not recover its signature.
Source instances support `instance.method(selector)` and instance bodies support
`self.method(selector)` or bare `method(selector)`. Bare calls resolve to the
current instance and use the same semantic operations as explicit self calls. A
declared source member named `method` keeps ordinary source-call priority. Bare
`self` produces opaque Object and requires an instance body; it does not recover
a source-class type or concrete callback signature. Existing `self.member(...)`
source calls retain checked signatures. Bare `method(...)` requires an instance
body; a local, parameter, or field named `method` prevents implicit lookup.
Calling such lexical values remains unsupported; use explicit `self.method(...)`
for the instance member. Other unqualified function/member calls are outside
this profile. Receiver capture still requires all required instance fields to be
initialized.

Unlike the SDK checker, this erased lookup profile does not statically resolve a
literal selector to a source target. A missing target or incompatible invocation
remains a runtime/caller obligation even for literal selectors. Constructors and
typed signatures remain deferred. Array sorting uses a [separate Comparator
contract](containers.md#source-comparators). See the
[binding/lifetime evidence](../../investigations/compiler-methods.md).

### Method Arrays

`Array<Method>` and `Array<Method or Null>` preserve invariant element contracts
through parameters, results, locals, indexed reads/writes, and matching Array
conditional arms. The value types require API 1.0.0; native Array methods retain
their existing API requirements. A non-null-element Array cannot widen to a
nullable-element Array. Empty/nullable literals need a concrete expected
contract; otherwise a first non-null Method can establish the literal element
type.

Method-bearing literals allocate privately and fill in source order at runtime.
They do not embed Method objects or addresses in DATA constants. Null-only
nullable literals can use the existing Null constant encoding. Each construction
produces a fresh mutable Array.

The supported `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`,
`removeAll`, `reverse`, and `slice` profiles apply. Element arguments match the
declared nullability; `addAll` requires the same invariant Array contract.
Reverse/slice produce independent Arrays while retaining the same Method
references. Search and removal use runtime Method equality; separately created
bindings need not compare equal. Non-null Method elements support sorting with a
non-null checked
[Comparator](containers.md#source-comparators), preserving their bound references.

Nullable indexed reads need guarded local snapshots before invocation or member
use; a test of one indexed read does not refine another. Replacing/removing an
element preserves saved aliases. Releasing the last Method reference releases
its bound receiver. Array storage does not recover a callback signature or a
concrete invocation result. See the
[specimen](../../../examples/compiler-methods/README.md) and [ownership
evidence](../../investigations/compiler-methods.md#nullable-ownership).

### Method Dictionaries

Typed Dictionaries accept non-null Method keys, Method values, and nullable
Method values from API 1.0.0, alongside their other supported key/value domains.
Both mutable parameters remain invariant through signatures, storage, and
writes. Nullable keys require a non-null proof before lookup or mutation; Null
keys remain unsupported.

Method-bearing literals allocate privately and evaluate each key before its
value, in entry order. Homogeneous Method candidates can establish a literal's
contract; Null-only values need an expected nullable contract. Existing scalar
keys with Null-only nullable Method values may use DATA constants. Method keys
or values are never encoded as pooled scalar payloads or numeric addresses.

`get` and indexed lookup return Method or Null even for non-null Method values,
because the key may be missing. Guard a local snapshot before invocation;
`hasKey` and a test of another mutable lookup do not refine that read.
`put`/indexed writes preserve declared nullability. Existing `size`, `hasKey`,
`remove`, and observer profiles apply. Runtime Method equality/hash determines
key identity; repeated aliases replace an entry, while independently created
bindings need not compare equal. No hash algorithm is specified.

`keys()` returns `Array<Method>` for Method keys. `values()` returns an Array
with the declared Method/nullability contract, without adding the missing-key
Null alternative. These Arrays and lookup snapshots retain the same Method
objects independently of later Dictionary mutation. Keys and values both own
their stored Methods; the bound receiver expires only after all aliases release
them. Erased binding/invocation still establishes no concrete callback
signature. See the [specimen](../../../examples/compiler-methods/README.md) and
[ownership evidence](../../investigations/compiler-methods.md#nullable-ownership).

## Ordinary weak references

Supported Arrays, opaque/typed Dictionaries, Method, and WeakReference receivers
support `weak()` → `Toybox.Lang.WeakReference`, since API 1.2.0. WeakReference
is a non-null value with `get()` → `Toybox.Lang.Object or Null` and
`stillAlive()` → Boolean, both since 1.2.0. It also inherits `toString`,
`equals`, and `hashCode`; calls retain runtime behavior rather than synthesizing
wrapper identity. There is no `clear()` method. Release a strong reference
through assignment or frame exit; the runtime determines when the target ceases
to exist.

Source signatures accept `Toybox.Lang.WeakReference`, `WeakReference or Null`,
`Toybox.Lang.Object`, and `Object or Null`. They survive locals, function
calls/results, saved IR, and ordinary branch/loop null proofs. A retrieved
Object is a strong value; saving it can keep the target alive. Save and guard
that value before calling its supported Object methods (`toString`, `equals`,
`hashCode`). Neither `stillAlive()` nor testing a separate `get()` call proves a
later result non-null.

Nullable WeakReference locals retain their union through assignment and joins
with WeakReference/Null alternatives, since API 1.2.0. A fresh non-null proof
permits handle members; it proves neither target liveness nor a non-null `get()`
result. Writes invalidate local proofs; aliases and stored Boolean tests do not
transfer them. Lexical `as WeakReference or Null` asserts a type without runtime
checking, conversion, or ownership changes. Required/nullable
[WeakReference fields](fields.md#weakreference-fields) preserve these
contracts. Null storage and an expired non-null WeakReference are distinct
states.

Object contracts erase concrete type information without changing runtime
values. Compatible values may pass to Object parameters/results and replace
opaque Object locals; a nullable Object local also accepts Null. Erasure cannot
be reversed into a typed Array/Dictionary or a scalar. Object joins accept
matching opaque contracts and Object/Null alternatives; matching WeakReference
arms also join. These rules do not introduce heterogeneous container joins.

The declared Object type does not establish a WeakReference result from
`Object.weak()`: the runtime value might be an immutable scalar. That receiver
path, other native/source weak targets, Dictionary weak keys, and safe concrete
recovery remain outside this profile. The new types do not acquire arithmetic,
truthiness, direct printing, or general equality operators; nullable Object/Null
tests use the existing proof rules.

Hidden reference temporaries transfer ownership on their final read using IR
`.take`; private container builders release their frame reference on finish.
Source locals retain their assignment/frame lifetimes. Backend staging must not
silently extend a target's strong lifetime. See the
[lifetime specimen and evidence](../../investigations/compiler-weak-references.md).

### WeakReference construction

`new Toybox.Lang.WeakReference()` constructs a fresh native class instance;
canonical, module-alias, and imported bare names are accepted. Qualifier
shadowing and constructor arguments are rejected. This is distinct from
`target.weak()` and does not accept a target object.

The bounded profile follows SDK 9.2.0/`fr265` simulator evidence: a fresh
instance reports alive and its `get()` result compares equal to itself. Saved
retrievals retain the instance normally. The compiler still types `get()` as
Object or Null; it does not infer self identity or non-null retrieval from
construction.

Construction is effectful and uses ordinary native allocation/initialization.
The class's API 1.2.0 introduction is the minimum availability guard, including
after IR reload. Garmin's published WeakReference page does not declare a
constructor, so this guard is not a published constructor-compatibility claim
for every older SDK/device. See the
[constructor evidence](../../investigations/compiler-weak-references.md#zero-argument-construction).

### WeakReference containers

`Array<WeakReference>`, `Array<WeakReference or Null>`, and typed Dictionary
WeakReference/nullable-WeakReference values require API 1.2.0. Array elements
and both Dictionary parameters remain invariant. Existing native container
methods retain their API requirements. WeakReference Dictionary keys remain
unsupported following an [insertion
failure](../../investigations/compiler-weak-references.md#dictionary-key-rejection)
in SDK 9.2.0 programs on the `fr265` simulator, including live targets. This is
a bounded compatibility decision, not a claim about every Garmin runtime.

Weak-bearing literals allocate privately and evaluate entries in source order.
Homogeneous weak handles may infer a contract; empty and Null-only literals need
an expected type. Null-only nullable payloads with supported scalar Dictionary
keys may use existing DATA constants. Weak handles/targets are never encoded as
pooled objects or numeric addresses. Nested containers remain unsupported.

Arrays preserve their element type through parameters/results, indexed reads and
writes, and matching conditional arms. `size`, `toString`, `add`, `addAll`,
`indexOf`, `remove`, `removeAll`, `reverse`, and `slice` use the existing
profiles. Reverse/slice produce independent Arrays with the same weak handle
values; search/removal uses native WeakReference equality. Non-null weak
elements support sorting with a non-null checked
[Comparator](containers.md#source-comparators); sorting handles does not retain
their targets.

Dictionary `get`/indexing adds Null for a missing key, including for non-null
WeakReference values. `values()` returns an Array with exactly the declared
value contract; it does not add the missing-key alternative. Existing `put`,
indexed writes, `hasKey`, `remove`, and observers preserve that contract.

Nullable element/lookup members need a fresh guarded local snapshot. A test of
another mutable read does not refine the value. Storage and copies retain the
weak handle without retaining its target; a non-null handle may already be
expired. Neither storage nor `stillAlive()` removes Null from a later `get()`
Object result. See the
[specimen](../../../examples/compiler-weak-references/README.md) and
[evidence](../../investigations/compiler-weak-references.md#container-storage).

## Lang.format

`Lang.format(format as Lang.String, parameters as Lang.Array) as Lang.String` is
available since API 1.0.0. Module aliases and fully qualified Toybox paths use
the ordinary native-name rules. Arguments are evaluated left to right; the
runtime substitutes one-based `$1$`, `$2$`, etc. placeholders. The compiler
passes the template unchanged and does not perform formatting or conversion. The
returned String supports normal storage, calls, returns, and discarding.

The Array parameter is erased. Any supported concrete Array element contract can
pass unchanged; it does not become `Array<Object>` or lose its invariant element
type. Direct literals receive nullable Object construction context, allowing
heterogeneous scalar/reference contents, nulls, and empty literals. This context
does not widen inference for unrelated literals or existing arrays. Null/opaque
Object arguments cannot establish the required Array contract.

Formatting can invoke element `toString` implementations. Treat it as a possible
shutdown callback when checking required constructor field initialization.
Runtime conversion and placeholder failures remain runtime behavior. The
[specimen](../../../examples/compiler-lang-format/README.md) and
[evidence](../../investigations/compiler-lang-format.md) cover substitution,
evaluation order, and a missing-parameter failure.
