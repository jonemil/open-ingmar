# IR values and storage

## Boolean and Number inversion

`.not` consumes one non-null Boolean, Number, or Long and defines the same type.
Integer inversion complements all 32 or 64 bits, as for `.complement`; nullable
operands need a non-null proof. Other types are rejected without coercion. The
operand must be an earlier definition in the same block and obey ordinary
one-use rules. Typed and text IR recompute the contract and include the operand
in effect traversal. Lowering evaluates it once and emits `invv`, without a
native binding. Negating a direct null comparison reverses its non-null proof
edge when used by a branch. See the [runtime
evidence](../../investigations/compiler-boolean-negation.md).

Short-circuit expressions use existing `.block`, `.branch`, and `.jump` forms,
not `.and`/`.or` directives. Each operand ends its own block; the right operand
is entered only on the required outcome. Negated source conditions may reverse
destinations without a `.not`. Every branch still requires Boolean; ordinary
block-local value, effect-order, and empty-stack edge rules apply. Text resumes
the explicit graph rather than reconstructing source operators. See the
[source profile](../frontend/operators.md#short-circuit-expressions).

Value-producing logical expressions also use `.boolean`, `.local`, `.assign`,
and `.read`. A temporary Boolean local is initialized before branching, assigned
true on the true outcome, and read after both outcomes join. Surrounding
operands evaluated before a later branch are stored in distinct temporary locals
and read in the final block. Temporary locals obey the same type,
initialization, frame-budget, and null-proof rules as source locals; no
producer-only metadata or new directive grants special treatment. These
logical-result edges have an empty stack. See the [value-result
evidence](../../investigations/compiler-logical-values.md).

## Number negation

`.positive` consumes one non-null numeric value and defines an unchanged result
of the same type. Concrete widths and numeric aliases follow the [source numeric
contracts](../frontend/numeric-values.md). Nullable operands require a checked
fresh read under a non-null proof; other types are rejected. Definitions,
single-use rules, block boundaries, spans, and effect order apply as for other
unary operations. Lowering emits the operand exactly once and no additional VM
instruction. This preserves type checking through saved IR even though
executable code is identity. `.integer` spelling still rejects a leading plus;
source `+1` uses `.integer 1` followed by `.positive`. See [unary-plus
evidence](../../investigations/compiler-number-plus.md).

`.negate` consumes one Number Long, Float, Double, or a supported numeric alias
and defines the same type. String, Char, Boolean, Null, and unchecked nullable
values are rejected. Operands must be earlier definitions in the same block and
obey ordinary one-use rules. Source and typed/text IR recompute the contract;
fresh reads with a direct non-null proof may feed it. Effect traversal evaluates
the operand once and retains call order. Lowering emits zero, the operand, then
`subv`; it requires no native member or new opcode. Minimum Number negation
wraps to the same i32 minimum in the tested runtime. See the
[evidence](../../investigations/compiler-number-negation.md).

## Floating constants and values

`.float` and `.double` produce Float/Double values from exact IEEE 754 bits.
Width, prefix, and hexadecimal digits are mandatory. Native
`Operation::Float`/`Double` hold u32/u64 bits, preserving signed zero,
infinities, and NaN payloads through text, equality, and binary round trips.
Saved IR accepts every bit pattern even though source literals must be finite.

`float`/`double` are parameter, user-call result, and expression-join tags;
`nullable-float`/`nullable-double` additionally serve as explicit function
result contracts. Shared source/IR validation applies the
[floating contracts](../frontend/numeric-values.md#numeric-values), including numeric
promotion, null proofs, native calls, and invariant storage types.

Lower positive zero with `fpushz`/`dpushz`; all other bits, including negative
zero, use `fpush`/`dpush`. These carry four/eight big-endian operand bytes.
Arithmetic and comparisons use existing VM operations. Negation emits same-type
positive zero followed by the operand and `subv`. No DATA constant is needed.

## Long constants and values

`.long` produces a Long without Number narrowing. Decimal text permits an
optional leading minus; source suffixes, leading plus, hexadecimal text, and
values outside i64 are invalid. `Operation::Long` holds i64; text/native
construction share the same scalar type and value-use checks.

`long` and `nullable-long` are parameter and user-call result tags. Functions
may infer uniform Long returns; `.function result=long` is not a separate field.
Unit calls must match callee signatures exactly, including Long versus Number.
Locals and printing retain the Long type; unsupported scalar operators and
container forms remain invalid. API checking uses `Toybox.Lang.Long`, since
1.0.0. Explicit nullable returns use `.function result=nullable-long`; native
Long methods and String conversions use the shared `.call-native`
representation.

Lower zero as `lpushz`; other values use `lpush` with one big-endian eight-byte
operand containing the signed value's two's-complement bits. The semantic IR
holds no split words or DATA address. Both source and saved IR use this
lowering.

Existing arithmetic and comparison directives accept the bounded Number/Long
pairs from the [source contract](../frontend/numeric-values.md#long-values).
They retain semantic `Binary`/`Compare` operations, choose Long results where
required, and use the existing VM arithmetic/comparison instructions without
conversion calls. Long-aware `.positive` preserves its operand; `.negate` uses
`lpushz`, the operand, and `subv`. Validated value types drive zero selection.
Number negation keeps `ipushz`. No arithmetic results or promotions are trusted
from text tags.

`.not` and `.complement` on Long retain Long and select `invv`. `.increment` and
`.decrement` preserve Long through ordinary read/add-or-subtract/write lowering,
using a Number one operand. Shifts use `shlv`/`shrv` and leave count
normalization to the runtime; they add no masking or conversion instructions.
Source and text validation recompute all these results, including a Long result
for a Number left operand with Long count. Statement-only update syntax and
storage invariance remain unchanged.

## Char constants

`.char` defines Char from raw u32 bits. Exactly eight digits follow lowercase
`0x`; canonical output uses lowercase digits. This is independent of the
narrower source literal syntax: surrogates and non-scalar bit patterns are
preserved. Validation gives the value the distinct Char type and requires API
1.3.0 when a context is supplied. Lowering emits `cpush` with the same bits,
without a DATA string or native conversion call. Invalid widths and malformed
operands are rejected. See the
[literal evidence](../../investigations/compiler-char-literals.md).

## Number and Char steps

`.increment` and `.decrement` add or subtract one from a non-null Number, Long,
Float, Double, or numeric alias. Increment also accepts Char (API 1.3.0); both
operations preserve the operand type. Typed construction uses `Operation::Step`
with an `increment` Boolean. Char decrement, other classes, and unproven
nullable operands are rejected. These are semantic value operations:
capture/read and target write-back remain explicit surrounding operations.
Source updates themselves are statements and expose no expression result.

[VM lowering](../lowering/values.md#number-and-char-steps).

## Numeric aliases

`numeric`, `integer-union`, and `decimal` denote the supported Numeric, Integer,
and Decimal aliases. `integer` continues to mean concrete Number. Their nullable
tags are `nullable-numeric`, `nullable-integer-union`, and `nullable-decimal`.
These are parameter, function-result, user-call-result, cast, and
conditional-edge tags; aliases introduce no runtime class or conversion.
Validation recomputes operator results, common native members, class
refinements, and current null proofs under the
[source rules](../frontend/numeric-values.md#numeric-values). A result annotation cannot
narrow an arithmetic union or establish a null proof. Lowering uses ordinary VM
operators; alias negation uses Number zero subtraction. Concrete scalar field
contracts use
[`.field-scalar`](../frontend/fields.md#typed-scalar-fields), preserving the same
scalar/nullability tags. Numeric alias field declarations reuse their tags
through
[`.field-numeric`](../frontend/fields.md#numeric-alias-fields); reads retain the
exact declaration and writes are checked against it after unit reload.

Array element tags also use `numeric`, `integer-union`, `decimal`, and their
nullable forms. Parameter/call-result tags append `-array`; conditional-edge
tags prepend `array-`. `.array-new`, `.array-begin`, `.array-allocate`, and
`.array-constant` retain the selected element contract. Dictionary contracts
reuse these scalar tags on either side of `=>`, excluding nullable keys. Native
methods and index operations recheck the
[source container rules](../frontend/numeric-values.md#numeric-alias-containers), including
invariance and null proofs, rather than trusting a saved result annotation.

## Local operations

`.local` initializes a local from an earlier value. `.read` defines a fresh
value from an earlier initialized local. Both accept the same optional source
span. Local IDs use `$` followed by decimal `u32`; value IDs still use `%`.
Initializations must have unique IDs; reads before initialization,
reinitialization, and parameters plus locals exceeding 255 are rejected. Unread
locals are accepted: their initializer still executes. Source names are resolved
by the frontend and are not stored in this provisional IR.

`.take` defines the same typed value as `.read` but transfers ownership out of
the local. The local becomes unavailable on that CFG path; later reads, takes,
or assignments fail until its initialization executes again (for example, on
another loop iteration). Joins require availability on every incoming edge. A
take is ordered with other effects and cannot move across writes or calls. Each
resulting value still has exactly one use.

[VM lowering](../lowering/values.md#local-operations).

Local IDs are semantic identities, independent of their numeric value or VM
slot. The backend assigns local slots after parameters in initialization order
and executes every initializer before proceeding. Repeated reads may refer to
one local, while each resulting value still has one use. Pure-only functions use
dependency traversal while locals retain their initialized value. Effectful
functions also obey the [shared schedule](model.md#validation). `.assign` writes
a local without repeating initialization.

```text
ciq-ir
.function probe
.integer %10 42
.local $99 %10
.read %20 $99
.read %30 $99
.subtract %40 %20 %30
.return %40
.end
```

## Parameter metadata and reads

The optional `parameters=<nonnegative-decimal-count>` field on `.function`
defaults to zero, with a maximum of 254. `Function::with_parameters(name, count,
operations)` uses the shared validator; `Function::new` retains zero-parameter
behavior. `Function::with_parameter_types` supplies explicit contracts.
`.parameter` defines a fresh value with the signature position's type, narrowed
under the [null-check proof](#nullable-results) when available. Index must be
below the parameter count; repeated reads and unused declared parameters are
allowed. Source parameter names are resolved by the frontend, not stored in IR.
Position is independent of value/local IDs and VM slots.

Call argument IDs refer to earlier value definitions and count as uses; their
order is significant. The call-containing evaluation-order validator includes
argument trees and nested calls. Up to 254 arguments are encodable; this is a
structural bound, not a verified device maximum. Old zero-argument `.call` and
literal/locals IR remain compatible. Parameters and locals together cannot
exceed 255, preserving u8 slot indexes.

```text
ciq-ir
.function difference parameters=2
.parameter %0 0
.integer %1 3
.add %2 %0 %1
.local $0 %2
.read %4 $0
.parameter %5 1
.subtract %6 %4 %5
.return %6
.end
```

## String values and print statements

`.string` defines a string value; `.println` consumes a Number, Long, Float,
Double, String, Boolean, or Char without defining a result. Both accept optional
spans. Quotes preserve spaces, `#`, and text resembling a span. Escapes follow
[IR quoting rules](grammar.md#text-and-value-definitions); commas outside quotes are rejected.
The semantic string limit is 65535 UTF-8 bytes. String types propagate through
locals, function parameters, user calls, and returns. Arithmetic follows the
documented operand contracts. Value returns must have the same supported type;
Concrete scalar results are inferred from compatible return values, unless a
nullable result is declared as specified below. Void returns have no value. All
existing use/dependency/ order rules apply.

```text
ciq-ir
.function probe
.string %0 "Hello # from Rust"
.println %0
.integer %2 43
.return %2
.end
```

The IR string directive names a semantic value. Lowered assembly `.string`
directives name DATA requirements, deduplicated by content within the function:

```text
.string __ciq_string_probe_0 "Hello # from Rust"
probe:
argc 0x01
getmv @__ciq_sdk_System @__ciq_member_println
frpush
news &__ciq_string_probe_0
invokem 0x02
popv
ipush1 0x2b
return
```

For complete section-project assembly, put literal declarations in the
**authoritative DATA source**, removing them from CODE. Existing DATA records
can be reused by changing CODE aliases explicitly; otherwise append unplaced
strings to DATA and update the manifest length. CODE and IR/text paths consume
the same DATA bindings. See
[reproduction](../../investigations/compiler-strings.md) for source → IR → assembly
→ signed PRG and native CODE parity.

## Local assignment

Source local compounds use ordinary `.read`, a binary directive, and `.assign`.
There is no compound IR directive. Shared validation checks operand contracts,
result assignability, null proofs, and read-before-RHS/write ordering. Staged
RHS expressions use the existing typed local/snapshot representation. See
[source rules](../frontend/statements.md#local-compound-assignment) and
[evidence](../../investigations/compiler-compound-locals.md).

`.assign` consumes an earlier value and replaces an initialized local. No
explicit blocks are required for a straight-line assignment body.

```text
ciq-ir
.function probe
.integer %0 40
.local $0 %0
.read %1 $0
.integer %2 2
.add %3 %1 %2
.assign $0 %3
.read %4 $0
.return %4
.end
```

The target must be initialized on every incoming path. `.assign` does not
introduce an identity, count as another local, or establish initialization.
Repeated `.local` declarations for one ID remain invalid. Local IDs retain their
initializer's supported type; assignments must match it. Undefined
values/locals, wrong-path targets, shared values, and cross-block RHS values are
rejected. The RHS contributes one use; assignment defines no result. Each write
counts toward the 1024-operation limit and reuses the existing slot.

If a function contains any assignment, its entire operation sequence must match
left-to-right expression evaluation and ordered statement sinks, as for calls
and prints. A read defined before a write and used after it is rejected, since
dependency lowering would move the read past the write. Store a snapshot in a
separate initialized local before the write when that behavior is needed. This
rule applies to native typed construction and text resumption equally.
Assignment-free pure functions permit flexible definition order.

[The source subset](../frontend/statements.md#local-reassignment)
specifies name resolution and supported statement forms; source names/scopes are
not part of this IR.

## Expression joins

Conditional expressions use a single typed block parameter, independently of
mutable locals. Native IR uses `BlockParameter { result, value_type }`
immediately after `Block`; incoming edges use `JumpValue { target, value }`.
Text syntax is:

```text
.block ^3
.block-value %7 integer
.return %7
```

Each predecessor must end in `.jump-value`, consuming one assignable value in
that predecessor. Plain jumps and branch targets cannot enter a parameterized
block. The entry block cannot have a parameter; a block accepts at most one.
Missing targets, malformed placement, type/arity mismatches, duplicate
definitions, and unused/reused values are errors in native and text
construction. The parameter defines a fresh block-local value; the predecessor's
value ID does not cross the edge.

Type tags are `integer`, `long`, `float`, `double`, `char`, `boolean`, `string`,
`null`, `byte-array`, `nullable-byte-array`, `nullable-integer`,
`nullable-long`, `nullable-float`, `nullable-double`, `nullable-string`,
`nullable-boolean`, `nullable-char`, `symbol`, `nullable-symbol`, `method`,
`nullable-method`, `weak-reference`, and `nullable-weak-reference`. Array tags
prefix the [allocation element tag](containers.md#array-allocation) with
`array-`, for example `array-nullable-number`, `array-string`,
`array-comparable`, or `array-view`. All supported element contracts have a
corresponding join tag. Incoming Arrays must match the invariant element
contract exactly; non-null and nullable elements do not widen to a common Array
type. Tags describe element nullability, not nullable Array references.
Availability checks apply to the parameter and its incoming values, including
Char's API 1.3.0 requirement.

[VM lowering](../lowering/control-flow.md#expression-joins).

Source joins use matching supported types or scalar/Null alternatives; see
[conditional expressions](../frontend/operators.md#conditional-expressions) for
source inference and restrictions. Source construction resolves each join from
checked incoming operand types, including full-graph null proofs and local
writes. It stores the resolved type explicitly in the immutable function and its
text output. Native/text IR retain their declared parameter types and check
incoming values against them; parsing does not infer or silently narrow them.
Native IR may declare a compatible nullable parameter explicitly. Nullable edge
proofs still concern source storage and intersect at joins; an incoming value
does not create a proof about that storage. See [SDK
evidence](../../investigations/compiler-conditionals.md).

## Nullable results

`.call-native` for String `find`, `toNumber`, and `toNumberWithBase` defines
Number-or-Null according to its API contract; String `substring` defines
String-or-Null. String `toLong` and `toLongWithBase` define Long-or-Null;
`toFloat`/`toDouble` define their corresponding scalar-or-Null. Index API
requirements are the same as the [source
contract](../frontend/types.md#nullable-results). The existing `.local`,
`.read`, `.null`, `.equal`, `.not-equal`, and `.branch` forms carry this
behavior; no assertion or unchecked cast operation is introduced. Equality with
Null is accepted in either order.

Shared IR validation computes non-null local and parameter facts from control
flow. A branch whose condition is a direct `.equal` between `.read` or
`.parameter` and `.null` establishes that storage identity's fact on its false
edge. A direct `.not-equal` check establishes it on the true edge instead. A
chain of `.not` definitions around either comparison reverses the proof edge
once per operation. Every definition in the guard chain must precede its
consumer in the branch's block. The checked read must be in that block; a
checked local must have no intervening write. Both outcomes reaching the same
target provide no new proof. Entry has no external facts; joins intersect
incoming facts and every initialization/assignment kills that local's proof. The
fixed-point analysis handles backedges without treating a later guard as proof
for a loop's first entry.

A fresh read of a nullable scalar local or parameter under this proof defines
its non-null type; otherwise it defines the original union. Storage remains
nullable. Local assignment can supply Null, the non-null type, or the same
union, but always requires another proof before non-null use. Existing
effect-order, one-use, and block-local value rules apply. Stored Boolean guards
and comparisons against null-valued locals are not proof forms. Nullable values
may use matching user parameter/result contracts, but cannot use non-null
consumers without proof. Nullable Numbers may be passed as substring indices,
which explicitly accept that union. Readers and typed constructors recompute all
facts; IR text carries no trusted narrowing metadata. See
[source examples](../frontend/types.md#nullable-results).

## Nullable user signatures

A function declaration may carry `result=nullable-integer`,
`result=nullable-long`, `result=nullable-float`, `result=nullable-double`,
`result=nullable-string`, `result=nullable-boolean`, `result=nullable-char`, or
`result=nullable-symbol`. Every value return must be Null, the corresponding
scalar type, or the same union. A void return is invalid. The declared union is
preserved even when all returns are scalar or all are Null; canonical text
always writes it. Omitted result metadata infers a uniform scalar/Void contract
and rejects nullable/Null returns. Reference result tags follow the
[native value contracts](calls.md#native-value-calls). Unsupported result tags are errors.
Parameter lists independently accept the same nullable type names.

```text
ciq-ir
.function identity parameters=1 parameter-types=nullable-string result=nullable-string
.parameter %0 0
.return %0
.end
```

`Function::with_signature` and `method_with_signature` supply explicit result
contracts to the shared validator. Parameter-only constructors infer scalar/Void
results. Unit validation accepts Null, the corresponding scalar, or the same
union for nullable arguments; scalar arguments still require exact types. Call
result contracts must equal the callee's declared result, even when its body
always returns a scalar. This prevents edited call metadata from hiding nullable
results. Standalone IR has no callee signature environment.

Parameter reads retain their declared union until a direct branch proves the
parameter non-null. Later reads under that proof define the scalar alternative.
Local and parameter identities are distinct; neither a copied value nor another
parameter is narrowed by the original's proof. Parameters have no write
operation, so unrelated local writes preserve parameter facts. No casts or new
VM instructions are introduced. See the
[complete example](../../../examples/compiler-nullable-contracts/README.md).

## Runtime object predicates

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

Both operations define Boolean and accept optional source spans. `InstanceOf`
records a canonical concrete runtime class independently of symbol IDs. Function
validation checks the supported class and API level. `Has` requires a non-null
receiver and a Symbol selector. Each operand has one semantic use; native and
text input share these checks. A direct `.instanceof` of a `.read` or
`.parameter` can establish a branch proof for fresh reads, including through
`.not` and ordinary control-flow edges. The read, test, and branch must be in
the same block with no intervening local write. Entry, join, write invalidation,
and projection rules follow the [source
profile](../frontend/operators.md#runtime-object-predicates). `.has` establishes
no member-signature or API-availability proof.

Both typed construction and text parsing recompute the complete CFG fixed point;
branch proofs carry no trusted text annotations. Source selection uses candidate
facts from the prefix built so far, then submits the complete function to the
same validation. Backedges and joins can invalidate those candidates. No cast or
assertion operation is inserted. Narrowed value types guide existing native call
checks and numeric instruction selection.

[VM lowering](../lowering/values.md#runtime-object-predicates).

ByteArray tests use `Toybox.Lang.ByteArray` with API 3.0.0. Positive Object
refinement yields the concrete ByteArray contract, enabling indexed operations
as well as native methods under the shared proof rules.

## Lexical casts

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

`Cast { result, operand, target }` consumes one value and defines a value with
an explicitly asserted API type. Native construction and text parsing check the
supported target and its API availability; they do not require assignability
from the operand or a branch proof. This explicit assertion is separate from
recomputed control-flow facts.

Target tags are `integer`, `long`, `float`, `double`, `boolean`, `char`,
`string`, `object`, `symbol`, `method`, `weak-reference`, `byte-array`,
`numeric`, `number-format`, `endian`, and `dictionary`. Supported nullable
targets use `nullable-` tags. Types and their source obligations follow the
[source cast contract](../frontend/operators.md#lexical-casts).

[VM lowering](../lowering/values.md#lexical-casts).

## Source-class tests and casts

[Directive syntax](grammar.md#operation-syntax) is defined in the grammar.

`SourceInstanceOf` and `SourceCast` carry a typed `SourceClass` target with
separate name and base fields. Function validation checks target syntax, native
projection, and API requirements. Unit validation resolves the declaration and
requires both identity and base to match. Forward and cross-file targets work.
Standalone parsing retains that unit obligation; text cannot provide trusted
branch facts or bypass declaration/base checks.

Positive source-class tests participate in the same CFG fixed point as native
class tests. Fresh reads retain both the native value type and exact source
provenance. Casts establish the explicitly asserted source identity without a
proof. The [source contract](../frontend/operators.md#source-class-targets) owns
scope, assignability, and caller obligations. Class facts stay typed in memory;
text is only the save/resume representation.

[VM lowering](../lowering/values.md#source-class-tests-and-casts).
## Arithmetic definitions

Constants and unary/binary operations use the [grammar's value
directives](grammar.md#values-and-storage). Operands follow the [source operator
contracts](../frontend/operators.md) and
[numeric-width/alias rules](../frontend/numeric-values.md); saved result claims
cannot bypass promotion or non-null requirements. Concrete floating and integer
classes remain distinct. Binary operations define a new single-use value and
consume two earlier block-local values; unary operations consume one.

Division/remainder remain runtime operations, including literal zero divisors;
their possible failure requires evaluation-order checking. No constant folding,
coercion, or inserted exception guard changes that behavior. Integer complement
retains a distinct unary operation, including repeated complements, rather than
being rewritten as binary XOR. Its result preserves Number/Long width; floating,
Char, Boolean, and unproved nullable operands are rejected. Numeric aliases
require every alternative to satisfy the operator. See
[lowering](../lowering/values.md) for concrete instruction selection.

```text
ciq-ir
.function probe
.integer %0 40
.integer %1 8
.integer %2 5
.subtract %3 %1 %2
.add %4 %0 %3
.return %4
.end
```

Boolean constants use exactly `true` or `false`, producing Boolean rather than
Number zero/one or Null. Typed locals, calls, returns, branches, equality, and
printing preserve that identity. Booleans are not integer operands. Raw scalar
constant widths and source-literal restrictions remain separate; see the
[constant grammar](grammar.md#values-and-storage).
