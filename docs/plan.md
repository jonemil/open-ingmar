# Plan

## Next work

Keep complete runnable applications as the validation path, while expanding the
language and build layers in small steps. Language completeness is not a
prerequisite for project resolution; package writing is not a substitute for
compiling a project.

1. **Extend core Lang contracts and value types.** Extend Array elements to
   native object values, infer literal elements beyond Char (including
   contextual types for empty literals), and support
   ordinary object construction required by source `getInitialView`.
   Keep generic signatures and invariant element
   checking shared across source and saved IR. Comparator sorting and other
   Array/Object methods need separate receiver and runtime evidence.
   Revisit direct array filling once initialized-element contracts and memory
   behavior are established; the current literal path uses typed append.
   Keep positive signs separate
   until their contracts are established. Investigate the SDK's
   [negative-literal postfix discrepancy][negation] before widening that syntax.
   Revisit Long/Float/Double values through a useful runnable example.
   Investigate the SDK's
   [supplementary literal discrepancy][char-literals] before expanding BMP source
   literals. Follow the
   [class-focused strategy](#native-lang-strategy) for
   subsequent groups; revisit deferred [Number][number-methods],
   [String][string-methods], and [Boolean][boolean-methods] methods as their
   prerequisites become available.
   Keep ordinary object weak references separate from the verified
   [immutable-value specialization][weak-results]; probe lifetime and nullable
   dereference contracts when object ownership requires them.
   Retain branch-proof validation through saved IR, joins, loops, and writes.
   Extend supported comparisons without trusting frontend-only assumptions;
   support nullable Boolean when a runnable case requires it.
   Grow Object equality and other core methods through documented assignability
   rules rather than weakening argument validation to accept any value.

   Connect the explicit API context to verified device/app inputs as project
   resolution develops. Add app type, permission, scope, and VM constraints
   when supported calls need
   them; keep unknown constraints explicit. Plan `has`-guarded optional APIs
   before claiming portable references newer than the app minimum. Migrate
   remaining Application/System special cases as they acquire supported call
   forms. See [API declaration design][api-design].

   Read the language reference alongside `Lang` contracts when extending
   operator rules. Grow numeric conversions, mixed arithmetic, string addition,
   and equality through focused specimens; do not attempt all core operators in
   one increment. Preserve native/text pipeline equivalence and complete-app
   simulator validation as new call contracts and operators are added.

   Keep the app's minimum API and VM capabilities distinct from device/API
   selection. A full Jungle driver and API definition parser are not
   prerequisites for extending the bounded native profile.
2. **Extend native calls and callback values.** Use the declaration model for
   AppBase/View inherited members and callback contracts. Extend object-reference,
   array, and nullable values as required,
   retaining `Void` as a result contract. Start the View path with one
   constructed native View in the array returned by source `getInitialView`.
   Compare new dispatch forms with focused specimens.
   Acceptance: a source `getInitialView` returns a constructed View under its
   resolved result contract. Exercise a non-null launch-state dictionary when
   an intent example is available; null-state startup does not establish it.
   Keep the console bootstrap's selected method distinct from manifest entry.
3. **Compile source-authored lifecycle and a visible View.** Add construction,
   source/native inheritance, instance fields, ordinary object-member calls,
   array literals, and object/array returns as required by `getInitialView()`.
   Resolve AppBase/View callback signatures, including inherited annotations,
   rather than assigning every parameter our current integer contract. Replace
   generated lifecycle methods only when the source supplies validated methods;
   keep the console bootstrap available. Acceptance: a multi-file source app
   constructs its own View, draws text through `onUpdate(dc)`, and observes
   startup, view, and shutdown behavior in the simulator.
   Follow with a View/delegate pair whose input handler updates a counter and
   returns whether it handled the event. This exercises mutable fields and
   Boolean callback results; select an event supported by the target.
4. **Resolve real projects.** Implement a bounded manifest/Jungle subset into a
   deterministic build description: selected sources/resources, application
   identity and class entry, device/API target, permissions, supported products
   and languages, exclusions, and barrel dependencies with namespace/version
   requirements. Start with a source-only app and explicit target;
   unsupported resource/build features must be diagnosed. Test path bases,
   defaults, precedence, multiple Jungle files, and working-directory changes.
   This resolver work can proceed alongside language work after the target and
   declaration interfaces are clear. See [project-build design][builds].
5. **Generate resources and extend the PRG profile.** Start with resource strings
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
6. **Connect project builds to barrels and IQ export.** Reuse `ciq-package` and
   `ciq-sign`. Export a barrel from resolved source/resources and generate its
   packaged configuration; consume both archive and direct-project dependencies.
   Build selected device PRGs before creating IQ products/signatures. Acceptance:
   a code/resource barrel consumed by an app, SDK consumption of our exported
   barrel, and extraction/verification of our own-key IQ output. Broader package
   variants remain evidence-driven.

After each dependency is established, take the next language feature from the
backlog below that a representative application needs. Do not wait for complete
language coverage before advancing a working project example.

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
  Revisit deferred methods when their dependencies exist. Keep application and
  project work moving without waiting for complete Lang coverage.

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
  constants/enums, lexical/member lookup, bare calls, qualified names,
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

Each extension updates the relevant design/spec, native and text paths,
diagnostics, and focused checks. For complete apps compare unsigned native/text
PRGs and verify own-key signing separately. Inspect console or visible behavior
as appropriate; successful encoding or a running process is not runtime proof.
Use [research conventions](research.md) and [support scripts](../scripts/README.md)
for evidence and reproduction; do not keep completed-step lists in this plan.

[builds]: design/project-builds.md
[constraints]: design/project-builds.md#target-constraints-and-section-layout
[official-docs]: investigations/official-garmin-documentation.md
[source]: specs/compiler-subset.md
[boundary]: investigations/string-address-boundary.md

[api-design]: design/api-declarations.md

[number-methods]: investigations/compiler-number-methods.md#method-inventory

[string-methods]: investigations/compiler-string-methods.md#method-inventory

[boolean-methods]: investigations/compiler-boolean-methods.md#method-inventory

[weak-results]: investigations/compiler-immutable-weak.md

[char-literals]: investigations/compiler-char-literals.md

[negation]: investigations/compiler-number-negation.md
