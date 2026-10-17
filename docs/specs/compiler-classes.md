# Restricted source classes and receiver calls

This profile adds AppBase-, View-, and BehaviorDelegate-derived classes to
[compilation units][unit].
Existing function parameter and result rules still apply. The
[application builder][app] selects one AppBase class and binds additional Views/delegates.
See [runtime evidence][evidence].

## Source and ownership

```text
class = "class" identifier "extends" native-base
        "{" native-declaration* (number-field | reference-field | function)+ "}"
native-base = recognized Application.AppBase, WatchUi.View, or
              WatchUi.BehaviorDelegate identity
number-field = "var" identifier "=" signed-number-literal ";"
reference-field = "var" identifier "as" source-view-class-name ";"
receiver-call = "self" "." member "(" argument-list? ")"
```

Classes are top-level declarations. The base is a recognized native identity,
resolved through the [native name profile](compiler-subset.md#native-name-declarations).
It accepts canonical/qualified, aliased, and imported AppBase/View/BehaviorDelegate spellings.
Number/reference fields are supported below. Static methods, annotations, source
superclass lookup, and nested/empty classes are unsupported. Classes require at
least one method. Class names must be unique;
classes cannot be reopened. Reject class/module/global-function name collisions.

Reserved class names include language keywords, `self`, `System`, `Application`,
`AppBase`, `globals`, `statics`, `Toybox`, and the bootstrap's pinned native names:
`initialize`, `println`, `View`, `onStart`, `getInitialView`. These restrictions
belong to this constrained profile, not the general Monkey C language.

`self` is reserved and may appear as a call receiver or a declared field receiver.
It cannot be rebound or passed as a value. Bare member calls and class-name
calls remain unsupported. Calls on constructed source objects follow the
profile below. Module calls
from methods retain their existing rules.

A self expression call requires a declared value-returning method in the same
class. A self statement call discards any supported result, including Void.
Arguments follow scalar/nullable assignability and result contracts must equal
the callee's contract;
counts must match exactly. Forward and recursive
references are checked after collecting signatures. Self lookup never falls
back to a global/module member or an inherited native method. Recursion is
accepted but termination is not proven.

## Calls on constructed source objects

`new ClassName(arguments)` establishes an exact constructor class alongside its
native View or BehaviorDelegate value contract. Local initialization and copies
preserve this identity.
A local initialized from a source construction may subsequently receive only a
reference with the same proven constructor class; assignments from another source
class, a generic native View parameter, or an erased return are rejected.
Different instances of the same class are permitted, including conditional and
loop assignments. Local aliases refer to the same runtime instance; reassignment
changes only the local reference.

`receiver.method(arguments)` calls a declared method of the proven source class.
The receiver may be a construction, local/copy, or typed source parameter/result.
Forward
method declarations and multiple source files are supported. Value calls must
match the declared result contract; statement calls discard any supported result,
including Void. Arguments use the existing scalar/nullable/array contracts and
limits. Receiver evaluation precedes argument evaluation, in source order.
Empty Array arguments receive context from the selected source signature.

Unit validation rejects missing methods, wrong argument/result contracts, and
ordinary calls to native lifecycle callbacks, even through a different receiver.
Lookup does not fall back to inherited native or source methods. Source inheritance,
arbitrary receiver field access, and general dynamic
dispatch remain unsupported.

Exact constructor identity is an invariant local refinement, not a new native API
type. Generic View parameters, declared View returns, and Array<View> element
reads expose only their base contract. They cannot authorize source-method calls.
A generic View local does not gain constructor identity merely by assigning a
source instance later. No runtime cast is emitted.

`ObjectCall` and `ObjectCallStatement` record a receiver value, class name, member,
and arguments; value calls also record the result contract. Function validation
reconstructs source identity from construction, source signatures, and local
operations and requires an exact match with the claimed class. Unit validation
resolves the declaration.
This prevents saved IR from forging a source class for a native reference.

Lowering evaluates the receiver, selects its method with `getsv`, prepares the
argument context when needed, invokes it, and discards statement results. Native
and text paths use identical lowering and ordinary member-symbol binding. See
the [object-call example](../../examples/compiler-object-calls/README.md) and
[runtime evidence](../investigations/compiler-object-calls.md).

## Source-class signatures

Ordinary functions and instance methods may annotate parameters and results with
an unqualified source View class name:

```text
function pass(view as HelloView) as HelloView { return view; }
```

The name must resolve to a declared WatchUi.View subclass in the unit, including
forward or cross-file declarations. AppBase references, arbitrary native classes,
nullable source references, Array<ClassName>, aliases, and qualified source-class
names are unsupported. Bare native type names such as Number remain unsupported;
native annotations use their established canonical names.

Source contracts are exact and non-null. Arguments and every value return must
preserve the declared source class through construction, typed parameters,
declared call results, or local copies. A different source View class, Null, or
generic native View cannot satisfy the contract. The existing native View
contract may accept a source instance while erasing its exact class identity.

Module, self, and explicit object calls retain a declared source result, permitting
subsequent source dispatch and same-class local assignment. Parameter reads also
retain their declared class. Native callbacks keep their inherited signatures;
source annotations cannot replace those contracts. No runtime cast is emitted.

`ParameterType::SourceObject(String)` and `ReturnType::SourceObject(String)` carry
the source name. Their native API projection is View; exact assignability is
checked separately. Function validation checks names and return provenance, while
unit validation checks class declarations, call arguments, and result claims.
Standalone typed/function IR retains its class/callee resolution obligations.

Saved parameter tags and function/call result tags use `source(ClassName)`.
Result contracts must be explicit in saved function text to retain the source
identity. See the [signature example](../../examples/compiler-source-signatures/README.md)
and [runtime evidence](../investigations/compiler-source-signatures.md).

## Native and textual IR

`ClassDeclaration` records a name, `ClassBase`, and ordered
method bodies. Base identity does not contain a VM import index. `Function::method`
constructs receiver-bearing IR; ordinary function constructors reject self-call
operations. Unit validation requires instance bodies in classes and ordinary
bodies in global/module owners. An instance method can have no self calls and
still retains its receiver requirement.

`SelfCall` defines a value under its explicit result contract, using the same
`result=<type>` text field as a module call. `SelfCallStatement` consumes arguments
without defining a result. Both share dependency, type, one-use, block-local,
control-flow, and effect-schedule checks with existing calls. The hidden receiver
context is not an explicit parameter or a semantic value; positional parameters
still map after slot zero.

Function [IR][ir] permits `receiver=instance`, `.call-self`, and
`.call-self-discard`. Unit IR permits class containers:

```text
ciq-unit
.class Counter Application.AppBase
ciq-ir
.function start receiver=instance
.call-self-discard announce
.return-void
.end
ciq-ir
.function announce receiver=instance
.integer %0 41
.println %0
.return-void
.end
.end-class
```

`.class <name> <native-base>` uses `Application.AppBase`, `WatchUi.View`, or
`WatchUi.BehaviorDelegate` and contains
complete function documents until `.end-class`. Global/module owner records
remain valid alongside classes. The unit header is `ciq-unit` for both
class-bearing and ordinary units.

CODE layout places global/module bodies first, then class bodies. Each group
retains file/declaration order; methods retain order within their class. Class
labels are `__ciq_fn_c_<class-length>_<class>_<member-length>_<member>`, with
ASCII names and decimal lengths. Internal aliases use this prefix. The total
1..=256-body limit includes all functions and methods. Validation error body
indexes follow this layout order; serialized spans remain provenance only.

## Application and VM binding

Complete PRG generation permits exactly one declared AppBase class plus optional
WatchUi.View/BehaviorDelegate subclasses and requires
`--entry Class.method`. The selected method must have zero parameters, except
source onStart follows the callback contract below. Value results are printed
by generated onStart; void results are discarded. Class symbol `1` identifies
the application class. Plain names
matching the class reuse that identity; other names retain deterministic
allocation. Class methods are sorted DATA fields bound to their CODE labels,
separate from globals/modules even when member names share one symbol ID.

The `getInitialView` override follows the positional contract below. Methods
named `AppBase` remain unsupported.
The source initializer contract below permits `initialize`; the bootstrap
supplies missing AppBase lifecycle methods and a blank View when no source
initial-view callback exists. Constructed source Views require an initializer.
Native imports and empty resources retain the bounded application profile.
General object/native-call operations beyond the contracts below are unsupported.

## Source initialization

A declared class may supply `function initialize(...)`. It must return void and
call the native base initializer exactly once as its first statement. AppBase
initializers take no parameters; View/delegate initializers accept the ordinary
supported parameter contracts and limit. These are subset restrictions, not general
language rules.
Subsequent supported statements may print or call source methods. Initialization
cannot be selected as the generated onStart entry.

AppBase base calls are `AppBase.initialize()`,
`Application.AppBase.initialize()`, fully qualified
`Toybox.Application.AppBase.initialize()`, and equivalent module aliases.
Bare AppBase here identifies the inherited superclass even without an import;
this member-call rule does not make bare AppBase legal in an `extends` clause.
Arguments, value uses, other base members, and base calls outside a class
initializer are rejected. Conditional, late, duplicate, or absent base calls
are also rejected by unit validation.

View initializers use `View.initialize()`, `WatchUi.View.initialize()`, or the
equivalent fully qualified/aliased identity. The called base must match the
class's declared base, including in saved IR.

`NativeBaseInitialize { base }` is an ordered
effect with an implicit current instance and no explicit arguments/result.
It identifies a native superclass, not a source callee or executable import
index. Lowering selects `getselfv AppBase; getsv initialize; invokemz; popv`.
The application builder binds native aliases and directs the initializer's
single DATA method record to source CODE. AppBase retains its default initializer
when source omits one. Constructed source Views require an explicit initializer.
Default lifecycle/View bodies are used only where source does not supply them.

BehaviorDelegate initializers use `BehaviorDelegate.initialize()` or a recognized
qualified/aliased spelling, following the same native-base-first rule.

## Source Views and drawing

`new ClassName(arguments)` constructs a declared WatchUi.View subclass. Arguments
must match its source initialize method's arity and types, including exact source
reference contracts. Argument expressions run in order before the initializer;
contextual empty Array arguments use its declared parameter types. The existing
254-argument/frame limit applies. Native `new WatchUi.View()` still takes no
arguments.
Its initializer must satisfy the native-base contract above. Unit validation
rejects unresolved names, modules, AppBase construction, and missing initializers.
The result has the non-null native View contract; source identity is retained in
construction and source contracts, independently of VM storage. It may supply the slot
in `getInitialView()`. Calls on proven source instances follow the profile above;
field access on another instance remains unsupported.

## Number instance fields

AppBase and View classes may declare up to 256 fields with signed 32-bit decimal
Number literal initializers. Fields have a fixed Number contract. Uninitialized
Number declarations, Number annotations, non-Number literals, and computed initializers are
unsupported. Parentheses around a literal are accepted. These restrictions and
the field-count limit define our subset, not a device memory limit.

Each field name must be a valid, non-reserved identifier, unique within its class.
Reject names shared with source methods or catalogue-known inherited methods,
and `View`, `AppBase`, or `init`, which collide with native-base/constructor
metadata. Different classes may use the same field name.

```text
var count = 7;
function bump() { self.count = self.count + 1; }
```

Methods read a field with `self.<name>` and replace it with
`self.<name> = <expression>;`. The RHS must produce Number and is evaluated
before the write. A local or parameter with the same name remains separate;
bare names keep existing local/parameter lookup rules. Assignment is a statement,
not a value expression. Number fields also support all ten arithmetic/bitwise
compound operators: `self.count += rhs;` reads the old field once before RHS
effects, applies the corresponding Number binary operation, then stores the
result. A RHS that changes the field does not change the captured old value.
Branching RHS expressions stage the old read through ordinary typed locals.
Non-Number results are rejected. Arbitrary receiver properties, static fields,
and source-inherited fields remain unsupported. See
[target evidence](../investigations/compiler-compound-targets.md).

`ClassDeclaration.fields` holds names and FieldKind defaults/contracts. Unit validation
resolves every SelfFieldRead/SelfFieldAssign against its enclosing class; standalone
method IR retains this ownership obligation. Function validation requires an
instance receiver, Number assignments, and expression evaluation order.
A read that must survive a subsequent write/call is stored in a local first.
The same checks apply to typed and saved IR.

Unit text emits `.field-number <name> <i32>` immediately after `.class` and before
method documents. Declaration order is preserved; duplicate, misplaced, invalid,
or excessive field records are rejected. Function text uses
`.read-self %<result> <name>` and `.assign-self <name> %<value>` with optional spans.

Application generation records each field with key bits zero, type `0x01`, and
the initial Number's 32-bit pattern. Fields are sorted with other class members.
The runtime supplies a separate field value for each constructed instance;
constant initialization precedes its source initialize method. Reads lower to
`getselfv`; writes to `getself`, `spush`, RHS instructions, then `putv`.
See the [counter example](../../examples/compiler-fields/README.md) and
[instance isolation evidence](../investigations/compiler-fields.md).

## Source-reference fields

`var target as HelloView;` declares an exact source View reference field.
The class must resolve across the unit, including forward declarations. The
256-field limit and name/conflict rules above cover Number and reference fields
together. Declaration initializers, Null unions, native reference fields, and
other field kinds are unsupported.

Metadata creates a null slot (key bits zero, type `0x00`, value zero). A source
initialize method is required to establish the non-null field contract. It must
assign every reference field before exposing `self` as an Object, binding a
self callback, making a self call, branching, jumping, or returning. A
reference read before that field's assignment is rejected. Assignment must use
the exact declared source class; native View erasure, another class, and Null
cannot satisfy it. Requiring this straight-line initialization prefix is our
bounded checking policy, not Garmin's complete definite-initialization rules.
Calls that may request System.exit, including transitive source calls and
constructors and source comparator callbacks reached through Array sorting,
are also rejected until all reference fields are established.
This conservative call-graph check includes conditional exits; runtime failures
and additional native callback effects are not proven safe by it.

After initialization, methods may read or replace the field with the same exact
source class. Field reads retain identity for ordinary instance calls and source
returns. Access remains limited to `self.<field>`; a getter can forward the
reference to another caller. Reads/writes preserve the existing evaluation-order
rules. These contracts apply to typed and saved IR as well as source.

Unit text uses `.field-reference <name> <class>` before method documents.
Function text reads it with `.read-self %<result> <name> result=source(<class>)`;
the result claim must match the owning field declaration. Writes use the existing
`.assign-self <name> %<value>` form. See the
[retained View example](../../examples/compiler-reference-fields/README.md) and
[constructor/lifecycle evidence](../investigations/compiler-reference-fields.md).

View callbacks inherit these contracts, including when annotations are omitted:

| Callback | Parameters | Result |
|---|---|---|
| onShow | none | Void |
| onUpdate | Toybox.Graphics.Dc | Void |
| onHide | none | Void |

All require API 1.0.0. Their signatures are checked against the enclosing native
base; they cannot be invoked as ordinary source callees. Dc is an opaque native
reference with the [bounded drawing profile](compiler-subset.md#drawing-context).
It cannot be constructed or stored in an Array.

The application builder creates a DATA class for each source View, using native
import index one and binding callbacks to their source CODE. Constructor lowering
looks up the source class through globals, allocates it, invokes `initialize`,
and retains the instance while discarding the initializer result. See the
[visible example](../../examples/compiler-visible-view/README.md) and
[runtime evidence](../investigations/compiler-visible-view.md).

## Source initial View callback

An AppBase class may define `function getInitialView()` with zero parameters.
Every return must be a direct literal with either one non-null View or a View
followed by a non-null BehaviorDelegate. Examples: `return [self.makeView()];`
or `return [view, new CounterDelegate(view)];`. Parentheses are permitted.
All paths must return the same shape: `TupleShape::OneView` or
`TupleShape::ViewDelegate`. The pair's second API slot is InputDelegate;
BehaviorDelegate satisfies it through native inheritance.

The compiler evaluates and stores slot expressions left to right before packing
a fresh positional result. Hidden locals share the normal frame budget.
Ordinary mutable Arrays cannot supply this contract. Wrong cardinality/order,
Null, incompatible slot types/annotations, and ordinary user calls to the
callback are rejected. Tuple source annotations and general tuple-expression
inference remain unsupported.

Saved IR retains the shape through [positional
construction](compiler-ir-text.md#positional-result-construction). Unit validation
checks callback parameters and result alternatives. Application generation binds
native symbol `0x008002a0` to source CODE and omits the default blank-view body.
The existing bootstrap is used when no source callback exists. `getInitialView`
cannot be selected as the startup entry. See the [single-View example][initial-view-example]
and [input counter](../../examples/compiler-input-counter/README.md).

## Source behavior delegate

A BehaviorDelegate subclass may supply `function onSelect()` with zero
parameters and a Boolean result on every path, available since API 1.2.0.
The callback may mutate its retained View and call `WatchUi.requestUpdate()`.
It cannot be invoked as an ordinary source call. Other input callbacks and
event-object contracts are outside this profile.

Construction and local copies preserve exact delegate identity for ordinary
source-method dispatch. Canonical `Toybox.WatchUi.BehaviorDelegate` parameters
and results expose only the native base, erasing exact source identity.
Named source-class parameter/result annotations and reference-field targets
still require View subclasses; a delegate class name cannot be treated as a
View. Delegate arrays, nullable delegates, and source inheritance are unsupported.
See [input evidence](../investigations/compiler-input-counter.md).

[initial-view-example]: ../../examples/compiler-initial-view/README.md
[initial-view-evidence]: ../investigations/compiler-initial-view.md

## Source startup callback

An AppBase class may define `function onStart(state)` with one parameter and a
void result. Its inherited native contract is Dictionary-or-null, represented
by `ParameterType::NullableDictionary`; other functions retain integer
parameters. Callback contracts are resolved independently of VM symbol IDs,
slot numbers, and import indexes. Unit validation rejects wrong callback
signatures and direct user calls to onStart.

`function onStop(state)` uses the same resolved nullable-state/Void contract.
Its method record binds to native symbol `0x008002a1`; the runtime invokes it
on termination. It is neither a bootstrap entry nor an ordinary user-call
target. Supplying it does not change startup-entry selection. Normal shutdown
can be requested with [System.exit](compiler-subset.md#native-module-statements).

Complete application generation requires `--entry Class.onStart` when that
callback exists. Its DATA method record binds directly to source CODE; the
generated startup wrapper is omitted. Constructor and default blank View remain
generated unless their source overrides are supplied. Without source onStart, the
existing zero-parameter startup-entry workflow remains available.

The state parameter can be read, copied into locals, and compared with null
using `==` in either operand order. Nullable Dictionary locals may receive null
without changing their declared contract. Dictionary access/construction,
printing, numeric operations, returns, and forwarding through integer user
calls remain unsupported. Null locals can only receive null; this subset does
not infer wider local types. Tests preserve these types through saved IR.

These state restrictions also apply to onStop. The tested simulator exit path
supplies null; other termination states remain unverified. Application
generation rejects class names colliding with pinned native symbols before
allocating class IDs, so a name such as `exit` cannot overwrite a native binding.

Backend self calls select `getselfv @__ciq_member_<name>`, followed by `invokemz`
for zero arguments or `frpush`, ordered arguments, and `invokem <count + 1>`.
Statement calls append `popv`. No explicit receiver operand is pushed. This
sequence is observed in SDK 9.2.0 and verified in generated `fr265` applications;
other device/API profiles and broader inherited lookup remain unverified.

[unit]: compiler-unit.md
[app]: console-application.md
[ir]: compiler-ir-text.md#instance-receivers
[evidence]: ../investigations/compiler-class.md
