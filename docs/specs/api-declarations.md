# API declarations and core types

`ciq-compiler::api::core()` provides immutable runtime API declarations, populated
in Rust. It owns module/class/interface names, signatures, superclass paths, introduction
levels, callback flags, enum constants, generic parameters, and exact
receiver-result overrides. It contains no runtime method bodies or VM bindings.
No declaration-file grammar is defined. [Architecture](../architecture.md)
separates declaration lookup, compiler validation, and executable lowering.

## Declaration model

Types represent named classes/interfaces, Null, Void, unions, invariant Arrays and
Dictionaries, positional tuples, optional Symbol-keyed option fields, and class
type parameters. A declaration
describes a runtime-provided API; compiler analysis state does not belong in it.
`Lang.Object` is the root class. Modules are namespaces and do not inherit Object.

`Catalogue::type_alias()` expands Numeric, Integer, and Decimal into their
[scalar alternatives](compiler-subset.md#numeric-values). These typedefs are
neither classes nor enums. NumberFormat and Endian are declared enum types from
API 1.0.0; their published constants require 3.1.0.
Executable signatures retain these enum identities, while `.constant` retains
the member's introduction level and lowers to its integer value. ByteArray conversion
methods declare optional `offset`/`endianness` fields; the
[bounded source option builder](compiler-subset.md#bytearrays) checks each
field before exposing a record, independently of general structural assignability.

Structural interfaces record required method signatures separately from classes.
`Catalogue::interface()` resolves them without class inheritance or construction;
`interface_level()` includes the interface and enclosing module introduction
levels. Named interface types participate in `ApiContext::check_type()`.
Recording an interface alone does not enable source annotations or executable
implementations. Consumer-specific structural checks must establish the
required members before native-call checking can use that interface type.

`Lang.Comparable` requires `compareTo(Object) as Number` from API 5.0.0.
`CompilationUnit::check_interface(class, interface, context)` checks directly
declared source members against a catalogue interface with exact parameter and
result contracts. It checks the interface, signature types, members, and native
base against the supplied target/minimum API. The query can be repeated on
parsed unit IR; it does not serialize a proof or change the unit. A source-class
parameter cannot satisfy Object merely because its native projection is Object.
This establishes signatures, not comparison behavior. Executable interface values
follow the [source contract](compiler-subset.md#comparison-interface-values).

`Lang.Comparator` requires `compare(Object, Object) as Number` from API 5.0.0.
Array `sort(Comparator or Null)` returns Void from 5.0.0. The executable
[sorting profile](compiler-subset.md#source-comparators) accepts Null or an
exact source-class witness. Unit validation requires a declared `compare`
with exactly matching Object parameters and Number result, following the SDK
checks. The witness is local to the consumer; it does not establish general
interface assignability, inheritance, or interface-typed storage.

Method declarations have a fixed parameter prefix and an optional repeated
argument type. Receiver instantiation substitutes both through generic
container bindings. Fixed declarations still require exact arity; variadic
declarations check the prefix and each additional argument. Only the supported
native-value call profiles authorize execution.

Object `method(Symbol)` returns Method since API 1.0.0. Method declares
`hashCode()` and variadic `invoke` from 1.0.0. Its declared invocation result is
Object; the exact Method receiver uses a conservative Object-or-Null result
because erased callbacks can return Void or Null. Repeated arguments accept
Object or Null. This does not recover a target signature or validate its
introduction level; see the [executable contracts](compiler-subset.md#method-values-and-invocation).

`Catalogue::method()` follows superclass paths and returns the declaring class,
effective result, and required API level. `Method.result` retains the generic
signature; `ResolvedMethod.result` substitutes an exact receiver override.
Overrides do not propagate to subclasses. `Object.weak` therefore resolves to
the verified numeric, String, Boolean, Char, and Symbol types for those exact
immutable receivers, while other
receivers retain WeakReference. Executable support follows the
[bounded reference profile](compiler-subset.md#ordinary-weak-references).

Assignability follows declared superclass paths without implicit conversion or
acceptance of unknown classes. The executable profile also recognizes bounded
[native Comparable witnesses](compiler-subset.md#comparison-interface-values).
Every alternative of an actual union must satisfy
the expected type. `Catalogue::method_for_type()` substitutes Array's `Element`
or Dictionary's `Key`/`Value` throughout argument/result types, including nested
containers and tuples. It retains the resolved declaration/availability and
normalizes substituted unions by flattening and removing duplicate alternatives.
Unresolved signature parameters return None; an opaque named Dictionary cannot
invent key/value contracts. `Catalogue::method()` still returns generic declarations.

Container parameters are invariant: differing Array elements or Dictionary
key/value contracts cannot widen or narrow through assignment. Typed containers
may widen to their opaque class or Object; erasure does not permit assigning
them back to a typed container. Tuples require exact arity and
per-slot assignability; they are not homogeneous Array contracts.

Dictionary declarations specialize `get(K)` to `V or Null`, `put(K, V)` and
`remove(K)` to Void, `hasKey(K)` to Boolean, `keys()` to `Array<K>`, and
`values()` to `Array<V>`. A non-null V rejects Null writes even though the broad
published method accepts Null. These members are API 1.0.0; Dictionary's own
`toString()` override requires 1.0.1. The existing `size`/`isEmpty` declarations
remain 1.0.0. [SDK type probes](../investigations/compiler-dictionary.md#typed-keyvalue-contracts)
support specialization and invariant contracts. The compiler still uses the
[bounded executable Dictionary profile](compiler-subset.md#dictionary-values),
including typed source/saved IR, lookup/mutation, typed `keys`/`values` Arrays,
Dictionary's own `toString` dispatch, and inherited Object `equals`/`hashCode`
with shared argument and Dictionary identity contracts.

Declaration availability, assignability, and executable support are separate
checks. A broad Object parameter can have a narrower executable profile.
Enum constants retain their declared types even if VM encoding uses a Number.
Source class identity and native projection follow the
[class specification](compiler-classes.md).

WeakReference `get()` returns Object or Null and `stillAlive()` returns Boolean;
both members and their class require API 1.2.0. Opaque Object results retain no
concrete contract automatically; callers can apply the supported type tests or
lexical assertions. Parameterized-container recovery remains unsupported.

## Availability

Introduction checks include the requested class, intervening bases, enclosing
modules, methods, and recursively referenced signature/container types. Class
existence does not establish availability of every initializer or method.
Deprecation alone does not mean unavailable; unknown restrictions stay explicit.

`ApiContext` contains target and application minimum API levels. The minimum must
be at least 1.0.0 and no greater than the target. Unguarded references must fit
both levels. Unknown declaration, unavailable target, deployment-minimum
violation, and unsupported lowering require distinct diagnostics.

Context is supplied externally through source/IR `_with_api` entry points or
`validate_api`; ordinary entry points omit availability checks but still validate
supported lowering. Saved IR does not serialize context. These levels are caller
assertions: they neither select a device nor set PRG deployment metadata. Device,
app-type, scope, permission, and guarded optional-API checks remain planned in
[project builds](project-builds.md).

## Executable contracts

The [source specification](compiler-subset.md#native-value-calls) owns supported
methods, signatures, API thresholds, and operator behavior. The
[IR specification](compiler-ir-text.md) owns their serialized representation and
shared validation. Catalogue entries must not silently widen those profiles.
Internal values map to `Lang` API identities; operator rules select result types
before the backend selects instructions. Numeric operations need not be method
calls. API object identity does not establish a heap allocation, tagged layout,
or boxing protocol; do not infer those from the language's heap markers.

Current catalogue inheritance/type references are checked by tests. Any future
external definition reader must establish equivalent closure and availability
invariants. A runtime may later associate these declarations with its own
implementations without coupling the catalogue to VM storage.

## Deferred core methods

This inventory covers outstanding Lang contracts, not just missing method names.
The [source specification](compiler-subset.md) owns implemented behavior and its
bounded argument/storage profiles. Compare each class's own and inherited
members against the reviewed public API before marking it complete. Remaining
operator/type-system dependencies and sequencing belong to the
[core completion gate](../plan.md#core-completion-gate).

- **Number, Long, Float, Double, Boolean, Char, String:** close the full
  applicable Object/argument domains of comparison and observers. Resolve
  literal/Unicode discrepancies and validate remaining boundaries. Existing
  own-method names largely have executable profiles.
- **Object and source objects:** complete source inheritance/overrides, member
  lookup, source-inheritance/general-union type tests and refinement, and broader
  inherited-method dispatch.
- **Array:** general nested/heterogeneous/Object/reference
  elements, remaining nullable storage/receivers, and sorting and
  Comparable contracts outside the checked comparison profiles.
- **Dictionary:** general Object/reference keys and values, heterogeneous/nested
  containers, general nullable storage, and typed keys/values Arrays for those
  domains.
- **Symbol:** full selector/member-resolution contracts, shared with source
  inheritance and Method lookup work.
- **Method:** constructor and class/module receiver values,
  known-selector checks and concrete callback
  signature/refinement rules where required by consumers.
- **WeakReference and Object.weak:** broader source/Object targets
  and safe concrete recovery from get(); preserve exact
  immutable weak-result behavior.
- **Exception family:** exception construction, message/stack methods, source
  subclassing, and throw/try/catch/finally lowering and unwinding. Distinguish
  fatal runtime errors.
- **ResourceId:** resource-reference value/binding contracts and own/inherited
  methods, using minimal fixtures before broader resource generation.
- **Lang module and supporting types:** Lang.format, general unions, and broader
  structural option types.

Array default sorting and checked source Comparator instances are implemented
within their [documented profile](compiler-subset.md#source-comparators).
Direct source-instance/self Method binding is likewise implemented; remaining
Method work concerns construction and broader contracts. Neither closes all
container or callable requirements.

This compiler provides checking, binding, and invocation of runtime methods.
Completion does not require implementing native firmware method bodies. The
[public API review](../investigations/core-lang-completion.md) records inventory
provenance; future discoveries amend this list and the plan.
