# Core language coverage review

Reviewed 2026-10-21 against source/IR contracts, declaration/executable code,
retained language-guide articles, and Garmin's public API. SDK 9.2.0 / `fr265`
is the runtime reference environment; individual probes below distinguish
compiler acceptance from simulator observations.
The [prioritized checklist][inventory] owns remaining work.

## Type-checking policies

Experimentally observed, 2026-10-25, SDK 9.2.0, `fr265`: functions with untyped
parameters, an unknown
receiver call, and a Number-or-String argument passed to a Number parameter
compile under gradual/informative checking. Informative reports ambiguity when
compiler warnings are enabled (`-w`); strict rejects these cases. A String
returned from a Number function fails all three checked modes but compiles with
checking off. Set the policy through `project.typecheck` or compiler `-l`.
These are compiler-acceptance probes, not runtime evidence.

Place these functions in an application module and compile with checking set to
`gradual`, `informative`, or `strict`, with optimization disabled (`-O 0`):

```monkeyc
function exact(x as Toybox.Lang.Number) as Toybox.Lang.Number { return x; }
function unknown(x) as Toybox.Lang.Number { return x; }
function dynamic(x) as Toybox.Lang.Number { return x.size(); }
function overlap(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number {
    return exact(x);
}
// Test separately: rejected by all checked levels, accepted with checking off.
function bad() as Toybox.Lang.Number { return "wrong"; }
```

Experimentally observed, 2026-10-26, SDK 9.2.0, `fr265`, minimum API 5.0.0,
`-O 0`: overlapping Array parameters compile in gradual/informative mode in
both directions (`Array<Number>` and `Array<Number or String>`), including
nested Arrays. Overlapping Dictionary value parameters also compile. Strict
rejects these cases. `Array<Number>` to `Array<Object>` and `Array<String>` to
`Array<Number>` fail informative checking; nominal inheritance does not make
container parameters covariant. These observations distinguish uncertain
matches from the guide's equivalent-parameter rule for proven matches.

The [partial-match specimen](../../examples/compiler-partial-matches/README.md)
compiles under informative checking and produces the same 21 expected console
lines in SDK and independently generated simulator runs. It covers native/user
calls, returns, parameter/instance-field stores, Array/Dictionary/record
construction, nested Array views, Dictionary key/value views, and shared
mutations through narrowed/widened Array aliases. Source cases also send TokenB
through a TokenA return, argument, field, record, element store, and Array view.
TokenB still returns 62 from its own `number()` method through each declared
TokenA view; TokenA returns 61. Both SDK and generated execution preserve that
dispatch without a class-changing conversion or rejecting class check at the
boundary. This does not establish behavior when the actual object lacks the
invoked member or has an incompatible signature.

Separate authored probes show that `A or B` returned as `A` compiles in
gradual/informative mode and fails strict checking. In contrast, structural
method results `Number or String` versus `Number`, including a class used as an
interface implementation, and mutable structural fields with those differing
contracts fail informative checking. Reproduce by returning the wider shape
under the narrower annotation in the minimal console scaffold; the
[boundary tests](../../crates/ciq-compiler/tests/type_checking.rs) retain the
structural forms. The runtime specimen additionally confirms class-only
source-element Array overlap; mixed/erased source-element cases remain a
separate probe in the Array worklist.

SDK Dictionary<String, Number-or-String> narrowing also compiled. Our parser
currently admits established numeric aliases such as Integer in that position,
with heterogeneous native union domains retained in the Dictionary worklist.

The [public type guide][type-guide] describes False/Maybe/True matches and the
four policies. The [source contract][checking] states our current limits.

[type-guide]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
[checking]: ../specs/frontend/README.md#type-checking-policy

## Checking off

**Experimentally observed:** SDK 9.2.0, fr265, minimum API 5.0.0, `-O 0`,
`project.typecheck = off` accepts disjoint returns, arguments, field stores,
operator operands, Array index keys, and Boolean conditions. The
[unchecked specimen](../../examples/compiler-unchecked/README.md) produces the
same 16 expected lines from SDK and Rust PRGs. Its runtime inputs support the
operations even when their annotations disagree. Receiver cases pass a String
through Number for `length`, a Cell through Empty for `read` and `value`, and an
Array through Number for indexed reads/writes. Field updates cover compound and
conditional RHS evaluation. Structured reconstruction and
saved-stage equivalence pass without new CODE/DATA fallback.

