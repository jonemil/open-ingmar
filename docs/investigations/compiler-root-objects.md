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

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-root-objects/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#object-rooted-classes
