# IR calls and objects

## Native object construction

`.new-native` allocates and initializes a non-null View with no arguments. Typed
IR uses `NativeNew { result, class: NativeClass::View }`.

`Toybox.WatchUi.BehaviorDelegate` is also accepted, using the corresponding
NativeClass, `behavior-delegate` parameter/result tag, and initializer API
1.0.0. WeakReference construction follows its separate contract below; other
native class paths and extra arguments are rejected. View parameter text is
`view`; user value calls use `result=view`. Function results are inferred from
returns; `.function result=view` is unsupported.

Construction requires API 2.1.0 for View.initialize, while passing an existing
View needs only the class's 1.0.0 availability. It participates in single-use,
same-block, and effect-order checking as an allocation/call. Discarding its
result still executes allocation and initialization.

[VM lowering](../lowering/calls.md#native-object-construction).

`.new-native` uses `WeakReferenceNew { result }` and produces a non-null
WeakReference. It takes no operands, retains allocation effect order, and checks
class availability from API 1.2.0. Lowering resolves `__ciq_sdk_Lang` /
`__ciq_type_WeakReference`, then uses the existing `newc`, `initialize` lookup,
and zero-argument invocation sequence. It does not invoke `weak` or introduce a
new VM opcode/DATA record. The [source
contract](../frontend/lang-values.md#weakreference-construction) scopes the
SDK-observed constructor separately from target binding.

## Positional result construction

`.tuple-new` constructs a fresh positional result. One slot has
`TupleShape::OneView`; two have `TupleShape::ViewDelegate`, with View then
InputDelegate API contracts. The supported second-slot value is a non-null
BehaviorDelegate. Wrong slot count, ordering, types, or unsupported fields are
rejected. Typed IR uses `TupleNew { result, elements }` with the same checks.

Every operand must be a local/parameter read evaluated earlier. Store effectful
slot expressions left to right before constructing the tuple. Single-use,
same-block, initialization, and effect-order rules remain in force. Tuple shapes
are distinct from homogeneous Arrays and have no Array mutation/call profile.
Locals may retain/copy a shape. Function results are inferred from tuple
returns; explicit tuple function headers are unsupported. User-call metadata may
use `result=one-view` or `result=view-delegate`; unit validation checks the
actual callee result and rejects ordinary calls to native callbacks.

[VM lowering](../lowering/calls.md#positional-result-construction).

## Module calls and operation order

`.call` defines the result of a positional-argument call to a top-level user
module. Scalar result types are `integer`, `long`, `float`, `double`, `string`,
`boolean`, and `char`, plus `nullable-integer`, `nullable-long`,
`nullable-float`, `nullable-double`, `nullable-string`, `nullable-boolean`, and
`nullable-char`. Symbol calls use `symbol` or `nullable-symbol`. Method calls
use `method` or `nullable-method`. Array results use the [element
categories](containers.md#array-allocation). Omission means integer. Canonical
text omits the integer default and writes noninteger contracts. Void is invalid
in a value call. Names use ASCII identifier rules and remain symbolic. The
optional span has the same meaning as for other operations. The module's
expected stored symbol name is `globals/<module>` and the member's is
`<member>`; this is not numeric symbol allocation or method metadata generation.

Calls, prints, and writes are effectful. If a function contains any `.call`,
`.println`, `.call-native`, or `.assign`, all its operations must match the
backend's left-to-right expression traversal and ordered
initialization/assignment/print/branch/return sinks. The shared validator
rejects incompatible schedules, including moving a call across an initialization
or reversing call operands. Pure-only functions permit flexible definition
order. This conservative policy keeps runtime/IR operation order consistent
until a general scheduler is implemented.

```text
ciq-ir
.function probe
.call %0 Numbers left
.local $0 %0
.read %2 $0
.call %3 Numbers right
.add %4 %2 %3
.integer %5 39
.subtract %6 %4 %5
.return %6
.end
```

Names and result expectations are not checked against a whole-program type
environment in standalone mode: the scaffold must supply matching callees. Unit
mode checks names, exact arity, argument types, and call result contracts.
Source-instance calls follow the [receiver profile](#instance-receivers). Global
unqualified calls remain unsupported.

## Void returns and statement calls

Void returns and effect-only calls use:

- `.return-void`: terminate without a semantic result.
- `.call-discard`:
  invoke a qualified user function for effects without defining a value.

`.call-discard` may target any supported user result, including Void; the caller
discards it. Argument IDs name earlier supported scalar, nullable, or Null
values in the same block and each counts as a use. Calls use identifier rules,
argument limits, and left-to-right effect scheduling already required by
`.call`. Names, arity, and callee result contracts are checked in unit mode;
standalone functions retain their external-callee obligations.

```text
ciq-ir
.function startup
.integer %0 41
.call-discard Startup greet %0
.return-void
.end
```

Every return in a function must agree: `.return` yields its value's supported
type, `.return-void` yields void. The shared validator infers and exposes that
result contract. Nullable result declarations instead require assignability to
that union on every return. A block may terminate with either form. The final
declared block still returns, and the existing reachability, definite-local, and
empty-edge-stack rules apply. A single `.return-void` is a valid function body.
Implicit text fallthrough is rejected; only source compilation inserts a final
void return.

Void is not an expression value, and arbitrary SDK calls remain unsupported.
Object, nullable, and container values follow their separately checked profiles.
See [source rules](../frontend/statements.md#void-functions-and-statement-calls)
and
[runtime evidence](../../investigations/compiler-void.md).

## Native superclass initialization

`.base-initialize` accepts `Application.AppBase`, `WatchUi.View`, or
`WatchUi.BehaviorDelegate` and invokes the identified native superclass
initializer on the current instance, with no explicit arguments or value result.
It accepts the usual optional source span. Other base names or additional fields
are rejected. Function validation requires an instance receiver and preserves
the call's effect order; unit validation additionally enforces the
[source initializer contract](../frontend/classes.md#source-initialization).
Source, typed, and text paths use the same operation and VM lowering.

## Comparison interface contracts

`comparable` and `comparator` are non-null parameter/result tags for the
[source interface profile](../frontend/interfaces.md#interfaces).
`nullable-comparable` and `nullable-comparator` add Null to those contracts.
They retain API 5.0.0 availability and require non-null branch proofs for member
calls. Nullable result contracts must be explicit, as for other nullable types.
They occur in `parameter-types=`, function `result=`, user-call `result=`, and
interface block-parameter tags. Calls use `.call-native` with `compareTo` or
`compare`; Comparator values also supply the ordinary native `sort` call. No
interface class allocation, cast, or new VM instruction is implied.

```text
.function forward parameters=1 parameter-types=comparable result=comparable
.parameter %0 0
.return %0
```

Function validation checks interface call arity/types and retained value
contracts, including scalar Comparable witnesses at returns, local assignments,
and incoming block values. User-call arguments use the same witness rules. No
coercion operation is serialized; the declared storage/edge/call contract
retains the interface while the scalar keeps its runtime representation. Source
witnesses at arguments, returns, local/field assignments, Array fills, indexed
writes, and native element arguments remain unit obligations; unit validation
resolves their exact classes and checks required members after text reload. A
forged call result or a changed implementation signature cannot bypass that
check. Erased calls retain conservative constructor effects. The backend selects
the existing comparison symbols and invocation instructions. Class declarations
retain interface fields through
[`.field-interface`](../frontend/fields.md#comparison-interface-fields). Reads
must claim the declared result tag; writes require unit-level compatibility. No
new VM opcode or DATA value tag is introduced for interface storage. Ordinary
Null values, branches, and frame slots retain their existing encodings. Sorting
accepts a nullable Comparator argument without a receiver refinement; compare
member calls still require one.

## Nullable callback state

Instance-function metadata may include `parameter-types=...` with
colon-separated scalar/Array contracts. Array categories follow [Array
allocation](containers.md#array-allocation). Ordinary signatures also accept
`long`, `nullable-long`, `float`, `double`, `nullable-float`, `nullable-double`,
`nullable-integer`, `nullable-string`, `nullable-boolean`, and `nullable-char`.
Symbol signatures use `symbol` or `nullable-symbol`. Method signatures use
`method` or `nullable-method`. Separate entries with colons; the list must match
`parameters=<count>`. Omitted types are integers. This field applies to ordinary
functions and instance methods. `nullable-dictionary` is also accepted; unit
validation restricts it to native AppBase.onStart/onStop contracts. Unsupported
names, duplicate lists, and mismatched counts are errors. For example:

```text
.function onStart parameters=1 receiver=instance parameter-types=nullable-dictionary
```

`.null` defines null. `.equal`/`.not-equal` permit supported scalar/reference
equality domains and nullable/null checks; it also accepts the [String identity
profile](../frontend/operators.md#operator-contracts). Ordering directives
follow the [numeric comparison domains](../frontend/numeric-values.md) and [Char
restrictions](../frontend/operators.md#operator-contracts). Callback-state
reads/locals retain their declared nullable type; proven reads yield a non-null
Dictionary. Printing and arithmetic reject Null and Dictionary values. Null may
be used in nullable scalar user contracts. Non-null Dictionary references
support the [allocation/call profile](containers.md#ir-container-operations). A
nullable callback Dictionary local may be assigned null; other assignments
preserve the initializer type. Lowering uses `npush` for null and the existing
local-slot and equality instructions.

## Instance receivers

The optional declaration field `receiver=instance` marks an instance method;
ordinary bodies omit it. It may appear before or after `parameters=<count>`;
duplicate or unknown fields are rejected. Every instance method requires
receiver metadata, even without self calls. The receiver is the hidden context,
not an explicit parameter/value ID.

`.self` defines the current receiver as opaque Object. It requires
`receiver=instance` and lowers to `getself`. It does not claim an exact source
class. Reflective `self.method(selector)` uses this value with ordinary
`.call-native`; its Method result remains erased.

Two call directives also require an instance method:

- `.call-self`:
  define a value under the same result contract rules as `.call`.
- `.call-self-discard`:
  invoke a current-receiver member for effects and discard its result.

Arguments are ordered, earlier-defined supported scalar, nullable, or Null
values in the same block. The existing use/effect/control-flow checks apply.
Unit validation binds calls only to declared methods in the enclosing class,
checks arity/result contracts, and rejects instance bodies in global/module
owners. Standalone method IR retains its required receiver and unbound
class/callee obligations.

```text
ciq-ir
.function start receiver=instance
.integer %0 50
.integer %1 3
.call-self %2 answer %0 %1
.println %2
.return-void
.end
```

Canonical method output keeps `receiver=instance`. Native base-class identity
and enclosing class name belong to [unit class
containers](unit.md#ir-compilation-units-and-classes), not function IR; neither
carries a VM import index. General receiver values, object construction, fields,
native self calls, and source inheritance are unsupported.

## Native module values

`.call-native-module` invokes a runtime-provided module function and defines its
catalogue-declared result. Module names are canonical API paths; optional spans
use the ordinary syntax. The executable profile accepts `Toybox.Lang format`:
String and erased Array arguments, String result, API 1.0.0. Validation checks
arity, assignability, availability, supported lowering, and ordered single-use
operands. Neither the consumer nor saved text supplies a result-type override.
Void members require the [statement form](#native-module-statements); ignoring a
value uses `.discard`.

[VM lowering](../lowering/calls.md#native-module-values).

## Native module statements

`.call-native-module-discard` invokes a runtime-provided module function without
defining a value. Module names are canonical API paths, distinct from user
module owners; arguments follow the ordinary definition/use and effect-order
rules. Optional spans use the usual syntax. The profile accepts `Toybox.System
exit` and `Toybox.WatchUi requestUpdate`, each with no arguments and a catalogue
Void result. Both require API 1.0.0. API context checks match source
compilation.

[VM lowering](../lowering/calls.md#native-module-statements).

## Discarded values and native Void calls

`.discard` evaluates and consumes one earlier non-Void value without defining a
result or changing the function return type. It accepts every supported value
category, including Null, nullable values, and Char arrays. Definition/use,
same-block, and evaluation-order rules apply. The backend evaluates the operand
through the usual expression path and emits one `popv`; nested call results
remain available to their consumers.

Native source statements use an ordinary `.call-native` followed by `.discard`:

```text
ciq-ir
.function f parameters=1
.parameter %0 0
.call-native %1 %0 abs
.discard %1
.return-void
.end
```

The call retains its argument/result and API checks. A dropped nullable result
does not prove its receiver non-null. User Void calls and native module Void
statements keep their existing forms; no Void expression value is created. See
[runtime evidence](../../investigations/compiler-native-statements.md).

For a Void native member, use `.call-native-discard`. It defines no value.
Validation requires the resolved method result to be Void; receiver/argument
types, API availability, and ordered evaluation still apply. The bounded Dc
clear/setColor/drawText profile uses this form.

## Source construction and native constants

Source instance calls use:

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Optional spans follow the existing syntax; omitted result types mean Integer.
The receiver's exact source class must follow construction, source signatures,
local operations, a positive source-class test, or an explicit source assertion,
independently of the class written in the call. Unit validation checks the
selected class method's arity, parameter/result types, and callback
restrictions. Both forms preserve receiver-before-argument evaluation order.
Statement calls define no semantic result. Standalone function parsing retains
declaration-resolution obligations; it does not authorize arbitrary receiver
casts. See the [source-call
contract](../frontend/classes.md#calls-on-constructed-source-objects).

`source(ClassName@Native.Base)` is an exact non-null source-class contract in
`parameter-types=...` and function/call `result=...` fields. It occupies one
parameter-list entry without a colon inside the tag. Source function results
always serialize this tag; omission erases the exact class at the return
boundary. Class names must be valid identifiers; native bases are `Lang.Object`,
`WatchUi.View`, or `WatchUi.BehaviorDelegate`. Unit validation checks both
against the actual source declaration. Return provenance and call
arguments/results are checked through the [source signature
rules](../frontend/classes.md#source-class-signatures); the tag does not cast an
erased native reference.

Instance field operations are `.read-self` and `.assign-self`, with optional
source spans. Reads define Number by default;
`result=source(ClassName@Native.Base)` selects a source-reference read. Writes
consume Number or a source reference without a result. Both require an instance
receiver and preserve observable order. Their symbolic field identities resolve
through the enclosing unit class's
[field declarations](../frontend/fields.md#instance-storage).
Reference declarations, result claims, and initializer safety follow the
[reference-field contract](../frontend/fields.md#source-reference-fields).
Symbol fields use [`.field-symbol`](../frontend/fields.md#symbol-fields),
retaining literal default names and explicit Symbol/nullability read contracts.
Standalone method parsing leaves that unit obligation unresolved.

`.new-source` allocates and initializes a source object. Supported bases are
`Lang.Object`, `WatchUi.View`, and `WatchUi.BehaviorDelegate`; for example,
`.new-source %0 HelloView WatchUi.View %1` passes one argument. The base is
required and defines the native value projection: Object, View, or
BehaviorDelegate. Source class identity remains separate from that projection.
Construction is order-sensitive. Function validation checks the identifier,
supported construction profile, and API availability (Object: 1.0.0; native
View.initialize: 2.1.0; BehaviorDelegate.initialize: 1.0.0). Object has no
native base-initialization operation. Unit text records `.class Name
Lang.Object`; its explicit source initializer is Void and contains no native
base call. Unit validation requires the named class's declared base to equal the
operation's base, with a matching explicit initializer whose parameter contracts
match the arguments. A standalone function retains this unresolved declaration
obligation and cannot supply complete application metadata by itself. AppBase
construction is reserved for application startup; it is not accepted by this
operation.

`.constant` defines a FontDefinition enum value. Validation checks the catalogue
identity, type, and API level; lowering emits its numeric value 3. It is not an
Integer operation and cannot be replaced by one when used as a font. No other
constants are accepted.

`dc` identifies a non-null Graphics.Dc reference in parameter/result contracts.
It is not constructible and `dc-array` is rejected. View.onUpdate requires one
Dc parameter and Void; onShow/onHide require zero parameters and Void. Callback
ownership and constructor resolution follow the [class
rules](../frontend/classes.md).

## Native value calls

Comparison and equality calls retain the catalogue's Object argument contracts,
including nullable equality arguments and non-null comparison arguments.
Validation does not replace these with a narrower runtime-safe class list; see
the [shared source
contract](../frontend/lang-values.md#comparison-object-arguments).

Method parameters, call results, and matching conditional-edge values use the
`method` or `nullable-method` tag, retained by explicit function result
contracts. `nullable-method` means Method or Null; reads after a proven null
test use the existing storage refinement rules. No new runtime representation is
introduced. Creation uses ordinary `.call-native`; invocation uses
`.call-native`. No separate callback instruction or target-signature annotation
is inferred. Result validation recomputes Object-or-Null for invocation,
including when all observed results were non-null. Native API declarations carry
a repeated argument type for `invoke`; ordinary fixed-arity methods remain
fixed. The existing 254-argument VM limit applies. See [source Method
contracts](../frontend/lang-values.md#method-values-and-invocation). Class
storage uses [`.field-method`](../frontend/fields.md#method-fields), retaining
the declared Method/nullability contract through field reads and writes.

`.symbol` defines a non-null Symbol by ASCII identifier, without a numeric ID.
Parameter, call-result, and matching conditional-edge contracts use `symbol` or
`nullable-symbol`; explicit function result contracts retain either tag.
Validation applies the
[source Symbol profile](../frontend/lang-values.md#symbol-values), including API checks.

[VM lowering](../lowering/calls.md#native-value-calls).

`.call-native` invokes a runtime-provided method and defines its semantic
result. The receiver and arguments must be earlier values with the usual
single-use/block restrictions. Optional source spans use the ordinary syntax.

Receiver, argument, result, generic-element, and API contracts are the same as
[the supported native methods](../frontend/lang-values.md#native-value-calls). Validation
recomputes native results from catalogue/receiver contracts rather than trusting
frontend metadata. Unknown or unsupported members, receivers, argument counts,
and types are rejected. Runtime formatting, conversion ranges, and case mapping
are not compile-time evaluations.

Scalar and Array values use the same parameter/user-call categories described in
the [type grammar](grammar.md#type-tags), with `result=<category>` for valued
calls. Function result categories are inferred from returns; those categories
are not `.function result=` fields. Each chained call checks the preceding
result's type. Nullable Array receivers and unsupported element types cannot be
introduced by text edits.

Array `sort` uses a native call with one Null, exact source-comparator, or typed
Comparator(-or-Null) argument and a supported scalar/comparison-interface Array
receiver. Comparator elements require a non-null callback; nullable elements
remain unsupported. Its Void result is consumed by `.discard`, or the call uses
`.call-native-discard`. Validation reapplies the [sorting
contract](../frontend/containers.md#source-comparators), including API 5.0.0 and
element restrictions. Source identity is reconstructed from construction, typed
source parameters/calls/fields, and locals; no text claim can turn an erased
View/Object into a Comparator. Standalone function IR retains a class/member
resolution obligation, which unit parsing must discharge against the recorded
`compare` declaration. Source and parsed units share this check. Lowering uses
existing member lookup and `invokem`, followed by `popv`; application DATA binds
`compare` to the runtime callback symbol.

Native `equals` uses the shared
[Object argument profile](../frontend/lang-values.md#object-equality-and-container-observers),
including supported scalar, Array, Dictionary, and nullable/Null arguments. An
Array receiver's `equals` defines Boolean and `hashCode` defines Number; neither
changes its invariant element contract. They use existing native call
directives, including ordinary discard for unused results.

Arithmetic and comparison directives enforce the same
[operator contracts](../frontend/operators.md#operator-contracts) as source, including
mixed Char addition, String identity equality, and nullable restrictions. Native
`weak` remains a call whose exact immutable receiver determines its specialized
result; it is not erased or rewritten as a copy.

Ordinary WeakReference values use `weak-reference` or `nullable-weak-reference`
for parameter, call-result, and conditional-edge tags. Class fields use
[`.field-weak-reference`](../frontend/fields.md#weakreference-fields).
Opaque Object values use `object` and `nullable-object`. Explicit function
`result=` contracts retain these tags, including nullable Object results even if
all returns happen to be non-null. `get` and `stillAlive` use existing
`.call-native` operations, with result types recomputed as Object-or-Null and
Boolean. Availability, erasure, joins, and null proofs follow the [source
reference contracts](../frontend/lang-values.md#ordinary-weak-references).

Native calls are effectful: receiver evaluation precedes arguments and
invocation, and their operation order must agree with dependency traversal at
statement sinks. The backend emits receiver evaluation and `getsv
@__ciq_native_<member>`. A zero-argument call uses `invokemz`; calls with
arguments add `frpush`, evaluate arguments in order, and use `invokem` with
argument count plus one (the receiver). Substring therefore uses `invokem 3`.
Binding requires exactly one stored symbol with the method's stored name in an
explicit source, or the supported complete application's native profile. IR
contains no symbol ID or import index.

```text
ciq-ir
.function show parameters=1
.parameter %0 0
.call-native %1 %0 toString
.println %1
.return-void
.end
```