The SDK also compiles a wrong-arity source call with checking off. Our current
ABI profile retains known-call arity errors. Explicit local annotations are
rejected by the SDK even with checking off.

Further authored compile-only probes accept `x.length()` and `x[0]` for Number
parameters, `["x"]` returned as `Array<Number>`, and `{"x" => true}` returned as
`Dictionary<Number, Number>`. `C.missing()` and `C.absent` compile for an empty C
when another class declares `missing` and `absent`; without those declarations,
the SDK rejects undefined selectors. Receiver behavior is exercised by the
specimen above; the mismatched literal probes identify remaining construction
gaps and have no runtime evidence. Current implementation
contracts are in the [unchecked profile](../specs/frontend/README.md#unchecked-lowering).

## Partial operators and native receivers

**Observed:** SDK 9.2.0, fr265, API minimum 5.0, `-O 0`, informative
checking accepts Number-or-String subtraction, negation, complement, comparison,
updates, and Array index keys. It also accepts `length()` on Number-or-String
and String-or-Null, and a source-class union with only one `value()` method.
Strict (`-l 3`) rejects the uncertain alternatives; informative still rejects
`length(1)` for the known String signature.

Unary plus preserves its input: informative accepts a Number-or-String operand
returned as String. The runtime specimen passes String through unchanged.
The independently authored
[partial-operations example](../../examples/compiler-partial-operations/README.md)
produces the same 21 expected console lines from SDK and Rust PRGs, including
source-only partial lookup, nullable source receivers, source/native mixtures,
different source result types, and discarded Void calls. Each executed receiver
provides the member; these runs do not establish the failure behavior of an
actual missing member or Null receiver.
Preparation checks saved-stage equivalence and complete structured SDK/generated
reconstruction; no new opcode or DATA fallback was needed.

Reproduce with the example's SDK configuration and source, then repeat with
strict checking. Current contracts are in the
[frontend reference](../specs/frontend/README.md#native-unions).

## Public Lang inventory

**Officially documented:** the [Lang index][lang] lists 24 classes, the
Comparable/Comparator interfaces, Numeric/Integer/Decimal aliases,
NumberFormat/Endian constants, and `format`.

Own-method summaries for all non-exception classes were compared with
`api::core()`, native-call profiles, and frontend contracts. Implemented scalar
methods, Array sorting, source/self Method binding, ByteArray operations, and
`format` must not be relisted as wholly missing. Their supported domains are
specified in the [frontend reference][source]. New type/storage domains need
explicit work only where a concrete unsupported path is identified.

ResourceId's own member is `toString`. Method additionally declares the
`initialize(aClass, aMethod)` constructor. Exception declares `initialize`,
`getErrorMessage`, and `printStackTrace`. Each of the eight Lang exception
subclasses declares a constructor: seven take a String message;
UnexpectedTypeException also takes two Object-or-Null compatibility arguments.
These names and signatures were checked against their linked API pages via the
Lang index. Inherited Object/Exception members need not be duplicated per class
in the worklist. Public API signatures alone do not establish VM bindings or
runtime acceptance.

Downloaded public class pages and extracted own-member names are retained under
`scratch/core-plan-review/`. The compiler/SDK executables were not inspected.

## Language coverage

The official reference/functions, Objects and Memory, Monkey Types,
annotations, and exception-guide snapshots under
`scratch/official-docs/plan-review/` supplied the language coverage map.
The [reading map][reading] retains article links and attribution. Current source
contracts, parser, call/type validation, and format documentation distinguish
implemented forms from remaining items in the
[language checklist](../plan.md#core-language-worklist).

The VM `throw` opcode and ordinary EXCEPTIONS parsing/writing already exist;
source exception IR, handler-range generation, and unwinding semantics do not.
Likewise, implemented `for` loops, parameter assignment, and Object default
initializers are absent from the remaining-work list. An SDK-rejected form is a
documented restriction, not a perpetually unfinished implementation task.

An authored compiler probe (`scratch/core-plan-review/interface-fact.mc`)
confirms that a Comparable parameter refined with `instanceof Token` already
retains Token member calls and can be forwarded to a Comparable consumer.
That source-identity task is not missing. The initial `interface-native.mc`
probe exposed rejection of `c + 1` after a Comparable parameter tested positive
for Number. The [grouped refinement experiment](compiler-comparators.md#native-comparable-refinement)
records its resolution across the scalar Comparable implementations.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html
[inventory]: ../plan.md#toyboxlang-worklist
[source]: ../specs/frontend/README.md
[reading]: official-garmin-documentation.md
