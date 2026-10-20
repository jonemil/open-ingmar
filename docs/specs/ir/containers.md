# IR container operations

## Dictionary allocation

`.dictionary-new` defines a fresh empty non-null Dictionary. Native construction
uses `Operation::DictionaryNew` with optional `dictionary_type`; omission means
an opaque Dictionary. Validation requires Dictionary's API 1.0.0 class, single
use, block-local dependencies, and ordered allocation. Lowering emits `ipushz;
newd`; no DATA record or compiler capacity operand is involved. Allocation
failures remain runtime obligations.

`dictionary` is the opaque parameter-type and value-call result tag.
`dictionary(<key>=><value>)` also tags `.block-value` joins of matching typed
Dictionaries. Both incoming contracts must match exactly; this neither enables
Dictionary/Null joins nor parameterized Dictionary casts. Reference ownership
and empty-stack receiver setup follow the ordinary join/staging rules.

`dictionary(<key>=><value>)` preserves typed contracts in `parameter-types=`,
function `result=`, call `result=`, and allocation `type=` fields. Keys are
`integer`, `long`, `float`, `double`, `string`, `boolean`, `char`, `symbol`,
`comparable`, `comparator`, or `method`. Values use those tags,
`weak-reference`, `byte-array`, or their `nullable-` counterparts. Keys and
values also accept `numeric`, `integer-union`, and `decimal`; only values accept
their nullable forms. Values also accept `object` and `nullable-object`, with
the
[source erasure rules](../frontend/containers.md#object-containers).
Interface value contracts require API 5.0.0; WeakReference requires 1.2.0 and
ByteArray requires 3.0.0. Other tags, nested ordinary containers, and arbitrary
heterogeneous unions are rejected. For example:

```text
ciq-ir
.function make result=dictionary(string=>nullable-integer)
.dictionary-new %0 type=dictionary(string=>nullable-integer)
.return %0
.end
```

Typed and opaque Dictionary result headers remain explicit so an erased or
differently typed allocation cannot change the contract on reload.
`DictionaryType` backs the `TypedDictionary` parameter/result variants. Locals
and calls preserve reference identity and invariant contracts; erasure to opaque
Dictionary is one-way. `.call-native` accepts the [source method
profile](../frontend/lang-dictionary.md#dictionary-values); Void `put`/`remove`
calls use `.call-native-discard`. Existing nullable callback reads narrow to
opaque Dictionary under a non-null proof. Conditional Dictionary joins remain
unsupported. See the
[source profile](../frontend/lang-dictionary.md#dictionary-values).

## Dictionary access

`.dictionary-read` reads a typed non-null Dictionary and yields V or Null.
Native construction uses `Operation::DictionaryRead`; lowering evaluates
receiver then key and emits `agetv`. Missing keys yield Null rather than an
Array bounds failure.

`.dictionary-write` inserts or replaces an entry without producing a result.
`Operation::DictionaryWrite` evaluates receiver, key, and value, then emits
`aputv`. It does not use an Array index or require an existing key. Both
operations validate against the generic `get`/`put` contracts, including
key/value types and API availability.

Native `get`/`hasKey` use ordinary `.call-native`; `put`/`remove` use the Void
statement form. Explicit method calls preserve member dispatch. Saved IR infers
lookup nullability from the validated receiver, never from key presence. Opaque
receivers, wrong keys/values, unproved nullable operands, and invalid value uses
of Void methods fail validation. See the
[source rules](../frontend/lang-dictionary.md#dictionary-values).

Reads and writes participate in the effect schedule. A read consumed before a
write cannot move after it. Branching operands use the existing staging rules;
incoming join parameters cannot bypass the stack-use checks. Source candidate
types select the operation, while full IR validation proves its safety.

## Dictionary constants

`.dictionary-constant` defines a fresh non-null Dictionary from nonempty flat
scalar entries. Native construction uses `Operation::DictionaryConstant`,
`DictionaryType`, and pairs of `DictionaryScalar`. Scalar syntax is `number
0x<bits>`, `char 0x<bits>`, `boolean true|false`, `string "text"`, or `null`.
Number/Char bits must fit u32; Strings require quotes and use the existing
escape rules. For example:

```text
.dictionary-constant %0 dictionary(string=>nullable-integer) string "one" number 0x00000001 string "missing" null
```

An interface value contract can accept compatible scalar witnesses while each
token retains its VM tag. Source/interface references use private construction
instead; they cannot be represented as scalar constant tokens. Private fills,
indexed writes, and native `put` share the
[source/unit interface proof](../frontend/interfaces.md#comparison-interface-dictionaries).
Interface keys require runtime construction, even with scalar contents; this
constant operation rejects those key contracts. Private fills, indexed reads and
writes, and native key arguments retain the
[key witness obligation](../frontend/interfaces.md#interface-dictionary-keys).

The [source contracts](../frontend/lang-dictionary.md#dictionary-values) govern
supported key/value kinds, nullability, and entry typing. Missing pairs, empty
entry lists, incompatible scalar types, and unencodable pool strings fail before
lowering. The operation retains source entry order and duplicate keys for
inspection; materialization retains the last value for equal scalar keys and
shares equal pool strings. No computed entries are folded into this operation.

[VM lowering](../lowering/values.md#dictionary-constants).

Long/Double scalar payloads use u64 hexadecimal bits; Float uses u32, alongside
the existing Number/Char/Boolean/String/Null payloads. Exact class/bits survive
text and DATA binding. Constant-key coalescing follows the
[source rules](../frontend/lang-dictionary.md#dictionary-values), including distinct signed
zeros and non-coalescing Double NaNs.

## Computed Dictionary construction

`DictionaryBegin`, `DictionaryFill`, and `DictionaryFinish` represent a private
nonempty literal with the [source key/value
contract](../frontend/lang-dictionary.md#dictionary-values). Their text forms
are:

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Each accepts optional `span=<start>:<end>`. Length is the positive source entry
count, including duplicates, and must fit a nonnegative Number. A fill supplies
one or more pairs, beginning at the next source-entry ordinal and fitting the
remaining count. Entries check against K/V using the same put contract as
indexed writes. Finish consumes the completed builder and produces its typed
Dictionary.

[VM lowering](../lowering/calls.md#computed-dictionary-construction).

## Private container checks

Array and Dictionary builders share one identity namespace and lifetime checker.
Begin declarations are unique within a function; operations cannot substitute
one builder kind for the other. Every CFG join requires identical active-builder
progress on incoming paths, including backedges. Returns cannot leave builders
active. Unknown/inactive builders, early publication,
skipped/repeated/out-of-order fills, and incompatible values fail validation. A
completed identity can begin again only by re-executing its original
declaration, such as in a loop.

Builders reserve frame slots alongside parameters and ordinary locals under the
shared 255-slot limit. They are not semantic values or ordinary local IDs; only
finish exposes a usable reference. Allocation remains ordered before entry
effects and can fail there. Finishing either builder kind clears its private
strong reference; a completed container is owned by the produced value and its
subsequent consumers.

## ByteArray values and allocation

`byte-array` denotes a non-null `Toybox.Lang.ByteArray`; `nullable-byte-array`
adds Null. Both are parameter, function-result, call-result, and
conditional-edge tags. ByteArray is distinct from `number-array`; no conversion
or element-nullability inference connects them. Local assignment and matching
conditional arms retain the ByteArray reference. ByteArray/Null joins retain
either runtime value without conversion. Reuse the
[source null-proof rules](../frontend/lang-bytearray.md#bytearrays) in native and saved IR;
annotations do not assert a proof. API 3.0.0 also applies to nullable contracts
without an allocation. Class fields use
[`.field-byte-array`](../frontend/fields.md#bytearray-fields). Array elements reuse
these tags under the [allocation grammar](#array-allocation). Dictionary value
contracts use `dictionary(<key>=>byte-array)` or
`dictionary(<key>=>nullable-byte-array)` with an existing supported key
contract. Lookups yield nullable ByteArray; `values()` preserves the declared
value nullability in its Array element tag. Availability applies to unused types
and Null-only constants as well as construction and reads. ByteArray references
use private Dictionary fills, never scalar constant tokens; nullable constants
can contain Null under existing payload validation.
`dictionary(byte-array=><value>)` enables non-null ByteArray keys, with matching
`byte-array-array` results from `keys()`. Reference-bearing entries remain
runtime construction rather than scalar constants.

`decodeNumber` and `encodeNumber` use ordinary native calls, API 3.1.0, and the
[conversion contracts](../frontend/lang-bytearray.md#bytearrays). `number-format` and
`endian` are distinct parameter/result/cast/join tags, not `integer`.
`.constant` retains enum identity and introduction level; lowering pushes its
integer value. Decode retains its Numeric result after reload.

`.byte-options-begin` allocates a private Dictionary for zero to two fields.
Existing `.dictionary-fill` operations require direct literal Symbol keys:
`offset` with Number, or `endianness` with Endian. Values are staged before
key/value setup when needed. `.dictionary-finish` exposes a sealed
`byte-options` record after complete ordered fills, clearing builder ownership.
The `byte-options` tag also supports matching conditional edges; lexical casts
cannot manufacture this proof. Native and saved IR check field names/types,
counts, order, block/value uses, and the shared frame limit. CODE uses ordinary
Dictionary construction and calls; no codec opcode or new DATA framing is added.

`.byte-array-new` allocates a fresh zero-filled ByteArray. Length must be an
earlier same-block non-null Number value. Validate API 3.0.0, single-use/block
locality, and allocation effect order; negative/oversized lengths and allocation
failure remain VM obligations. Lower the length once followed by `newba`, with
no DATA binding or initializer call. A direct incoming block parameter can
supply the length without staging.

Existing `.array-read` and `.array-write` operations also accept ByteArrays.
Reads produce Number; writes accept Number/Char. Preserve evaluation order,
runtime index bounds and byte range checks, and unsigned readback. No masking,
clamping, or expansion is emitted by the compiler. Native calls reuse the
catalogue and ordinary value-call lowering under the
[source method profile](../frontend/lang-bytearray.md#bytearrays).

`.byte-array-constant` contains exact unsigned bytes; bare `0x` denotes an empty
payload. Hex digits must occur in pairs. API and ordered-effect checks still
apply. Lower to a ByteArray literal requirement and `bpush`, using a separate
DATA binding namespace. Source signed byte normalization happens before this IR
operation; readers never truncate wider numbers or reinterpret the payload as
scalar tags.

`.byte-array-begin` allocates a private ByteArray of a positive constant Number
length with `newba`. It shares `.array-fill`, `.array-fill-many`, and
`.array-finish` with indexed Array construction, retaining a distinct ByteArray
contract. Values must be non-null Number/Char; bounds remain runtime checks.
Existing builder validation requires complete, ordered, exactly-once
initialization on every CFG path, forbids early escape/restart, and counts
private slots toward the common frame limit. Grouped fills use retaining stores;
branching elements stage incoming values before builder/index setup. Finish
produces ByteArray and clears the builder's hidden ownership. No append loop or
intermediate Array is emitted.

## Array allocation

`.array-new` allocates a fresh empty non-null array. Scalar element tags are
`number`, `long`, `float`, `double`, `char`, `boolean`, `string`, and `symbol`,
each also accepting a `nullable-` prefix. `view` represents the supported native
View contract. `comparable`, `comparator`, `nullable-comparable`, and
`nullable-comparator` retain comparison-interface element contracts. `method`,
`weak-reference`, and their `nullable-` forms retain callable/weak-handle
contracts. `byte-array` and `nullable-byte-array` retain strong ByteArray
references. Numeric alias tags follow the [alias
grammar](values.md#numeric-aliases). `object` and `nullable-object` retain
erased Object elements; their parameter and call-result tags are `object-array`
and `nullable-object-array`, and their conditional-edge tags are `array-object`
and `array-nullable-object`. Construction uses `Operation::ArrayNew` with the
corresponding `ArrayElement` variant. Other parsed element contracts are
rejected. Definition/use, block, and effect checks apply even when the
allocation result is consumed later. API checking requires Array and the element
type: 5.0.0 for comparison interfaces, 1.3.0 for Char, 1.2.0 for WeakReference,
3.0.0 for ByteArray, otherwise 1.0.0 within this profile.

Parameter/user-call Array categories append `-array` to these element tags, for
example `long-array`, `nullable-double-array`, or `view-array`. Nullability
belongs to the element, not the Array reference. Function Array results are
inferred from typed returns rather than a separate `.function result=` field.

[VM lowering](../lowering/values.md#array-allocation).

`.array-allocate` allocates a fresh Null-filled Array. Length is an earlier
same-block non-null Number value, consumed once before `newa`. Only nullable
element tags from the allocation profile above are accepted. Lengths are not
clamped or validated against a compile-time size limit; VM failures remain
observable. Validation rechecks type availability, definition/use rules, and
allocation effect order. An incoming block parameter can supply length directly:
`newa` consumes it without inserting stack setup. The result retains its
invariant nullable element type even for a constant zero length.

## Constant arrays

```text
.array-constant %0 number 0x0000000a 0x00000014
.return %0
```

`.array-constant` defines a fresh mutable array. Element tags follow [Array
allocation](#array-allocation), excluding View; at least one element is
required. Number/Char/Float values use `0x` followed by exactly eight
hexadecimal digits; Long/Double require sixteen. Values retain their raw bits,
including signed floating zero and NaN payloads. String values are quoted text
using the ordinary IR String escapes, for example `.array-constant %0 string
"one" ""`. Boolean payloads use `true`/`false` or `ArrayConstantValue::Boolean`;
numeric 0/1 do not substitute for them; Null requires a nullable element
contract. Typed IR uses distinct `ArrayConstantValue::Number` and `::Char`
variants, `::Long`, `::Float`, or `::Double` for those exact bits, or `::String`
for text; nullable contracts also permit `null` / `ArrayConstantValue::Null`.
Every payload must match its element contract; other mixed types and nested
containers are rejected. API checks require Array and each element class, as for
allocation. Definition/use and ordered-effect checks include this allocation.

Object element tags use the tagged scalar tokens below for supported scalar
constants; `nullable-object` also permits Null. Validation checks each payload's
API requirements independently of its Object contract. Reference values use
private construction, not scalar DATA tokens. Unit/native calls preserve the
same [Object storage contracts](../frontend/containers.md#object-containers).

Numeric alias element tags use tagged scalar tokens to retain each alternative,
with the same syntax as [Dictionary constants](#dictionary-constants):

```text
.array-constant %0 numeric number 0x00000001 long 0x0000000000000001 float 0x3f800000 double 0x3ff0000000000000
.array-constant %1 nullable-integer-union number 0x00000001 null
```

Each scalar must satisfy the alias/nullability contract. In particular, Char
cannot substitute for a numeric token, and integer payloads cannot satisfy
Decimal. Numeric aliases reuse the supported mixed-scalar DATA framing.

Interface element tags use tagged scalar tokens with the same syntax as
[Dictionary constants](#dictionary-constants), retaining each runtime class:

```text
.array-constant %0 comparable number 0x00000061 char 0x00000061 string "a"
.array-constant %1 nullable-comparable null boolean true
.array-constant %2 nullable-comparator null
```

Every token must satisfy the declared interface/nullability contract; no source
reference or arbitrary Object token is supported. Comparable admits the seven
supported native scalar witnesses. Comparator has no scalar witness, so only
Null fits its nullable constant profile. Interface contracts require API 5.0.0.
Typed IR shares the tagged scalar enum with `DictionaryScalar`; Number and Char
remain distinct even outside a scalar element contract. Nested containers and
empty constant payloads remain rejected. Computed/source-object elements use
private construction. Concrete scalar-element Array text retains the compact
untagged payload syntax above.

`symbol` and `nullable-symbol` are Array element tags; their parameter/call
result categories are `symbol-array` and `nullable-symbol-array`. Dictionary
keys and values also accept `symbol`, with `nullable-symbol` available only for
values. These contracts require API 1.0.0 and use the
[Symbol container rules](../frontend/lang-values.md#symbol-containers). Symbol-bearing
literals emit `.symbol` values and private fills. No constant token accepts an
embedded numeric Symbol ID; Null-only nullable payloads remain valid constants.

Method Array elements use `method` or `nullable-method`; parameter/call result
categories use `method-array` or `nullable-method-array`. Runtime literals use
private Array builders and ordinary Method values; constants permit only Null
under the nullable element contract. No Method payload token or DATA tag is
added. See [Method Arrays](../frontend/lang-values.md#method-arrays). Dictionary
tags also accept `method` keys and `method`/`nullable-method` values, for
example `dictionary(method=>nullable-method)`. Method entries use private
runtime construction; [Method Dictionary
rules](../frontend/lang-values.md#method-dictionaries) also permit existing
scalar-key/Null-only nullable constants.

WeakReference Array elements use `weak-reference`/`nullable-weak-reference`;
parameter/call results use
`weak-reference-array`/`nullable-weak-reference-array`. Dictionary values use
those element tags, for example `dictionary(string=>nullable-weak-reference)`;
weak keys are unsupported. Weak-bearing literals use private builders. Existing
Null-only constants need no new DATA tag. See [WeakReference
containers](../frontend/lang-values.md#weakreference-containers).

[VM lowering](../lowering/values.md#constant-arrays).

## Private array construction

Builders are not language values or ordinary locals. Their IDs use `&<u32>`;
they cannot be returned, copied, passed to calls, or read with `.read`.

```text
.array-begin &0 number 2
.integer %0 10
.integer %1 20
.array-fill-many &0 0 %0 %1
.array-finish %2 &0
.return %2
```

- `.array-begin` allocates the final length.
  Element contracts follow [Array allocation](#array-allocation).
  Length is decimal, positive, and fits a nonnegative Number. Builder
  declarations are unique within a function. Allocation remains ordered and may fail before element effects.
- `.array-fill` initializes the next slot.
  The decimal index must equal the number of slots already filled and be less
  than the declared length. The value must be assignable to the element contract. Nullable contracts
  accept the base scalar, Null, or their union without weakening null checks
  for non-null builders.
- `.array-fill-many` initializes a nonempty group
  of consecutive remaining slots, evaluating and storing each value before
  the next. Start and group length must fit the remaining construction range.
  Values obey ordinary definition, single-use, block, and effect-order checks.
- `.array-finish` requires all slots initialized. It consumes
  the builder and exposes a non-null typed Array; reuse requires execution of
  the original begin again, such as in a loop.

Optional `span=<start>:<end>` follows each operation. Builders follow the
[shared private construction checks](#private-container-checks).

Each builder reserves a VM frame slot alongside parameters and ordinary locals,
under the shared 255-slot limit. Lowering selects a Number push, `newa`, and a
private store at begin; fills use `aputv`, or `aputvdup` followed by final
`aputv` for grouped values. Finish loads the completed array. No native `add`
binding or availability requirement is imposed by construction. API checks still
apply to Array, element types, and any calls within elements. Empty construction
keeps the separate `.array-new` operation. See
[lowering evidence](../../investigations/natural-lowering.md#array-construction).

## Array element reads

The read result follows the receiver's element type, including native View.

`.array-read` consumes a non-null supported array and non-null Number index, and
defines the element type, including nullable scalar elements. Shared validation
recomputes operand types, null proofs, block-local definitions, one-use rules,
and evaluation order. No element/result type annotation is trusted. Array
evaluates before index, once each; lowering emits those operands then `agetv`.
Reads are order-sensitive because bounds failures are observable, even when the
function has no calls, prints, or writes.

Index values are not range-checked, clamped, or adjusted by the compiler.
Negative and other out-of-range Numbers remain accepted IR inputs whose reads
may fail at runtime. Other element types, String/dictionary receivers, and
nullable arrays remain unsupported. See the
[source profile](../frontend/containers.md#char-arrays) and
[runtime evidence](../../investigations/compiler-array-reads.md).

## Array element writes

`.array-write` is a statement with no result. It consumes a non-null supported
array, non-null Number index, and a value matching the invariant Number, Char,
or native View element type. Source View instances project to native View; array
slots retain no exact source-class identity. Shared validation recomputes these
types and null proofs, checks defined block-local single-use operands, and
preserves their left-to-right evaluation order. Writes activate the effect-order
check even without calls or printing; reads cannot move across a write.

[VM lowering](../lowering/values.md#array-element-writes).

Source indexed compounds capture array/index with ordinary typed locals, perform
`.array-read` before RHS effects, apply the binary directive, and finish with
`.array-write`. Plain RHS expressions inline the old read into the binary value.
A branching RHS stores that old read and the computed result before creating the
final write operands, preserving block-local IDs and effect order. No
indexed-compound directive or result-type annotation is introduced. See
[source rules](../frontend/statements.md#indexed-compound-assignment) and
[evidence](../../investigations/compiler-compound-targets.md).
