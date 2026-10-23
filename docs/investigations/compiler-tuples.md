# Positional tuple contracts

**Experimentally observed, 2026-10-22:** SDK 9.2.0 strict checking accepts
positional annotations, nested tuples, per-slot widening, and tuple-to-Array
assignment. Mutation can replace slot types or change cardinality. These are
compile-time observations; the runtime check below covers construction, typed
transport, and direct slot replacement separately.

**Officially documented:** [Monkey Types][types] defines equal-length,
per-position tuple assignment and compatibility with an Array whose element
contract includes every slot. Array literals infer tuples. Its mutation example
changes one slot's type, making the original return annotation invalid. Reviewed
2026-10-22. The examples below test those rules independently.

## Environment and reproduction

Use an ignored project, such as `scratch/tuple-probe`, with the resources and
manifest from `examples/minimal-console`. Select fr265, minimum API 3.0.0, a
distinct application ID, and this Jungle configuration:

```text
project.manifest = manifest.xml
project.typecheck = strict
base.sourcePath = source
base.resourcePath = resources
```

Prefix each probe with `using Toybox.Application; using Toybox.Lang;` and an
AppBase wrapper; put the probe functions at global scope:

```monkeyc
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {}
}
```

Build with `sh scripts/build-example.sh scratch/tuple-probe`, which selects
optimization level zero. Successful cases return 0; type errors return 105;
the rejected annotation syntax below returns 100. The functions need not run
to reproduce these static checks.

## Assignment and construction

This succeeds; replacing the return annotation with `Array<Lang.Number>` fails:

```monkeyc
function test(x as [Lang.Number, Lang.String])
    as Lang.Array<Lang.Number or Lang.String> {
    return x;
}
```

The reverse direction, `Array<Lang.Number>` to `[Lang.Number]`, fails even with
one element expected. `[Lang.Number]` widens to `[Lang.Numeric]`; the reverse
fails. Returning `[1, "yes"]` satisfies `[Lang.Number, Lang.String]`; reversed
slot types or a one-element literal fail that annotation.

Nested `[Lang.Number, [Lang.String]]` annotations and `x[1][0]` succeed with a
String result. `[Lang.Number, Lang.String] or Null` succeeds after a non-null
guard. A `[Token]` parameter retains a source class's method: `x[0].value()`
type-checks against a directly declared `Token.value() as Lang.Number`.
The annotations `[]` and `[Lang.Number, Lang.Number...]` are syntax errors in
this SDK. The latter does not establish a variadic tuple form.

## Indexing and inference

For `x as [Lang.Number, Lang.String]`:

| Read | Accepted result contract |
|---|---|
| `x[0]` | `Lang.Number` |
| `x[1]` | `Lang.String` |
| `x[i]`, with `i as Lang.Number` | `Lang.Number or Lang.String` |

Returning `x[1]` as Number or `x[i]` as Number fails. Constant indexes `2` and
`-1` produce out-of-bounds diagnostics. `x.size()` returns Number. An inferred
local `var x = [1, "yes"]; return x[1];` satisfies a String return annotation
without declaring a tuple type on the local.

## Mutation and tracking limits

This succeeds. Changing the result annotation back to
`[Lang.Number, Lang.String]` fails with the updated `[String, String]` type:

```monkeyc
function test(x as [Lang.Number, Lang.String])
    as [Lang.String, Lang.String] {
    x[0] = "changed";
    return x;
}
```

`x.add("new")` changes a `[Lang.Number]` parameter into a value accepted as
`[Lang.Number, Lang.String]`. In contrast, `x[i] = 2` on a
`[Lang.Number, Lang.String]` parameter makes a subsequent return under the
original positional annotation fail: the diagnostic reports
`Array<Lang.Number or Lang.String>`, losing positional shape.

The following **also compiles** with a Number return, although mutation is
through an alias. This establishes a limit of SDK checking, not a guarantee
that the runtime value remains Number:

```monkeyc
function test() as Lang.Number {
    var x = [1, "yes"];
    var y = x;
    y[0] = "changed";
    return x[0];
}
```

