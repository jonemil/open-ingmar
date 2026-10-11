# API declarations and core types

This is the intended boundary; a shared catalogue is not implemented yet.
Current native names, callback contracts, intrinsics, and executable bindings
are bounded special cases. [The plan](../plan.md) owns migration and expansion.

## Declaration model

Start with semantic declarations populated directly in Rust inside
`ciq-compiler`. A future definition-file reader should produce the same model;
consumers must not depend on how declarations were loaded. No declaration-file
syntax or new crate is required initially.

Represent module/class ownership, superclass identity, methods, fields, and
constants. Members retain receiver kind, parameter/result types, and relevant
callback contracts. Type expressions must grow to describe unions, nullability,
and containers. Declarations describe runtime-provided APIs without compiling
replacement method bodies. Keep compiler analysis state outside this model.

Semantic identities such as `Toybox.Lang.Number` are independent of VM symbol
IDs, import indexes, section offsets, and implementation functions. Native
binding maps resolved references to executable forms. A future runtime could
associate these declarations with its own implementations; extract a shared
crate only when another consumer needs it.

## Availability

Record availability on classes and individual members, including inherited
members through their declaring class. Track introduction levels and documented
signature changes or removal where relevant; deprecation alone does not mean
unavailable. Preserve sources and unknown restrictions rather than treating
missing information as universal support.

Use current public API documentation as a starting catalogue, retaining the
metadata needed for older targets. Keep these inputs distinct:

- Catalogue coverage: declarations and contracts known to the compiler.
- Target capabilities: device/API level, app type, scope, and permissions.
- Application minimum API: its declared deployment requirement.
- VM capabilities: executable instruction and encoding support.

A known declaration can be unavailable on a target or lack implemented lowering.
Those cases need different diagnostics from an unknown declaration. Defining a
class does not establish support for every member or runtime binding.

Optional APIs guarded by `has` need explicit availability analysis. Do not reject
all references newer than the application's minimum API indiscriminately, or
claim portable guarded code before its compilation/runtime behavior is tested.
Exact checking and target-specialization rules remain to be established.

## Core language relationship

Model `Lang.Object` as the class-hierarchy root and connect built-in values to
their `Lang` identities. Begin with types needed by existing examples, then grow
coverage through runnable applications. Modules remain namespaces with their
own rules; they do not inherit `Lang.Object`.

Keep operator rules separate from method declarations. Rules determine accepted
operand types, conversions, result types, and language-visible errors. Semantic
IR preserves operations such as addition; the backend selects VM instructions
or other supported implementations. A numeric class identity does not imply
that addition becomes a method call.

Object semantics and runtime storage are separate. The language's heap markers
describe allocation costs, not an established tagged-value layout or a compiler
boxing protocol. Do not add allocations around arithmetic based on those
markers. A future VM must choose a representation consistent with observable
language behavior; the current compiler needs validated instruction and call
lowering, not the runtime's internal storage layout.

Public contracts and unresolved representation questions are recorded in the
[official documentation investigation][evidence]. Accepted source/IR formats
remain specified by the existing compiler specs; catalogue entries must not
silently expand those contracts.

[evidence]: ../investigations/official-garmin-documentation.md#core-lang-types-and-api-declarations
