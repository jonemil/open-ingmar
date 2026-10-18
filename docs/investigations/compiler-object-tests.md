# Runtime object predicates

2026-10-17 to 2026-10-18, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.
Question: how do runtime predicates encode and support guarded Object uses?

**Experimentally observed:** the [tracked specimen][example] matches all 75
expected lines in SDK/generated runs. It tests all supported concrete Lang
classes, source field/method existence, inherited observers, Symbol parameters,
computed selectors, null guards, and Boolean short-circuiting. Computed operands
print `receiver` before `selector`; each appears once.

The SDK accepts positive class tests before Number arithmetic, String methods,
negated early returns, AND operands, conditional arms, and loop bodies. The
extended specimen narrows nullable Object inputs, changes an Object local after
its loop guard, and sorts `[3, 1, 2]` to `[1, 2, 3]` with Number arithmetic in an
Object-parameter comparator. The generated program matches these outcomes.
Saved IR reconstructs the same program without additional assertion operations,
VM opcodes, or DATA records.

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

**Limits:** source inheritance, interface tests, parameterized-container casts,
general union subtraction, and guarded API availability remain unverified. A preliminary
`1.0f` literal is SDK-accepted but unsupported by our parser; the final specimen
uses unsuffixed Float syntax. The lexical gap remains in the plan.

## Lexical casts

**Officially documented:** [Monkey Types][types] describes `as` casts as lexical,
with no runtime changes. The earlier plan's phrase "checked cast" did not
establish a runtime check.

**Experimentally observed:** the SDK accepts `o as Number + 1` and
`1 + o as Number` for an Object operand. `-o as Number` is rejected because unary
negation still sees Object; `-(o as Number)` is accepted. Its authored probe
returns `4`, `4`, and `-3` from these accepted forms. Number asserted as String
still passes `instanceof Number`, and Number asserted as Double remains Number.
Null asserted as String remains null. The probe and runtime capture are in
`scratch/compiler-cast-probe/` and `scratch/cast-probe-sdk-runtime/`.

The tracked example additionally checks scalar/nullable assertions, opaque
Object erasure/recovery, Symbol selectors, Method invocation, WeakReference and
opaque Dictionary methods, conditional values, and one-time operand effects
(`cast-effect`). SDK/generated output matches all expected lines. Both programs
export and reconstruct without new instructions or DATA records. Casts alone
require no emitted VM operations; this does not establish safe uses of falsely
asserted values.

## Source-class targets

**Experimentally observed:** the [two-file specimen][source-types] accepts
source-class tests and casts across file boundaries. An Object-rooted Probe
matches itself and rejects another Object-rooted class and Null. Positive tests
permit own-method calls on Object/nullable Object reads, including through a loop
whose local is later reassigned. Explicit assertions recover exact source return
contracts and native-base projections for View/BehaviorDelegate classes.

A false Number-to-Probe assertion still passes `instanceof Number`; Null asserted
as Probe still fails `instanceof Probe`. Casts introduce no runtime validation.
SDK/generated output matches the complete expected console, and both PRGs export
CODE/DATA structurally, rebuild exactly, and re-export identically. The source
tests use existing `getmv`/`isa` encoding; no new instruction or record was found.
These cases do not establish source inheritance, nullable source signatures,
dynamic class operands, or qualified/nested source lookup.

## Reproduction

Use the example's compiler command and the shared [console checker][check].
The ignored `scratch/compiler-object-tests/monkey.jungle` selects the tracked
source and shared native-names manifest/resources. Build through
`scripts/build-example.sh`, then prepare and simulate `compiler-object-tests`.
Accepted saved-stage, SDK/generated round-trip, and runtime artifacts are under
`scratch/check-source-types/`. The isolated nullable failure is under
`scratch/object-tests-sdk-runtime-full/`; its source is the preliminary
`scratch/compiler-object-tests/source/app.mc`.

[example]: ../../examples/compiler-object-tests/README.md
[check]: ../../scripts/README.md#console-specimen-check
[roots]: compiler-root-objects.md#import-identity

[types]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/

[source-types]: ../../examples/compiler-object-tests/source-types.mc
