# Restricted source classes and receiver calls

This profile adds AppBase- and View-derived classes to [compilation units][unit].
Existing function parameter and result rules still apply. The
[application builder][app] selects one AppBase class and binds additional Views.
See [receiver design][design] and [runtime evidence][evidence].

## Source and ownership

```text
class = "class" identifier "extends" native-base
        "{" native-declaration* (number-field | function)+ "}"
native-base = recognized Application.AppBase or WatchUi.View identity
number-field = "var" identifier "=" signed-number-literal ";"
receiver-call = "self" "." member "(" argument-list? ")"
```

Classes are top-level declarations. The base is a recognized native identity,
resolved through the [native name profile](compiler-subset.md#native-name-declarations).
It accepts canonical/qualified, aliased, and imported AppBase/View spellings.
Number fields are supported below. Static methods, annotations, source
superclass lookup, and nested/empty classes are unsupported. Classes require at
least one method. Class names must be unique;
classes cannot be reopened. Reject class/module/global-function name collisions.

Reserved class names include language keywords, `self`, `System`, `Application`,
`AppBase`, `globals`, `statics`, `Toybox`, and the bootstrap's pinned native names:
`initialize`, `println`, `View`, `onStart`, `getInitialView`. These restrictions
belong to this constrained profile, not the general Monkey C language.

`self` is reserved and may appear as a call receiver or a declared field receiver.
It cannot be rebound or passed as a value. Bare member calls,
class-name calls, and calls on other objects remain unsupported. Module calls
from methods retain their existing rules.

A self expression call requires a declared value-returning method in the same
class. A self statement call discards any supported result, including Void.
Arguments follow scalar/nullable assignability and result contracts must equal
the callee's contract;
counts must match exactly. Forward and recursive
references are checked after collecting signatures. Self lookup never falls
back to a global/module member or an inherited native method. Recursion is
accepted but termination is not proven.

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

`.class <name> Application.AppBase` or `.class <name> WatchUi.View` contains
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
WatchUi.View subclasses and requires
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

A declared class may supply `function initialize()`. It must return void,
accept no parameters, and call the native base initializer exactly once as its
first statement. These are subset restrictions, not general language rules.
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

## Source Views and drawing

`new ClassName()` constructs a declared WatchUi.View subclass with no arguments.
Its initializer must satisfy the native-base contract above. Unit validation
rejects unresolved names, modules, AppBase construction, and missing initializers.
The result has the non-null native View contract; source identity is retained in
the construction operation, not as a distinct value type. It may supply the slot
in `getInitialView()`. Field access and calls on another source instance are
unsupported; current-instance fields use the contract below.

## Number instance fields

AppBase and View classes may declare up to 256 fields with signed 32-bit decimal
Number literal initializers. Fields have a fixed Number contract. Uninitialized
declarations, annotations, non-Number literals, and computed initializers are
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
not a value expression. Compound assignments, arbitrary receiver properties,
static fields, and source-inherited fields are unsupported.

`ClassDeclaration.fields` holds names and initial Number values. Unit validation
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
The inherited contract supplies `TupleShape::OneView` without a source result
annotation. Every return must be a direct one-slot literal, such as
`return [new WatchUi.View()];` or `return [self.makeView()];`. Parentheses around
the literal are permitted. Its element must produce a non-null native View;
factories, locals, and element reads can supply it. All paths must return.

The compiler evaluates and stores the slot expression before packing a fresh
one-slot result; its hidden local shares the normal frame budget. Ordinary
mutable Array values cannot supply this contract, even if their current size is
one. Empty/multiple slots, Null, non-View values, incompatible annotations, and
ordinary user calls to the callback are rejected. Delegate pairs, tuple source
annotations, and general tuple-expression inference remain unsupported.

Saved IR retains the shape through [positional
construction](compiler-ir-text.md#positional-result-construction); unit validation
checks the callback's zero parameters and one-view result. Application generation
binds native symbol `0x008002a0` directly to its source CODE and omits the default
blank-view body. The existing bootstrap is used when no source callback exists.
`getInitialView` cannot be selected as the startup entry. See the
[complete example][initial-view-example] and [evidence][initial-view-evidence].

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
[design]: ../design/compiler-ir.md#class-ownership-and-instance-receivers
[evidence]: ../investigations/compiler-class.md
