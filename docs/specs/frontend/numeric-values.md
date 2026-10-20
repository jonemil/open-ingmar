# Numeric values

## Floating-point values

Float and Double are distinct numeric scalars, available since API 1.0.0.
Decimal literals with a fractional part or decimal exponent default to Float; a
lowercase `d` suffix selects Double, including an integer mantissa such as `1d`.
A fractional part requires digits on both sides of the point; an exponent uses
`e`/`E`, optional sign, and at least one digit. Literal parsing rounds to IEEE
754 binary32/binary64. Values rounding to infinity are rejected; underflow may
round to zero. A negative literal preserves signed zero. Parenthesize a negative
literal before calling a member, as for Number/Long.

Explicit Float/Double parameters/results, local storage, module/instance calls,
printing, conditional joins, and `or Null` contracts use the shared scalar
rules. Assignments, arguments, returns, and conditional arms do not implicitly
convert numeric widths. Arrays and Dictionaries follow their shared scalar
contracts;
[scalar fields](fields.md#typed-scalar-fields) preserve these types as well. String concatenation and
Char equality accept these widths; Char ordering/addition with floating values
remain unsupported.

The four arithmetic operators `+`, `-`, `*`, and `/` accept Number, Long, Float,
and Double pairs. Double dominates; Long mixed with Float also produces Double;
otherwise Float dominates integer operands. Integer-only pairs follow their
existing rules. All six numeric comparison operators accept these pairs and
return Boolean. Floating `%`, bitwise/shift operations, `!`, and `~` are
rejected. Unary plus preserves type/value. Negation subtracts from a same-type
positive zero; this differs from a negative-zero literal at zero. Statement
`++`/`--` and arithmetic compound writes preserve the local's type under these
promotion rules. Floating arithmetic and exceptional values are runtime
operations; the compiler inserts no conversions or constant folding. The tested
SDK 9.2.0 / fr265 runtime rejects the tested Float/Double zero-divisor cases
with `Invalid Value`; these operations do not supply infinity or NaN values. See
[isolated boundary
evidence](../../investigations/compiler-numeric-boundaries.md).

Float/Double receivers support `abs()` returning the receiver type,
`format(String)` and `toString()` returning String, and numeric conversions
`toNumber()`, `toLong()`, `toFloat()`, and `toDouble()` returning their named
scalar types, since API 1.0.0. Number and Long also support both floating
conversions since 1.0.0. In-range floating-to-integer conversions truncate
toward zero. Tested extreme finite and non-finite values convert to the minimum
signed Number/Long on the SDK 9.2.0 / fr265 simulator; this is not a portable
conversion guarantee or a rule to fold into generated code. See
[inputs and limits](../../investigations/compiler-numeric-boundaries.md).

Inherited `hashCode()` returns Number, since 1.0.0; immutable `weak()` returns
the effective receiver type since 1.2.0. Float inherits Object `equals` since
1.0.0; Double overrides it since 1.3.0. Equality accepts the
[shared Object argument profile](lang-values.md#object-equality-and-container-observers)
and distinguishes floating widths and integer classes. Do not infer a
cross-instance Double hash contract from numeric equality. `compareTo` follows
the shared [Object argument
contract](lang-values.md#comparison-object-arguments) and returns Number since
5.0.0. String `toFloat()` returns Float or Null since 1.0.0; `toDouble()`
returns Double or Null since 3.1.0. Guard nullable results before scalar use.
Conversion content and formatting are passed through to the runtime.

See the [grouped example](../../../examples/compiler-float-values/README.md) and
[encoding/runtime evidence](../../investigations/compiler-floating-values.md).

## Long values

Long is a signed 64-bit scalar, distinct from Number, available since API 1.0.0.
Decimal literals end in `l` or `L`; values must fit i64, including
`-9223372036854775808l` and `9223372036854775807l`. An unsuffixed token retains
Number's signed 32-bit limit. A minus immediately applied to a Long token forms
a negative literal under the same spacing/postfix restrictions as Number. Unary
plus and negation also accept Long expressions, preserving their width.

Explicit `Toybox.Lang.Long` parameters/results, local copies/reassignment,
module/instance calls, discarded calls, repeated construction, and
`System.println` preserve all 64 bits. Long and Number contracts do not
implicitly convert. Unannotated source returns still require Number; inferred IR
returns can carry Long.

Long/Long and mixed Number/Long pairs support `+`, `-`, `*`, `/`, and `%`.
Either Long operand makes the result Long; Number/Number keeps Number. Promotion
belongs to these operators only: arguments, returns, and assignments still
require their declared storage type. Long locals support the corresponding
compound assignments with Number/Long right operands; a Number local cannot
store a promoted Long result. Nullable numeric operands require a non-null
proof.

These pairs also support all six numeric comparisons, returning Boolean. Long
mixing with Char admits equality/inequality and addition returning Char;
ordering remains unsupported. String mixing follows the shared String profile.
Boolean mixing remains unsupported; nullable Long equality with Null supports
proofs. Unary plus preserves the value; unary minus, addition, subtraction, and
multiplication wrap to signed 64-bit results. Integer division truncates toward
zero, and remainder follows the dividend's sign. On the SDK 9.2.0 / fr265
simulator, zero divisors report `Invalid Value`; minimum Long divided by or
reduced modulo -1 terminates the simulator process. No portable boundary result
or exception recovery is established. Keep native instructions without guards or
folding; see [isolated boundary
evidence](../../investigations/compiler-numeric-boundaries.md).

Long/Long and mixed Number/Long pairs also support `&`, `|`, `^`, `<<`, and
`>>`, producing Long whenever either operand is Long. This includes a Number
left operand and Long shift count. The current runtime uses the low six bits of
the count for a promoted Long shift; negative counts and counts above 63
therefore wrap. Right shift is arithmetic, retaining the sign. Pure Number pairs
retain their existing 32-bit contracts; the compiler inserts no count conversion
or masking code.

`!` and `~` complement all 64 Long bits and return Long. `!` does not make a
Long into Boolean; conditions still require Boolean. Prefix/postfix `++` and
`--` accept Long as statements, preserve its type, and wrap at the signed
boundaries. Bitwise/shift compound assignments follow operator promotion and
must preserve the local's storage type. Updates remain invalid as expressions.

Long receivers support these native methods:

| Method | Result | Since API |
|---|---|---|
| `abs()`, `toLong()` | Long | 1.0.0 |
| `toNumber()` | Number | 1.0.0 |
| `toString()`, `format(String)` | String | 1.0.0 |
| inherited `hashCode()` | Number | 1.0.0 |
| inherited `weak()` | Long | 1.2.0 |
| `equals(other)` | Boolean | 1.3.0 |
| `compareTo(Object)` | Number | 5.0.0 |

`toNumber()` retains the low 32 bits as a signed Number. Number's `toLong()`
sign-extends its value, since API 1.0.0. `abs()` of minimum Long wraps to
minimum. Equality does not equate Long and Number values; comparison supports
numeric cross-types. Native argument profiles and null-proof requirements follow
the
[shared rules](lang-values.md#native-value-calls).

String `toLong()` and `toLongWithBase(Number)` return Long or Null, since API
3.1.0. The runtime parses a numeric prefix and returns Null for an invalid
prefix or signed 64-bit overflow. Base 0 detects prefixes; 2..=36 specifies a
radix. The compiler passes conversion content and bases through without
validation.

`Long or Null` parameters/results, locals, and conditional joins use the shared
nullable rules. A direct non-null proof is required before numeric operations,
printing, or receiver dispatch. Matching Long arms join without conversion;
mixed Number/Long conditional arms remain invalid.

Long `toFloat()` and `toDouble()` are available since API 1.0.0, under the
[floating contracts](#numeric-values). Long arrays and Dictionaries follow their
shared scalar contracts;
[scalar fields](fields.md#typed-scalar-fields) also support Long. See the
[example](../../../examples/compiler-long-values/README.md) and
[encoding/runtime evidence](../../investigations/compiler-long-values.md).

## Numeric values

The numeric typedefs expand to these concrete alternatives:

| Alias | Alternatives |
|---|---|
| `Toybox.Lang.Integer` | Number, Long |
| `Toybox.Lang.Decimal` | Float, Double |
| `Toybox.Lang.Numeric` | Number, Float, Long, Double |

They introduce no VM class, wrapper, or conversion. Canonical names, imports,
and Lang aliases work in parameters, returns, lexical assertions, and their `or
Null` forms. Locals retain the declared union across assignments of its
alternatives. A non-null alias cannot receive Null; a nullable alias can. Native
and saved-IR validation enforce the same contracts.

Arithmetic and comparisons require every possible concrete operand pair to
support the operation. Existing scalar promotion rules apply to each pair; the
result is one exact scalar or the narrowest supported numeric alias covering all
results. For example, Integer + Integer yields Integer, Numeric + Float and
Integer + Float yield Decimal, and Decimal + Long yields Double. Only Integer
supports remainder, bitwise operations, shifts, and complement. Unary signs and
statement updates preserve each non-null alias. Operations lower to the ordinary
VM instructions without inserting conversions. Alias negation uses Number zero
subtraction, with result width selected by runtime promotion.

Printing accepts non-null numeric aliases. Native member calls must be valid for
every alternative and have one common result contract, including `toString`,
`format`, and numeric conversions. Direct `instanceof Number/Long/Float/Double`
branches refine fresh alias local/parameter reads under the existing branch
rules. Nullable aliases require a current non-null proof before arithmetic,
printing, or member calls. Writes invalidate that proof; a saved Boolean guard
is not a proof for later reads.

Conditional joins retain an existing numeric alias when it covers both arms;
adding a Null arm makes it nullable. Concrete Number/Long or Float/Double arms
alone do not introduce a numeric alias.
[Class fields](fields.md#numeric-alias-fields) retain these same alias
contracts. Arbitrary source unions and general union inference remain
unsupported.

### Numeric alias containers

The numeric aliases are supported as invariant Array element and Dictionary
key/value contracts. Array elements and Dictionary values may additionally
include Null; keys remain non-null. Canonical names, imports, and aliases share
these contracts in parameters, results, locals, and conditional joins. This does
not add nullable Array/Dictionary references or container fields.

Explicit literal context may admit different scalar classes within the selected
alias. Without context, a computed value already typed as a numeric alias can
establish that alias contract; mixed concrete literals alone do not invent it.
Array/Dictionary construction, element/key/value writes, and native calls check
every supplied alternative. Mutable contracts are invariant: an Array of Number
cannot substitute for an Array of Integer or Numeric, nor can a narrower
Dictionary substitute for one with a wider numeric alias.

Non-null indexed reads support the shared arithmetic/update rules; writes must
still fit the element contract. Nullable reads need a guarded local snapshot.
Sized Array allocation permits only nullable elements, including nullable
numeric aliases, and does not infer non-nullness from subsequent writes.
Dictionary lookup adds Null for an absent key even when stored values are
non-null; `values()` preserves the stored value contract and `keys()` the
non-null key contract.

Generic Array and Dictionary methods preserve these types. Non-null numeric
alias Arrays support default and checked source Comparator sorting under the
existing API 5.0.0 sorting rules; nullable element sorting remains unsupported.
Constant Arrays and Dictionaries retain each scalar's runtime type and exact
bits. Computed literals use the existing ordered private builders. No implicit
conversion, numeric-key merging across scalar classes, or mutable-container
widening is introduced. See the
[example](../../../examples/compiler-numeric-containers/README.md) and
[evidence](../../investigations/compiler-numeric-unions.md#container-storage).
