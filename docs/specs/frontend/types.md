# Types and value contracts

## Function type contracts

Ordinary source parameters default to Number. Annotations accept the
[value-type domains](grammar.md#type-syntax), resolved through canonical names,
imports, and module aliases. Constructible source-class annotations preserve
[exact identity](classes.md#source-class-signatures). Nullable forms append
`or Null` only for the listed nullable domains; general unions are unsupported.

Unannotated functions with value returns require Number results; those with no
value returns have a Void contract. Every non-Number value result requires an
explicit source annotation. Nullable exact source-class annotations remain
unsupported. Each return must be assignable to its contract: a nullable result
accepts its scalar type, Null, or the same union. Unrelated types, value/void
mixtures, and missing returns are errors. Typed IR constructors infer uniform
scalar/Void returns unless an explicit signature is supplied. Native callbacks
retain their independently resolved signatures; annotations cannot override
them.

## Nullable results


String `find(String)` and `toNumber()` are available since API 1.0.0 and return
Number or Null; `toNumberWithBase(Number)` has the same result since 1.4.1.
String `toLong()`/`toLongWithBase(Number)` return Long or Null since 3.1.0.
String `substring` and user calls may also return nullable values. Compare
nullable parameters or stored results with null. A direct `if (value == null)`
or `if (null == value)` proves that parameter or local non-null on the false
path. Direct `value != null` or `null != value` proves non-null on the true
path. Boolean negation reverses this proof edge; `if (!(value == null))` proves
non-null on the true path. Fresh reads there have the non-null numeric, String,
Boolean, Char, or Symbol type and support its consumers, including ordinary user
arguments/returns. For example:

```monkeyc
function parse(text as Toybox.Lang.String) as Toybox.Lang.Number {
    var value = text.toNumber();
    if (value == null) { return 0; }
    return value + 1;
}
```

A join retains a proof only if all incoming paths provide it. The false exit of
`while (value == null)` also provides a proof, unless another exit such as
`break` lacks it. Assignments to that local invalidate the proof, including
assignments of non-null values. Nullable locals accept Null, their non-null
type, or the same union; their declared storage type stays nullable.

Only direct parameter/local-read comparisons with literal null used by the
branch establish facts. A stored Boolean condition, a null-valued local used as
the comparison operand, or a check of a different storage identity does not
narrow the target. Boolean negation chains around the direct comparison reverse
its edge once per negation; storing the Boolean result loses the proof relation.
Checking one local or parameter does not narrow aliases, copies, or later
native-call results. Unchecked nullable values cannot be printed or used as
non-null operands/receivers. They may be passed to matching nullable parameters
and returned under matching nullable result contracts. Scalar parameters/results
still require a non-null value. Fresh parameter reads on the non-null path use
the scalar type directly. Parameter slots cannot be assigned in this subset.
Local writes do not invalidate parameter facts, even when their numeric
local/parameter IDs match. Other unions remain outside this profile. Conversion,
search, and slicing behavior is supplied by the runtime, not folded by the
compiler. See
[Number evidence](../../investigations/compiler-nullable-numbers.md) and
[substring evidence](../../investigations/compiler-substring.md).

`substring` takes zero-based start and exclusive end indices. Number indices are
accepted from API 1.0.0. Null or Number-or-Null indices require target and
minimum API at least 3.3.2 when an API context is supplied: Null selects the
respective beginning/end boundary. Negative indices are relative to the end from
API 3.3.2. Numeric ranges are not analyzed, so callers using negative values
must select a compatible minimum themselves. No API context means these
compatibility checks are omitted. Index normalization/error behavior remains a
runtime obligation; the declared result remains nullable.
