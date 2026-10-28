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

## Guarded recursive aliases

**Experimentally observed**, 2026-10-27: SDK 9.2.0, `fr265`, minimum API
5.0.0, strict checking, `-O 0` accepts the
[recursive-contract specimen](../../examples/compiler-recursive-contracts/README.md).
SDK and independently generated PRGs both produce its 23 expected lines.
The specimen exercises Array/Dictionary recursion, nullable values, interface
method chaining, and an interface stored in a source field. Structured
reconstruction succeeds for both programs without CODE/DATA fallback.

To distinguish typed recursion from Any, add these negative functions to the
same strict SDK scaffold:

```monkeyc
function wrongTree(t as Tree) as Lang.Number { return t[0]; }
function wrongChain(x as Recursive) as Lang.String {
    return x.next().value();
}
```

The SDK rejects the Array result as Number and the chained Number result as
String. The corresponding correctly typed functions compile. This establishes
retained recursive type information, not the SDK's internal representation.

Both compilers accept a self-returning implementation:

```monkeyc
class SelfNode {
    function next() as Recursive { return self; }
    function value() as Lang.Number { return 41; }
}
```

The specimen also passes `self` to a function, stores it in a nullable interface
field and a local, and returns it under an exact source-class annotation. A saved
interface reference observes a mutation performed through the original Node:
both subsequent reads report 42. This establishes retained runtime identity,
separately from successful checking and reconstruction. Reproduce with the
specimen's current source and strict scaffold. No SDK executables were
disassembled or decompiled. The
[source alias contract](../specs/frontend/README.md#type-aliases) owns supported
behavior.

The application retains its own AppBase instance through an exact class return,
nullable exact/interface fields, local copies, and a recursive interface
argument. Its counter starts at 17; mutation through an exact reference makes
the saved interface and exact references report 18. A native AppBase parameter
and result erase the source contract; `instanceof MinimalConsoleApp` recovers
it and reads 18. The same test on a Node reports -1. Both strict SDK compilation
and both simulator runs accept these paths. This establishes reference and
class-test behavior, not permission to construct application instances in
source. Our constructor restriction is checked independently by the
[contract tests](../../crates/ciq-compiler/tests/recursive_contracts.rs).

The same strict SDK scaffold accepts `typedef A as {:next as A}; typedef B as
{:next as B, :z as Lang.Number}; function f(x as B) as A { return x; }`.
The specimen extends that check with matching recursive `:label` fields and
an extra optional `:extra` field. Both strict compilers accept it. SDK and
independently generated simulator runs produce the same final five lines:
`41`, `41`, `42`, `43`, `43`. They cover a narrower function result, an asserted
record view, a shared child mutated through another reference, a parent write
through the narrower view, and a `:next` link to the parent itself. No record
copy or conversion is needed; both PRGs reconstruct without CODE/DATA fallback.
Conflicting shared fields and mismatched exact source fields fail our boundary
checks; the tests also retain API requirements and invalidate stale field proofs
after a write through an asserted view. Current matching and heap-view rules
belong in the [IR reference](../specs/ir/README.md#recursive-contracts).

## Constant bindings and identity

**Experimentally observed**, 2026-10-28, SDK 9.2.0, `fr265`, minimum API
5.0.0, `-O 0`: the [immediate-constant
specimen](../../examples/compiler-constants/README.md) compiles with the SDK's
default checking policy. It covers global/module/class declarations,
forward/cross-file references, lexical shadowing, instance receiver effects,
non-heap literal types, and Number constant operations. The overflow case retains
Number type and prints -2147483648. Structured reconstruction checks both
SDK/generated PRGs without CODE/DATA fallback. Both simulator runs produce the
same 16 expected lines, including the factory's 44 exactly once before its
constant member value.

A separate authored SDK probe establishes storage behavior for heap values:

```monkeyc
const ITEMS = [1];
module C { const BAG = [2]; function get() { return BAG; } }
class Box { const ARRAY = [3]; const TEXT = "member"; }
```

In `onStart`, assigning `ITEMS[0]=5` then reading it prints 5; assigning
`C.BAG[0]=6` then reading `C.get()[0]` prints 6. For two Box instances `a` and
`b`, changing `a.ARRAY[0]=7` leaves `b.ARRAY[0]` at 3. `a.TEXT == a.TEXT` prints
true, while `a.TEXT == b.TEXT` prints false. A global String constant compared
with itself prints true. Repeat in the minimal console scaffold with SDK
checking defaults and the ordinary simulator runner. These observations rule
out reconstructing heap-backed constants on each access or sharing class
constant storage across instances.
The SDK also accepts `return self.BAG;` in the module getter; this is an
owner-relative lookup rather than an instance of the source Box class.

The SDK rejects `Counter.OFFSET` when OFFSET is a class constant; an instance
receiver is required. The current specimen uses a factory call whose printed 44
must occur once before its constant member value is printed. The public
[language reference][constant-reference] likewise places constants at module or
class level, excludes function-local declarations, and permits mutation of
constant Array contents. Its type table marks String, Long, and Double as heap
values. The retained readable snapshot is
`scratch/official-docs/language-reference-module.readable.txt`.

The [stored-class specimen](../../examples/compiler-class-constants/README.md)
uses the same SDK/target/settings. SDK and generated simulator runs match its
expected output, covering String identity, Long/Double values, container aliasing
and mutation, source-object storage, differing union-member values, application
constants, and constructor effects. A focused SDK probe with
`const A = Trace.build()` printing 1 and `initialize()` printing 2 yields
`1, 2, 2, 7` after allocation, an explicit second `initialize()`, and an element
read. Adding an argument expression that prints 3 yields `1, 3, 2, 2, 7`:
allocation initialization precedes user argument evaluation and is not repeated
by user `initialize()`.

The authored SDK PRG's allocation hooks have no ordinary argument-count
prologue. Tail-transferring into a generated ordinary initializer with
`argcincsp` crashed before onStart; invoking it through a reserved method slot
provides a working call frame. This is evidence about application allocation
entry conventions, not inspection of SDK/compiler executables. The
[constant profile](../specs/frontend/README.md#constants) owns implemented
contracts and limits.

The [shared-storage specimen](../../examples/compiler-shared-constants/README.md)
uses the same SDK/target/settings. SDK and generated programs match the expected
output, including global/module alias mutation and identity, Long/Double,
ByteArray, source objects, WeakReference, and Method storage.

An SDK counter probe distinguishes startup from lazy initialization:
`Trace.make(n)` increments a module counter, prints `n`, and returns `[n]`.
Use it for a global constant (1), a module constant (2), and an application
constant (3). On entering onStart, the counter is 2; after accessing the module
constant it is 3. The module's initializer print is captured at first access.
Startup prints are absent from this console capture, so absence of those lines
alone does not establish absent initialization.

A second SDK probe replaces the global initializer with `makeGlobal()`, an
ordinary global helper returning `Trace.make(1)`. It compiles but the simulator
reports `Circular Dependency Error` in makeGlobal during `<init>`. The authored
SDK PRG binds ordinary function frames using owner lookup, whereas allocation
hooks use their existing receiver. Our generated initializers likewise inherit
the hook receiver; ordinary functions bind their lexical owner. Unit checks
reject initializer dependencies requiring re-entry into that owner instead of
producing this runtime failure. This does not establish that every SDK-accepted
initializer expression is supported.

The [forward-constant specimen](../../examples/compiler-forward-constants/README.md)
uses the same SDK/target/settings. SDK and generated programs match its expected
output, covering later String/Long defaults, global/module flat Array and
ByteArray defaults, later source-class field contracts, per-instance identity,
and unchanged order of runtime initializer calls.

Focused SDK probes with `const EARLY=LATE;` distinguish default availability:
String/Long literal aliases are non-Null, and a String alias equals its literal
binding by reference. Global/module flat Array and ByteArray aliases are
non-Null, including a flat Long Array. Global Dictionary and nested Array aliases
instead observe Null. For class Array and ByteArray forward reads, the SDK
rejects access as potentially uninitialized. A class scalar alias chain also
fails when it reads an intervening runtime alias before that alias's assignment;
a direct reference to a later literal succeeds.

**Interpretation:** resolve declaration types independently of readiness. Model
supported literal-default availability before runtime expressions, without
sorting arbitrary initializer dependencies or changing their effects. This does
not establish identical SDK storage layout or support for every constant
expression; the [source contract](../specs/frontend/README.md#constants) defines
the implemented default profile.

The [Null snapshot specimen](../../examples/compiler-null-snapshots/README.md)
uses the same SDK/target/settings, with a reopened module in a second file.
SDK and generated programs match its 19 expected console lines.
Direct global/module forward reads of computed values, Dictionaries, and nested
Arrays observe Null; reads after assignment observe the completed values.
Its saved Array element, nullable argument, and comparison distinguish captured
Null from a live alias to the later slot. The module's initializer prints expose
declaration order at first access. Class readiness restrictions are unchanged.

[constant-reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/

## Checking off

**Experimentally observed:** SDK 9.2.0, fr265, minimum API 5.0.0, `-O 0`,
`project.typecheck = off` accepts disjoint returns, arguments, field stores,
operator operands, Array index keys, and Boolean conditions. The
[unchecked specimen](../../examples/compiler-unchecked/README.md) produces the
same 36 expected lines from SDK and Rust PRGs. Its runtime inputs support the
operations even when their annotations disagree. Receiver cases pass a String
through Number for `length`, a Cell through Empty for `read` and `value`, and an
Array through Number for indexed reads/writes. Field updates cover compound and
conditional RHS evaluation. Literal cases preserve String/Boolean/Double tokens
inside Number-declared Arrays, Null-only contents, nested Arrays, and actual
String keys/Boolean values in Number-declared Dictionaries. Computed entry
markers establish key-before-value and element order; duplicate-key markers
show both values evaluated before the last replaces the first. Structured
reconstruction and saved-stage equivalence pass without new CODE/DATA fallback.

The SDK also compiles a wrong-arity source call with checking off. Our current
ABI profile retains known-call arity errors. Explicit local annotations are
rejected by the SDK even with checking off.

Further authored compile-only probes accept `x.length()` and `x[0]` for Number
parameters, `["x"]` returned as `Array<Number>`, and `{"x" => true}` returned as
`Dictionary<Number, Number>`. `C.missing()` and `C.absent` compile for an empty C
when another class declares `missing` and `absent`; without those declarations,
the SDK rejects undefined selectors. Receiver behavior is exercised by the
specimen above, along with the mismatched literal cases. Actual unsupported
runtime operands/receivers remain outside these successful executions. Current
implementation contracts are in the [unchecked profile](../specs/frontend/README.md#unchecked-lowering).

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
