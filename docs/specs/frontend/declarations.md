# Declarations and name resolution

## User module calls

Qualified calls resolve to a top-level user module in the compilation unit.
Value calls require a supported non-Void result. Source bodies and ownership
metadata are collected before call checking; callers need not precede callees.
Arguments must satisfy the declared types and positional arity: a nullable
parameter accepts Null, its scalar type, or the same union; scalar parameters
require exact types. Global unqualified calls and nested modules remain
unsupported. Calls through proven source-instance receivers follow the [class
contracts](classes.md). Void module calls are supported as
[statements](statements.md#void-functions-and-statement-calls). The separate
[System.println statement](lang-values.md#strings-and-printing) prints supported
scalars. Module/member names stay symbolic until encoding; neither IDs nor DATA
method records are generated here.

IR contains `ModuleCall { result, result_type, module, member, arguments }`.
[Call text](../ir/calls.md#module-calls-and-operation-order) preserves the
result contract. Calls may have effects. For any function containing a call, IR
operations must appear in the backend's left-to-right expression evaluation
order, with initialization and print statements in order. Reject incompatible
schedules rather than silently reordering effects; pure-only IR keeps its
existing ordering flexibility. This conservative restriction can change when a
more general scheduling policy is implemented.

[VM lowering](../lowering/calls.md#user-module-calls).

## Parameters and positional arguments

Parameters use the [type/default rules](types.md#types-and-value-contracts).
Identities are zero-based positions, distinct from locals/value IDs; repeated
reads define fresh values. Duplicate names and local/parameter shadowing are
rejected. Unit validation checks callee arity and argument/result contracts.

[IR parameter operations](../ir/values.md#parameter-metadata-and-reads)
retain positional identity and typed argument order; the [IR
grammar](../ir/grammar.md) owns saved metadata. [Frame/call
lowering](../lowering/README.md) maps those identities to the observed
hidden-context convention and u8 slot/arity bounds. The [argument
specimen](../../investigations/compiler-arguments.md) verifies nested calls and
observable argument order, not maximum device capacity.

## Native name declarations

Default and unit source modes accept these declarations before functions or
owners at the start of a file, and before methods at the start of a
class/module:

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

`using` introduces a module binding, optionally aliased. `import` introduces the
module name and its directly declared type names known to the API catalogue:
classes, interfaces, and enums. Bare native names require that import; qualified
module names and aliases resolve through the same catalogue. For example,
`import Toybox.Lang;` permits `Number` in annotations, casts, and runtime class
tests, while `using Toybox.Lang as L;` permits `L.Number`.

Resolution also covers supported Array element and Dictionary key/value type
arguments. Each consumer retains its own executable profile: recognizing
Comparator does not make it a runtime class, and recognizing Array does not
enable an unparameterized cast. API introduction checks use canonical types;
imports neither filter names by API level nor bypass those checks.

The same environment resolves base clauses and supported constructors: `import
Toybox.Application;` permits `extends AppBase`; module aliases permit `extends
App.AppBase`. Existing canonical short module paths such as
`Application.AppBase` and `System.println` remain available without
declarations. Fully qualified Toybox paths are also accepted.

Bindings apply to following declarations in their file and are inherited by
class/module bodies. Owner-local bindings do not escape their owner; bindings
are not shared between files, including reopened source modules. Imports in the
body cannot resolve that class's superclass clause. This is a bounded frontend
profile, not a specification of all Monkey C namespace behavior.

Reject duplicate bindings, shadowing by parameters/locals/functions/owners,
aliases of reserved identities, import aliases, declarations after functions,
and native modules outside this profile. Imported type names also conflict with
module aliases in either declaration order. Bare imported module functions and
constants, general source namespace aliases, and types outside the catalogue
remain unsupported. Number conversion uses
[inherited lookup](../sdk/declarations.md#declaration-model) within executable profiles. Calls on ordinary user modules
and `self` keep their existing rules. Supported native spellings resolve to the
same semantic operation or superclass identity before IR construction; aliases
are not serialized as VM symbols or executable import indexes.

[IR printing](../ir/values.md#string-values-and-print-statements) and
[literal lowering](../lowering/values.md#native-name-declarations) use resolved
native identities; source import aliases are not VM symbols.

[VM lowering](../lowering/values.md#native-name-declarations).

## Instance class extension

Unit mode also accepts [Object-, AppBase-, View-, and BehaviorDelegate-derived
class declarations](classes.md). Their methods may call `self.<member>(...)` as
typed value expressions or statements; class ownership and receiver contracts
remain explicit in typed/saved IR. Default single-function source mode does not
accept self calls. Source initialization and onStart use the
[native lifecycle contracts](../sdk/callbacks.md#source-startup-callback).
Null literals and opaque nullable callback-state locals support equality checks;
[object/field contracts](fields.md) apply alongside the supported core Lang
profiles. Source inheritance and unrestricted native calls remain unsupported.
See [runtime evidence](../../investigations/compiler-class.md).

## Compilation units

This profile collects multiple restricted functions before lowering CODE. It is
independent of application profiles; the
[console application builder](../build/application.md) supplies a complete PRG
from this unit, while Jungle resolution remains future work. The
[function subset](README.md) and [function IR](../ir/model.md)
remain the per-function contracts.

## Source declarations

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

Inputs are one or more source files in caller-supplied order. Global functions
and flat module members are collected in file/declaration order. Modules may be
reopened within or across files; each `(module, function-name)` identity must be
unique. Global functions use a separate owner. Reject duplicate globals and a
global function whose name matches a declared module. `System` is reserved as an
intrinsic receiver and cannot be declared as a module.

All function bodies use the existing grammar and limits. Qualified user calls
must resolve to a module member in this unit, regardless of declaration order.
Call argument counts must exactly match the callee's parameter count.
Parameter/result domains, nullable assignability, and unannotated defaults
follow the [function type contracts](types.md#types-and-value-contracts). These
are supported-profile rules, not general Monkey C type inference.

AppBase.onStart/onStop have a [native nullable-state
contract](../sdk/callbacks.md#source-startup-callback). Their parameter types
and void result must match the resolved callback signature; neither can be
invoked as an ordinary source callee.

View.onShow/onUpdate/onHide use the [View callback
contracts](../sdk/callbacks.md#application-callback-profiles). The enclosing
class's native base selects the callback signature. Source constructors must
resolve to a declared Object-rooted, View, or BehaviorDelegate class with a
validated initializer. Modules and AppBase classes cannot be constructed.

Class containers may declare the supported [instance fields](fields.md). The
unit rejects duplicate or conflicting members and resolves every field operation
against its class, independently of shared member symbol IDs and source-file
order.

Constructor calls resolve initialize as a source callee and check all arguments.
Reference fields must be assigned in an initializer prefix before reads, self
calls, control flow, or return. Field read/result claims and assignments must
match the owning declaration; generic native references cannot forge a source
field contract.

Calls on constructed source references resolve to their proven class's declared
methods. Function validation preserves exact constructor identity through local
copies and same-class assignment; unit validation checks ordinary method
contracts and prohibits native callback invocation through any receiver. See the
[instance-call profile](classes.md#calls-on-constructed-source-objects).

Exact source-class signatures resolve across the whole unit before call
checking. Every source parameter/result name must designate a constructible
declared class; arguments and returned references retain that exact identity.
Native base signatures erase it. Function and call text preserve these contracts
with `source(ClassName@Native.Base)` tags; see the
[source signature profile](classes.md#source-class-signatures).

AppBase.getInitialView has a [one-view positional
contract](../sdk/callbacks.md#source-initial-view-callback). Its return shape
and zero parameters are validated for source, typed, and saved IR. It also
cannot be invoked as an ordinary source callee.

The [native name profile](#native-name-declarations) defines supported
`using`/`import` declarations and lexical boundaries. Native bindings are
resolved before unit call validation; user declarations still collect across
files before calls are checked.

Forward and recursive calls are accepted; termination is not proven. Calls to
undeclared external modules/members are rejected. `System.println` remains the
separate intrinsic. Global functions can serve as entry methods but cannot yet
be called by bare source names. General imports, nested/empty modules,
optional/default parameters, overloading, and arbitrary SDK calls remain
unsupported.

The unit contains 1..=256 functions total across all files. Function operation,
nesting, parameter/local, literal, and control-flow limits remain unchanged. No
total CODE or device-memory budget is inferred from this declaration limit.
Source failures identify the input-file position and a location within it.

## Class extension

[Source class rules](classes.md) define supported native bases, explicit
instance receiver contracts, and self-call validation. `classes()` exposes
ordered class declarations; `with_classes` validates them alongside ordinary
functions. Existing `new` and `declarations()` retain their global/module roles.

The [IR unit contract](../ir/unit.md) owns receiver-bearing serialization and
cross-body checks. [Unit lowering](../lowering/unit-layout.md) owns grouped CODE
order and labels. Complete applications select exactly one AppBase class plus
supported helpers; unit validity alone does not select an application.
