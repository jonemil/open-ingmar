# Restricted source classes and receiver calls

This profile adds AppBase-derived classes to [compilation units][unit]. Existing
integer/void function rules still apply. The [console application builder][app]
can use one declared class and invoke a selected method on its instance.
See [receiver design][design] and [runtime evidence][evidence].

## Source and ownership

```text
class = "class" identifier "extends" "Application" "." "AppBase"
        "{" function+ "}"
receiver-call = "self" "." member "(" argument-list? ")"
```

Classes are top-level declarations. The base is a recognized native identity,
not a general import/name resolver. No `using` declaration is required or
accepted yet. Fields, static methods, annotations, source superclass lookup,
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

Function [IR version 4][ir] adds `receiver=instance`, `.call-self`, and
`.call-self-discard`. Unit version 2 adds class containers:

```text
ciq-unit 2
.class Counter Application.AppBase
ciq-ir 4
.function start receiver=instance
.call-self-discard announce
.return-void
.end
ciq-ir 4
.function announce receiver=instance
.integer %0 41
.println %0
.return-void
.end
.end-class
```

`.class <name> Application.AppBase` contains complete function documents until
`.end-class`. Global/module owner records remain valid alongside classes. Unit
version 1 rejects class records. Canonical output uses version 2 only when
classes exist; old function/unit documents remain accepted.

CODE layout places global/module bodies first, then class bodies. Each group
retains file/declaration order; methods retain order within their class. Class
labels are `__ciq_fn_c_<class-length>_<class>_<member-length>_<member>`, with
ASCII names and decimal lengths. Internal aliases use this prefix. The total
1..=256-body limit includes all functions and methods. Validation error body
indexes follow this layout order; serialized spans remain provenance only.

## Application and VM binding

Complete PRG generation permits exactly one declared AppBase class and requires
`--entry Class.method`. The selected method must have zero parameters. Integer
results are printed by generated onStart; void results are discarded. Class
symbol `1` replaces the previously anonymous application identity. Plain names
matching the class reuse that identity; other names retain deterministic
allocation. Class methods are sorted DATA fields bound to their CODE labels,
separate from globals/modules even when member names share one symbol ID.

Methods named `initialize`, `onStart`, `getInitialView`, or `AppBase` collide with
generated lifecycle/base fields and are rejected by this application profile.
Those bodies, native imports, empty resources, and the blank View remain
provided by the bootstrap. Unit/IR stages can represent multiple classes, but
this PRG profile rejects them. Source callbacks and general object/native-call
operations remain subsequent work.

Backend self calls select `getselfv @__ciq_member_<name>`, followed by `invokemz`
for zero arguments or `frpush`, ordered arguments, and `invokem <count + 1>`.
Statement calls append `popv`. No explicit receiver operand is pushed. This
sequence is observed in SDK 9.2.0 and verified in generated `fr265` applications;
other device/API profiles and broader inherited lookup remain unverified.

[unit]: compiler-unit.md
[app]: console-application.md
[ir]: compiler-ir-text.md#version-4-instance-receivers
[design]: ../design/compiler-ir.md#class-ownership-and-instance-receivers
[evidence]: ../investigations/compiler-class.md
