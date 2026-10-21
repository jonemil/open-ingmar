# Monkey C frontend

[Grammar](grammar.md) defines accepted Monkey C syntax; this reference owns
meaning, validation, and implementation requirements.

The frontend parses a restricted Monkey C profile, resolves declarations and
native names, and constructs validated semantic IR. This is a supported subset,
not a replacement definition of the complete Monkey C language.

The default source entry point accepts one function. Unit mode collects multiple
files and owners before checking cross-function references. Neither mode
resolves Jungle projects. Diagnostics identify input byte offsets and one-based
line and column; columns count Unicode scalar values. Callers supply filenames.
Unsupported input is diagnosed rather than silently producing partial functions.

Source contracts are independent of [IR serialization](../ir/grammar.md),
[VM lowering](../lowering/README.md), and [application generation](../build/README.md#complete-application-generation).
Native API declarations and higher-level application callbacks belong in the
[SDK specifications](../sdk/README.md). The [plan](../../plan.md) owns remaining
language and library work, including exceptions and source inheritance.

Contents:

- [Types and nullability](#types-and-nullability)
- [Numbers and numeric aliases](#numbers-and-numeric-aliases)
- [Declarations and name resolution](#declarations-and-name-resolution)
- [Source classes](#source-classes)
- [Instance fields](#instance-fields)
- [Expressions and operators](#expressions-and-operators)
- [Statements and control flow](#statements-and-control-flow)
- [Core Lang values and methods](#core-lang-values-and-methods)
- [Arrays](#arrays)
- [Dictionaries](#dictionaries)
- [ByteArray](#bytearray)
- [Structural interfaces](#structural-interfaces)

## Types and nullability

### Function type contracts

Omitted ordinary parameter and result annotations select Any. Any includes Null;
arguments, assignments, and results may cross supported typed boundaries without
a generated conversion or guard. Operations on unknown receivers/operands defer
type failures to runtime. Concrete invalid operations remain compile errors.
The [IR dynamic contract](../ir/README.md#dynamic-values) defines these shared
rules. Annotations accept the
[value-type domains](grammar.md#type-syntax), resolved through canonical names,
imports, and module aliases. Constructible source-class annotations preserve
[exact identity](#source-class-signatures). Native signatures also accept the
[union contracts](#native-unions) below.

An unannotated function exposes Any even when every return has one concrete
type. A body with no value returns supplies Null on bare return or fallthrough;
it remains usable as a value. Explicit `as Void` bodies produce no semantic
value. Value-returning bodies require value returns on every returning path:
explicit value/Null mixtures are accepted under Any, while value/bare-return
mixtures and value/fallthrough paths are rejected, matching the tested SDK.
Each value return must be assignable to its contract: a nullable result accepts
its non-null type, Null, or the same union. Known unrelated concrete types remain
errors under an explicit contract; Any crosses that boundary dynamically.
A structurally non-returning body, such as `for (;;) {}`, may retain any
supported declared result; an unannotated non-returning body retains Any.
Callers use the declared result; no interprocedural non-returning-call analysis
is implied. Typed IR constructors infer uniform
scalar/Void returns unless an explicit signature is supplied; IR defaults are
independent of source defaults. Constructors retain their implicit Void result,
and native callbacks retain their independently resolved signatures; annotations
cannot override callback contracts.

Bare `Any` is not accepted source annotation syntax in the tested SDK profile;
omitted annotations provide the dynamic contract.

### Native unions

Parameter and result annotations accept `A or B` chains of supported native
value types, including Null and existing parameterized Array/Dictionary types.
Bare Null is also a signature contract. Fields and lexical `as` assertions keep
their separately specified type domains. [Exact source-class unions](#source-class-signatures)
retain identities separately and cannot yet mix with native alternatives.

Normalization flattens alternatives, removes duplicates, and orders them
consistently. Numeric aliases and existing nullable types retain their established
representations. Any absorbs other value alternatives; Void is never a union
alternative. No subtype subsumption or implicit scalar conversion is performed.
A union admits assignment only when every source alternative fits its contract.
Array elements and Dictionary keys/values remain invariant: a union of container
references does not widen their mutable content types.

Calls, local writes, returns, and conditional joins preserve native alternatives.
Each possible receiver must support a member/index operation and accept its
arguments. Operators check every possible operand pairing; their result is the
normalized union of the possible results. Thus Number-or-String supports `+ 1`
with a Number-or-String result, but not `- 1`. A nullable receiver needs a
non-null proof before ordinary member/index use. API availability is checked
for every alternative, including unused signature contracts.

Direct local/parameter `instanceof` tests retain matching alternatives on the
true edge and exclude covered alternatives on the false edge. Direct Null tests
likewise distinguish Null from non-Null. Negation reverses the edges; writes
invalidate proofs. Aliases and stored Boolean tests do not transfer proofs.
Empty intersections do not introduce a bottom type or mark code unreachable.

### Nullable results

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
assignments of non-null values. A subsequent read uses the contracts reaching
that point; a non-null write may itself establish a non-null inferred type.
Explicit parameter/field contracts retain their declared nullability.

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
the scalar type directly. Parameter writes invalidate facts for that parameter;
fresh reads need a new proof before non-null or narrowed uses. Local writes
do not invalidate parameter facts, even when their numeric
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

## Numbers and numeric aliases

### Integer literals

Unsuffixed integers produce Number; `l`/`L` selects Long. Decimal magnitudes
must fit the signed 32-bit or 64-bit range after applying a literal's sign,
including the minimum signed value. Lowercase `0x` introduces hexadecimal;
multiple integer digits beginning with zero select octal. Hex digits may use
either case; octal digits exclude 8 and 9. Binary prefixes, uppercase `0X`, and
digit separators are rejected. See the connected [literal grammar](grammar.md#lexical-rules-and-literals).

Hexadecimal/octal magnitudes encode the full width's bit pattern, interpreted
as signed. Thus `0xffffffff` is Number -1, but `0xffffffffL` is Long 4294967295.
More than 32/64 significant bits is rejected. Negating such a literal wraps
within its width: `-0xffffffff` is 1 and `-0x80000000` remains the minimum
Number. This does not broaden decimal range or define general arithmetic
overflow. Signed zero integer literals become zero. Saved IR retains canonical
signed decimal values; source prefixes/suffixes do not extend IR or assembly
syntax. [Literal evidence](../../investigations/compiler-number-literals.md#radix-and-floating-token-probes)
records the SDK checks.

### Floating-point values

Float and Double are distinct numeric scalars, available since API 1.0.0.
Decimal literals with a fractional part or decimal exponent default to Float;
lowercase `f` explicitly selects Float, including integer mantissas such as `1f`.
Lowercase `d` selects Double, including `1d`. A fractional part requires digits
after the point; leading-dot forms such as `.5` and `.5d` are accepted. A
multi-digit integer part beginning with zero, uppercase `F`/`D`, and trailing
points are rejected. An exponent uses
`e`/`E`, optional sign, and at least one digit. Literal parsing rounds to IEEE
754 binary32/binary64. Values rounding to infinity are rejected; underflow may
round to zero. A negative literal preserves signed zero. Parenthesize a negative
literal before calling a member, as for Number/Long.

`NaN` is a Float constant with bits `0x7fc00000`; `-NaN` retains that canonical
payload in the verified SDK profile. It is not a Double literal or a spelling
accepted in saved IR, which uses the ordinary hexadecimal Float bits. Arrays,
locals, calls, and comparisons use the existing Float contracts; no special
operator or VM opcode is introduced. Runtime propagation, conversion, and
comparison behavior remain runtime obligations.

Explicit Float/Double parameters/results, local storage, module/instance calls,
printing, conditional joins, and `or Null` contracts use the shared scalar
rules. Assignments, arguments, returns, and conditional arms do not implicitly
convert numeric widths. Arrays and Dictionaries follow their shared scalar
contracts;
[scalar fields](#typed-scalar-fields) preserve these types as well. String concatenation and
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
[shared Object argument profile](#object-equality-and-container-observers)
and distinguishes floating widths and integer classes. Do not infer a
cross-instance Double hash contract from numeric equality. `compareTo` follows
the shared [Object argument contract](#comparison-object-arguments) and returns
Number since 5.0.0. String `toFloat()` returns Float or Null since 1.0.0;
`toDouble()` returns Double or Null since 3.1.0. Guard nullable results before
scalar use. Conversion content and formatting are passed through to the runtime.

See the [grouped example](../../../examples/compiler-float-values/README.md) and
[encoding/runtime evidence](../../investigations/compiler-floating-values.md).

### Long values

Long is a signed 64-bit scalar, distinct from Number, available since API 1.0.0.
Its [integer literal rules](#integer-literals) cover decimal signed magnitudes
and radix bit patterns, selected with `l` or `L`. A minus immediately applied
to a Long token forms a negative literal under the same spacing/postfix
restrictions as Number. Unary
plus and negation also accept Long expressions, preserving their width.

Explicit `Toybox.Lang.Long` parameters/results, local copies/reassignment,
module/instance calls, discarded calls, repeated construction, and
`System.println` preserve all 64 bits. Long and Number contracts do not
implicitly convert. Unannotated source returns still require Number; inferred IR
returns can carry Long.

Long/Long and mixed Number/Long pairs support `+`, `-`, `*`, `/`, and `%`.
Either Long operand makes the result Long; Number/Number keeps Number. Promotion
belongs to these operators only: arguments, returns, and fixed storage still
require their declared type. Inferred locals may store a promoted Long result,
including through compound assignment with Number/Long operands. Nullable numeric
operands require a non-null proof.

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
[shared rules](#native-value-calls).

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
[floating contracts](#numbers-and-numeric-aliases). Long arrays and Dictionaries follow their
shared scalar contracts;
[scalar fields](#typed-scalar-fields) also support Long. See the
[example](../../../examples/compiler-long-values/README.md) and
[encoding/runtime evidence](../../investigations/compiler-long-values.md).

### Numeric values

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
every alternative and retain the normalized result contract, including `toString`,
`format`, and numeric conversions. Direct `instanceof Number/Long/Float/Double`
branches refine fresh alias local/parameter reads under the existing branch
rules. Nullable aliases require a current non-null proof before arithmetic,
printing, or member calls. Writes invalidate that proof; a saved Boolean guard
is not a proof for later reads.

Conditional joins infer the narrowest supported numeric alias covering both
arms, including concrete Number/Long or Float/Double alternatives; adding a
Null arm makes it nullable.
[Class fields](#numeric-alias-fields) retain these same alias
contracts. Other supported native alternatives use the shared
[union rules](#native-unions).

#### Numeric alias containers

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

## Declarations and name resolution

### User calls and owner lookup

Qualified calls resolve to a top-level user module in the compilation unit.
Value calls require a supported non-Void result. Source bodies and ownership
metadata are collected before call checking; callers need not precede callees.
Arguments must satisfy the declared types and positional arity: a nullable
parameter accepts Null, its scalar type, or the same union; scalar parameters
require exact types. Nested modules remain unsupported. Calls through proven
source-instance receivers follow the [class
contracts](#source-classes). Void module calls are supported as
[statements](#void-functions-and-statement-calls). The separate
[System.println statement](#strings-and-printing) prints supported
scalars. Module/member names stay symbolic until encoding; neither IDs nor DATA
method records are generated here.

An unqualified `member(arguments)` resolves to a declared function in the current
module, or a declared method on the current instance. Forward references and
recursion use the collected signatures. A local/parameter with the same spelling
does not replace that declared call target. A local named after the owning module
also does not change an unqualified call; explicit qualified calls retain their
existing qualifier-shadowing rules.

If no declared owner member exists, instance bodies can use the supported
inherited Object methods, including reflective `method(...)`, under their native
contracts. For this inherited fallback, lexical name conflicts remain rejected.
Otherwise, unqualified lookup falls back to a collected global function. Global
bodies use that same global lookup directly. A declared owner member keeps
priority even when its signature rejects the arguments; an incompatible owner
call does not select a global overload. A same-named instance field blocks
fallback and is diagnosed as an unsupported callable value. Calls through
lexical callable values and unresolved names remain unsupported.

`$.member(arguments)` selects the global function explicitly, bypassing owner
lookup and local/parameter names. `$.Module.member(arguments)` selects a function
in a top-level user module with the same protection from qualifier shadowing.
Both forms retain ordinary call signatures, argument contexts, and effect order.
The module form resolves to existing module-call IR; a root prefix is not needed
after source names have been resolved. Instance methods cannot be selected as
module functions through the root. Global value reads, nested module/native
paths, root-object values, and class/static paths remain outside this profile.
Unqualified resolution requires collected unit
declarations; explicit calls in standalone IR/CODE retain external-callee
obligations.

User calls share declared arity, argument/result checking, contextual literal
construction, and effect order with qualified calls. They become ordinary
global/module/self call IR; Void results are statement-only and non-Void statement
results are discarded. See [lookup evidence](../../investigations/compiler-unit.md#owner-relative-calls).

IR contains `ModuleCall { result, result_type, module, member, arguments }`.
[Call text](../ir/README.md#module-calls-and-operation-order) preserves the
result contract. Calls may have effects. For any function containing a call, IR
operations must appear in the backend's left-to-right expression evaluation
order, with initialization and print statements in order. Reject incompatible
schedules rather than silently reordering effects; pure-only IR keeps its
existing ordering flexibility. This conservative restriction can change when a
more general scheduling policy is implemented.

[VM lowering](../lowering/README.md#user-module-calls).

### Parameters and positional arguments

Parameters use the [type/default rules](#types-and-nullability). Identities are
zero-based positions, distinct from locals/value IDs; repeated reads define
fresh values. Duplicate names and local/parameter shadowing are rejected. Unit
validation checks callee arity and argument/result contracts.

Parameters support statement assignment, compound assignment, and supported
increment/decrement operators. A write preserves the declared parameter domain
(including Any for an omitted annotation), exact source identity, and container
invariance. Interface writes require a checked implementation. It replaces the
callee's parameter value without rebinding the caller's variable. Replacing a
reference differs from mutating the referenced object, whose aliases still
observe ordinary object mutations.

Evaluate the RHS before storing; earlier reads keep their snapshots and later
reads observe the replacement. Writes invalidate that parameter's null/class
refinements, including on loop backedges. They use the existing parameter slot
and allocate no source local. See [assignment evidence](../../investigations/compiler-assignments.md#parameter-writes-and-local-annotations).

[IR parameter operations](../ir/README.md#parameter-metadata-and-reads)
retain positional identity and typed argument order; the [IR
grammar](../ir/grammar.md) owns saved metadata. [Frame/call
lowering](../lowering/README.md) maps those identities to the observed
hidden-context convention and u8 slot/arity bounds. The [argument
specimen](../../investigations/compiler-arguments.md) verifies nested calls and
observable argument order, not maximum device capacity.

### Native name declarations

Default and unit source modes accept these declarations before functions or
owners at the start of a file, and before methods at the start of a
class/module:

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

`using` introduces a module binding, optionally aliased. `import` introduces the
module name and its directly declared type names known to the API catalogue:
classes, interfaces, and enums. Bare native names require that import; qualified
module names and aliases resolve through the same catalogue. For example,
`import Toybox.Lang;` permits `Number` in annotations, casts, and runtime class
tests, while `using Toybox.Lang as L;` permits `L.Number`.

Resolution also covers supported Array element and Dictionary key/value type
arguments. Each consumer retains its own executable profile: recognizing
Comparator does not make it a runtime class, and recognizing Array does not
enable an unparameterized cast. API introduction checks use canonical types;
imports neither filter names by API level nor bypass those checks.

The same environment resolves base clauses and supported constructors: `import
Toybox.Application;` permits `extends AppBase`; module aliases permit `extends
App.AppBase`. Existing canonical short module paths such as
`Application.AppBase` and `System.println` remain available without
declarations. Fully qualified Toybox paths are also accepted.

Bindings apply to following declarations in their file and are inherited by
class/module bodies. Owner-local bindings do not escape their owner; bindings
are not shared between files, including reopened source modules. Imports in the
body cannot resolve that class's superclass clause. This is a bounded frontend
profile, not a specification of all Monkey C namespace behavior.

Reject duplicate bindings, shadowing by parameters/locals/functions/owners,
aliases of reserved identities, import aliases, declarations after functions,
and native modules outside this profile. Imported type names also conflict with
module aliases in either declaration order. Bare imported module functions and
constants, general source namespace aliases, and types outside the catalogue
remain unsupported. Number conversion uses
[inherited lookup](../sdk/README.md#declaration-model) within executable profiles. Calls on ordinary user modules
and `self` keep their existing rules. Supported native spellings resolve to the
same semantic operation or superclass identity before IR construction; aliases
are not serialized as VM symbols or executable import indexes.

[IR printing](../ir/README.md#string-values-and-print-statements) and
[literal lowering](../lowering/README.md#native-name-declarations) use resolved
native identities; source import aliases are not VM symbols.

[VM lowering](../lowering/README.md#native-name-declarations).

### Instance class extension

Unit mode also accepts [Object-, AppBase-, View-, and BehaviorDelegate-derived
class declarations](#source-classes). Their methods may call
`self.<member>(...)` as typed value expressions or statements; class ownership
and receiver contracts remain explicit in typed/saved IR. Default
single-function source mode does not accept self calls. Source initialization
and onStart use the
[native lifecycle contracts](../sdk/README.md#source-startup-callback).
Null literals and opaque nullable callback-state locals support equality checks;
[object/field contracts](#instance-fields) apply alongside the supported core Lang
profiles. Source inheritance and unrestricted native calls remain unsupported.
See [runtime evidence](../../investigations/compiler-class.md).

### Compilation units

This profile collects multiple restricted functions before lowering CODE. It is
independent of application profiles; the
[console application builder](../build/README.md#complete-application-generation) supplies a complete PRG
from this unit, while Jungle resolution remains future work. The
[function subset]() and [function IR](../ir/README.md#model-and-validation)
remain the per-function contracts.

### Source declarations

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

Inputs are one or more source files in caller-supplied order. Global functions
and flat module members are collected in file/declaration order. Modules may be
reopened within or across files; each `(module, function-name)` identity must be
unique. Global functions use a separate owner. Reject duplicate globals and a
global function whose name matches a declared module. `System` is reserved as an
intrinsic receiver and cannot be declared as a module.

All function bodies use the existing grammar and limits. Qualified user calls
must resolve to a module member in this unit, regardless of declaration order.
Call argument counts must exactly match the callee's parameter count.
Parameter/result domains, nullable assignability, and unannotated defaults
follow the [function type contracts](#types-and-nullability). These are
supported-profile rules, not general Monkey C type inference.

AppBase.onStart/onStop have a [native nullable-state
contract](../sdk/README.md#source-startup-callback). Their parameter types and
void result must match the resolved callback signature; neither can be invoked
as an ordinary source callee.

View.onShow/onUpdate/onHide use the [View callback
contracts](../sdk/README.md#application-callbacks). The enclosing class's native
base selects the callback signature. Source constructors must resolve to a
declared Object-rooted, View, or BehaviorDelegate class with a validated
initializer. Modules and AppBase classes cannot be constructed.

Class containers may declare the supported [instance fields](#instance-fields).
The unit rejects duplicate or conflicting members and resolves every field
operation against its class, independently of shared member symbol IDs and
source-file order.

Constructor calls resolve initialize as a source callee and check all arguments.
Reference fields must be assigned in an initializer prefix before reads, self
calls, control flow, or return. Field read/result claims and assignments must
match the owning declaration; generic native references cannot forge a source
field contract.

Calls on constructed source references resolve to their proven class's declared
methods. Function validation preserves exact constructor identity through local
copies and same-class assignment; unit validation checks ordinary method
contracts and prohibits native callback invocation through any receiver. See the
[instance-call profile](#calls-on-constructed-source-objects).

Exact source-class signatures resolve across the whole unit before call
checking. Every source parameter/result name must designate a constructible
declared class; arguments and returned references retain that exact identity.
Native base signatures erase it. Function and call text preserve these contracts
with `source(ClassName@Native.Base)` tags; see the
[source signature profile](#source-class-signatures).

AppBase.getInitialView has a [one-view positional
contract](../sdk/README.md#source-initial-view-callback). Its return shape and
zero parameters are validated for source, typed, and saved IR. It also cannot be
invoked as an ordinary source callee.

The [native name profile](#native-name-declarations) defines supported
`using`/`import` declarations and lexical boundaries. Native bindings are
resolved before unit call validation; user declarations still collect across
files before calls are checked.

Forward and recursive calls are accepted; termination is not proven. Calls to
undeclared external modules/members are rejected. `System.println` remains the
separate intrinsic. Global functions can serve as entries and checked call
targets under [owner lookup](#user-calls-and-owner-lookup). General imports, nested/empty modules,
optional/default parameters, overloading, and arbitrary SDK calls remain
unsupported.

The unit contains 1..=256 functions total across all files. Function operation,
nesting, parameter/local, literal, and control-flow limits remain unchanged. No
total CODE or device-memory budget is inferred from this declaration limit.
Source failures identify the input-file position and a location within it.

### Class extension

[Source class rules](#source-classes) define supported native bases, explicit
instance receiver contracts, and self-call validation. `classes()` exposes
ordered class declarations; `with_classes` validates them alongside ordinary
functions. Existing `new` and `declarations()` retain their global/module roles.

The [IR unit contract](../ir/README.md#compilation-units-and-classes) owns
receiver-bearing serialization and cross-body checks. [Unit
lowering](../lowering/README.md#unit-layout) owns grouped CODE order and labels.
Complete applications select exactly one AppBase class plus supported helpers;
unit validity alone does not select an application.

## Source classes

This profile adds Object-, AppBase-, View-, and BehaviorDelegate-derived classes
to
[compilation units](#declarations-and-name-resolution).
Existing function parameter and result rules still apply. The
[application builder](../build/README.md#complete-application-generation) selects one AppBase class and binds constructed source
classes, including additional Views/delegates. See [runtime
evidence](../../investigations/compiler-class.md).

### Source and ownership

[Accepted declaration syntax](grammar.md#declarations) is defined in the grammar.

Classes are top-level declarations. The base is a recognized native identity,
resolved through the [native name profile](#native-name-declarations). It
accepts canonical/qualified, aliased, and imported Object/AppBase/View/
BehaviorDelegate spellings. An omitted base selects Lang.Object. Supported
[field contracts](#instance-fields) are defined separately. Static methods,
annotations, source superclass lookup, and nested classes are unsupported.
Object-rooted classes may be empty; other classes require at least one method.
Class names must be unique; classes cannot be reopened. Reject
class/module/global-function name collisions.

Reserved class names include language keywords, `self`, `System`, `Application`,
`AppBase`, `globals`, `statics`, `Toybox`, and the bootstrap's pinned native
names: `initialize`, `println`, `View`, `Object`, `onStart`, `getInitialView`.
These restrictions belong to this constrained profile, not the general Monkey C
language.

`self` is reserved and cannot be rebound. In an instance body it may be a call
or declared-field receiver, or produce an opaque Object value. Bare member calls
and class-name calls remain unsupported. Calls on constructed source objects
follow the profile below. Module calls from methods retain their existing rules.

A self expression call requires a declared value-returning method in the same
class. A self statement call discards any supported result, including Void.
Arguments follow scalar/nullable assignability and result contracts must equal
the callee's contract; counts must match exactly. Forward and recursive
references are checked after collecting signatures. Self lookup never falls back
to a global/module member. Object-rooted classes may use the supported inherited
Object members described below; source declarations keep priority. Recursion is
accepted but termination is not proven.

### Calls on constructed source objects

`new ClassName(arguments)` establishes an exact constructor class alongside its
native Object, View, or BehaviorDelegate value contract. Local initialization
and copies preserve this identity. A local initialized from a source
construction may subsequently receive only a reference with the same proven
constructor class; assignments from another source class, a generic native View
parameter, or an erased return are rejected. Different instances of the same
class are permitted, including conditional and loop assignments. Local aliases
refer to the same runtime instance; reassignment changes only the local
reference.

`receiver.method(arguments)` calls a declared method of the proven source class.
The receiver may be a construction, local/copy, or typed source
parameter/result. Forward method declarations and multiple source files are
supported. Value calls must match the declared result contract; statement calls
discard any supported result, including Void. Arguments use the existing
scalar/nullable/array contracts and limits. Receiver evaluation precedes
argument evaluation, in source order. Empty Array arguments receive context from
the selected source signature.

Unit validation rejects missing methods, wrong argument/result contracts, and
ordinary calls to native lifecycle callbacks, even through a different receiver.
Object-rooted classes may resolve supported inherited Object members below.
Other undeclared native/source inheritance remains unsupported. Source
inheritance, arbitrary receiver field access, and general dynamic dispatch
remain unsupported.

Exact constructor identity is a flow-sensitive local refinement, not a new
native API type. Generic View parameters, declared View returns, and Array<View>
element reads expose only their base contract. They cannot authorize
source-method calls. Assigning a known source instance to a local establishes
its identity for subsequent reads; joining different classes erases that exact
identity until a runtime type test establishes it. No runtime cast is emitted.

`ObjectCall` and `ObjectCallStatement` record a receiver value, class name,
member, and arguments; value calls also record the result contract. Function
validation reconstructs source identity from construction, source signatures,
and local operations and requires an exact match with the claimed class. Unit
validation resolves the declaration. This prevents saved IR from forging a
source class for a native reference.

[VM lowering](../lowering/README.md#calls-and-objects).

### Object-rooted classes

`class Counter { ... }` and `class Counter extends Lang.Object { ... }` declare
Object-rooted source classes. The base also accepts `Toybox.Lang.Object`, Lang
module aliases, and imported bare Object. Object construction requires API
1.0.0. A declared `initialize(arguments)` must return Void and must not call a
native base initializer. When omitted, the frontend supplies a zero-argument
Void initializer with an empty body. This also permits empty Object-rooted
classes. The default becomes an ordinary explicit method in IR and counts
toward the unit's 256-body limit; saved IR does not infer omitted methods.
Construction checks its zero-argument signature. Field defaults still come
from declaration metadata; fields requiring explicit initialization remain
rejected unless a source initializer fills them under the ordinary field rules.
Declarations inside the class cannot shadow its reserved Object base binding.
Direct native `new Object()` remains outside this profile.

Number fields, supported constructor arguments, local aliases/reassignment, self
calls, and calls on a proven source instance use the ordinary class rules.
Inherited `toString`, `equals`, `hashCode`, and `method` dispatch through the
runtime Object contract when no source override exists. Bound Methods retain the
instance. A source class with the required `compare(Object, Object) as Number`
method can serve as an Array comparator. Weak targets retain their separate
bounded profile.

Source `hashCode` and `equals` overrides must exactly match the native Object
signatures: `hashCode() as Number` and `equals(Object or Null) as Boolean`. Unit
validation checks declarations even when unused, so interface/Object erasure
cannot hide an invalid method reached by Dictionary dispatch. Inherited
implementations need no declaration. This proves call compatibility, not stable
hashes or consistency between equality and hashing. Other native override
families retain their existing restrictions.

The runtime value projects to opaque Object, while construction and locals
retain the exact source identity for own-member calls and structural checking.
It may pass through ordinary Object contracts, but erasure loses own-member
access. Exact source annotations and reference fields preserve the Object-rooted
class and its Object projection. Source tests/assertions can recover exact
identity after erasure. Object-element Arrays erase exact source identities
under the
[container contract](#object-containers). Exact source-class Array
elements and source superclass chains remain unsupported.

Complete application DATA gives each class an Object base reference and a
return-only hidden allocation constructor. Its explicit or default `initialize`
runs through normal source construction. The [application
profile](../build/README.md#complete-application-generation) owns import
binding; no native Object allocation/initialization hook is invoked. See the
[example](../../../examples/compiler-root-objects/README.md) and
[evidence](../../investigations/compiler-root-objects.md).

### Source-class signatures

Ordinary functions and instance methods may annotate parameters and results with
unqualified constructible source class names joined with `or`, optionally
including Null:

```text
function pass(view as HelloView) as HelloView { return view; }
function maybe(view as HelloView or Null) as HelloView or Null { return view; }
function either(value as Token or Marker) as Token or Marker { return value; }
```

The name must resolve to a declared Object-, View-, or BehaviorDelegate-derived
class in the unit, including forward or cross-file declarations. AppBase
references, arbitrary native classes, Array<ClassName>, aliases, and qualified
source-class names are unsupported.
Native annotations use canonical, qualified-alias, or imported bare names
through the shared native type environment.

Source contracts preserve exact class identity. Arguments and every value return
must retain the declared source class through construction, typed parameters,
declared results, local copies, positive source-class tests, or explicit source
assertions. The [source-target contract](#source-class-targets) distinguishes
runtime proofs from caller assertions. A different source class, Null, or an
erased native reference cannot satisfy a non-null contract. Native contracts may
accept a source instance through its base while erasing its exact class identity.

Nullable source contracts accept that exact class, its nullable form, or Null;
they reject unrelated source classes and erased native references. Parameters,
declared call results, and local copies retain both class and nullability.
Source dispatch and non-null source consumers require a non-null proof on the
read, using the shared [branch rules](#runtime-object-predicates). Checking an
alias does not prove the original storage non-null. Writes invalidate that
local's proof; joins and loop backedges retain only shared facts.
A local initialized from a nullable source value exposes that contract until
flow establishes a different one. A fresh copy inside a proven branch has a
non-null contract. Checks refine reads without rewriting storage; assignment
updates an inferred local's contract, including class identity and nullability.
Nullable source Array elements remain unsupported.

A source-class union retains a sorted, deduplicated set of exact class/native-base
identities and optional Null. Its signature may contain up to 254 alternatives.
Different native bases are permitted. Assignments, arguments, and returns must
retain a subset of the declared classes; Null requires a nullable contract.
Opaque native references cannot satisfy an exact class alternative. Any retains
its separate gradual boundary rules. Mixing source and native alternatives is
unsupported, and union fields and general union casts remain outside this profile.

Conditional joins and inferred local writes preserve these exact alternatives.
Direct source `instanceof` tests select a class on the true edge and exclude it
on the false edge; nested tests can select from three or more alternatives.
Negation and loop exits use the same facts. Null tests preserve the remaining
class set. Aliases and stored Booleans do not transfer facts, and writes kill them.
There is no bottom type: contradictory tests do not establish unreachable code.

Source method dispatch requires a single proven, non-null class. A guard can
establish it for a parameter or fresh local read. An unrefined union cannot yet
call a common source-declared member even if every class declares it. Supported
native methods use the union's native projection and existing receiver checks.
Structural interface consumers require every retained class to implement the
required signature, including after saved IR reload. A nullable source contract
can satisfy a nullable interface, but not a non-null interface without a proof.
When a callback can dispatch to multiple classes, initialization/shutdown effect
checking remains conservative rather than selecting one class's body.

Module, self, and explicit object calls retain declared source alternatives,
permitting subsequent checked narrowing and dispatch. Parameter reads retain
their declared alternatives. Native callbacks keep their inherited
signatures; source annotations cannot replace those contracts. No runtime cast
is emitted.

Source contracts carry exact names and native bases separately; the
[IR model](../ir/README.md#source-construction-and-native-constants) owns their representations. Source declaration collection resolves both across all input files
before checking bodies. API checking projects to that base; exact source
assignability is checked separately. Function validation checks names and return
provenance, while unit validation checks both class identity and native base
against declarations, call arguments, and result claims. Standalone
typed/function IR retains its class/callee resolution obligations.
Single-function source compilation rejects source-class annotations because it
has no declarations from which to resolve their native bases.

Saved parameter tags and function/call result tags use
`source(ClassName@Native.Base)`, `nullable-source(ClassName@Native.Base)`, or
`source-union(source(A@Native.Base)|source(B@Native.Base)|null)`.
Result contracts must be explicit in saved
function text to retain the source identity. See the [signature
example](../../../examples/compiler-source-signatures/README.md) and [runtime
evidence](../../investigations/compiler-source-signatures.md).

### Source initialization

A declared class may supply `function initialize(...)`. It must return Void.
Object-rooted classes have no native base call; other classes call the native
base initializer exactly once as their first statement. AppBase initializers
take no parameters; View/delegate initializers accept the ordinary supported
parameter contracts and limit. These are subset restrictions, not general
language rules. Subsequent supported statements may print or call source
methods. Initialization cannot be selected as the generated onStart entry.

AppBase base calls are `AppBase.initialize()`,
`Application.AppBase.initialize()`, fully qualified
`Toybox.Application.AppBase.initialize()`, and equivalent module aliases. Bare
AppBase here identifies the inherited superclass even without an import; this
member-call rule does not make bare AppBase legal in an `extends` clause.
Arguments, value uses, other base members, and base calls outside a class
initializer are rejected. Conditional, late, duplicate, or absent base calls are
also rejected by unit validation.

View initializers use `View.initialize()`, `WatchUi.View.initialize()`, or the
equivalent fully qualified/aliased identity. The called base must match the
class's declared base, including in saved IR.

`NativeBaseInitialize { base }` is an ordered effect with an implicit current
instance and no explicit arguments/result. It identifies a native superclass,
not a source callee or executable import index. Lowering selects `getselfv
AppBase; getsv initialize; invokemz; popv`. The application builder binds native
aliases and directs the initializer's single DATA method record to source CODE.
AppBase retains its default initializer when source omits one. Constructed
source Views require an explicit initializer. Default lifecycle/View bodies are
used only where source does not supply them.

BehaviorDelegate initializers use `BehaviorDelegate.initialize()` or a
recognized qualified/aliased spelling, following the same native-base-first
rule.

## Instance fields

[IR field syntax](../ir/grammar.md#unit-and-class-framing) records declarations
before methods. Native and saved units check the same field names, kinds,
read/result claims, assignment contracts, required initialization, and API
availability. Reloading text cannot bypass these checks. Per-kind rules below
specify defaults, identity, ownership, and permitted values.

### Number instance fields

Source classes may declare up to 256 fields with Number
literal initializers. These inferred fields have a fixed Number contract; typed
declarations follow the [scalar-field contract](#typed-scalar-fields). Other
inferred numeric literals and computed declaration initializers are unsupported.
Parentheses around a literal are accepted. These restrictions and the
field-count limit define our subset, not a device memory limit.

Each field name must be a valid, non-reserved identifier, unique within its
class. Reject names shared with source methods or catalogue-known inherited
methods, and `View`, `AppBase`, or `init`, which collide with
native-base/constructor metadata. Different classes may use the same field name.

```text
var count = 7;
function bump() { self.count = self.count + 1; }
```

Methods read a field with `self.<name>` and replace it with `self.<name> =
<expression>;`. The RHS must produce Number and is evaluated before the write. A
local or parameter with the same name remains separate; bare names keep existing
local/parameter lookup rules. Assignment is a statement, not a value expression.
Number fields also support all ten arithmetic/bitwise compound operators:
`self.count += rhs;` reads the old field once before RHS effects, applies the
corresponding Number binary operation, then stores the result. A RHS that
changes the field does not change the captured old value. Branching RHS
expressions stage the old read through ordinary typed locals. Non-Number results
are rejected. Arbitrary receiver properties, static fields, and source-inherited
fields remain unsupported. See
[target evidence](../../investigations/compiler-compound-targets.md).

`ClassDeclaration.fields` holds names and FieldKind defaults/contracts. Unit
validation resolves every SelfFieldRead/SelfFieldAssign against its enclosing
class; standalone method IR retains this ownership obligation. Function
validation requires an instance receiver, supported assignment values, and
expression evaluation order; unit validation enforces the exact field
declaration. A read that must survive a subsequent write/call is stored in a
local first. The same checks apply to typed and saved IR.

Application generation records each field with key bits zero, type `0x01`, and
the initial Number's 32-bit pattern. Fields are sorted with other class members.
The runtime supplies a separate field value for each constructed instance;
constant initialization precedes its source initialize method. Reads lower to
`getselfv`; writes to `getself`, `spush`, RHS instructions, then `putv`. See the
[counter example](../../../examples/compiler-fields/README.md) and
[instance isolation evidence](../../investigations/compiler-fields.md).

### Typed scalar fields

All supported source class bases accept explicit Number, Long, Float, Double,
Boolean, Char, and String fields, including `or Null` forms. Canonical names,
imports, and module aliases resolve the same contracts. Fields retain their
exact scalar class without implicit conversion; numeric promotion in an RHS must
still produce a type assignable to the field. Operators and native methods
follow the ordinary scalar rules. Char declarations require API 1.3.0; the other
scalar classes require 1.0.0. Base classes retain their own availability.

Typed fields start as Null metadata slots. A non-null declaration requires
assignment in `initialize` under the
[required-field prefix rules](#source-reference-fields). Nullable fields may
omit a default or use literal `null`. Other declaration initializers remain
outside this profile. Shared member-name and 256-field limits apply.

Nullable reads need a guarded local snapshot before non-null use; testing one
mutable field read does not refine a later read. Clearing the field leaves
earlier snapshots intact. Writes, compound assignments, and statement updates
use the same type and evaluation-order rules as locals. Each instance owns its
slots independently.

See the
[example](../../../examples/compiler-scalar-fields/README.md) and
[evidence](../../investigations/compiler-scalar-fields.md).

### Numeric alias fields

All supported source class bases accept fields declared as `Lang.Numeric`,
`Lang.Integer`, or `Lang.Decimal`, including `or Null` forms. Canonical names,
imports, and aliases resolve to the same contracts. Their scalar alternatives
and operator rules follow the [numeric value
contract](#numbers-and-numeric-aliases).

Initialization, Null metadata, snapshots, evaluation order, and field limits
follow the [typed-field rules](#typed-scalar-fields).

Reads retain the declared alias and nullability. Writes must satisfy every
possible alternative of the RHS, without implicit numeric conversion. Numeric
accepts all four scalar classes, Integer accepts Number/Long, and Decimal
accepts Float/Double; non-null fields reject nullable values without a proof.

See the
[example](../../../examples/compiler-numeric-fields/README.md) and
[evidence](../../investigations/compiler-numeric-unions.md#field-storage).

### Object and container fields

All supported source class bases accept `Lang.Object` and `Lang.Object or Null`
fields. Values erase to Object without conversion; scalars, known enums, source
objects, and supported native/container references retain their runtime values.
Concrete source identity, member signatures, and container parameters are not
recovered from the field declaration. Use the ordinary class proofs or lexical
assertion contracts for more specific consumers. Nullable reads require fresh
local snapshots and guards.

Non-null `Lang.Array<T>` and `Lang.Dictionary<K,V>` fields use the existing
supported element/key/value contracts, including nullable contents and numeric
aliases. Reads, writes, mutation, and method calls retain invariant container
types. Existing arrays/dictionaries cannot widen through assignment. Direct
literals receive field construction context, including empty literals and
conditional arms; this does not broaden unrelated literal inference.

Initialization and field limits follow the [typed-field
rules](#typed-scalar-fields). Object fields may have literal Null defaults only
when nullable. Containers require assignment in `initialize`; declaration
defaults, opaque unparameterized Dictionary fields, and nullable whole-container
fields remain unsupported. Payload types retain their API requirements even for
unused fields.

Fields hold strong references. Replacing a field releases its ownership, while
other aliases, snapshots, entries, or enumerations can keep the value alive.
Transfer conditional values out of their CFG join before field receiver setup;
hidden staging slots must not extend ownership after their final use.

See the
[example](../../../examples/compiler-container-fields/README.md) and
[evidence](../../investigations/compiler-container-fields.md).

### Source-reference fields

`var target as HelloView;` declares an exact source reference field for an
Object-, View-, or BehaviorDelegate-derived class. The class must resolve across
the unit, including forward declarations. The 256-field limit and name/conflict
rules cover all supported fields together. `var target as HelloView or Null;`
declares a nullable slot; it may also use a literal `= null` default. Non-null
source fields reject declaration initializers. Comparison interfaces follow their
[own field rules](#comparison-interface-fields).

Metadata creates a null slot (key bits zero, type `0x00`, value zero). Nullable
source fields need no source assignment before use. A source initialize method
is required to establish a non-null field contract. It must
assign every required non-null reference field before exposing `self` as an
Object, binding a self callback, making a self call, branching, jumping, or
returning. A reference read before that field's assignment is rejected.
Assignment must use the exact declared source class; native erasure, another
class, and Null cannot satisfy it. Requiring this straight-line initialization
prefix is our bounded checking policy, not Garmin's complete
definite-initialization rules. Calls that may request System.exit, including
transitive source calls and constructors and source comparator callbacks reached
through Array sorting, are also rejected until all required fields are
established. Erased Method invocation, interface calls, and sorting through
nullable/non-null Comparator values are conservatively included when the exact
callback body is unavailable. This conservative call-graph check includes
conditional exits; runtime failures and additional native callback effects are
not proven safe by it.

After initialization, methods may read or replace the field with the same exact
source class. Field reads retain identity for ordinary instance calls and source
returns. Access remains limited to `self.<field>`; a getter can forward the
reference to another caller. Reads/writes preserve the existing evaluation-order
rules. These contracts apply to typed and saved IR as well as source.

Nullable writes accept the exact declared source class, its nullable form, or
Null. Other classes and native erasure cannot satisfy the contract. Each read
is a nullable snapshot retaining that class identity; copy it to a local and
check the local before source dispatch. Checking `self.target != null` does
not refine a subsequent field read, since it is a different read. Clearing or
replacing a field leaves existing snapshots intact. Typed getter results can
forward the nullable source contract through ordinary calls.

See the
[retained View example](../../../examples/compiler-reference-fields/README.md) and
[constructor/lifecycle evidence](../../investigations/compiler-reference-fields.md).

View lifecycle callbacks follow the [SDK callback
contracts](../sdk/README.md#source-views-and-drawing).

### Comparison interface fields

Source classes support `var value as Toybox.Lang.Comparable;` and the
corresponding Comparator declaration, optionally `or Null`. Module aliases and
imports resolve the type through the existing name rules. All require API 5.0.0,
including unused declarations; source and saved-IR API entry points check field
types independently of method references. Fields share the existing name,
conflict, and 256-field limits. Access remains limited to `self.<field>`.

Metadata creates ordinary Null slots (type `0x00`, value zero). A non-null field
requires a source initializer and follows the straight-line
[reference-initialization requirements](#source-reference-fields). It cannot
be read until assigned, and must be filled before self exposure, self calls,
control flow, return, or calls that may request shutdown. Nullable fields start
as Null and do not require assignment; an explicit literal `= null` is
equivalent. Other declaration initializers remain unsupported.

Writes accept the corresponding interface, a verified native witness, or an
exact source implementation under the
[interface contract](#structural-interfaces). Nullable
fields additionally accept Null and compatible nullable values. Unit validation
checks required source members on every write, including after text reload.
Nonnull fields reject nullable writes without a non-null proof. A read's saved
result contract must exactly match its field declaration. Function IR alone
checks supported operands; the declaring unit proves the field's actual type.

Reads retain the declared interface contract, rather than the last stored source
class. Non-null reads can call interface methods directly. A nullable read needs
a local snapshot and null guard for member calls: testing one field read does
not refine another mutable field read. Clearing or replacing the field does not
change a previously captured local value. Nullable Comparator reads can supply
Array.sort directly. These rules do not add field/class intersection proofs,
general native field types, or container field declarations.

See the [comparison example](../../../examples/compiler-comparators/README.md)
and
[evidence](../../investigations/compiler-comparators.md#interface-fields).

### Method fields

All supported source class bases accept annotated `Toybox.Lang.Method` and
`Method or Null` fields from API 1.0.0, with canonical/imported/aliased names:

```monkeyc
var required as Toybox.Lang.Method;
var optional as Toybox.Lang.Method or Null = null;
function initialize(callback as Toybox.Lang.Method) {
    self.required = callback;
}
```

Both slots start as Null in metadata (type `0x00`, value zero). A non-null field
requires assignment in source `initialize`, following the required-field prefix
and self-exposure restrictions of [reference fields](#source-reference-fields).
Binding `self.method(...)` exposes self and cannot fill the first required slot.
Nullable fields may omit their initializer or use literal `null`; computed
defaults remain unsupported. Writes must preserve the declared
Method/nullability contract. Methods retain their erased invocation contracts.

Reads retain the declaration's type. Nullable invocation/member access requires
a guarded local snapshot; testing one mutable field read does not refine
another. Replacing or clearing a field does not alter saved aliases. Each stored
Method retains its bound receiver until all owning aliases are released.
Ordinary instance isolation, field name/count limits, and evaluation order still
apply.

See the
[specimen](../../../examples/compiler-methods/README.md) and
[ownership evidence](../../investigations/compiler-methods.md#nullable-ownership).

### WeakReference fields

All supported source class bases accept annotated `Toybox.Lang.WeakReference`
and `WeakReference or Null` fields from API 1.2.0, including unused
declarations. Canonical, imported, and aliased type names are accepted. Both
slots start as Null metadata (type `0x00`, value zero). Non-null fields require
assignment in source `initialize`, with the existing required-field prefix and
self-exposure rules. Nullable fields may omit initialization or use literal
`null`; computed defaults are unsupported.

Reads and writes preserve declared nullability. Nullable field members need a
guarded local snapshot; a test of one mutable field read cannot refine another.
Stored weak references and their aliases do not retain the target. A saved
`get()` Object result is a separate strong reference, governed by the
[ordinary weak-reference contract](#ordinary-weak-references).
Immutable scalar `weak()` results cannot satisfy an ordinary WeakReference
field.

See the
[specimen](../../../examples/compiler-weak-references/README.md).

### ByteArray fields

All supported source class bases accept `Toybox.Lang.ByteArray` and `ByteArray
or Null` fields from API 3.0.0, including unused declarations. Canonical names,
imports, and aliases work. Both slots start as Null metadata (type `0x00`, value
zero). Non-null fields require assignment in source `initialize` under the
required-field prefix and self-exposure rules. Nullable fields may omit a
default or use literal `null`; computed defaults are rejected.

Reads/writes preserve the declared reference and nullability. Ordinary Arrays
cannot satisfy ByteArray fields. Methods and indexed updates accept non-null
field reads; nullable fields need a guarded local snapshot. Testing one field
read does not refine another mutable read. A snapshot retains its object after
the field is replaced or cleared. Fields hold strong references; clearing the
last strong reference allows an existing weak target to expire.

See the
[example](../../../examples/compiler-byte-arrays/README.md) and
[evidence](../../investigations/byte-arrays.md#source-and-ir-profile).

### Symbol fields

All supported source class bases accept Symbol fields from API 1.0.0:

```monkeyc
var required as Toybox.Lang.Symbol;
var maybe as Toybox.Lang.Symbol or Null;
var optional as Toybox.Lang.Symbol or Null = null;
var preset as Toybox.Lang.Symbol = :alpha;
var inferred = :size;
```

Annotations accept canonical, imported, and aliased Symbol names. Literal Symbol
defaults are permitted with either nullability; an omitted type infers non-null
Symbol. Nullable fields default to Null when no Symbol is supplied. Non-null
fields without defaults must be assigned in source `initialize`, using the same
straight-line prefix and exposure/callback restrictions as required reference
fields. A literal default already satisfies initialization. Null requires an
explicit nullable declaration; other/computed defaults remain unsupported.

Reads retain the declared Symbol/nullability type, including when a nullable
field has a Symbol default. Writes must satisfy that contract. Nullable reads
need guarded local snapshots for members and selectors; a test of one mutable
field read does not refine another read. Replacing the field does not change a
saved local value. Field names, count limits, assignment order, and instance
isolation follow the existing class rules.

Metadata defaults use type `0x08` with a Symbol ID; other slots use Null type
`0x00` and value zero. Application generation resolves default names in the
shared Symbol map and retains development names even when no CODE literal
mentions them. Native names reuse pinned IDs. AppBase and constructed classes
share this encoding. Reads/writes use existing field instructions.

See the
[specimen](../../../examples/compiler-symbols/README.md) and
[evidence](../../investigations/compiler-symbols.md#symbol-fields).

## Expressions and operators

### Arithmetic and lowering contract

[Expression grammar](grammar.md#expressions-and-precedence) owns precedence,
associativity, prefix/postfix syntax, and grouping. Operators evaluate operands
once, left to right, under the type/result contracts here. Unary plus preserves
a checked numeric value without conversion; updates and compound writes follow
the [statement rules](#statements-and-control-flow).

Number division truncates toward zero; remainder has the dividend's sign.
Multiplication uses the Number VM instruction; representative overflow probes
wrap to signed 32-bit values. Zero divisors are runtime failures, including
literal zero; the compiler does not fold arithmetic or insert exception guards.
The SDK 9.2.0 simulator exits when the minimum Number is divided by, or has
remainder taken against, -1. This boundary does not establish a portable result
or a catchable Monkey C exception. See [product
evidence](../../investigations/compiler-number-products.md).

Number-only bitwise operators use non-null signed 32-bit bit patterns. Mixed
Long operations follow the [Long contracts](#long-values). Floating, Char,
Boolean, String, and nullable operands without a proof are rejected by bitwise
operations. `>>` is an arithmetic right shift in the tested VM. Complement
retains a unary IR operation and lowers to `invv`, evaluating its operand once.

For Number-only shifts, counts are non-null Numbers. Mixed Long shifts follow
the [Long contracts](#long-values). The compiler passes counts unchanged,
without range checks or masking. SDK/generated probes for -1, 32, and 33 agree
with selecting the low five count bits; this is evidence for the tested VM, not
a portable guarantee for other runtimes. See [bitwise
evidence](../../investigations/compiler-number-bits.md).

Unary plus is retained as a checked identity operation in IR and emits no VM
instruction. It does not parse text, convert types, or relax null proofs. The
SDK probe passes other value types through too; non-numeric types remain outside
our current profile. See [unary-plus
evidence](../../investigations/compiler-number-plus.md).

#### Operator contracts

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

Supported null checks follow the [nullable rules](#nullable-results). These
contracts are independent of method lookup and VM instruction selection. API
declarations do not broaden accepted operator combinations or cause heap
allocations to be emitted.

Unary negation accepts non-null Number, Long, Float, and Double, plus checked
numeric aliases under their [numeric contracts](#numbers-and-numeric-aliases).
Fresh reads under a validated direct null guard are accepted. It evaluates the
operand once and uses [typed-zero
subtraction](../lowering/README.md#scalar-instruction-selection). Negating the
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

The [IR model](../ir/README.md#model-and-validation) owns definition/use and
scheduling requirements;
[lowering](../lowering/README.md#scalar-instruction-selection) owns concrete
instruction choices. Neither layer changes these source operand domains.

### Runtime object predicates

`value instanceof Lang.Class` produces Boolean for supported concrete classes:
Object, Number, Long, Float, Double, Boolean, Char, String, Array, Dictionary,
Symbol, Method, WeakReference, and ByteArray. Canonical `Toybox.Lang.Class` and
Lang module aliases also work. The class must satisfy the selected API
requirements.
[Imported type names](#native-name-declarations) and qualified aliases resolve to
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
member-signature or API-availability proof. Guarded newer APIs and broader cast
targets remain planned. Native union refinement uses direct `instanceof` and
Null tests, independently of `has`.

#### Object refinement

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

#### Source-class targets

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

`value as ClassName or Null` asserts the same exact class while retaining Null
in its contract. It preserves the operand, including Null or a falsely asserted
runtime class. A local initialized from this assertion retains nullable source
identity; check that local before dispatch, or use an explicit non-null source
assertion. This is a caller assertion, not runtime validation or conversion.

Class operands are named declarations, not dynamic class values. The bounded
profile rejects local/parameter shadowing of a source target. Qualified/nested
source names, source inheritance, and AppBase targets remain unsupported.
Unit validation checks target identity and base;
standalone function IR retains that declaration-resolution obligation.

### Lexical casts

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

### Short-circuit expressions

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

### Conditional expressions

`condition ? whenTrue : whenFalse` binds below logical OR and associates to the
right. The condition must be Boolean. Evaluate it once, then evaluate only the
selected arm. Both arms must type-check even with a constant condition. Preserve
operand/argument order when an enclosing expression has earlier effects.

Native arms use the [union rules](#native-unions), retaining each selected
value/reference without copying or conversion. Different invariant container
contracts can remain separate union alternatives; neither content type is widened.
Void arms and non-Boolean truthiness remain unsupported. Expected container context from a declaration,
assignment, call, or
return reaches direct literals and supported sized Array allocations in both
arms; no element contract is inferred solely from a sibling arm.

Exact source references join as [source-class unions](#source-class-signatures),
retaining each identity and optional Null. A source/native mixture still requires
explicit erasure before joining. Neither a union nor a shared native base proves
one source class for dispatch; class-specific use needs a checked singleton.

Join inference uses checked incoming operand types after the complete graph's
null-flow analysis. A direct nullable read under a non-null proof contributes
Number or String, so `n != null ? n : 0` has type Number. Nested joins and
copies of proven reads retain those checked types. Unproven reads remain
nullable; stored Boolean tests do not establish proofs, and local writes
invalidate them. Nullable results still require proof at non-null consumers.
Source construction and saved IR share these validation rules.

Lower through [typed IR joins](../ir/README.md#expression-joins). Stage results
and earlier operands when a consumer needs call/field/array stack setup; these
locals count toward the existing frame limit. A direct conditional return
carries its selected value on the VM stack without a staging local. Existing
nesting/node/operation bounds apply, including nested arms, calls, and arrays.
See the [example](../../../examples/compiler-conditionals/README.md) and
[SDK/runtime evidence](../../investigations/compiler-conditionals.md).

## Statements and control flow

### Local declarations and reads

`var <identifier> = <expression>;` initializes a local with a supported value
contract. `var value;` declares a local without assigning a default value.
A declaration list shares one `var`, as in `var a, b = 2; a = b;`.
Each initializer executes and binds its name before the next is checked or
executed. Items may have different supported value types; each retains its own
local identity. Calls, container literals, and their internal
commas remain ordinary initializer expressions. The whole list belongs to the
current scope; it creates no nested scope. Expressions may read locals repeatedly
after initialization. Duplicate
declarations in one scope, shadowing an enclosing local or parameter,
self/forward/unknown reads are rejected. An arm's
locals are visible within that arm and nested arms, but not in a sibling or
after the condition. Disjoint arms may reuse a name; each declaration receives a
distinct semantic local and VM slot. A later outer declaration may also reuse a
name that has gone out of scope. A standalone braced block introduces the same
local scope: declarations disappear on exit, and writes to enclosing locals
persist. Empty and nested blocks are allowed. Scope exit does not reuse local
slots or add runtime reference-release instructions. A block preserves its
body's return/break/continue behavior and does not create a loop-control target.
Reassignment follows the flow-sensitive rules below.

Reads require assignment on every executable incoming path; declaration alone
does not establish a readable Null. A plain assignment may initialize declared
storage; compound assignments and updates still require an assigned value.
Both arms may supply the first value before a join, and a `do` body may assign
before its post-test. A possibly skipped `while` body cannot establish a value
after the loop. Re-executing a declaration in a loop resets assignment status.

Local types are inferred from values reaching each read, including sequential
assignments and branch/loop joins. A write replaces the local's current inferred
contract; it does not change earlier snapshots. Joins retain compatible exact,
nullable, numeric-alias, or comparison-interface contracts. Other supported
native alternatives retain normalized unions. Exact source alternatives retain
source-class unions; source/native mixtures still use opaque projections, which
cannot recover an unproven source identity.
An unused unassigned declaration has an inert
Null storage contract in saved IR, with no readable runtime default.
Explicit `var name as Type = value`
annotations are rejected, matching SDK 9.2.0; these are distinct from supported
parameter/result/field annotations and expression casts. See the
[declaration probes](../../investigations/compiler-assignments.md#parameter-writes-and-local-annotations).

Locals retain stored state; repeated expression reads produce distinct
single-use values. The [IR storage rules](../ir/README.md#local-operations) own
initialization/availability checking, and [frame
lowering](../lowering/README.md) assigns slots independently of semantic IDs.
Unused initializers still execute.
[Controlled SDK evidence](../../investigations/compiler-locals.md) verifies ordinary
local bodies; the frame limit is structural, not an established device maximum.

### Terminal conditions

Runtime comparisons follow the [Number-comparison
evidence](../../investigations/compiler-number-comparisons.md).

A function may end with `if (<condition>) { return <expression>; } else { return
<expression>; }`. Braces are required; `else` is optional. An omitted false arm
continues to the next statement, which must satisfy the function return
contract. Arms may also contain local declarations, print statements, and nested
conditions, as described below. Loop syntax is specified separately below.

Comparisons follow the [operator](#expressions-and-operators) and
[numeric](#numbers-and-numeric-aliases) contracts, producing Boolean results.
Equality/inequality also accept Boolean pairs and the String identity profile;
`!=` negates the existing equality contract, not String content equality.
Dictionary/null checks follow the
[nullable contracts](#nullable-results).
Arithmetic has higher precedence, so `value + 1 > 2` compares the sum. At most
one comparison is accepted per expression level; parentheses retain their
grouping behavior. The condition must produce a Boolean: a literal, typed
parameter or local read, supported call, or comparison. Boolean locals,
assignments, and print arguments are accepted. Annotated Boolean
parameters/results and user-call arguments use these same rules. Boolean
arithmetic, integer truthiness, mixed Boolean/Number equality and Boolean/String
ordering are unsupported. Short-circuit operators follow the [short-circuit
rules](#short-circuit-expressions).

[VM lowering](../lowering/README.md#control-flow).

### Statement arms and joins

Declarations, assignments, updates, calls, and the ending of `do/while` accept
one or more semicolons, including comments/whitespace between them. Standalone
scoped blocks may optionally have the same trailing terminators. Additional
terminators add no IR operation, scope, or effect. This is not a general empty
statement: a leading semicolon or a semicolon after an `if`/`while` body is
rejected. Return/break/continue still require exactly one semicolon, as do `for`
header separators. See the connected [statement grammar](grammar.md#statements).

Conditions may appear between statements; combined conditional/loop/switch/block
nesting is limited to 64 levels, counting each `else if` link as a nested
conditional. Empty arms are allowed. `else if` chains evaluate conditions in
order until one is true, then execute only that arm; a final `else` is optional.
Each selected body still requires braces. Each arm has its own local scope.
Every value-returning path must satisfy the declared result contract; Void
fallthrough inserts a return.
Statements after a terminating construct are rejected as unreachable, including
a scoped block whose body terminates, a conditional whose arms both terminate,
or a loop with no reachable exit. A value-returning path with no return is
rejected at the function's closing brace.

Continuing arms jump to a shared block after both arms. Returning or
loop-control arms do not contribute an edge to that join; when both arms
terminate, no join is generated. Only enclosing locals are visible after the
condition. Even if both arms declare the same name, those distinct declarations
do not define a result at the join. Enclosing locals can carry results using
assignment; no phi values or shared stack values are introduced.

The semantic 1024-operation limit includes generated block markers and
terminators. Parsing also bounds source nodes; standalone blocks count even
when empty and generate no operations. The 255 parameter/local slots count all
declarations across the function, including mutually exclusive arms; slots are
not reused. Local initializers, prints, comparison operands, and module call
arguments retain
source evaluation order within the selected path. Common initializers run once
before the branch. These forms use the control-flow IR directives.

See
[nested-conditions.mc](../../../examples/compiler-probe/nested-conditions.mc)
and [nested/continuing-arm simulator
evidence](../../investigations/compiler-nested-conditions.md). Broader language
scoping and shadowing rules remain future work.

### Local reassignment

`<identifier> = <expression>;` assigns an earlier declared, in-scope local.
The local target must be a bare identifier. Unknown/forward names and arm locals
used outside their scope are rejected. Parameters follow the
[parameter storage contract](#parameters-and-positional-arguments). Assignment is a statement,
not an expression. Local compound and increment/decrement statements are
specified below. Class methods additionally support [explicit self field
assignment](#instance-fields).

Assignments may change a local's inferred type, including its source-class
identity or nullability. Parameters, fields, and container elements retain
their declared contracts; changing a local binding does not widen the contents
of an existing mutable container. RHS evaluation finishes before the write, so
`n = n + 1` reads the old value. Later reads observe the new content on that
path. Assigning an enclosing local inside an arm is allowed; assignment does not
create a declaration.

A write consumes one semantic value and defines no result. It reuses the local's
existing slot and does not contribute to the 255-local count. The operation
count still includes each write and its RHS definitions. Both arms may update a
common local, or one arm may leave it unchanged; it remains available at the
join because its initializer executes before the condition.

Source, typed IR, and text IR share type, availability, and effect-order checks.
Assignment-containing IR uses the conservative schedule specified in
[IR text](../ir/README.md#local-assignment). See
[assignments.mc](../../../examples/compiler-probe/assignments.mc) and
[integer/string simulator evidence](../../investigations/compiler-assignments.md).

### Increment and decrement

`target++;` and `++target;` add one to a numeric scalar or Char, preserving its
type. `target--;` and `--target;` subtract one from a numeric scalar; Char
decrement is rejected. The two operator characters must be contiguous. Accepted
targets are bare numeric/Char locals or parameters, explicit `self` numeric/Char
fields, and supported numeric/Char indexed elements. Storage write-back must
preserve the field/element contract. Char requires API 1.3.0, as for other Char
expressions. Grouped local targets, other value/element types, and
unknown or out-of-scope names are rejected. A nullable operand needs an
established non-null proof at the read. Existing assignment contracts still
govern its write.

Both prefix and postfix forms are statements with identical supported behavior;
they cannot be arguments, conditions, initializers, return values, or parts of
larger expressions. Source lowering retains a checked step and separate write,
without assigning expression-result semantics to either spelling.

For indexed targets, evaluate/capture array then index once, read the selected
element, add/subtract one, and write back to that captured location. Bounds and
allocation failures remain runtime obligations. Captures share the ordinary
frame budget; local updates reuse their existing slot. The tested Number
boundaries wrap from maximum to minimum on increment and minimum to maximum on
decrement. See [SDK/runtime
evidence](../../investigations/compiler-increments.md).

### Local and parameter compound assignment

A bare in-scope initialized local or parameter supports `+=`, `-=`, `*=`, `/=`, `%=`, `&=`,
`|=`, `^=`, `<<=`, and `>>=` as statements. Operator characters must be
contiguous. Grouped local targets and unknown or out-of-scope names
are rejected. Field/indexed compound targets are specified below. Assignment
expressions, logical compounds, and unsigned-right-shift compounds remain
unsupported.

`n op= rhs;` reads the existing storage once, evaluates `rhs` once, applies the
corresponding supported binary operator, then writes the result to the existing
slot. The whole RHS is parsed before applying the implicit operator: `n *= 1 +
2` means `n = n * (1 + 2)`. Existing binary contracts apply, with parameters
retaining their signature contract: all ten operators
accept non-null Number pairs; String `+=` uses
supported scalar concatenation, and Char `+=` accepts Number/Long. Number/Long
`+= Char` produces Char; Char `+= Char` produces String. Inferred locals may
change type accordingly, while fixed parameter/field/element contracts still
govern their write-back. No
new coercion, folding, or runtime guard is introduced.

The read/binary/write sequence uses ordinary IR operations and source spans. A
RHS needing control-flow staging snapshots the old local before RHS effects;
hidden locals share the existing frame budget. Plain compounds need no hidden
slots. A nullable local requires an existing non-null proof for its old read,
and the write invalidates that proof as ordinary reassignment does. A later
non-null read needs a fresh check. See [compound
evidence](../../investigations/compiler-compound-locals.md).

### While loops

`while (<condition>) { <statements> }` evaluates its condition before each
iteration. The condition must produce a Boolean, with the same Boolean/type
rules as `if`. Braces are required; the body may be empty or contain locals,
assignments, prints, nested conditions/loops, and early returns.

Body locals are visible within that body and nested scopes, but not after the
loop or in its condition. Their existing slots are initialized anew on each
iteration; new slots are not allocated per iteration. Enclosing local writes
persist into later tests and after exit. Reads account for all incoming writes,
including loop backedges, rather than assuming the first iteration's type.

The body may execute zero times. Its declarations cannot establish a local
needed at the test or exit. A function ending with a loop still needs a return
for its false-condition path, even if its body always returns. The compiler does
not prove termination or condition outcomes and does not insert an iteration
limit. "Bounded subset" refers to supported syntax and compilation limits.

[VM lowering](../lowering/README.md#while-loops).

See [loops.mc](../../../examples/compiler-probe/loops.mc) and
[zero/multiple-iteration simulator evidence](../../investigations/compiler-loops.md).

### For and post-test loops

`for (initializer; condition; update) { body }` executes its initializer once,
then tests before each iteration and executes its update after a continuing
body. Each header clause is optional; an absent condition means unconditional
iteration. The initializer accepts a comma-separated declaration list sharing
one `var`, or a comma-separated assignment/update list. The update accepts only
assignment/update lists. Each item completes before the next begins; a later
initializer can read an earlier declaration, and a later update observes earlier
writes. Calls inside expressions retain ordinary argument/effect order.
Standalone calls, repeated `var` keywords, mixed declaration/effect lists, and
trailing commas are rejected, matching the SDK header grammar. See the connected
[statement grammar](grammar.md#statements).

All initializer locals are visible to the condition, body, and update, but not
after the loop. Body locals are not visible in either header
clause. Existing inference and no-shadowing rules apply; a name can be reused
after its loop scope ends.

`do { body } while (condition);` executes its body before testing. The condition
must be Boolean and cannot refer to body-local declarations. Braces and at least
one trailing semicolon are required. `for` conditions, when present, also require
Boolean. No loop form proves constant condition outcomes or imposes an iteration
limit.

All header clauses are name- and type-checked, including an update skipped by
an unconditional break/return or a post-test skipped by a terminating body.
Their [disconnected IR blocks](../ir/README.md#control-flow) preserve those
checks without contributing proofs to executable paths. A function with no
reachable return retains its [declared result](#types-and-nullability). A
post-test body that always returns may satisfy a value result without a
reachable condition/exit.

Disconnected headers currently start without enclosing branch refinements.
For example, a skipped update using a nullable Number needs its own null guard
or explicit cast even inside an enclosing `if (n != null)`. Executable headers
retain the usual incoming proofs; preserving lexical proofs for skipped headers
is a remaining frontend checking requirement.

The SDK also accepts unassigned updates in provably unexecuted headers, such as
`var x; for(;;x++){return;}`. This frontend still requires a selectable operand
type there; no runtime default is inferred from that SDK acceptance.

See the grouped `CoreLoops` specimen in
[source-types.mc](../../../examples/compiler-object-tests/source-types.mc) and
[SDK/generated evidence](../../investigations/compiler-loops.md#for-and-post-test-loops).

### Loop control

`break;` exits the innermost enclosing loop or switch. `continue;` skips switches
and jumps to the enclosing loop's test for
`while`/`do`, or to the whole update list and then test for `for`. Both require
semicolons and an appropriate enclosing construct, including when nested within
conditional arms. They terminate
their current path and carry no expression values across the edge. A nested
loop's controls never target an outer loop. Labeled controls are unsupported.

A pre-test loop with a condition may run zero times, so its body cannot satisfy
the function's fallthrough return requirement. Mixed
returning/breaking/continuing arms omit a join when all paths terminate. A
false-condition exit remains unless the condition is absent or unreachable.
Skipped declarations stay scoped to their body/arm; initialized enclosing locals
retain their slots and flow-sensitive types. These statements emit existing `.jump`
operations.

See [loop-control.mc](../../../examples/compiler-probe/loop-control.mc) and
[simulator evidence](../../investigations/compiler-loop-control.md).

### Switch statements

`switch (selector) { case value: statements ... default: statements }` selects
an ordered case group and then executes bodies in textual order until a break,
return, continue, or the end. The selector's value is fixed before case tests;
case expressions execute in order only until the first match. At most one
`default` may appear anywhere; it is selected only when all tests fail. Falling
through a default group still executes its body. Consecutive labels share a
fall-through destination, and duplicate value cases are accepted. Empty switches
and statements before the first label are rejected. See the connected
[switch grammar](grammar.md#statements).

A value label tests `selector.equals(caseValue)`, using normal native or source
method dispatch. This includes String content equality and user overrides;
`==` is not substituted. Existing `equals` receiver/argument/result and API
contracts apply. In particular, an unproved nullable receiver is rejected for
value cases. A Null argument to `equals` is not a null-check proof. Type-only
switches can instead use the supported `case instanceof ClassName` targets,
with the existing runtime class and nullable-operand contracts.

Case groups share one lexical switch scope. A declaration becomes visible to
later textual groups but must be initialized on every path to a read, including
direct case entry; fall-through alone cannot establish availability there.
Duplicate declarations across groups are rejected. Explicit braced blocks
isolate their locals and allow sibling name reuse. Switch locals do not escape
the switch. Enclosing assignments persist after exit.

Type cases use ordinary class-test proofs for a local/parameter selector.
Fall-through joins retain only shared facts, and writes invalidate local proofs.
A computed selector is saved privately; its proof does not narrow a separate
source expression. The SDK specimen uses explicit casts because SDK 9.2.0 does
not narrow its original Object parameter inside type-case bodies.

`break` targets the nearest switch or loop; `continue` targets the nearest loop,
including its update list for `for`. A switch without default has a possible
unmatched exit. With a default and every path returning, no additional source
return is required; exhaustive class coverage without default is not inferred.
Case bodies and labels count toward the existing nesting/node/operation limits.
Source, saved IR, and unit checking share initialization, signature, and effect
checks. See [lowering](../lowering/README.md#switch-dispatch) and
[SDK evidence](../../investigations/compiler-nested-conditions.md#switch-selection-and-dispatch).

### Void functions and statement calls

A value-returning function must return its declared type on every path. A Void
function's `return;` terminates immediately; reaching the final closing brace
inserts a void return. Value and void return forms cannot be mixed, including
across branches. Unreachable statements after either form are rejected. Loops
and scoped locals retain their existing rules. See [function type
contracts](#types-and-nullability) for annotations and defaults.

```monkeyc
function main() {
    Startup.greet(41);
}
module Startup {
    function greet(number) {
        System.println(number);
        return;
    }
}
```

Qualified user calls may be expression statements. They take typed positional
arguments and discard any supported result, including Void. Void calls cannot
initialize locals, provide arguments, or appear in any other expression. Unit
mode checks the callee's result type and arity after collecting all signatures.
Single-function mode still leaves external callee contracts to the caller;
expression calls there assume integers. No void expression value is created.
Statement calls preserve argument evaluation and statement order.

Typed IR adds `ReturnVoid` and `ModuleCallStatement`; `Function::return_type()`
reports the validated scalar, nullable, or Void contract. Saved text uses
[IR](../ir/README.md#void-returns-and-statement-calls).
Both forms share type, control-flow, use, and effect-schedule validation. A void
function may contain only its return, so the operation bound is now 1..=1024.
The backend emits bare `return`; statement calls use the existing call sequence
and discard the VM result with `popv`.

The [console application
profile](../build/README.md#complete-application-generation) prints a value
entry's Number, Long, Float, Double, String, Boolean, or Char result and
discards a Void entry's result. The
[example](../../../examples/compiler-void/README.md) and
[simulator evidence](../../investigations/compiler-void.md) verify both void return forms and caller
continuation after discarded calls. Source class/receiver syntax follows the
[restricted class profile](#source-classes).

### Array element writes

`array[index] = value;` replaces an element of a non-null supported array. The
index must be non-null Number; the value must match the invariant Number, Char,
String, or native View element contract. Guarded fresh nullable reads are
accepted for Number indices and Number/String values. View values may be native
or source View instances; storing/reading through a View array erases exact
source identity. Null values, delegates, and drawing contexts cannot fill View
slots. Receiver, index, and value evaluate once, in that order. The assignment
produces no value and does not change the array's length. Aliases observe the
changed element. Calls in the value expression may mutate array contents or
length before the write checks its index.

[VM lowering](../lowering/README.md#array-element-writes).

Nested arrays and assignments used as expressions remain unsupported. Indexed
compound targets follow the contract below. See [Number write
evidence](../../investigations/compiler-array-writes.md) and
[Char/View write evidence](../../investigations/compiler-typed-array-writes.md).

### Indexed compound assignment

`array[index] op= rhs;` supports Number/Long elements with all ten compound
operators, Float/Double elements with the four arithmetic operators, and Char
elements with `+= Number/Long`, and String elements with `+=` using the shared
String concatenation operands. Existing binary contracts and invariant array
element types apply; View elements have no supported compound operator. The
index must be non-null Number, including a fresh guarded nullable read.
String/dictionary receivers, nested arrays, and assignment expressions remain
unsupported.

Evaluate array and index once, in that order, then read the old element before
RHS effects. Evaluate RHS once, apply the binary operation, and write through
the captured array/index. Changes to contents or length during RHS evaluation do
not change the old value used by the operation. An invalid initial index fails
at the read before RHS effects; a RHS that shrinks the array can make the final
write fail after those effects. Both accesses retain runtime bounds checks,
without clamping or static range rejection.

Array and index are captured in two typed hidden locals. Plain RHS expressions
need no additional old-value/result slots. A branching RHS stages the old read
and result before defining fresh final write operands, keeping expression IDs
block-local. These temporaries share the ordinary frame budget; loop iterations
reuse their static slots. Saved IR uses existing locals, `.array-read`, binary
operations, and `.array-write`, with no compound directive. See [target
evidence](../../investigations/compiler-compound-targets.md).

## Core Lang values and methods

### Strings and printing

Double-quoted literals produce UTF-8 strings under the
[literal grammar and decoding rules](grammar.md#lexical-rules-and-literals). Source decoding
is independent of IR/assembly quoting: those writers serialize the decoded
value using their own escapes. Empty strings and embedded NUL are preserved;
values longer than 65535 decoded UTF-8 bytes are rejected.

String literals, native conversion results, and string-valued local reads can
initialize locals or supply `System.println(expression);`. Print statements may
occur between local initializations and accept one Number, Long, Float, Double,
String, Boolean, or Char value. String types propagate through locals and
annotated function parameters/results; incompatible uses are rejected. Addition
follows the [operator contracts](#operator-contracts), including Char
concatenation and supported scalar conversions. Arbitrary SDK calls, and using
`System.println` as an expression remain unsupported. A local/parameter named
`System` cannot be used as the print receiver. The function's scaffold must
provide the SDK module. The [native name profile](#native-name-declarations)
resolves its supported spellings; Jungle resolution remains unimplemented.

### Object equality and container observers

Supported numeric, String, Boolean, Char, Symbol, Array, Dictionary, ByteArray,
opaque Object, Method, and WeakReference receivers accept `equals(other)` →
Boolean. Arguments follow the catalogue's Object-or-Null contract, including
supported source/native objects, comparison interfaces, containers, numeric
aliases, and nullable forms. Nullable arguments require no proof; nullable
receivers still require a non-null proof. Container comparison neither requires
matching element/key/value contracts nor converts their types. General Object
storage and recovery retain their existing restrictions.

All supported Arrays and Dictionaries inherit `equals` and `hashCode()` → Number
from Object, since API 1.0.0. Element/key/value availability is checked
separately. Container equality tests reference identity: aliases compare equal,
while separately allocated containers with equal contents compare unequal.
Hashes remain stable across mutations and agree for aliases. The runtime
supplies their values; distinct containers need not have different hashes.

Both calls preserve receiver-before-argument evaluation and support discarded
results. Native dispatch retains runtime semantics for scalar equality,
including String content comparison and numeric class distinctions; no result or
hash is folded. Scalar override/API requirements remain in their respective
sections. This profile does not enable Array/Dictionary equality operators, or
general callback consumers. See the
[observer evidence](../../investigations/compiler-object-observers.md).

### Comparison Object arguments

Number, Long, Float, Double, Boolean, Char, and String `compareTo(other)` take
one non-null Object and return Number, since API 5.0.0. Supported Object
arguments include scalar classes, numeric aliases, containers, source/native
objects, and comparison interfaces. A nullable argument requires a current
non-null proof. Source and saved IR check the declared API signature without
narrowing it to the receiver's runtime comparison domain.

The runtime still checks that domain: numeric/Boolean/Char receivers compare
numeric, Boolean, or Char values; String compares Strings. An incompatible
Object argument is valid at the static call boundary but can throw
`UnexpectedTypeException`. The compiler inserts no conversion, dynamic class
guard, or fallback comparison. Null is rejected statically by the non-null
parameter contract. Comparison signs are meaningful; particular magnitudes and
exception recovery are not established by this profile. See the
[Object argument example](../../../examples/compiler-comparison-objects/README.md)
and [runtime evidence](../../investigations/compiler-comparison-objects.md).

### Native value calls

A Number-valued expression may receive these catalogue-resolved methods:

| Method | Declaring class | Result |
|---|---|---|
| `toString()` | `Lang.Object` | String |
| `format(String)` | `Lang.Number` | String |
| `abs()` | `Lang.Number` | Number |
| `toNumber()` | `Lang.Number` | Number |
| `toLong()` | `Lang.Number` | Long |
| `compareTo(Object)` | `Lang.Number` | Number |
| `equals(other)` | `Lang.Object` | Boolean |
| `hashCode()` | `Lang.Object` | Number |

Number `compareTo` requires API 5.0.0 and follows the
[shared Object argument contract](#comparison-object-arguments). The other
listed Number methods require 1.0.0. `equals` uses the
[shared equality contract](#object-equality-and-container-observers).
Object arguments use catalogue-declared inheritance, without implicit
conversions or acceptance of unknown classes. Scalar user-function contracts
remain exact. Comparison/equality/hash behavior is supplied by the runtime;
comparison signs and hash consistency are the contracts, not particular
magnitudes or hashes. See the [Number group
evidence](../../investigations/compiler-number-methods.md).

String-valued expressions support these zero-argument methods, declared by
`Lang.String`: `length()` and `hashCode()` return Number; `toLower()`,
`toUpper()`, and `toString()` return String. These methods are available since
API 1.0.0. Runtime implementations determine character counts and case mapping;
the compiler does not evaluate these calls at compile time.

String `compareTo(Object)` follows the
[shared comparison contract](#comparison-object-arguments) and requires API 5.0.0. String `equals` returns Boolean and accepts
the shared Object equality arguments, since API 1.0.0.
`toNumberWithBase(Number)` returns Number or Null, since API 1.4.1, and follows
the existing null-check rules. The base is 0 for prefix detection or 2..=36 for
an explicit radix. Numeric ranges and conversion content are passed to the
runtime without compiler validation. See the
[String group evidence](../../investigations/compiler-string-methods.md).

Boolean receivers support `toString()` returning String, `hashCode()` returning
Number, and `equals` returning Boolean with the shared Object argument profile
as Number/String equality. These inherited methods date from API 1.0.0. Boolean
`compareTo` follows the
[shared Object argument contract](#comparison-object-arguments) since API 5.0.0.
This native-call support does not enable Boolean arithmetic or ordering
operators. See the
[Boolean group evidence](../../investigations/compiler-boolean-methods.md).

Char values originate from literals or Number's zero-argument `toChar()` and are
distinct from Number. The [literal rules](grammar.md#lexical-rules-and-literals) distinguish
raw BMP scalars from escaped UTF-16 code units, including surrogate values.
They support `toLower()`/`toUpper()` returning Char,
`toNumber()` returning Number, and `toString()` returning String. Char's direct
`compareTo` follows the [shared Object argument
contract](#comparison-object-arguments) since API 5.0.0. Inherited `equals`
accepts the shared Object argument profile and returns Boolean; `hashCode()`
returns Number. Char itself and its conversions require API 1.3.0, including
when it appears only in a function signature. Typed arguments, returns, local
initialization/assignment, printing, and chained calls retain the exact Char
type. Char operators follow the [operator contracts](#operator-contracts);
nullable Char requires a non-null proof for scalar operators and method
receivers. Conversion ranges and case/String behavior are runtime obligations;
the compiler adds no Unicode scalar-range validation. See the
[Char group evidence](../../investigations/compiler-char-methods.md).

Numeric, String, Boolean, and Char receivers also support zero-argument `weak()`
since API 1.2.0, with Char receivers additionally requiring 1.3.0. Its effective
result is the exact receiver type, not WeakReference. It remains an effectful
native call; the compiler does not replace it with its receiver. Nullable scalar
receivers require a direct non-null proof before the call. Ordinary references
follow the separate profile below. See the [weak-result
evidence](../../investigations/compiler-immutable-weak.md).

String results may initialize/replace a String local or be printed. Number
results support integer locals, arithmetic, comparisons, user-call arguments,
integer returns, and further Number calls. For example, `(0 -
41).abs().format("%03d")` produces `"041"` on the validated profile. Existing
user-call arguments and return restrictions remain unchanged.

Postfix calls bind more tightly than addition/subtraction; parenthesize a
computed arithmetic receiver, for example `(n + 1).format("%03d")`. Literal,
local, numeric/String/Boolean/Char parameter, and supported call-result
receivers are accepted. Evaluate the receiver first, then arguments left to
right, then invoke the member.

The frontend distinguishes visible local/parameter receivers from module calls.
Supported native value calls may also be statements: `n.abs();`,
`chars.remove('a');`, and `factory.make().add('x').size();` evaluate the call
and discard its final result. Receiver/argument order, API checks, and nullable
receiver proofs are the same as in value contexts. A discarded result does not
contribute to function return inference. Nullable results need no guard when
discarded. Bare arithmetic, literals, and element reads are not source
expression statements. Native Void methods still require their own supported
statement contract; discarding does not enable unsupported members or receiver
profiles. See the
[statement evidence](../../investigations/compiler-native-statements.md).

Null receivers remain unsupported; Dictionary receivers follow their bounded
[profile](#dictionaries). Chained calls
must resolve each member against the preceding result's type, for example
`41.toString().toUpper().length().format("%03d")`. String and Boolean user
parameters/results require the [function annotations](#types-and-nullability).
String `find(String)`, `toNumber()`, and `toNumberWithBase(Number)` return
Number or Null; `substring(Number or Null, Number or Null)` returns String or
Null. These follow the
[null-check rules](#nullable-results). Unknown catalogue
members, wrong arity/types, and known members lacking lowering are distinct
errors. Explicit [API context inputs](../sdk/README.md#api-context-inputs) check
introduction levels; general native dispatch remains unsupported. Format content
is passed to the runtime; type checking does not establish that a String
contains a valid format pattern.

[Native call IR](../ir/README.md#native-value-calls) preserves receiver/member
identity separately from executable IDs. Runtime evidence and reproduction are
in the [native call experiment](../../investigations/compiler-native-calls.md).

### Symbol values

`:name` produces a non-null `Toybox.Lang.Symbol` since API 1.0.0. Names use the
ASCII identifier grammar. A Symbol is distinct from Number and String; matching
names denote the same identity within a build. Numeric IDs are assigned during
application/assembly binding and need not agree between builds or toolchains. Do
not treat them as persistent identifiers.

Symbols support annotated parameters/results, exact local assignment, matching
conditional arms, `==`/`!=` between Symbols, and the shared native `equals`
argument profile. They support `toString()` → String, inherited `hashCode()` →
Number and `equals` → Boolean since 1.0.0, immutable `weak()` → Symbol since
1.2.0, and `toNumber()` → Number since 2.3.0. Equality and conversions remain
native operations; no numeric ID or hash is folded into semantic IR.

Complete applications retain names for literal Symbols in a SYMBOLS section, so
their development `toString()` results use those names. This does not promise
names for arbitrary runtime-created values or release builds without reflection
metadata. Raw CODE output needs an explicit symbol source; it cannot allocate an
independent identity space.

`Symbol or Null` supports annotated parameters/results, local storage, and
Symbol/Null conditional joins from API 1.0.0. Forwarding and returns preserve
the declared union even when the current value is a Symbol. Local writes may
replace it with Symbol, Null, or the same union. `==`/`!=` with literal Null
support fresh local/parameter proofs in either operand order. Guarded reads may
compare with other Symbols, invoke methods, or act as `has`/`method` selectors.
Existing
[null-proof rules](#nullable-results) apply across branches, short circuit
expressions, and loops; writes invalidate the local's proof. Copies and stored
Boolean guards do not transfer it. Nullable Symbols cannot satisfy a non-null
Symbol consumer without proof. Native `equals` arguments explicitly permitting
Null retain their shared argument profile. Method API requirements still apply
after a guard, including 2.3.0 for `toNumber`.

Symbol arithmetic, ordering, truthiness, and direct printing remain unsupported.
Classes follow the [Symbol field contract](#symbol-fields). Method lookup
follows its separate contract below. See the [specimen and binding
evidence](../../investigations/compiler-symbols.md).

#### Symbol containers

Arrays support invariant Symbol and Symbol-or-Null elements from API 1.0.0.
Symbol Dictionary keys are non-null; values may be Symbol or Symbol-or-Null,
combined with other supported key/value contracts. An expected contract types
empty and nullable literals. Non-null Symbol literals can infer homogeneous
Array/Dictionary contracts; arbitrary scalar/Symbol mixtures do not infer a
broader union. Existing mutation, collection, Object observer, and enumeration
profiles apply. `reverse` and `slice` return independent Arrays. Symbol is not a
Comparable witness; non-null elements require a non-null checked
[Comparator](#source-comparators) for sorting.

Indexed reads, `get`, `keys`, and `values` preserve their declared types.
Dictionary lookup always adds missing-key Null; `values()` does not add it.
Snapshot a nullable element and guard the local before Symbol consumers.
Repeated indexed reads and `hasKey` do not establish a proof. Nullable key
arguments need a fresh non-null proof; nullable key contracts and nullable
container references remain unsupported.

Symbol-bearing literals use ordered private construction with symbolic `spush`
operands. Raw CODE resolves names through the supplied symbol source; complete
applications share the normal name/ID map, including pinned native member IDs.
No numeric Symbol ID is placed in semantic IR or treated as Number. Flat DATA
constants do not accept Symbol tokens. A Null-only payload under a nullable
Symbol element/value contract can still use the existing Null constant encoding.
Pooled Symbol encodings require separate evidence/contracts.

### Method values and invocation

Supported scalar, Array, Dictionary, opaque Object, WeakReference, and Method
values and supported native/source-class receivers support `method(Symbol)` →
non-null `Toybox.Lang.Method`, since API 1.0.0. Receiver class availability and
construction retain their own API requirements. Method lookup and missing-member
errors remain runtime operations. The Method retains its bound receiver;
replacing the original variable does not release that receiver while a Method
still owns it.

Method supports annotated parameters/results, local storage/assignment, matching
conditional arms, and inherited `toString`/`equals` plus its own `hashCode`,
since 1.0.0. `weak()` returns WeakReference from 1.2.0. Native observers retain
runtime identity/hash behavior: repeated bindings need not compare equal even
for the same target/member.

`Method or Null` supports explicit parameter/result contracts, locals,
assignment, and conditional joins with Method or Null alternatives. Nullable
locals retain their declared union when assigned a Method or Null. Equality and
inequality with literal Null establish proofs for fresh reads of that storage;
invocation and every Method member require a non-null proof. Writes invalidate
local proofs; aliases and stored Boolean tests do not transfer them. Releasing
the last Method alias also releases its bound receiver. Lexical `as Method or
Null` is an unchecked type assertion, with no runtime conversion or signature
recovery. [Method fields](#method-fields) support required/nullable storage.
Arrays and Dictionaries follow the
[Array](#method-arrays) and [Dictionary](#method-dictionaries) profiles below.

`invoke(arguments...)` accepts up to 254 supported Object/Null values and
returns opaque `Object or Null`. This erased result covers scalar/container
values, nullable returns, and Void callbacks. Save and guard it before Object
member calls; it cannot become a Number, typed container, or Method without a
separate refinement contract. Statement invocation discards the result. Receiver
evaluation precedes arguments, left to right, and each runs once.

The compiler checks the Method API and VM argument-count limit. The target's
existence, arity, accepted types, introduction level, and result behavior remain
runtime/caller obligations; dynamic lookup does not recover its signature.
Source instances support `instance.method(selector)` and instance bodies support
`self.method(selector)` or bare `method(selector)`. Bare calls resolve to the
current instance and use the same semantic operations as explicit self calls. A
declared source member named `method` keeps ordinary source-call priority. Bare
`self` produces opaque Object and requires an instance body; it does not recover
a source-class type or concrete callback signature. Existing `self.member(...)`
source calls retain checked signatures. Bare `method(...)` requires an instance
body unless it names a declared module function. For inherited native lookup,
a local, parameter, or field named `method` prevents implicit lookup.
Calling such lexical values remains unsupported; use explicit `self.method(...)`
for the instance member. Declared source members follow the ordinary
[owner lookup rules](#user-calls-and-owner-lookup). Receiver capture still
requires all required instance fields to be initialized.

Unlike the SDK checker, this erased lookup profile does not statically resolve a
literal selector to a source target. A missing target or incompatible invocation
remains a runtime/caller obligation even for literal selectors. Constructors and
typed signatures remain deferred. Array sorting uses a [separate Comparator
contract](#source-comparators). See the
[binding/lifetime evidence](../../investigations/compiler-methods.md).

#### Method Arrays

`Array<Method>` and `Array<Method or Null>` preserve invariant element contracts
through parameters, results, locals, indexed reads/writes, and matching Array
conditional arms. The value types require API 1.0.0; native Array methods retain
their existing API requirements. A non-null-element Array cannot widen to a
nullable-element Array. Empty/nullable literals need a concrete expected
contract; otherwise a first non-null Method can establish the literal element
type.

Method-bearing literals allocate privately and fill in source order at runtime.
They do not embed Method objects or addresses in DATA constants. Null-only
nullable literals can use the existing Null constant encoding. Each construction
produces a fresh mutable Array.

The supported `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`,
`removeAll`, `reverse`, and `slice` profiles apply. Element arguments match the
declared nullability; `addAll` requires the same invariant Array contract.
Reverse/slice produce independent Arrays while retaining the same Method
references. Search and removal use runtime Method equality; separately created
bindings need not compare equal. Non-null Method elements support sorting with a
non-null checked
[Comparator](#source-comparators), preserving their bound references.

Nullable indexed reads need guarded local snapshots before invocation or member
use; a test of one indexed read does not refine another. Replacing/removing an
element preserves saved aliases. Releasing the last Method reference releases
its bound receiver. Array storage does not recover a callback signature or a
concrete invocation result. See the
[specimen](../../../examples/compiler-methods/README.md) and [ownership
evidence](../../investigations/compiler-methods.md#nullable-ownership).

#### Method Dictionaries

Typed Dictionaries accept non-null Method keys, Method values, and nullable
Method values from API 1.0.0, alongside their other supported key/value domains.
Both mutable parameters remain invariant through signatures, storage, and
writes. Nullable keys require a non-null proof before lookup or mutation; Null
keys remain unsupported.

Method-bearing literals allocate privately and evaluate each key before its
value, in entry order. Homogeneous Method candidates can establish a literal's
contract; Null-only values need an expected nullable contract. Existing scalar
keys with Null-only nullable Method values may use DATA constants. Method keys
or values are never encoded as pooled scalar payloads or numeric addresses.

`get` and indexed lookup return Method or Null even for non-null Method values,
because the key may be missing. Guard a local snapshot before invocation;
`hasKey` and a test of another mutable lookup do not refine that read.
`put`/indexed writes preserve declared nullability. Existing `size`, `hasKey`,
`remove`, and observer profiles apply. Runtime Method equality/hash determines
key identity; repeated aliases replace an entry, while independently created
bindings need not compare equal. No hash algorithm is specified.

`keys()` returns `Array<Method>` for Method keys. `values()` returns an Array
with the declared Method/nullability contract, without adding the missing-key
Null alternative. These Arrays and lookup snapshots retain the same Method
objects independently of later Dictionary mutation. Keys and values both own
their stored Methods; the bound receiver expires only after all aliases release
them. Erased binding/invocation still establishes no concrete callback
signature. See the [specimen](../../../examples/compiler-methods/README.md) and
[ownership evidence](../../investigations/compiler-methods.md#nullable-ownership).

### Ordinary weak references

Supported Arrays, opaque/typed Dictionaries, Method, and WeakReference receivers
support `weak()` → `Toybox.Lang.WeakReference`, since API 1.2.0. WeakReference
is a non-null value with `get()` → `Toybox.Lang.Object or Null` and
`stillAlive()` → Boolean, both since 1.2.0. It also inherits `toString`,
`equals`, and `hashCode`; calls retain runtime behavior rather than synthesizing
wrapper identity. There is no `clear()` method. Release a strong reference
through assignment or frame exit; the runtime determines when the target ceases
to exist.

Source signatures accept `Toybox.Lang.WeakReference`, `WeakReference or Null`,
`Toybox.Lang.Object`, and `Object or Null`. They survive locals, function
calls/results, saved IR, and ordinary branch/loop null proofs. A retrieved
Object is a strong value; saving it can keep the target alive. Save and guard
that value before calling its supported Object methods (`toString`, `equals`,
`hashCode`). Neither `stillAlive()` nor testing a separate `get()` call proves a
later result non-null.

WeakReference/Null alternatives join as nullable WeakReference, since API 1.2.0.
A sequential write may narrow an inferred local. A fresh non-null proof
permits handle members; it proves neither target liveness nor a non-null `get()`
result. Writes invalidate local proofs; aliases and stored Boolean tests do not
transfer them. Lexical `as WeakReference or Null` asserts a type without runtime
checking, conversion, or ownership changes. Required/nullable
[WeakReference fields](#weakreference-fields) preserve these
contracts. Null storage and an expired non-null WeakReference are distinct
states.

Object contracts erase concrete type information without changing runtime
values. Compatible values may pass to Object parameters/results and replace
opaque Object locals; a nullable Object local also accepts Null. Erasure cannot
be reversed into a typed Array/Dictionary or a scalar. Object joins accept
matching opaque contracts and Object/Null alternatives; matching WeakReference
arms also join. Other native alternatives follow the shared union rules;
reference joins do not widen mutable container contents.

The declared Object type does not establish a WeakReference result from
`Object.weak()`: the runtime value might be an immutable scalar. That receiver
path, other native/source weak targets, Dictionary weak keys, and safe concrete
recovery remain outside this profile. The new types do not acquire arithmetic,
truthiness, direct printing, or general equality operators; nullable Object/Null
tests use the existing proof rules.

Hidden reference temporaries transfer ownership on their final read using IR
`.take`; private container builders release their frame reference on finish.
Source locals retain their assignment/frame lifetimes. Backend staging must not
silently extend a target's strong lifetime. See the
[lifetime specimen and evidence](../../investigations/compiler-weak-references.md).

#### WeakReference construction

`new Toybox.Lang.WeakReference()` constructs a fresh native class instance;
canonical, module-alias, and imported bare names are accepted. Qualifier
shadowing and constructor arguments are rejected. This is distinct from
`target.weak()` and does not accept a target object.

The bounded profile follows SDK 9.2.0/`fr265` simulator evidence: a fresh
instance reports alive and its `get()` result compares equal to itself. Saved
retrievals retain the instance normally. The compiler still types `get()` as
Object or Null; it does not infer self identity or non-null retrieval from
construction.

Construction is effectful and uses ordinary native allocation/initialization.
The class's API 1.2.0 introduction is the minimum availability guard, including
after IR reload. Garmin's published WeakReference page does not declare a
constructor, so this guard is not a published constructor-compatibility claim
for every older SDK/device. See the
[constructor evidence](../../investigations/compiler-weak-references.md#zero-argument-construction).

#### WeakReference containers

`Array<WeakReference>`, `Array<WeakReference or Null>`, and typed Dictionary
WeakReference/nullable-WeakReference values require API 1.2.0. Array elements
and both Dictionary parameters remain invariant. Existing native container
methods retain their API requirements. WeakReference Dictionary keys remain
unsupported following an [insertion
failure](../../investigations/compiler-weak-references.md#dictionary-key-rejection)
in SDK 9.2.0 programs on the `fr265` simulator, including live targets. This is
a bounded compatibility decision, not a claim about every Garmin runtime.

Weak-bearing literals allocate privately and evaluate entries in source order.
Homogeneous weak handles may infer a contract; empty and Null-only literals need
an expected type. Null-only nullable payloads with supported scalar Dictionary
keys may use existing DATA constants. Weak handles/targets are never encoded as
pooled objects or numeric addresses. Nested containers remain unsupported.

Arrays preserve their element type through parameters/results, indexed reads and
writes, and matching conditional arms. `size`, `toString`, `add`, `addAll`,
`indexOf`, `remove`, `removeAll`, `reverse`, and `slice` use the existing
profiles. Reverse/slice produce independent Arrays with the same weak handle
values; search/removal uses native WeakReference equality. Non-null weak
elements support sorting with a non-null checked
[Comparator](#source-comparators); sorting handles does not retain
their targets.

Dictionary `get`/indexing adds Null for a missing key, including for non-null
WeakReference values. `values()` returns an Array with exactly the declared
value contract; it does not add the missing-key alternative. Existing `put`,
indexed writes, `hasKey`, `remove`, and observers preserve that contract.

Nullable element/lookup members need a fresh guarded local snapshot. A test of
another mutable read does not refine the value. Storage and copies retain the
weak handle without retaining its target; a non-null handle may already be
expired. Neither storage nor `stillAlive()` removes Null from a later `get()`
Object result. See the
[specimen](../../../examples/compiler-weak-references/README.md) and
[evidence](../../investigations/compiler-weak-references.md#container-storage).

### Lang.format

`Lang.format(format as Lang.String, parameters as Lang.Array) as Lang.String` is
available since API 1.0.0. Module aliases and fully qualified Toybox paths use
the ordinary native-name rules. Arguments are evaluated left to right; the
runtime substitutes one-based `$1$`, `$2$`, etc. placeholders. The compiler
passes the template unchanged and does not perform formatting or conversion. The
returned String supports normal storage, calls, returns, and discarding.

The Array parameter is erased. Any supported concrete Array element contract can
pass unchanged; it does not become `Array<Object>` or lose its invariant element
type. Direct literals receive nullable Object construction context, allowing
heterogeneous scalar/reference contents, nulls, and empty literals. This context
does not widen inference for unrelated literals or existing arrays. Null/opaque
Object arguments cannot establish the required Array contract.

Formatting can invoke element `toString` implementations. Treat it as a possible
shutdown callback when checking required constructor field initialization.
Runtime conversion and placeholder failures remain runtime behavior. The
[specimen](../../../examples/compiler-lang-format/README.md) and
[evidence](../../investigations/compiler-lang-format.md) cover substitution,
evaluation order, and a missing-parameter failure.

## Arrays

Computed literals whose first element has type Any infer dynamic elements. For
example, `function wrap(x) { return [x, 7]; }` constructs an Array with Any
elements and exposes Any as the function result. Fills, indexed writes, and
reads use the ordinary dynamic boundary rules, including Null values; the
runtime Array remains a shared mutable reference. Existing concrete element
contracts stay invariant. General heterogeneous inference without a dynamic
first value, unconstrained empty literals, and explicit `Array<Any>` source
annotations remain outside this profile.

### Char arrays

Nonempty Char-array literals accept Char-valued expressions, for example `['a',
'b']`, `[65.toChar(), 'z']`, and `[Chars.make(), 'x']`. They produce a fresh
non-null `Array<Char>` and may receive postfix reads/calls. Number literals
follow the [Number-array contract](#number-arrays); View literals follow the
[native object contract](../sdk/README.md#native-views); String literals follow the
[String-array contract](#string-arrays). Other element types, mixed
types, and nested arrays are rejected. Container separators follow the
[literal grammar](grammar.md#lexical-rules-and-literals). Empty literals follow
the [contextual construction rules](#empty-array-literals).

Homogeneous nonempty lists of Number, Char, or String tokens become typed IR
constants and DATA array records loaded with `apush`. Each execution creates a
fresh mutable array. Computed expressions are not folded into these records.

Other nonempty construction allocates the final length before evaluating
elements left to right, once each, and stores each result with VM array
instructions. A private builder prevents unfilled slots from escaping; branching
elements use a reusable staging local. Builder and ordinary slots share the
normal frame budget. Operation/nesting limits still apply. Allocation and store
failures remain runtime obligations. See
[literal evidence](../../investigations/compiler-array-literals.md).

String `toCharArray()` returns a non-null `Array<Char>` since API 1.3.0. The
supported source annotation is `Toybox.Lang.Array<Toybox.Lang.Char>`, with
ordinary token whitespace permitted. Array parameters and results retain the
element contract; source array results require an explicit annotation. Locals
may store, copy, and reassign matching Char-array values, and user calls may
pass/return them. [Number elements](#number-arrays) have a separate supported
profile. Raw Array annotations remain unsupported. Other supported element domains
are described in the following sections. Nullable Array references follow the
[union rules](#native-unions). Element contracts are invariant; Char arrays are not widened to
mutable arrays of Object.

Char-array receivers support `size()` returning Number, `toString()` returning
String, `indexOf(Char)` returning Number, `add(Char)` returning `Array<Char>`,
`addAll(Array<Char>)` returning `Array<Char>`, `remove(Char)` and
`removeAll(Char)` returning Boolean, `reverse()` returning `Array<Char>`, and
`slice(Number or Null, Number or Null)` returning `Array<Char>`. Search returns
the first matching index or `-1` when absent. Generic API parameters and results
substitute the receiver's element type; executable array values remain limited
to Char elements. Array methods dispatch through the Array class. Explicit
context checks both the Array class and Char element, requiring API 1.3.0 for
this profile.

`add` appends one element; `addAll` appends the argument's elements. Both mutate
the receiver and return that same array. `remove` deletes the lowest-index
match, while `removeAll` deletes every match. Each returns true if it removed an
element, otherwise false. Remaining elements retain their relative order, and
aliases observe the mutation. Calls evaluate receiver before arguments, once
each; native mutation uses the existing ordered-call validation and lowering.
See [mutation evidence](../../investigations/compiler-array-mutation.md).

`reverse` returns the elements in reverse order and leaves the receiver's
contents unchanged. `slice` uses a start-inclusive, end-exclusive range. Null
start/end select the beginning/end; negative bounds offset from the end, and
out-of-range bounds are clipped. A start after the end returns an empty array.

Convert arrays to String before printing. Arrays cannot feed arithmetic,
concatenation, comparison operators, Boolean conditions, or console entry
results. Native Array constructors and ordinary weak targets outside the bounded
profile remain unsupported. Default sorting follows its
[separate contract](#default-array-sorting). Native Array
calls may be values or statements that discard the result. Guard nullable String
receivers before conversion; that proof does not imply support for nullable
Array results. See the
[Char-array evidence](../../investigations/compiler-char-arrays.md).

`array[index]` reads a Char from a non-null Char array. The index must be a
non-null Number; a fresh nullable Number read under an existing null proof is
accepted. Indexing and member calls share the postfix level and may interleave:
`array[0].toUpper()` and `text.toCharArray()[0]` are supported. Array evaluates
before index, once each. Logical subexpressions use the ordinary temporary
snapshot path to preserve this order across joins. The combined postfix/index
nesting limit is 64, and diagnostics retain the complete indexed-expression
span.

The compiler emits `agetv`, with no clamping, negative-index adjustment, or
static bounds analysis. An index outside `0..array.size()` remains a runtime
obligation; the tested VM reports a fatal bounds error. Reads are
order-sensitive in IR validation even without other calls/writes, because
failure order is observable. Indexing a String itself and nullable Array
receivers remain unsupported; Dictionary indexing follows its [own
contract](#dictionaries). See the
[array-read evidence](../../investigations/compiler-array-reads.md).

### Number arrays

String `toUtf8Array()` returns a non-null `Array<Number>` since API 1.3.0. Its
initial elements are UTF-8 bytes. The Array element contract is Number, so later
appends need not stay within byte range. Source parameters/results use
`Toybox.Lang.Array<Toybox.Lang.Number>`; non-Number results require explicit
source annotations as usual.

Number arrays use the same storage, native-method, and evaluation-order rules as
Char arrays. `add`, `indexOf`, `remove`, and `removeAll` require Number;
`addAll` requires `Array<Number>`. `reverse` and `slice` retain `Array<Number>`.
Indexing produces Number, permitting ordinary arithmetic and native Number
calls. Number and Char arrays are invariant and cannot substitute for each
other; no implicit numeric/Char conversion is performed.

The Array/Number classes require API 1.0.0; conversion and the mutation/search/
transform methods above require 1.3.0. Context checking keeps signature-type
availability separate from method availability. Nullable String receivers need
proof before conversion. Nullable arrays and unsupported element types remain
rejected.

Nonempty literals such as `[1, 2]`, `[n, -n]`, and `[M.make(), 3]` produce a
fresh `Array<Number>`. The first element selects the supported element type from
its expression contract; every element must have that same non-null type.
Parameters, locals, user/native calls, arithmetic, and element reads can supply
the first element. Nullable values require the usual branch proof, including
when used first. Allocation precedes element effects; evaluation and typed
stores proceed left to right as for Char literals. Construction requires
Array/Number availability (1.0.0), without an incidental native `add`
requirement. Element calls and other Array methods retain their own API
constraints. Empty construction uses [contextual types](#empty-array-literals).
See the
[literal evidence](../../investigations/compiler-number-array-literals.md).

Arrays must be converted to String before printing, and cannot be console entry
results. See the
[Number-array evidence](../../investigations/compiler-number-arrays.md).

### Numeric arrays

`Array<Long>`, `Array<Float>`, and `Array<Double>` use fully qualified scalar
names in source and are invariant. Parameters/results, locals, reads/writes,
empty construction, and the supported generic Array methods preserve the exact
element contract. No Number/Long/Float/Double substitution or implicit widening
is permitted at array boundaries. Indexes remain non-null Number.

Each element also supports `or Null` under the shared nullable-element rules.
Guard a saved nullable read before scalar use; guarding one read does not narrow
a later read of the same array. Non-null arrays reject Null values/arguments.

Literal-only homogeneous arrays use typed DATA constants, preserving signed Long
and floating bit patterns. Nullable constants may contain Null. Computed
literals use ordered private construction and may include branching elements;
expected contracts reach empty and nullable literals. Repeated constant loads
produce independent mutable arrays.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`,
`reverse`, and `slice` use the existing generic Array profile. `addAll` takes an
invariant Array of the same element contract; `removeAll` takes an element.
Native search and removal semantics remain runtime supplied. Allocation,
indexing, `size`, and `toString` require API 1.0.0; the other listed methods
require 1.3.0.

Non-null indexed updates/compounds follow scalar operator promotion and must
retain the element type. For example, `Float += Number` is valid but `Float +=
Long` would produce Double and is rejected. Nullable elements cannot be compound
targets without a supported direct non-null read contract. Non-null elements
support [default sorting](#default-array-sorting) and
[source comparators](#source-comparators). Broader callback consumers and nested containers remain deferred. Whole-array
joins and nullable references use the [union rules](#native-unions). See the
[example](../../../examples/compiler-numeric-arrays/README.md) and
[record/runtime evidence](../../investigations/compiler-numeric-arrays.md).

### String arrays

`Toybox.Lang.Array<Toybox.Lang.String>` is an invariant non-null String-element
contract. Parameters/results preserve that type; indexed reads produce String
and writes require String. Nullable source values require a non-null proof
before construction, writes, or native arguments. Mixed element inference and nested arrays remain unsupported for this profile.
Nullable Array references require a non-null proof before receiver use.
Nullable String elements have a [separate contract](#nullable-array-elements).

Homogeneous literals such as `["one", "two"]` use pooled DATA constants.
Repeated text shares a local pool entry; repeated execution loads independently
mutable arrays. Computed literals, including conditional and guarded elements,
use the same ordered private construction as Char/Number arrays. An expected
String-array contract supplies the type of a direct `[]`.

`size`, `toString`, `indexOf`, `add`, `addAll`, `remove`, `removeAll`,
`reverse`, and `slice` use the existing Array contracts with String substituted
for the element type. Search/removal compare String text. Array/String types,
construction, indexing, `size`, and `toString` require API 1.0.0; the other
methods require 1.3.0. [Default sorting](#default-array-sorting) requires 5.0.0.
Source comparators follow their [separate contract](#source-comparators);
broader callback consumers remain deferred. Arrays must be converted to String
before printing.

See the [example](../../../examples/compiler-string-arrays/README.md) and
[runtime evidence](../../investigations/compiler-string-arrays.md).

### Boolean arrays

`Toybox.Lang.Array<Toybox.Lang.Boolean>` is an invariant non-null
Boolean-element contract. Literals infer Boolean; reads produce Boolean for
conditions, logical operators, and native Boolean methods. Writes and
element-taking Array methods require Boolean, with no Number conversion. Null
elements remain unsupported. Typed parameters/results and direct contextual `[]`
preserve the contract.

Literal-only arrays use `apush` constants; computed elements use ordered private
construction, including conditional/short-circuit expressions. Copies share
mutation; repeated constant loads are independently mutable. The supported Array
methods and API levels match the String/Number profile, including
[default sorting](#default-array-sorting) from 5.0.0. Source comparators follow their
[separate contract](#source-comparators); broader callback consumers remain deferred. See the
[example](../../../examples/compiler-boolean-arrays/README.md) and
[evidence](../../investigations/compiler-boolean-arrays.md).

### Default Array sorting

`array.sort(null)` sorts a non-null Array of Number, Long, Float, Double,
Boolean, Char, or String elements in ascending order. It mutates the receiver,
so aliases observe the new order; separately created copies retain their own
contents. Empty/singleton Arrays are accepted, and duplicates are retained. The
invariant element contract does not change. Sorting requires API 5.0.0.

The result is Void: use a call statement, not an initializer, return expression,
or chained value. Evaluate the receiver once before the Null argument, then
invoke the runtime method and discard its VM result slot. The compiler does not
implement a sorting algorithm or promise sort stability, NaN ordering,
locale-sensitive String collation, or atomic behavior if comparison fails.

Garmin's full parameter type is `Lang.Comparator or Null`. Comparator is a
structural interface requiring `compare(Object, Object) as Number`; it is not
Method or an instantiable native class. Null selects default sorting; supported
source comparator instances follow the contract below. Non-null
[Comparable elements](#comparison-interface-arrays) also support default sorting.
Non-null Object elements also support default sorting. Their declared type does
not prove that the runtime contents are mutually comparable: String/Number
comparisons, for example, raise the native UnexpectedTypeException. Preserve
those runtime failures instead of inventing a narrower static element contract.
Nullable elements remain unsupported, even if current contents are non-null; an
indexed write does not prove whole-Array initialization or comparability.

See the [example](../../../examples/compiler-array-sort/README.md) and
[evidence](../../investigations/compiler-array-sort.md).

### Source comparators

`array.sort(comparator)` accepts an instance of a declared source class that
supplies `compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as
Toybox.Lang.Number`. Both parameter types, arity, and result must exactly match
the interface declaration. Narrower scalar parameters, nullable Object
parameters/results, and missing/extra parameters are rejected. Untyped source
parameters still infer Number and therefore do not establish this interface.
Matching is structural: no Comparator superclass or `implements` syntax is used.

Supported receivers are Arrays with any supported non-null element contract,
from API 5.0.0. This includes Object, Symbol, Method, WeakReference, ByteArray,
View, and [comparison-interface elements](#comparison-interface-arrays). For
elements without a supported default-order profile (Symbol, Method,
WeakReference, ByteArray, View, or Comparator), the comparator must be provably
non-null. A nullable Comparator could select an unsupported default order.
Scalar, Numeric-alias, Comparable, and Object elements also accept Null or
Comparator-or-Null arguments under the default-order rules above. Nullable
element contracts remain excluded: the checked callback takes two non-null
Objects. Container invariance and exact element identities are unchanged. A
negative/zero/positive Number orders the first element before, equal to, or
after the second. The runtime calls the source `compare` body on the supplied
instance, preserving its fields. It controls comparison count and order; callers
must supply a consistent ordering and must not depend on a particular sorting
algorithm. Custom comparators may choose an order different from the default,
including ordering Object string conversions.

Construction, aliases, exact source-class parameters/results, and source
reference fields can supply a comparator while retaining class identity.
Whole-unit validation resolves the class and rechecks the required method; an
erased View/Object or Method cannot establish the structural contract. The
[comparison-interface profile](#structural-interfaces) also accepts Comparator
parameters/results and locals, including Comparator-or-Null values. Bare `self`
as a comparator remains unsupported. Object-rooted source classes support
construction, exact source signatures/reference fields, and comparator use
through their [class contract](#object-rooted-classes). The existing source
class/base and constructor requirements still apply.

Sort uses ordinary native-call lowering with receiver-before-argument order. The
runtime callback member `compare` uses its pinned symbol ID; it remains a source
method body. Constructor checks propagate known comparator shutdown requests
through sort and source wrappers, rejecting them while reference fields are
pending. Default sorting of opaque Object elements conservatively permits erased
callback effects; reject it before required fields are initialized. A pinned
source comparator retains its known-body effect analysis. These checks do not
prove callback termination, exception safety, or comparator consistency.

See the [example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md).
Object/reference-element ordering has a separate
[example](../../../examples/compiler-object-sort/README.md) and
[evidence record](../../investigations/compiler-object-sort.md).

### Nullable Array elements

`Toybox.Lang.Array<T or Null>` supports numeric, String, Boolean, Char, or
Symbol T with invariant element contracts and a non-null Array reference.
Indexed reads produce the base type or Null. Stores and native `add`, `indexOf`,
`remove`, and `removeAll` accept either alternative; `addAll` requires the same
Array contract. `reverse` and `slice` retain it. API levels match the
base-element Array profile.

Direct literals acquire a nullable element contract from an annotated return, a
typed user/native argument, or assignment to an existing nullable-element Array
local. This includes empty, Null-only, scalar-only, and computed literals. For
example, a function returning `Array<Number or Null>` may return `[1, null]`.
Literal tokens use typed DATA constants; other expressions allocate before
ordered fills. Branching nullable literals use a separate staging local per
element so differing operand types do not change an existing local's contract.

Existing Array references cannot widen or narrow their element contracts.
Unannotated literals retain the non-null inference rules; `[null]` and `[1,
null]` without nullable context remain rejected. Nested arrays remain deferred.
Nullable Array references require a non-null proof before receiver use.

Store a nullable read in a local and guard a fresh local read before scalar
operators, methods, or printing. A present false Boolean is distinct from Null.
Writes and `a[index] != null` do not prove that a later indexed read is
non-null. A scalar saved before Array mutation retains its earlier value.
Reassigning that scalar invalidates its null proof. See the
[example](../../../examples/compiler-nullable-arrays/README.md) and
[evidence](../../investigations/compiler-nullable-arrays.md).

### Object containers

`Array<Lang.Object>` and `Array<Lang.Object or Null>` retain non-null Array
references with invariant erased element contracts. Dictionary values likewise
accept `Lang.Object` or `Lang.Object or Null` with any supported key contract;
Object keys remain unsupported. Canonical names, imports, and Lang aliases work
in signatures and contextual construction. Explicit nested parameters and exact
source-class element/value contracts remain separate requirements.

Object slots accept supported scalar classes, numeric aliases, source/native
objects, comparison interfaces, and container/Method/WeakReference references.
Known enum values may widen to Object with their Number runtime representation;
this does not make them assignable to Number or permit numeric operators on
nominal enum types. Nullable Object slots additionally accept Null. Mutable
containers are invariant: existing Array/Dictionary references cannot widen
their parameters to Object, even though individual values can be stored there.

Explicit context admits heterogeneous literal contents. Without context, a value
already represented as Object can establish that erased contract; mixed scalar
literals alone do not infer it. Scalar constants use tagged DATA records,
retaining class, exact bits, order, and fresh mutable results. Symbol/reference
or computed contents use the existing ordered private builders. Check API
availability for each constant payload class as well as the erased container
type; storing Char under Object still requires API 1.3.0.

Indexing and native methods preserve erased types. Non-null Object reads support
Object observers; specific members need the existing lexical assertion or class
proof. Erasure does not recover nested container parameters. Dictionary lookup
adds absent-key Null, while `values()` retains the stored value contract.
Nullable reads require a guarded local snapshot; testing a mutable container
read does not refine another read. Sized allocation requires nullable elements.
Non-null Object arrays follow the [sorting contracts](#default-array-sorting);
nullable elements require further ordering/callback contracts.

Containers and their shallow enumerations retain strong references to stored
objects. Removal/replacement releases that ownership; other aliases or snapshots
may still keep a weak target alive. A source object retrieved under Object loses
its exact source identity until a supported runtime class proof reestablishes
it. See the [example](../../../examples/compiler-object-containers/README.md)
and
[evidence](../../investigations/compiler-object-containers.md).

### Empty array literals

A direct `[]` requires a concrete expected `Array<Number>`, `Array<Char>`,
`Array<String>`, `Array<Boolean>`, `Array<Symbol>`, `Array<View>`, or a
supported
[nullable element contract](#nullable-array-elements).
Parentheses preserve direct context. Supported contexts are:

- An explicitly annotated array return: `return [];`.
- A typed user-call argument, including module, cross-file, and self calls.
- Assignment to an initialized array local: `var a = [1]; a = [];`.
- A native argument whose resolved contract is an array, such as `a.addAll([])`.

The literal allocates a fresh empty array at its position in evaluation order.
Reassignment replaces the local reference; existing aliases retain the old
array. No `add` call or element-storage temporary is emitted. Empty
Number/String/View allocation requires API 1.0.0; Char element availability
requires 1.3.0. Enclosing calls retain their own API requirements.

Context applies only to the direct empty literal, not through arbitrary
operators or calls. `var a = []`, `[].size()`, and `return [].reverse()` are
rejected, including the last case under an array return annotation. Local type
annotations, nested arrays, and inference from later mutations are unsupported.
Nonempty literals keep their independently inferred contract; argument/return
context cannot coerce mixed elements or substitute Number and Char arrays. Saved
IR records the concrete allocation type and rechecks all uses. See the
[complete example](../../../examples/compiler-empty-arrays/README.md) and [evidence](../../investigations/compiler-empty-arrays.md).

### Sized array allocation

`new[length]` creates a fresh non-null Array with Null in every slot. It needs a
direct expected Array contract with supported nullable elements, supplied by a
typed return/argument or an established local. Existing supported conditional
result contracts pass that context into their arms. For example:

```monkeyc
function allocate(n as Toybox.Lang.Number)
    as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
    return new[n];
}
```

Evaluate the non-null Number length once before allocation. Conditional lengths
retain their branch effects. No clamping, element conversion, or initialization
loop is inserted; invalid sizes and allocation failures remain runtime concerns.
Reads preserve the nullable element contract after writes. Reassignment replaces
the reference while aliases retain the previous array.

An untyped `var a = new[n]`, non-null element contracts (even `new[0]`), nested
arrays, and inference from later writes remain unsupported. Conditional arms can
select sized arrays when their invariant element contracts match. Type
availability is checked independently of Array methods: nullable Number
allocation requires API 1.0.0, while nullable Char requires 1.3.0. See
[IR allocation](../ir/README.md#array-allocation) and
[runtime evidence](../../investigations/compiler-empty-arrays.md#sized-allocation).

## Dictionaries

`{}` creates a fresh empty `Toybox.Lang.Dictionary` (API 1.0.0). Whitespace and
comments may separate the braces. Dictionary references may initialize/copy or
replace Dictionary locals and pass through explicitly annotated non-null
Dictionary parameters/results. Copies preserve reference identity; reassignment
does not copy contents. Calls and construction retain source evaluation order.

`Toybox.Lang.Dictionary<K, V>` accepts invariant non-null numeric, String,
Boolean, Char, or Symbol keys and values of those types, optionally `or Null`.
It also accepts [comparison-interface keys and
values](#comparison-interface-dictionaries). The Dictionary itself is non-null.
Char keys or values require API 1.3.0; the other contracts require 1.0.0.
`DictionaryType` carries the bounded contracts through source, semantic IR,
locals, function arguments/results, and API checks.

A direct `{}` receives its key/value contracts from an annotated return, call
parameter, or reassigned typed local. Without that context it stays opaque;
later uses do not retrospectively type an opaque local. Different key or value
contracts cannot widen or narrow through assignment. Typed references may pass
to opaque Dictionary parameters/results, but erasure cannot be reversed.

[Numeric alias containers](#numeric-alias-containers) additionally admit bounded
mixed numeric classes under explicit or already established alias contracts.

Nonempty `{key => value, ...}` literals accept expressions producing homogeneous
numeric, String, Boolean, Char, or Symbol keys and homogeneous values with
optional Null. Without context, a non-null value establishes the value type; any
Null makes it nullable. Null-only values require an expected nullable contract.
Typed contexts check every entry without changing invariant key/value types.
Null keys, mixed scalar kinds without an expected interface, numeric alias, or
Object contract, and container-valued entries outside the Object value profile
are unsupported. Container separators follow the
[literal grammar](grammar.md#lexical-rules-and-literals). Nested Dictionary
literal syntax is also unsupported, including inside entry call arguments.
Ordinary calls and Array reads may produce scalar entries.

Literal-only entries use `hpush` and a [scalar DATA
constant](../formats/data.md#dictionary-constants). Signed Number tokens use the
existing 32-bit range. Constant strings are UTF-8; their local pool length,
including the zero terminator, must fit u16. Every entry is type-checked before
repeated scalar keys retain their last value. Numeric constant keys match exact
class and bits: signed zeros stay distinct, and Double NaN keys never coalesce.
String keys match text. These rules preserve the tested Dictionary behavior
rather than ordinary numeric operator equality. Runtime hashing/lookups remain
native operations; no key conversions are emitted. See [wider numeric key
evidence](../../investigations/compiler-numeric-dictionaries.md). Parentheses
around literals do not introduce effects or require computed construction.

Computed literals allocate a private Dictionary before entry expressions. Each
pair evaluates key, then value, then inserts before the next pair. Every
overwritten expression still executes. The `newd` argument is the source entry
count, including duplicates; its capacity/allocation effect is not specified. No
partially constructed reference escapes. Branching operands use private locals
under the existing frame limit. Every evaluation returns a fresh mutable
Dictionary; String keys compare by text.

Computed value inference retains candidate scalar nullability, including
nullable call results. An expected contract instead checks actual values using
the full IR null proofs. A guarded nullable key still needs a non-null proof;
selecting its non-null class during inference does not authorize an unsafe key.

Opaque and typed references support `size()` → Number, `isEmpty()` → Boolean,
and `toString()` → String. Dictionary's `toString` override requires API 1.0.1;
it does not fall back to the older Object declaration. Conversion reflects the
current contents and produces a String unaffected by later Dictionary mutation.
Entry order is unspecified. Conversion uses ordinary native dispatch and String
result contracts, including chaining and discarded results.

Opaque and typed references also inherit the
[Object equality and hash observers](#object-equality-and-container-observers).

Typed references additionally support the following API 1.0.0 contracts:

| Form | Result |
|---|---|
| `d[key]`, `d.get(key)` | V or Null |
| `d[key] = value`, `d.put(key, value)` | Statement only |
| `d.hasKey(key)` | Boolean |
| `d.remove(key)` | Statement only |
| `d.keys()` | Array<K> |
| `d.values()` | Array<V> |

Enumeration returns independent mutable Arrays containing the current keys or
values, with unspecified order. Later Dictionary changes do not update these
Arrays, and Array edits do not change the Dictionary. Nullable V remains
nullable in the returned Array; non-null V does not acquire lookup's missing-key
Null. Array argument/result invariance, element checks, null proofs, and method
API minimums still apply. Opaque Dictionary references cannot supply K or V.

Reads require a key of K; writes require K and V. Null is permitted only when V
includes Null. Inserting Null retains the key; removal deletes it, and removing
an absent key leaves the Dictionary unchanged. Copies share mutations. Receiver,
key, and value expressions execute once in that order. Indexed access uses
`agetv`/`aputv`; explicit methods retain native dispatch.

Every lookup is nullable, including reads from non-null V and `hasKey` branches.
Store the result and use the existing non-null proofs before arithmetic, String
methods, or printing. A stored scalar result retains its value across later
Dictionary mutations. `hasKey` does not establish a proof for a later lookup.
Guarded nullable keys use the existing proof rules; there is no implicit key
conversion. Indexed compound assignments and increments remain unsupported.

Chaining and discarded results use existing native-call rules. Native callback
Dictionary-or-Null state supports these opaque methods under a non-null proof;
unguarded access is rejected. Ordinary nullable Dictionary signatures and
reference assignments use the [union rules](#native-unions).

Matching typed Dictionary conditional arms retain their exact invariant
key/value contract and selected reference. Different contracts, opaque
Dictionary joins, and Dictionary/Null joins retain native union alternatives.
Conditional literals receive an available declared contract in each arm.

Dictionary `==`/`!=`, direct printing, `new Dictionary()`, and the other methods
remain unsupported. No key or value contract is inferred from an empty literal
without context. See
[the example](../../../examples/compiler-dictionary/README.md) and
[SDK evidence](../../investigations/compiler-dictionary.md).

#### Planned Dictionary extensions

These are intended contracts, not executable support yet. Do not infer key/value
contracts from later writes or reinterpret an opaque callback Dictionary as a
typed one.

Null keys and explicit nested parameter contracts need separate rules before
acceptance. Native reference unions do not change literal content inference. Nested references stored as
[Object values](#object-containers) remain erased. Object `method` follows the
[bounded Method profile](#method-values-and-invocation);
ordinary `weak` follows the [bounded reference
profile](#ordinary-weak-references).

## ByteArray

`Toybox.Lang.ByteArray` is a mutable reference, separate from `Array<Number>`.
Canonical names, imported types, and Lang aliases work in parameters/results.
Locals, reassignment, and matching conditional arms preserve the selected
reference; aliases observe its mutations. API minimum: 3.0.0.

`ByteArray or Null` is supported in parameters/results, conditional joins, and
inferred locals. ByteArray/Null alternatives join without allocation or
conversion. Sequential writes update inferred local contracts.
Direct null checks on fresh local/parameter reads establish non-null use in
branches, short-circuit expressions, conditional arms, and loops. Early returns
can exclude Null from a following path. Methods, indexing, and non-null typed
calls require that proof; reassignment invalidates it. An alias or a saved
Boolean does not inherit another read's proof. API availability applies even to
unused nullable declarations or Null-only results.

Nullable storage holds an ordinary strong reference while present. Clearing one
alias leaves the object alive through any remaining strong reference; clearing
the last one allows an existing weak reference to expire.

`new[length]b` allocates a fresh zero-filled ByteArray without expected element
context. Evaluate its non-null Number length once before allocation. Conditional
lengths and allocation arms retain lazy evaluation and effect order. Invalid
sizes and allocation failures remain runtime concerns.

`[elements]b`, including `[]b`, creates a fresh ByteArray with Number/Char
elements. Valid literal Number tokens in `-128..=255` and ASCII Char tokens use
compact DATA constants, normalizing valid signed bytes to their unsigned bits.
Arithmetic and calls are not folded into constants. Computed literals allocate
their final length before evaluating elements left-to-right, storing each value
before evaluating the next. Conditional elements evaluate only their chosen arm.
Private construction cannot escape before every slot has been filled.

Out-of-range Number/Char tokens use indexed stores rather than truncation in a
constant payload, retaining the runtime range check. Other element types are
rejected. Repeated loads of a constant, including an empty constant, produce
independently mutable ByteArrays. No expected element context is required.

Index reads require a non-null Number and return an unsigned Number byte. Writes
accept Number/Char, including indexed compounds whose result matches that
contract. Evaluate receiver, index, then value once, preserving runtime
bounds/range checks. The compiler does not mask or clamp values; a stored `-1`
reads back as `255` in the verified simulator.

Supported native calls use ordinary value-call lowering and API checking:

| Calls | Arguments and result |
|---|---|
| `size()`, `toString()`, `hashCode()` | Number, String, Number respectively |
| `equals(value)` | Supported Object-or-Null value; Boolean result |
| `add(byte)` | Number/Char; ByteArray result |
| `addAll(bytes)` | ByteArray or Number/Char Array; ByteArray result |
| `indexOf(byte)` | Number/Char; Number index or `-1` |
| `remove(byte)`, `removeAll(byte)` | Number/Char; Boolean result |
| `reverse()` | Independent ByteArray with reversed elements |
| `slice(start, end)` | Number-or-Null indexes; independent ByteArray |
| `decodeNumber(format, options)` | NumberFormat and options; Numeric result |
| `encodeNumber(value, format, options)` | Numeric, NumberFormat, options; ByteArray result |

Both append methods mutate the receiver. Bulk Array inputs retain their
invariant non-null Number/Char element contract.

`method(Symbol)` and `weak()` use the existing inherited binding/lifetime
profiles. Weak retrieval remains Object-or-Null. Direct arguments to `addAll`
need an established Array element contract; ambiguous empty Array inference does
not choose Number or Char automatically.

Annotated [fields](#bytearray-fields) preserve these reference contracts, with
constructor initialization for non-null storage and guarded local snapshots for
nullable reads.

`Array<ByteArray>` and `Array<ByteArray or Null>` preserve invariant element
contracts from API 3.0.0. Nonempty literals can infer non-null ByteArray
elements; typed context also supports empty, nullable, and Null-only literals.
Computed elements use private Array initialization, preserving reference
identity and evaluation order. ByteArray values are never embedded as scalar
Array constant tokens; Null-only literals may use existing DATA constants.

Indexing, writes, and existing native Array methods retain that contract;
`addAll` requires matching element nullability. A nullable indexed read needs a
guarded local snapshot; a check on one indexed read does not prove another.
Sized allocation accepts only nullable elements because each slot starts Null.
Array copies made by `slice`/`reverse` share the ByteArray elements. Replacing
an element leaves saved aliases intact. Elements hold strong references until
removed or replaced, independently of any saved snapshot. Sorting and widening
to ordinary or heterogeneous Array elements remain unsupported.

`Dictionary<K, ByteArray>` and `Dictionary<K, ByteArray or Null>` use existing
supported key contracts from API 3.0.0. Nonempty literals infer ByteArray
values; explicit context also permits empty or Null-only literals. Computed
entries use private ordered Dictionary construction. Value types remain
invariant across parameters, calls, assignments, and saved IR.

Indexed reads and `get` return ByteArray-or-Null even for non-null value
contracts because a key may be absent. Store a read in a local and guard it
before ByteArray operations; `hasKey` or a check on another lookup is not a
non-null proof. Writes and `put` must satisfy the declared value nullability.
`values()` returns an Array with that exact value contract, retaining strong
references independently of later Dictionary changes. Removing/replacing a value
leaves saved aliases and enumerations intact; clearing the last strong reference
allows a weak target to expire. Enumeration order is not specified.

Object/Object-or-Null storage can recover ByteArray through a positive
`instanceof Lang.ByteArray` branch on a fresh local/parameter read. This is a
concrete class contract, with no element parameter to infer. It enables methods,
indexing, and typed calls on the proven path, including saved Object results
from weak retrieval or Method invocation. `as Lang.ByteArray` and `as
Lang.ByteArray or Null` follow the
[unchecked lexical assertion contract](#lexical-casts); neither checks nor
converts the runtime value. Both forms require API 3.0.0.

`Dictionary<ByteArray, V>` accepts existing value contracts, with API 3.0.0
availability on the key even for unused declarations. Keys must be non-null;
literal construction uses private fills, never scalar DATA keys. `keys()`
returns `Array<ByteArray>` and retains strong references after entry removal.
The verified simulator distinguishes separately allocated ByteArray keys even
when their contents and hashes compare equal. Aliases address the same entry.
The compiler preserves references and performs no key copying, deduplication, or
hash normalization; mutation behavior is delegated to the runtime.

Numeric conversion requires API 3.1.0. `Lang.NUMBER_FORMAT_*` constants are:

| Suffix | Value | Binary representation |
|---|---|---|
| `FLOAT` | 0 | IEEE 754 binary32 |
| `SINT16` | 1 | Signed 16-bit |
| `SINT32` | 2 | Signed 32-bit |
| `SINT8` | 3 | Signed 8-bit |
| `UINT16` | 4 | Unsigned 16-bit |
| `UINT32` | 5 | Unsigned 32-bit |
| `UINT8` | 6 | Unsigned 8-bit |

`Lang.ENDIAN_LITTLE` is 0 and `Lang.ENDIAN_BIG` is 1. These constants require
API 3.1.0. NumberFormat and Endian annotations retain distinct enum contracts;
neither plain Number nor the other enum satisfies them. The VM receives their
integer representations without a separate runtime class or conversion opcode.
An explicit lexical assertion can claim an enum type without validating its
value.

Both conversion calls require an options argument. Direct literals, including
`{}`, receive a checked record contract from call context. Up to two literal
Symbol fields are supported: `:offset` requires Number and defaults to 0;
`:endianness` requires Endian and defaults to little endian. Unknown keys or
wrong field types are rejected. A literal containing an Endian constant/local
under `:endianness` can infer this record contract for reuse through a saved
local. An arbitrary homogeneous Dictionary cannot claim the record proof.
General structural source annotations and record mutation remain unsupported.

Private construction allocates before field effects, preserves source order, and
exposes the record only after every declared field is filled. Values use
ordinary runtime Dictionary storage. Invalid asserted formats/endian values,
offset bounds, conversion ranges, and failures remain native checks; no masking,
conversion, or portable exception guarantee is added by the compiler.

Encoding mutates and returns the receiver. Decoding returns Numeric without
assuming a result class from the format token: UINT32 can require Long, while
FLOAT produces Float in the verified runtime. Use Numeric's common methods or
runtime-class refinement before class-specific operations. See the
[example](../../../examples/compiler-byte-arrays/README.md),
[IR contract](../ir/README.md#bytearray-values-and-allocation), and
[evidence](../../investigations/byte-arrays.md#source-and-ir-profile).

## Structural interfaces

### Comparison interface values

`Toybox.Lang.Comparable` and `Toybox.Lang.Comparator`, optionally `or Null`, are
supported parameter and result contracts from API 5.0.0. Module aliases and
imports resolve them like other API types. Comparable exposes `compareTo(Object)
as Number`; Comparator exposes `compare(Object, Object) as Number`. Calls use
ordinary receiver/member dispatch, evaluate receiver before arguments once, and
preserve the implementing instance. Supported inherited Object observers use
the ordinary Object executable profile. Comparator values can also supply Array
`sort`.

User calls, returns, and fixed interface storage accept the same interface,
a verified native scalar witness, or an exact source-class witness. Unit
validation rechecks every required source member, arity, parameter, and result
contract. Opaque Object/View values, a `has` test, and source classes with
narrowed parameters do not establish the interface. A standalone function can
retain unresolved source witnesses; assembling a complete unit requires proof.

Number, Long, Float, Double, Boolean, Char, and String satisfy Comparable. The
bounded native witness check requires the catalogue's exact method signature and
an effective introduction level no later than Comparable's API 5.0.0
requirement. Checking the receiving interface therefore covers these native
members as well. No runtime conversion, allocation, or wrapper is added. Other
native types do not establish an interface witness. A nullable scalar needs a
non-null/concrete proof before non-null Comparable use; a positive concrete
class test on an Object read can likewise establish a scalar witness. Comparison
still follows the receiver's runtime method; a common interface does not
guarantee that arbitrary operand domains can be ordered or that a call cannot
fail.

A positive native `instanceof` test refines fresh local/parameter reads of
Comparable to the compatible Number, Long, Float, Double, Boolean, Char, or
String contract. The same rule applies to Comparable-or-Null and removes Null
on that edge. Concrete operations and forwarding to Comparable consumers can
then use the read without a cast or conversion. The storage's interface
contract remains unchanged by the test; writes invalidate facts, and untested
aliases do not inherit them. Refinement requires a verified native interface witness:
testing a Comparator against Number does not authorize Number operations.

An interface parameter or call result retains its contract through inferred
locals, aliases, and same-interface assignment and conditional joins. A
Comparable local also accepts a verified scalar assignment. A conditional with a
Comparable arm and a verified scalar arm retains Comparable in either arm order.
Different scalar witnesses retain a native union when no narrower numeric
alias covers them; each alternative can still satisfy a Comparable consumer. There is no
implicit conversion between Comparable and Comparator. Writes to fixed interface
storage defer their source signature proof to unit validation, including after
IR reload. Inferred local bindings may change type; a later interface consumer
still requires the appropriate witness.

A nullable interface accepts Null and compatible non-null values. Nullable
scalars can widen to Comparable-or-Null. Nullable reads require a non-null proof
before member calls or non-null forwarding; inferred local writes update their
flow contract. Equality/inequality with Null works in either
operand order. The shared branch rules cover early returns, conditional arms,
and loops; writes invalidate local proofs, and aliases/stored Boolean guards do
not transfer them. A conditional combining an interface and Null produces the
corresponding nullable interface. Existing nullable-interface arms can also
accept compatible scalar/interface alternatives under the shared union rules.

`Array.sort` accepts Comparator-or-Null directly: Null selects default ordering,
and a non-null instance supplies custom ordering. This does not permit calling
`compare` on a nullable receiver without a non-null proof.

Source classes also support [comparison-interface
fields](#comparison-interface-fields) with the same write contracts and checked
initialization. Arrays follow the
[interface element contract](#comparison-interface-arrays). Dictionaries follow
their [interface value contract](#comparison-interface-dictionaries). Unchecked
`as` assertions and interface runtime type targets remain unsupported. A
non-null interface can widen to Object without proving additional members. A
nullable interface can widen to Object-or-Null; Object requires a non-null
proof. Ordinary Object casts and concrete type tests retain their existing
rules.

Erased comparison calls, including sorting through a Comparator value, may
request shutdown. Constructor checks conservatively propagate that effect
through source wrappers and require reference fields to be initialized first.
This does not establish exception safety or callback termination.

### Comparison interface Arrays

Non-null Arrays support Comparable and Comparator elements, optionally `or
Null`, from API 5.0.0. Parameter/result annotations use
`Array<Toybox.Lang.Comparable>` and the corresponding Comparator/nullable forms.
Array contracts stay invariant: `Array<Number>` cannot widen to
`Array<Comparable>`, and non-null element Arrays cannot widen to
nullable-element Arrays.

An expected Array annotation or argument contract supplies literal element
typing, including empty and mixed native/source literals. Otherwise the first
element must already establish a supported element contract; arbitrary mixtures
do not infer an interface. Scalar-token literals use tagged DATA constants,
preserving Number/Char distinctions and each native runtime tag. Nullable
contracts also permit Null tokens, including Null-only literals. Each evaluation
produces a fresh mutable Array. Source-object and computed literals allocate
privately before evaluating and filling elements in order. The same declared
element contract applies to either construction path.

Indexed reads retain the declared interface, including Null where allowed; they
do not recover an exact source class. Member calls on nullable elements require
a guarded local snapshot. A test of one indexed read does not refine another
read. Writes, private literal fills, and typed native element arguments accept
compatible native/interface values or an exact source witness. Unit validation
checks required source members at every such ingress after IR reload. Opaque
Object values and incompatible signatures cannot bypass that proof.

The existing `size`, `toString`, `add`, `addAll`, `indexOf`, `remove`,
`removeAll`, `reverse`, and `slice` profiles apply; reverse/slice preserve the
element type. `addAll` requires the same invariant Array contract, though a
direct argument literal can receive that context. Inherited Object methods
retain their existing profiles. Collection operations that may call erased
element methods (`toString`, lookup, and removal) conservatively propagate
possible shutdown effects through constructor checking.

`Array<Comparable>.sort` accepts Null, a checked exact source Comparator, or a
Comparator(-or-Null) value. Null dispatches through element `compareTo` methods;
an explicit Comparator supplies `compare` instead. `Array<Comparator>.sort`
requires a non-null Comparator: its element contract alone does not prove
`compareTo` for default ordering. A guarded nullable Comparator snapshot can
satisfy this requirement. Exact source arguments retain their non-null proof
while unit checking verifies their comparison signature.

Callers must provide mutually comparable elements or a callback that handles
every element pair. The interface proves a signature, not a common comparison
domain: native scalar witnesses and unrelated source implementations need not
accept each other. Sorting mutates shared contents and retains the invariant
element type. Erased element/callback calls conservatively propagate possible
shutdown effects through constructor checking, including default sorting.
Runtime comparison failures, sort stability, and callback count/order are not
guaranteed. Existing [sort evaluation/lowering](#default-array-sorting) applies.

Sorting nullable elements, nested/general Object containers, and Array field
declarations remain outside this profile. Comparator values read from an Array
can supply an ordering callback. See the
[example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md#interface-arrays).

### Comparison interface Dictionaries

`Dictionary<K, Comparable>` and `Dictionary<K, Comparator>`, optionally `or
Null` on the value type, support scalar keys and the interface-key profile
below. Interfaces require API 5.0.0. Key/value parameters stay invariant across
assignments and calls; scalar-valued Dictionaries do not widen to
interface-valued ones. Erased Object values alone do not establish these
interface contracts;
[Object-valued Dictionaries](#object-containers) use a separate erased profile.

An expected Dictionary contract types direct literals and empty construction.
Values accept compatible native/interface witnesses or exact source instances;
unit validation checks required source members for every private literal fill,
indexed write, and `put` argument, including after IR reload. Opaque Object
values do not establish an interface. Computed literals retain ordered
evaluation and private construction. Scalar-only literals can use the existing
Dictionary constant payload, which distinguishes Number and Char tags under an
erased Comparable contract. Implicit inference of interface-valued literals is
deferred.

Lookup always produces the value type or Null, including for a non-null value
contract. Calls require a guarded local snapshot; `hasKey` or a guard on another
read does not refine the lookup. Indexed writes and `put` accept Null only for
nullable value contracts. `values()` returns `Array<V>`, preserving value
nullability without adding lookup's missing-key Null. `keys()` retains the key
contract. Enumeration snapshots membership, sharing element references under the
existing [Dictionary rules](#dictionaries).

Other supported Dictionary methods retain their profiles. Formatting may call
erased element methods, so constructor checking treats `toString` as possibly
requesting shutdown while required reference fields are pending. Opaque
Dictionary/Object string conversions retain this conservative effect: erasing
the container must not bypass initialization checks. No guarantees about
callback sequence, exception recovery, or element string representations follow.

See the [example](../../../examples/compiler-comparators/README.md) and
[evidence](../../investigations/compiler-comparators.md#interface-dictionaries).

#### Interface Dictionary keys

`Dictionary<Comparable, V>` and `Dictionary<Comparator, V>` support non-null
interface keys from API 5.0.0, combined with any supported scalar/interface
value contract. Container parameters remain invariant. `keys()` produces an
Array of the declared interface; lookup's result type and missing-key Null are
unchanged. Nullable or general Object key contracts remain unsupported. A
nullable key argument requires a fresh non-null proof.

Every literal fill, indexed read/write, and native `get`, `put`, `hasKey`, or
`remove` argument requires an interface witness. Unit validation checks exact
source members after IR reload as well as source compilation. Key identity uses
runtime hashing/equality, not `compareTo` or `compare`. Source instances may use
inherited identity or compatible [Object overrides](#object-rooted-classes).
Callers must keep key hashes stable while stored and make equal keys hash
consistently; signature checking does not establish these behavioral laws.

Interface-key literals use private runtime construction and ordered insertion.
They do not become Dictionary DATA constants, even with scalar-token contents;
saved IR rejects `.dictionary-constant` with interface keys. All-scalar-token
literals require a single key token kind. Mixed scalar-token literals are
rejected because SDK constant construction can conflate Number and Char keys
that runtime insertion distinguishes. Heterogeneous native witnesses can enter
through explicit `put` calls or computed/runtime key operands. No compile-time
key normalization or deduplication is applied to this path.

Construction fills and key lookups/writes/removal may invoke erased key methods.
Constructor checking conservatively propagates possible shutdown through those
operations and source wrappers while reference fields are pending. Formatting
may also invoke key methods. Enumeration does not itself compare keys. Callback
count/order, mutation during callbacks, and exception recovery are not
guaranteed. See
[evidence](../../investigations/compiler-comparators.md#interface-keys).