A separate `change(x as [Lang.Number]) as Void { x[0] = "changed"; }`
called with a local `[1]` likewise does not prevent returning that local's
index zero as Number. This agrees with the documented lack of tuple mutation
tracking across calls. The complete application also passes strict SDK checking
for a conditional slot replacement joined with the original tuple, Number slot
compound updates, and replacement values containing nested tuples or exact
source instances.
A separate strict probe accepts rebinding a `[Lang.Number]` parameter to
`["changed"]` and returning `[Lang.String]`. These observations do not establish
constant folding of arbitrary index expressions or other Array mutators.

Strict probes also accept returning `x.add("tail")` directly as
`[Number, String]` for `x as [Number]`, and returning `x.addAll(y)` as
`[Number, String, Boolean]` for `y as [String, Boolean]`. After storing the
returned alias, `x[1]` is still accepted as String. Appending `[2, 3]` adds two
Number slots, while `addAll([])` preserves the original shape.

A nested append exposes another SDK tracking limit:

```monkeyc
function nested(x as [Lang.Number])
    as [Lang.Number, Lang.String, Lang.Number] {
    x.add(x.add("tail").size());
    return x;
}
```

SDK 9.2.0 strict checking rejects this return, reporting `[Number, Number]`;
it drops the inner append's String slot from the final static receiver view.
The complete application's `Append.nested` instead returns the size and prints
`3` in both SDK and independently generated PRGs. This confirms both runtime
appends. Our direct named-receiver tracking retains both slots; it does not
establish tracking through arbitrary aliases.

## Field and computed receiver probes

**Experimentally observed, 2026-10-23:** SDK 9.2.0 strict checking distinguishes
direct fields from indexed aliases. Use the same fr265/minimum 3.0.0 wrapper and
build procedure above, with `Pair` defined as `[Lang.Number, Lang.String]`.

For a `Box` with `var pair as Pair`, initialized to `[1, "seed"]`, assigning
`self.pair[0] = "changed"` permits returning `self.pair[0]` as String and rejects
a Number return. After `self.pair.add(true)`, returning the field as
`[Number, String, Boolean]` succeeds. Returning that append expression directly
also succeeds. These observations require a method-local refined field view;
the declared entry contract alone cannot check subsequent reads.

For `x as [Pair, Lang.Boolean]`, the SDK rejects this attempted parent update:

```monkeyc
function changed(x as [Pair, Lang.Boolean])
    as [Lang.Number, Lang.String, Lang.Number] {
    x[0].add(2);
    return x[0];
}
```

The diagnostic still describes `x[0]` as Pair. Similarly, after
`x[0][0] = "changed"`, returning `x[0][0]` as String is rejected while a Number
return is accepted. This is a static tracking limit, not proof that the nested
runtime value remains Number. Returning `makePair().add(true)` directly with a
three-slot annotation succeeds.

The complete application's `Receivers` module checks this runtime distinction:
an indexed append returns the extended reference, the parent observes the same
mutated Array, and a nested write changes the stored value. Console markers
check computed/chained calls and nested stores evaluate receiver, index, and
value effects once in order.

**Experimentally observed, 2026-10-23:** SDK 9.2.0 strict, unoptimized fr265
probes refine subsequent `self.pair` reads after direct writes and appends.
Starting with `[Number, String]`, a conditional Boolean replacement of slot zero
accepts a `Number or Boolean` result but rejects a Number-only result. Loop
appends accept the widened Array element domain. Mutating a named local alias
does not refine subsequent field reads.

An intervening self call restores the declaration's static view, even when the
called method is empty. After replacing slot zero with String, a subsequent
self call makes a Number return accepted and a String return rejected. This is
type-checking behavior, not evidence that the call restores the stored values.
Explicit field reassignment also establishes a new view.

Receiver capture and field replacement remain distinct. With a two-slot field,
`self.pair.add(self.resetAndValue())` has a three-slot expression result even
when `resetAndValue()` replaces the field before returning Boolean. The
[complete application](../../examples/compiler-tuples/app.mc) reproduces this
case: both SDK and independently generated simulator programs print `2` for
the replacement field's size and `3` for the captured append result's size.
Later field reads must load the actual property; writing the captured receiver
back would change the program's behavior.

## Other-instance field probes

**Experimentally observed, 2026-10-23:** SDK 9.2.0, strict checking, fr265,
minimum API 3.0.0, `-O 0`. Use the wrapper above with these declarations:

