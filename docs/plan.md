# Plan

## Next work

Finish the core Monkey C language, operators, and `Toybox.Lang` objects and
their methods before returning to broader SDK classes. Reuse the existing console
apps and [input counter](../examples/compiler-input-counter/README.md) to validate
core changes. New UI classes, event objects, drawing APIs, and broader SDK
modules belong to a later phase. Project/resource/package work stays in the
backlog while these language foundations are the priority.

Choose increments by dependency, following the
[class-focused strategy](#native-lang-strategy):

**Object and structural contracts.** Establish member/signature proofs and API
guards through explicit contracts. Extend source
type targets alongside nested names and source superclass chains. Extend source
references to container elements only with explicit storage, variance, and
refinement contracts.
Keep lexical `as` assertions separate from runtime tests and conversions.
Native Object-typed parameters should retain their declared domains; do not
replace runtime type exceptions with narrower compiler-only argument rules.
Parameterized-container and general-union casts need their own type contracts.
Extend general union rules and broader container/storage contracts as core
consumers require them.
Broader option records need explicit field/value contracts beyond the checked
ByteArray option builder. Extend structural source annotations and option
storage/mutation only with corresponding record proofs.
Resolve SDK constant-literal Number/Char key conflation before accepting mixed
scalar-token interface-key constants. Interface-key pool emission also needs a
hash/equality-preserving materialization contract.
Nullable Array sorting needs element/callback contracts and evidence: checked
Comparator parameters currently exclude Null. The SDK rejects explicitly nullable
Comparator parameters, while default sorting failed on a Null element; resolve
this [contract mismatch](investigations/compiler-object-sort.md#failure-probes)
before widening the profile. Further interface refinement
should preserve both structural members and concrete class facts where needed.
Preserve exact source identities until unit
checking can establish required members; an erased Object or a runtime `has`
test alone cannot prove a signature. Keep interface storage distinct from the
exact source-class Comparator witness used by sorting.

Establish checked Method signatures when a concrete callback consumer needs
argument/result guarantees. Keep source-target resolution and saved-IR checking
consistent. Erased invocation does not establish a target's API availability,
arity, or concrete result type.
Symbol-bearing container literals use runtime construction; pooled Symbol payloads need
independent encoding/relocation evidence before adding a DATA representation.

Keep SDK-rejected wider Char ordering and floating Char addition outside the
operator profile. WeakReference Dictionary keys also stay outside the supported
profile following the
[SDK/simulator insertion failure](investigations/compiler-weak-references.md#dictionary-key-rejection);
revisit that decision when new target/runtime evidence warrants it.
Further numeric boundary work should answer a concrete
consumer need: exception recovery, other runtime targets, additional NaN key
patterns, or conversion thresholds. The isolated
[boundary probes](investigations/compiler-numeric-boundaries.md) establish
simulator observations, not portable failure/recovery or conversion guarantees.

Keep general source unions, Null keys, and explicit nested container parameters
deferred under the [bounded source contracts](specs/frontend/README.md#planned-dictionary-extensions).
Nullable Array elements currently need explicit context; broader literal
inference can follow once mixed scalar/Null contracts are established.
Sized Array allocation keeps nullable elements until a separate initialization
proof can establish safe non-null reads; do not infer that proof from writes.

Broader weak targets and container recovery from
opaque Object results need their own type/refinement contracts. Keep hidden
temporary ownership explicit when adding new source staging paths.

A blocked method should lead to its core type/operator/callable prerequisite or
another core class group. Retain explicit deferred-method inventories; declaring
an API member does not establish executable support. Reassess remaining core
gaps before changing focus to the later SDK/project work below.

### Core contracts and evidence

Keep general unions and non-Boolean truthiness deferred, as scoped by the
[operator review](investigations/compiler-operator-coverage.md). Keep updates
statement-only following the [SDK evidence](investigations/compiler-increments.md),
and extend numeric update contracts alongside their value types.

Keep generic signatures, invariant container elements, assignability, and
nullable branch proofs shared across source and saved IR. Extend constructor
and object-reference contracts as core classes need them. Comparator calls and
broader object weak targets need their own dispatch/lifetime evidence;
keep them distinct from the verified [immutable weak specialization][weak-results].
Keep semantic operations explicit enough to select established VM instructions,
following the [lowering review](investigations/natural-lowering.md). Preserve
private initialization for effectful/branching array literals and typed DATA
constants for scalar literals. Baseline instruction selection should preserve
evaluation order and failure behavior without optimization or requiring the
SDK's exact layout. Validate native/text paths and structured reconstruction.

Close the remaining [core API contracts][core-gaps] as their prerequisites
become available.
Read the language reference alongside each Lang contract. Probe sign syntax,
conversion ranges, overflow, promotion, and evaluation order independently;
resolve the [negative-literal postfix][negation] and
[supplementary Char literal][char-literals] discrepancies before widening them.

Preserve native/text pipeline equivalence and complete-app simulator validation
for each group. Keep API minimum, selected target API, VM capabilities, and
device capacity distinct. Grow the [API declarations][api-design] through
core contracts; broader device/app/permission inputs and Application/System
integration belong to the later project/SDK phase. A Jungle driver or complete
API definition parser is not a prerequisite for this core work.

## Core completion gate

Finish the reviewed core Monkey C language and Toybox.Lang contracts before
adding broader SDK classes/modules or resuming project/resource/package
expansion. The [remaining core API inventory][core-gaps] covers named Lang classes,
inherited methods, interfaces, module functions, and supporting constants/types.
Existing names or successful narrow specimens do not establish full coverage.

The [core language worklist](#core-language-worklist) owns remaining language
syntax and semantics, including exceptions. It and the core API inventory are
both prerequisites; completing only operators or named Lang methods is
insufficient. An item deferred for a prerequisite remains inside this phase.

A core feature is complete only when its applicable syntax, semantics,
signatures, argument/result domains, dispatch, API checks, and storage forms
work through the [whole toolchain](#toolchain-completion), with matching
SDK/generated behavior. Explicitly identify unknowns and unresolved failures;
do not move a missing core contract into the later SDK backlog to close a class.
Invalid/unavailable SDK uses should produce appropriate diagnostics. The
compiler emits runtime calls; it does not reimplement firmware method bodies.

Reuse existing application scaffolding, and add only minimal native/resource
fixtures needed to validate a core contract such as ResourceId. Those fixtures
are not a reason to start broader UI or project/resource generation. Reconcile
the reviewed language/reference-guide coverage, public Lang inventory, and
operator tables once more before opening the later SDK phase; close newly
found gaps first.

## Toolchain completion

Requests to implement language operators or Lang/API objects include required
support through every affected toolchain stage: source/API checking, typed and
saved IR, lowering, VM encoding/decoding, assembly text, disassembly, PRG record
reading/writing, and section export/reconstruction. Packaging and signing need
changes when the feature affects their existing contracts.

Inspect diagnostics from feature specimens, including successful commands that
fall back to raw CODE or DATA. Investigate unknown opcodes/records encountered
there and implement their required representations and round trips. Generating
equivalent behavior with already-supported instructions does not close a
decoder/format gap in the reference specimen. Preserve raw fallback for unknown
content, but report unresolved gaps and keep the feature's toolchain coverage
incomplete until the required support is verified.

Validate structured export/reassembly for the affected instructions/records,
lossless payload reconstruction where applicable, source/text pipeline
equivalence, and simulator behavior. These checks establish separate guarantees;
a zero exit status or successful compiler-generated app alone is insufficient.

For our SDK-compiled specimens, require SDK PRG → structured section project →
our assembler → rebuilt PRG → structured section project. Unchanged unsigned
payloads must reconstruct byte-for-byte, and the two exported projects must
match. Required CODE/DATA support cannot pass through raw fallback. Apply the
same reconstruction checks to our compiler-generated PRGs. Preserve signatures
for exact reconstruction tests; test own-key signing separately.

This baseline validates the assembler/disassembler independently of instruction
selection. Our compiler need not emit the SDK's instruction sequence or an
identical PRG, including before optimization work. Compare SDK/generated runtime
behavior separately; future lowering and optimizations may change assembly
while preserving that behavior and our own reconstruction guarantees.

## Native Lang strategy

Organize native Lang work around one class at a time. Batch methods that fit
the available types, including closely related prerequisites when practical,
to share a complete-app specimen and final validation run. The compiler
implements contracts, availability checks, and dispatch; the runtime supplies
method bodies.
Methods may be sequenced across core classes as dependencies require; the
[core completion gate](#core-completion-gate) still blocks broader SDK work.

- Review the class's methods, including inherited members, together. Record
  argument/result contracts, API constraints, evidence, and unknowns in a focused
  investigation. Identify methods that fit existing types and dispatch, and
  name the prerequisites for deferred methods, such as new numeric types,
  arrays, or callable values. Do not treat declarations as executable support.
- Implement the supported group together. Add shared type, assignability, or
  lowering machinery once where needed; keep operator work separate from native
  method dispatch. Extend specs only for the accepted contracts.
- Reuse a complete application for grouped SDK/generated simulator comparison
  and source/IR/assembly equivalence. Cover method-specific behavior and failures,
  wrong arguments, nullable results, and API boundaries with focused tests.
  Run shared checks after the group rather than repeating them for every method;
  isolate probes whose runtime failures would hide later results.
- Move to the next useful class or prerequisite after validating the group.
  Revisit deferred methods when their core dependencies exist. Keep validation
  applications small and reusable while advancing the operator/Lang sequence.

## Core language worklist

Close the remaining gaps against the [official reading map][official-docs] and
[current supported subset][source]. Existing partial support does not remove a
feature from this worklist until its applicable contracts are covered. Items
below are planned requirements, not claims that every encoding is understood.

This worklist includes language keywords and control flow independently of the
Lang method inventory; a missing construct need not block a native method to
remain required core work. The frontend grammar records current acceptance.

- **Lexical forms and literals:** reconcile reserved identifiers, comments,
  and remaining lexical contexts beyond the verified numeric and literal-escape
  profiles. Resolve raw supplementary Char truncation and unpaired String
  surrogate replacement before widening those explicit restrictions. Distinguish
  reserved but unused words such as `alias`/`native` from executable features.
- **Declarations and storage:** uninitialized/typed locals, multiple declaration
  forms, constants, named/anonymous enums, instance/module/global/static fields,
  and supported field value types. Establish default values, declaration versus
  initialization order, dependency cycles, definite initialization, lifetime,
  and access rules. Const reference bindings do not imply immutable contents.
- **Namespaces and lookup:** nested/reopened modules, classes within modules,
  qualified names, inherited lookup, static lookup, `self`/`me`, and explicit
  global `$.name` values and nested paths beyond supported function/module calls.
  Complete `using` aliases, `import` type visibility, lexical scope, shadowing, and cross-file
  resolution; keep module behavior distinct from Object inheritance.
- **Functions and calls:** untyped Any parameters/results, declared contracts,
  implicit returns/fallthrough, Void consumers, recursion, argument evaluation,
  and instance/static/module/global call forms. Complete Method construction,
  class/module receiver values, callback type syntax, lookup, and invocation
  through the shared type and core API contracts. Verify applicable VM/SDK
  argument and frame limits, including the hidden receiver.
- **Classes and construction:** complete source superclass chains,
  inherited fields/methods, overrides and signature rules, parent-qualified
  method/initializer calls, static members, and public/private/protected/hidden
  access. Resolve default/inherited initializer requirements against the SDK.
  Cover nested classes, outer-instance construction, and their distinct lookup
  rules rather than assuming captured outer members.
- **Expressions and operators:** finish valid unary/binary/conditional and
  assignment/compound/update domains, lvalue forms, precedence, associativity,
  short-circuiting, and single-evaluation/effect order. Include non-Boolean
  truth-value and operand-returning logical behavior, nullability, `has`,
  `instanceof`, and lexical `as` casts with their distinct contracts. Preserve SDK-rejected Char
  operations and expression updates as diagnosed compatibility restrictions;
  verify other forms rather than importing C semantics.
- **Control flow and scopes:** preserve enclosing
  lexical refinements when checking unexecuted loop headers, without adding dead
  edges to executable CFG proofs. Extend case operand, receiver, and type-proof
  domains with broader types and lookup; retain direct-entry initialization and
  shared facts through fall-through. Keep return/reachability analysis consistent
  as truthiness and exception handling expand.
- **Types and dynamic behavior:** support implicit Any, Void/Null distinctions,
  general unions, changing local types and joins, casts/refinement, and guarded
  member/API availability. Add typedefs, concrete class values, named/anonymous
  structural interfaces with fields/methods, enum types, callback signatures,
  general tuples, parameterized containers, and structured option Dictionaries.
  Define assignability, ambiguity, override, and checking-policy rules shared
  by source and saved IR; current fixed/integer defaults are subset policies.
- **Containers and references:** complete dynamic-sized allocation, nested and
  heterogeneous contents, general object/reference values and keys, nullable
  references/receivers, indexing and member writes, and general contextual
  construction. Close ownership, aliasing, weak-target recovery, and callback
  retention contracts alongside the [remaining Lang API inventory][core-gaps].
- **Exceptions:** implement `try`, `throw`, typed and catch-all `catch`,
  multiple handlers, rethrowing caught exceptions, and `finally`. Define handler selection, exception variable
  scope, propagation across calls, frame/stack unwinding and cleanup, and
  finally behavior on normal completion, throw, return, break, and continue.
  Cover throwing from handlers/finally, user subclasses of Lang.Exception, and
  the applicable exception methods. Extend IR/CFG, lowering, VM/EXCEPTIONS
  records, and structured reconstruction together. Fatal VM/API errors are
  distinct from catchable exceptions and need separate negative specimens.
- **Language annotations:** parse and preserve annotations at accepted
  declaration/block sites, with semantics for type checking/initialization and
  debug/release/test selection. Account for code-placement and optimizer-control
  annotations where applicable, including extended-code target/layout contracts.
  Do not silently discard annotations that affect behavior or availability.
  Background/glance annotation representation belongs here; their application
  registration, lifecycle APIs, and runtime profiles remain later SDK work.

The [core API inventory][core-gaps] owns individual Lang classes, methods,
constants, aliases, and interfaces; do not duplicate that inventory here.
Resolve reference/SDK discrepancies through focused authored specimens and
record uncertainty. Overloading or other forms the SDK rejects need diagnostics,
not an implementation copied from another language. Extend source and saved IR
in the same increment and use the core completion gate for final reconciliation.

## Later SDK and project work

Resume these areas only after the core-language/Lang completion gate; they are
not the current implementation sequence. Existing application scaffolding remains available
for core-language validation throughout.

1. **Extend native calls and callback values.** Use the declaration model for
   AppBase/View/delegate inherited members and callback contracts. Retain
   slot-type and cardinality checks for View and View/delegate results through
   saved IR. Broaden contextual
   tuple construction beyond direct callback literals only when provenance and
   shape remain provable. Reuse core object-reference, array, nullable, and Void
   contracts; add SDK-specific shapes only as required. Compare new dispatch forms
   with focused specimens. Exercise a non-null launch-state dictionary when
   an intent example is available; null-state startup does not establish it.
   Keep the console bootstrap's selected method distinct from manifest entry.
2. **Resume UI profiles after the core language phase.** Keep the input counter
   bounded while expanding core Lang support. Reuse the established core source
   inheritance, reference projection, and assignability rules. Other input
   callbacks/event objects need separate contracts and runtime evidence.
   Later, probe View transitions to verify onHide dispatch; the exit-from-update
   specimen only establishes onStop. Extend drawing methods/constants and
   device-relative layout through focused examples rather than a fixed screen size.
3. **Resolve real projects.** Implement a bounded manifest/Jungle subset into a
   deterministic build description: selected sources/resources, application
   identity and class entry, device/API target, permissions, supported products
   and languages, exclusions, and barrel dependencies with namespace/version
   requirements. Start with a source-only app and explicit target;
   unsupported resource/build features must be diagnosed. Test path bases,
   defaults, precedence, multiple Jungle files, and working-directory changes.
   Take up this resolver in the later project-build phase, using the target and
   declaration interfaces established by core language work. See
   [project-build design][builds].
4. **Generate resources and extend the PRG profile.** Start with resource strings
   and their `Rez` identities, then application name/icon resources and a small
   drawable/layout example. Keep DATA literals separate from compiled resources.
   Derive imports, method/class records, entry points, permissions, and resource
   references from resolved inputs; investigate opaque container fields as
   concrete profiles require them. Acceptance: an independently generated app
   loads a resource and displays it, with consistent manifest/resource bindings.
   Add source-linked debug/line information for diagnosing generated code.
   Extend resource resolution with properties/settings and localization;
   validate resource-to-property/string references. Add Monkey Style personality
   selection and generated `Rez.Styles` bindings when a layout example needs
   them. Keep runtime `Application.Storage` calls separate from compiled
   property defaults and settings metadata.
5. **Connect project builds to barrels and IQ export.** Reuse `ciq-package` and
   `ciq-sign`. Export a barrel from resolved source/resources and generate its
   packaged configuration; consume both archive and direct-project dependencies.
   Build selected device PRGs before creating IQ products/signatures. Acceptance:
   a code/resource barrel consumed by an app, SDK consumption of our exported
   barrel, and extraction/verification of our own-key IQ output. Broader package
   variants remain evidence-driven.


## Build scopes and validation

Model debug, release, and test selection separately from foreground,
background, and glance availability. Preserve annotations and resource scopes
through resolution; check dependencies within each execution scope rather than
treating scope annotations as ordinary file exclusions. Background/glance
entry callbacks and their constrained resource budgets need their own specimens.

Plan a simulator test build once native callbacks and Boolean results work:
include `:test` functions with `Test.Logger` parameters, require static test
members, and exclude tests from ordinary builds. Determine test discovery and
runner metadata experimentally; support barrel tests through a runnable test
application. Produce source/debug metadata for useful failures. Reuse existing
simulator helpers and add verdict checks when the test-output contract is known.

## Targets, containers, and later directions

Start with the existing SDK 9.2.0 / Forerunner 265 watch-app environment.
Watch faces, data fields, widgets, audio-content-provider apps, and
background/glance execution need their own
manifest, base-class, lifecycle, permissions, and resource-scope evidence; they
are not just alternate packaging flags. Select subsequent profiles through a
small example using the source/build features already established.

Use later complete examples to grow native support for persistent state,
asynchronous communications, sensors/location, activity recording, and
complications. These expose callbacks, permissions, and device/API availability;
select one bounded example at a time. Native controls and on-device settings
extend the UI/resource path. Companion mobile SDKs, store operations, ERA, and
profiling are integration references, not initial compiler components. Trial
and activity-filter manifest extensions remain explicit later profile work.

Revisit DATA placement and address-region rules when generated applications
expose concrete limits. The [boundary experiment][boundary] demonstrates a
`0x01000000` literal offset with increased simulator memory; the upper limit
remains unverified. Update parsing/layout/binding consistently after resolving
the shared rule, and investigate CODE/method offsets separately. Keep device
capacity/API constraints separate from VM encoding and static PRG size; see
[target constraints][constraints]. Extended code requires its own evidence.

A separate linker, LLVM, optimization, broad archive variants, and a full VM
remain deferred. Extract IR/codegen crates only when consumers or dependencies
justify it. An independent runtime may later reuse `ciq-prg`/`ciq-mc`, beginning
with headless execution and a small API subset.

## Working basis

Build an independent Rust toolchain using public documentation, reference
specimens compiled from our sources, and the official simulator. Do not
reverse Garmin's compiler/SDK implementation. Documentation establishes source
and API contracts; binary-format and dispatch conclusions require experiments.

Each extension updates the relevant specification, native and text paths,
diagnostics, and focused checks. For complete apps compare unsigned native/text
PRGs and verify own-key signing separately. Inspect console or visible behavior
as appropriate; successful encoding or a running process is not runtime proof.
Use [research conventions](research.md) and [support scripts](../scripts/README.md)
for evidence and reproduction; do not keep completed-step lists in this plan.

[builds]: specs/build/README.md#planned-project-builds-and-targets
[constraints]: specs/build/README.md#target-constraints-and-section-layout
[official-docs]: investigations/official-garmin-documentation.md
[source]: specs/frontend/README.md
[boundary]: investigations/string-address-boundary.md

[api-design]: specs/sdk/README.md#api-declarations

[core-gaps]: specs/sdk/README.md#deferred-core-methods

[weak-results]: investigations/compiler-immutable-weak.md

[char-literals]: investigations/compiler-char-literals.md

[negation]: investigations/compiler-number-negation.md

[official-language]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
