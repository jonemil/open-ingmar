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

Experimentally observed, 2026-10-26: the
[partial-match specimen](../../examples/compiler-partial-matches/README.md)
compiles with SDK 9.2.0 gradual checking (`fr265`, minimum API 5.0.0, `-O 0`).
SDK and independently generated programs produce the same eight expected
console lines in the simulator. This confirms matching runtime alternatives
through native/user calls, returns, parameter/instance-field stores, and
Array/Dictionary/record construction. It does not establish behavior for a
non-matching runtime alternative or different container reference contracts.

The [public type guide][type-guide] describes False/Maybe/True matches and the
four policies. The [source contract][checking] states our current limits.

[type-guide]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
[checking]: ../specs/frontend/README.md#type-checking-policy

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
