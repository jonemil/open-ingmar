# Restricted source classes and receiver calls

This profile adds AppBase-derived classes to [compilation units][unit]. Existing
integer/void function rules still apply. The [console application builder][app]
can use one declared class and invoke a selected method on its instance.
See [receiver design][design] and [runtime evidence][evidence].

## Source and ownership

```text
class = "class" identifier "extends" native-app-base
        "{" native-declaration* function+ "}"
receiver-call = "self" "." member "(" argument-list? ")"
```

Classes are top-level declarations. The base is a recognized native identity,
resolved through the [native name profile](compiler-subset.md#native-name-declarations).
It accepts canonical/qualified, aliased, and imported AppBase spellings, not
general native classes. Fields, static methods, annotations, source superclass lookup,
nested/empty classes, and `new` are unsupported. Class names must be unique;
classes cannot be reopened. Reject class/module/global-function name collisions.

Reserved class names include language keywords, `self`, `System`, `Application`,
`AppBase`, `globals`, `statics`, `Toybox`, and the bootstrap's pinned native names:
`initialize`, `println`, `View`, `onStart`, `getInitialView`. These restrictions
belong to this constrained profile, not the general Monkey C language.

`self` is reserved and may appear only as this explicit call receiver. It cannot
be rebound, passed as a value, or used for property access. Bare member calls,
class-name calls, and calls on other objects remain unsupported. Module calls
from methods retain their existing rules.

A self expression call requires a declared integer-return method in the same
class. A self statement call permits an integer or void result and discards it.
Arguments remain integers; counts must match exactly. Forward and recursive
references are checked after collecting signatures. Self lookup never falls
back to a global/module member or an inherited native method. Recursion is
accepted but termination is not proven.

## Native and textual IR

`ClassDeclaration` records a name, `ClassBase::ApplicationAppBase`, and ordered
method bodies. Base identity does not contain a VM import index. `Function::method`
constructs receiver-bearing IR; ordinary function constructors reject self-call
operations. Unit validation requires instance bodies in classes and ordinary
bodies in global/module owners. An instance method can have no self calls and
still retains its receiver requirement.

`SelfCall` defines an integer value. `SelfCallStatement` consumes argument values
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

`.class <name> Application.AppBase` contains complete function documents until
`.end-class`. Global/module owner records remain valid alongside classes. The unit header is `ciq-unit` for both class-bearing and ordinary units.

CODE layout places global/module bodies first, then class bodies. Each group
retains file/declaration order; methods retain order within their class. Class
labels are `__ciq_fn_c_<class-length>_<class>_<member-length>_<member>`, with
ASCII names and decimal lengths. Internal aliases use this prefix. The total
1..=256-body limit includes all functions and methods. Validation error body
indexes follow this layout order; serialized spans remain provenance only.

## Application and VM binding

Complete PRG generation permits exactly one declared AppBase class and requires
`--entry Class.method`. The selected method must have zero parameters, except
source onStart follows the callback contract below. Integer
results are printed by generated onStart; void results are discarded. Class symbol `1` identifies the application class. Plain names
matching the class reuse that identity; other names retain deterministic
allocation. Class methods are sorted DATA fields bound to their CODE labels,
separate from globals/modules even when member names share one symbol ID.

Methods named `getInitialView` or `AppBase` remain unsupported.
The source initializer contract below permits `initialize`; otherwise its
body, native imports, empty resources, and the blank View remain
provided by the bootstrap. Unit/IR stages can represent multiple classes, but
this PRG profile rejects them. Source callbacks and general object/native-call
operations beyond the contracts below remain subsequent work.

## Source initialization

A declared class may supply `function initialize()`. It must return void,
accept no parameters, and call the native base initializer exactly once as its
first statement. These are subset restrictions, not general language rules.
Subsequent supported statements may print or call source methods. Initialization
cannot be selected as the generated onStart entry.

Accepted base calls are `AppBase.initialize()`,
`Application.AppBase.initialize()`, fully qualified
`Toybox.Application.AppBase.initialize()`, and equivalent module aliases.
Bare AppBase here identifies the inherited superclass even without an import;
this member-call rule does not make bare AppBase legal in an `extends` clause.
Arguments, value uses, other base members, and base calls outside a class
initializer are rejected. Conditional, late, duplicate, or absent base calls
are also rejected by unit validation.

`NativeBaseInitialize { base: ClassBase::ApplicationAppBase }` is an ordered
effect with an implicit current instance and no explicit arguments/result.
It identifies a native superclass, not a source callee or executable import
index. Lowering selects `getselfv AppBase; getsv initialize; invokemz; popv`.
The application builder binds native aliases and directs the initializer's
single DATA method record to source CODE. It keeps the default initializer when
the source omits one. Constructor, onStart, and blank View generation remain
provided by the bootstrap unless source onStart is supplied.

## Source startup callback

An AppBase class may define `function onStart(state)` with one parameter and a
void result. Its inherited native contract is Dictionary-or-null, represented
by `ParameterType::NullableDictionary`; other functions retain integer
parameters. Callback contracts are resolved independently of VM symbol IDs,
slot numbers, and import indexes. Unit validation rejects wrong callback
signatures and direct user calls to onStart.

Complete application generation requires `--entry Class.onStart` when that
callback exists. Its DATA method record binds directly to source CODE; the
generated startup wrapper is omitted. Constructor and blank View remain
generated, and a source initializer is optional. Without source onStart, the
existing zero-parameter startup-entry workflow remains available.

The state parameter can be read, copied into locals, and compared with null
using `==` in either operand order. Nullable Dictionary locals may receive null
without changing their declared contract. Dictionary access/construction,
printing, numeric operations, returns, and forwarding through integer user
calls remain unsupported. Null locals can only receive null; this subset does
not infer wider local types. Tests preserve these types through saved IR.

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
