# Object-rooted source construction

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Lang.Object][object] is the root of the Monkey C
class hierarchy and dates from API 1.0.0.

**Observed:** an SDK probe accepts an implicit Object base and explicit
`extends Lang.Object`, with source initializers that omit native base calls.
The [tracked two-file specimen][example] additionally uses a Lang module alias,
independent Number fields, local aliases, Object observers, a retained Method,
and an Object-rooted comparator. Replacing all original Counter variables
leaves its Method bound to the original state (`6 + 4 = 10`); the replacement
remains at `100`. The specimen also passes Counter references through cross-file
parameters, module/self results, a retained CounterBox field, replacement, and Object
erasure. SDK/generated programs match the expected output. The original
reference remains independently usable after field replacement (`22`), while
the box refers to the replacement (`42`).

SDK DATA records carry an Object superclass and imported Object field.
The hidden constructor targets for both implicit and explicit classes contain
only `return`; source `initialize` runs separately through construction.
No new instruction or DATA record was needed for structured reconstruction.

## Import identity

The SDK symbol table distinguishes `Toybox_Lang` (`0x00800007`), short `Lang`
(`0x00800020`), and Object (`0x0080004c`). Its Object import at index `0x82`
is the pair `0x00800007 / 0x0080004c`. The reduced generated import table uses
that pair at index 2, or index 3 when a BehaviorDelegate import precedes it.
Class base references select the generated index, independently of SDK layout.

A generated specimen with a return-only hidden constructor but short Lang's
ID in its import pair crashed with `Symbol Not Found` during `newc`. Replacing
the module ID with `Toybox_Lang` made the complete specimen execute. Artifacts:
`scratch/check-root-objects-final/` (failed import) and
`scratch/check-root-objects-verified/` (accepted).
This is evidence for the observed import identity, not a rule for inferring
arbitrary module IDs from their names.

## Reproduction and limits

Build tracked sources through an ignored Jungle using the shared native-names
manifest/resources and `scripts/build-example.sh`. The [console checker][check]
validates saved stages, structured SDK/generated CODE/DATA/SYMBOLS, lossless
reconstruction, and runtime output. The initial SDK probe and its section export
are in `scratch/root-object-probe/`.

The extended reference check is in `scratch/check-root-references/`; it includes
SDK/generated structured round trips and runtime output. These probes do not
establish source superclass chains, Object
casts/refinement, or broader weak targets.
See the [class contract][spec] and [plan](../plan.md).

## Omitted initializers

2026-10-21, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Observed:** the SDK accepts Object-rooted classes without `initialize`,
including an empty class, an explicit aliased Object base, and classes with
Number-literal or nullable String fields. The extended [tracked specimen][example]
constructs these classes and uses a comparator whose initializer is omitted.
It checks inherited Object dispatch, initial field values (`7` and Null), and
independent state after mutation (`8` versus `7`). SDK and generated runtime
results are checked against [expected output](../../examples/compiler-root-objects/expected.txt).
Structured export/reassembly needs no additional opcode or DATA record.

Reproduce with the shared console checker using `compiler-root-objects`.
The initial accepted probe and structured SDK export are in
`scratch/compiler-default-initialize/`; the extended comparison uses its `check/`
directory. This does not establish default initialization for other native bases
or source superclass chains.

## Static storage and receivers

