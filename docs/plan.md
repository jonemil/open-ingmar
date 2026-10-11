# Plan

## Next work

Keep complete runnable applications as the validation path, while expanding the
language and build layers in small steps. Language completeness is not a
prerequisite for project resolution; package writing is not a substitute for
compiling a project.

1. **Connect core types and API declarations.** Extend the bounded catalogue
   in `ciq-compiler` where current language behavior requires it. Connect internal
   value types and operator validation to its Lang identities, then use inherited
   member lookup in source resolution. Keep native lowering support explicit:
   known declarations alone do not make calls compilable. Migrate remaining
   Application/System special cases before expanding native support.
   See [API declaration design][api-design].

   Connect current integer/string/null behavior to those identities. Read the
   language reference alongside `Lang` contracts to define operator operand and
   result rules separately from method signatures and VM lowering. Grow numeric
   conversions, mixed arithmetic, string addition, and equality through focused
   specimens; do not attempt all core types/operators in one increment.
   Acceptance: existing examples retain their behavior through native and text
   paths; inherited lookup and class/member availability have focused checks.
   Diagnose missing catalogue knowledge separately from known unavailable APIs
   and declarations whose lowering is unsupported.

   Define explicit target context alongside availability: selected device/API,
   app type, permissions, and execution scope. Keep the app's minimum API and VM
   capabilities distinct. Plan guarded optional references through `has` before
   claiming compatibility across API levels. A full Jungle driver and API
   definition parser are not prerequisites.
2. **Extend native calls and callback values.** Use the declaration model for
   AppBase/View inherited members and callback contracts. Extend Boolean,
   object-reference, array, and nullable values as required, retaining `Void`
   as a result contract. Compare new dispatch forms with focused specimens.
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