```monkeyc
typedef Pair as [Lang.Number, Lang.String];
typedef Record as interface { var pair as Pair; };
typedef Other as interface { var pair as Pair; };
class Box {
    var pair as Pair;
    function initialize() { self.pair = [1, "seed"]; }
}
```

For `function f(x as Record, y as Record)`, after
`x.pair[0] = "changed"`, returning `y.pair[0]` as String succeeds; returning it
as Number fails. Making `y` an `Other` instead reverses both outcomes, despite
the identical structural shapes. Two separate inline interface annotations also
remain independent. An alias `var y = x` shares the change; assigning another
Record parameter to `x` does not restore the Number slot view.

After `x.pair.add(true)`, returning the field as
`[Number, String, Boolean]` succeeds and returning it as Pair fails. A conditional
String replacement accepts a Number-or-String result and rejects Number alone.
An intervening empty global call restores the Number view and makes a String
return fail. Constructing a new owner or calling an owner-producing function
also restores the declared view.

For `x as Box`, the same String slot replacement instead leaves subsequent
`x.pair[0]` reads accepted as Number and rejected as String. Returning
`x.pair.add(true)` directly still satisfies the extended three-slot annotation.
Source-instance access and structural annotation access therefore have different
observed refinement rules.

**Interpretation:** structural field refinement follows declaration identity,
not object identity or structural shape equality. The accepted String result
through `y` does not prove that its runtime field changed. Our expanded
structural shapes currently discard the declaration identity needed to reproduce
this checking behavior; keying flow by identical shapes would conflate distinct
typedefs and inline annotations.

The complete application's `ExternalFields` module checks ordinary source
instance behavior with both SDK and generated PRGs. Matching simulator output
covers owner-before-RHS evaluation once, independent owners after reassignment,
retained aliases, scalar updates, nullable/source references, tuple writes,
chained append results, and field replacement during argument evaluation.
Fresh reads observe the actual property, including its runtime Array size;
declared positional metadata does not assert an unchanged runtime shape.

## Runtime construction and typed transport

**Experimentally observed, 2026-10-23:** strict SDK 9.2.0 probes accept an
unknown-length `Array<Number>` tail on `[Number, String]`, returning
`Array<Number or String>`. A `for` loop appending String values to the same
tuple also satisfies that Array return. Writing Boolean through a dynamic
Number index satisfies `Array<Number or String or Boolean>` instead of the
original tuple contract.

After shape loss, ordinary Array invariance applies. For example:

```monkeyc
function follow(x as [Lang.Number, Lang.String],
                tail as Lang.Array<Lang.Number>) as Lang.Number {
    x.addAll(tail);
    x.add(true); // rejected: the Array elements are Number or String
    return x.size();
}
```

Replacing `x.add(true)` with another `x.addAll(tail)` is also rejected: the
second call requires `Array<Number or String>`, while the tail is still
`Array<Number>`. These checks are distinct from the first tuple-to-Array
transition. The compiler retains these invariant call requirements after
shape loss outside loop inference.

**Experimentally observed, 2026-10-22:** the
[complete application](../../examples/compiler-tuples/README.md) built with SDK
9.2.0 strict checking and independently through source, saved IR, and assembly
produced the same expected console output in the fr265 simulator. This covers
nested/source slots, positional/dynamic reads, nullable guards, fields, Method
results, contextual literal arguments, conditional construction, Array widening,
and ordered slot effects. The application also checks inferred mixed scalar
tokens, Null-only tuples, nested literals, source slots selected by nested
inference, and ordered computed effects inside nested literals. Direct slot
replacement changes parameter/local contracts, including nested/source values;
the console also checks both paths of a branch join, compound updates, and a
conditional RHS. Append checks cover returned aliases, source-class tails,
homogeneous/mixed/empty literal arguments, argument effects, branch-dependent
cardinality, and the nested append above.
The 2026-10-23 extension checks unknown-length tails, zero/multiple loop
iterations, `while`/`do` loops, multiple append types, dynamic writes, and nullable
tuple locals/parameters on both guarded paths. Both SDK and generated output
match the complete expected console.
Both SDK and generated PRGs reconstructed through structured section export
without required CODE/DATA fallback. It does not establish mutation tracking
through arbitrary aliases, across calls, or
inference for every unconstrained literal.

[types]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
