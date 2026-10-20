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
erasure. SDK/generated programs match all 16 expected lines. The original
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
establish automatic default initializers, source superclass chains, Object
casts/refinement, or broader weak targets.
See the [class contract][spec] and [plan](../plan.md).

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html
[example]: ../../examples/compiler-root-objects/README.md
[check]: ../../scripts/README.md#console-specimen-check
[spec]: ../specs/frontend/README.md#object-rooted-classes
