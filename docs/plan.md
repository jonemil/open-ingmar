# Plan

## Next work

Keep implementation focused on operators and `Toybox.Lang` objects and their
methods before returning to broader SDK classes. Reuse the existing console
apps and [input counter](../examples/compiler-input-counter/README.md) to validate
core changes. New UI classes, event objects, drawing APIs, and broader SDK
modules belong to a later phase. Project/resource/package work stays in the
backlog while these language foundations are the priority.

Choose increments by dependency, following the
[class-focused strategy](#native-lang-strategy):

1. **Dictionary.** Enable `keys` and `values` for the supported key/value
   groups using the Array contracts. Widen beyond Number/String values
   alongside nullable Boolean/Char core types; keep heterogeneous and nullable
   results explicit. Inherited Object methods need Dictionary-specific evidence.
   Follow the [source contracts](specs/compiler-subset.md#dictionary-values) and
   [remaining extensions](specs/compiler-subset.md#planned-dictionary-extensions),
   using the [investigation](investigations/compiler-dictionary.md) for evidence.
   Nullable Array elements currently need explicit context; broader literal
   inference can follow once mixed scalar/Null contracts are established.
2. **Numeric types and deferred methods.** Add Long/Float/Double values,
   literals, conversions, arithmetic/promotion, and comparisons in bounded
   groups. Revisit Number/String/Boolean/Char methods as their required value
   types become available.
3. **Object and callable/reference facilities.** Grow shared Object contracts,
   Symbol and Method values/invocation, and ordinary WeakReference consumers.
   Use these prerequisites to revisit deferred core methods, including Array
   comparators. Plan `has`/`instanceof` and their checking/refinement rules.

Array sorting remains deferred until comparator/callable support. Inherited
Array equality, hashing, and ordinary weak references need array-specific
evidence in the Object-contract phase; the immutable weak specialization does
not establish them.

A blocked method should lead to its core type/operator/callable prerequisite or
another core class group. Retain explicit deferred-method inventories; declaring
an API member does not establish executable support. Reassess remaining core
gaps before changing focus to the later SDK/project work below.

Investigate the simulator process failure for minimum Number / -1 and % -1
separately before claiming portable boundary behavior or exception recovery.

### Core contracts and evidence

Keep general unions and non-Boolean truthiness deferred, as scoped by the
[operator review](investigations/compiler-operator-coverage.md). Keep updates
statement-only following the [SDK evidence](investigations/compiler-increments.md),
and extend numeric update contracts alongside their value types.

Keep generic signatures, invariant container elements, assignability, and
nullable branch proofs shared across source and saved IR. Extend constructor
and object-reference contracts as core classes need them. Comparator calls and
ordinary object weak references need their own dispatch/lifetime evidence;
keep them distinct from the verified [immutable weak specialization][weak-results].
Keep semantic operations explicit enough to select established VM instructions,
following the [lowering review](investigations/natural-lowering.md). Preserve
private initialization for effectful/branching array literals and typed DATA
constants for scalar literals. Baseline instruction selection should preserve
evaluation order and failure behavior without optimization or requiring the
SDK's exact layout. Validate native/text paths and structured reconstruction.

Revisit deferred [Number][number-methods], [String][string-methods], and
[Boolean][boolean-methods] methods as their prerequisites become available.
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

Organize native Lang work around one class at a time. The compiler implements
contracts, availability checks, and dispatch; the runtime supplies method bodies.
Completing every method of a class is not a prerequisite for moving on.

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

## Language and API backlog

The [official reading map][official-docs] identifies documented language features
beyond our [supported subset][source]. These are work areas, not promises that
all their encodings or runtime behavior are understood:

- **Values and expressions:** full signed Number literals, numeric widths and
  conversions, Long/Float/Double/Char, Boolean/null values, symbols, unary,
  arithmetic, relational, bitwise, logical/short-circuit, conditional, and
  compound-assignment operators. Confirm precedence, overflow, evaluation order,
  and immediate encodings with focused specimens.
- **Declarations and lookup:** global/module variables and initialization,
  the complete reserved-keyword set (including `alias` and `typedef`),
  constants/enums, lexical/member lookup, bare calls (including the
  [audited SDK examples](investigations/example-toolchain-audit.md#frontendproject-limits)),
  qualified names,
  `using` aliases and `import` type visibility. Add field/static initialization,
  access modifiers, overriding, and source superclass dispatch as the object
  model grows.
- **Containers and control flow:** array allocation/indexing/writes, dictionary
  literals/lookups/writes, general assignment targets, `for`, `do/while`,
  `switch`, and their scopes and break targets. Native callbacks may first pass
  an opaque dictionary without requiring all dictionary operations.
- **Types and dynamic behavior:** `as` declarations/casts, `Any`, unions,
  nullability, container/tuple/callback types, flow refinement, and changing
  local types. Decide a checking policy explicitly; our fixed local types and
  integer-only arguments are subset restrictions. Extend typed IR and text
  together without making full gradual checking the first milestone.
- **Exceptions and callbacks:** `throw`, typed/catch-all handlers, `finally`,
  symbol-based method references, and callback invocation. Exception lowering
  needs unwinding/stack/frame evidence even though table assembly exists.
  Separate recoverable exceptions from fatal VM/API errors; investigate weak
  references when object ownership or callback examples require them.
- **Toybox compatibility:** grow native descriptors from public contracts and
  controlled specimens: types/signatures, constants, inheritance, API levels,
  supported devices/app types, and permissions. Distinguish source type checking
  from executable binding. Add `has`/`instanceof`, runtime availability checks,
  annotations, and execution scopes before relying on them in portable projects.
  Implement API call compilation, not a replacement implementation of every API.

Validate the documented method-argument limit, hidden-receiver accounting, and
SDK/target applicability before widening call/frame limits. Existing opcode
support does not establish source-level semantics or acceptance at a boundary.

## Later SDK and project work

Resume these areas after the operator/core-Lang phase; they are not the current
implementation sequence. Existing application scaffolding remains available
for core-language validation throughout.

1. **Extend native calls and callback values.** Use the declaration model for
   AppBase/View/delegate inherited members and callback contracts. Retain
   slot-type and cardinality checks for View and View/delegate results through
   saved IR. Broaden contextual
   tuple construction beyond direct callback literals only when provenance and
   shape remain provable. Extend object-reference, array, and nullable values as
   required, retaining `Void` as a result contract. Compare new dispatch forms
   with focused specimens. Exercise a non-null launch-state dictionary when
   an intent example is available; null-state startup does not establish it.
   Keep the console bootstrap's selected method distinct from manifest entry.
2. **Resume UI profiles after the core language phase.** Keep the input counter
   bounded while expanding core Lang support. Widen named source signatures and
   reference-field targets beyond View only with checked native-base projection
   and assignability through saved IR. Source inheritance and other input
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

[builds]: specs/project-builds.md
[constraints]: specs/project-builds.md#target-constraints-and-section-layout
[official-docs]: investigations/official-garmin-documentation.md
[source]: specs/compiler-subset.md
[boundary]: investigations/string-address-boundary.md

[api-design]: specs/api-declarations.md

[number-methods]: specs/api-declarations.md#deferred-core-methods

[string-methods]: specs/api-declarations.md#deferred-core-methods

[boolean-methods]: specs/api-declarations.md#deferred-core-methods

[weak-results]: investigations/compiler-immutable-weak.md

[char-literals]: investigations/compiler-char-literals.md

[negation]: investigations/compiler-number-negation.md
