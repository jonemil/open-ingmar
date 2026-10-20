# Functions and source classes

This profile adds Object-, AppBase-, View-, and BehaviorDelegate-derived classes
to
[compilation units](declarations.md).
Existing function parameter and result rules still apply. The
[application builder](../build/application.md) selects one AppBase class and binds constructed source
classes, including additional Views/delegates. See [runtime
evidence](../../investigations/compiler-class.md).

## Source and ownership

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

Classes are top-level declarations. The base is a recognized native identity,
resolved through the [native name
profile](declarations.md#native-name-declarations). It accepts
canonical/qualified, aliased, and imported Object/AppBase/View/ BehaviorDelegate
spellings. An omitted base selects Lang.Object. Supported [field
contracts](fields.md) are defined separately. Static methods, annotations,
source superclass lookup, and nested/empty classes are unsupported. Classes
require at least one method. Class names must be unique; classes cannot be
reopened. Reject class/module/global-function name collisions.

Reserved class names include language keywords, `self`, `System`, `Application`,
`AppBase`, `globals`, `statics`, `Toybox`, and the bootstrap's pinned native
names: `initialize`, `println`, `View`, `Object`, `onStart`, `getInitialView`.
These restrictions belong to this constrained profile, not the general Monkey C
language.

`self` is reserved and cannot be rebound. In an instance body it may be a call
or declared-field receiver, or produce an opaque Object value. Bare member calls
and class-name calls remain unsupported. Calls on constructed source objects
follow the profile below. Module calls from methods retain their existing rules.

A self expression call requires a declared value-returning method in the same
class. A self statement call discards any supported result, including Void.
Arguments follow scalar/nullable assignability and result contracts must equal
the callee's contract; counts must match exactly. Forward and recursive
references are checked after collecting signatures. Self lookup never falls back
to a global/module member. Object-rooted classes may use the supported inherited
Object members described below; source declarations keep priority. Recursion is
accepted but termination is not proven.

## Calls on constructed source objects

`new ClassName(arguments)` establishes an exact constructor class alongside its
native Object, View, or BehaviorDelegate value contract. Local initialization
and copies preserve this identity. A local initialized from a source
construction may subsequently receive only a reference with the same proven
constructor class; assignments from another source class, a generic native View
parameter, or an erased return are rejected. Different instances of the same
class are permitted, including conditional and loop assignments. Local aliases
refer to the same runtime instance; reassignment changes only the local
reference.

`receiver.method(arguments)` calls a declared method of the proven source class.
The receiver may be a construction, local/copy, or typed source
parameter/result. Forward method declarations and multiple source files are
supported. Value calls must match the declared result contract; statement calls
discard any supported result, including Void. Arguments use the existing
scalar/nullable/array contracts and limits. Receiver evaluation precedes
argument evaluation, in source order. Empty Array arguments receive context from
the selected source signature.

Unit validation rejects missing methods, wrong argument/result contracts, and
ordinary calls to native lifecycle callbacks, even through a different receiver.
Object-rooted classes may resolve supported inherited Object members below.
Other undeclared native/source inheritance remains unsupported. Source
inheritance, arbitrary receiver field access, and general dynamic dispatch
remain unsupported.

Exact constructor identity is an invariant local refinement, not a new native
API type. Generic View parameters, declared View returns, and Array<View>
element reads expose only their base contract. They cannot authorize
source-method calls. A generic View local does not gain constructor identity
merely by assigning a source instance later. No runtime cast is emitted.

`ObjectCall` and `ObjectCallStatement` record a receiver value, class name,
member, and arguments; value calls also record the result contract. Function
validation reconstructs source identity from construction, source signatures,
and local operations and requires an exact match with the claimed class. Unit
validation resolves the declaration. This prevents saved IR from forging a
source class for a native reference.

[VM lowering](../lowering/calls.md#call-and-object-lowering).

## Object-rooted classes

`class Counter { ... }` and `class Counter extends Lang.Object { ... }` declare
Object-rooted source classes. The base also accepts `Toybox.Lang.Object`, Lang
module aliases, and imported bare Object. Object construction requires API
1.0.0. Each constructed class must declare a Void `initialize(arguments)`; it
must not call a native base initializer. Declarations inside the class cannot
shadow its reserved Object base binding. An automatic/default initializer and
direct native `new Object()` are outside this profile.

Number fields, supported constructor arguments, local aliases/reassignment, self
calls, and calls on a proven source instance use the ordinary class rules.
Inherited `toString`, `equals`, `hashCode`, and `method` dispatch through the
runtime Object contract when no source override exists. Bound Methods retain the
instance. A source class with the required `compare(Object, Object) as Number`
method can serve as an Array comparator. Weak targets retain their separate
bounded profile.

Source `hashCode` and `equals` overrides must exactly match the native Object
signatures: `hashCode() as Number` and `equals(Object or Null) as Boolean`. Unit
validation checks declarations even when unused, so interface/Object erasure
cannot hide an invalid method reached by Dictionary dispatch. Inherited
implementations need no declaration. This proves call compatibility, not stable
hashes or consistency between equality and hashing. Other native override
families retain their existing restrictions.

The runtime value projects to opaque Object, while construction and locals
retain the exact source identity for own-member calls and structural checking.
It may pass through ordinary Object contracts, but erasure loses own-member
access. Exact source annotations and reference fields preserve the Object-rooted
class and its Object projection. Source tests/assertions can recover exact
identity after erasure. Object-element Arrays erase exact source identities
under the
[container contract](containers.md#object-containers). Exact source-class Array
elements and source superclass chains remain unsupported.

Complete application DATA gives each class an Object base reference and a
return-only hidden allocation constructor. Its user `initialize` runs through
normal source construction. The [application profile](../build/application.md)
owns import binding; no native Object allocation/initialization hook is invoked.
See the
[example](../../../examples/compiler-root-objects/README.md) and
[evidence](../../investigations/compiler-root-objects.md).

## Source-class signatures

Ordinary functions and instance methods may annotate parameters and results with
an unqualified constructible source class name:

```text
function pass(view as HelloView) as HelloView { return view; }
```

The name must resolve to a declared Object-, View-, or BehaviorDelegate-derived
class in the unit, including forward or cross-file declarations. AppBase
references, arbitrary native classes, nullable source references,
Array<ClassName>, aliases, and qualified source-class names are unsupported.
Native annotations use canonical, qualified-alias, or imported bare names
through the shared native type environment.

Source contracts are exact and non-null. Arguments and every value return must
retain the declared source class through construction, typed parameters,
declared results, local copies, positive source-class tests, or explicit source
assertions. The [source-target contract](operators.md#source-class-targets)
distinguishes runtime proofs from caller assertions. A different source class,
Null, or an erased native reference cannot satisfy the contract. Native
contracts may accept a source instance through its base while erasing its exact
class identity.

Module, self, and explicit object calls retain a declared source result,
permitting subsequent source dispatch and same-class local assignment. Parameter
reads also retain their declared class. Native callbacks keep their inherited
signatures; source annotations cannot replace those contracts. No runtime cast
is emitted.

`ParameterType::SourceObject(SourceClass)` and
`ReturnType::SourceObject(SourceClass)` carry the source name and native base
separately. Source declaration collection resolves both across all input files
before checking bodies. API checking projects to that base; exact source
assignability is checked separately. Function validation checks names and return
provenance, while unit validation checks both class identity and native base
against declarations, call arguments, and result claims. Standalone
typed/function IR retains its class/callee resolution obligations.
Single-function source compilation rejects source-class annotations because it
has no declarations from which to resolve their native bases.

Saved parameter tags and function/call result tags use
`source(ClassName@Native.Base)`. Result contracts must be explicit in saved
function text to retain the source identity. See the [signature
example](../../../examples/compiler-source-signatures/README.md) and [runtime
evidence](../../investigations/compiler-source-signatures.md).

## Source initialization

A declared class may supply `function initialize(...)`. It must return Void.
Object-rooted classes have no native base call; other classes call the native
base initializer exactly once as their first statement. AppBase initializers
take no parameters; View/delegate initializers accept the ordinary supported
parameter contracts and limit. These are subset restrictions, not general
language rules. Subsequent supported statements may print or call source
methods. Initialization cannot be selected as the generated onStart entry.

AppBase base calls are `AppBase.initialize()`,
`Application.AppBase.initialize()`, fully qualified
`Toybox.Application.AppBase.initialize()`, and equivalent module aliases. Bare
AppBase here identifies the inherited superclass even without an import; this
member-call rule does not make bare AppBase legal in an `extends` clause.
Arguments, value uses, other base members, and base calls outside a class
initializer are rejected. Conditional, late, duplicate, or absent base calls are
also rejected by unit validation.

View initializers use `View.initialize()`, `WatchUi.View.initialize()`, or the
equivalent fully qualified/aliased identity. The called base must match the
class's declared base, including in saved IR.

`NativeBaseInitialize { base }` is an ordered effect with an implicit current
instance and no explicit arguments/result. It identifies a native superclass,
not a source callee or executable import index. Lowering selects `getselfv
AppBase; getsv initialize; invokemz; popv`. The application builder binds native
aliases and directs the initializer's single DATA method record to source CODE.
AppBase retains its default initializer when source omits one. Constructed
source Views require an explicit initializer. Default lifecycle/View bodies are
used only where source does not supply them.

BehaviorDelegate initializers use `BehaviorDelegate.initialize()` or a
recognized qualified/aliased spelling, following the same native-base-first
rule.
