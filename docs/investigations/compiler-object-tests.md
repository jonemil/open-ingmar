# Runtime object predicates

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.
Question: how do authored `instanceof` and `has` expressions execute and encode?

**Experimentally observed:** the [tracked specimen][example] matches all 35
expected lines in SDK/generated runs. It tests all supported concrete Lang
classes, source field/method existence, inherited observers, Symbol parameters,
computed selectors, null guards, and Boolean short-circuiting. Computed operands
print `receiver` before `selector`; each appears once.

An initial SDK probe emits `lgetv 1; getmv 0x00800007 0x00800096; 21` for an
Object argument tested against Number. String selects `0x0080009b`; member tests
use `lgetv 1; spush <symbol>; 22`. These single-byte opcodes have no immediates.
The initial export identified unsupported opcode `0x21` at CODE+`0x8f`.
Adding the two encodings allows structured CODE and exact reconstruction.
No new DATA record was encountered. Lang's module selector is `Toybox_Lang`
(`0x00800007`), as in the [Object import evidence][roots].

The same probe returns false for `null instanceof Lang.Number`, then crashes
with `Null Reference Error` at `null has :toString`. The accepted specimen guards
the latter operation. Our non-null receiver requirement is a checking policy,
not a claim that the SDK rejects nullable source expressions.

**Limits:** these results establish predicates, not type refinement, casts,
source-class tests, interface tests, or guarded API availability. A preliminary
`1.0f` literal is SDK-accepted but unsupported by our parser; the final specimen
uses unsuffixed Float syntax. The lexical gap remains in the plan.

## Reproduction

Use the example's compiler command and the shared [console checker][check].
The ignored `scratch/compiler-object-tests/monkey.jungle` selects the tracked
source and shared native-names manifest/resources. Build through
`scripts/build-example.sh`, then prepare and simulate `compiler-object-tests`.
Accepted saved-stage, SDK/generated round-trip, and runtime artifacts are under
`scratch/check-object-tests/`. The isolated nullable failure is under
`scratch/object-tests-sdk-runtime-full/`; its source is the preliminary
`scratch/compiler-object-tests/source/app.mc`.

[example]: ../../examples/compiler-object-tests/README.md
[check]: ../../scripts/README.md#console-specimen-check
[roots]: compiler-root-objects.md#import-identity