**Observed, 2026-11-03:** SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`,
minimum API 3.0.0. The independently authored
[static-members specimen](../../examples/static-members/README.md) reproduces
the successful cases; its README owns build/run commands and expected output.
No compiler or SDK implementation was inspected.

Static fields and methods occupy a separate DATA class record, referenced by
the instance class's `statics` header word and registered under the root statics
record. The instance record contains `own`, `bump`, and `initialize`; the static
record contains `literal`, `value`, `add`, `owner`, and a separate hidden
initializer. In the initial probe, these records were at DATA `0x4fc` and
`0x53e` respectively. Those offsets identify that probe, not a layout rule.

The static literal Number uses type `0x01`, value `7`; the expression-initialized
static Number starts as type `0x00`, value `0` and is assigned in the static
initializer. Static variable key low bytes are `0x40`; static method low bytes
are `0x50`, compared with `0x10` for ordinary instance methods. Hidden initializer
keys use `0x30`. These observations do not establish every flag bit's meaning.

Reading the class value alone leaves the trace at zero. Reading its static
literal then activates the expression initializer once, changing the trace to
`2`. Two allocations append `3` independently, giving `233`; they do not rerun
the static initializer. Static reads/writes and calls through instances share
the same mutable slot. A nullable uninitialized static defaults to Null.

The SDK emits class selection followed by `getsv` for direct static reads/calls,
and class selection, Symbol, value, `putv` for direct writes. Within the static
method, accesses use `getselfv` / `getself; spush; ...; putv`. Its `self` compares
unequal to the class-definition value. Methods bound to that static `self`, or
through `instance.method(:add)`, invoke successfully and update the shared slot.

Isolated source variants delimit the contract:

- Replacing `Counter.owner()` with `Counter` in
  `new Lang.Method(Counter.owner(), :add)` compiles and constructs, but invocation
  crashes with `Unexpected Type Error`. The failing CODE site is the outer
  `invoke` call, not constructor allocation.
- Printing `Counter.owner()` directly crashes with `Symbol Not Found` in native
  API conversion. The successful specimen compares identity instead; this does
  not establish inherited Object observer methods for static storage.
- Replacing the nullable `absent` declaration with
  `static var absent as Lang.Number;` is rejected as potentially uninitialized
  and not accepting Null.
- Replacing static `owner`'s body with `return own;` is rejected because the
  instance member is absent on static `self`.

**Interpretation:** class definitions, static storage, and source instances must
remain distinct receiver contexts. A static declaration must not acquire
instance-field proofs, and a class-definition Method binding must not be given
a proven static callable signature merely because the selector exists.
The [plan](../plan.md#1-declarations-namespaces-and-member-storage) owns the
remaining source/IR/lowering work. This specimen does not establish inheritance,
cross-class static cycles, or access-control behavior.

## Static constants

**Observed, 2026-11-03:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [static-constant
specimen](../../examples/compiler-static-constants/README.md) produces matching
[SDK/generated console output](../../examples/compiler-static-constants/expected.txt)
through the shared specimen checker. Static Array/Method references survive
two allocations and retain shared contents; conditional initialization runs
once. The trace is `235` after first access and `23544` after the two instance
allocations. The SDK also accepts an added unannotated `static const EMPTY;`
declaration without an initializer.

A nested `Nested.Settings` initializer calling `Nested.select()` exposed the
statics record's lexical parent requirement. SDK DATA puts both the instance
definition and statics record under the runtime `Nested` module. Generated
statics initially parented to globals failed with `Symbol Not Found` during
that helper lookup. Using the containing module as parent made the generated
application match the SDK, including the final `10`. This applies to the
shared class-enum storage path too.

The SDK uses String DATA defaults directly for `MESSAGE`; our current profile
writes it in the statics hook before runtime expressions. The matched trace
establishes the specimen's observable order and identity, not identical SDK
instruction selection or static-initialization cycle behavior. Current
contracts and conservative readiness restrictions belong in the
[frontend spec](../specs/frontend/README.md#static-constants).

## Mutable static fields

**Observed, 2026-11-04:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [mutable-static
specimen](../../examples/compiler-static-fields/README.md) passes the shared
SDK/generated console comparison and structured reconstruction. Its
[expected output](../../examples/compiler-static-fields/expected.txt) captures
the distinctive ordering and lifetime observations.

`Cold.VALUE = Trace.mark(1)` evaluates the RHS before first static initialization:
the trace becomes `12`, and the stored value is `1`. In contrast,
`Warm.VALUE += Trace.mark(7)` initializes/read-captures the old `6` first,
yielding `13` and trace `1267`. A later compound RHS overwrites `Cold.VALUE`
with `50`, but the compound result still uses the captured old value and becomes
`4`. Subsequent instance reads/writes see the same slot. Two allocations append
their independent `4` markers without rerunning statics. Replacing a static
Array through a branching assignment expires its old weak reference. An
unannotated Number-initialized static accepts String replacement; nullable and
nested-module fields also match the SDK output.

These observations establish the tracked assignment profile, not static-method
receiver behavior or unrestricted initialization cycles. Current contracts and
conservative readiness checks belong in the
[frontend spec](../specs/frontend/README.md#mutable-static-fields).

## Static method contexts

**Observed, 2026-11-04:** SDK/simulator 9.2.0, `fr265`, `-O 0`, minimum API
3.0.0, target 5.2.0. The [static-method
specimen](../../examples/compiler-static-methods/README.md) matches all 17
[expected SDK/generated output lines](../../examples/compiler-static-methods/expected.txt)
through the shared specimen checker, including structured reconstruction.

An initial `new Counter()` only marks instance allocation: the trace becomes
`4`. First static method access then runs the statics hook, which marks `2`
and calls static `seed()` to mark `3`; later allocations produce trace `42344`
without rerunning statics. This isolates a limit the earlier specimens did not:
allocation alone does not activate static storage. Instance initialization can
activate it indirectly by accessing a static member.

For static `owner(){ return self; }`, calls through two instances return
different receivers. `first.owner() == first` is true, while comparison with
the class value and with the statics hook's stored `OWNER=self` is false.
The earlier direct `Counter.owner()` probe instead returns static storage.
Thus a static declaration alone does not determine the physical receiver;
lookup/invocation context matters. The compiler's opaque receiver view retains
identity without supplying instance-member evidence.

Instance `method(:add)`, a Method constructed from a static body's `self`, and
a Method constructed from the hook's stored receiver all update the same
static slot. A method-only class needs a separate statics record even without
shared fields. A static method in `Nested.Settings` reaches an enclosing-module
helper through the statics record's parent. These cases require existing member
lookup/invocation and `getself`, not a new opcode or DATA record kind.

The [frontend contract](../specs/frontend/README.md#static-methods) defines the
checked member scope. The matched specimen does not establish Object inheritance
for every possible static receiver.

The [class-call specimen](../../examples/compiler-class-calls/README.md) matches
all 19 SDK/generated lines with structured reconstruction under the same profile.
`Cold.add(Trace.mark(3))` first initializes static storage (trace `2`), then
runs the argument (trace `23`); captures preserve this order across conditional
and constructed arguments. Aliases/root paths reach the same static declarations.
Direct `owner()` equals the hook's stored receiver, but differs from the class
definition; calling through an instance instead returns that instance.

Separate authored probes returned static `self` with Object and source-class
annotations, both implicitly and via explicit casts. SDK compilation and
execution succeed. On direct class calls, `instanceof` the annotated type is
false; through instances it is true. Thus accepted annotations do not prove
physical instance identity. The checked specimen exercises implicit Object/source-class returns through a
local receiver alias and the stored `OWNER` constant. Returning stored `OWNER`
fails `instanceof Cold` for both class and instance dispatch, while returning the
current `self` passes for instance dispatch only. Origin-aware opaque contracts
allow these annotations without treating an unannotated alias as an instance.

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-root-objects/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#object-rooted-classes
