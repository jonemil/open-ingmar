# Plan

## Next work

Priority order:

1. [Language inference, unions, and storage](#1-inference-unions-and-storage).
2. [Declarations, namespaces, and member storage](#2-declarations-namespaces-and-member-storage).
3. [Source inheritance and construction](#3-source-inheritance-and-construction).
4. [Operators, calls, and control flow](#4-operators-calls-and-control-flow).
5. [Toybox.Lang](#toyboxlang-worklist), with classes ordered within the module.
6. [Exception handling](#exceptions).
7. [Lexical compatibility and annotations](#lexical-compatibility-and-annotations).

Items within each scope are also prioritized. Implement a named prerequisite
before its consumer, and batch related items when they share implementation and
validation. Shared language/type machinery should serve later class work rather
than be reimplemented per class; class-specific signatures, bindings, API checks,
and runtime validation still need their own coverage.

Aim for a coherent proposed commit covering as much related functionality as can
be implemented and validated together. Follow priorities while grouping shared
prerequisites and their consumers; do not split a ready class/method group into
tiny increments or add unrelated work to enlarge a change. The user makes the
commits. Broader SDK modules and project work wait until this core list is empty.

This is the single core backlog. It lists remaining work, not current
capabilities. Delete each completed item after the
[completion gate](#core-completion-gate) passes; delete empty subsections too.
Do not retain completed items as checked boxes (`[x]`).
For partial completion, remove the finished part and leave only the specific
remainder. Add a new item only for an identified missing behavior or a named
unresolved experiment, and place it at its actual priority. Specs describe
implemented contracts; investigations retain evidence, not another backlog.

## Core language worklist

### 1. Inference, unions, and storage

- [ ] Establish SDK partial-match rules for class-containing unions (`A or B`
  passed/returned as `A`), structural method signatures, and parameterized
  container references (`Array<Number or String>` as `Array<Number>`); implement
  accepted boundaries without losing source witnesses or mutable alias rules.
- [ ] Probe and support SDK partial operator/index/member-receiver cases:
  Number-or-String subtraction, Number-or-String Array indices, and a union
  whose alternatives do not all expose the invoked member. Preserve API,
  arity, stack, and metadata checks; keep strict rejection of uncertainty.
- [ ] Implement disabled type checking with dynamic lowering for otherwise
  mismatched operations; preserve declaration, ABI, API, and format checks.
- [ ] Represent SDK-accepted recursive contracts through annotations, saved IR,
  and value operations: container-guarded aliases such as
  `typedef Tree as Array<Tree>` and interfaces such as
  `typedef Recursive as interface { function next() as Recursive; };`.
  Container recursion requires nested Array/Dictionary contracts below;
  unguarded alias cycles remain errors.

### 2. Declarations, namespaces, and member storage

- [ ] Parse `const` declarations and bind constant references; reject rebinding
  while allowing mutation of referenced containers.
- [ ] Parse named/anonymous `enum` declarations, implicit/explicit values, member
  lookup, and enum annotations.
- [ ] Support global and module `var` fields, typed/untyped defaults, reads, and
  writes; implement initialization order and diagnose initialization cycles.
- [ ] Support scalar/String/container and computed instance-field declaration
  initializers for typed and inferred fields; infer declaration types beyond
  Number/Symbol defaults and preserve per-instance evaluation order.
- [ ] Resolve bare field names in instance methods with the SDK's local,
  parameter, field, and outer-owner shadowing rules.
- [ ] Support nested module declarations and qualified names through multiple
  module levels; reuse reopened-module collection across files.
- [ ] Support classes declared inside modules and owner-qualified class names
  in annotations, construction, and runtime type targets.
- [ ] Resolve `using` aliases and `import` visibility for source namespaces and
  their classes/types, including conflicting aliases and cross-file lookup.
- [ ] Implement `me` and owner-relative `self` in module/global contexts after
  checking their distinct SDK meanings; preserve instance receiver identity.
- [ ] Support root-object `$.name` field/value access and nested qualified paths
  beyond the existing global and one-module call forms.
- [ ] Represent class and module values so they can be stored, passed, returned,
  and used as Method receivers.
- [ ] Support static fields/methods, their initialization, and class-qualified
  access; distinguish static owners from instance receivers.
- [ ] Enforce `public`, `private`, `protected`, and `hidden` on declarations and
  inherited/member access, including access across source files.

### 3. Source inheritance and construction

- [ ] Resolve source superclass chains and reject cycles; retain assignable
  superclass/subclass identities in source and saved IR.
- [ ] Lay out inherited fields, resolve inherited methods, and dispatch overrides
  using the actual instance's class.
- [ ] Inherit undecorated override signatures and check explicitly decorated
  overrides for arity, argument, and result compatibility.
- [ ] Support superclass-qualified method and initializer calls, including their
  receiver, argument order, and field-initialization effects.
- [ ] Establish SDK default/inherited initializer rules for View,
  BehaviorDelegate, and source subclasses, then implement accepted forms.
- [ ] Support nested classes and outer-instance construction/member lookup;
  distinguish an explicit outer receiver from captured lexical names.
- [ ] Implement SDK-valid direct `new Lang.Object()` construction; determine its
  initialization/metadata contract with an authored specimen.
- [ ] Permit receiver exposure, branches, and early exits in initializers when
  required-field initialization is proven on every relevant path, replacing the
  current mandatory straight-line initialization prefix.

### 4. Operators, calls, and control flow

- [ ] Implement non-Boolean conditions for `if`, `while`, `for`, `do`, and `?:`;
  probe truth values for Null, zero/nonzero numbers, and reference values.
- [ ] Implement operand-returning `&&`/`and` and `||`/`or` for non-Boolean values,
  with short-circuit evaluation and the resulting union contracts.
- [ ] Probe unsupported operator spellings/forms (`>>>`, logical/unsigned-shift
  compounds, and assignment expressions); implement SDK-accepted cases and
  document explicit rejection for the others.
- [ ] Support reference `==`/`!=` and String concatenation with object/container
  operands where accepted by the SDK; preserve runtime equality/conversion.
- [ ] Support runtime source-subclass targets and union refinement for
  `instanceof`; verify which interface/parameterized targets the SDK accepts.
- [ ] Make `has` refine guarded member/API availability while keeping member
  existence separate from callable signature proof.
- [ ] Preserve enclosing lexical refinements while checking unreachable loop
  updates and post-tests; accept unassigned operands in provably unexecuted
  headers where the SDK does, without inventing a runtime value. Keep those
  checks out of executable CFG edges.
- [ ] Accept unbraced control bodies and standalone empty statements where the
  SDK permits them; verify block/loop/switch declaration scope and shadowing.
- [ ] Extend switch case equality and `case instanceof` to the new union,
  source-inheritance, and receiver types; preserve fall-through facts.
- [ ] Support calls through lexical callback values and source/module/class
  callable members, rather than resolving every bare call to a declaration.
- [ ] Probe call argument/frame limits, including the hidden receiver and
  Method.invoke; reconcile the current 254-argument profile with SDK/runtime
  limits and emit diagnostics at the applicable boundary.

## Toybox.Lang worklist

Class sections are in priority order. Shared language work is not repeated
for every affected class. Inherited methods belong to their declaring class's
items and apply to each newly supported receiver type. An implemented own
method is not listed again merely because its class is not yet unrestricted.
API availability, arguments/results, and storage/dispatch must work together for
all items. The [public API review](investigations/core-lang-completion.md) records
inventory provenance; the [frontend spec](specs/frontend/README.md) owns current
executable profiles.

### Object

- [ ] Dispatch `toString`, `equals`, and `hashCode` on source-subclass,
  Exception, and ResourceId values using their
  resolved runtime contracts; preserve overrides and appropriate Null checks.
- [ ] Extend `method(Symbol)` to inherited/static and module/global members;
  resolve lexical conflicts using ordinary owner/member lookup.
- [ ] Support `weak()` on source instances and opaque Object values, including
  the possible immutable-scalar result instead of always claiming WeakReference.

### Method

- [ ] Implement `initialize(aClass, aMethod)` / `new Lang.Method(receiver, symbol)`
  for supported instance, class, and module values, retaining the receiver.
- [ ] Diagnose missing literal method targets; preserve distinct signatures when
  a source-class union binds a selector with different target contracts.
- [ ] Represent variadic bound-method signatures and infer native lifecycle
  callback bindings, including the actual initial View tuple result.

### Comparable

- [ ] Permit an inherited source `compareTo(Object) as Number` to establish
  the structural witness, including after saved-IR reload.

### Comparator

- [ ] Permit an inherited source `compare(Object, Object) as Number` to establish
  the structural witness, including after saved-IR reload.
- [ ] Resolve the published nullable Comparator arguments versus SDK rejection
  of explicitly nullable `compare` parameters; use the verified signature
  consistently in declarations, source/saved IR, and `Array.sort`.

### Array

- [ ] Support sorting nullable elements through a checked custom comparator,
  after resolving the Comparator signature discrepancy above.
- [ ] Check multidimensional `new [sizes]` and native Array constructor syntax;
  implement accepted forms with independent nested allocations.

### Dictionary

- [ ] Support general Object, exact source-class, container, and Method keys
  through literals, indexed access, `get`, `put`, `hasKey`, `remove`, and `keys`;
  preserve runtime hashing/equality and key identity.
- [ ] Support Null keys if SDK/runtime accepts them; determine typed-key rules
  and missing-value behavior with isolated specimens.
- [ ] Preserve exact source-class values through storage, `get`, and `values`.
- [ ] Support explicit nested key/value parameters and nested Dictionary/Array
  literals; retain inner contracts through lookup, writes, and enumeration.
- [ ] Infer empty, Any-valued, and heterogeneous Dictionary literals without
  immediate context under the language's changing-local/union rules.
- [ ] Support indexed compound assignment and numeric updates with single
  receiver/key evaluation and a proven non-null lookup value.
- [ ] Establish and implement SDK-valid `new Dictionary()` construction.
- [ ] Resolve mixed Number/Char literal keys under Comparable/Comparator
  contracts: compare SDK constant construction with runtime insertion and remove
  the current mixed-token restriction only with preserved hash/equality behavior.

### ByteArray

- [ ] Probe Number/Char union Array inputs to `addAll`; implement SDK-accepted
  combinations beyond the current homogeneous profiles.

### WeakReference

- [ ] Recover source-class and parameterized-container contracts from `get()`
  using explicit casts/runtime tests; keep Null and liveness checks independent.
- [ ] Validate `get()`/`stillAlive()` and receiver lifetime for the broader
  Object.weak targets, including expiry while only a weak handle remains.

### ResourceId

- [ ] Declare and bind ResourceId values from a minimal authored resource fixture;
  carry the identity through signatures, storage, and saved IR without starting
  general resource generation.
- [ ] Implement `toString()` and test the inherited Object methods on those
  values with SDK/generated comparison.

### Exception

- [ ] Declare/construct `Exception.initialize()` with its API introduction level.
- [ ] Implement `getErrorMessage()` and `printStackTrace()` with their nullable
  String/Void contracts and API levels.
- [ ] Support source subclasses, superclass initialization, and inherited
  exception methods; retain the runtime class for typed handler matching.

Each remaining native exception subclass needs its declared base/API level,
constructor binding, inherited methods, and typed catch behavior. The shared
Exception implementation supplies the inheritance and dispatch machinery.

### InvalidOptionsException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### InvalidValueException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### OperationNotAllowedException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### SerializationException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### StorageFullException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### SymbolNotAllowedException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

### UnexpectedTypeException

- [ ] Implement `initialize(msg as String, unused1 as Object or Null,
  unused2 as Object or Null)`, retaining all three arguments and inherited
  exception behavior.

### ValueOutOfBoundsException

- [ ] Implement `initialize(msg as String)` and its inherited exception contract.

## Exceptions

Requires the Lang.Exception constructor and subclass contracts
[above](#exception).

- [ ] Parse and lower `throw`, typed `catch`, and catch-all handlers; implement
  first-matching handler selection and catch-variable scope.
- [ ] Propagate exceptions through source/native calls, unwind frames/stack
  temporaries, and support rethrowing a caught exception.
- [ ] Implement `finally` on normal completion, throw, return, break, and
  continue; handle exceptions raised by a handler or finally block.
- [ ] Generate handler ranges and transfer/cleanup code from exception IR;
  establish protected-range endpoint semantics using the existing `throw`
  opcode and ordinary EXCEPTIONS reader/writer.
- [ ] Add negative specimens distinguishing catchable Lang exceptions from fatal
  VM/API errors, without treating fatal errors as recoverable handlers.

## Lexical compatibility and annotations

- [ ] Match the SDK's raw supplementary Char literal truncation and unpaired
  String-surrogate replacement, replacing the current explicit restrictions.
- [ ] Resolve negative-literal postfix parsing against the SDK and remove the
  current parentheses requirement for SDK-valid forms.
- [ ] Check reserved/contextual identifiers at declaration, member, selector,
  and alias sites; classify `alias`/`native` with explicit accepted/rejected
  specimens instead of reserving unverified executable features.
- [ ] Check unterminated/nested-comment diagnostics and accepted identifier
  character ranges against the SDK.
- [ ] Parse and retain annotations at SDK-accepted declaration/block sites.
- [ ] Apply `:debug`, `:release`, and `:test` source selection; preserve custom
  annotation symbols in the appropriate debug metadata.
- [ ] Implement `:typecheck` and `:initialized` checking semantics.
- [ ] Preserve `:background`/`:glance` code availability and diagnose invalid
  cross-scope references; runtime lifecycle registration stays in later SDK work.
- [ ] Implement `:extendedCode` placement/API/target checks and the corresponding
  CODE/PRG representation using a controlled SDK specimen.
- [ ] Preserve runtime lookup for constants decorated with
  `:optimizer(do_not_remove)`; no optimizer implementation is required.

## Core completion gate

Remove an item only when its accepted syntax, type/API checks, storage, IR/text,
lowering, and affected binary formats pass
[toolchain completion](#toolchain-completion) and SDK/generated behavior agrees.
For a probe item, an SDK rejection may close the question with a documented
restriction and diagnostic test. A blocked item stays here with its specific
prerequisite; it is not moved to the later SDK backlog. Do not retain completed
checkboxes or replace a completed task with a generic obligation to extend it.

Broader SDK/project work starts when every core scope is empty. Verify that
every entry in the public Lang index, including inherited members and supported
API versions, is accounted for before removing the final item; newly identified
gaps become specific checklist entries.

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

Work on a class's remaining items together when they share prerequisites and a
complete-app specimen. Native firmware supplies method bodies; this compiler
checks contracts, binds references, and emits calls. Reuse the shared validation
helpers and batch final checks for the group. Put a discovered prerequisite in
the ordered checklist once, at its owning language/class scope.

## Later SDK and project work

Resume these areas only after the core-language/Lang completion gate; they are
not the current implementation sequence. Existing application scaffolding remains available
for core-language validation throughout.

1. **Extend native calls and callback values.** Use the declaration model for
   AppBase/View/delegate inherited members and callback contracts. Retain
   slot-type and cardinality checks for View and View/delegate results through
   saved IR. Reuse core object-reference, array, nullable, and Void
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

Build an independent toolchain in Rust using public documentation, reference
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
[boundary]: investigations/string-address-boundary.md
