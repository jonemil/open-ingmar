# Instance storage

## Number instance fields

Source classes may declare up to 256 fields with signed 32-bit decimal Number
literal initializers. These inferred fields have a fixed Number contract; typed
declarations follow the [scalar-field contract](#typed-scalar-fields). Other
inferred numeric literals and computed declaration initializers are unsupported.
Parentheses around a literal are accepted. These restrictions and the
field-count limit define our subset, not a device memory limit.

Each field name must be a valid, non-reserved identifier, unique within its
class. Reject names shared with source methods or catalogue-known inherited
methods, and `View`, `AppBase`, or `init`, which collide with
native-base/constructor metadata. Different classes may use the same field name.

```text
var count = 7;
function bump() { self.count = self.count + 1; }
```

Methods read a field with `self.<name>` and replace it with `self.<name> =
<expression>;`. The RHS must produce Number and is evaluated before the write. A
local or parameter with the same name remains separate; bare names keep existing
local/parameter lookup rules. Assignment is a statement, not a value expression.
Number fields also support all ten arithmetic/bitwise compound operators:
`self.count += rhs;` reads the old field once before RHS effects, applies the
corresponding Number binary operation, then stores the result. A RHS that
changes the field does not change the captured old value. Branching RHS
expressions stage the old read through ordinary typed locals. Non-Number results
are rejected. Arbitrary receiver properties, static fields, and source-inherited
fields remain unsupported. See
[target evidence](../../investigations/compiler-compound-targets.md).

`ClassDeclaration.fields` holds names and FieldKind defaults/contracts. Unit
validation resolves every SelfFieldRead/SelfFieldAssign against its enclosing
class; standalone method IR retains this ownership obligation. Function
validation requires an instance receiver, supported assignment values, and
expression evaluation order; unit validation enforces the exact field
declaration. A read that must survive a subsequent write/call is stored in a
local first. The same checks apply to typed and saved IR.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation.

Application generation records each field with key bits zero, type `0x01`, and
the initial Number's 32-bit pattern. Fields are sorted with other class members.
The runtime supplies a separate field value for each constructed instance;
constant initialization precedes its source initialize method. Reads lower to
`getselfv`; writes to `getself`, `spush`, RHS instructions, then `putv`. See the
[counter example](../../../examples/compiler-fields/README.md) and
[instance isolation evidence](../../investigations/compiler-fields.md).

## Typed scalar fields

All supported source class bases accept explicit Number, Long, Float, Double,
Boolean, Char, and String fields, including `or Null` forms. Canonical names,
imports, and module aliases resolve the same contracts. Fields retain their
exact scalar class without implicit conversion; numeric promotion in an RHS must
still produce a type assignable to the field. Operators and native methods
follow the ordinary scalar rules. Char declarations require API 1.3.0; the other
scalar classes require 1.0.0. Base classes retain their own availability.

Typed fields start as Null metadata slots. A non-null declaration requires
assignment in `initialize` under the
[required-field prefix rules](#source-reference-fields). Nullable fields may
omit a default or use literal `null`. Other declaration initializers remain
outside this profile. Shared member-name and 256-field limits apply.

Nullable reads need a guarded local snapshot before non-null use; testing one
mutable field read does not refine a later read. Clearing the field leaves
earlier snapshots intact. Writes, compound assignments, and statement updates
use the same type and evaluation-order rules as locals. Each instance owns its
slots independently.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[example](../../../examples/compiler-scalar-fields/README.md) and
[evidence](../../investigations/compiler-scalar-fields.md).

## Numeric alias fields

All supported source class bases accept fields declared as `Lang.Numeric`,
`Lang.Integer`, or `Lang.Decimal`, including `or Null` forms. Canonical names,
imports, and aliases resolve to the same contracts. Their scalar alternatives
and operator rules follow the [numeric value
contract](numeric-values.md#numeric-values).

Initialization, Null metadata, snapshots, evaluation order, and field limits
follow the [typed-field rules](#typed-scalar-fields).

Reads retain the declared alias and nullability. Writes must satisfy every
possible alternative of the RHS, without implicit numeric conversion. Numeric
accepts all four scalar classes, Integer accepts Number/Long, and Decimal
accepts Float/Double; non-null fields reject nullable values without a proof.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[example](../../../examples/compiler-numeric-fields/README.md) and
[evidence](../../investigations/compiler-numeric-unions.md#field-storage).

## Object and container fields

All supported source class bases accept `Lang.Object` and `Lang.Object or Null`
fields. Values erase to Object without conversion; scalars, known enums, source
objects, and supported native/container references retain their runtime values.
Concrete source identity, member signatures, and container parameters are not
recovered from the field declaration. Use the ordinary class proofs or lexical
assertion contracts for more specific consumers. Nullable reads require fresh
local snapshots and guards.

Non-null `Lang.Array<T>` and `Lang.Dictionary<K,V>` fields use the existing
supported element/key/value contracts, including nullable contents and numeric
aliases. Reads, writes, mutation, and method calls retain invariant container
types. Existing arrays/dictionaries cannot widen through assignment. Direct
literals receive field construction context, including empty literals and
conditional arms; this does not broaden unrelated literal inference.

Initialization and field limits follow the [typed-field
rules](#typed-scalar-fields). Object fields may have literal Null defaults only
when nullable. Containers require assignment in `initialize`; declaration
defaults, opaque unparameterized Dictionary fields, and nullable whole-container
fields remain unsupported. Payload types retain their API requirements even for
unused fields.

Fields hold strong references. Replacing a field releases its ownership, while
other aliases, snapshots, entries, or enumerations can keep the value alive.
Transfer conditional values out of their CFG join before field receiver setup;
hidden staging slots must not extend ownership after their final use.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[example](../../../examples/compiler-container-fields/README.md) and
[evidence](../../investigations/compiler-container-fields.md).

## Source-reference fields

`var target as HelloView;` declares an exact source reference field for an
Object-, View-, or BehaviorDelegate-derived class. The class must resolve across
the unit, including forward declarations. The 256-field limit and name/conflict
rules cover all supported fields together. These exact-source fields reject
declaration initializers and Null unions. Comparison interfaces follow their
[own field rules](#comparison-interface-fields).

Metadata creates a null slot (key bits zero, type `0x00`, value zero). A source
initialize method is required to establish the non-null field contract. It must
assign every required non-null reference field before exposing `self` as an
Object, binding a self callback, making a self call, branching, jumping, or
returning. A reference read before that field's assignment is rejected.
Assignment must use the exact declared source class; native erasure, another
class, and Null cannot satisfy it. Requiring this straight-line initialization
prefix is our bounded checking policy, not Garmin's complete
definite-initialization rules. Calls that may request System.exit, including
transitive source calls and constructors and source comparator callbacks reached
through Array sorting, are also rejected until all required fields are
established. Erased Method invocation, interface calls, and sorting through
nullable/non-null Comparator values are conservatively included when the exact
callback body is unavailable. This conservative call-graph check includes
conditional exits; runtime failures and additional native callback effects are
not proven safe by it.

After initialization, methods may read or replace the field with the same exact
source class. Field reads retain identity for ordinary instance calls and source
returns. Access remains limited to `self.<field>`; a getter can forward the
reference to another caller. Reads/writes preserve the existing evaluation-order
rules. These contracts apply to typed and saved IR as well as source.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[retained View example](../../../examples/compiler-reference-fields/README.md) and
[constructor/lifecycle evidence](../../investigations/compiler-reference-fields.md).

View lifecycle callbacks follow the [SDK callback
contracts](../sdk/callbacks.md#source-views-and-drawing).

## Comparison interface fields

Source classes support `var value as Toybox.Lang.Comparable;` and the
corresponding Comparator declaration, optionally `or Null`. Module aliases and
imports resolve the type through the existing name rules. All require API 5.0.0,
including unused declarations; source and saved-IR API entry points check field
types independently of method references. Fields share the existing name,
conflict, and 256-field limits. Access remains limited to `self.<field>`.

Metadata creates ordinary Null slots (type `0x00`, value zero). A non-null field
requires a source initializer and follows the straight-line
[reference-initialization requirements](#source-reference-fields). It cannot
be read until assigned, and must be filled before self exposure, self calls,
control flow, return, or calls that may request shutdown. Nullable fields start
as Null and do not require assignment; an explicit literal `= null` is
equivalent. Other declaration initializers remain unsupported.

Writes accept the corresponding interface, a verified native witness, or an
exact source implementation under the
[interface contract](interfaces.md#interfaces). Nullable
fields additionally accept Null and compatible nullable values. Unit validation
checks required source members on every write, including after text reload.
Nonnull fields reject nullable writes without a non-null proof. A read's saved
result contract must exactly match its field declaration. Function IR alone
checks supported operands; the declaring unit proves the field's actual type.

Reads retain the declared interface contract, rather than the last stored source
class. Non-null reads can call interface methods directly. A nullable read needs
a local snapshot and null guard for member calls: testing one field read does
not refine another mutable field read. Clearing or replacing the field does not
change a previously captured local value. Nullable Comparator reads can supply
Array.sort directly. These rules do not add field/class intersection proofs,
general native field types, or container field declarations.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the [comparison
example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md#interface-fields).

## Method fields

All supported source class bases accept annotated `Toybox.Lang.Method` and
`Method or Null` fields from API 1.0.0, with canonical/imported/aliased names:

```monkeyc
var required as Toybox.Lang.Method;
var optional as Toybox.Lang.Method or Null = null;
function initialize(callback as Toybox.Lang.Method) {
    self.required = callback;
}
```

Both slots start as Null in metadata (type `0x00`, value zero). A non-null field
requires assignment in source `initialize`, following the required-field prefix
and self-exposure restrictions of [reference fields](#source-reference-fields).
Binding `self.method(...)` exposes self and cannot fill the first required slot.
Nullable fields may omit their initializer or use literal `null`; computed
defaults remain unsupported. Writes must preserve the declared
Method/nullability contract. Methods retain their erased invocation contracts.

Reads retain the declaration's type. Nullable invocation/member access requires
a guarded local snapshot; testing one mutable field read does not refine
another. Replacing or clearing a field does not alter saved aliases. Each stored
Method retains its bound receiver until all owning aliases are released.
Ordinary instance isolation, field name/count limits, and evaluation order still
apply.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[specimen](../../../examples/compiler-methods/README.md) and
[ownership evidence](../../investigations/compiler-methods.md#nullable-ownership).

## WeakReference fields

All supported source class bases accept annotated `Toybox.Lang.WeakReference`
and `WeakReference or Null` fields from API 1.2.0, including unused
declarations. Canonical, imported, and aliased type names are accepted. Both
slots start as Null metadata (type `0x00`, value zero). Non-null fields require
assignment in source `initialize`, with the existing required-field prefix and
self-exposure rules. Nullable fields may omit initialization or use literal
`null`; computed defaults are unsupported.

Reads and writes preserve declared nullability. Nullable field members need a
guarded local snapshot; a test of one mutable field read cannot refine another.
Stored weak references and their aliases do not retain the target. A saved
`get()` Object result is a separate strong reference, governed by the
[ordinary weak-reference contract](lang-values.md#ordinary-weak-references).
Immutable scalar `weak()` results cannot satisfy an ordinary WeakReference
field.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[specimen](../../../examples/compiler-weak-references/README.md).

## ByteArray fields

All supported source class bases accept `Toybox.Lang.ByteArray` and `ByteArray
or Null` fields from API 3.0.0, including unused declarations. Canonical names,
imports, and aliases work. Both slots start as Null metadata (type `0x00`, value
zero). Non-null fields require assignment in source `initialize` under the
required-field prefix and self-exposure rules. Nullable fields may omit a
default or use literal `null`; computed defaults are rejected.

Reads/writes preserve the declared reference and nullability. Ordinary Arrays
cannot satisfy ByteArray fields. Methods and indexed updates accept non-null
field reads; nullable fields need a guarded local snapshot. Testing one field
read does not refine another mutable read. A snapshot retains its object after
the field is replaced or cleared. Fields hold strong references; clearing the
last strong reference allows an existing weak target to expire.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[example](../../../examples/compiler-byte-arrays/README.md) and
[evidence](../../investigations/byte-arrays.md#source-and-ir-profile).

## Symbol fields

All supported source class bases accept Symbol fields from API 1.0.0:

```monkeyc
var required as Toybox.Lang.Symbol;
var maybe as Toybox.Lang.Symbol or Null;
var optional as Toybox.Lang.Symbol or Null = null;
var preset as Toybox.Lang.Symbol = :alpha;
var inferred = :size;
```

Annotations accept canonical, imported, and aliased Symbol names. Literal Symbol
defaults are permitted with either nullability; an omitted type infers non-null
Symbol. Nullable fields default to Null when no Symbol is supplied. Non-null
fields without defaults must be assigned in source `initialize`, using the same
straight-line prefix and exposure/callback restrictions as required reference
fields. A literal default already satisfies initialization. Null requires an
explicit nullable declaration; other/computed defaults remain unsupported.

Reads retain the declared Symbol/nullability type, including when a nullable
field has a Symbol default. Writes must satisfy that contract. Nullable reads
need guarded local snapshots for members and selectors; a test of one mutable
field read does not refine another read. Replacing the field does not change a
saved local value. Field names, count limits, assignment order, and instance
isolation follow the existing class rules.

Metadata defaults use type `0x08` with a Symbol ID; other slots use Null type
`0x00` and value zero. Application generation resolves default names in the
shared Symbol map and retains development names even when no CODE literal
mentions them. Native names reuse pinned IDs. AppBase and constructed classes
share this encoding. Reads/writes use existing field instructions.

Saved representation uses the [IR field
grammar](../ir/grammar.md#unit-and-class-framing); unit validation rechecks this
declaration and every field operation. See the
[specimen](../../../examples/compiler-symbols/README.md) and
[evidence](../../investigations/compiler-symbols.md#symbol-fields).
