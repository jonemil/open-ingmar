# API declarations and core types

`ciq-compiler::api::core()` provides immutable runtime API declarations, populated
in Rust. It owns module/class names, signatures, superclass paths, introduction
levels, callback flags, enum constants, generic parameters, and exact
receiver-result overrides. It contains no runtime method bodies or VM bindings.
No declaration-file grammar is defined. [Architecture](../architecture.md)
separates declaration lookup, compiler validation, and executable lowering.

## Declaration model

Types represent named classes, Null, Void, unions, invariant Arrays and
Dictionaries, positional tuples, and class type parameters. A declaration
describes a runtime-provided API; compiler analysis state does not belong in it.
`Lang.Object` is the root class. Modules are namespaces and do not inherit Object.

`Catalogue::method()` follows superclass paths and returns the declaring class,
effective result, and required API level. `Method.result` retains the generic
signature; `ResolvedMethod.result` substitutes an exact receiver override.
Overrides do not propagate to subclasses. `Object.weak` therefore resolves to
Number, String, or Boolean for those exact immutable receivers, while other
receivers retain WeakReference. This does not enable ordinary weak references.

Assignability follows declared superclass paths without implicit conversion or
acceptance of unknown classes. Every alternative of an actual union must satisfy
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
with Dictionary-specific argument/equality contracts.

Declaration availability, assignability, and executable support are separate
checks. A broad Object parameter can have a narrower executable profile.
Enum constants retain their declared types even if VM encoding uses a Number.
Source class identity and native projection follow the
[class specification](compiler-classes.md).

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

These documented members lack executable prerequisites. Supported methods are
listed only in the source specification. Public API attribution and runtime
results remain in the linked investigations.

- **Inherited Object methods:** `method(Symbol)` (1.0.0) needs Symbol and callable
  Method values. Ordinary `weak` (1.2.0), including Char receivers, needs
  WeakReference values/consumers; Char itself requires 1.3.0. The immutable
  numeric/String/Boolean specialization does not establish this behavior.
  [Boolean](../investigations/compiler-boolean-methods.md),
  [Char](../investigations/compiler-char-methods.md), and
  [weak-result evidence](../investigations/compiler-immutable-weak.md).
- **Array:** sorting needs comparator/callable support. Inherited equality,
  hashing, and weak references require array-specific evidence.

[The plan](../plan.md) owns sequencing and newly discovered prerequisites.
