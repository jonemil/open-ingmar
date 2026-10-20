# Expressions and operators

## Arithmetic and lowering contract

[Expression grammar](grammar.md#expressions-and-precedence) owns precedence,
associativity, prefix/postfix syntax, and grouping. Operators evaluate operands
once, left to right, under the type/result contracts here. Unary plus preserves
a checked numeric value without conversion; updates and compound writes follow
the [statement rules](statements.md).

Number division truncates toward zero; remainder has the dividend's sign.
Multiplication uses the Number VM instruction; representative overflow probes
wrap to signed 32-bit values. Zero divisors are runtime failures, including
literal zero; the compiler does not fold arithmetic or insert exception guards.
The SDK 9.2.0 simulator exits when the minimum Number is divided by, or has
remainder taken against, -1. This boundary does not establish a portable result
or a catchable Monkey C exception. See [product
evidence](../../investigations/compiler-number-products.md).

Number-only bitwise operators use non-null signed 32-bit bit patterns. Mixed
Long operations follow the [Long contracts](numeric-values.md#long-values).
Floating, Char, Boolean, String, and nullable operands without a proof are
rejected by bitwise operations. `>>` is an arithmetic right shift in the tested
VM. Complement retains a unary IR operation and lowers to `invv`, evaluating its
operand once.

For Number-only shifts, counts are non-null Numbers. Mixed Long shifts follow
the [Long contracts](numeric-values.md#long-values). The compiler passes counts
unchanged, without range checks or masking. SDK/generated probes for -1, 32, and
33 agree with selecting the low five count bits; this is evidence for the tested
VM, not a portable guarantee for other runtimes. See [bitwise
evidence](../../investigations/compiler-number-bits.md).

Unary plus is retained as a checked identity operation in IR and emits no VM
instruction. It does not parse text, convert types, or relax null proofs. The
SDK probe passes other value types through too; non-numeric types remain outside
our current profile. See [unary-plus
evidence](../../investigations/compiler-number-plus.md).

### Operator contracts

Validation resolves operand API types to these result contracts:

| Operands | Operators | Result |
|---|---|---|
| Number / Number | `+`, `-`, `*`, `/`, `%`, `&`, `\|`, `^`, `<<`, `>>` | Number |
| Number | unary `+`, `-`, `~`, `!` | Number |
| Boolean | unary `!` | Boolean |
| Char / Number or Long, either order | `+` | Char |
| Char / Char | `+` | String |
| Exact String / supported scalar or null, either order | `+` | String |
| Number / Number, Char / Char, Number / Char, Char / Number | `==`, `!=`, `<`, `<=`, `>`, `>=` | Boolean |
| Boolean / Boolean | `==`, `!=` | Boolean |
| Char / Long, Float, or Double, either order | `==`, `!=` | Boolean |
| Exact String / String, numeric, Char, or Null, either order | `==`, `!=` | Boolean |

Mixed Char/integer addition changes the numeric character value, including
negative Number/Long offsets. Char + Char concatenates the two characters.
Comparison uses numeric values: `'A' == 65` is true on the validated profile,
while `'A'.equals(65)` is false. Char ordering with Long/Float/Double is
unsupported, as is Char addition with Float/Double and all Char subtraction.
With one exact String operand, concatenation accepts numeric scalars, String,
Boolean, Char, Null, or their nullable scalar forms, returning String. The VM
supplies scalar conversion and formats Null as `null`. Equality accepts the same
profile except Boolean (including Boolean-or-Null), using String identity:
copied references compare true, while separately created equal-content strings
compare false. Use `.equals()` for content. Nullable alternatives are accepted
only with an exact String on the other side; this does not establish a non-null
proof for subsequent use. String ordering, subtraction, and other
object/container operands are unsupported. See the
[String operator evidence](../../investigations/compiler-string-operators.md) and
[wider operand evidence](../../investigations/compiler-wide-operators.md).
Unchecked nullable operands require a non-null proof before numeric use. Result
contracts remain exact in assignments, calls, and returns. Overflow and invalid
Unicode ranges are runtime responsibilities. See the
[operator evidence](../../investigations/compiler-char-operators.md).

Supported null checks follow the [nullable rules](types.md#nullable-results).
These contracts are independent of method lookup and VM instruction selection.
API declarations do not broaden accepted operator combinations or cause heap
allocations to be emitted.

Unary negation accepts non-null Number, Long, Float, and Double, plus checked
numeric aliases under their [numeric contracts](numeric-values.md). Fresh reads
under a validated direct null guard are accepted. It evaluates the operand once
and uses [typed-zero
subtraction](../lowering/values.md#scalar-instruction-selection). Negating the
i32 minimum retains `-2147483648` on the validated profile. Other arithmetic
overflow behavior is not specified by that observation. Signed decimal literals
remain constants; general negation uses a distinct IR operation and counts
toward the nesting bound. See the [negation
evidence](../../investigations/compiler-number-negation.md).

Unary `!` inverts a Boolean or complements the 32/64 bits of Number/Long,
preserving its operand type and evaluating it once. Thus `!7` is Number `-8`,
not Boolean; `!!7` is Number `7`. Value-producing inversion lowers directly to
`invv`. Boolean condition negation reverses the true/false destinations. Other
types are rejected without truth-value conversion; numeric results cannot be
used as Boolean conditions. A nullable Number needs a non-null proof. Postfix
calls bind first: `!value.equals(1)` inverts the method result, while
`(!value).equals(1)` calls the method on the inverted value. See the
[Boolean evidence](../../investigations/compiler-boolean-negation.md) and
[operator review](../../investigations/compiler-operator-coverage.md).

The [IR model](../ir/model.md) owns definition/use and scheduling requirements;
[lowering](../lowering/values.md#scalar-instruction-selection) owns concrete
instruction choices. Neither layer changes these source operand domains.

## Runtime object predicates

`value instanceof Lang.Class` produces Boolean for supported concrete classes:
Object, Number, Long, Float, Double, Boolean, Char, String, Array, Dictionary,
Symbol, Method, WeakReference, and ByteArray. Canonical `Toybox.Lang.Class` and
Lang module aliases also work. The class must satisfy the selected API
requirements.
[Imported type names](declarations.md#native-name-declarations) and qualified aliases resolve to
the same targets. Interfaces, unions, and parameterized class operands remain
outside this profile. Null is not an instance of these classes. This is a
runtime identity/inheritance test, not a conversion or an exact source-class
assertion.

`receiver has selector` produces Boolean for member existence. The receiver must
be non-null; a preceding supported null guard can establish that fact. The
selector must be a non-null Symbol, including a parameter or computed result.
Its identity is shared with ordinary field/method binding. The receiver is
computed before the selector, once each. The tested member is not invoked.
Source fields/methods and inherited native methods can be tested.

Both predicates have the existing comparison precedence below arithmetic and
above logical AND/OR; chained comparisons are unsupported. Ordinary
short-circuit and conditional evaluation rules apply. `has` establishes no
member-signature or API-availability proof. Guarded newer APIs, general union
refinement, and broader cast targets remain planned.

### Object refinement

A positive `instanceof` branch refines fresh reads of the tested local or
parameter. Object and Object-or-Null can narrow to the supported concrete
scalar, Dictionary, Method, Symbol, WeakReference, or ByteArray contract. An
Object test establishes non-nullness. An Array test also establishes
non-nullness, but cannot infer an Array element contract or enable indexed
access on an erased Object. Existing concrete storage contracts retain their
types, with Null removed when proven.

```text
using Toybox.Lang;
function length(o as Toybox.Lang.Object) as Toybox.Lang.Number {
    if (o instanceof Lang.String) { return o.length(); }
    else { return 0; }
}
```

The tested read and predicate must directly feed the branch in the same block.
Boolean negation reverses the proof edge; AND/OR and conditional expressions
carry facts through their emitted branches. False tests do not subtract types.
Entry has no facts. Joins retain only facts shared by every incoming edge; loops
include their entry and backedges. A local write invalidates its facts,
including a write between the tested read and branch. Unrelated writes and calls
preserve facts about local/parameter reference values.

Aliases and stored Boolean conditions do not transport proofs. New copies made
inside a proven branch receive the narrowed value type; storage that existed
before the test keeps its original assignment contract. Fields and arbitrary
expressions are not proof-bearing storage locations. Source superclass chains,
container elements, method signatures, and API guards require separate
contracts. Source and saved IR share these rules. See
[runtime evidence](../../investigations/compiler-object-tests.md).

### Source-class targets

`value instanceof ClassName` tests against a declared Object-, View-, or
BehaviorDelegate-derived source class. Targets resolve across all unit input
files, independently of declaration order. A successful test gives fresh reads
of the tested local/parameter the exact source identity and its native
projection, using the same branch, join, and write-invalidation rules above.
This enables source calls, exact signature/field contracts, and structural
comparator checking. Different classes sharing a native base remain distinct.
Null does not match.

`value as ClassName` is a lexical assertion of that same exact identity and
projection. It needs no proof and leaves runtime identity unchanged, including
for false assertions. A copy initialized from a proven/asserted source value
retains its exact class requirement. An opaque Object local narrowed only for a
branch keeps its original assignment contract; a later write invalidates its
proof.

Class operands are named declarations, not dynamic class values. The bounded
profile rejects local/parameter shadowing of a source target. Qualified/nested
source names, source inheritance, AppBase targets, and nullable source-class
assertions remain unsupported. Unit validation checks target identity and base;
standalone function IR retains that declaration-resolution obligation.

## Lexical casts

`expression as Lang.Type` explicitly asserts a value's static type. Supported
targets are Number, Long, Float, Double, Boolean, Char, String, Object, Symbol,
Method, WeakReference, ByteArray, Numeric, Integer, Decimal, NumberFormat,
Endian, and opaque Dictionary. Canonical `Toybox.Lang.Type` and Lang module
aliases are accepted. Number, Long, Float, Double, Boolean, Char, String,
Symbol, Object, Method, WeakReference, ByteArray, Numeric, Integer, and Decimal
targets can include `or Null`. Target types still satisfy selected API
introduction requirements.
[Source-class targets](#source-class-targets) additionally retain exact identity.
Imported native names use the same cast profile. Parameterized containers,
interfaces, general unions, Void, and Null alone remain outside this profile.

```text
using Toybox.Lang;
function length(o as Toybox.Lang.Object) as Toybox.Lang.Number {
    return (o as Lang.String).length();
}
```

The operand is evaluated once. The assertion produces the same runtime value,
without conversion, allocation, class lookup, or a check. It can remove Null or
assert a type unrelated to the operand's static type. The caller is responsible
for that assertion; a falsely asserted value can fail when subsequently used.
Runtime `instanceof` still sees the original value's class. A source object's
assertion to an opaque native type erases exact source-class provenance.

Casts bind below unary/member/index operations and above binary arithmetic and
comparisons. Chains associate left to right and share the 64-level expression
nesting limit. `o as Lang.Number + 1` asserts the left arithmetic operand;
negating an Object requires `-(o as Lang.Number)`. Parenthesize a cast before
member/index access. `or Null` belongs to its target; other `or` expressions use
ordinary logical precedence. Assertions in conditional arms, staged operands,
and native/user call arguments preserve existing evaluation and ownership rules.
A cast does not change a storage declaration or establish a branch/API proof.

See [SDK/runtime
evidence](../../investigations/compiler-object-tests.md#lexical-casts).

## Short-circuit expressions

Boolean `&&`/`and` and `||`/`or` support conditions and value-producing
expressions, including grouping and negation. Word spellings require complete
identifier tokens and have identical short-circuit behavior and precedence; type
annotations still use `or` for unions. Comparison binds before `&&`, which binds
before `||`; each binary level is left associative. Evaluate the left operand
once. `&&` evaluates the right only when the left is true; `||` evaluates it
only when the left is false. Every operand must be Boolean, even when a constant
left operand would skip it at runtime. No truth-value conversion or bitwise
behavior is performed by logical operators.

The builder emits explicit branches and intermediate blocks, with no eager
binary operation or cross-block expression values. Condition negation swaps
destinations. Nullable facts follow the selected edges; joins retain only facts
established on every incoming path. For example:

```monkeyc
if (!(value == null) && value > 0) { System.println(value); }
if (value == null || value > 0) { return 0; }
return value;
```

The right operand and true arm of the first condition know `value` is non-null.
The false path of the second condition knows the same. In contrast, `!(value ==
null) || flag` does not prove non-null on its true path.

Logical results are Boolean and may initialize/assign locals, feed calls,
returns, printing, comparisons, concatenation, and native receiver calls under
those consumers' normal type contracts. Value results initialize a temporary
Boolean local to false, assign true on the selected outcome, then read it at the
join. Earlier operands and receivers are evaluated and stored before later
operands branch; fresh temporary reads feed the final operation in source order.
This applies to expression and statement-call arguments alike. Temporaries have
distinct local identities and count toward the existing combined frame-slot and
operation limits. Slots are not reused.

Storing a logical result carries no null-proof relation to the checked source
storage. Guarded right operands may use non-null reads, but a later branch on
the result local does not narrow the original. Existing nesting/operation limits
apply to parsing and generated blocks. See the
[condition evidence](../../investigations/compiler-short-circuit.md) and
[value-result evidence](../../investigations/compiler-logical-values.md).

## Conditional expressions

`condition ? whenTrue : whenFalse` binds below logical OR and associates to the
right. The condition must be Boolean. Evaluate it once, then evaluate only the
selected arm. Both arms must type-check even with a constant condition. Preserve
operand/argument order when an enclosing expression has earlier effects.

Supported arm types match exactly: numeric scalars, Char, Boolean, String, Null,
or an invariant Array/typed Dictionary with supported content contracts.
Container arms must have exactly the same element or key/value types, including
content nullability and interface identity; selection preserves the chosen
reference without copying or conversion. Scalar alternatives may also join with
Null or their existing nullable type, producing the corresponding
scalar-or-Null. No Char-to-Number promotion, general heterogeneous union,
nullable Array reference, class join, Void arm, or non-Boolean truthiness is
supported. Expected container context from a declaration, assignment, call, or
return reaches direct literals and supported sized Array allocations in both
arms; no element contract is inferred solely from a sibling arm.

Join inference uses checked incoming operand types after the complete graph's
null-flow analysis. A direct nullable read under a non-null proof contributes
Number or String, so `n != null ? n : 0` has type Number. Nested joins and
copies of proven reads retain those checked types. Unproven reads remain
nullable; stored Boolean tests do not establish proofs, and local writes
invalidate them. Nullable results still require proof at non-null consumers.
Source construction and saved IR share these validation rules.

Lower through [typed IR joins](../ir/values.md#expression-joins). Stage results
and earlier operands when a consumer needs call/field/array stack setup; these
locals count toward the existing frame limit. A direct conditional return
carries its selected value on the VM stack without a staging local. Existing
nesting/node/operation bounds apply, including nested arms, calls, and arrays.
See the [example](../../../examples/compiler-conditionals/README.md) and
[SDK/runtime evidence](../../investigations/compiler-conditionals.md).
