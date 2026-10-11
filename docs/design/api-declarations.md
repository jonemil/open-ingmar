# API declarations and core types

`ciq-compiler::api::core()` supplies a bounded shared catalogue. Native callback
validation reads its `onStart` contract; supported imports obtain their module
name from its declarations. Native lowering and application bindings remain
restricted profiles. [The plan](../plan.md) owns migration and expansion.

## Declaration model

Semantic declarations are populated directly in Rust inside `ciq-compiler`.
The catalogue owns names, signatures, and superclass paths; callers receive
immutable declarations. A future definition-file reader should produce the same
model. No declaration-file syntax or new crate is required initially.

Current entries cover Lang's Object, Number, String, Boolean, Char, and
Dictionary, plus Application.AppBase and the Lang/Application/System modules.
Only selected methods are recorded: Object equality/string/hash contracts,
Number arithmetic-related methods, AppBase initialization/startup, and System
printing. Parameter/result subset types expose `api_type()` mappings. Internal
value categories map to Number, String, Boolean, Null, and nullable Dictionary;
operator validation uses those semantic types. The bounded native call profile
resolves Number's inherited `toString` contract and lowers it separately.

Classes hold instance methods; modules hold module functions. Methods carry
parameter/result types, an introduction API level, and a callback flag. Types
represent named classes, Null, Void, and unions. Add fields, constants, explicit
static member distinctions, and parameterized containers as needed. Declarations
describe runtime-provided APIs without compiling replacement method bodies.
Keep compiler analysis state outside this model.

Semantic identities such as `Toybox.Lang.Number` are independent of VM symbol
IDs, import indexes, section offsets, and implementation functions. Native
binding maps resolved references to executable forms. A future runtime could
associate these declarations with its own implementations; extract a shared
crate only when another consumer needs it.

## Availability

`Catalogue::method()` follows superclass paths and returns the declaring class,
member, and effective minimum API level. `available_at()` checks that level,
including the requested class and intervening bases. It does not validate a
complete device/app profile. The fixed catalogue's inheritance and type
references are checked by tests; a future reader needs equivalent validation.

Introduction levels are recorded on modules, classes, and methods. Add documented
signature changes or removal when relevant; deprecation alone does not mean
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

Operator rules live separately from method declarations in
`ciq-compiler::operators`. IR validation maps operand categories to API types,
checks the supported operator contract, and maps its result back to an executable
value category. Parameter/result mappings share that bridge. The internal enum
remains a bounded profile, not a replacement for arbitrary API type expressions.

Current rules accept Number/Number addition, subtraction, equality, and greater
comparison; equality also accepts Null/Null and nullable Dictionary/Null in
either order. Arithmetic produces Number and comparisons produce Boolean.
These are our supported rules, not a complete Monkey C operator specification.
String addition, mixed numeric conversions, and general equality need evidence
and lowering support before being added.

Semantic IR preserves operations such as addition; the backend selects VM
instructions. Rules contain no opcodes, storage slots, or method bindings. A
numeric class identity does not imply that addition becomes a method call.

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
