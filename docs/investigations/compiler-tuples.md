# Positional tuple contracts

**Experimentally observed, 2026-10-22:** SDK 9.2.0 strict checking accepts
positional annotations, nested tuples, per-slot widening, and tuple-to-Array
assignment. Mutation can replace slot types or change cardinality. These are
compile-time observations; the runtime check below covers construction and
typed transport separately.

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
tracking across calls. It does not establish branch-merge rules, constant
folding of arbitrary index expressions, or the effect of other Array mutators.

## Runtime construction and typed transport

**Experimentally observed, 2026-10-22:** the
[complete application](../../examples/compiler-tuples/README.md) built with SDK
9.2.0 strict checking and independently through source, saved IR, and assembly
produced the same expected console output in the fr265 simulator. This covers
nested/source slots, positional/dynamic reads, nullable guards, fields, Method
results, contextual literal arguments, conditional construction, Array widening,
and ordered slot effects. The application also checks inferred mixed scalar
tokens, Null-only tuples, nested literals, source slots selected by nested
inference, and ordered computed effects inside nested literals. Both SDK and
generated PRGs reconstructed through
structured section export without required CODE/DATA fallback. It does not
establish tuple mutation tracking or inference for every unconstrained literal.

[types]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
