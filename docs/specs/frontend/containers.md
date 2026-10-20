# Core Lang Arrays

## Char arrays

Nonempty Char-array literals accept Char-valued expressions, for example `['a',
'b']`, `[65.toChar(), 'z']`, and `[Chars.make(), 'x']`. They produce a fresh
non-null `Array<Char>` and may receive postfix reads/calls. Number literals
follow the [Number-array contract](#number-arrays); View literals follow the
[native object contract](../sdk/profiles.md#native-views); String literals follow the
[String-array contract](#string-arrays). Other element types, mixed
types, nested arrays, and trailing commas are rejected. Empty literals follow
the [contextual construction rules](#empty-array-literals).

Homogeneous nonempty lists of Number, Char, or String tokens become typed IR
constants and DATA array records loaded with `apush`. Each execution creates a
fresh mutable array. Computed expressions are not folded into these records.

Other nonempty construction allocates the final length before evaluating
elements left to right, once each, and stores each result with VM array
instructions. A private builder prevents unfilled slots from escaping; branching
elements use a reusable staging local. Builder and ordinary slots share the
normal frame budget. Operation/nesting limits still apply. Allocation and store
failures remain runtime obligations. See
[literal evidence](../../investigations/compiler-array-literals.md).

String `toCharArray()` returns a non-null `Array<Char>` since API 1.3.0. The
supported source annotation is `Toybox.Lang.Array<Toybox.Lang.Char>`, with
ordinary token whitespace permitted. Array parameters and results retain the
element contract; source array results require an explicit annotation. Locals
may store, copy, and reassign matching Char-array values, and user calls may
pass/return them. [Number elements](#number-arrays) have a separate supported
profile. Raw Array annotations, other element types, and nullable arrays are
unsupported. Element contracts are invariant; Char arrays are not widened to
mutable arrays of Object.

Char-array receivers support `size()` returning Number, `toString()` returning
String, `indexOf(Char)` returning Number, `add(Char)` returning `Array<Char>`,
`addAll(Array<Char>)` returning `Array<Char>`, `remove(Char)` and
`removeAll(Char)` returning Boolean, `reverse()` returning `Array<Char>`, and
`slice(Number or Null, Number or Null)` returning `Array<Char>`. Search returns
the first matching index or `-1` when absent. Generic API parameters and results
substitute the receiver's element type; executable array values remain limited
to Char elements. Array methods dispatch through the Array class. Explicit
context checks both the Array class and Char element, requiring API 1.3.0 for
this profile.

`add` appends one element; `addAll` appends the argument's elements. Both mutate
the receiver and return that same array. `remove` deletes the lowest-index
match, while `removeAll` deletes every match. Each returns true if it removed an
element, otherwise false. Remaining elements retain their relative order, and
aliases observe the mutation. Calls evaluate receiver before arguments, once
each; native mutation uses the existing ordered-call validation and lowering.
See [mutation evidence](../../investigations/compiler-array-mutation.md).

`reverse` returns the elements in reverse order and leaves the receiver's
contents unchanged. `slice` uses a start-inclusive, end-exclusive range. Null
start/end select the beginning/end; negative bounds offset from the end, and
out-of-range bounds are clipped. A start after the end returns an empty array.

Convert arrays to String before printing. Arrays cannot feed arithmetic,
concatenation, comparison operators, Boolean conditions, or console entry
results. Native Array constructors and ordinary weak targets outside the bounded
profile remain unsupported. Default sorting follows its
[separate contract](#default-array-sorting). Native Array
calls may be values or statements that discard the result. Guard nullable String
receivers before conversion; that proof does not imply support for nullable
Array results. See the
[Char-array evidence](../../investigations/compiler-char-arrays.md).

`array[index]` reads a Char from a non-null Char array. The index must be a
non-null Number; a fresh nullable Number read under an existing null proof is
accepted. Indexing and member calls share the postfix level and may interleave:
`array[0].toUpper()` and `text.toCharArray()[0]` are supported. Array evaluates
before index, once each. Logical subexpressions use the ordinary temporary
snapshot path to preserve this order across joins. The combined postfix/index
nesting limit is 64, and diagnostics retain the complete indexed-expression
span.

The compiler emits `agetv`, with no clamping, negative-index adjustment, or
static bounds analysis. An index outside `0..array.size()` remains a runtime
obligation; the tested VM reports a fatal bounds error. Reads are
order-sensitive in IR validation even without other calls/writes, because
failure order is observable. Indexing a String itself and nullable Array
receivers remain unsupported; Dictionary indexing follows its [own
contract](lang-dictionary.md#dictionary-values). See the
[array-read evidence](../../investigations/compiler-array-reads.md).

## Number arrays

String `toUtf8Array()` returns a non-null `Array<Number>` since API 1.3.0. Its
initial elements are UTF-8 bytes. The Array element contract is Number, so later
appends need not stay within byte range. Source parameters/results use
`Toybox.Lang.Array<Toybox.Lang.Number>`; non-Number results require explicit
source annotations as usual.

Number arrays use the same storage, native-method, and evaluation-order rules as
Char arrays. `add`, `indexOf`, `remove`, and `removeAll` require Number;
`addAll` requires `Array<Number>`. `reverse` and `slice` retain `Array<Number>`.
Indexing produces Number, permitting ordinary arithmetic and native Number
calls. Number and Char arrays are invariant and cannot substitute for each
other; no implicit numeric/Char conversion is performed.

The Array/Number classes require API 1.0.0; conversion and the mutation/search/
transform methods above require 1.3.0. Context checking keeps signature-type
availability separate from method availability. Nullable String receivers need
proof before conversion. Nullable arrays and unsupported element types remain
rejected.

Nonempty literals such as `[1, 2]`, `[n, -n]`, and `[M.make(), 3]` produce a
fresh `Array<Number>`. The first element selects the supported element type from
its expression contract; every element must have that same non-null type.
Parameters, locals, user/native calls, arithmetic, and element reads can supply
the first element. Nullable values require the usual branch proof, including
when used first. Allocation precedes element effects; evaluation and typed
stores proceed left to right as for Char literals. Construction requires
Array/Number availability (1.0.0), without an incidental native `add`
requirement. Element calls and other Array methods retain their own API
constraints. Empty construction uses [contextual types](#empty-array-literals).
See the
[literal evidence](../../investigations/compiler-number-array-literals.md).

Arrays must be converted to String before printing, and cannot be console entry
results. See the
[Number-array evidence](../../investigations/compiler-number-arrays.md).

## Numeric arrays

`Array<Long>`, `Array<Float>`, and `Array<Double>` use fully qualified scalar
names in source and are invariant. Parameters/results, locals, reads/writes,
empty construction, and the supported generic Array methods preserve the exact
element contract. No Number/Long/Float/Double substitution or implicit widening
is permitted at array boundaries. Indexes remain non-null Number.

Each element also supports `or Null` under the shared nullable-element rules.
Guard a saved nullable read before scalar use; guarding one read does not narrow
a later read of the same array. Non-null arrays reject Null values/arguments.

Literal-only homogeneous arrays use typed DATA constants, preserving signed Long
and floating bit patterns. Nullable constants may contain Null. Computed
literals use ordered private construction and may include branching elements;
expected contracts reach empty and nullable literals. Repeated constant loads
produce independent mutable arrays.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`,
`reverse`, and `slice` use the existing generic Array profile. `addAll` takes an
invariant Array of the same element contract; `removeAll` takes an element.
Native search and removal semantics remain runtime supplied. Allocation,
indexing, `size`, and `toString` require API 1.0.0; the other listed methods
require 1.3.0.

Non-null indexed updates/compounds follow scalar operator promotion and must
retain the element type. For example, `Float += Number` is valid but `Float +=
Long` would produce Double and is rejected. Nullable elements cannot be compound
targets without a supported direct non-null read contract. Non-null elements
support [default sorting](#default-array-sorting) and
[source comparators](#source-comparators). Broader callback consumers, nested
containers, nullable Array references, and whole-array expression joins outside
the existing Number/Char profile remain deferred. See the
[example](../../../examples/compiler-numeric-arrays/README.md) and
[record/runtime evidence](../../investigations/compiler-numeric-arrays.md).

## String arrays

`Toybox.Lang.Array<Toybox.Lang.String>` is an invariant non-null String-element
contract. Parameters/results preserve that type; indexed reads produce String
and writes require String. Nullable source values require a non-null proof
before construction, writes, or native arguments. Mixed types, Null elements,
nested arrays, and nullable Array receivers remain unsupported for this profile.
Nullable String elements have a [separate contract](#nullable-array-elements).

Homogeneous literals such as `["one", "two"]` use pooled DATA constants.
Repeated text shares a local pool entry; repeated execution loads independently
mutable arrays. Computed literals, including conditional and guarded elements,
use the same ordered private construction as Char/Number arrays. An expected
String-array contract supplies the type of a direct `[]`.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`,
`reverse`, and `slice` use the existing Array contracts with String substituted
for the element type. Search/removal compare String text. Array/String types,
construction, indexing, `size`, and `toString` require API 1.0.0; the other
methods require 1.3.0. [Default sorting](#default-array-sorting) requires 5.0.0.
Source comparators follow their [separate contract](#source-comparators);
broader callback consumers remain deferred. Arrays must be converted to String
before printing.

See the [example](../../../examples/compiler-string-arrays/README.md) and
[runtime evidence](../../investigations/compiler-string-arrays.md).

## Boolean arrays

`Toybox.Lang.Array<Toybox.Lang.Boolean>` is an invariant non-null
Boolean-element contract. Literals infer Boolean; reads produce Boolean for
conditions, logical operators, and native Boolean methods. Writes and
element-taking Array methods require Boolean, with no Number conversion. Null
elements remain unsupported. Typed parameters/results and direct contextual `[]`
preserve the contract.

Literal-only arrays use `apush` constants; computed elements use ordered private
construction, including conditional/short-circuit expressions. Copies share
mutation; repeated constant loads are independently mutable. The supported Array
methods and API levels match the String/Number profile, including
[default sorting](#default-array-sorting) from 5.0.0. Source comparators follow their
[separate contract](#source-comparators); broader callback consumers remain deferred. See the
[example](../../../examples/compiler-boolean-arrays/README.md) and
[evidence](../../investigations/compiler-boolean-arrays.md).

## Default Array sorting

`array.sort(null)` sorts a non-null Array of Number, Long, Float, Double,
Boolean, Char, or String elements in ascending order. It mutates the receiver,
so aliases observe the new order; separately created copies retain their own
contents. Empty/singleton Arrays are accepted, and duplicates are retained. The
invariant element contract does not change. Sorting requires API 5.0.0.

The result is Void: use a call statement, not an initializer, return expression,
or chained value. Evaluate the receiver once before the Null argument, then
invoke the runtime method and discard its VM result slot. The compiler does not
implement a sorting algorithm or promise sort stability, NaN ordering,
locale-sensitive String collation, or atomic behavior if comparison fails.

Garmin's full parameter type is `Lang.Comparator or Null`. Comparator is a
structural interface requiring `compare(Object, Object) as Number`; it is not
Method or an instantiable native class. Null selects default sorting; supported
source comparator instances follow the contract below. Non-null
[Comparable elements](interfaces.md#comparison-interface-arrays) also support default sorting.
Non-null Object elements also support default sorting. Their declared type does
not prove that the runtime contents are mutually comparable: String/Number
comparisons, for example, raise the native UnexpectedTypeException. Preserve
those runtime failures instead of inventing a narrower static element contract.
Nullable elements remain unsupported, even if current contents are non-null; an
indexed write does not prove whole-Array initialization or comparability.

See the [example](../../../examples/compiler-array-sort/README.md) and
[evidence](../../investigations/compiler-array-sort.md).

## Source comparators

`array.sort(comparator)` accepts an instance of a declared source class that
supplies `compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as
Toybox.Lang.Number`. Both parameter types, arity, and result must exactly match
the interface declaration. Narrower scalar parameters, nullable Object
parameters/results, and missing/extra parameters are rejected. Untyped source
parameters still infer Number and therefore do not establish this interface.
Matching is structural: no Comparator superclass or `implements` syntax is used.

Supported receivers are Arrays with any supported non-null element contract,
from API 5.0.0. This includes Object, Symbol, Method, WeakReference, ByteArray,
View, and [comparison-interface
elements](interfaces.md#comparison-interface-arrays). For elements without a
supported default-order profile (Symbol, Method, WeakReference, ByteArray, View,
or Comparator), the comparator must be provably non-null. A nullable Comparator
could select an unsupported default order. Scalar, Numeric-alias, Comparable,
and Object elements also accept Null or Comparator-or-Null arguments under the
default-order rules above. Nullable element contracts remain excluded: the
checked callback takes two non-null Objects. Container invariance and exact
element identities are unchanged. A negative/zero/positive Number orders the
first element before, equal to, or after the second. The runtime calls the
source `compare` body on the supplied instance, preserving its fields. It
controls comparison count and order; callers must supply a consistent ordering
and must not depend on a particular sorting algorithm. Custom comparators may
choose an order different from the default, including ordering Object string
conversions.

Construction, aliases, exact source-class parameters/results, and source
reference fields can supply a comparator while retaining class identity.
Whole-unit validation resolves the class and rechecks the required method; an
erased View/Object or Method cannot establish the structural contract. The
[comparison-interface profile](interfaces.md#interfaces) also accepts Comparator
parameters/results and locals, including Comparator-or-Null values. Bare `self`
as a comparator remains unsupported. Object-rooted source classes support
construction, exact source signatures/reference fields, and comparator use
through their [class contract](classes.md#object-rooted-classes). The existing
source class/base and constructor requirements still apply.

Sort uses ordinary native-call lowering with receiver-before-argument order. The
runtime callback member `compare` uses its pinned symbol ID; it remains a source
method body. Constructor checks propagate known comparator shutdown requests
through sort and source wrappers, rejecting them while reference fields are
pending. Default sorting of opaque Object elements conservatively permits erased
callback effects; reject it before required fields are initialized. A pinned
source comparator retains its known-body effect analysis. These checks do not
prove callback termination, exception safety, or comparator consistency.

See the [example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md).
Object/reference-element ordering has a separate
[example](../../../examples/compiler-object-sort/README.md) and
[evidence record](../../investigations/compiler-object-sort.md).

## Nullable Array elements

`Toybox.Lang.Array<T or Null>` supports numeric, String, Boolean, Char, or
Symbol T with invariant element contracts and a non-null Array reference.
Indexed reads produce the base type or Null. Stores and native `add`, `indexOf`,
`remove`, and `removeAll` accept either alternative; `addAll` requires the same
Array contract. `reverse` and `slice` retain it. API levels match the
base-element Array profile.

Direct literals acquire a nullable element contract from an annotated return, a
typed user/native argument, or assignment to an existing nullable-element Array
local. This includes empty, Null-only, scalar-only, and computed literals. For
example, a function returning `Array<Number or Null>` may return `[1, null]`.
Literal tokens use typed DATA constants; other expressions allocate before
ordered fills. Branching nullable literals use a separate staging local per
element so differing operand types do not change an existing local's contract.

Existing Array references cannot widen or narrow their element contracts.
Unannotated literals retain the non-null inference rules; `[null]` and `[1,
null]` without nullable context remain rejected. Nested arrays and nullable
Array receivers remain deferred.

Store a nullable read in a local and guard a fresh local read before scalar
operators, methods, or printing. A present false Boolean is distinct from Null.
Writes and `a[index] != null` do not prove that a later indexed read is
non-null. A scalar saved before Array mutation retains its earlier value.
Reassigning that scalar invalidates its null proof. See the
[example](../../../examples/compiler-nullable-arrays/README.md) and
[evidence](../../investigations/compiler-nullable-arrays.md).

## Object containers

`Array<Lang.Object>` and `Array<Lang.Object or Null>` retain non-null Array
references with invariant erased element contracts. Dictionary values likewise
accept `Lang.Object` or `Lang.Object or Null` with any supported key contract;
Object keys remain unsupported. Canonical names, imports, and Lang aliases work
in signatures and contextual construction. General source unions, explicit
nested parameters, and exact source-class element/value contracts remain
separate requirements.

Object slots accept supported scalar classes, numeric aliases, source/native
objects, comparison interfaces, and container/Method/WeakReference references.
Known enum values may widen to Object with their Number runtime representation;
this does not make them assignable to Number or permit numeric operators on
nominal enum types. Nullable Object slots additionally accept Null. Mutable
containers are invariant: existing Array/Dictionary references cannot widen
their parameters to Object, even though individual values can be stored there.

Explicit context admits heterogeneous literal contents. Without context, a value
already represented as Object can establish that erased contract; mixed scalar
literals alone do not infer it. Scalar constants use tagged DATA records,
retaining class, exact bits, order, and fresh mutable results. Symbol/reference
or computed contents use the existing ordered private builders. Check API
availability for each constant payload class as well as the erased container
type; storing Char under Object still requires API 1.3.0.

Indexing and native methods preserve erased types. Non-null Object reads support
Object observers; specific members need the existing lexical assertion or class
proof. Erasure does not recover nested container parameters. Dictionary lookup
adds absent-key Null, while `values()` retains the stored value contract.
Nullable reads require a guarded local snapshot; testing a mutable container
read does not refine another read. Sized allocation requires nullable elements.
Non-null Object arrays follow the [sorting contracts](#default-array-sorting);
nullable elements require further ordering/callback contracts.

Containers and their shallow enumerations retain strong references to stored
objects. Removal/replacement releases that ownership; other aliases or snapshots
may still keep a weak target alive. A source object retrieved under Object loses
its exact source identity until a supported runtime class proof reestablishes
it. See the [example](../../../examples/compiler-object-containers/README.md)
and
[evidence](../../investigations/compiler-object-containers.md).

## Empty array literals

A direct `[]` requires a concrete expected `Array<Number>`, `Array<Char>`,
`Array<String>`, `Array<Boolean>`, `Array<Symbol>`, `Array<View>`, or a
supported
[nullable element contract](#nullable-array-elements).
Parentheses preserve direct context. Supported contexts are:

- An explicitly annotated array return: `return [];`.
- A typed user-call argument, including module, cross-file, and self calls.
- Assignment to an initialized array local: `var a = [1]; a = [];`.
- A native argument whose resolved contract is an array, such as `a.addAll([])`.

The literal allocates a fresh empty array at its position in evaluation order.
Reassignment replaces the local reference; existing aliases retain the old
array. No `add` call or element-storage temporary is emitted. Empty
Number/String/View allocation requires API 1.0.0; Char element availability
requires 1.3.0. Enclosing calls retain their own API requirements.

Context applies only to the direct empty literal, not through arbitrary
operators or calls. `var a = []`, `[].size()`, and `return [].reverse()` are
rejected, including the last case under an array return annotation. Local type
annotations, nested arrays, and inference from later mutations are unsupported.
Nonempty literals keep their independently inferred contract; argument/return
context cannot coerce mixed elements or substitute Number and Char arrays. Saved
IR records the concrete allocation type and rechecks all uses. See the
[complete example](../../../examples/compiler-empty-arrays/README.md) and [evidence](../../investigations/compiler-empty-arrays.md).

## Sized array allocation

`new[length]` creates a fresh non-null Array with Null in every slot. It needs a
direct expected Array contract with supported nullable elements, supplied by a
typed return/argument or an established local. Existing supported conditional
result contracts pass that context into their arms. For example:

```monkeyc
function allocate(n as Toybox.Lang.Number)
    as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
    return new[n];
}
```

Evaluate the non-null Number length once before allocation. Conditional lengths
retain their branch effects. No clamping, element conversion, or initialization
loop is inserted; invalid sizes and allocation failures remain runtime concerns.
Reads preserve the nullable element contract after writes. Reassignment replaces
the reference while aliases retain the previous array.

An untyped `var a = new[n]`, non-null element contracts (even `new[0]`), nested
arrays, and inference from later writes remain unsupported. Conditional arms can
select sized arrays when their invariant element contracts match. Type
availability is checked independently of Array methods: nullable Number
allocation requires API 1.0.0, while nullable Char requires 1.3.0. See
[IR allocation](../ir/containers.md#array-allocation) and
[runtime evidence](../../investigations/compiler-empty-arrays.md#sized-allocation).
