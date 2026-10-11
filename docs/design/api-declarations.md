# API declarations and core types

`ciq-compiler::api::core()` supplies a bounded shared catalogue. Native callback
validation reads its `onStart`/`onStop` contracts; supported imports obtain module
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
Number arithmetic-related and comparison methods, Boolean comparison, String
length/case/string methods, AppBase initialization/startup/shutdown, and System
printing/exit. Parameter/result subset types expose `api_type()` mappings.
Internal value categories map to Number, String, Boolean, Null, nullable Number,
nullable String, and nullable Dictionary; operator validation uses those
semantic types. The
bounded native call profile resolves Number's inherited `toString`, `equals`,
and `hashCode`, and direct `format`, `abs`, `toNumber`, and `compareTo`,
and String's length, case, string, search, slicing, numeric conversion, equality,
hash, and comparison contracts. Boolean uses direct comparison and inherited
Object string conversion, equality, and hash contracts.
Argument types must satisfy their signatures or a declared union; every
alternative of an actual union must be accepted. This does not
permit implicit conversions or unknown classes. Named arguments may widen
through catalogue-declared superclass paths. Executable method profiles can
restrict a broad declaration further: Number/Boolean comparison admits
Number/Boolean, String comparison admits String, and all three equality
profiles admit supported scalars and nullable scalars. Executable support remains
a separate check. Number results use the existing integer
value category, so arithmetic, call arguments, returns, and further Number calls
need no new IR operation or type.

String results use the existing String category and can receive further
supported calls. User functions accept Number, String, Boolean, and nullable
Number/String parameters and results; API declarations alone do not enable
other types. String `find`, `toNumber`, and `toNumberWithBase` retain
Number-or-Null result contracts; `substring` retains
String-or-Null. Flow validation permits non-null consumers for local or parameter
reads proved non-null by a direct branch.
Other result unions need corresponding validation before executable support.

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
including the requested class, intervening bases, and enclosing modules. It does
not validate a complete device/app profile. The fixed catalogue's inheritance
and type references are checked by tests; a future reader needs equivalent
validation.

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

`ApiContext` holds an explicit target API and application minimum API. The
minimum must be at least 1.0.0 and no higher than the target. Contextual source
and IR entry points check known native references before executable support:
unknown declaration, unavailable on target, above the deployment minimum, and
unsupported lowering are distinct errors. The current source subset has no
`has` guards, so its direct calls must fit both levels. Guarded optional APIs
need availability analysis before this unconditional rule can be relaxed.

The context is external to semantic IR. Source/IR functions with `_with_api`
accept it explicitly; `Function::validate_api` and
`CompilationUnit::validate_api` check already constructed models. The ordinary
entry points omit API-level checks and retain their bounded lowering checks.
Saved IR builds must resupply the context. It neither changes serialized IR
nor configures PRG deployment metadata.

Current checks cover supported Number/String/Boolean value calls, System
printing/exit, AppBase bases, initialization, and known callbacks. Method
introductions in the executable profile are API 1.0.0 except
Number/String/Boolean.compareTo at 5.0.0 and String.toNumberWithBase at 1.4.1;
substring's nullable index form additionally requires 3.3.2. Catalogue knowledge
does not enable methods whose executable types or profiles are unsupported.
Context levels are caller assertions: no device database, permission, app-type,
execution-scope, or VM-capability validation is implied. The console PRG generator
remains the SDK 9.2.0 / fr265-tested profile.

Native module statements have canonical module/member identities and no result
value. Catalogue lookup validates their contract and optional API context before
checking executable support. The current profile admits only zero-argument
`Toybox.System.exit`; printing retains its dedicated IR operation. Exit is an
effectful statement, separate from an IR return or branch. This establishes a
module-call path without adding runtime bodies to declarations.

Substring's nullable index form is checked separately at API 3.3.2, while the
method itself dates from 1.0.0. Numeric range-dependent behavior is not inferred
from types; callers must provide a compatible minimum for negative indices.
See the [source contract](../specs/compiler-subset.md#nullable-results).

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
comparison; equality also accepts Null/Null and nullable
Number/String/Dictionary with
Null in either order; Boolean pairs support equality. Arithmetic produces Number and
comparisons produce Boolean. Boolean literals and locals share that semantic
identity and use ordinary conditions, assignments, and printing.
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
